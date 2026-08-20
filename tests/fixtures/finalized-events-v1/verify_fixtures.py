#!/usr/bin/env python3
"""Fail-closed verifier for the independently authored finalized-events-v1 oracle."""

from __future__ import annotations

import hashlib
import json
import os
import re
import stat
import sys
import urllib.parse
import uuid
from pathlib import Path, PurePosixPath
from typing import Any, NoReturn


FIXTURE_ROOT = Path(__file__).resolve().parent
REPOSITORY_ROOT = FIXTURE_ROOT.parents[2]

DEPLOYMENT = "3046cb2cf3f5f9c565a85493cfff10fee94d12d950a0d6f54d7c1ff32a6afc42"
PARACHAIN_GENESIS = "627f53b3abc01130ec273ef85759f90779e8497614a428a66d862a624ee01a17"
RUNTIME_CODE = "e95e40bb618591b98b315b7901f3586ee5899f8bf26bda01401601c4f86b8a00"
ZERO_HASH = "00" * 32
TABLES = (
    "projection_anchor",
    "projected_blocks",
    "projected_events",
    "projection_checkpoint",
    "intent_units",
    "relationship_definitions",
    "intent_unit_relationships",
    "recorded_associations",
)
KINDS = (
    "unit_created",
    "unit_transitioned",
    "unit_completed",
    "relationship_definition_created",
    "relationship_created",
    "relationship_deleted",
    "association_recorded",
    "association_revoked",
)
EXPECTED_PAYLOAD_PATHS = tuple(
    f"raw/payloads/{sequence:04d}-{name}.scale.hex"
    for sequence, name in enumerate(
        (
            "unit-created-a",
            "unit-created-b",
            "relationship-definition-created",
            "relationship-created",
            "association-recorded-a",
            "unit-transitioned-a",
            "unit-completed-a",
            "relationship-deleted",
            "association-revoked-a",
            "association-recorded-b",
            "relationship-recreated",
        ),
        1,
    )
)
REQUIRED_FAULT_IDS = {
    "archive_blocks_flag_missing",
    "archive_state_flag_missing",
    "archive_flag_value_drift",
    "deployment_manifest_digest_mismatch",
    "metadata_digest_mismatch",
    "runtime_wasm_digest_mismatch",
    "genesis_body_probe_unavailable",
    "early_events_probe_unavailable",
    "mid_code_probe_unavailable",
    "current_header_probe_unavailable",
    "live_parachain_genesis_hash_mismatch",
    "live_relay_genesis_hash_mismatch",
    "live_para_id_storage_mismatch",
    "live_deployment_storage_mismatch",
    "live_pallet_storage_version_mismatch",
    "live_event_schema_storage_mismatch",
    "live_metadata_bytes_mismatch",
    "live_runtime_code_bytes_mismatch",
    "live_runtime_identity_mutations",
    "first_sync_statement_faults",
    "first_sync_commit_fault",
    "best_only_block",
    "displaced_finalized_block",
    "identical_duplicate_complete_equality",
    "conflicting_duplicate_hash",
    "conflicting_duplicate_event_row",
    "skipped_block",
    "out_of_order_block",
    "wrong_parent_hash",
    "wrong_block_zero_hash",
    "wrong_deployment_id",
    "wrong_runtime_spec_version",
    "wrong_runtime_spec_name",
    "wrong_runtime_impl_name",
    "wrong_runtime_authoring_version",
    "wrong_runtime_impl_version",
    "wrong_runtime_transaction_version",
    "wrong_runtime_state_version",
    "wrong_runtime_system_version",
    "wrong_runtime_api_version",
    "wrong_runtime_api_order",
    "wrong_runtime_code_hash",
    "wrong_event_schema_version",
    "wrong_cubikan_event_count",
    "global_sequence_zero",
    "global_sequence_gap",
    "global_sequence_duplicate",
    "system_event_order_reversed",
    "accepted_event_initialization_phase",
    "accepted_event_finalization_phase",
    "system_events_trailing_bytes",
    "system_events_declared_count_too_small",
    "system_events_declared_count_too_large",
    "malformed_scale_compact_length",
    "truncated_scale_payload",
    "trailing_scale_payload_bytes",
    "unknown_payload_variant",
    "overbound_payload",
    "replay_invalid_transition",
    "extrinsic_index_out_of_body",
    "extrinsic_hash_join_mismatch",
    "block_statement_faults",
    "block_commit_fault",
    "block_space_fault",
    "block_limit_fault",
    "rpc_source_interruption_mid_block",
    "attestation_raw_event_forgery",
    "attestation_derived_row_forgery",
    "attestation_coherent_event_and_derived_forgery",
    "checkpoint_advances_before_pin",
    "attestation_rpc_interrupt",
    "restart_after_block_fetch",
    "restart_before_block_commit",
    "restart_after_block_commit",
    "two_projectors_contend",
}


def fail(message: str) -> NoReturn:
    raise ValueError(message)


def exact_keys(value: dict[str, Any], expected: set[str], label: str) -> None:
    actual = set(value)
    if actual != expected:
        fail(
            f"{label} keys differ: missing={sorted(expected - actual)!r}, "
            f"extra={sorted(actual - expected)!r}"
        )


def reject_float(_value: str) -> NoReturn:
    fail("JSON floating-point numbers are forbidden")


def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            fail(f"duplicate JSON member: {key!r}")
        result[key] = value
    return result


def load_json(relative: str) -> dict[str, Any]:
    path = safe_fixture_path(relative)
    raw = path.read_bytes()
    if not raw.endswith(b"\n") or b"\r" in raw or b"\x00" in raw:
        fail(f"{relative} must be LF-terminated UTF-8 without CR/NUL")
    try:
        parsed = json.loads(
            raw,
            object_pairs_hook=unique_object,
            parse_float=reject_float,
            parse_constant=reject_float,
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"{relative} is not strict JSON: {error}")
    if not isinstance(parsed, dict):
        fail(f"{relative} top level must be an object")
    return parsed


def safe_fixture_path(relative: str) -> Path:
    pure = PurePosixPath(relative)
    if (
        not relative
        or pure.is_absolute()
        or "\\" in relative
        or any(part in ("", ".", "..") for part in pure.parts)
        or pure.as_posix() != relative
    ):
        fail(f"unsafe fixture path: {relative!r}")
    path = FIXTURE_ROOT.joinpath(*pure.parts)
    if not path.is_file() or path.is_symlink():
        fail(f"fixture path is not one regular non-symlink file: {relative}")
    if path.resolve().parent != path.parent.resolve():
        fail(f"fixture parent resolves unexpectedly: {relative}")
    return path


def sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def u64_be(value: int | str) -> str:
    number = int(value)
    if str(number) != str(value) or not 0 <= number <= (1 << 64) - 1:
        fail(f"noncanonical u64: {value!r}")
    return number.to_bytes(8, "big").hex()


def lower_hex(value: Any, size: int | None, label: str) -> bytes:
    if not isinstance(value, str) or value != value.lower() or len(value) % 2:
        fail(f"{label} is not even-length lowercase hex")
    try:
        decoded = bytes.fromhex(value)
    except ValueError:
        fail(f"{label} is not hexadecimal")
    if decoded.hex() != value or (size is not None and len(decoded) != size):
        fail(f"{label} has the wrong canonical encoding or size")
    return decoded


def read_hex(relative: str) -> bytes:
    raw = safe_fixture_path(relative).read_bytes()
    if not raw.endswith(b"\n") or raw.count(b"\n") != 1:
        fail(f"{relative} must contain one LF-terminated hex line")
    try:
        text = raw[:-1].decode("ascii")
    except UnicodeDecodeError:
        fail(f"{relative} is not ASCII")
    if not text:
        fail(f"{relative} is empty")
    return lower_hex(text, None, relative)


class Scale:
    def __init__(self, raw: bytes, label: str):
        self.raw = raw
        self.position = 0
        self.label = label

    def take(self, size: int) -> bytes:
        end = self.position + size
        if size < 0 or end > len(self.raw):
            fail(f"{self.label}: truncated at byte {self.position}")
        result = self.raw[self.position : end]
        self.position = end
        return result

    def byte(self) -> int:
        return self.take(1)[0]

    def u16(self) -> int:
        return int.from_bytes(self.take(2), "little")

    def u32(self) -> int:
        return int.from_bytes(self.take(4), "little")

    def u64(self) -> int:
        return int.from_bytes(self.take(8), "little")

    def compact(self) -> int:
        start = self.position
        first = self.byte()
        mode = first & 3
        if mode == 0:
            value = first >> 2
        elif mode == 1:
            value = int.from_bytes(bytes((first, self.byte())), "little") >> 2
            if value < 1 << 6:
                fail(f"{self.label}: noncanonical two-byte compact at {start}")
        elif mode == 2:
            value = int.from_bytes(bytes((first,)) + self.take(3), "little") >> 2
            if value < 1 << 14:
                fail(f"{self.label}: noncanonical four-byte compact at {start}")
        else:
            count = (first >> 2) + 4
            if count > 8:
                fail(f"{self.label}: compact integer exceeds u64 at {start}")
            encoded = self.take(count)
            if not encoded or encoded[-1] == 0:
                fail(f"{self.label}: noncanonical big compact at {start}")
            value = int.from_bytes(encoded, "little")
            if value < 1 << 30:
                fail(f"{self.label}: noncanonical big compact at {start}")
        return value

    def vector(self) -> bytes:
        return self.take(self.compact())

    def text(self) -> str:
        raw = self.vector()
        try:
            text = raw.decode("utf-8")
        except UnicodeDecodeError:
            fail(f"{self.label}: text is not UTF-8")
        if not text or "\x00" in text:
            fail(f"{self.label}: text is empty or contains NUL")
        return text

    def option_text(self) -> str | None:
        tag = self.byte()
        if tag == 0:
            return None
        if tag == 1:
            return self.text()
        fail(f"{self.label}: invalid Option tag {tag}")

    def finish(self) -> None:
        if self.position != len(self.raw):
            fail(f"{self.label}: {len(self.raw) - self.position} trailing bytes")


def parse_reference(decoder: Scale) -> dict[str, str]:
    return {
        "namespace": decoder.text(),
        "scope": decoder.text(),
        "value": decoder.text(),
    }


def parse_definition_key(decoder: Scale) -> tuple[str, int]:
    return decoder.text(), decoder.u64()


def parse_payload(decoder: Scale) -> dict[str, Any]:
    variant = decoder.byte()
    if variant == 0:
        command_schema_version = decoder.u16()
        unit_id = str(uuid.UUID(bytes=decoder.take(16)))
        origin = parse_reference(decoder)
        species = decoder.text()
        workflow_id = decoder.text()
        phases = [decoder.text() for _ in range(decoder.compact())]
        initial_phase = decoder.text()
        edges = [
            {"from": decoder.text(), "to": decoder.text()}
            for _ in range(decoder.compact())
        ]
        completion_phases = [decoder.text() for _ in range(decoder.compact())]
        return {
            "kind": KINDS[variant],
            "command_schema_version": command_schema_version,
            "id": unit_id,
            "origin": origin,
            "species": species,
            "workflow": {
                "id": workflow_id,
                "phases": phases,
                "initial_phase": initial_phase,
                "edges": edges,
                "completion_phases": completion_phases,
            },
        }
    if variant in (1, 2):
        result: dict[str, Any] = {
            "kind": KINDS[variant],
            "id": str(uuid.UUID(bytes=decoder.take(16))),
            "revision": decoder.u64(),
        }
        if variant == 1:
            result["from"] = decoder.text()
            result["to"] = decoder.text()
        else:
            result["phase"] = decoder.text()
        return result
    if variant == 3:
        definition_id, definition_version = parse_definition_key(decoder)
        return {
            "kind": KINDS[variant],
            "definition_id": definition_id,
            "definition_version": definition_version,
            "direction": decoder.byte(),
            "source_species": decoder.option_text(),
            "target_species": decoder.option_text(),
            "self_policy": decoder.byte(),
            "cycle_policy": decoder.byte(),
        }
    if variant in (4, 5):
        definition_id, definition_version = parse_definition_key(decoder)
        return {
            "kind": KINDS[variant],
            "definition_id": definition_id,
            "definition_version": definition_version,
            "source_id": str(uuid.UUID(bytes=decoder.take(16))),
            "target_id": str(uuid.UUID(bytes=decoder.take(16))),
        }
    if variant in (6, 7):
        unit_id = str(uuid.UUID(bytes=decoder.take(16)))
        subject_tag = decoder.byte()
        if subject_tag == 0:
            subject_kind = "whole_unit"
            subject_revision = None
        elif subject_tag == 1:
            subject_kind = "revision"
            subject_revision = decoder.u64()
        else:
            fail(f"{decoder.label}: invalid association subject tag {subject_tag}")
        return {
            "kind": KINDS[variant],
            "unit_id": unit_id,
            "subject_kind": subject_kind,
            "subject_revision": subject_revision,
            "reference": parse_reference(decoder),
        }
    fail(f"{decoder.label}: unknown payload variant {variant}")


def decode_payload(raw: bytes, label: str) -> dict[str, Any]:
    decoder = Scale(raw, label)
    payload = parse_payload(decoder)
    decoder.finish()
    return payload


def parse_system_events(raw: bytes, label: str) -> tuple[int, list[dict[str, Any]]]:
    decoder = Scale(raw, label)
    count = decoder.compact()
    accepted: list[dict[str, Any]] = []
    for system_event_index in range(count):
        phase = decoder.byte()
        if phase != 0:
            fail(f"{label}: record {system_event_index} is not ApplyExtrinsic")
        extrinsic_index = decoder.u32()
        pallet_index = decoder.byte()
        event_index = decoder.byte()
        if (pallet_index, event_index) == (0, 5):
            decoder.take(32)
            decoder.take(32)
        elif (pallet_index, event_index) == (50, 0):
            deployment_id = decoder.take(32).hex()
            event_schema_version = decoder.u16()
            global_sequence = decoder.u64()
            signer = decoder.take(32).hex()
            payload_start = decoder.position
            payload = parse_payload(decoder)
            payload_raw = decoder.raw[payload_start : decoder.position]
            accepted.append(
                {
                    "extrinsic_index": extrinsic_index,
                    "system_event_index": system_event_index,
                    "global_sequence": str(global_sequence),
                    "deployment_id": deployment_id,
                    "event_schema_version": event_schema_version,
                    "signer": signer,
                    "kind": payload["kind"],
                    "payload_raw": payload_raw,
                    "decoded_payload": payload,
                }
            )
        else:
            fail(
                f"{label}: unsupported runtime event "
                f"({pallet_index}, {event_index}) at record {system_event_index}"
            )
        topic_count = decoder.compact()
        if topic_count != 0:
            fail(f"{label}: record {system_event_index} has nonempty topics")
    decoder.finish()
    return count, accepted


def verify_inventory(manifest: dict[str, Any]) -> None:
    exact_keys(
        manifest,
        {
            "format",
            "fixture_root",
            "authority",
            "inventory",
            "documents",
            "external_artifacts",
            "stream_summary",
            "verifier",
        },
        "manifest",
    )
    if manifest["format"] != "cubikan-finalized-fixture-manifest-v1":
        fail("manifest format differs")
    if manifest["fixture_root"] != "tests/fixtures/finalized-events-v1":
        fail("manifest fixture_root differs")
    exact_keys(
        manifest["authority"],
        {"independent_oracle", "production_may_regenerate", "trust_boundary"},
        "manifest.authority",
    )
    if manifest["authority"] != {
        "independent_oracle": True,
        "production_may_regenerate": False,
        "trust_boundary": "configured-local-node-finalized-rpc-no-grandpa-proof",
    }:
        fail("manifest authority differs")

    inventory_pin = manifest["inventory"]
    exact_keys(
        inventory_pin, {"path", "entry_count", "size", "sha256"}, "manifest.inventory"
    )
    if inventory_pin["path"] != "inventory-v1.json":
        fail("manifest inventory path differs")
    inventory_path = safe_fixture_path(inventory_pin["path"])
    inventory_raw = inventory_path.read_bytes()
    if len(inventory_raw) != inventory_pin["size"] or sha256(inventory_raw) != inventory_pin["sha256"]:
        fail("inventory bytes do not match manifest pin")
    inventory = load_json("inventory-v1.json")
    exact_keys(inventory, {"format", "entries"}, "inventory")
    if inventory["format"] != "cubikan-finalized-fixture-inventory-v1":
        fail("inventory format differs")
    entries = inventory["entries"]
    if not isinstance(entries, list) or len(entries) != inventory_pin["entry_count"]:
        fail("inventory entry count differs")
    paths = [entry.get("path") for entry in entries]
    if paths != sorted(paths) or len(paths) != len(set(paths)):
        fail("inventory paths are not unique bytewise order")

    actual_paths: set[str] = set()
    for path in FIXTURE_ROOT.rglob("*"):
        relative = path.relative_to(FIXTURE_ROOT).as_posix()
        mode = path.lstat().st_mode
        if stat.S_ISDIR(mode):
            continue
        if not stat.S_ISREG(mode) or path.is_symlink():
            fail(f"fixture tree contains a nonregular entry: {relative}")
        actual_paths.add(relative)
    expected_paths = set(paths) | {"inventory-v1.json", "manifest-v1.json"}
    if actual_paths != expected_paths:
        fail(
            "closed fixture inventory differs: "
            f"missing={sorted(expected_paths - actual_paths)!r}, "
            f"extra={sorted(actual_paths - expected_paths)!r}"
        )

    for entry in entries:
        if not isinstance(entry, dict):
            fail("inventory entry is not an object")
        kind = entry.get("kind")
        expected_keys = {"path", "kind", "file_size", "file_sha256"}
        if kind == "lowercase_hex":
            expected_keys |= {"decoded_size", "decoded_sha256"}
        elif kind != "file":
            fail(f"unsupported inventory kind: {kind!r}")
        exact_keys(entry, expected_keys, f"inventory[{entry.get('path')!r}]")
        path = safe_fixture_path(entry["path"])
        raw = path.read_bytes()
        if len(raw) != entry["file_size"] or sha256(raw) != entry["file_sha256"]:
            fail(f"inventory file identity differs: {entry['path']}")
        if kind == "lowercase_hex":
            decoded = read_hex(entry["path"])
            if len(decoded) != entry["decoded_size"] or sha256(decoded) != entry["decoded_sha256"]:
                fail(f"inventory decoded identity differs: {entry['path']}")

    documents = manifest["documents"]
    exact_keys(
        documents,
        {
            "scale_reasoning",
            "rpc_preflight",
            "valid_stream",
            "expected_projection",
            "fault_cases",
        },
        "manifest.documents",
    )
    for label, pin in documents.items():
        exact_keys(pin, {"path", "size", "sha256"}, f"manifest.documents.{label}")
        raw = safe_fixture_path(pin["path"]).read_bytes()
        if len(raw) != pin["size"] or sha256(raw) != pin["sha256"]:
            fail(f"manifest document pin differs: {label}")


def verify_external_artifacts(manifest: dict[str, Any], preflight: dict[str, Any]) -> None:
    expected = {
        "deployment_anchor": (
            "chain/artifacts/local-deployment-anchor-v1.json",
            5868,
            "38f795fb3bbb666f571b3bd1e4fa3ad1666476f3fff20dee9d93feb9c925dee7",
        ),
        "metadata": (
            "chain/metadata/cubikan-runtime-v1.scale",
            63327,
            "171a323b1e6bf0122e549eecd5f5932e672a3e0835f32edf0b8808cfefd97302",
        ),
        "runtime_wasm": (
            "chain/artifacts/cubikan-runtime-v1.compact.compressed.wasm",
            637930,
            "640cc616674fe7393fc93928904f0fd92d77571209c8200f08b8da6290c6a275",
        ),
    }
    if set(manifest["external_artifacts"]) != set(expected):
        fail("manifest external artifact labels differ")
    preflight_pins = preflight["pinned_artifacts"]
    for label, (relative, size, digest) in expected.items():
        pin = manifest["external_artifacts"][label]
        exact_keys(pin, {"path", "size", "sha256"}, f"external_artifacts.{label}")
        if pin != {"path": relative, "size": size, "sha256": digest}:
            fail(f"external artifact pin differs: {label}")
        preflight_pin = preflight_pins[label]
        if {key: preflight_pin[key] for key in ("path", "size", "sha256")} != pin:
            fail(f"preflight artifact pin differs: {label}")
        path = REPOSITORY_ROOT.joinpath(*PurePosixPath(relative).parts)
        if not path.is_file() or path.is_symlink():
            fail(f"external artifact is absent/nonregular: {relative}")
        raw = path.read_bytes()
        if len(raw) != size or sha256(raw) != digest:
            fail(f"external artifact bytes differ: {relative}")
    if preflight_pins["runtime_wasm"]["blake2_256"] != RUNTIME_CODE:
        fail("runtime Wasm code hash pin differs")


def strict_local_ws(url: str) -> bool:
    match = re.fullmatch(
        r"ws://(?P<host>(?:[0-9]+(?:\.[0-9]+){3})|\[::1\]):(?P<port>[0-9]+)/",
        url,
    )
    if match is None:
        return False
    host = match.group("host")
    port_text = match.group("port")
    if (len(port_text) > 1 and port_text.startswith("0")) or not port_text:
        return False
    port = int(port_text)
    if not 1 <= port <= 65535 or port == 80:
        return False
    if host != "[::1]":
        octets = host.split(".")
        if any(
            not octet
            or (len(octet) > 1 and octet.startswith("0"))
            or int(octet) > 255
            for octet in octets
        ) or int(octets[0]) != 127:
            return False
    try:
        parsed = urllib.parse.urlsplit(url)
    except ValueError:
        return False
    return (
        parsed.scheme == "ws"
        and parsed.hostname == ("::1" if host == "[::1]" else host)
        and parsed.username is None
        and parsed.password is None
        and parsed.port == port
        and parsed.path == "/"
        and not parsed.query
        and not parsed.fragment
        and parsed.netloc == f"{host}:{port}"
        and url == f"ws://{host}:{port}/"
    )


def verify_preflight(preflight: dict[str, Any]) -> None:
    exact_keys(
        preflight,
        {
            "format",
            "trust_boundary",
            "connection",
            "pinned_artifacts",
            "identity",
            "genesis_probes",
            "metadata_probe",
            "runtime_version_probe",
            "storage_probes",
            "historical_probes",
            "probe_order",
        },
        "preflight",
    )
    if preflight["format"] != "cubikan-finalized-rpc-preflight-v1":
        fail("preflight format differs")
    exact_keys(
        preflight["trust_boundary"],
        {
            "assertion",
            "grandpa_proof_verified",
            "perpetual_archive_retention_claimed",
            "shared_security_claimed",
        },
        "preflight.trust_boundary",
    )
    if preflight["trust_boundary"] != {
        "assertion": "configured-local-node-finalized-rpc",
        "grandpa_proof_verified": False,
        "perpetual_archive_retention_claimed": False,
        "shared_security_claimed": False,
    }:
        fail("preflight trust boundary differs")
    connection = preflight["connection"]
    exact_keys(
        connection,
        {
            "accepted_url",
            "required_node_argv_pairs",
            "normalized_flag_display",
            "rejected_raw_argv_tokens",
            "url_cases",
        },
        "connection",
    )
    if connection["accepted_url"] != "ws://127.0.0.1:9988/":
        fail("accepted RPC URL differs")
    if connection["required_node_argv_pairs"] != [
        ["--blocks-pruning", "archive"],
        ["--state-pruning", "archive"],
    ]:
        fail("archive argv pairs differ")
    if connection["normalized_flag_display"] != [
        "--blocks-pruning=archive",
        "--state-pruning=archive",
    ] or connection["rejected_raw_argv_tokens"] != [
        "--blocks-pruning=archive",
        "--state-pruning=archive",
    ]:
        fail("archive display/rejected raw token inventory differs")
    for index, case in enumerate(connection["url_cases"]):
        exact_keys(case, {"input", "accepted"}, f"url_cases[{index}]")
        if strict_local_ws(case["input"]) != case["accepted"]:
            fail(f"URL expectation differs for {case['input']!r}")
    identity = preflight["identity"]
    exact_keys(
        identity,
        {
            "namespace",
            "relay_genesis_hash",
            "parachain_genesis_hash",
            "para_id",
            "deployment_id",
            "pallet_storage_version",
            "event_schema_version",
            "runtime",
        },
        "preflight.identity",
    )
    if (
        identity["namespace"] != "polkadot-sdk-parachain"
        or identity["parachain_genesis_hash"] != PARACHAIN_GENESIS
        or identity["deployment_id"] != DEPLOYMENT
        or identity["para_id"] != 1000
        or identity["pallet_storage_version"] != 1
        or identity["event_schema_version"] != 1
    ):
        fail("preflight identity differs")
    runtime = identity["runtime"]
    exact_keys(
        runtime,
        {
            "spec_name",
            "impl_name",
            "authoring_version",
            "spec_version",
            "impl_version",
            "apis",
            "transaction_version",
            "state_version",
            "system_version",
            "code_hash",
        },
        "preflight.identity.runtime",
    )
    expected_apis = [
        ("ab3c0572291feb8b", 2),
        ("ea93e3f16f3d6962", 3),
        ("37e397fc7c91f5e4", 2),
        ("37c8bb1350a9a2a8", 4),
        ("bc9d89904f5b923f", 1),
        ("df6acb689907609b", 5),
        ("d7bdd8a272ca0d65", 2),
        ("075f8cd374350e84", 1),
        ("d2bc9897eed08f15", 3),
        ("fbc577b9d747efd6", 1),
        ("f78b278be53f454c", 2),
        ("a2ddb6a58477bf63", 1),
        ("dd718d5cc53262d4", 1),
        ("04e70521a0d3d2f8", 2),
        ("40fe3ad401f8959a", 6),
        ("ccd9de6396c899ca", 1),
        ("f3ff14d5ab527059", 3),
    ]
    actual_apis = []
    for index, api in enumerate(runtime["apis"]):
        exact_keys(api, {"id", "version"}, f"runtime.apis[{index}]")
        lower_hex(api["id"], 8, f"runtime.apis[{index}].id")
        actual_apis.append((api["id"], api["version"]))
    if runtime != {
        "spec_name": "cubikan-runtime",
        "impl_name": "cubikan-runtime",
        "authoring_version": 1,
        "spec_version": 1,
        "impl_version": 0,
        "apis": runtime["apis"],
        "transaction_version": 1,
        "state_version": 1,
        "system_version": 1,
        "code_hash": RUNTIME_CODE,
    } or actual_apis != expected_apis:
        fail("complete ordered runtime identity differs")
    genesis_probes = preflight["genesis_probes"]
    if genesis_probes != [
        {
            "role": "relay",
            "rpc_url": "ws://127.0.0.1:9944/",
            "method": "chain_getBlockHash",
            "params": [0],
            "expected_hash": "eb2ada687ce553d3b9d695afd5d9d0a9c44a0b82e1f6eb823ac87e81638200f0",
        },
        {
            "role": "parachain",
            "rpc_url": "ws://127.0.0.1:9988/",
            "method": "chain_getBlockHash",
            "params": [0],
            "expected_hash": PARACHAIN_GENESIS,
        },
    ]:
        fail("live genesis probe inventory differs")
    if preflight["metadata_probe"] != {
        "method": "state_getMetadata",
        "params": [PARACHAIN_GENESIS],
        "expected_size": 63327,
        "expected_sha256": "171a323b1e6bf0122e549eecd5f5932e672a3e0835f32edf0b8808cfefd97302",
    }:
        fail("live metadata probe identity differs")
    if preflight["runtime_version_probe"] != {
        "method": "state_getRuntimeVersion",
        "params": [PARACHAIN_GENESIS],
        "expected_identity": "identity.runtime",
    }:
        fail("live runtime-version probe identity differs")
    if [probe["label"] for probe in preflight["storage_probes"]] != [
        "deployment_id",
        "event_schema_version",
        "pallet_storage_version",
        "para_id",
        "runtime_code",
    ]:
        fail("storage probe inventory/order differs")
    roles = [probe["role"] for probe in preflight["historical_probes"]]
    numbers = [probe["block_number"] for probe in preflight["historical_probes"]]
    if roles != ["genesis", "early", "mid", "current"] or numbers != ["0", "1", "3", "5"]:
        fail("historical probe roles/numbers differ")
    required_historical_methods = [
        "chain_getBlockHash",
        "chain_getHeader",
        "chain_getBlock",
        "state_getStorage(System::Events)",
        "state_getStorage(:code)",
    ]
    for index, probe in enumerate(preflight["historical_probes"]):
        exact_keys(
            probe,
            {"role", "block_number", "block_hash", "required_methods"},
            f"historical_probes[{index}]",
        )
        if probe["required_methods"] != required_historical_methods:
            fail(f"historical probe method inventory differs at index {index}")
    if preflight["probe_order"][-1] != "permit_decode_apply_or_attestation":
        fail("preflight does not gate decode/apply/attestation last")


def verify_body(relative: str, block_number: str) -> list[dict[str, Any]]:
    body = load_json(relative)
    exact_keys(body, {"fixture_format", "block_number", "extrinsics"}, relative)
    if body["fixture_format"] != "cubikan-finalized-body-v1" or body["block_number"] != block_number:
        fail(f"body identity differs: {relative}")
    extrinsics = body["extrinsics"]
    for expected_index, extrinsic in enumerate(extrinsics):
        exact_keys(extrinsic, {"index", "scale_hex", "blake2_256"}, f"{relative}.extrinsic")
        if extrinsic["index"] != expected_index:
            fail(f"body extrinsic indices are not contiguous: {relative}")
        raw = lower_hex(extrinsic["scale_hex"], None, f"{relative}.scale_hex")
        digest = hashlib.blake2b(raw, digest_size=32).hexdigest()
        if digest != extrinsic["blake2_256"]:
            fail(f"body extrinsic hash differs: {relative} index {expected_index}")
    return extrinsics


def verify_stream(stream: dict[str, Any]) -> list[dict[str, Any]]:
    exact_keys(stream, {"format", "system_events_storage_key", "accepted_event", "blocks"}, "stream")
    if stream["format"] != "cubikan-finalized-event-stream-v1":
        fail("stream format differs")
    lower_hex(stream["system_events_storage_key"], 32, "system events storage key")
    if stream["accepted_event"] != {
        "pallet_index": 50,
        "event_index": 0,
        "deployment_id": DEPLOYMENT,
        "event_schema_version": 1,
    }:
        fail("accepted runtime event identity differs")
    blocks = stream["blocks"]
    if len(blocks) != 6:
        fail("stream must contain exactly blocks 0 through 5")
    all_events: list[dict[str, Any]] = []
    prior_hash = ZERO_HASH
    prior_sequence: int | None = None
    payload_paths: list[str] = []
    for expected_number, block in enumerate(blocks):
        exact_keys(
            block,
            {
                "block_number",
                "block_hash",
                "parent_hash",
                "finalized",
                "runtime_spec_version",
                "runtime_code_hash",
                "body",
                "system_events",
                "system_event_record_count",
                "cubikan_event_count",
                "first_global_sequence",
                "last_global_sequence",
                "checkpoint_after",
                "events",
            },
            f"block[{expected_number}]",
        )
        if block["block_number"] != str(expected_number) or not block["finalized"]:
            fail(f"block number/finality differs at {expected_number}")
        lower_hex(block["block_hash"], 32, f"block {expected_number} hash")
        lower_hex(block["parent_hash"], 32, f"block {expected_number} parent")
        if block["parent_hash"] != prior_hash:
            fail(f"parent continuity differs at block {expected_number}")
        if expected_number == 0 and block["block_hash"] != PARACHAIN_GENESIS:
            fail("block zero hash differs from parachain genesis")
        if block["runtime_spec_version"] != 1 or block["runtime_code_hash"] != RUNTIME_CODE:
            fail(f"runtime identity differs at block {expected_number}")
        extrinsics = verify_body(block["body"], block["block_number"])
        record_count, decoded_events = parse_system_events(
            read_hex(block["system_events"]), block["system_events"]
        )
        if record_count != block["system_event_record_count"]:
            fail(f"System event count differs at block {expected_number}")
        if len(decoded_events) != block["cubikan_event_count"] or len(decoded_events) != len(block["events"]):
            fail(f"CubiKan event count differs at block {expected_number}")
        sequences = [int(event["global_sequence"]) for event in block["events"]]
        expected_first = str(sequences[0]) if sequences else None
        expected_last = str(sequences[-1]) if sequences else None
        if block["first_global_sequence"] != expected_first or block["last_global_sequence"] != expected_last:
            fail(f"per-block sequence endpoints differ at block {expected_number}")
        for fixture_event, decoded in zip(block["events"], decoded_events, strict=True):
            exact_keys(
                fixture_event,
                {
                    "extrinsic_index",
                    "system_event_index",
                    "global_sequence",
                    "kind",
                    "signer",
                    "extrinsic_hash",
                    "payload",
                },
                f"block {expected_number} event",
            )
            for key in (
                "extrinsic_index",
                "system_event_index",
                "global_sequence",
                "kind",
                "signer",
            ):
                if fixture_event[key] != decoded[key]:
                    fail(f"decoded {key} differs at global sequence {fixture_event['global_sequence']}")
            if decoded["deployment_id"] != DEPLOYMENT or decoded["event_schema_version"] != 1:
                fail(f"accepted event identity differs at sequence {fixture_event['global_sequence']}")
            payload_raw = read_hex(fixture_event["payload"])
            if decoded["payload_raw"] != payload_raw:
                fail(f"embedded payload bytes differ at sequence {fixture_event['global_sequence']}")
            if decode_payload(payload_raw, fixture_event["payload"]) != decoded["decoded_payload"]:
                fail(f"payload decode differs at sequence {fixture_event['global_sequence']}")
            payload_paths.append(fixture_event["payload"])
            index = fixture_event["extrinsic_index"]
            if index >= len(extrinsics) or extrinsics[index]["blake2_256"] != fixture_event["extrinsic_hash"]:
                fail(f"extrinsic body/hash join differs at sequence {fixture_event['global_sequence']}")
            sequence = int(fixture_event["global_sequence"])
            if sequence == 0 or sequence != (1 if prior_sequence is None else prior_sequence + 1):
                fail(f"global sequence is not contiguous at {sequence}")
            prior_sequence = sequence
            joined = dict(fixture_event)
            joined["block_number"] = block["block_number"]
            joined["block_hash"] = block["block_hash"]
            joined["decoded_payload"] = decoded["decoded_payload"]
            all_events.append(joined)
        expected_checkpoint = None if prior_sequence is None else str(prior_sequence)
        if block["checkpoint_after"] != expected_checkpoint:
            fail(f"checkpoint_after differs at block {expected_number}")
        prior_hash = block["block_hash"]
    if tuple(payload_paths) != EXPECTED_PAYLOAD_PATHS:
        fail("payload path inventory/order differs")
    if len(all_events) != 11 or prior_sequence != 11:
        fail("valid stream must contain exactly sequences 1 through 11")
    return all_events


def coordinate(event: dict[str, Any]) -> dict[str, Any]:
    return {
        "parachain_genesis_hash": PARACHAIN_GENESIS,
        "deployment_id": DEPLOYMENT,
        "block_number": event["block_number"],
        "block_hash": event["block_hash"],
        "extrinsic_index": event["extrinsic_index"],
        "extrinsic_hash": event["extrinsic_hash"],
        "system_event_index": event["system_event_index"],
        "global_sequence": event["global_sequence"],
    }


def canonical_envelope(unit: dict[str, Any]) -> str:
    envelope = {
        "representation_version": 2,
        "id": unit["id"],
        "origin": unit["origin"],
        "species": unit["species"],
        "workflow": unit["workflow"],
        "phase": unit["phase"],
        "status": unit["status"],
        "revision": str(unit["revision"]),
        "history": unit["history"],
    }
    return json.dumps(envelope, ensure_ascii=False, separators=(",", ":"))


def replay_domain(events: list[dict[str, Any]]) -> tuple[dict[str, Any], ...]:
    units: dict[str, dict[str, Any]] = {}
    definitions: dict[tuple[str, int], dict[str, Any]] = {}
    relationships: dict[tuple[str, int, str, str], dict[str, Any]] = {}
    associations: dict[tuple[Any, ...], dict[str, Any]] = {}
    by_sequence = {event["global_sequence"]: event for event in events}
    for event in events:
        payload = event["decoded_payload"]
        kind = payload["kind"]
        sequence = event["global_sequence"]
        if kind == "unit_created":
            if payload["command_schema_version"] != 1 or payload["id"] in units:
                fail(f"invalid unit creation at sequence {sequence}")
            workflow = payload["workflow"]
            if (
                workflow["initial_phase"] not in workflow["phases"]
                or any(edge["from"] not in workflow["phases"] or edge["to"] not in workflow["phases"] for edge in workflow["edges"])
                or any(phase not in workflow["phases"] for phase in workflow["completion_phases"])
            ):
                fail(f"invalid workflow at sequence {sequence}")
            units[payload["id"]] = {
                "id": payload["id"],
                "origin": payload["origin"],
                "species": payload["species"],
                "workflow": workflow,
                "phase": workflow["initial_phase"],
                "status": "active",
                "revision": 0,
                "history": [],
                "last_sequence": sequence,
            }
        elif kind == "unit_transitioned":
            unit = units.get(payload["id"])
            if (
                unit is None
                or unit["status"] != "active"
                or payload["revision"] != unit["revision"] + 1
                or payload["from"] != unit["phase"]
                or {"from": payload["from"], "to": payload["to"]} not in unit["workflow"]["edges"]
            ):
                fail(f"invalid transition at sequence {sequence}")
            unit["revision"] = payload["revision"]
            unit["phase"] = payload["to"]
            unit["history"].append(
                {
                    "type": "transition",
                    "sequence": str(payload["revision"]),
                    "from": payload["from"],
                    "to": payload["to"],
                }
            )
            unit["last_sequence"] = sequence
        elif kind == "unit_completed":
            unit = units.get(payload["id"])
            if (
                unit is None
                or unit["status"] != "active"
                or payload["revision"] != unit["revision"] + 1
                or payload["phase"] != unit["phase"]
                or payload["phase"] not in unit["workflow"]["completion_phases"]
            ):
                fail(f"invalid completion at sequence {sequence}")
            unit["revision"] = payload["revision"]
            unit["status"] = "completed"
            unit["history"].append(
                {
                    "type": "completion",
                    "sequence": str(payload["revision"]),
                    "phase": payload["phase"],
                }
            )
            unit["last_sequence"] = sequence
        elif kind == "relationship_definition_created":
            key = (payload["definition_id"], payload["definition_version"])
            if key in definitions or payload["direction"] != 0 or payload["self_policy"] != 1 or payload["cycle_policy"] != 1:
                fail(f"invalid relationship definition at sequence {sequence}")
            definitions[key] = {**payload, "created_sequence": sequence}
        elif kind in ("relationship_created", "relationship_deleted"):
            definition_key = (payload["definition_id"], payload["definition_version"])
            key = (*definition_key, payload["source_id"], payload["target_id"])
            if definition_key not in definitions or payload["source_id"] not in units or payload["target_id"] not in units:
                fail(f"unknown relationship identity at sequence {sequence}")
            if kind == "relationship_created":
                if key in relationships:
                    fail(f"duplicate relationship at sequence {sequence}")
                relationships[key] = {**payload, "created_sequence": sequence}
            else:
                if key not in relationships:
                    fail(f"missing relationship deletion target at sequence {sequence}")
                del relationships[key]
        elif kind in ("association_recorded", "association_revoked"):
            if payload["unit_id"] not in units:
                fail(f"unknown association unit at sequence {sequence}")
            revision_key = "" if payload["subject_kind"] == "whole_unit" else u64_be(payload["subject_revision"])
            reference = payload["reference"]
            key = (
                payload["unit_id"],
                payload["subject_kind"],
                revision_key,
                reference["namespace"],
                reference["scope"],
                reference["value"],
            )
            if kind == "association_recorded":
                if key in associations:
                    fail(f"duplicate association at sequence {sequence}")
                associations[key] = {**payload, "created_sequence": sequence}
            else:
                if key not in associations:
                    fail(f"missing association revocation target at sequence {sequence}")
                del associations[key]
        else:
            fail(f"unhandled domain payload kind: {kind}")
    return units, definitions, relationships, associations, by_sequence


def verify_projection(
    projection: dict[str, Any], stream: dict[str, Any], events: list[dict[str, Any]]
) -> None:
    exact_keys(
        projection,
        {
            "format",
            "schema_version",
            "integer_blob_encoding",
            *TABLES,
            "row_counts",
            "event_effects",
        },
        "projection",
    )
    if (
        projection["format"] != "cubikan-finalized-expected-projection-v1"
        or projection["schema_version"] != 3
        or projection["integer_blob_encoding"] != "u64-big-endian"
    ):
        fail("projection format/schema/encoding differs")
    anchor = projection["projection_anchor"]
    if (
        anchor["singleton"] != 1
        or anchor["namespace"] != "polkadot-sdk-parachain"
        or anchor["parachain_genesis_hash"] != PARACHAIN_GENESIS
        or anchor["deployment_id"] != DEPLOYMENT
        or anchor["para_id"] != 1000
        or anchor["pallet_storage_version"] != 1
        or anchor["event_schema_version"] != 1
        or anchor["initial_runtime_spec_version"] != 1
        or anchor["initial_runtime_code_hash"] != RUNTIME_CODE
    ):
        fail("projection anchor differs")

    blocks = projection["projected_blocks"]
    if len(blocks) != len(stream["blocks"]):
        fail("projected block count differs")
    for expected, row in zip(stream["blocks"], blocks, strict=True):
        if (
            row["anchor_singleton"] != 1
            or row["block_number"] != expected["block_number"]
            or row["block_number_be_hex"] != u64_be(expected["block_number"])
            or row["block_hash"] != expected["block_hash"]
            or row["parent_hash"] != expected["parent_hash"]
            or row["runtime_spec_version"] != expected["runtime_spec_version"]
            or row["runtime_code_hash"] != expected["runtime_code_hash"]
            or row["cubikan_event_count"] != expected["cubikan_event_count"]
            or row["first_global_sequence_be_hex"] != (
                None if expected["first_global_sequence"] is None else u64_be(expected["first_global_sequence"])
            )
            or row["last_global_sequence_be_hex"] != (
                None if expected["last_global_sequence"] is None else u64_be(expected["last_global_sequence"])
            )
        ):
            fail(f"projected block row differs at {expected['block_number']}")

    rows = projection["projected_events"]
    if len(rows) != len(events):
        fail("projected event count differs")
    for event, row in zip(events, rows, strict=True):
        expected = {
            "block_number_be_hex": u64_be(event["block_number"]),
            "extrinsic_index": event["extrinsic_index"],
            "system_event_index": event["system_event_index"],
            "global_sequence_be_hex": u64_be(event["global_sequence"]),
            "deployment_id": DEPLOYMENT,
            "event_schema_version": 1,
            "event_kind": event["kind"],
            "scale_payload": event["payload"],
            "signer": event["signer"],
            "extrinsic_hash": event["extrinsic_hash"],
        }
        if row != expected:
            fail(f"projected event row differs at sequence {event['global_sequence']}")

    last_block = stream["blocks"][-1]
    checkpoint = projection["projection_checkpoint"]
    if checkpoint != {
        "singleton": 1,
        "block_number": last_block["block_number"],
        "block_number_be_hex": u64_be(last_block["block_number"]),
        "block_hash": last_block["block_hash"],
        "last_global_sequence": "11",
        "last_global_sequence_be_hex": u64_be(11),
        "runtime_spec_version": 1,
        "runtime_code_hash": RUNTIME_CODE,
    }:
        fail("projection checkpoint differs")

    units, definitions, relationships, associations, by_sequence = replay_domain(events)
    expected_unit_rows: list[dict[str, Any]] = []
    for unit_id in sorted(units):
        unit = units[unit_id]
        last_event = by_sequence[unit["last_sequence"]]
        expected_unit_rows.append(
            {
                "id": unit_id,
                "envelope_version": 2,
                "envelope": canonical_envelope(unit),
                "origin_namespace": unit["origin"]["namespace"],
                "origin_scope": unit["origin"]["scope"],
                "origin_value": unit["origin"]["value"],
                "workflow_id": unit["workflow"]["id"],
                "species": unit["species"],
                "phase": unit["phase"],
                "status": unit["status"],
                "revision_be_hex": u64_be(unit["revision"]),
                "last_global_sequence_be_hex": u64_be(unit["last_sequence"]),
                "last_coordinate": coordinate(last_event),
            }
        )
    if projection["intent_units"] != expected_unit_rows:
        fail("final intent-unit rows/envelopes differ from independent replay")

    expected_definition_rows = []
    for key in sorted(definitions):
        definition = definitions[key]
        expected_definition_rows.append(
            {
                "definition_id": key[0],
                "definition_version": str(key[1]),
                "definition_version_be_hex": u64_be(key[1]),
                "directed": 1,
                "source_species": definition["source_species"],
                "target_species": definition["target_species"],
                "self_policy": "reject",
                "cycle_policy": "reject",
                "created_global_sequence_be_hex": u64_be(definition["created_sequence"]),
            }
        )
    if projection["relationship_definitions"] != expected_definition_rows:
        fail("relationship definition rows differ from independent replay")

    expected_relationship_rows = []
    for key in sorted(relationships):
        relationship = relationships[key]
        created = by_sequence[relationship["created_sequence"]]
        expected_relationship_rows.append(
            {
                "definition_id": key[0],
                "definition_version": str(key[1]),
                "definition_version_be_hex": u64_be(key[1]),
                "source_id": key[2],
                "target_id": key[3],
                "created_global_sequence_be_hex": u64_be(relationship["created_sequence"]),
                "created_coordinate": coordinate(created),
            }
        )
    if projection["intent_unit_relationships"] != expected_relationship_rows:
        fail("relationship rows differ from independent replay")

    expected_association_rows = []
    for key in sorted(associations):
        association = associations[key]
        created = by_sequence[association["created_sequence"]]
        expected_association_rows.append(
            {
                "unit_id": key[0],
                "subject_kind": key[1],
                "subject_revision_key_be_hex": key[2],
                "namespace": key[3],
                "scope": key[4],
                "value": key[5],
                "created_global_sequence_be_hex": u64_be(association["created_sequence"]),
                "created_coordinate": coordinate(created),
            }
        )
    if projection["recorded_associations"] != expected_association_rows:
        fail("association rows differ from independent replay")

    expected_counts = {
        "projection_anchor": 1,
        "projected_blocks": len(blocks),
        "projected_events": len(rows),
        "projection_checkpoint": 1,
        "intent_units": len(expected_unit_rows),
        "relationship_definitions": len(expected_definition_rows),
        "intent_unit_relationships": len(expected_relationship_rows),
        "recorded_associations": len(expected_association_rows),
    }
    if projection["row_counts"] != expected_counts:
        fail("projection row-count oracle differs")
    if [effect["global_sequence"] for effect in projection["event_effects"]] != [
        str(value) for value in range(1, 12)
    ]:
        fail("event-effects sequence inventory differs")


def verify_faults(faults: dict[str, Any]) -> None:
    exact_keys(
        faults,
        {"format", "base_stream", "expected_projection", "baselines", "generated_values", "cases"},
        "fault corpus",
    )
    if (
        faults["format"] != "cubikan-finalized-fault-corpus-v1"
        or faults["base_stream"] != "valid-stream-v1.json"
        or faults["expected_projection"] != "expected-projection-v1.json"
    ):
        fail("fault corpus identity differs")
    if set(faults["baselines"]) != {"schema_only", "through_block_3", "full_valid"}:
        fail("fault baseline inventory differs")
    for name, baseline in faults["baselines"].items():
        exact_keys(baseline, {"row_counts", "checkpoint"}, f"baseline {name}")
        if set(baseline["row_counts"]) != set(TABLES):
            fail(f"baseline {name} table inventory differs")
    generated = faults["generated_values"]
    exact_keys(generated, {"overbound_payload"}, "generated_values")
    overbound = generated["overbound_payload"]
    exact_keys(overbound, {"recipe", "byte_hex", "decoded_size", "sha256"}, "overbound payload")
    generated_raw = lower_hex(overbound["byte_hex"], 1, "overbound byte") * overbound["decoded_size"]
    if (
        overbound["recipe"] != "repeat-byte"
        or overbound["decoded_size"] != 1_048_577
        or sha256(generated_raw) != overbound["sha256"]
    ):
        fail("overbound payload recipe/identity differs")
    ids: list[str] = []
    for index, case in enumerate(faults["cases"]):
        exact_keys(case, {"id", "stage", "baseline", "mutation", "expected"}, f"fault case {index}")
        if not isinstance(case["id"], str) or not case["id"]:
            fail(f"fault case {index} has no canonical id")
        if case["baseline"] not in faults["baselines"]:
            fail(f"fault case {case['id']} references an unknown baseline")
        if not isinstance(case["mutation"], dict) or not case["mutation"]:
            fail(f"fault case {case['id']} has no mutation")
        if not isinstance(case["expected"], dict) or not case["expected"]:
            fail(f"fault case {case['id']} has no expected result")
        ids.append(case["id"])
    if len(ids) != len(set(ids)):
        fail("fault case ids are not unique")
    if set(ids) != REQUIRED_FAULT_IDS:
        fail(
            "fault case inventory differs: "
            f"missing={sorted(REQUIRED_FAULT_IDS - set(ids))!r}, "
            f"extra={sorted(set(ids) - REQUIRED_FAULT_IDS)!r}"
        )
    identical = faults["cases"][ids.index("identical_duplicate_complete_equality")]["expected"]
    if identical.get("outcome") != "exact_noop" or not identical.get(
        "complete_row_and_envelope_equality_required"
    ):
        fail("identical duplicate is not gated by complete equality")


def verify_manifest_summary(manifest: dict[str, Any], projection: dict[str, Any]) -> None:
    summary = manifest["stream_summary"]
    exact_keys(
        summary,
        {
            "first_block",
            "last_block",
            "block_count",
            "accepted_event_count",
            "first_global_sequence",
            "last_global_sequence",
            "final_checkpoint_block_hash",
            "row_counts",
            "fault_case_count",
        },
        "manifest.stream_summary",
    )
    expected = {
        "first_block": "0",
        "last_block": "5",
        "block_count": 6,
        "accepted_event_count": 11,
        "first_global_sequence": "1",
        "last_global_sequence": "11",
        "final_checkpoint_block_hash": "55" * 32,
        "row_counts": projection["row_counts"],
        "fault_case_count": len(REQUIRED_FAULT_IDS),
    }
    if summary != expected:
        fail("manifest stream summary differs")
    verifier = manifest["verifier"]
    exact_keys(verifier, {"path", "command", "requires"}, "manifest.verifier")
    if verifier != {
        "path": "verify_fixtures.py",
        "command": "PYTHONDONTWRITEBYTECODE=1 python3 tests/fixtures/finalized-events-v1/verify_fixtures.py",
        "requires": "Python 3.10+ standard library only",
    }:
        fail("manifest verifier invocation differs")


def main() -> None:
    if os.environ.get("PYTHONDONTWRITEBYTECODE") != "1":
        fail("set PYTHONDONTWRITEBYTECODE=1 so verification cannot mutate the fixture tree")
    manifest = load_json("manifest-v1.json")
    verify_inventory(manifest)
    preflight = load_json("rpc-preflight-v1.json")
    verify_preflight(preflight)
    verify_external_artifacts(manifest, preflight)
    stream = load_json("valid-stream-v1.json")
    events = verify_stream(stream)
    projection = load_json("expected-projection-v1.json")
    verify_projection(projection, stream, events)
    faults = load_json("fault-cases-v1.json")
    verify_faults(faults)
    verify_manifest_summary(manifest, projection)
    print(
        "finalized-events-v1 verified: "
        f"{len(stream['blocks'])} blocks, {len(events)} accepted events, "
        f"{len(faults['cases'])} fault cases"
    )


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, TypeError, KeyError, IndexError) as error:
        print(f"finalized-events-v1 verification failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
