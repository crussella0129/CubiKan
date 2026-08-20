#!/usr/bin/env bash
set -euo pipefail

if [[ $# -gt 1 ]] || [[ $# -eq 1 && $1 != "--locked" ]]; then
  echo "usage: $0 [--locked]" >&2
  exit 2
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/../.." && pwd)

python3 -I -S - "$repo_root" <<'PY'
from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any


ROOT = Path(sys.argv[1]).resolve(strict=True)
SCHEMA = ROOT / "protocol/v2/cubikan.schema.json"
FIXTURES = ROOT / "tests/fixtures/protocol-v2/cubikan"
MANIFEST = FIXTURES / "manifest-v1.json"
IO_ORACLE = FIXTURES / "io-v1.json"

EXPECTED_SCHEMA_SHA256 = "309697fe6e718c78ef8802861d60a660500a985c05b5a94aaba35a28fb2cb4a3"
EXPECTED_MANIFEST_SHA256 = "46eab998ec22d8c806c7f8ac347aa89efb4f69578c7a34f6ee4737fc24e97c75"
EXPECTED_IO_SHA256 = "32ed09ae7ec55005229e1a7fa1b5edc02ae1867c55bbbd6279b88048b0dd14f4"

EXPECTED_CASE_IDS = (
    "success_explicit_id_zero_operations",
    "success_omitted_id_uses_manifest_uuid",
    "success_canonical_nil_uuid",
    "success_transition",
    "success_completion",
    "success_canonical_escaping_and_utf8",
    "success_scalar_minima_and_zero_completion_phases",
    "success_scalar_maxima",
    "success_workflow_collection_maxima",
    "success_256_operations_and_history_records",
    "error_transition_unknown_target",
    "error_transition_not_allowed",
    "error_completion_phase_not_eligible",
    "error_transition_already_completed",
    "error_completion_already_completed",
    "error_operation_number_255",
    "error_malformed_truncated",
    "error_malformed_multiple_values",
    "error_malformed_invalid_utf8",
    "error_malformed_nan",
    "error_malformed_positive_infinity",
    "error_malformed_negative_infinity",
    "error_unsupported_protocol_v1",
    "error_unsupported_protocol_future",
    "error_invalid_request_top_level_array",
    "error_invalid_request_missing_protocol_version",
    "error_invalid_request_fractional_protocol_version",
    "error_invalid_request_missing_operations",
    "error_invalid_request_null_operations",
    "error_invalid_request_null_workflow",
    "error_invalid_request_null_intent_unit",
    "error_invalid_request_unknown_top_level",
    "error_invalid_request_duplicate_top_level",
    "error_invalid_request_unknown_workflow_member",
    "error_invalid_request_duplicate_workflow_member",
    "error_invalid_request_unknown_edge_member",
    "error_invalid_request_duplicate_edge_member",
    "error_invalid_request_unknown_intent_unit_member",
    "error_invalid_request_duplicate_intent_unit_member",
    "error_invalid_request_unknown_origin_member",
    "error_invalid_request_duplicate_origin_member",
    "error_invalid_request_unknown_transition_member",
    "error_invalid_request_duplicate_transition_member",
    "error_invalid_request_unknown_complete_member",
    "error_invalid_request_duplicate_complete_member",
    "error_invalid_request_unknown_operation_type",
    "error_invalid_request_operation_wrong_type",
    "error_invalid_request_257_operations",
    "error_invalid_intent_unit_id_null",
    "error_invalid_intent_unit_id_uppercase",
    "error_invalid_intent_unit_id_nonhyphenated",
    "error_invalid_intent_unit_id_wrong_type",
    "error_invalid_external_reference_missing",
    "error_invalid_external_reference_null",
    "error_invalid_external_reference_namespace_empty",
    "error_invalid_external_reference_namespace_uppercase",
    "error_invalid_external_reference_namespace_too_long",
    "error_invalid_external_reference_namespace_bad_punctuation",
    "error_invalid_external_reference_scope_empty",
    "error_invalid_external_reference_scope_blank",
    "error_invalid_external_reference_scope_nul",
    "error_invalid_external_reference_scope_257_bytes",
    "error_invalid_external_reference_scope_multibyte_258_bytes",
    "error_invalid_external_reference_value_blank",
    "error_invalid_external_reference_value_nul",
    "error_invalid_external_reference_value_257_bytes",
    "error_invalid_species_empty",
    "error_invalid_species_blank",
    "error_invalid_species_nul",
    "error_invalid_species_257_bytes",
    "error_invalid_species_multibyte_258_bytes",
    "error_invalid_workflow_id_empty",
    "error_invalid_workflow_id_blank",
    "error_invalid_workflow_id_nul",
    "error_invalid_workflow_id_257_bytes",
    "error_invalid_workflow_id_multibyte_258_bytes",
    "error_invalid_phase_id_phase_empty",
    "error_invalid_phase_id_phase_blank",
    "error_invalid_phase_id_initial_nul",
    "error_invalid_phase_id_edge_257_bytes",
    "error_invalid_phase_id_completion_multibyte_258_bytes",
    "error_invalid_phase_id_operation_target_blank",
    "error_invalid_workflow_empty_phases",
    "error_invalid_workflow_33_phases",
    "error_invalid_workflow_duplicate_phase",
    "error_invalid_workflow_unknown_initial_phase",
    "error_invalid_workflow_129_edges",
    "error_invalid_workflow_unknown_edge_source",
    "error_invalid_workflow_unknown_edge_target",
    "error_invalid_workflow_duplicate_edge",
    "error_invalid_workflow_33_completion_phases",
    "error_invalid_workflow_unknown_completion_phase",
    "error_invalid_workflow_duplicate_completion_phase",
    "success_request_size_1048575",
    "success_request_size_1048576",
    "error_request_size_1048577",
)

EXPECTED_IO_IDS = (
    "read_failure_after_17_bytes",
    "body_failure_after_17_bytes",
    "newline_failure_before_lf",
    "flush_failure_after_complete_response",
)

MESSAGES = {
    "malformed_json": "request is not valid RFC 8259 JSON",
    "request_too_large": "request exceeds the 1048576-byte limit",
    "invalid_request": "request does not match the stateless protocol v2 schema",
    "unsupported_protocol_version": "protocol_version must be 2",
    "invalid_intent_unit_id": "intent_unit.id must be a lowercase hyphenated RFC 4122 UUID",
    "invalid_external_reference": "origin must be an exact bounded external reference",
    "invalid_species": "intent_unit.species must be nonblank NUL-free UTF-8 of at most 256 bytes",
    "invalid_workflow_id": "workflow.id must be nonblank NUL-free UTF-8 of at most 256 bytes",
    "invalid_phase_id": "phase identifier must be nonblank NUL-free UTF-8 of at most 256 bytes",
    "invalid_workflow": "workflow topology is invalid",
    "transition_already_completed": "cannot transition a completed intent unit",
    "transition_unknown_target": "transition target is not declared by the workflow",
    "transition_not_allowed": "workflow does not allow this transition",
    "completion_already_completed": "cannot complete an already completed intent unit",
    "completion_phase_not_eligible": "current phase is not eligible for completion",
}
SETUP_WITHOUT_FIELD = {"malformed_json", "request_too_large"}
SETUP_WITH_FIELD = {
    "invalid_request",
    "unsupported_protocol_version",
    "invalid_intent_unit_id",
    "invalid_external_reference",
    "invalid_species",
    "invalid_workflow_id",
    "invalid_phase_id",
    "invalid_workflow",
}
OPERATION_CODES = {
    "transition_already_completed",
    "transition_unknown_target",
    "transition_not_allowed",
    "completion_already_completed",
    "completion_phase_not_eligible",
}

DUPLICATE_CASE_IDS = {
    "error_invalid_request_duplicate_top_level",
    "error_invalid_request_duplicate_workflow_member",
    "error_invalid_request_duplicate_edge_member",
    "error_invalid_request_duplicate_intent_unit_member",
    "error_invalid_request_duplicate_origin_member",
    "error_invalid_request_duplicate_transition_member",
    "error_invalid_request_duplicate_complete_member",
}
MALFORMED_CASE_IDS = {
    "error_malformed_truncated",
    "error_malformed_multiple_values",
    "error_malformed_invalid_utf8",
    "error_malformed_nan",
    "error_malformed_positive_infinity",
    "error_malformed_negative_infinity",
}
UNKNOWN_CASE_IDS = {
    "error_invalid_request_unknown_top_level",
    "error_invalid_request_unknown_workflow_member",
    "error_invalid_request_unknown_edge_member",
    "error_invalid_request_unknown_intent_unit_member",
    "error_invalid_request_unknown_origin_member",
    "error_invalid_request_unknown_transition_member",
    "error_invalid_request_unknown_complete_member",
}


class DuplicateMember(ValueError):
    pass


class NonFiniteNumber(ValueError):
    pass


def reject_duplicate_members(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise DuplicateMember(key)
        result[key] = value
    return result


def reject_nonfinite_number(value: str) -> None:
    raise NonFiniteNumber(value)


def load_json_bytes(data: bytes) -> Any:
    return json.loads(
        data.decode("utf-8"),
        object_pairs_hook=reject_duplicate_members,
        parse_constant=reject_nonfinite_number,
    )


def load_json_file(path: Path) -> Any:
    return load_json_bytes(path.read_bytes())


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def exact_keys(value: dict[str, Any], keys: tuple[str, ...], where: str) -> None:
    require(tuple(value) == keys, f"{where}: expected keys/order {keys}, got {tuple(value)}")


def inside_repo(ref_path: str) -> Path:
    require(ref_path != "", "empty fixture path")
    candidate_text = Path(ref_path)
    require(not candidate_text.is_absolute(), f"absolute fixture path: {ref_path}")
    require(".." not in candidate_text.parts, f"parent traversal in fixture path: {ref_path}")
    candidate = ROOT / candidate_text
    resolved = candidate.resolve(strict=True)
    require(resolved == ROOT or ROOT in resolved.parents, f"fixture escapes repository: {ref_path}")
    cursor = ROOT
    for part in candidate_text.parts:
        cursor /= part
        require(not cursor.is_symlink(), f"symlink forbidden in fixture path: {ref_path}")
    require(candidate.is_file(), f"fixture is not a regular file: {ref_path}")
    return candidate


def validate_ref(ref: Any, where: str) -> tuple[Path, bytes]:
    require(isinstance(ref, dict), f"{where}: file reference must be an object")
    exact_keys(ref, ("path", "bytes", "sha256"), where)
    require(isinstance(ref["path"], str), f"{where}: path must be text")
    require(type(ref["bytes"]) is int and ref["bytes"] >= 0, f"{where}: invalid byte count")
    require(
        isinstance(ref["sha256"], str)
        and re.fullmatch(r"[0-9a-f]{64}", ref["sha256"]) is not None,
        f"{where}: invalid SHA-256",
    )
    path = inside_repo(ref["path"])
    data = path.read_bytes()
    require(len(data) == ref["bytes"], f"{where}: byte count mismatch")
    require(digest(data) == ref["sha256"], f"{where}: SHA-256 mismatch")
    return path, data


def assert_schema(schema: Any) -> None:
    require(isinstance(schema, dict), "schema root must be an object")
    exact_keys(schema, ("$schema", "$id", "title", "description", "oneOf", "$defs"), "schema")
    require(schema["$schema"] == "https://json-schema.org/draft/2020-12/schema", "wrong draft")
    require(
        schema["oneOf"]
        == [
            {"$ref": "#/$defs/request"},
            {"$ref": "#/$defs/success_response"},
            {"$ref": "#/$defs/setup_error_response"},
            {"$ref": "#/$defs/operation_error_response"},
        ],
        "schema root union drift",
    )
    defs = schema["$defs"]
    expected_defs = {
        "namespace",
        "text256",
        "uuid",
        "u64_text",
        "json_pointer256",
        "external_reference",
        "workflow_edge",
        "workflow",
        "intent_unit_input",
        "transition_operation",
        "complete_operation",
        "operation",
        "request",
        "transition_history",
        "completion_history",
        "history_record",
        "unit_view",
        "simulation_result",
        "success_response",
        "setup_error_without_field",
        "setup_error_with_field",
        "setup_error_detail",
        "setup_error_response",
        "operation_error_detail",
        "operation_error_response",
    }
    require(set(defs) == expected_defs, "schema definition inventory drift")
    require(
        defs["uuid"]["pattern"]
        == "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$",
        "UUID syntax drift",
    )
    require(defs["namespace"]["pattern"] == "^[a-z][a-z0-9._-]{0,63}$", "namespace drift")
    require(defs["text256"]["pattern"] == r"^(?=[\s\S]*\S)[^\u0000]+$", "Text256 syntax drift")
    require(defs["text256"]["x-cubikan-max-utf8-bytes"] == 256, "Text256 byte bound drift")
    require(defs["request"]["properties"]["operations"]["maxItems"] == 256, "operation bound drift")
    require(defs["workflow"]["properties"]["phases"]["maxItems"] == 32, "phase bound drift")
    require(defs["workflow"]["properties"]["edges"]["maxItems"] == 128, "edge bound drift")
    require(
        defs["workflow"]["properties"]["completion_phases"]["maxItems"] == 32,
        "completion bound drift",
    )
    require(defs["unit_view"]["properties"]["history"]["maxItems"] == 256, "history bound drift")
    for member in ("phases", "edges", "completion_phases"):
        require(
            defs["workflow"]["properties"][member].get("uniqueItems") is True,
            f"workflow {member} uniqueness drift",
        )
    for name, definition in defs.items():
        if isinstance(definition, dict) and definition.get("type") == "object":
            require(definition.get("additionalProperties") is False, f"{name}: object is not closed")
            required = definition.get("required")
            require(isinstance(required, list), f"{name}: required inventory missing")
            require(len(required) == len(set(required)), f"{name}: duplicate required member")
            require(set(required).issubset(definition.get("properties", {})), f"{name}: unknown required member")


def assert_external_reference(value: Any, where: str) -> None:
    require(isinstance(value, dict), f"{where}: reference must be object")
    exact_keys(value, ("namespace", "scope", "value"), where)


def assert_workflow(value: Any, where: str) -> None:
    require(isinstance(value, dict), f"{where}: workflow must be object")
    exact_keys(value, ("id", "phases", "initial_phase", "edges", "completion_phases"), where)
    for index, edge in enumerate(value["edges"]):
        exact_keys(edge, ("from", "to"), f"{where}.edges[{index}]")


def assert_unit_view(value: Any, where: str) -> None:
    require(isinstance(value, dict), f"{where}: unit must be object")
    exact_keys(
        value,
        ("id", "origin", "species", "workflow", "phase", "status", "revision", "history"),
        where,
    )
    assert_external_reference(value["origin"], f"{where}.origin")
    assert_workflow(value["workflow"], f"{where}.workflow")
    require(value["status"] in ("active", "completed"), f"{where}: invalid status")
    require(value["revision"] == str(len(value["history"])), f"{where}: revision/history mismatch")
    phase = value["workflow"]["initial_phase"]
    completed = False
    for index, record in enumerate(value["history"]):
        require(record["sequence"] == str(index + 1), f"{where}: noncanonical history sequence")
        if record["type"] == "transition":
            exact_keys(record, ("type", "sequence", "from", "to"), f"{where}.history[{index}]")
            require(not completed and record["from"] == phase, f"{where}: incoherent transition history")
            phase = record["to"]
        elif record["type"] == "completion":
            exact_keys(record, ("type", "sequence", "phase"), f"{where}.history[{index}]")
            require(not completed and record["phase"] == phase, f"{where}: incoherent completion history")
            completed = True
        else:
            raise ValueError(f"{where}: unknown history record type")
    require(value["phase"] == phase, f"{where}: projected phase mismatch")
    require(value["status"] == ("completed" if completed else "active"), f"{where}: status mismatch")


def assert_response(value: Any, case_id: str, exit_code: int) -> str | None:
    require(isinstance(value, dict), f"{case_id}: response must be object")
    require(value.get("protocol_version") == 2, f"{case_id}: response version drift")
    require(value.get("authority") == "simulation_only", f"{case_id}: authority drift")
    forbidden_keys = {
        "canonical",
        "accepted",
        "committed",
        "finalized",
        "ledger",
        "verified",
        "rpc",
        "database",
        "session",
        "signer",
        "coordinate",
    }

    def inspect_keys(item: Any) -> None:
        if isinstance(item, dict):
            require(forbidden_keys.isdisjoint(item), f"{case_id}: forbidden authority/state vocabulary")
            for nested in item.values():
                inspect_keys(nested)
        elif isinstance(item, list):
            for nested in item:
                inspect_keys(nested)

    inspect_keys(value)
    if value.get("outcome") == "success":
        exact_keys(value, ("protocol_version", "authority", "outcome", "result"), case_id)
        require(exit_code == 0, f"{case_id}: success exit must be zero")
        result = value["result"]
        exact_keys(result, ("type", "intent_unit"), f"{case_id}.result")
        require(result["type"] == "simulation", f"{case_id}: result type drift")
        assert_unit_view(result["intent_unit"], f"{case_id}.result.intent_unit")
        return None
    require(value.get("outcome") == "error", f"{case_id}: invalid outcome")
    error = value.get("error")
    require(isinstance(error, dict), f"{case_id}: missing error detail")
    code = error.get("code")
    require(code in MESSAGES, f"{case_id}: code outside stateless registry")
    require(error.get("message") == MESSAGES[code], f"{case_id}: diagnostic bytes drift")
    if code in OPERATION_CODES:
        exact_keys(
            value,
            ("protocol_version", "authority", "outcome", "error", "intent_unit"),
            case_id,
        )
        exact_keys(error, ("code", "message", "operation_number"), f"{case_id}.error")
        require(exit_code == 3, f"{case_id}: operation error exit must be three")
        require(type(error["operation_number"]) is int, f"{case_id}: operation number type")
        require(0 <= error["operation_number"] <= 255, f"{case_id}: operation number bound")
        assert_unit_view(value["intent_unit"], f"{case_id}.intent_unit")
    else:
        exact_keys(value, ("protocol_version", "authority", "outcome", "error"), case_id)
        require(exit_code == 2, f"{case_id}: setup error exit must be two")
        if code in SETUP_WITH_FIELD:
            exact_keys(error, ("code", "message", "field"), f"{case_id}.error")
            field = error["field"]
            require(isinstance(field, str), f"{case_id}: field must be text")
            require(len(field.encode("utf-8")) <= 256 and "\x00" not in field, f"{case_id}: field bound")
            require(
                re.fullmatch(r"(?:/(?:[^~/\x00]|~[01])*)*", field) is not None,
                f"{case_id}: field is not RFC 6901",
            )
        else:
            require(code in SETUP_WITHOUT_FIELD, f"{case_id}: illegal setup code")
            exact_keys(error, ("code", "message"), f"{case_id}.error")
    return code


schema_bytes = SCHEMA.read_bytes()
manifest_bytes = MANIFEST.read_bytes()
io_bytes = IO_ORACLE.read_bytes()
require(digest(schema_bytes) == EXPECTED_SCHEMA_SHA256, "locked schema SHA-256 drift")
require(digest(manifest_bytes) == EXPECTED_MANIFEST_SHA256, "locked manifest SHA-256 drift")
require(digest(io_bytes) == EXPECTED_IO_SHA256, "locked I/O oracle SHA-256 drift")
schema = load_json_bytes(schema_bytes)
manifest = load_json_bytes(manifest_bytes)
io_oracle = load_json_bytes(io_bytes)
assert_schema(schema)

exact_keys(manifest, ("fixture_schema_version", "hash_algorithm", "schema", "cases"), "manifest")
require(manifest["fixture_schema_version"] == 1, "fixture schema version drift")
require(manifest["hash_algorithm"] == "sha256", "fixture hash algorithm drift")
schema_path, referenced_schema = validate_ref(manifest["schema"], "manifest.schema")
require(schema_path == SCHEMA, "manifest references wrong schema path")
require(referenced_schema == schema_bytes, "manifest schema bytes drift")
require([case["id"] for case in manifest["cases"]] == list(EXPECTED_CASE_IDS), "case order/inventory drift")

referenced_fixture_paths = {MANIFEST, IO_ORACLE}
seen_codes: set[str] = set()
seen_operation_types: set[str] = set()
seen_result_types: set[str] = set()
case_by_id: dict[str, dict[str, Any]] = {}
request_by_id: dict[str, bytes] = {}
stdout_by_id: dict[str, bytes] = {}

for index, case in enumerate(manifest["cases"]):
    where = f"manifest.cases[{index}]"
    require(isinstance(case, dict), f"{where}: case must be object")
    allowed_keys = ("id", "request", "stdout", "exit_code")
    if "context" in case:
        allowed_keys = ("id", "request", "context", "stdout", "exit_code")
    exact_keys(case, allowed_keys, where)
    case_id = case["id"]
    require(isinstance(case_id, str), f"{where}: case ID must be text")
    request_path, request_bytes = validate_ref(case["request"], f"{where}.request")
    stdout_path, stdout_bytes = validate_ref(case["stdout"], f"{where}.stdout")
    referenced_fixture_paths.update((request_path, stdout_path))
    require(type(case["exit_code"]) is int, f"{where}: exit code type")
    require(stdout_bytes.endswith(b"\n") and not stdout_bytes.endswith(b"\n\n"), f"{case_id}: stdout LF")
    response = load_json_bytes(stdout_bytes[:-1])
    require(canonical_json(response) == stdout_bytes[:-1], f"{case_id}: noncanonical stdout bytes")
    code = assert_response(response, case_id, case["exit_code"])
    if code is not None:
        seen_codes.add(code)
    else:
        seen_result_types.add(response["result"]["type"])
    if "context" in case:
        context = case["context"]
        exact_keys(context, ("generated_uuid",), f"{where}.context")
        require(
            context["generated_uuid"] == "123e4567-e89b-42d3-a456-426614174000",
            f"{case_id}: generated UUID drift",
        )
        require(case_id == "success_omitted_id_uses_manifest_uuid", f"{case_id}: context is illegal")
    case_by_id[case_id] = case
    request_by_id[case_id] = request_bytes
    stdout_by_id[case_id] = stdout_bytes
    if case_id in DUPLICATE_CASE_IDS:
        try:
            load_json_bytes(request_bytes)
        except DuplicateMember:
            pass
        else:
            raise ValueError(f"{case_id}: raw duplicate member was lost")
    elif case_id in MALFORMED_CASE_IDS:
        try:
            load_json_bytes(request_bytes)
        except (UnicodeDecodeError, json.JSONDecodeError, NonFiniteNumber):
            pass
        else:
            raise ValueError(f"{case_id}: malformed JSON became valid")
    else:
        parsed_request = load_json_bytes(request_bytes)
        if isinstance(parsed_request, dict):
            operations = parsed_request.get("operations")
            if isinstance(operations, list):
                for operation in operations:
                    if isinstance(operation, dict) and isinstance(operation.get("type"), str):
                        seen_operation_types.add(operation["type"])

require(seen_codes == set(MESSAGES), "stateless error-code coverage drift")
require(seen_result_types == {"simulation"}, "stateless result inventory drift")
require({"transition", "complete"}.issubset(seen_operation_types), "operation coverage incomplete")
require(UNKNOWN_CASE_IDS.issubset(case_by_id), "unknown-member coverage incomplete")
require(DUPLICATE_CASE_IDS.issubset(case_by_id), "duplicate-member coverage incomplete")

# Exact maxima, omission/null, escaping, and raw size evidence.
omitted = load_json_bytes(request_by_id["success_omitted_id_uses_manifest_uuid"])
require("id" not in omitted["intent_unit"], "omitted-ID fixture contains ID")
null_id = load_json_bytes(request_by_id["error_invalid_intent_unit_id_null"])
require(null_id["intent_unit"]["id"] is None, "null-ID fixture drift")
nil_id = load_json_bytes(request_by_id["success_canonical_nil_uuid"])
require(nil_id["intent_unit"]["id"] == "00000000-0000-0000-0000-000000000000", "nil UUID drift")
max_ops = load_json_bytes(request_by_id["success_256_operations_and_history_records"])
max_ops_out = load_json_bytes(stdout_by_id["success_256_operations_and_history_records"][:-1])
require(len(max_ops["operations"]) == 256, "operation maximum fixture drift")
require(len(max_ops_out["result"]["intent_unit"]["history"]) == 256, "history maximum fixture drift")
require(max_ops_out["result"]["intent_unit"]["revision"] == "256", "revision maximum fixture drift")
op_255 = load_json_bytes(stdout_by_id["error_operation_number_255"][:-1])
require(op_255["error"]["operation_number"] == 255, "operation_number maximum drift")
max_collections = load_json_bytes(request_by_id["success_workflow_collection_maxima"])["workflow"]
require(
    (len(max_collections["phases"]), len(max_collections["edges"]), len(max_collections["completion_phases"]))
    == (32, 128, 32),
    "workflow collection maxima drift",
)
max_scalars = load_json_bytes(request_by_id["success_scalar_maxima"])
require(len(max_scalars["intent_unit"]["origin"]["namespace"].encode()) == 64, "namespace maximum drift")
for text in (
    max_scalars["workflow"]["id"],
    max_scalars["workflow"]["phases"][0],
    max_scalars["intent_unit"]["origin"]["scope"],
    max_scalars["intent_unit"]["origin"]["value"],
    max_scalars["intent_unit"]["species"],
):
    require(len(text.encode("utf-8")) == 256, "Text256 maximum drift")
min_scalars = load_json_bytes(request_by_id["success_scalar_minima_and_zero_completion_phases"])
require(len(min_scalars["intent_unit"]["origin"]["namespace"].encode()) == 1, "namespace minimum drift")
for text in (
    min_scalars["workflow"]["id"],
    min_scalars["workflow"]["phases"][0],
    min_scalars["intent_unit"]["origin"]["scope"],
    min_scalars["intent_unit"]["origin"]["value"],
    min_scalars["intent_unit"]["species"],
):
    require(len(text.encode("utf-8")) == 1, "Text256 minimum drift")
require(min_scalars["workflow"]["completion_phases"] == [], "zero completion-phase boundary drift")
escaping = stdout_by_id["success_canonical_escaping_and_utf8"]
require(b"\\u0001" in escaping and b"\\u0002" in escaping and b"\\u0003" in escaping, "control escaping drift")
require("é".encode() in escaping and "終".encode() in escaping, "UTF-8 escaping drift")
require(b"\\/" not in escaping, "slash must not be escaped")
for size in (1_048_575, 1_048_576, 1_048_577):
    prefix = "success" if size <= 1_048_576 else "error"
    case_id = f"{prefix}_request_size_{size}"
    require(len(request_by_id[case_id]) == size, f"{case_id}: exact byte boundary drift")

# Closed I/O fault oracle, raw bytes, source chains, and one-response attempts.
exact_keys(io_oracle, ("fixture_schema_version", "hash_algorithm", "cases"), "io oracle")
require(io_oracle["fixture_schema_version"] == 1, "I/O fixture schema version drift")
require(io_oracle["hash_algorithm"] == "sha256", "I/O hash algorithm drift")
require([case["id"] for case in io_oracle["cases"]] == list(EXPECTED_IO_IDS), "I/O case inventory drift")
expected_io = {
    "read_failure_after_17_bytes": ("read", 17, 0, 0, 0, b"", b"cubikan: failed to read request: fixture read failure\n"),
    "body_failure_after_17_bytes": ("body", 17, 1, 0, 0, None, b"cubikan: failed to write response body: fixture body failure\n"),
    "newline_failure_before_lf": ("newline", 0, 1, 1, 0, None, b"cubikan: failed to write response newline: fixture newline failure\n"),
    "flush_failure_after_complete_response": ("flush", 0, 1, 1, 1, None, b"cubikan: failed to flush response: fixture flush failure\n"),
}
io_stdout: dict[str, bytes] = {}
for index, case in enumerate(io_oracle["cases"]):
    where = f"io.cases[{index}]"
    exact_keys(
        case,
        (
            "id",
            "request",
            "fault",
            "stdout",
            "stderr",
            "exit_code",
            "response_attempts",
            "newline_attempts",
            "flush_attempts",
            "expected_source_chain",
        ),
        where,
    )
    exact_keys(case["fault"], ("stage", "after_bytes", "io_kind", "source_message"), f"{where}.fault")
    request_path, request_data = validate_ref(case["request"], f"{where}.request")
    stdout_path, stdout_data = validate_ref(case["stdout"], f"{where}.stdout")
    stderr_path, stderr_data = validate_ref(case["stderr"], f"{where}.stderr")
    referenced_fixture_paths.update((request_path, stdout_path, stderr_path))
    require(request_data == request_by_id["success_explicit_id_zero_operations"], f"{where}: request drift")
    stage, after, responses, newlines, flushes, exact_stdout, exact_stderr = expected_io[case["id"]]
    require(case["fault"]["stage"] == stage, f"{where}: stage drift")
    require(case["fault"]["after_bytes"] == after, f"{where}: fault offset drift")
    require(case["fault"]["io_kind"] == "other", f"{where}: I/O kind drift")
    require(case["exit_code"] == 1, f"{where}: I/O exit drift")
    require(case["response_attempts"] == responses, f"{where}: response count drift")
    require(case["newline_attempts"] == newlines, f"{where}: newline count drift")
    require(case["flush_attempts"] == flushes, f"{where}: flush count drift")
    if exact_stdout is not None:
        require(stdout_data == exact_stdout, f"{where}: stdout drift")
    require(stderr_data == exact_stderr, f"{where}: stderr diagnostic drift")
    require(
        case["expected_source_chain"]
        == [
            exact_stderr.decode().removeprefix("cubikan: ").rsplit(": ", 1)[0],
            case["fault"]["source_message"],
        ],
        f"{where}: source chain drift",
    )
    io_stdout[case["id"]] = stdout_data

full_body = stdout_by_id["success_explicit_id_zero_operations"][:-1]
require(io_stdout["body_failure_after_17_bytes"] == full_body[:17], "body-failure prefix drift")
require(io_stdout["newline_failure_before_lf"] == full_body, "newline-failure body drift")
require(io_stdout["flush_failure_after_complete_response"] == full_body + b"\n", "flush-failure body drift")

# No unmanifested corpus files or symlinks may hide alongside the oracle.
actual_fixture_paths: set[Path] = set()
for path in FIXTURES.rglob("*"):
    require(not path.is_symlink(), f"symlink forbidden in fixture inventory: {path}")
    if path.is_file():
        actual_fixture_paths.add(path)
require(actual_fixture_paths == referenced_fixture_paths, "fixture file inventory drift")

print(
    "verified cubikan protocol v2: "
    f"schema={EXPECTED_SCHEMA_SHA256} manifest={EXPECTED_MANIFEST_SHA256} "
    f"cases={len(EXPECTED_CASE_IDS)} io_cases={len(EXPECTED_IO_IDS)}"
)
PY
