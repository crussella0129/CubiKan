#!/usr/bin/env python3
"""Fail-closed verifier for the independent submission-journal-v1 oracle."""

from __future__ import annotations

import hashlib
import json
import os
import sys
from pathlib import Path, PurePosixPath
from typing import Any, NoReturn


FIXTURE_ROOT = Path(__file__).resolve().parent
REPOSITORY_ROOT = FIXTURE_ROOT.parents[2]

LANE_DOMAIN = b"CubiKan signer lane v1\0"
JOURNAL_DOMAIN = b"CubiKan submission-journal-v1\0"
DEPLOYMENT = bytes.fromhex(
    "3046cb2cf3f5f9c565a85493cfff10fee94d12d950a0d6f54d7c1ff32a6afc42"
)
ALICE = bytes.fromhex(
    "d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"
)
GENESIS = bytes.fromhex(
    "627f53b3abc01130ec273ef85759f90779e8497614a428a66d862a624ee01a17"
)
SIGNING_HASH = bytes.fromhex(
    "838281807f7e7d7c7b7a797877767574737271706f6e6d6c6b6a696867666564"
)
EXTRINSIC_HASH = bytes.fromhex(
    "8dc2048ba261737a5d52bb1320df571e2aa3028c22783fc60a0dd021649cb5ca"
)
ZERO_HASH = bytes(32)

STATE_NAMES = (
    "prepared",
    "finalized_accepted",
    "finalized_dispatch_rejected",
    "finalized_invariant_failed",
    "expired_not_included",
)
OPERATION_NAMES = (
    "create_unit",
    "transition_unit",
    "complete_unit",
    "create_relationship_definition",
    "create_relationship",
    "delete_relationship",
    "record_association",
    "revoke_association",
)
LAYOUT = (
    ("magic", 0, 8, "ascii:CUBKJNL1"),
    ("format_version", 8, 10, "u16_be"),
    ("state", 10, 11, "u8_tag"),
    ("flags", 11, 12, "zero"),
    ("total_length", 12, 14, "u16_be"),
    ("reserved_header", 14, 16, "zero"),
    ("deployment_id", 16, 48, "bytes32"),
    ("signer", 48, 80, "account_id32"),
    ("nonce", 80, 88, "u64_be"),
    ("extrinsic_hash", 88, 120, "bytes32"),
    ("signing_finalized_number", 120, 128, "u64_be"),
    ("signing_finalized_hash", 128, 160, "bytes32"),
    ("birth", 160, 168, "u64_be"),
    ("death", 168, 176, "u64_be"),
    ("resolution_number", 176, 184, "u64_be"),
    ("resolution_hash", 184, 216, "bytes32"),
    ("mutation_operation", 216, 217, "u8_tag"),
    ("reserved_body", 217, 224, "zero"),
    ("checksum", 224, 256, "sha256_domain_plus_0_224"),
)

REQUIRED_REJECTION_IDS = {
    "empty_record",
    "short_magic",
    "short_body",
    "checksum_absent",
    "one_byte_short",
    "one_trailing_byte",
    "overlong_second_record",
    "bad_magic_rechecksummed",
    "wrong_version_rechecksummed",
    "unknown_state_5_rechecksummed",
    "unknown_state_255_rechecksummed",
    "nonzero_flags_rechecksummed",
    "declared_length_255_rechecksummed",
    "declared_length_257_rechecksummed",
    "reserved_header_first_rechecksummed",
    "reserved_header_last_rechecksummed",
    "reserved_body_first_rechecksummed",
    "reserved_body_last_rechecksummed",
    "unknown_operation_8_rechecksummed",
    "unknown_operation_255_rechecksummed",
    "checksum_bit_flip",
    "body_bit_flip_without_checksum",
    "prepared_resolution_number_only",
    "prepared_resolution_hash_only",
    "prepared_resolution_both",
    "finalized_resolution_all_zero",
    "finalized_resolution_number_only",
    "finalized_resolution_hash_only",
    "finalized_inclusion_before_birth",
    "finalized_inclusion_after_death",
    "expiry_at_death",
    "expiry_before_death",
    "birth_differs_from_signing",
    "death_not_birth_plus_63",
    "era_addition_overflow",
    "zero_extrinsic_hash",
    "zero_signing_hash",
    "deployment_lane_mismatch_rechecksummed",
    "signer_lane_mismatch_rechecksummed",
}

REQUIRED_CRASH_IDS = {
    "before_prepared_temp_create",
    "after_prepared_temp_create",
    "after_prepared_partial_write",
    "after_prepared_complete_write_and_checksum",
    "before_prepared_temp_fsync",
    "after_prepared_temp_fsync",
    "before_prepared_rename",
    "after_prepared_rename_before_parent_fsync",
    "after_prepared_parent_fsync_before_send",
    "before_submit_and_watch",
    "after_submit_and_watch_begins",
    "watcher_loss",
    "response_loss",
    "before_resolution_temp_create",
    "after_resolution_temp_create",
    "after_resolution_partial_write",
    "after_resolution_complete_write_and_checksum",
    "after_resolution_temp_fsync",
    "after_resolution_rename_before_parent_fsync",
    "after_resolution_parent_fsync",
    "after_terminal_response_before_remove",
    "after_remove_before_parent_fsync",
    "after_remove_parent_fsync",
}

RECONCILIATION_EXPECTED = {
    "exact_finalized_success_one_matching_event": ("accept", "finalized_accepted", "publish_state_1"),
    "exact_finalized_dispatch_failure": ("accept", "finalized_dispatch_rejected", "publish_state_2"),
    "successful_inclusion_zero_matching_events": ("accept", "finalized_invariant_failed", "publish_state_3"),
    "successful_inclusion_multiple_matching_events": ("accept", "finalized_invariant_failed", "publish_state_3"),
    "successful_inclusion_wrong_deployment_event": ("accept", "finalized_invariant_failed", "publish_state_3"),
    "successful_inclusion_wrong_version_event": ("accept", "finalized_invariant_failed", "publish_state_3"),
    "successful_inclusion_wrong_signer_event": ("accept", "finalized_invariant_failed", "publish_state_3"),
    "successful_inclusion_wrong_call_event": ("accept", "finalized_invariant_failed", "publish_state_3"),
    "complete_birth_through_death_absence": ("accept", "expired_not_included", "publish_state_4"),
    "terminal_recovery_exact_unique_evidence": ("accept", "reproduce_persisted_terminal_operation_and_chain_effect", "remove_only_after_response_boundary"),
    "expired_recovery_revalidates_head_and_absence": ("accept", "expired_not_included", "remove_only_after_response_boundary"),
    "finalized_head_before_death": ("reject", "submission_lane_unresolved", "retain_prepared"),
    "finalized_head_equal_death": ("reject", "submission_lane_unresolved", "retain_prepared"),
    "post_death_scan_missing_birth": ("reject", "submission_lane_unresolved", "retain_prepared"),
    "post_death_scan_missing_interior": ("reject", "submission_lane_unresolved", "retain_prepared"),
    "post_death_scan_missing_death": ("reject", "submission_lane_unresolved", "retain_prepared"),
    "post_death_scan_finds_exact_hash": ("reject", "reconcile_exact_inclusion_not_expiry", "retain_until_exact_evidence"),
    "era_expiry_alone": ("reject", "submission_lane_unresolved", "retain_prepared"),
    "nonce_moved": ("reject", "submission_lane_unresolved", "retain_prepared"),
    "sqlite_claims_acceptance": ("reject", "submission_lane_unresolved", "retain_prepared"),
    "watcher_invalid": ("reject", "delivery_indeterminate", "retain_prepared"),
    "watcher_dropped": ("reject", "delivery_indeterminate", "retain_prepared"),
    "watcher_error": ("reject", "delivery_indeterminate", "retain_prepared"),
    "watcher_stream_end": ("reject", "delivery_indeterminate", "retain_prepared"),
    "watcher_unknown_status": ("reject", "delivery_indeterminate", "retain_prepared"),
    "watch_timeout": ("reject", "delivery_indeterminate", "retain_prepared"),
    "transport_or_response_loss": ("reject", "delivery_indeterminate", "retain_prepared"),
    "terminal_evidence_unavailable": ("reject", "submission_lane_unresolved", "retain_terminal"),
    "terminal_evidence_duplicate_hash": ("reject", "submission_lane_unresolved", "retain_terminal"),
    "terminal_evidence_mismatch": ("reject", "submission_lane_unresolved", "retain_terminal"),
    "incoming_operation_b_cannot_replace_a": ("reject", "reconcile_complete_unit", "retain_persisted_operation"),
    "fresh_projection_may_advance": ("nonclaim", "only_stable_submission_members_repeat", "no_complete_stdout_byte_equality_claim"),
    "response_crash_may_duplicate_semantics": ("nonclaim", "semantic_response_may_repeat", "never_resubmit"),
    "alternate_projection_path": ("nonclaim", "separate_lane_outside_coordination", "nonce_disagreement_fails_closed"),
    "external_signer_user": ("nonclaim", "outside_coordination", "nonce_disagreement_fails_closed"),
    "same_user_deletes_unresolved_record": ("nonclaim", "undetectable_out_of_scope", "no_exactly_once_claim"),
    "lying_hardware_power_loss": ("nonclaim", "outside_process_crash_contract", "no_durability_claim"),
    "signer_is_not_attribution": ("nonclaim", "no_owner_author_responsibility_or_causality", "technical_metadata_only"),
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
    fail("JSON floating-point values are forbidden")


def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            fail(f"duplicate JSON member: {key!r}")
        result[key] = value
    return result


def safe_fixture_path(relative: str) -> Path:
    if not isinstance(relative, str):
        fail("fixture path must be a string")
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
    return path


def safe_repository_path(relative: str) -> Path:
    if not isinstance(relative, str):
        fail("repository path must be a string")
    pure = PurePosixPath(relative)
    if (
        not relative
        or pure.is_absolute()
        or "\\" in relative
        or any(part in ("", ".", "..") for part in pure.parts)
        or pure.as_posix() != relative
    ):
        fail(f"unsafe repository path: {relative!r}")
    path = REPOSITORY_ROOT.joinpath(*pure.parts)
    if not path.is_file() or path.is_symlink():
        fail(f"repository path is not one regular non-symlink file: {relative}")
    return path


def load_json(relative: str) -> dict[str, Any]:
    raw = safe_fixture_path(relative).read_bytes()
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


def lower_hex(value: Any, size: int | None, label: str) -> bytes:
    if not isinstance(value, str) or value != value.lower() or len(value) % 2:
        fail(f"{label} must be even-length lowercase hex")
    try:
        decoded = bytes.fromhex(value)
    except ValueError:
        fail(f"{label} is not hexadecimal")
    if decoded.hex() != value or (size is not None and len(decoded) != size):
        fail(f"{label} has a noncanonical encoding or wrong size")
    return decoded


def read_hex(relative: str) -> bytes:
    raw = safe_fixture_path(relative).read_bytes()
    if not raw.endswith(b"\n") or raw.count(b"\n") != 1:
        fail(f"{relative} must be one LF-terminated hexadecimal line")
    try:
        text = raw[:-1].decode("ascii")
    except UnicodeDecodeError:
        fail(f"{relative} is not ASCII")
    if not text:
        fail(f"{relative} is empty")
    return lower_hex(text, None, relative)


def sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def blake2_256(raw: bytes) -> bytes:
    return hashlib.blake2b(raw, digest_size=32).digest()


def canonical_u64(value: Any, label: str) -> int:
    if not isinstance(value, str) or not value or (len(value) > 1 and value[0] == "0"):
        fail(f"{label} is not a canonical decimal string")
    if not value.isascii() or not value.isdigit():
        fail(f"{label} is not a canonical decimal string")
    number = int(value)
    if not 0 <= number <= (1 << 64) - 1 or str(number) != value:
        fail(f"{label} is outside u64")
    return number


def verify_closed_tree(manifest: dict[str, Any], inventory: dict[str, Any]) -> None:
    exact_keys(
        inventory,
        {"format", "entries", "entry_count"},
        "inventory",
    )
    if inventory["format"] != "cubikan-submission-journal-inventory-v1":
        fail("inventory format differs")
    entries = inventory["entries"]
    if not isinstance(entries, list) or inventory["entry_count"] != len(entries):
        fail("inventory count differs")
    listed: list[str] = []
    for index, entry in enumerate(entries):
        if not isinstance(entry, dict):
            fail(f"inventory entry {index} is not an object")
        exact_keys(entry, {"path", "size", "sha256", "role"}, f"inventory entry {index}")
        path = safe_fixture_path(entry["path"])
        raw = path.read_bytes()
        if not isinstance(entry["size"], int) or entry["size"] != len(raw):
            fail(f"inventory size differs for {entry['path']}")
        if entry["sha256"] != sha256(raw):
            fail(f"inventory hash differs for {entry['path']}")
        if not isinstance(entry["role"], str) or not entry["role"]:
            fail(f"inventory role missing for {entry['path']}")
        listed.append(entry["path"])
    if listed != sorted(listed) or len(listed) != len(set(listed)):
        fail("inventory paths must be sorted and unique")

    control = {
        "manifest-v1.json",
        "inventory-v1.json",
        "verify_fixtures.py",
        "SCALE-REASONING.md",
    }
    expected = control | set(listed)
    actual: set[str] = set()
    for path in FIXTURE_ROOT.rglob("*"):
        if path.is_symlink():
            fail(f"fixture tree contains a symlink: {path}")
        if path.is_file():
            actual.add(path.relative_to(FIXTURE_ROOT).as_posix())
        elif not path.is_dir():
            fail(f"fixture tree contains a non-file object: {path}")
    if actual != expected:
        fail(
            f"closed fixture tree differs: missing={sorted(expected - actual)!r}, "
            f"extra={sorted(actual - expected)!r}"
        )

    inv_pin = manifest["inventory"]
    exact_keys(inv_pin, {"path", "size", "sha256", "entry_count"}, "manifest inventory")
    inv_raw = safe_fixture_path(inv_pin["path"]).read_bytes()
    if (
        inv_pin["path"] != "inventory-v1.json"
        or inv_pin["size"] != len(inv_raw)
        or inv_pin["sha256"] != sha256(inv_raw)
        or inv_pin["entry_count"] != len(entries)
    ):
        fail("manifest inventory pin differs")


def verify_manifest() -> tuple[dict[str, Any], dict[str, Any]]:
    manifest = load_json("manifest-v1.json")
    exact_keys(
        manifest,
        {
            "format",
            "fixture_root",
            "authority",
            "inventory",
            "control_documents",
            "external_artifacts",
            "summary",
            "verifier",
        },
        "manifest",
    )
    if (
        manifest["format"] != "cubikan-submission-journal-manifest-v1"
        or manifest["fixture_root"] != "tests/fixtures/submission-journal-v1"
    ):
        fail("manifest identity differs")
    authority = manifest["authority"]
    exact_keys(
        authority,
        {
            "independent_oracle",
            "production_may_regenerate",
            "synthetic_only",
            "alice_authorized_submitter",
            "authorized_production_submitters",
        },
        "manifest authority",
    )
    if authority != {
        "independent_oracle": True,
        "production_may_regenerate": False,
        "synthetic_only": True,
        "alice_authorized_submitter": False,
        "authorized_production_submitters": ["Charlie", "Dave"],
    }:
        fail("manifest authority differs")

    inventory = load_json("inventory-v1.json")
    verify_closed_tree(manifest, inventory)

    control_expected = {
        "scale_reasoning": "SCALE-REASONING.md",
        "verifier": "verify_fixtures.py",
    }
    controls = manifest["control_documents"]
    if not isinstance(controls, dict) or set(controls) != set(control_expected):
        fail("manifest control document inventory differs")
    for name, expected_path in control_expected.items():
        pin = controls[name]
        exact_keys(pin, {"path", "size", "sha256"}, f"control {name}")
        raw = safe_fixture_path(pin["path"]).read_bytes()
        if pin["path"] != expected_path or pin["size"] != len(raw) or pin["sha256"] != sha256(raw):
            fail(f"control document pin differs for {name}")

    external_expected = {
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
    external = manifest["external_artifacts"]
    if not isinstance(external, dict) or set(external) != set(external_expected):
        fail("external artifact inventory differs")
    for name, expected in external_expected.items():
        pin = external[name]
        exact_keys(pin, {"path", "size", "sha256"}, f"external {name}")
        path, size, digest = expected
        raw = safe_repository_path(path).read_bytes()
        if pin != {"path": path, "size": size, "sha256": digest}:
            fail(f"external manifest pin differs for {name}")
        if len(raw) != size or sha256(raw) != digest:
            fail(f"external artifact bytes differ for {name}")

    verifier = manifest["verifier"]
    exact_keys(verifier, {"path", "command", "requires"}, "manifest verifier")
    if verifier != {
        "path": "verify_fixtures.py",
        "command": "PYTHONDONTWRITEBYTECODE=1 python3 tests/fixtures/submission-journal-v1/verify_fixtures.py",
        "requires": "Python 3.10+ standard library only",
    }:
        fail("manifest verifier command differs")
    return manifest, inventory


def canonical_unix_path(raw: bytes) -> bool:
    if not raw or raw[0] != 0x2F or b"\0" in raw or len(raw) > (1 << 32) - 1:
        return False
    if raw == b"/":
        return True
    if raw.endswith(b"/"):
        return False
    return all(component not in (b"", b".", b"..") for component in raw[1:].split(b"/"))


def lane_basename(digest: str, kind: str) -> str:
    suffix = {"lock": "lock", "journal": "journal", "temporary": "tmp"}[kind]
    return f"cubikan-submission-{digest}.{suffix}"


def verify_lane_paths() -> None:
    vectors = load_json("lane-path-vectors-v1.json")
    exact_keys(
        vectors,
        {"format", "domain_hex", "accepted", "rejected_paths", "rejected_basenames"},
        "lane path vectors",
    )
    if (
        vectors["format"] != "cubikan-submission-lane-path-vectors-v1"
        or lower_hex(vectors["domain_hex"], None, "lane domain") != LANE_DOMAIN
    ):
        fail("lane vector identity differs")
    accepted = vectors["accepted"]
    if not isinstance(accepted, list) or len(accepted) != 4:
        fail("accepted lane vector count differs")
    ids: set[str] = set()
    for index, vector in enumerate(accepted):
        exact_keys(
            vector,
            {"id", "path_hex", "path_length", "deployment_id", "signer", "digest", "basenames"},
            f"lane vector {index}",
        )
        if vector["id"] in ids:
            fail("duplicate lane vector id")
        ids.add(vector["id"])
        path = lower_hex(vector["path_hex"], None, f"lane path {index}")
        deployment = lower_hex(vector["deployment_id"], 32, f"lane deployment {index}")
        signer = lower_hex(vector["signer"], 32, f"lane signer {index}")
        if not canonical_unix_path(path) or vector["path_length"] != len(path):
            fail(f"lane path is not canonical for {vector['id']}")
        digest = sha256(LANE_DOMAIN + len(path).to_bytes(4, "big") + path + deployment + signer)
        if vector["digest"] != digest:
            fail(f"lane digest differs for {vector['id']}")
        basenames = vector["basenames"]
        exact_keys(basenames, {"lock", "journal", "temporary"}, f"lane basenames {index}")
        expected = {kind: lane_basename(digest, kind) for kind in basenames}
        if basenames != expected:
            fail(f"lane basenames differ for {vector['id']}")
        if any("/" in basename or "\\" in basename or "\0" in basename for basename in basenames.values()):
            fail(f"lane basename is not a direct child for {vector['id']}")
    if ids != {
        "primary-asymmetric",
        "last-path-byte-differs",
        "raw-non-utf8-path",
        "deployment-signer-order-sentinel",
    }:
        fail("lane accepted vector ids differ")

    rejected_paths = vectors["rejected_paths"]
    if not isinstance(rejected_paths, list) or len(rejected_paths) != 8:
        fail("rejected path vector count differs")
    rejected_ids: set[str] = set()
    for case in rejected_paths:
        if not isinstance(case, dict):
            fail("rejected path case is not an object")
        if "path_hex" in case:
            exact_keys(case, {"id", "path_hex", "reason"}, f"rejected path {case.get('id')}")
            path = lower_hex(case["path_hex"], None, f"rejected path {case['id']}")
            if canonical_unix_path(path):
                fail(f"rejected path was canonical: {case['id']}")
        else:
            exact_keys(case, {"id", "virtual_length", "reason"}, f"rejected path {case.get('id')}")
            if case["virtual_length"] != 1 << 32 or case["reason"] != "length_overflow":
                fail("virtual length overflow vector differs")
        rejected_ids.add(case["id"])
    if len(rejected_ids) != len(rejected_paths):
        fail("rejected path ids are not unique")

    primary = accepted[0]
    rejected_basenames = vectors["rejected_basenames"]
    if not isinstance(rejected_basenames, list) or len(rejected_basenames) != 5:
        fail("rejected basename vector count differs")
    for case in rejected_basenames:
        exact_keys(case, {"id", "kind", "value"}, f"rejected basename {case.get('id')}")
        if case["kind"] not in ("lock", "journal", "temporary"):
            fail("rejected basename kind differs")
        if case["value"] == primary["basenames"][case["kind"]]:
            fail(f"rejected basename equals canonical value: {case['id']}")


def parse_record(raw: bytes) -> dict[str, Any]:
    if len(raw) != 256:
        fail("journal record is not exactly 256 bytes")
    if raw[0:8] != b"CUBKJNL1":
        fail("journal magic differs")
    if int.from_bytes(raw[8:10], "big") != 1:
        fail("journal version differs")
    state = raw[10]
    if state >= len(STATE_NAMES):
        fail("journal state is unknown")
    if raw[11] != 0 or raw[14:16] != b"\0\0" or raw[217:224] != bytes(7):
        fail("journal flags or reserved bytes are nonzero")
    if int.from_bytes(raw[12:14], "big") != 256:
        fail("journal declared length differs")
    if raw[16:48] == ZERO_HASH or raw[48:80] == ZERO_HASH:
        fail("journal deployment or signer is zero")
    if raw[88:120] == ZERO_HASH or raw[128:160] == ZERO_HASH:
        fail("journal extrinsic or signing hash is zero")
    signing = int.from_bytes(raw[120:128], "big")
    birth = int.from_bytes(raw[160:168], "big")
    death = int.from_bytes(raw[168:176], "big")
    if birth != signing or birth > (1 << 64) - 64 or death != birth + 63:
        fail("journal era coordinates differ")
    resolution_number = int.from_bytes(raw[176:184], "big")
    resolution_hash = raw[184:216]
    if state == 0:
        if resolution_number != 0 or resolution_hash != ZERO_HASH:
            fail("prepared record has a resolution coordinate")
    elif state in (1, 2, 3):
        if resolution_hash == ZERO_HASH or not birth <= resolution_number <= death:
            fail("finalized resolution coordinate differs")
    elif resolution_hash == ZERO_HASH or resolution_number <= death:
        fail("expiry resolution coordinate is not post-death")
    operation = raw[216]
    if operation >= len(OPERATION_NAMES):
        fail("journal operation is unknown")
    expected_checksum = hashlib.sha256(JOURNAL_DOMAIN + raw[:224]).digest()
    if raw[224:256] != expected_checksum:
        fail("journal checksum differs")
    return {
        "state": STATE_NAMES[state],
        "deployment_id": raw[16:48],
        "signer": raw[48:80],
        "nonce": int.from_bytes(raw[80:88], "big"),
        "extrinsic_hash": raw[88:120],
        "signing_finalized_number": signing,
        "signing_finalized_hash": raw[128:160],
        "birth": birth,
        "death": death,
        "resolution_number": resolution_number,
        "resolution_hash": resolution_hash,
        "mutation_operation": OPERATION_NAMES[operation],
    }


def verify_journal_vectors() -> dict[str, bytes]:
    vectors = load_json("journal-vectors-v1.json")
    exact_keys(
        vectors,
        {"format", "record_size", "checksum_domain_hex", "layout", "states", "operations", "common", "records"},
        "journal vectors",
    )
    if (
        vectors["format"] != "cubikan-submission-journal-vectors-v1"
        or vectors["record_size"] != 256
        or lower_hex(vectors["checksum_domain_hex"], None, "journal domain") != JOURNAL_DOMAIN
    ):
        fail("journal vector identity differs")
    layout = vectors["layout"]
    if not isinstance(layout, list) or len(layout) != len(LAYOUT):
        fail("journal layout count differs")
    for index, (actual, expected) in enumerate(zip(layout, LAYOUT, strict=True)):
        exact_keys(actual, {"name", "start", "end", "encoding"}, f"layout {index}")
        if (actual["name"], actual["start"], actual["end"], actual["encoding"]) != expected:
            fail(f"journal layout differs at {index}")
    if [(s.get("tag"), s.get("name")) for s in vectors["states"]] != list(enumerate(STATE_NAMES)):
        fail("journal state registry differs")
    if [(o.get("tag"), o.get("name")) for o in vectors["operations"]] != list(enumerate(OPERATION_NAMES)):
        fail("journal operation registry differs")

    common = vectors["common"]
    exact_keys(
        common,
        {
            "deployment_id",
            "signer",
            "nonce",
            "extrinsic_hash",
            "signing_finalized_number",
            "signing_finalized_hash",
            "birth",
            "death",
            "mutation_operation",
        },
        "journal common",
    )
    expected_common = {
        "deployment_id": DEPLOYMENT,
        "signer": ALICE,
        "nonce": 66051,
        "extrinsic_hash": EXTRINSIC_HASH,
        "signing_finalized_number": 131,
        "signing_finalized_hash": SIGNING_HASH,
        "birth": 131,
        "death": 194,
        "mutation_operation": "complete_unit",
    }
    actual_common = {
        "deployment_id": lower_hex(common["deployment_id"], 32, "journal deployment"),
        "signer": lower_hex(common["signer"], 32, "journal signer"),
        "nonce": canonical_u64(common["nonce"], "journal nonce"),
        "extrinsic_hash": lower_hex(common["extrinsic_hash"], 32, "journal extrinsic hash"),
        "signing_finalized_number": canonical_u64(common["signing_finalized_number"], "journal signing number"),
        "signing_finalized_hash": lower_hex(common["signing_finalized_hash"], 32, "journal signing hash"),
        "birth": canonical_u64(common["birth"], "journal birth"),
        "death": canonical_u64(common["death"], "journal death"),
        "mutation_operation": common["mutation_operation"],
    }
    if actual_common != expected_common:
        fail("journal common vector differs")

    records = vectors["records"]
    if not isinstance(records, list) or len(records) != 5:
        fail("journal record vector count differs")
    by_name: dict[str, bytes] = {}
    for index, record in enumerate(records):
        exact_keys(
            record,
            {"state", "path", "decoded_size", "decoded_sha256", "resolution_number", "resolution_hash"},
            f"journal record {index}",
        )
        if record["state"] != STATE_NAMES[index]:
            fail("journal record order differs")
        raw = read_hex(record["path"])
        if record["decoded_size"] != 256 or len(raw) != 256 or record["decoded_sha256"] != sha256(raw):
            fail(f"journal record bytes differ for {record['state']}")
        parsed = parse_record(raw)
        for name, expected in expected_common.items():
            if parsed[name] != expected:
                fail(f"journal common field {name} differs for {record['state']}")
        if (
            parsed["state"] != record["state"]
            or parsed["resolution_number"] != canonical_u64(record["resolution_number"], "resolution number")
            or parsed["resolution_hash"] != lower_hex(record["resolution_hash"], 32, "resolution hash")
        ):
            fail(f"journal resolution differs for {record['state']}")
        by_name[record["state"]] = raw
    return by_name


def apply_mutation(base: bytes, mutation: dict[str, Any], label: str) -> bytes:
    if not isinstance(mutation, dict) or "kind" not in mutation:
        fail(f"{label} mutation is malformed")
    kind = mutation["kind"]
    if kind == "truncate":
        exact_keys(mutation, {"kind", "length"}, label)
        length = mutation["length"]
        if not isinstance(length, int) or not 0 <= length < len(base):
            fail(f"{label} truncate length differs")
        return base[:length]
    if kind == "append":
        exact_keys(mutation, {"kind", "hex"}, label)
        suffix = lower_hex(mutation["hex"], None, label)
        if not suffix:
            fail(f"{label} append is empty")
        return base + suffix
    if kind == "xor":
        exact_keys(mutation, {"kind", "offset", "value"}, label)
        offset = mutation["offset"]
        value = mutation["value"]
        if not isinstance(offset, int) or not 0 <= offset < len(base) or not isinstance(value, int) or not 1 <= value <= 255:
            fail(f"{label} xor differs")
        result = bytearray(base)
        result[offset] ^= value
        return bytes(result)
    if kind == "patch":
        exact_keys(mutation, {"kind", "patches", "recompute_checksum"}, label)
        patches = mutation["patches"]
        if not isinstance(patches, list) or not patches:
            fail(f"{label} patches differ")
        result = bytearray(base)
        occupied: set[int] = set()
        for patch in patches:
            exact_keys(patch, {"offset", "hex"}, f"{label} patch")
            offset = patch["offset"]
            replacement = lower_hex(patch["hex"], None, f"{label} patch")
            if not isinstance(offset, int) or not replacement or offset < 0 or offset + len(replacement) > 224:
                fail(f"{label} patch range differs")
            patch_range = set(range(offset, offset + len(replacement)))
            if occupied & patch_range:
                fail(f"{label} patches overlap")
            occupied |= patch_range
            result[offset : offset + len(replacement)] = replacement
        if mutation["recompute_checksum"] is not True:
            fail(f"{label} patch must explicitly recompute checksum")
        result[224:256] = hashlib.sha256(JOURNAL_DOMAIN + result[:224]).digest()
        return bytes(result)
    fail(f"{label} mutation kind is unknown")


def verify_rejections(records: dict[str, bytes]) -> int:
    document = load_json("rejection-cases-v1.json")
    exact_keys(document, {"format", "mutation_grammar", "cases"}, "rejection cases")
    if document["format"] != "cubikan-submission-journal-rejection-cases-v1":
        fail("rejection case format differs")
    grammar = document["mutation_grammar"]
    exact_keys(grammar, {"patch", "xor", "truncate", "append"}, "mutation grammar")
    cases = document["cases"]
    if not isinstance(cases, list):
        fail("rejection cases are not a list")
    ids: set[str] = set()
    for case in cases:
        exact_keys(case, {"id", "base", "mutation", "validation", "reason"}, f"rejection {case.get('id')}")
        if case["id"] in ids or case["base"] not in records or case["validation"] not in ("codec", "lane"):
            fail(f"rejection case identity differs: {case['id']}")
        ids.add(case["id"])
        mutated = apply_mutation(records[case["base"]], case["mutation"], case["id"])
        try:
            parsed = parse_record(mutated)
        except ValueError:
            if case["validation"] != "codec":
                fail(f"lane rejection failed prematurely in codec: {case['id']}")
            continue
        if case["validation"] != "lane":
            fail(f"codec accepted rejection case: {case['id']}")
        if parsed["deployment_id"] == DEPLOYMENT and parsed["signer"] == ALICE:
            fail(f"lane rejection did not mismatch lane identity: {case['id']}")
    if ids != REQUIRED_REJECTION_IDS:
        fail(
            f"rejection registry differs: missing={sorted(REQUIRED_REJECTION_IDS - ids)!r}, "
            f"extra={sorted(ids - REQUIRED_REJECTION_IDS)!r}"
        )
    return len(cases)


def verify_transitions() -> None:
    document = load_json("transitions-v1.json")
    exact_keys(document, {"format", "state_order", "allowed_matrix", "allowed", "forbidden_invariants"}, "transitions")
    if document["format"] != "cubikan-submission-journal-transitions-v1":
        fail("transition format differs")
    state_order = ["absent", *STATE_NAMES]
    if document["state_order"] != state_order:
        fail("transition state order differs")
    allowed_set = {
        ("absent", "prepared"),
        *(("prepared", state) for state in STATE_NAMES[1:]),
        *((state, "absent") for state in STATE_NAMES[1:]),
    }
    matrix = document["allowed_matrix"]
    if not isinstance(matrix, dict) or list(matrix) != state_order:
        fail("transition matrix rows differ")
    observed: set[tuple[str, str]] = set()
    for source in state_order:
        row = matrix[source]
        if not isinstance(row, list) or len(row) != len(state_order) or any(type(v) is not bool for v in row):
            fail(f"transition matrix row differs for {source}")
        for target, is_allowed in zip(state_order, row, strict=True):
            if is_allowed:
                observed.add((source, target))
            if is_allowed != ((source, target) in allowed_set):
                fail(f"transition matrix cell differs: {source}->{target}")
    if observed != allowed_set:
        fail("transition allowed set differs")
    allowed = document["allowed"]
    if not isinstance(allowed, list) or len(allowed) != len(allowed_set):
        fail("transition allowed details count differs")
    detailed: set[tuple[str, str]] = set()
    for transition in allowed:
        exact_keys(transition, {"from", "to", "boundary"}, "allowed transition")
        pair = (transition["from"], transition["to"])
        if pair not in allowed_set or pair in detailed or not isinstance(transition["boundary"], str) or not transition["boundary"]:
            fail(f"allowed transition details differ: {pair}")
        detailed.add(pair)
    if detailed != allowed_set:
        fail("allowed transition details are incomplete")
    if document["forbidden_invariants"] != [
        "prepared_to_absent_without_proven_resolution",
        "resolved_to_resolved",
        "resolved_to_prepared",
        "incoming_operation_may_not_replace_persisted_operation",
    ]:
        fail("forbidden transition invariants differ")


def verify_crash_points() -> int:
    document = load_json("crash-points-v1.json")
    exact_keys(document, {"format", "global_invariants", "points"}, "crash points")
    if document["format"] != "cubikan-submission-journal-crash-points-v1":
        fail("crash point format differs")
    invariants = document["global_invariants"]
    exact_keys(
        invariants,
        {"journal_visibility", "temporary_visibility", "restart_rule", "orphan_limit", "durability_scope", "excluded"},
        "crash invariants",
    )
    if invariants != {
        "journal_visibility": "only_absent_old_or_new_complete_checksummed_record",
        "temporary_visibility": "at_most_one_fixed_derived_owner_0600_regular_torn_or_complete_temp_removed_under_lock_then_parent_fsync",
        "restart_rule": "no_send_before_safe_republication_or_exact_hash_reconciliation",
        "orphan_limit": 1,
        "durability_scope": "supported_linux_local_filesystem_process_crash",
        "excluded": "lying_hardware_power_loss",
    }:
        fail("crash global invariants differ")
    points = document["points"]
    if not isinstance(points, list):
        fail("crash points are not a list")
    ids: set[str] = set()
    journal_values = {"absent", "prepared", "same_terminal"}
    temp_values = {"absent", "owner_0600_regular_torn", "owner_0600_regular_complete"}
    phases = {"prepared_publication", "prepared_durable", "send", "watch", "response", "resolution_publication", "resolution_durable", "removal", "clean"}
    by_id: dict[str, dict[str, Any]] = {}
    for point in points:
        exact_keys(point, {"id", "phase", "admissible_journal", "admissible_temp", "delivery_possible"}, f"crash point {point.get('id')}")
        if point["id"] in ids or point["phase"] not in phases or type(point["delivery_possible"]) is not bool:
            fail(f"crash point identity differs: {point['id']}")
        if (
            not isinstance(point["admissible_journal"], list)
            or not point["admissible_journal"]
            or len(point["admissible_journal"]) != len(set(point["admissible_journal"]))
            or not set(point["admissible_journal"]) <= journal_values
            or not isinstance(point["admissible_temp"], list)
            or not point["admissible_temp"]
            or len(point["admissible_temp"]) != len(set(point["admissible_temp"]))
            or not set(point["admissible_temp"]) <= temp_values
        ):
            fail(f"crash point visibility differs: {point['id']}")
        ids.add(point["id"])
        by_id[point["id"]] = point
    if ids != REQUIRED_CRASH_IDS:
        fail(
            f"crash registry differs: missing={sorted(REQUIRED_CRASH_IDS - ids)!r}, "
            f"extra={sorted(ids - REQUIRED_CRASH_IDS)!r}"
        )
    if by_id["after_prepared_parent_fsync_before_send"]["admissible_journal"] != ["prepared"]:
        fail("prepared durability point differs")
    if by_id["after_submit_and_watch_begins"]["delivery_possible"] is not True:
        fail("send boundary delivery possibility differs")
    if by_id["after_resolution_parent_fsync"]["admissible_journal"] != ["same_terminal"]:
        fail("resolution durability point differs")
    if by_id["after_remove_parent_fsync"]["admissible_journal"] != ["absent"]:
        fail("removal durability point differs")
    return len(points)


def verify_reconciliation() -> int:
    document = load_json("reconciliation-cases-v1.json")
    exact_keys(document, {"format", "persisted_operation", "incoming_operation_sentinel", "cases"}, "reconciliation")
    if (
        document["format"] != "cubikan-submission-reconciliation-cases-v1"
        or document["persisted_operation"] != "complete_unit"
        or document["incoming_operation_sentinel"] != "create_relationship"
    ):
        fail("reconciliation identity differs")
    cases = document["cases"]
    if not isinstance(cases, list):
        fail("reconciliation cases are not a list")
    observed: dict[str, tuple[str, str, str]] = {}
    preconditions: set[str] = set()
    for case in cases:
        exact_keys(case, {"id", "kind", "precondition", "outcome", "journal_action"}, f"reconciliation {case.get('id')}")
        if case["id"] in observed or not isinstance(case["precondition"], str) or not case["precondition"]:
            fail(f"reconciliation case identity differs: {case['id']}")
        if case["precondition"] in preconditions:
            fail(f"duplicate reconciliation precondition: {case['precondition']}")
        preconditions.add(case["precondition"])
        observed[case["id"]] = (case["kind"], case["outcome"], case["journal_action"])
    if observed != RECONCILIATION_EXPECTED:
        fail(
            f"reconciliation registry differs: missing={sorted(set(RECONCILIATION_EXPECTED) - set(observed))!r}, "
            f"extra={sorted(set(observed) - set(RECONCILIATION_EXPECTED))!r}"
        )
    kinds = [case["kind"] for case in cases]
    if kinds.count("accept") != 11 or kinds.count("reject") != 20 or kinds.count("nonclaim") != 7:
        fail("reconciliation accept/reject/nonclaim counts differ")
    return len(cases)


def compact_encode(value: int) -> bytes:
    if not 0 <= value < 1 << 536:
        fail("compact value out of supported range")
    if value < 1 << 6:
        return bytes([value << 2])
    if value < 1 << 14:
        return ((value << 2) | 1).to_bytes(2, "little")
    if value < 1 << 30:
        return ((value << 2) | 2).to_bytes(4, "little")
    width = max(4, (value.bit_length() + 7) // 8)
    return bytes([((width - 4) << 2) | 3]) + value.to_bytes(width, "little")


def compact_decode(raw: bytes, offset: int) -> tuple[int, int]:
    if offset >= len(raw):
        fail("truncated compact integer")
    first = raw[offset]
    mode = first & 3
    if mode == 0:
        value, end = first >> 2, offset + 1
    elif mode == 1:
        end = offset + 2
        if end > len(raw):
            fail("truncated compact mode 1")
        value = int.from_bytes(raw[offset:end], "little") >> 2
    elif mode == 2:
        end = offset + 4
        if end > len(raw):
            fail("truncated compact mode 2")
        value = int.from_bytes(raw[offset:end], "little") >> 2
    else:
        width = (first >> 2) + 4
        end = offset + 1 + width
        if end > len(raw):
            fail("truncated compact mode 3")
        value = int.from_bytes(raw[offset + 1 : end], "little")
    if raw[offset:end] != compact_encode(value):
        fail("noncanonical compact integer")
    return value, end


def decode_era(raw: bytes) -> tuple[int, int]:
    if raw == b"\0":
        return 0, 0
    if len(raw) != 2:
        fail("mortal era is not two bytes")
    encoded = int.from_bytes(raw, "little")
    period = 2 << (encoded % 16)
    quantize = max(period >> 12, 1)
    phase = (encoded >> 4) * quantize
    if period < 4 or period > 65536 or phase >= period:
        fail("mortal era is invalid")
    return period, phase


# Minimal independent Merlin/STROBE-128 and Ristretto255 verification follows.
FIELD_P = (1 << 255) - 19
SCALAR_L = (1 << 252) + 27742317777372353535851937790883648493
EDWARDS_D = (-121665 * pow(121666, FIELD_P - 2, FIELD_P)) % FIELD_P
SQRT_M1 = pow(2, (FIELD_P - 1) // 4, FIELD_P)
if SQRT_M1 & 1:
    SQRT_M1 = FIELD_P - SQRT_M1
RISTRETTO_BASEPOINT = bytes.fromhex(
    "e2f2ae0a6abc4e71a884a961c500515f58e30b6aa582dd8db6a65945e08d2d76"
)
KECCAK_RC = (
    0x0000000000000001, 0x0000000000008082, 0x800000000000808A,
    0x8000000080008000, 0x000000000000808B, 0x0000000080000001,
    0x8000000080008081, 0x8000000000008009, 0x000000000000008A,
    0x0000000000000088, 0x0000000080008009, 0x000000008000000A,
    0x000000008000808B, 0x800000000000008B, 0x8000000000008089,
    0x8000000000008003, 0x8000000000008002, 0x8000000000000080,
    0x000000000000800A, 0x800000008000000A, 0x8000000080008081,
    0x8000000000008080, 0x0000000080000001, 0x8000000080008008,
)
KECCAK_RHO = (1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 2, 14, 27, 41, 56, 8, 25, 43, 62, 18, 39, 61, 20, 44)
KECCAK_PI = (10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4, 15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1)
MASK64 = (1 << 64) - 1


def rotate_left_64(value: int, amount: int) -> int:
    return ((value << amount) | (value >> (64 - amount))) & MASK64


def keccak_f1600(state: bytearray) -> None:
    lanes = [int.from_bytes(state[index * 8 : index * 8 + 8], "little") for index in range(25)]
    for round_constant in KECCAK_RC:
        columns = [lanes[x] ^ lanes[5 + x] ^ lanes[10 + x] ^ lanes[15 + x] ^ lanes[20 + x] for x in range(5)]
        for x in range(5):
            delta = columns[(x + 4) % 5] ^ rotate_left_64(columns[(x + 1) % 5], 1)
            for y in range(5):
                lanes[5 * y + x] ^= delta
        last = lanes[1]
        for index in range(24):
            saved = lanes[KECCAK_PI[index]]
            lanes[KECCAK_PI[index]] = rotate_left_64(last, KECCAK_RHO[index])
            last = saved
        for y in range(5):
            row = lanes[5 * y : 5 * y + 5]
            for x in range(5):
                lanes[5 * y + x] = row[x] ^ ((~row[(x + 1) % 5]) & row[(x + 2) % 5])
        lanes[0] ^= round_constant
    for index, lane in enumerate(lanes):
        state[index * 8 : index * 8 + 8] = (lane & MASK64).to_bytes(8, "little")


class Strobe128:
    def __init__(self, protocol_label: bytes):
        self.state = bytearray(200)
        self.state[:18] = bytes([1, 168, 1, 0, 1, 96]) + b"STROBEv1.0.2"
        keccak_f1600(self.state)
        self.position = 0
        self.position_begin = 0
        self.current_flags = 0
        self.meta_ad(protocol_label, False)

    def run_f(self) -> None:
        self.state[self.position] ^= self.position_begin
        self.state[self.position + 1] ^= 0x04
        self.state[167] ^= 0x80
        keccak_f1600(self.state)
        self.position = 0
        self.position_begin = 0

    def absorb(self, data: bytes) -> None:
        for byte in data:
            self.state[self.position] ^= byte
            self.position += 1
            if self.position == 166:
                self.run_f()

    def begin(self, flags: int, more: bool) -> None:
        if more:
            if self.current_flags != flags:
                fail("STROBE continuation flags differ")
            return
        old_begin = self.position_begin
        self.position_begin = self.position + 1
        self.current_flags = flags
        self.absorb(bytes([old_begin, flags]))
        if flags & (4 | 32) and self.position != 0:
            self.run_f()

    def meta_ad(self, data: bytes, more: bool = False) -> None:
        self.begin(18, more)
        self.absorb(data)

    def ad(self, data: bytes) -> None:
        self.begin(2, False)
        self.absorb(data)

    def prf(self, size: int) -> bytes:
        self.begin(7, False)
        output = bytearray(size)
        for index in range(size):
            output[index] = self.state[self.position]
            self.state[self.position] = 0
            self.position += 1
            if self.position == 166:
                self.run_f()
        return bytes(output)


class MerlinTranscript:
    def __init__(self, label: bytes):
        self.strobe = Strobe128(b"Merlin v1.0")
        self.append(b"dom-sep", label)

    def append(self, label: bytes, message: bytes) -> None:
        if len(message) > (1 << 32) - 1:
            fail("Merlin message is too long")
        self.strobe.meta_ad(label)
        self.strobe.meta_ad(len(message).to_bytes(4, "little"), True)
        self.strobe.ad(message)

    def challenge(self, label: bytes, size: int) -> bytes:
        self.strobe.meta_ad(label)
        self.strobe.meta_ad(size.to_bytes(4, "little"), True)
        return self.strobe.prf(size)


Point = tuple[int, int, int, int]
IDENTITY: Point = (0, 1, 1, 0)


def inverse_sqrt(value: int) -> tuple[bool, int]:
    if value == 0:
        return False, 0
    inverse = pow(value, FIELD_P - 2, FIELD_P)
    root = pow(inverse, (FIELD_P + 3) // 8, FIELD_P)
    if root * root % FIELD_P != inverse:
        root = root * SQRT_M1 % FIELD_P
    ok = root * root % FIELD_P == inverse
    if root & 1:
        root = FIELD_P - root
    return ok, root


def ristretto_decompress(raw: bytes) -> Point:
    if len(raw) != 32:
        fail("Ristretto encoding length differs")
    scalar = int.from_bytes(raw, "little")
    if scalar >= FIELD_P or scalar & 1:
        fail("Ristretto encoding is noncanonical or negative")
    square = scalar * scalar % FIELD_P
    u1 = (1 - square) % FIELD_P
    u2 = (1 + square) % FIELD_P
    u2_square = u2 * u2 % FIELD_P
    v = (-EDWARDS_D * u1 * u1 - u2_square) % FIELD_P
    ok, inverse = inverse_sqrt(v * u2_square % FIELD_P)
    dx = inverse * u2 % FIELD_P
    dy = inverse * dx % FIELD_P * v % FIELD_P
    x = 2 * scalar * dx % FIELD_P
    if x & 1:
        x = FIELD_P - x
    y = u1 * dy % FIELD_P
    t = x * y % FIELD_P
    if not ok or t & 1 or y == 0:
        fail("Ristretto encoding does not decompress")
    return x, y, 1, t


def point_add(left: Point, right: Point) -> Point:
    x1, y1, z1, t1 = left
    x2, y2, z2, t2 = right
    a = (y1 - x1) * (y2 - x2) % FIELD_P
    b = (y1 + x1) * (y2 + x2) % FIELD_P
    c = 2 * EDWARDS_D * t1 * t2 % FIELD_P
    d = 2 * z1 * z2 % FIELD_P
    e = (b - a) % FIELD_P
    f = (d - c) % FIELD_P
    g = (d + c) % FIELD_P
    h = (b + a) % FIELD_P
    return e * f % FIELD_P, g * h % FIELD_P, f * g % FIELD_P, e * h % FIELD_P


def point_double(point: Point) -> Point:
    x, y, z, _t = point
    a = x * x % FIELD_P
    b = y * y % FIELD_P
    c = 2 * z * z % FIELD_P
    d = -a % FIELD_P
    e = ((x + y) * (x + y) - a - b) % FIELD_P
    g = (d + b) % FIELD_P
    f = (g - c) % FIELD_P
    h = (d - b) % FIELD_P
    return e * f % FIELD_P, g * h % FIELD_P, f * g % FIELD_P, e * h % FIELD_P


def point_multiply(scalar: int, point: Point) -> Point:
    result = IDENTITY
    addend = point
    while scalar:
        if scalar & 1:
            result = point_add(result, addend)
        addend = point_double(addend)
        scalar >>= 1
    return result


def ristretto_equal(left: Point, right: Point) -> bool:
    x1, y1, _z1, _t1 = left
    x2, y2, _z2, _t2 = right
    return (
        (x1 * y2 - y1 * x2) % FIELD_P == 0
        or (x1 * x2 - y1 * y2) % FIELD_P == 0
    )


def sr25519_verify(signature: bytes, message: bytes, public_key: bytes) -> bool:
    try:
        if len(signature) != 64 or len(public_key) != 32 or not signature[63] & 0x80:
            return False
        scalar_bytes = bytearray(signature[32:])
        scalar_bytes[31] &= 0x7F
        response = int.from_bytes(scalar_bytes, "little")
        if response >= SCALAR_L:
            return False
        public = ristretto_decompress(public_key)
        claimed_r = ristretto_decompress(signature[:32])
        transcript = MerlinTranscript(b"SigningContext")
        transcript.append(b"", b"substrate")
        transcript.append(b"sign-bytes", message)
        transcript.append(b"proto-name", b"Schnorr-sig")
        transcript.append(b"sign:pk", public_key)
        transcript.append(b"sign:R", signature[:32])
        challenge = int.from_bytes(transcript.challenge(b"sign:c", 64), "little") % SCALAR_L
        basepoint = ristretto_decompress(RISTRETTO_BASEPOINT)
        calculated = point_add(
            point_multiply(response, basepoint),
            point_multiply((-challenge) % SCALAR_L, public),
        )
        return ristretto_equal(calculated, claimed_r)
    except ValueError:
        return False


def verify_blob(blob: dict[str, Any], expected_path: str, expected_size: int, expected_hash: str, label: str) -> bytes:
    exact_keys(blob, {"path", "decoded_size", "decoded_sha256"}, label)
    raw = read_hex(blob["path"])
    if blob["path"] != expected_path or blob["decoded_size"] != expected_size or blob["decoded_sha256"] != expected_hash:
        fail(f"{label} metadata differs")
    if len(raw) != expected_size or sha256(raw) != expected_hash:
        fail(f"{label} bytes differ")
    return raw


def verify_signed_extrinsic() -> None:
    document = load_json("signed-extrinsic-v1.json")
    exact_keys(document, {"format", "authority", "runtime", "call", "parameters", "signer_payload", "signature", "signed_extrinsic"}, "signed extrinsic")
    if document["format"] != "cubikan-submission-signed-extrinsic-v1":
        fail("signed extrinsic format differs")
    authority = document["authority"]
    exact_keys(authority, {"purpose", "signer_authorized_by_local_runtime", "signer_name", "production_submitters", "nonclaim"}, "signing authority")
    if authority != {
        "purpose": "independent_offline_signature_and_scale_codec_oracle",
        "signer_authorized_by_local_runtime": False,
        "signer_name": "Alice",
        "production_submitters": ["Charlie", "Dave"],
        "nonclaim": "This vector proves bytes and cryptography only; it is not a successful authorized dispatch fixture.",
    }:
        fail("signed vector authority differs")
    runtime = document["runtime"]
    exact_keys(runtime, {"metadata_path", "metadata_size", "metadata_sha256", "genesis_hash", "spec_version", "transaction_version", "extrinsic_version"}, "signed runtime")
    if runtime != {
        "metadata_path": "chain/metadata/cubikan-runtime-v1.scale",
        "metadata_size": 63327,
        "metadata_sha256": "171a323b1e6bf0122e549eecd5f5932e672a3e0835f32edf0b8808cfefd97302",
        "genesis_hash": GENESIS.hex(),
        "spec_version": 1,
        "transaction_version": 1,
        "extrinsic_version": 4,
    }:
        fail("signed vector runtime differs")

    call_meta = document["call"]
    exact_keys(call_meta, {"pallet_name", "pallet_index", "call_name", "call_index", "command_schema_version", "intent_unit_id", "expected_revision", "path", "decoded_size", "decoded_sha256"}, "signed call")
    if (
        call_meta["pallet_name"] != "Cubikan"
        or call_meta["pallet_index"] != 50
        or call_meta["call_name"] != "complete_unit"
        or call_meta["call_index"] != 2
        or call_meta["command_schema_version"] != 1
        or call_meta["intent_unit_id"] != "00112233445566778899aabbccddeeff"
        or canonical_u64(call_meta["expected_revision"], "expected revision") != 0x0102030405060708
    ):
        fail("signed call semantics differ")
    call_blob = {key: call_meta[key] for key in ("path", "decoded_size", "decoded_sha256")}
    call = verify_blob(call_blob, "raw/signing/call.scale.hex", 28, "c03c29f74432b39e5647e0003bb5f03b8e526fb8c309ec7e3929ca1d4e27fe70", "call")
    expected_call = bytes([50, 2]) + (1).to_bytes(2, "little") + bytes.fromhex(call_meta["intent_unit_id"]) + (0x0102030405060708).to_bytes(8, "little")
    if call != expected_call:
        fail("call SCALE bytes differ")

    parameters = document["parameters"]
    exact_keys(parameters, {"account_id", "signature_scheme", "signature_context_ascii", "nonce", "period", "phase", "era_scale_hex", "signing_finalized_number", "signing_finalized_hash", "inclusive_birth", "inclusive_death", "tip"}, "signed parameters")
    nonce = canonical_u64(parameters["nonce"], "signed nonce")
    signing_number = canonical_u64(parameters["signing_finalized_number"], "signed block")
    birth = canonical_u64(parameters["inclusive_birth"], "signed birth")
    death = canonical_u64(parameters["inclusive_death"], "signed death")
    era = lower_hex(parameters["era_scale_hex"], 2, "signed era")
    period, phase = decode_era(era)
    if (
        lower_hex(parameters["account_id"], 32, "signed account") != ALICE
        or parameters["signature_scheme"] != "sr25519"
        or parameters["signature_context_ascii"] != "substrate"
        or nonce != 66051
        or parameters["period"] != period
        or parameters["phase"] != phase
        or (period, phase) != (64, 3)
        or signing_number != 131
        or signing_number % period != phase
        or lower_hex(parameters["signing_finalized_hash"], 32, "signed block hash") != SIGNING_HASH
        or birth != signing_number
        or death != birth + period - 1
        or canonical_u64(parameters["tip"], "signed tip") != 0
    ):
        fail("signed parameters differ")

    payload_meta = document["signer_payload"]
    exact_keys(payload_meta, {"path", "decoded_size", "decoded_sha256", "ordered_components", "additional_block_hash_is_encoded_in_extrinsic"}, "signer payload")
    payload_blob = {key: payload_meta[key] for key in ("path", "decoded_size", "decoded_sha256")}
    payload = verify_blob(payload_blob, "raw/signing/signer-payload.scale.hex", 107, "d008bdf5721d1c6c8871e4d286346034e65d658cdfb0cf9ffc57f48f3baa42b7", "signer payload")
    if payload_meta["ordered_components"] != ["call", "era", "compact_nonce", "compact_zero_tip", "spec_version_le", "transaction_version_le", "genesis_hash", "signing_finalized_hash"] or payload_meta["additional_block_hash_is_encoded_in_extrinsic"] is not False:
        fail("signer payload component contract differs")
    expected_payload = call + era + compact_encode(nonce) + compact_encode(0) + (1).to_bytes(4, "little") + (1).to_bytes(4, "little") + GENESIS + SIGNING_HASH
    if payload != expected_payload:
        fail("signer payload reconstruction differs")

    signature_meta = document["signature"]
    exact_keys(signature_meta, {"path", "decoded_size", "decoded_sha256", "schnorrkel_marker_bit_set", "verification"}, "signature")
    signature_blob = {key: signature_meta[key] for key in ("path", "decoded_size", "decoded_sha256")}
    signature = verify_blob(signature_blob, "raw/signing/signature.scale.hex", 64, "5eea7e2f1e8f13becda311296a15f1958362f617d4e0807c01b493f88329596a", "signature")
    if signature_meta["schnorrkel_marker_bit_set"] is not True or signature_meta["verification"] != "sr25519_verify_context_substrate" or not signature[63] & 0x80:
        fail("signature marker contract differs")
    if not sr25519_verify(signature, payload, ALICE):
        fail("independent sr25519 signature verification failed")
    mutated_payload = bytearray(payload)
    mutated_payload[0] ^= 1
    if sr25519_verify(signature, bytes(mutated_payload), ALICE):
        fail("sr25519 signature accepted a mutated payload")
    mutated_signature = bytearray(signature)
    mutated_signature[0] ^= 1
    if sr25519_verify(bytes(mutated_signature), payload, ALICE):
        fail("sr25519 verification accepted a mutated signature")

    extrinsic_meta = document["signed_extrinsic"]
    exact_keys(extrinsic_meta, {"path", "decoded_size", "decoded_sha256", "blake2_256", "outer_compact_length", "signed_version_byte", "address_variant", "signature_variant", "compact_nonce_hex", "compact_tip_hex"}, "signed extrinsic bytes")
    extrinsic_blob = {key: extrinsic_meta[key] for key in ("path", "decoded_size", "decoded_sha256")}
    extrinsic = verify_blob(extrinsic_blob, "raw/signing/signed-extrinsic.scale.hex", 136, "e8b72ddef2a52a81f623c56fca2cde55e0da1913728a4adf5ef0052fec94ee45", "signed extrinsic")
    body_length, cursor = compact_decode(extrinsic, 0)
    if body_length != extrinsic_meta["outer_compact_length"] or body_length != len(extrinsic) - cursor:
        fail("signed extrinsic outer length differs")
    if extrinsic[cursor] != extrinsic_meta["signed_version_byte"] or extrinsic[cursor] != 0x84:
        fail("signed extrinsic version differs")
    cursor += 1
    if extrinsic[cursor] != extrinsic_meta["address_variant"] or extrinsic[cursor] != 0:
        fail("signed extrinsic address variant differs")
    cursor += 1
    if extrinsic[cursor : cursor + 32] != ALICE:
        fail("signed extrinsic account differs")
    cursor += 32
    if extrinsic[cursor] != extrinsic_meta["signature_variant"] or extrinsic[cursor] != 1:
        fail("signed extrinsic signature variant differs")
    cursor += 1
    if extrinsic[cursor : cursor + 64] != signature:
        fail("signed extrinsic signature differs")
    cursor += 64
    if extrinsic[cursor : cursor + 2] != era:
        fail("signed extrinsic era differs")
    cursor += 2
    nonce_start = cursor
    decoded_nonce, cursor = compact_decode(extrinsic, cursor)
    if decoded_nonce != nonce or extrinsic[nonce_start:cursor].hex() != extrinsic_meta["compact_nonce_hex"]:
        fail("signed extrinsic nonce differs")
    tip_start = cursor
    decoded_tip, cursor = compact_decode(extrinsic, cursor)
    if decoded_tip != 0 or extrinsic[tip_start:cursor].hex() != extrinsic_meta["compact_tip_hex"]:
        fail("signed extrinsic tip differs")
    if extrinsic[cursor:] != call:
        fail("signed extrinsic call or trailing bytes differ")
    if lower_hex(extrinsic_meta["blake2_256"], 32, "extrinsic hash") != EXTRINSIC_HASH or blake2_256(extrinsic) != EXTRINSIC_HASH:
        fail("signed extrinsic BLAKE2-256 differs")
    if SIGNING_HASH in extrinsic or payload.count(SIGNING_HASH) != 1:
        fail("additional signing block hash encoding claim differs")


def verify_summary(manifest: dict[str, Any], rejection_count: int, crash_count: int, reconciliation_count: int) -> None:
    summary = manifest["summary"]
    exact_keys(summary, {"lane_vector_count", "journal_state_count", "journal_record_size", "operation_tag_count", "rejection_case_count", "transition_cell_count", "crash_point_count", "reconciliation_case_count", "signed_extrinsic_size"}, "manifest summary")
    expected = {
        "lane_vector_count": 4,
        "journal_state_count": 5,
        "journal_record_size": 256,
        "operation_tag_count": 8,
        "rejection_case_count": rejection_count,
        "transition_cell_count": 36,
        "crash_point_count": crash_count,
        "reconciliation_case_count": reconciliation_count,
        "signed_extrinsic_size": 136,
    }
    if summary != expected:
        fail("manifest summary differs")


def main() -> None:
    manifest, inventory = verify_manifest()
    verify_lane_paths()
    records = verify_journal_vectors()
    rejection_count = verify_rejections(records)
    verify_transitions()
    crash_count = verify_crash_points()
    reconciliation_count = verify_reconciliation()
    verify_signed_extrinsic()
    verify_summary(manifest, rejection_count, crash_count, reconciliation_count)
    print(
        "submission-journal-v1 fixtures verified: "
        f"{len(inventory['entries'])} inventoried files, 5 states, "
        f"{rejection_count} rejection cases, 36 transition cells, "
        f"{crash_count} crash points, {reconciliation_count} reconciliation cases"
    )


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError) as error:
        print(f"submission-journal-v1 verification failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
