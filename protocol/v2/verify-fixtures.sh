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

python3 -I -S - "$repo_root" <<'LOCAL_PY'
from __future__ import annotations

import hashlib
import json
import re
import sys
from collections import Counter
from pathlib import Path
from typing import Any


ROOT = Path(sys.argv[1]).resolve(strict=True)
SCHEMA = ROOT / "protocol/v2/cubikan-local.schema.json"
FIXTURES = ROOT / "tests/fixtures/protocol-v2/cubikan-local"
MANIFEST = FIXTURES / "manifest-v1.json"
INVENTORY = FIXTURES / "inventory-v1.json"
PROCESS = FIXTURES / "process-v1.json"
IO_ORACLE = FIXTURES / "io-v1.json"
SOURCE_MAPPING = FIXTURES / "source-mapping-v1.json"

EXPECTED_SCHEMA_SHA256 = "869d30e3865cbaf80af60616fdf13d0e4b848997bb36f4bcc68f0e7cc2d7d4ff"
EXPECTED_MANIFEST_SHA256 = "09cb8615098c7d38ca0b2420d554b21b758267e1f2d2c4089025610b25148e70"
EXPECTED_INVENTORY_SHA256 = "b22d2a457e2082c736128718431531dec2acf83162319c6c12d54d2870ee8e5e"
EXPECTED_PROCESS_SHA256 = "12ab16324568bc207367d05b1dd9cd49d284054cebbad1a222a582ff4801071d"
EXPECTED_IO_SHA256 = "e46c9ff56ee7bd3df3a9d74ce0fae1116a258bad0e748d7122ed88331a14607e"
EXPECTED_SOURCE_MAPPING_SHA256 = "dbe31207223b65cf06610ed17238b136e8295b873ec2ef8cb6f847bcebf1486c"

OPERATIONS = (
    "create_intent_unit",
    "get_intent_unit",
    "list_intent_units",
    "transition_intent_unit",
    "complete_intent_unit",
    "create_relationship_definition",
    "get_relationship_definition",
    "create_relationship",
    "delete_relationship",
    "list_relationships",
    "project_intent_units_v1",
    "record_association",
    "revoke_association",
    "list_associations_by_unit",
    "list_associations_by_reference",
)
MUTATIONS = (
    "create_intent_unit",
    "transition_intent_unit",
    "complete_intent_unit",
    "create_relationship_definition",
    "create_relationship",
    "delete_relationship",
    "record_association",
    "revoke_association",
)
READS = tuple(operation for operation in OPERATIONS if operation not in MUTATIONS)
RESULT_TAGS = (
    "intent_unit",
    "intent_unit_page",
    "relationship_definition",
    "relationship_page",
    "projection_v1_page",
    "association_page",
)
MUTATION_OUTCOMES = (
    "submission_rejected",
    "submission_lane_unresolved",
    "expired_not_included",
    "finalized_dispatch_rejected",
    "finalized_invariant_failed",
    "delivery_indeterminate",
    "finalized_accepted",
)
EFFECT_BY_OPERATION = {
    "create_intent_unit": "unit_created",
    "transition_intent_unit": "unit_transitioned",
    "complete_intent_unit": "unit_completed",
    "create_relationship_definition": "relationship_definition_created",
    "create_relationship": "relationship_created",
    "delete_relationship": "relationship_deleted",
    "record_association": "association_recorded",
    "revoke_association": "association_revoked",
}
READ_RESULT_BY_OPERATION = {
    "get_intent_unit": ("intent_unit", None),
    "list_intent_units": ("intent_unit_page", None),
    "get_relationship_definition": ("relationship_definition", None),
    "list_relationships": ("relationship_page", None),
    "project_intent_units_v1": ("projection_v1_page", None),
    "list_associations_by_unit": ("association_page", "by_unit"),
    "list_associations_by_reference": ("association_page", "by_reference"),
}
RESULT_DEFINITION_BY_TAG = {
    "intent_unit": "intent_unit_result",
    "intent_unit_page": "intent_unit_page_result",
    "relationship_definition": "relationship_definition_result",
    "relationship_page": "relationship_page_result",
    "projection_v1_page": "projection_v1_page_result",
    "association_page": "association_page_result",
}
OPERATION_SCHEMA_FIELDS = {
    "create_intent_unit": (("type", "intent_unit", "workflow"), ("type", "intent_unit", "workflow")),
    "get_intent_unit": (("type", "id"), ("type", "id")),
    "list_intent_units": (("type", "filters", "limit", "after"), ("type", "filters", "limit")),
    "transition_intent_unit": (("type", "id", "target", "expected_revision"), ("type", "id", "target", "expected_revision")),
    "complete_intent_unit": (("type", "id", "expected_revision"), ("type", "id", "expected_revision")),
    "create_relationship_definition": (("type", "definition", "source_species", "target_species", "self_policy", "cycle_policy"), ("type", "definition", "self_policy", "cycle_policy")),
    "get_relationship_definition": (("type", "definition"), ("type", "definition")),
    "create_relationship": (("type", "relationship"), ("type", "relationship")),
    "delete_relationship": (("type", "relationship"), ("type", "relationship")),
    "list_relationships": (("type", "definition", "source_id", "target_id", "limit", "after"), ("type", "definition", "limit")),
    "project_intent_units_v1": (("type", "query_version", "filters", "predicate", "limit", "after"), ("type", "query_version", "filters", "limit")),
    "record_association": (("type", "association"), ("type", "association")),
    "revoke_association": (("type", "association"), ("type", "association")),
    "list_associations_by_unit": (("type", "unit_id", "subject", "limit", "after"), ("type", "unit_id", "limit")),
    "list_associations_by_reference": (("type", "reference", "limit", "after"), ("type", "reference", "limit")),
}
ERROR_CODES = (
    "malformed_json",
    "request_too_large",
    "invalid_request",
    "unsupported_protocol_version",
    "invalid_intent_unit_id",
    "invalid_external_reference",
    "invalid_species",
    "invalid_workflow_id",
    "invalid_phase_id",
    "invalid_workflow",
    "invalid_revision",
    "invalid_definition_id",
    "invalid_definition_version",
    "invalid_relationship_policy",
    "invalid_association_subject",
    "invalid_query",
    "invalid_cursor",
    "invalid_coordinate",
    "invalid_rpc_endpoint",
    "unsupported_platform",
    "insecure_projection_path",
    "projection_busy",
    "unsupported_schema_version",
    "unsupported_envelope_version",
    "corrupt_schema",
    "corrupt_envelope",
    "projection_mismatch",
    "refresh_required",
    "archive_rpc_unavailable",
    "archive_history_unavailable",
    "deployment_mismatch",
    "runtime_mismatch",
    "unsupported_event_schema_version",
    "conflicting_finalized_block",
    "projection_error",
    "dev_signer_unavailable",
    "submission_lane_corrupt",
    "submission_lane_unresolved",
    "nonce_conflict",
    "insufficient_balance",
    "transaction_invalid",
    "rpc_submission_rejected",
    "submission_watch_lost",
    "submission_timeout",
    "finalized_invariant_failed",
    "expired_not_included",
    "unsupported_command_schema_version",
    "unsigned_call",
    "unauthorized_submitter",
    "duplicate_intent_unit",
    "intent_unit_not_found",
    "revision_conflict",
    "lifecycle_history_capacity_exceeded",
    "transition_already_completed",
    "transition_unknown_target",
    "transition_not_allowed",
    "completion_already_completed",
    "completion_phase_not_eligible",
    "global_sequence_exhausted",
    "relationship_definition_already_exists",
    "relationship_definition_not_found",
    "relationship_source_not_found",
    "relationship_target_not_found",
    "relationship_source_species_mismatch",
    "relationship_target_species_mismatch",
    "self_relationship_rejected",
    "duplicate_relationship",
    "cycle_rejected",
    "relationship_capacity_exceeded",
    "relationship_not_found",
    "association_revision_out_of_range",
    "duplicate_association",
    "association_capacity_exceeded",
    "association_not_found",
)
FIELD_CODES = (
    "invalid_request",
    "unsupported_protocol_version",
    "invalid_intent_unit_id",
    "invalid_external_reference",
    "invalid_species",
    "invalid_workflow_id",
    "invalid_phase_id",
    "invalid_workflow",
    "invalid_revision",
    "invalid_definition_id",
    "invalid_definition_version",
    "invalid_relationship_policy",
    "invalid_association_subject",
    "invalid_query",
    "invalid_cursor",
    "invalid_coordinate",
    "invalid_rpc_endpoint",
)
GENERIC_EXIT4 = (
    "unsupported_platform",
    "insecure_projection_path",
    "projection_busy",
    "unsupported_schema_version",
    "unsupported_envelope_version",
    "corrupt_schema",
    "corrupt_envelope",
    "projection_mismatch",
    "refresh_required",
    "archive_rpc_unavailable",
    "archive_history_unavailable",
    "deployment_mismatch",
    "runtime_mismatch",
    "unsupported_event_schema_version",
    "conflicting_finalized_block",
    "projection_error",
)
GENERIC_PLAIN_CODES = (
    "malformed_json",
    "request_too_large",
    *GENERIC_EXIT4,
    "dev_signer_unavailable",
    "submission_lane_corrupt",
    "intent_unit_not_found",
    "relationship_definition_not_found",
)
SUBMISSION_REJECTED_CODES = ("nonce_conflict", "insufficient_balance", "transaction_invalid")
DISPATCH_CODES = (
    "runtime_mismatch",
    "unsupported_command_schema_version",
    "unsigned_call",
    "unauthorized_submitter",
    "duplicate_intent_unit",
    "intent_unit_not_found",
    "revision_conflict",
    "lifecycle_history_capacity_exceeded",
    "transition_already_completed",
    "transition_unknown_target",
    "transition_not_allowed",
    "completion_already_completed",
    "completion_phase_not_eligible",
    "global_sequence_exhausted",
    "relationship_definition_already_exists",
    "relationship_definition_not_found",
    "relationship_source_not_found",
    "relationship_target_not_found",
    "relationship_source_species_mismatch",
    "relationship_target_species_mismatch",
    "self_relationship_rejected",
    "duplicate_relationship",
    "cycle_rejected",
    "relationship_capacity_exceeded",
    "relationship_not_found",
    "association_revision_out_of_range",
    "duplicate_association",
    "association_capacity_exceeded",
    "association_not_found",
)
FINALIZED_DISPATCH_PLAIN_CODES = tuple(code for code in DISPATCH_CODES if code != "revision_conflict")
DELIVERY_CODES = (
    "nonce_conflict",
    "transaction_invalid",
    "rpc_submission_rejected",
    "submission_watch_lost",
    "submission_timeout",
)

MESSAGES = {
    "malformed_json": "request is not valid RFC 8259 JSON",
    "request_too_large": "request exceeds the 1048576-byte limit",
    "invalid_request": "request does not match the local protocol v2 schema",
    "unsupported_protocol_version": "protocol_version must be 2",
    "invalid_intent_unit_id": "intent unit identifier must be a lowercase hyphenated RFC 4122 UUID",
    "invalid_external_reference": "external reference must contain an exact bounded namespace, scope, and value",
    "invalid_species": "species must be nonblank NUL-free UTF-8 of at most 256 bytes",
    "invalid_workflow_id": "workflow identifier must be nonblank NUL-free UTF-8 of at most 256 bytes",
    "invalid_phase_id": "phase identifier must be nonblank NUL-free UTF-8 of at most 256 bytes",
    "invalid_workflow": "workflow topology is invalid",
    "invalid_revision": "revision must be canonical unsigned decimal text within u64",
    "invalid_definition_id": "definition identifier must use the canonical namespace grammar",
    "invalid_definition_version": "definition version must be canonical nonzero unsigned decimal text within u64",
    "invalid_relationship_policy": "relationship policy must be allow or reject",
    "invalid_association_subject": "association subject must be whole_unit or an exact revision",
    "invalid_query": "query does not match the supported bounded query contract",
    "invalid_cursor": "cursor does not belong to the requested ordered query",
    "invalid_coordinate": "ledger coordinate is structurally invalid or fails its joined hash invariant",
    "invalid_rpc_endpoint": "RPC endpoint must be a canonical loopback ws URL with an explicit nondefault port",
    "unsupported_platform": "the required local platform boundary is unsupported",
    "insecure_projection_path": "projection path failed the local security boundary",
    "projection_busy": "projection storage is busy",
    "unsupported_schema_version": "projection schema version is unsupported",
    "unsupported_envelope_version": "stored envelope version is unsupported",
    "corrupt_schema": "projection schema is corrupt",
    "corrupt_envelope": "stored envelope is corrupt",
    "projection_mismatch": "projection does not match the attested finalized archive",
    "refresh_required": "projection changed before the verified read was pinned",
    "archive_rpc_unavailable": "archive RPC is unavailable or lacks unique local process evidence",
    "archive_history_unavailable": "required finalized archive history is unavailable",
    "deployment_mismatch": "archive deployment identity does not match the pinned deployment",
    "runtime_mismatch": "archive runtime identity or response does not match the pinned runtime",
    "unsupported_event_schema_version": "finalized event schema version is unsupported",
    "conflicting_finalized_block": "finalized block history conflicts at one height",
    "projection_error": "projection processing failed without a more specific safe classification",
    "dev_signer_unavailable": "the selected named development signer is unavailable",
    "submission_lane_corrupt": "the derived signer submission lane is corrupt",
    "submission_lane_unresolved": "the derived signer submission lane remains unresolved",
    "nonce_conflict": "canonical signer nonce could not be used safely",
    "insufficient_balance": "canonical signer balance cannot pay the transaction",
    "transaction_invalid": "transaction validity could not be proven",
    "rpc_submission_rejected": "submission RPC failed after the send boundary",
    "submission_watch_lost": "submission watcher ended before a finalized outcome",
    "submission_timeout": "submission did not reach a finalized outcome within the bounded wait",
    "finalized_invariant_failed": "finalized inclusion violated the exact accepted-event invariant",
    "expired_not_included": "the exact extrinsic hash was absent throughout its finalized mortal era",
    "unsupported_command_schema_version": "runtime rejected the fixed command schema version",
    "unsigned_call": "runtime rejected an unsigned mutation call",
    "unauthorized_submitter": "runtime rejected the development submitter",
    "duplicate_intent_unit": "intent unit already exists",
    "intent_unit_not_found": "intent unit was not found",
    "revision_conflict": "expected revision does not match canonical parent state",
    "lifecycle_history_capacity_exceeded": "lifecycle history reached its fixed capacity",
    "transition_already_completed": "cannot transition a completed intent unit",
    "transition_unknown_target": "transition target is not declared by the workflow",
    "transition_not_allowed": "workflow does not allow this transition",
    "completion_already_completed": "cannot complete an already completed intent unit",
    "completion_phase_not_eligible": "current phase is not eligible for completion",
    "global_sequence_exhausted": "global event sequence is exhausted",
    "relationship_definition_already_exists": "relationship definition already exists",
    "relationship_definition_not_found": "relationship definition was not found",
    "relationship_source_not_found": "relationship source intent unit was not found",
    "relationship_target_not_found": "relationship target intent unit was not found",
    "relationship_source_species_mismatch": "relationship source species violates its definition",
    "relationship_target_species_mismatch": "relationship target species violates its definition",
    "self_relationship_rejected": "relationship definition rejects self relationships",
    "duplicate_relationship": "relationship already exists",
    "cycle_rejected": "relationship definition rejects the resulting cycle",
    "relationship_capacity_exceeded": "relationship definition reached its fixed edge capacity",
    "relationship_not_found": "relationship was not found",
    "association_revision_out_of_range": "association revision is outside the intent unit history",
    "duplicate_association": "association already exists",
    "association_capacity_exceeded": "intent unit reached its fixed association capacity",
    "association_not_found": "association was not found",
}

DEFINITION_NAMES = (
    "namespace", "text256", "uuid", "u64_text", "nonzero_u64_text", "u32", "hash32",
    "json_pointer256", "page_limit", "unit_status", "relationship_policy", "mutation_operation",
    "error_code", "external_reference", "workflow_edge", "workflow", "intent_unit_input",
    "definition_key", "relationship_key", "association_subject_whole_unit",
    "association_subject_revision", "association_subject", "association_key", "list_filters",
    "outgoing_predicate", "incoming_predicate", "direct_relationship_predicate",
    "ledger_coordinate", "finalized_extrinsic_coordinate", "projection_checkpoint", "mortal_era",
    "create_intent_unit_operation", "get_intent_unit_operation", "list_intent_units_operation",
    "transition_intent_unit_operation", "complete_intent_unit_operation",
    "create_relationship_definition_operation", "get_relationship_definition_operation",
    "create_relationship_operation", "delete_relationship_operation", "list_relationships_operation",
    "project_intent_units_v1_operation", "record_association_operation", "revoke_association_operation",
    "list_associations_by_unit_operation", "list_associations_by_reference_operation", "operation",
    "request", "transition_history", "completion_history", "history_record", "unit_view",
    "projected_unit", "unit_summary", "projected_definition", "projected_relationship",
    "projected_association", "intent_unit_result", "intent_unit_page_result",
    "relationship_definition_result", "relationship_page_result", "projection_v1_page_result",
    "association_page_result", "result", "read_success_response", "unit_created_effect",
    "unit_transitioned_effect", "unit_completed_effect", "relationship_definition_created_effect",
    "relationship_created_effect", "relationship_deleted_effect", "association_recorded_effect",
    "association_revoked_effect", "accepted_effect", "caught_up_projection", "lagging_projection",
    "accepted_projection", "field_error_detail", "generic_plain_error_detail", "generic_error_detail",
    "generic_error_response", "submission_rejected_error_detail",
    "submission_lane_unresolved_error_detail", "expired_not_included_error_detail",
    "finalized_dispatch_plain_error_detail", "revision_conflict_error_detail",
    "finalized_dispatch_error_detail", "finalized_invariant_error_detail",
    "delivery_indeterminate_error_detail", "submission_rejected_response",
    "submission_lane_unresolved_response", "expired_not_included_response",
    "finalized_dispatch_rejected_response", "finalized_invariant_failed_response",
    "delivery_indeterminate_response", "finalized_accepted_response",
)

PARACHAIN_GENESIS = "0x627f53b3abc01130ec273ef85759f90779e8497614a428a66d862a624ee01a17"
DEPLOYMENT_ID = "0x3046cb2cf3f5f9c565a85493cfff10fee94d12d950a0d6f54d7c1ff32a6afc42"
RUNTIME_CODE_HASH = "0xe95e40bb618591b98b315b7901f3586ee5899f8bf26bda01401601c4f86b8a00"
SIGNING_BLOCK_HASH = "0x838281807f7e7d7c7b7a797877767574737271706f6e6d6c6b6a696867666564"
CHARLIE = bytes.fromhex("90b5ab205c6974c9ea841be688864633dc9ca8a357843eeacf2314649965fe22")
BLOCK_HASHES = {
    "0": PARACHAIN_GENESIS,
    "2": "0x" + "22" * 32,
    "3": "0x" + "33" * 32,
    "4": "0x" + "44" * 32,
    "5": "0x" + "55" * 32,
}


class DuplicateMember(ValueError):
    pass


class NonFiniteNumber(ValueError):
    pass


def reject_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise DuplicateMember(key)
        result[key] = value
    return result


def reject_nonfinite(value: str) -> None:
    raise NonFiniteNumber(value)


def load_json(data: bytes) -> Any:
    return json.loads(
        data.decode("utf-8"),
        object_pairs_hook=reject_duplicates,
        parse_constant=reject_nonfinite,
    )


def canonical(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"), allow_nan=False).encode()


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def fail(message: str) -> None:
    raise ValueError(message)


def require(condition: bool, message: str) -> None:
    if not condition:
        fail(message)


def exact_keys(value: Any, keys: tuple[str, ...], where: str) -> None:
    require(isinstance(value, dict), f"{where}: expected object")
    require(tuple(value) == keys, f"{where}: expected keys/order {keys}, got {tuple(value)}")


def inside_repo(relative: str) -> Path:
    require(isinstance(relative, str) and relative != "", "empty/nontext artifact path")
    rel = Path(relative)
    require(not rel.is_absolute() and ".." not in rel.parts, f"unsafe artifact path: {relative}")
    candidate = ROOT / rel
    resolved = candidate.resolve(strict=True)
    require(resolved == ROOT or ROOT in resolved.parents, f"artifact escapes repository: {relative}")
    cursor = ROOT
    for part in rel.parts:
        cursor /= part
        require(not cursor.is_symlink(), f"artifact traverses symlink: {relative}")
    require(candidate.is_file(), f"artifact is not a regular file: {relative}")
    return candidate


REFERENCED: set[Path] = set()


def validate_ref(value: Any, where: str) -> tuple[Path, bytes]:
    exact_keys(value, ("path", "bytes", "sha256"), where)
    require(type(value["bytes"]) is int and value["bytes"] >= 0, f"{where}: byte count")
    require(isinstance(value["sha256"], str) and re.fullmatch(r"[0-9a-f]{64}", value["sha256"]) is not None, f"{where}: hash")
    path = inside_repo(value["path"])
    data = path.read_bytes()
    require(len(data) == value["bytes"], f"{where}: size mismatch")
    require(digest(data) == value["sha256"], f"{where}: digest mismatch")
    if path == FIXTURES or FIXTURES in path.parents:
        REFERENCED.add(path)
    return path, data


def assert_schema(schema: Any) -> None:
    exact_keys(schema, ("$schema", "$id", "title", "description", "oneOf", "$defs"), "schema")
    require(schema["$schema"] == "https://json-schema.org/draft/2020-12/schema", "schema draft drift")
    expected_root = [
        {"$ref": f"#/$defs/{name}"}
        for name in (
            "request", "read_success_response", "generic_error_response",
            "submission_rejected_response", "submission_lane_unresolved_response",
            "expired_not_included_response", "finalized_dispatch_rejected_response",
            "finalized_invariant_failed_response", "delivery_indeterminate_response",
            "finalized_accepted_response",
        )
    ]
    require(schema["oneOf"] == expected_root, "schema root union drift")
    defs = schema["$defs"]
    require(tuple(defs) == DEFINITION_NAMES and len(defs) == 96, "schema definition inventory drift")
    for name, definition in defs.items():
        if isinstance(definition, dict) and definition.get("type") == "object":
            require(definition.get("additionalProperties") is False, f"{name}: object not closed")
            required = definition.get("required")
            properties = definition.get("properties")
            require(isinstance(required, list) and isinstance(properties, dict), f"{name}: object inventory missing")
            require(len(required) == len(set(required)), f"{name}: duplicate required name")
            require(set(required).issubset(properties), f"{name}: required/property mismatch")
    def walk(value: Any, where: str) -> None:
        if isinstance(value, dict):
            if "$ref" in value:
                require(tuple(value) == ("$ref",), f"{where}: $ref has siblings")
                target = value["$ref"]
                require(isinstance(target, str) and target.startswith("#/$defs/") and target[8:] in defs, f"{where}: dangling/nonlocal $ref")
            for key, child in value.items():
                walk(child, f"{where}/{key}")
        elif isinstance(value, list):
            for index, child in enumerate(value):
                walk(child, f"{where}/{index}")
    walk(schema, "schema")
    require(defs["uuid"]["pattern"] == "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$", "UUID grammar drift")
    require(defs["namespace"]["pattern"] == "^[a-z][a-z0-9._-]{0,63}$", "namespace grammar drift")
    require(defs["text256"]["x-cubikan-max-utf8-bytes"] == 256, "Text256 byte bound drift")
    require(defs["u64_text"]["x-cubikan-maximum"] == "18446744073709551615", "u64 bound drift")
    require(defs["page_limit"] == {"type": "integer", "minimum": 1, "maximum": 100}, "page limit drift")
    require(defs["mutation_operation"]["enum"] == list(MUTATIONS), "mutation operation enum drift")
    require(defs["error_code"]["enum"] == list(ERROR_CODES), "error code enum drift")
    require(defs["projection_checkpoint"]["required"] == ["block_number", "block_hash", "last_global_sequence", "runtime_spec_version", "runtime_code_hash"], "checkpoint required inventory drift")
    require(defs["unit_view"]["properties"]["status"] == {"$ref": "#/$defs/unit_status"}, "unit status schema drift")
    require(defs["operation"]["oneOf"] == [{"$ref": f"#/$defs/{op}_operation"} for op in OPERATIONS], "operation union drift")
    for operation, (properties, required) in OPERATION_SCHEMA_FIELDS.items():
        definition = defs[f"{operation}_operation"]
        require(tuple(definition["properties"]) == properties, f"{operation}: field inventory drift")
        require(tuple(definition["required"]) == required, f"{operation}: required inventory drift")
        require(definition["properties"]["type"] == {"const": operation}, f"{operation}: tag drift")
    require(defs["intent_unit_input"]["required"] == ["origin", "species"], "optional generated ID drift")
    require(defs["projected_definition"]["required"] == ["key", "directed", "self_policy", "cycle_policy", "created_coordinate"], "optional species result drift")
    require(
        defs["result"]["oneOf"]
        == [{"$ref": f"#/$defs/{RESULT_DEFINITION_BY_TAG[tag]}"} for tag in RESULT_TAGS],
        "result union drift",
    )
    require(
        defs["accepted_effect"]["oneOf"]
        == [{"$ref": f"#/$defs/{effect}_effect"} for effect in EFFECT_BY_OPERATION.values()],
        "accepted effect union drift",
    )
    require(defs["field_error_detail"]["properties"]["code"]["enum"] == list(FIELD_CODES), "field-code allowlist drift")
    require(defs["generic_plain_error_detail"]["properties"]["code"]["enum"] == list(GENERIC_PLAIN_CODES), "generic plain-code allowlist drift")
    require(defs["submission_rejected_error_detail"]["properties"]["code"]["enum"] == list(SUBMISSION_REJECTED_CODES), "submission rejected-code allowlist drift")
    require(defs["submission_lane_unresolved_error_detail"]["properties"]["code"] == {"const": "submission_lane_unresolved"}, "submission lane code drift")
    require(defs["expired_not_included_error_detail"]["properties"]["code"] == {"const": "expired_not_included"}, "expired code drift")
    require(defs["finalized_dispatch_plain_error_detail"]["properties"]["code"]["enum"] == list(FINALIZED_DISPATCH_PLAIN_CODES), "dispatch plain-code allowlist drift")
    require(defs["revision_conflict_error_detail"]["properties"]["code"] == {"const": "revision_conflict"}, "revision-conflict code drift")
    require(defs["revision_conflict_error_detail"]["required"] == ["code", "message", "expected_revision", "actual_revision"], "revision pair drift")
    require(defs["finalized_invariant_error_detail"]["properties"]["code"] == {"const": "finalized_invariant_failed"}, "invariant code drift")
    require(defs["delivery_indeterminate_error_detail"]["properties"]["code"]["enum"] == list(DELIVERY_CODES), "delivery-code allowlist drift")
    require(defs["generic_error_detail"]["oneOf"] == [{"$ref": "#/$defs/field_error_detail"}, {"$ref": "#/$defs/generic_plain_error_detail"}], "generic error union drift")
    require(defs["finalized_dispatch_error_detail"]["oneOf"] == [{"$ref": "#/$defs/finalized_dispatch_plain_error_detail"}, {"$ref": "#/$defs/revision_conflict_error_detail"}], "dispatch error union drift")
    response_shapes = {
        "read_success_response": (
            "success",
            ("protocol_version", "outcome", "result"),
            {"result": "result"},
        ),
        "generic_error_response": (
            "error",
            ("protocol_version", "outcome", "error"),
            {"error": "generic_error_detail"},
        ),
        "submission_rejected_response": (
            "submission_rejected",
            ("protocol_version", "outcome", "operation", "error"),
            {"operation": "mutation_operation", "error": "submission_rejected_error_detail"},
        ),
        "submission_lane_unresolved_response": (
            "submission_lane_unresolved",
            ("protocol_version", "outcome", "operation", "expected_extrinsic_hash", "era", "error"),
            {"operation": "mutation_operation", "expected_extrinsic_hash": "hash32", "era": "mortal_era", "error": "submission_lane_unresolved_error_detail"},
        ),
        "expired_not_included_response": (
            "expired_not_included",
            ("protocol_version", "outcome", "operation", "expected_extrinsic_hash", "era", "error"),
            {"operation": "mutation_operation", "expected_extrinsic_hash": "hash32", "era": "mortal_era", "error": "expired_not_included_error_detail"},
        ),
        "finalized_dispatch_rejected_response": (
            "finalized_dispatch_rejected",
            ("protocol_version", "outcome", "operation", "finalized_extrinsic", "error"),
            {"operation": "mutation_operation", "finalized_extrinsic": "finalized_extrinsic_coordinate", "error": "finalized_dispatch_error_detail"},
        ),
        "finalized_invariant_failed_response": (
            "finalized_invariant_failed",
            ("protocol_version", "outcome", "operation", "finalized_extrinsic", "error"),
            {"operation": "mutation_operation", "finalized_extrinsic": "finalized_extrinsic_coordinate", "error": "finalized_invariant_error_detail"},
        ),
        "delivery_indeterminate_response": (
            "delivery_indeterminate",
            ("protocol_version", "outcome", "operation", "expected_extrinsic_hash", "era", "error"),
            {"operation": "mutation_operation", "expected_extrinsic_hash": "hash32", "era": "mortal_era", "error": "delivery_indeterminate_error_detail"},
        ),
    }
    require(
        tuple(outcome for name, (outcome, _, _) in response_shapes.items() if name not in ("read_success_response", "generic_error_response"))
        + ("finalized_accepted",)
        == MUTATION_OUTCOMES,
        "mutation outcome registry drift",
    )
    for name, (outcome, members, refs) in response_shapes.items():
        definition = defs[name]
        require(tuple(definition["properties"]) == members and tuple(definition["required"]) == members, f"{name}: member inventory drift")
        require(definition["properties"]["protocol_version"] == {"const": 2}, f"{name}: protocol version drift")
        require(definition["properties"]["outcome"] == {"const": outcome}, f"{name}: outcome drift")
        for member, target in refs.items():
            require(definition["properties"][member] == {"$ref": f"#/$defs/{target}"}, f"{name}.{member}: reference drift")
    require(len(defs["finalized_accepted_response"]["oneOf"]) == 8, "accepted pair count drift")
    for branch, (operation, effect) in zip(defs["finalized_accepted_response"]["oneOf"], EFFECT_BY_OPERATION.items(), strict=True):
        members = ("protocol_version", "outcome", "operation", "coordinate", "effect", "projection")
        require(tuple(branch["properties"]) == members and tuple(branch["required"]) == members, "accepted member inventory drift")
        require(branch["properties"]["protocol_version"] == {"const": 2} and branch["properties"]["outcome"] == {"const": "finalized_accepted"}, "accepted header drift")
        require(branch["properties"]["operation"] == {"const": operation}, "accepted operation pair drift")
        require(branch["properties"]["coordinate"] == {"$ref": "#/$defs/ledger_coordinate"}, "accepted coordinate drift")
        require(branch["properties"]["effect"] == {"$ref": f"#/$defs/{effect}_effect"}, "accepted effect pair drift")
        require(branch["properties"]["projection"] == {"$ref": "#/$defs/accepted_projection"}, "accepted projection drift")


def artifact_hash(path: Path, expected: str, where: str) -> bytes:
    data = path.read_bytes()
    require(digest(data) == expected, f"{where}: locked SHA-256 drift")
    return data


def assert_text256(value: Any, where: str) -> None:
    require(isinstance(value, str), f"{where}: Text256 type")
    encoded = value.encode("utf-8")
    require(1 <= len(encoded) <= 256 and "\x00" not in value and re.search(r"\S", value) is not None, f"{where}: Text256 value")


def assert_namespace(value: Any, where: str) -> None:
    require(isinstance(value, str) and re.fullmatch(r"[a-z][a-z0-9._-]{0,63}", value, re.ASCII) is not None, f"{where}: namespace")


def assert_uuid(value: Any, where: str) -> None:
    require(isinstance(value, str) and re.fullmatch(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}", value) is not None, f"{where}: UUID")


def assert_u64(value: Any, where: str, *, nonzero: bool = False) -> int:
    require(isinstance(value, str) and re.fullmatch(r"0|[1-9][0-9]{0,19}", value) is not None, f"{where}: u64 text")
    parsed = int(value)
    require(parsed <= (1 << 64) - 1 and (not nonzero or parsed > 0), f"{where}: u64 bound")
    return parsed


def assert_u32(value: Any, where: str) -> None:
    require(type(value) is int and 0 <= value <= (1 << 32) - 1, f"{where}: u32")


def assert_hash(value: Any, where: str) -> None:
    require(isinstance(value, str) and re.fullmatch(r"0x[0-9a-f]{64}", value) is not None, f"{where}: hash32")


def assert_reference(value: Any, where: str) -> None:
    exact_keys(value, ("namespace", "scope", "value"), where)
    assert_namespace(value["namespace"], f"{where}.namespace")
    assert_text256(value["scope"], f"{where}.scope")
    assert_text256(value["value"], f"{where}.value")
    if value["namespace"] == "git.commit.sha1":
        require(re.fullmatch(r"[0-9a-f]{40}", value["value"]) is not None, f"{where}: SHA-1 spelling")
    if value["namespace"] == "git.commit.sha256":
        require(re.fullmatch(r"[0-9a-f]{64}", value["value"]) is not None, f"{where}: SHA-256 spelling")


def assert_workflow(value: Any, where: str) -> None:
    exact_keys(value, ("id", "phases", "initial_phase", "edges", "completion_phases"), where)
    assert_text256(value["id"], f"{where}.id")
    phases = value["phases"]
    require(isinstance(phases, list) and 1 <= len(phases) <= 32, f"{where}: phase count")
    for index, phase in enumerate(phases):
        assert_text256(phase, f"{where}.phases[{index}]")
    require(len(phases) == len(set(phases)), f"{where}: duplicate phases")
    assert_text256(value["initial_phase"], f"{where}.initial_phase")
    require(value["initial_phase"] in phases, f"{where}: initial phase absent")
    edges = value["edges"]
    require(isinstance(edges, list) and len(edges) <= 128, f"{where}: edge count")
    edge_keys: list[tuple[str, str]] = []
    for index, edge in enumerate(edges):
        exact_keys(edge, ("from", "to"), f"{where}.edges[{index}]")
        assert_text256(edge["from"], f"{where}.edges[{index}].from")
        assert_text256(edge["to"], f"{where}.edges[{index}].to")
        require(edge["from"] in phases and edge["to"] in phases, f"{where}: edge endpoint absent")
        edge_keys.append((edge["from"], edge["to"]))
    require(len(edge_keys) == len(set(edge_keys)), f"{where}: duplicate edge")
    completions = value["completion_phases"]
    require(isinstance(completions, list) and len(completions) <= 32, f"{where}: completion count")
    for index, phase in enumerate(completions):
        assert_text256(phase, f"{where}.completion_phases[{index}]")
        require(phase in phases, f"{where}: completion phase absent")
    require(len(completions) == len(set(completions)), f"{where}: duplicate completion phase")


def assert_definition(value: Any, where: str) -> None:
    exact_keys(value, ("id", "version"), where)
    assert_namespace(value["id"], f"{where}.id")
    assert_u64(value["version"], f"{where}.version", nonzero=True)


def assert_relationship(value: Any, where: str) -> None:
    exact_keys(value, ("definition", "source_id", "target_id"), where)
    assert_definition(value["definition"], f"{where}.definition")
    assert_uuid(value["source_id"], f"{where}.source_id")
    assert_uuid(value["target_id"], f"{where}.target_id")


def assert_subject(value: Any, where: str) -> None:
    require(isinstance(value, dict), f"{where}: subject object")
    if value.get("type") == "whole_unit":
        exact_keys(value, ("type",), where)
    elif value.get("type") == "revision":
        exact_keys(value, ("type", "revision"), where)
        assert_u64(value["revision"], f"{where}.revision")
    else:
        fail(f"{where}: subject tag")


def assert_association(value: Any, where: str) -> None:
    exact_keys(value, ("unit_id", "subject", "reference"), where)
    assert_uuid(value["unit_id"], f"{where}.unit_id")
    assert_subject(value["subject"], f"{where}.subject")
    assert_reference(value["reference"], f"{where}.reference")


def assert_coordinate(value: Any, where: str, *, finalized_only: bool = False) -> None:
    keys = (
        "parachain_genesis_hash", "deployment_id", "block_number", "block_hash",
        "extrinsic_index", "extrinsic_hash",
    )
    if not finalized_only:
        keys += ("system_event_index", "global_sequence")
    exact_keys(value, keys, where)
    for member in ("parachain_genesis_hash", "deployment_id", "block_hash", "extrinsic_hash"):
        assert_hash(value[member], f"{where}.{member}")
    require(value["parachain_genesis_hash"] == PARACHAIN_GENESIS, f"{where}: genesis drift")
    require(value["deployment_id"] == DEPLOYMENT_ID, f"{where}: deployment drift")
    block = assert_u64(value["block_number"], f"{where}.block_number")
    require(str(block) in BLOCK_HASHES and value["block_hash"] == BLOCK_HASHES[str(block)], f"{where}: joined block hash")
    assert_u32(value["extrinsic_index"], f"{where}.extrinsic_index")
    if not finalized_only:
        assert_u32(value["system_event_index"], f"{where}.system_event_index")
        assert_u64(value["global_sequence"], f"{where}.global_sequence", nonzero=True)


def assert_checkpoint(value: Any, where: str) -> None:
    exact_keys(value, ("block_number", "block_hash", "last_global_sequence", "runtime_spec_version", "runtime_code_hash"), where)
    block = assert_u64(value["block_number"], f"{where}.block_number")
    require(str(block) in BLOCK_HASHES and value["block_hash"] == BLOCK_HASHES[str(block)], f"{where}: checkpoint block join")
    if value["last_global_sequence"] is not None:
        assert_u64(value["last_global_sequence"], f"{where}.last_global_sequence", nonzero=True)
    assert_u32(value["runtime_spec_version"], f"{where}.runtime_spec_version")
    require(value["runtime_spec_version"] == 1 and value["runtime_code_hash"] == RUNTIME_CODE_HASH, f"{where}: runtime pin drift")


def assert_unit_base(value: Any, where: str, *, projected: bool) -> None:
    keys = ("id", "origin", "species", "workflow", "phase", "status", "revision", "history")
    if projected:
        keys += ("last_coordinate",)
    exact_keys(value, keys, where)
    assert_uuid(value["id"], f"{where}.id")
    assert_reference(value["origin"], f"{where}.origin")
    assert_text256(value["species"], f"{where}.species")
    assert_workflow(value["workflow"], f"{where}.workflow")
    assert_text256(value["phase"], f"{where}.phase")
    require(value["status"] in ("active", "completed"), f"{where}: status")
    revision = assert_u64(value["revision"], f"{where}.revision")
    history = value["history"]
    require(isinstance(history, list) and len(history) <= 256 and revision == len(history), f"{where}: history/revision")
    phase = value["workflow"]["initial_phase"]
    completed = False
    for index, record in enumerate(history):
        require(record.get("sequence") == str(index + 1), f"{where}: history sequence")
        if record.get("type") == "transition":
            exact_keys(record, ("type", "sequence", "from", "to"), f"{where}.history[{index}]")
            require(not completed and record["from"] == phase and record["to"] in value["workflow"]["phases"], f"{where}: transition history")
            phase = record["to"]
        elif record.get("type") == "completion":
            exact_keys(record, ("type", "sequence", "phase"), f"{where}.history[{index}]")
            require(not completed and record["phase"] == phase, f"{where}: completion history")
            completed = True
        else:
            fail(f"{where}: history tag")
    require(value["phase"] == phase and value["status"] == ("completed" if completed else "active"), f"{where}: aggregate state")
    if projected:
        assert_coordinate(value["last_coordinate"], f"{where}.last_coordinate")


def assert_summary(value: Any, where: str) -> None:
    exact_keys(value, ("id", "origin", "species", "workflow_id", "phase", "status", "revision", "last_coordinate"), where)
    assert_uuid(value["id"], f"{where}.id")
    assert_reference(value["origin"], f"{where}.origin")
    for member in ("species", "workflow_id", "phase"):
        assert_text256(value[member], f"{where}.{member}")
    require(value["status"] in ("active", "completed"), f"{where}: status")
    assert_u64(value["revision"], f"{where}.revision")
    assert_coordinate(value["last_coordinate"], f"{where}.last_coordinate")


def assert_result(value: Any, where: str) -> str:
    require(isinstance(value, dict) and value.get("type") in RESULT_TAGS, f"{where}: result tag")
    tag = value["type"]
    if tag == "intent_unit":
        exact_keys(value, ("type", "intent_unit", "checkpoint"), where)
        assert_unit_base(value["intent_unit"], f"{where}.intent_unit", projected=True)
    elif tag == "intent_unit_page":
        exact_keys(value, ("type", "items", "next_cursor", "checkpoint"), where)
        require(isinstance(value["items"], list) and len(value["items"]) <= 100, f"{where}: page bound")
        for index, item in enumerate(value["items"]):
            assert_summary(item, f"{where}.items[{index}]")
        if value["next_cursor"] is not None:
            assert_uuid(value["next_cursor"], f"{where}.next_cursor")
    elif tag == "relationship_definition":
        exact_keys(value, ("type", "definition", "checkpoint"), where)
        definition = value["definition"]
        allowed = ("key", "directed", "source_species", "target_species", "self_policy", "cycle_policy", "created_coordinate")
        require(tuple(definition) in (
            allowed,
            ("key", "directed", "self_policy", "cycle_policy", "created_coordinate"),
            ("key", "directed", "source_species", "self_policy", "cycle_policy", "created_coordinate"),
            ("key", "directed", "target_species", "self_policy", "cycle_policy", "created_coordinate"),
        ), f"{where}.definition: optional member order")
        assert_definition(definition["key"], f"{where}.definition.key")
        require(definition["directed"] is True, f"{where}: fixed direction")
        for member in ("source_species", "target_species"):
            if member in definition:
                assert_text256(definition[member], f"{where}.definition.{member}")
        require(definition["self_policy"] in ("allow", "reject") and definition["cycle_policy"] in ("allow", "reject"), f"{where}: policy")
        assert_coordinate(definition["created_coordinate"], f"{where}.definition.created_coordinate")
    elif tag == "relationship_page":
        exact_keys(value, ("type", "items", "next_cursor", "checkpoint"), where)
        require(isinstance(value["items"], list) and len(value["items"]) <= 100, f"{where}: page bound")
        for index, item in enumerate(value["items"]):
            exact_keys(item, ("key", "created_coordinate"), f"{where}.items[{index}]")
            assert_relationship(item["key"], f"{where}.items[{index}].key")
            assert_coordinate(item["created_coordinate"], f"{where}.items[{index}].created_coordinate")
        if value["next_cursor"] is not None:
            assert_relationship(value["next_cursor"], f"{where}.next_cursor")
    elif tag == "projection_v1_page":
        exact_keys(value, ("type", "query_version", "items", "next_cursor", "checkpoint"), where)
        require(value["query_version"] == 1 and isinstance(value["items"], list) and len(value["items"]) <= 100, f"{where}: projection query")
        for index, item in enumerate(value["items"]):
            assert_summary(item, f"{where}.items[{index}]")
        if value["next_cursor"] is not None:
            assert_uuid(value["next_cursor"], f"{where}.next_cursor")
    else:
        exact_keys(value, ("type", "direction", "items", "next_cursor", "checkpoint"), where)
        require(value["direction"] in ("by_unit", "by_reference") and isinstance(value["items"], list) and len(value["items"]) <= 100, f"{where}: association page")
        for index, item in enumerate(value["items"]):
            exact_keys(item, ("key", "created_coordinate"), f"{where}.items[{index}]")
            assert_association(item["key"], f"{where}.items[{index}].key")
            assert_coordinate(item["created_coordinate"], f"{where}.items[{index}].created_coordinate")
        if value["next_cursor"] is not None:
            assert_association(value["next_cursor"], f"{where}.next_cursor")
    assert_checkpoint(value["checkpoint"], f"{where}.checkpoint")
    return tag


def assert_error_detail(value: Any, where: str, allowed: tuple[str, ...]) -> str:
    require(isinstance(value, dict), f"{where}: error object")
    code = value.get("code")
    require(code in allowed and code in MESSAGES, f"{where}: illegal code")
    require(value.get("message") == MESSAGES[code], f"{where}: message drift")
    assert_text256(value["message"], f"{where}.message")
    if code in FIELD_CODES:
        exact_keys(value, ("code", "message", "field"), where)
        field = value["field"]
        require(isinstance(field, str) and len(field.encode()) <= 256 and "\x00" not in field and re.fullmatch(r"(?:/(?:[^~/\x00]|~[01])*)*", field) is not None, f"{where}: RFC 6901 field")
    elif code == "revision_conflict":
        exact_keys(value, ("code", "message", "expected_revision", "actual_revision"), where)
        assert_u64(value["expected_revision"], f"{where}.expected_revision")
        assert_u64(value["actual_revision"], f"{where}.actual_revision")
    else:
        exact_keys(value, ("code", "message"), where)
    require("operation_number" not in value, f"{where}: local error leaked operation_number")
    return code


def assert_effect(value: Any, operation: str, where: str) -> None:
    expected = EFFECT_BY_OPERATION[operation]
    require(isinstance(value, dict) and value.get("type") == expected, f"{where}: operation/effect mismatch")
    if expected in ("unit_created", "unit_transitioned", "unit_completed"):
        exact_keys(value, ("type", "unit_id", "committed_revision"), where)
        assert_uuid(value["unit_id"], f"{where}.unit_id")
        assert_u64(value["committed_revision"], f"{where}.committed_revision")
    elif expected == "relationship_definition_created":
        exact_keys(value, ("type", "definition"), where)
        assert_definition(value["definition"], f"{where}.definition")
    elif expected in ("relationship_created", "relationship_deleted"):
        exact_keys(value, ("type", "relationship"), where)
        assert_relationship(value["relationship"], f"{where}.relationship")
    else:
        exact_keys(value, ("type", "association"), where)
        assert_association(value["association"], f"{where}.association")


def assert_response(value: Any, case_id: str, exit_code: int) -> tuple[str | None, str | None]:
    require(isinstance(value, dict) and value.get("protocol_version") == 2, f"{case_id}: response version")
    outcome = value.get("outcome")
    if outcome == "success":
        exact_keys(value, ("protocol_version", "outcome", "result"), case_id)
        require(exit_code == 0, f"{case_id}: read success exit")
        return None, assert_result(value["result"], f"{case_id}.result")
    if outcome == "error":
        exact_keys(value, ("protocol_version", "outcome", "error"), case_id)
        code = value["error"].get("code") if isinstance(value["error"], dict) else None
        if code in ("malformed_json", "request_too_large"):
            allowed = ("malformed_json", "request_too_large")
            require(exit_code == 2, f"{case_id}: parse exit")
        elif code in FIELD_CODES:
            allowed = FIELD_CODES
            require(exit_code == 2, f"{case_id}: value exit")
        elif code in GENERIC_EXIT4:
            allowed = GENERIC_EXIT4
            require(exit_code == 4, f"{case_id}: infrastructure exit")
        elif code in ("dev_signer_unavailable", "submission_lane_corrupt"):
            allowed = ("dev_signer_unavailable", "submission_lane_corrupt")
            require(exit_code == 1, f"{case_id}: operational exit")
        else:
            allowed = ("intent_unit_not_found", "relationship_definition_not_found")
            require(exit_code == 3, f"{case_id}: read miss exit")
        return assert_error_detail(value["error"], f"{case_id}.error", allowed), None
    require(outcome in MUTATION_OUTCOMES, f"{case_id}: unknown outcome")
    operation = value.get("operation")
    require(operation in MUTATIONS, f"{case_id}: mutation operation")
    if outcome == "submission_rejected":
        exact_keys(value, ("protocol_version", "outcome", "operation", "error"), case_id)
        code = assert_error_detail(value["error"], f"{case_id}.error", SUBMISSION_REJECTED_CODES)
        require(exit_code == 3, f"{case_id}: submission rejection exit")
    elif outcome in ("submission_lane_unresolved", "expired_not_included", "delivery_indeterminate"):
        exact_keys(value, ("protocol_version", "outcome", "operation", "expected_extrinsic_hash", "era", "error"), case_id)
        assert_hash(value["expected_extrinsic_hash"], f"{case_id}.expected_extrinsic_hash")
        exact_keys(value["era"], ("birth", "death"), f"{case_id}.era")
        birth = assert_u64(value["era"]["birth"], f"{case_id}.era.birth")
        death = assert_u64(value["era"]["death"], f"{case_id}.era.death")
        require(death == birth + 63, f"{case_id}: 64-block era")
        allowed = {
            "submission_lane_unresolved": ("submission_lane_unresolved",),
            "expired_not_included": ("expired_not_included",),
            "delivery_indeterminate": DELIVERY_CODES,
        }[outcome]
        code = assert_error_detail(value["error"], f"{case_id}.error", allowed)
        require(exit_code == 1, f"{case_id}: unresolved exit")
    elif outcome == "finalized_dispatch_rejected":
        exact_keys(value, ("protocol_version", "outcome", "operation", "finalized_extrinsic", "error"), case_id)
        assert_coordinate(value["finalized_extrinsic"], f"{case_id}.finalized_extrinsic", finalized_only=True)
        code = assert_error_detail(value["error"], f"{case_id}.error", DISPATCH_CODES)
        require(exit_code == 3, f"{case_id}: dispatch exit")
    elif outcome == "finalized_invariant_failed":
        exact_keys(value, ("protocol_version", "outcome", "operation", "finalized_extrinsic", "error"), case_id)
        assert_coordinate(value["finalized_extrinsic"], f"{case_id}.finalized_extrinsic", finalized_only=True)
        code = assert_error_detail(value["error"], f"{case_id}.error", ("finalized_invariant_failed",))
        require(exit_code == 1, f"{case_id}: invariant exit")
    else:
        exact_keys(value, ("protocol_version", "outcome", "operation", "coordinate", "effect", "projection"), case_id)
        require(exit_code == 0, f"{case_id}: accepted exit")
        assert_coordinate(value["coordinate"], f"{case_id}.coordinate")
        assert_effect(value["effect"], operation, f"{case_id}.effect")
        projection = value["projection"]
        require(isinstance(projection, dict) and projection.get("status") in ("caught_up", "lagging"), f"{case_id}: projection status")
        exact_keys(projection, ("status", "checkpoint"), f"{case_id}.projection")
        if projection["status"] == "caught_up":
            require(projection["checkpoint"] is not None, f"{case_id}: caught_up checkpoint")
        if projection["checkpoint"] is not None:
            assert_checkpoint(projection["checkpoint"], f"{case_id}.projection.checkpoint")
            if projection["status"] == "caught_up":
                sequence = projection["checkpoint"]["last_global_sequence"]
                require(sequence is not None and int(sequence) >= int(value["coordinate"]["global_sequence"]), f"{case_id}: caught_up sequence behind acceptance")
        code = None
    return code, None


def assert_filters(value: Any, where: str) -> None:
    require(isinstance(value, dict), f"{where}: filters object")
    allowed = ("workflow_id", "species", "phase", "status")
    require(tuple(value) == tuple(member for member in allowed if member in value), f"{where}: filter order/unknown")
    for member in ("workflow_id", "species", "phase"):
        if member in value:
            assert_text256(value[member], f"{where}.{member}")
    if "status" in value:
        require(value["status"] in ("active", "completed"), f"{where}: status")


OPERATION_FIELDS = {
    "create_intent_unit": (("type", "intent_unit", "workflow"), ()),
    "get_intent_unit": (("type", "id"), ()),
    "list_intent_units": (("type", "filters", "limit", "after"), ("after",)),
    "transition_intent_unit": (("type", "id", "target", "expected_revision"), ()),
    "complete_intent_unit": (("type", "id", "expected_revision"), ()),
    "create_relationship_definition": (("type", "definition", "source_species", "target_species", "self_policy", "cycle_policy"), ("source_species", "target_species")),
    "get_relationship_definition": (("type", "definition"), ()),
    "create_relationship": (("type", "relationship"), ()),
    "delete_relationship": (("type", "relationship"), ()),
    "list_relationships": (("type", "definition", "source_id", "target_id", "limit", "after"), ("source_id", "target_id", "after")),
    "project_intent_units_v1": (("type", "query_version", "filters", "predicate", "limit", "after"), ("predicate", "after")),
    "record_association": (("type", "association"), ()),
    "revoke_association": (("type", "association"), ()),
    "list_associations_by_unit": (("type", "unit_id", "subject", "limit", "after"), ("subject", "after")),
    "list_associations_by_reference": (("type", "reference", "limit", "after"), ("after",)),
}


def assert_valid_request(value: Any, where: str) -> str:
    exact_keys(value, ("protocol_version", "operation"), where)
    require(value["protocol_version"] == 2, f"{where}: protocol version")
    operation = value["operation"]
    require(isinstance(operation, dict) and operation.get("type") in OPERATIONS, f"{where}: operation tag")
    tag = operation["type"]
    allowed, optional = OPERATION_FIELDS[tag]
    expected = tuple(member for member in allowed if member not in optional or member in operation)
    exact_keys(operation, expected, f"{where}.operation")
    if tag == "create_intent_unit":
        intent = operation["intent_unit"]
        require(tuple(intent) in (("origin", "species"), ("id", "origin", "species")), f"{where}: intent input fields")
        if "id" in intent:
            assert_uuid(intent["id"], f"{where}.operation.intent_unit.id")
        assert_reference(intent["origin"], f"{where}.operation.intent_unit.origin")
        assert_text256(intent["species"], f"{where}.operation.intent_unit.species")
        assert_workflow(operation["workflow"], f"{where}.operation.workflow")
    elif tag == "get_intent_unit":
        assert_uuid(operation["id"], f"{where}.operation.id")
    elif tag == "list_intent_units":
        assert_filters(operation["filters"], f"{where}.operation.filters")
        require(type(operation["limit"]) is int and 1 <= operation["limit"] <= 100, f"{where}: limit")
        if "after" in operation:
            assert_uuid(operation["after"], f"{where}.operation.after")
    elif tag in ("transition_intent_unit", "complete_intent_unit"):
        assert_uuid(operation["id"], f"{where}.operation.id")
        if tag == "transition_intent_unit":
            assert_text256(operation["target"], f"{where}.operation.target")
        assert_u64(operation["expected_revision"], f"{where}.operation.expected_revision")
    elif tag == "create_relationship_definition":
        assert_definition(operation["definition"], f"{where}.operation.definition")
        for member in ("source_species", "target_species"):
            if member in operation:
                assert_text256(operation[member], f"{where}.operation.{member}")
        require(operation["self_policy"] in ("allow", "reject") and operation["cycle_policy"] in ("allow", "reject"), f"{where}: policy")
    elif tag == "get_relationship_definition":
        assert_definition(operation["definition"], f"{where}.operation.definition")
    elif tag in ("create_relationship", "delete_relationship"):
        assert_relationship(operation["relationship"], f"{where}.operation.relationship")
    elif tag == "list_relationships":
        assert_definition(operation["definition"], f"{where}.operation.definition")
        for member in ("source_id", "target_id"):
            if member in operation:
                assert_uuid(operation[member], f"{where}.operation.{member}")
        require(type(operation["limit"]) is int and 1 <= operation["limit"] <= 100, f"{where}: limit")
        if "after" in operation:
            assert_relationship(operation["after"], f"{where}.operation.after")
            require(operation["after"]["definition"] == operation["definition"], f"{where}: cursor definition")
    elif tag == "project_intent_units_v1":
        require(operation["query_version"] == 1, f"{where}: query version")
        assert_filters(operation["filters"], f"{where}.operation.filters")
        if "predicate" in operation:
            predicate = operation["predicate"]
            exact_keys(predicate, ("type", "definition", "anchor_id"), f"{where}.operation.predicate")
            require(predicate["type"] in ("outgoing", "incoming"), f"{where}: predicate tag")
            assert_definition(predicate["definition"], f"{where}.operation.predicate.definition")
            assert_uuid(predicate["anchor_id"], f"{where}.operation.predicate.anchor_id")
        require(type(operation["limit"]) is int and 1 <= operation["limit"] <= 100, f"{where}: limit")
        if "after" in operation:
            assert_uuid(operation["after"], f"{where}.operation.after")
    elif tag in ("record_association", "revoke_association"):
        assert_association(operation["association"], f"{where}.operation.association")
    elif tag == "list_associations_by_unit":
        assert_uuid(operation["unit_id"], f"{where}.operation.unit_id")
        if "subject" in operation:
            assert_subject(operation["subject"], f"{where}.operation.subject")
        require(type(operation["limit"]) is int and 1 <= operation["limit"] <= 100, f"{where}: limit")
        if "after" in operation:
            assert_association(operation["after"], f"{where}.operation.after")
            require(operation["after"]["unit_id"] == operation["unit_id"], f"{where}: association unit cursor")
            if "subject" in operation:
                require(operation["after"]["subject"] == operation["subject"], f"{where}: association subject cursor")
    else:
        assert_reference(operation["reference"], f"{where}.operation.reference")
        require(type(operation["limit"]) is int and 1 <= operation["limit"] <= 100, f"{where}: limit")
        if "after" in operation:
            assert_association(operation["after"], f"{where}.operation.after")
            require(operation["after"]["reference"] == operation["reference"], f"{where}: association reference cursor")
    return tag


def compact_encode(value: int) -> bytes:
    require(0 <= value < (1 << 30), "compact encoder bound")
    if value < 64:
        return bytes([value << 2])
    if value < 16384:
        return ((value << 2) | 1).to_bytes(2, "little")
    return ((value << 2) | 2).to_bytes(4, "little")


def compact_decode(raw: bytes, offset: int = 0) -> tuple[int, int]:
    require(offset < len(raw), "truncated compact")
    mode = raw[offset] & 3
    width = (1, 2, 4, (raw[offset] >> 2) + 5)[mode]
    require(offset + width <= len(raw), "truncated compact body")
    if mode == 0:
        value = raw[offset] >> 2
    elif mode in (1, 2):
        value = int.from_bytes(raw[offset : offset + width], "little") >> 2
    else:
        value = int.from_bytes(raw[offset + 1 : offset + width], "little")
    require(raw[offset : offset + width] == compact_encode(value), "noncanonical compact")
    return value, offset + width


def scale_text(value: str) -> bytes:
    encoded = value.encode()
    return compact_encode(len(encoded)) + encoded


def uuid_bytes(value: str) -> bytes:
    return bytes.fromhex(value.replace("-", ""))


def scale_reference(value: dict[str, Any]) -> bytes:
    return scale_text(value["namespace"]) + scale_text(value["scope"]) + scale_text(value["value"])


def scale_workflow(value: dict[str, Any]) -> bytes:
    out = bytearray(scale_text(value["id"]))
    out += compact_encode(len(value["phases"]))
    for phase in value["phases"]:
        out += scale_text(phase)
    out += scale_text(value["initial_phase"])
    out += compact_encode(len(value["edges"]))
    for edge in value["edges"]:
        out += scale_text(edge["from"]) + scale_text(edge["to"])
    out += compact_encode(len(value["completion_phases"]))
    for phase in value["completion_phases"]:
        out += scale_text(phase)
    return bytes(out)


def scale_definition(value: dict[str, Any]) -> bytes:
    return scale_text(value["id"]) + int(value["version"]).to_bytes(8, "little")


def scale_relationship(value: dict[str, Any]) -> bytes:
    return scale_definition(value["definition"]) + uuid_bytes(value["source_id"]) + uuid_bytes(value["target_id"])


def scale_subject(value: dict[str, Any]) -> bytes:
    if value["type"] == "whole_unit":
        return b"\x00"
    return b"\x01" + int(value["revision"]).to_bytes(8, "little")


def scale_association(value: dict[str, Any]) -> bytes:
    return uuid_bytes(value["unit_id"]) + scale_subject(value["subject"]) + scale_reference(value["reference"])


def expected_call(request: dict[str, Any], generated_uuid: str | None) -> bytes:
    operation = request["operation"]
    tag = operation["type"]
    call_index = {
        "create_intent_unit": 0,
        "transition_intent_unit": 1,
        "complete_intent_unit": 2,
        "create_relationship_definition": 4,
        "create_relationship": 5,
        "delete_relationship": 6,
        "record_association": 7,
        "revoke_association": 8,
    }[tag]
    out = bytearray((0x32, call_index, 1, 0))
    if tag == "create_intent_unit":
        intent = operation["intent_unit"]
        unit_id = intent.get("id", generated_uuid)
        require(unit_id is not None, "create signer vector lacks generated UUID")
        out += uuid_bytes(unit_id)
        out += scale_reference(intent["origin"])
        out += scale_text(intent["species"])
        out += scale_workflow(operation["workflow"])
    elif tag == "transition_intent_unit":
        out += uuid_bytes(operation["id"])
        out += scale_text(operation["target"])
        out += int(operation["expected_revision"]).to_bytes(8, "little")
    elif tag == "complete_intent_unit":
        out += uuid_bytes(operation["id"])
        out += int(operation["expected_revision"]).to_bytes(8, "little")
    elif tag == "create_relationship_definition":
        out += scale_definition(operation["definition"])
        # Runtime field is fixed Directed (variant 0); JSON never exposes it.
        out += b"\x00"
        for member in ("source_species", "target_species"):
            out += b"\x00" if member not in operation else b"\x01" + scale_text(operation[member])
        out += bytes([0 if operation["self_policy"] == "allow" else 1])
        out += bytes([0 if operation["cycle_policy"] == "allow" else 1])
    elif tag in ("create_relationship", "delete_relationship"):
        out += scale_relationship(operation["relationship"])
    else:
        out += scale_association(operation["association"])
    return bytes(out)


# Minimal independent Merlin/STROBE-128 and Ristretto255 verifier. It uses only
# Python integer arithmetic and does not call the production signer or decoder.
FIELD_P = (1 << 255) - 19
SCALAR_L = (1 << 252) + 27742317777372353535851937790883648493
EDWARDS_D = (-121665 * pow(121666, FIELD_P - 2, FIELD_P)) % FIELD_P
SQRT_M1 = pow(2, (FIELD_P - 1) // 4, FIELD_P)
if SQRT_M1 & 1:
    SQRT_M1 = FIELD_P - SQRT_M1
RISTRETTO_BASEPOINT = bytes.fromhex("e2f2ae0a6abc4e71a884a961c500515f58e30b6aa582dd8db6a65945e08d2d76")
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
        self.meta_ad(protocol_label)

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
            require(self.current_flags == flags, "STROBE continuation flags")
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
        require(len(message) <= (1 << 32) - 1, "Merlin message length")
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
    require(len(raw) == 32, "Ristretto length")
    scalar = int.from_bytes(raw, "little")
    require(scalar < FIELD_P and not scalar & 1, "Ristretto canonical encoding")
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
    require(ok and not t & 1 and y != 0, "Ristretto decompression")
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
    x, y, z, _ = point
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
    x1, y1, _, _ = left
    x2, y2, _, _ = right
    return (x1 * y2 - y1 * x2) % FIELD_P == 0 or (x1 * x2 - y1 * y2) % FIELD_P == 0


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
        calculated = point_add(point_multiply(response, ristretto_decompress(RISTRETTO_BASEPOINT)), point_multiply((-challenge) % SCALAR_L, public))
        return ristretto_equal(calculated, claimed_r)
    except ValueError:
        return False


def verify_signer_vector(value: Any, request: dict[str, Any], generated_uuid: str | None, where: str) -> str:
    exact_keys(
        value,
        (
            "context_schema_version", "authority", "production_signature_byte_equality_required",
            "dev_signer", "account_id", "operation", "command_schema_version", "metadata",
            "deployment_anchor", "nonce", "signing_finalized", "era", "period", "phase", "tip",
            "call_scale", "signer_payload", "payload_mode", "signature_scheme", "signature_context",
            "signature", "signed_extrinsic", "expected_extrinsic_hash", "new_sign_count", "new_send_count",
        ),
        where,
    )
    operation = value["operation"]
    require(value["context_schema_version"] == 1 and value["authority"] == "independent_oracle_only", f"{where}: authority")
    require(value["production_signature_byte_equality_required"] is False, f"{where}: production byte nonclaim")
    require(value["dev_signer"] == "charlie" and value["account_id"] == "0x" + CHARLIE.hex(), f"{where}: signer identity")
    require(operation == request["operation"]["type"] and operation in MUTATIONS, f"{where}: operation bind")
    require(value["command_schema_version"] == 1, f"{where}: command schema")
    metadata_path, metadata_bytes = validate_ref(value["metadata"], f"{where}.metadata")
    anchor_path, anchor_bytes = validate_ref(value["deployment_anchor"], f"{where}.deployment_anchor")
    require(metadata_path == ROOT / "chain/metadata/cubikan-runtime-v1.scale" and len(metadata_bytes) == 63327, f"{where}: metadata pin")
    require(anchor_path == ROOT / "chain/artifacts/local-deployment-anchor-v1.json" and len(anchor_bytes) == 5868, f"{where}: anchor pin")
    require(value["nonce"] == "66051" and value["tip"] == "0", f"{where}: nonce/tip")
    require(value["signing_finalized"] == {"block_number": "131", "block_hash": SIGNING_BLOCK_HASH}, f"{where}: signing finalized")
    require(value["era"] == {"birth": "131", "death": "194"} and value["period"] == 64 and value["phase"] == 3, f"{where}: era")
    call = bytes.fromhex(value["call_scale"].removeprefix("0x"))
    require(call == expected_call(request, generated_uuid), f"{where}: independently encoded call differs")
    tail = b"\x35\x00" + compact_encode(66051) + compact_encode(0) + (1).to_bytes(4, "little") + (1).to_bytes(4, "little") + bytes.fromhex(PARACHAIN_GENESIS[2:]) + bytes.fromhex(SIGNING_BLOCK_HASH[2:])
    payload = bytes.fromhex(value["signer_payload"].removeprefix("0x"))
    require(payload == call + tail and len(payload) % 64 != 0, f"{where}: payload bind/alignment")
    require(value["payload_mode"] == "raw_not_blake2_256" and len(payload) <= 256, f"{where}: payload mode")
    signature = bytes.fromhex(value["signature"].removeprefix("0x"))
    require(value["signature_scheme"] == "sr25519" and value["signature_context"] == "substrate", f"{where}: signature parameters")
    require(sr25519_verify(signature, payload, CHARLIE), f"{where}: independent sr25519 proof")
    mutated = bytearray(payload)
    mutated[0] ^= 1
    require(not sr25519_verify(signature, bytes(mutated), CHARLIE), f"{where}: signature accepted mutated payload")
    extrinsic = bytes.fromhex(value["signed_extrinsic"].removeprefix("0x"))
    body_len, offset = compact_decode(extrinsic)
    require(body_len == len(extrinsic) - offset and len(extrinsic) % 64 != 0, f"{where}: extrinsic length/alignment")
    body = extrinsic[offset:]
    require(body[:2] == b"\x84\x00" and body[2:34] == CHARLIE and body[34] == 1, f"{where}: signed address/signature tags")
    require(body[35:99] == signature and body[99:101] == b"\x35\x00", f"{where}: signature/era bind")
    nonce, cursor = compact_decode(body, 101)
    tip, cursor = compact_decode(body, cursor)
    require(nonce == 66051 and tip == 0 and body[cursor:] == call, f"{where}: signed extra/call decode")
    expected_hash = "0x" + hashlib.blake2b(extrinsic, digest_size=32).hexdigest()
    require(value["expected_extrinsic_hash"] == expected_hash, f"{where}: extrinsic hash")
    require(value["new_sign_count"] == 1 and value["new_send_count"] == 1, f"{where}: signer counters")
    return expected_hash


CONTEXT_CACHE: dict[Path, Any] = {}
SIGNER_PROOF_CACHE: dict[tuple[Path, str, str | None], str] = {}


def load_context_ref(ref: Any, where: str) -> tuple[Path, Any]:
    path, data = validate_ref(ref, where)
    value = load_json(data)
    require(canonical(value) == data, f"{where}: context must be compact canonical JSON")
    CONTEXT_CACHE[path] = value
    return path, value


def assert_rpc_context(value: Any, where: str) -> None:
    exact_keys(value, ("context_schema_version", "label", "endpoint", "proc_discovery", "dial_count", "calls", "terminal"), where)
    require(value["context_schema_version"] == 1 and isinstance(value["label"], str), f"{where}: header")
    require(value["endpoint"] is None or isinstance(value["endpoint"], str), f"{where}: endpoint")
    discovery = value["proc_discovery"]
    exact_keys(discovery, ("sorted_numeric_entries", "successful_archive_pids", "result"), f"{where}.proc_discovery")
    require(discovery["sorted_numeric_entries"] == sorted(discovery["sorted_numeric_entries"]), f"{where}: /proc sort")
    require(all(type(pid) is int and pid > 0 for pid in discovery["sorted_numeric_entries"]), f"{where}: synthetic PIDs")
    require(all(pid in discovery["sorted_numeric_entries"] for pid in discovery["successful_archive_pids"]), f"{where}: successful PID inventory")
    require(discovery["result"] in ("not_attempted", "unique", "zero", "multiple", "unsupported"), f"{where}: discovery result")
    require(type(value["dial_count"]) is int and value["dial_count"] in (0, 1), f"{where}: dial count")
    require(isinstance(value["calls"], list) and len(value["calls"]) == value["dial_count"], f"{where}: RPC call count")
    last_response: Any = None
    for index, call in enumerate(value["calls"]):
        exact_keys(call, ("sequence", "method", "raw_request", "raw_response"), f"{where}.calls[{index}]")
        require(call["sequence"] == index and isinstance(call["method"], str), f"{where}: RPC sequence")
        for member in ("raw_request", "raw_response"):
            require(isinstance(call[member], str), f"{where}: raw RPC text")
            parsed = load_json(call[member].encode())
            require(canonical(parsed).decode() == call[member], f"{where}: raw RPC canonical")
            if member == "raw_response":
                last_response = parsed
    require(isinstance(value["terminal"], dict), f"{where}: terminal")
    if value["calls"]:
        require(isinstance(last_response, dict) and last_response.get("result") == value["terminal"], f"{where}: terminal/raw response drift")


def assert_projection_context(value: Any, where: str) -> None:
    exact_keys(value, ("context_schema_version", "label", "database", "sync_count", "attest_count", "query_count", "terminal"), where)
    require(value["context_schema_version"] == 1 and isinstance(value["label"], str), f"{where}: header")
    exact_keys(value["database"], ("mode", "pre_submission_access_count", "open_count"), f"{where}.database")
    require(value["database"]["mode"] in ("create_if_absent_or_open_if_existing", "not_attempted"), f"{where}: database mode")
    require(value["database"]["pre_submission_access_count"] == 0, f"{where}: SQLite influenced mutation preflight")
    for member in ("open_count",):
        require(type(value["database"][member]) is int and value["database"][member] in (0, 1), f"{where}: {member}")
    for member in ("sync_count", "attest_count", "query_count"):
        require(type(value[member]) is int and value[member] in (0, 1), f"{where}: {member}")
    require(isinstance(value["terminal"], dict), f"{where}: terminal")


def assert_nonsign_context(value: Any, request_operation: str | None, where: str) -> None:
    keys = tuple(value)
    if "persisted_expected_extrinsic_hash" in value:
        exact_keys(value, ("context_schema_version", "authority", "dev_signer", "account_id", "operation", "persisted_expected_extrinsic_hash", "era", "terminal_code", "new_sign_count", "new_send_count"), where)
        assert_hash(value["persisted_expected_extrinsic_hash"], f"{where}.persisted_expected_extrinsic_hash")
        require(value["era"] == {"birth": "131", "death": "194"}, f"{where}: recovery era")
    else:
        exact_keys(value, ("context_schema_version", "authority", "dev_signer", "account_id", "operation", "phase", "terminal_code", "new_sign_count", "new_send_count"), where)
    require(value["context_schema_version"] == 1 and value["authority"] == "independent_oracle_only", f"{where}: authority")
    require(value["new_sign_count"] == 0 and value["new_send_count"] == 0, f"{where}: no-sign counters")
    if request_operation is not None and value["operation"] is not None:
        require(value["operation"] == request_operation, f"{where}: operation bind")
    require(value["terminal_code"] in ERROR_CODES, f"{where}: terminal code")
    if value["dev_signer"] is not None:
        require(value["dev_signer"] == "charlie" and value["account_id"] == "0x" + CHARLIE.hex(), f"{where}: signer identity")


def assert_manifest_context(context: Any, request: dict[str, Any] | None, response: dict[str, Any], case_id: str) -> dict[str, Any]:
    require(isinstance(context, dict), f"{case_id}.context: object")
    allowed = ("generated_uuid", "rpc", "signer", "projection")
    require(tuple(context) == tuple(member for member in allowed if member in context), f"{case_id}.context: member order/unknown")
    operation = None if request is None or not isinstance(request.get("operation"), dict) else request["operation"].get("type")
    if "generated_uuid" in context:
        assert_uuid(context["generated_uuid"], f"{case_id}.context.generated_uuid")
        require(operation == "create_intent_unit" and "id" not in request["operation"]["intent_unit"], f"{case_id}: generated UUID selector misuse")
    loaded: dict[str, Any] = {}
    if "rpc" in context:
        _, rpc = load_context_ref(context["rpc"], f"{case_id}.context.rpc")
        assert_rpc_context(rpc, f"{case_id}.context.rpc")
        loaded["rpc"] = rpc
    if "signer" in context:
        signer_path, signer = load_context_ref(context["signer"], f"{case_id}.context.signer")
        if "production_signature_byte_equality_required" in signer:
            require(request is not None, f"{case_id}: signer vector requires valid request")
            cache_key = (signer_path, digest(canonical(request)), context.get("generated_uuid"))
            if cache_key not in SIGNER_PROOF_CACHE:
                SIGNER_PROOF_CACHE[cache_key] = verify_signer_vector(signer, request, context.get("generated_uuid"), f"{case_id}.context.signer")
            loaded["expected_extrinsic_hash"] = SIGNER_PROOF_CACHE[cache_key]
        else:
            assert_nonsign_context(signer, operation, f"{case_id}.context.signer")
        loaded["signer"] = signer
    if "projection" in context:
        _, projection = load_context_ref(context["projection"], f"{case_id}.context.projection")
        assert_projection_context(projection, f"{case_id}.context.projection")
        loaded["projection"] = projection
    response_outcome = response.get("outcome")
    response_error = response.get("error") if isinstance(response.get("error"), dict) else {}
    if "rpc" in loaded:
        rpc = loaded["rpc"]
        terminal = rpc["terminal"]
        terminal_outcome = terminal.get("outcome")
        if terminal.get("phase") == "decode":
            require(rpc["dial_count"] == 0, f"{case_id}: decode barrier dialed RPC")
        elif terminal_outcome == "ready":
            require(operation in READS and "projection" in loaded, f"{case_id}: read-ready RPC lacks read projection")
        elif terminal_outcome is not None:
            require(terminal_outcome == response_outcome, f"{case_id}: RPC/stdout outcome drift")
        if terminal.get("code") is not None:
            require(terminal["code"] == response_error.get("code"), f"{case_id}: RPC/stdout code drift")
        if terminal.get("field") is not None:
            require(terminal["field"] == response_error.get("field"), f"{case_id}: RPC/stdout field drift")
        for member in ("coordinate", "effect", "finalized_extrinsic", "expected_extrinsic_hash", "era"):
            if member in terminal or member in response:
                require(terminal.get(member) == response.get(member), f"{case_id}: RPC/stdout {member} drift")
    if "projection" in loaded:
        projection = loaded["projection"]
        if projection["sync_count"] or projection["attest_count"]:
            require("rpc" in loaded and loaded["rpc"]["dial_count"] == 1, f"{case_id}: stateful projection lacks hashed RPC context")
        terminal = projection["terminal"]
        terminal_type = terminal.get("type")
        if terminal_type == "read_result":
            require(response_outcome == "success" and terminal.get("operation") == operation and terminal.get("result") == response.get("result"), f"{case_id}: read projection/stdout drift")
        elif terminal_type in ("read_error", "codec_error", "source_error"):
            require(response_outcome == "error" and terminal.get("code") == response_error.get("code"), f"{case_id}: projection/stdout code drift")
            if terminal.get("field") is not None:
                require(terminal["field"] == response_error.get("field"), f"{case_id}: projection/stdout field drift")
        elif terminal_type is None:
            require(terminal.get("phase") == "decode" and (projection["sync_count"], projection["attest_count"], projection["query_count"]) == (0, 0, 0), f"{case_id}: unknown projection terminal")
        else:
            require(terminal_type == "accepted_projection_probe", f"{case_id}: unknown projection terminal type")
    if "signer" in loaded and "production_signature_byte_equality_required" not in loaded["signer"]:
        terminal_code = loaded["signer"]["terminal_code"]
        if terminal_code != "invalid_request":
            require(terminal_code == response_error.get("code"), f"{case_id}: signer/stdout code drift")
    if response.get("outcome") == "finalized_accepted":
        require("rpc" in loaded and "signer" in loaded and "projection" in loaded, f"{case_id}: accepted context incomplete")
        require(loaded["expected_extrinsic_hash"] == response["coordinate"]["extrinsic_hash"], f"{case_id}: accepted hash not signed hash")
        projection = loaded["projection"]
        require(projection["database"]["pre_submission_access_count"] == 0, f"{case_id}: pre-submit SQLite")
        terminal = projection["terminal"]
        require(terminal.get("type") == "accepted_projection_probe" and terminal.get("projection") == response["projection"], f"{case_id}: projection response bind")
        anchor = terminal.get("semantic_anchor_query")
        require(isinstance(anchor, dict), f"{case_id}: semantic anchor query")
        if response["projection"]["status"] == "caught_up":
            require((projection["sync_count"], projection["attest_count"], projection["query_count"]) == (1, 1, 1), f"{case_id}: caught_up work counts")
            require(anchor.get("effect_present") is True and terminal.get("fallback_unit_page_checkpoint_probe") is None and terminal.get("lag_reason") is None, f"{case_id}: caught_up anchor proof")
        else:
            require(anchor.get("effect_present") is False, f"{case_id}: lagging must not claim effect")
            if response["projection"]["checkpoint"] is None:
                require(terminal.get("lag_reason") == "attestation_or_projection_error" and projection["attest_count"] == 0 and projection["query_count"] == 0, f"{case_id}: null lagging reason")
            else:
                require(terminal.get("lag_reason") == "semantic_anchor_missing" and terminal.get("fallback_unit_page_checkpoint_probe") is not None, f"{case_id}: bounded lag checkpoint probe")
    return loaded


def expected_legality(code: str) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = []
    if code in FIELD_CODES or code in ("malformed_json", "request_too_large"):
        result.append({"outcome": "error", "exit_code": 2})
    if code in GENERIC_EXIT4:
        result.append({"outcome": "error", "exit_code": 4})
    if code in ("dev_signer_unavailable", "submission_lane_corrupt"):
        result.append({"outcome": "error", "exit_code": 1})
    if code in ("intent_unit_not_found", "relationship_definition_not_found"):
        result.append({"outcome": "error", "exit_code": 3})
    if code in SUBMISSION_REJECTED_CODES:
        result.append({"outcome": "submission_rejected", "exit_code": 3})
    if code == "submission_lane_unresolved":
        result.append({"outcome": "submission_lane_unresolved", "exit_code": 1})
    if code == "expired_not_included":
        result.append({"outcome": "expired_not_included", "exit_code": 1})
    if code in DISPATCH_CODES:
        result.append({"outcome": "finalized_dispatch_rejected", "exit_code": 3})
    if code == "finalized_invariant_failed":
        result.append({"outcome": "finalized_invariant_failed", "exit_code": 1})
    if code in DELIVERY_CODES:
        result.append({"outcome": "delivery_indeterminate", "exit_code": 1})
    return result


def canonical_rpc_url(value: str) -> bool:
    match = re.fullmatch(r"ws://(127\.([0-9]{1,3})\.([0-9]{1,3})\.([0-9]{1,3})|\[::1\]):([0-9]{1,5})/", value)
    if match is None:
        return False
    host, octet2, octet3, octet4, port_text = match.groups()
    if host != "[::1]":
        for octet in (octet2, octet3, octet4):
            if str(int(octet)) != octet or int(octet) > 255:
                return False
    port = int(port_text)
    return str(port) == port_text and 1 <= port <= 65535 and port != 80


def validate_process_oracle(value: Any, case_by_id: dict[str, dict[str, Any]]) -> None:
    exact_keys(value, ("fixture_schema_version", "hash_algorithm", "usage", "cases"), "process")
    require(value["fixture_schema_version"] == 1 and value["hash_algorithm"] == "sha256", "process header")
    _, usage = validate_ref(value["usage"], "process.usage")
    require(usage == b"usage: cubikan-local --database PATH --rpc URL [--dev-signer charlie|dave]\n", "process usage bytes")
    require(len(value["cases"]) == 72, "process case count")
    ids = [case["id"] for case in value["cases"]]
    require(len(ids) == len(set(ids)), "duplicate process case ID")
    for index, case in enumerate(value["cases"]):
        where = f"process.cases[{index}]"
        exact_keys(case, ("id", "argv", "environment", "request", "stdout", "stderr", "exit_code", "stdin_bytes_consumed", "dial_count", "sqlite_access_count", "submission_count", "acknowledge_count"), where)
        require(isinstance(case["id"], str) and isinstance(case["argv"], list) and case["argv"] and case["argv"][0] == "cubikan-local", f"{where}: argv")
        require(all(isinstance(arg, str) for arg in case["argv"]), f"{where}: argv type")
        require(isinstance(case["environment"], dict) and all(isinstance(key, str) and isinstance(item, str) for key, item in case["environment"].items()), f"{where}: environment")
        _, request_bytes = validate_ref(case["request"], f"{where}.request")
        _, stdout = validate_ref(case["stdout"], f"{where}.stdout")
        _, stderr = validate_ref(case["stderr"], f"{where}.stderr")
        for member in ("exit_code", "stdin_bytes_consumed", "dial_count", "sqlite_access_count", "submission_count", "acknowledge_count"):
            require(type(case[member]) is int and case[member] >= 0, f"{where}: {member}")
        require(case["stdin_bytes_consumed"] <= min(len(request_bytes), 1_048_577), f"{where}: stdin retention bound")
        tail = case["argv"][1:]
        structurally_four = len(tail) == 4 and tail[0] == "--database" and tail[2] == "--rpc" and tail[1] != ""
        structurally_six = len(tail) == 6 and tail[0] == "--database" and tail[2] == "--rpc" and tail[4] == "--dev-signer" and tail[1] != ""
        structural = structurally_four or structurally_six
        if case["stdin_bytes_consumed"] == 0 and stderr == usage:
            require(not structural, f"{where}: structurally valid argv incorrectly precedes stdin")
            require(stdout == b"" and case["exit_code"] == 2, f"{where}: structural rejection delivery")
        if case["id"] in {"argv_read_with_signer", "argv_mutation_without_signer", "argv_unknown_signer", "argv_uppercase_signer", "argv_environment_cannot_fill_signer"}:
            require(structural and case["stdin_bytes_consumed"] == len(request_bytes), f"{where}: signer form must follow decode")
            require(stdout == b"" and stderr == usage and case["exit_code"] == 2, f"{where}: signer usage")
        if case["id"].startswith("url_reject_"):
            require(structurally_four and not canonical_rpc_url(tail[3]), f"{where}: URL rejection spelling")
            response = load_json(stdout[:-1])
            require(response["error"]["code"] == "invalid_rpc_endpoint" and case["dial_count"] == 0, f"{where}: URL reject outcome")
        if case["id"].startswith("url_accept_"):
            require(structurally_four and canonical_rpc_url(tail[3]), f"{where}: URL acceptance spelling")
            require((case["exit_code"], case["dial_count"], case["sqlite_access_count"], case["acknowledge_count"]) == (0, 1, 1, 1), f"{where}: read counts")
        if stdout:
            require(stdout.endswith(b"\n") and not stdout.endswith(b"\n\n"), f"{where}: stdout line")
            response = load_json(stdout[:-1])
            assert_response(response, f"process:{case['id']}", case["exit_code"])
        require(case["submission_count"] <= 1 and case["acknowledge_count"] <= 1 and case["dial_count"] <= 1, f"{where}: count bound")
    by_id = {case["id"]: case for case in value["cases"]}
    require(by_id["argv_precedes_malformed_stdin"]["stdin_bytes_consumed"] == 0, "argv precedence")
    require(by_id["argv_environment_only_forbidden"]["stdin_bytes_consumed"] == 0 and by_id["argv_environment_cannot_fill_rpc"]["stdin_bytes_consumed"] == 0, "environment filled structural argv")
    require(by_id["argv_environment_cannot_fill_signer"]["stdin_bytes_consumed"] == by_id["argv_environment_cannot_fill_signer"]["request"]["bytes"], "environment signer precedence")
    require((by_id["argv_environment_cannot_override_explicit_read"]["exit_code"], by_id["argv_environment_cannot_override_explicit_read"]["dial_count"], by_id["argv_environment_cannot_override_explicit_read"]["sqlite_access_count"]) == (0, 1, 1), "environment overrode explicit read")
    require(by_id["request_decode_precedes_rpc_parse"]["dial_count"] == 0 and by_id["request_decode_precedes_rpc_parse"]["stdin_bytes_consumed"] == 1, "decode/RPC precedence")
    require(by_id["signer_accept_charlie_mutation"]["submission_count"] == 1 and by_id["signer_accept_charlie_mutation"]["sqlite_access_count"] == 1, "Charlie mutation counts")
    require(by_id["signer_accept_dave_mutation"]["submission_count"] == 0 and by_id["signer_accept_dave_mutation"]["sqlite_access_count"] == 0, "Dave parser/pre-send counts")
    for size in (1_048_575, 1_048_576, 1_048_577):
        case = by_id[("ingress_accept_" if size < 1_048_577 else "ingress_reject_") + str(size)]
        require(case["request"]["bytes"] == size and case["stdin_bytes_consumed"] == size, f"process: size {size}")


def validate_io_oracle(value: Any) -> None:
    exact_keys(value, ("fixture_schema_version", "hash_algorithm", "cases"), "io")
    require(value["fixture_schema_version"] == 1 and value["hash_algorithm"] == "sha256", "I/O header")
    expected_ids = (
        "request_read_failure", "read_response_body_failure", "read_response_newline_failure",
        "read_response_flush_failure", "mutation_response_body_failure",
        "mutation_response_newline_failure", "mutation_response_flush_failure",
        "mutation_acknowledgement_failure",
    )
    require(tuple(case["id"] for case in value["cases"]) == expected_ids, "I/O case inventory")
    data_by_id: dict[str, tuple[bytes, bytes, bytes]] = {}
    for index, case in enumerate(value["cases"]):
        where = f"io.cases[{index}]"
        exact_keys(case, ("id", "operation_kind", "fault", "modeled_stdout", "emitted_stdout", "stderr", "exit_code", "response_body_attempts", "newline_attempts", "flush_attempts", "acknowledge_attempts", "durable_journal_state"), where)
        _, modeled = validate_ref(case["modeled_stdout"], f"{where}.modeled_stdout")
        _, emitted = validate_ref(case["emitted_stdout"], f"{where}.emitted_stdout")
        _, stderr = validate_ref(case["stderr"], f"{where}.stderr")
        require(case["operation_kind"] in ("read", "mutation") and case["exit_code"] == 1, f"{where}: outcome")
        require(modeled.endswith(b"\n") and canonical(load_json(modeled[:-1])) + b"\n" == modeled, f"{where}: modeled response")
        for member in ("response_body_attempts", "newline_attempts", "flush_attempts", "acknowledge_attempts"):
            require(case[member] in (0, 1), f"{where}: {member}")
        if case["fault"] == "body_after_17_bytes":
            require(emitted == modeled[:-1][:17] and (case["response_body_attempts"], case["newline_attempts"], case["flush_attempts"], case["acknowledge_attempts"]) == (1, 0, 0, 0), f"{where}: body fault")
        elif case["fault"] == "newline_before_write":
            require(emitted == modeled[:-1] and (case["response_body_attempts"], case["newline_attempts"], case["flush_attempts"], case["acknowledge_attempts"]) == (1, 1, 0, 0), f"{where}: newline fault")
        elif case["fault"] == "flush":
            require(emitted == modeled and (case["response_body_attempts"], case["newline_attempts"], case["flush_attempts"], case["acknowledge_attempts"]) == (1, 1, 1, 0), f"{where}: flush fault")
        elif case["fault"] == "acknowledge_after_flush":
            require(emitted == modeled and (case["response_body_attempts"], case["newline_attempts"], case["flush_attempts"], case["acknowledge_attempts"]) == (1, 1, 1, 1), f"{where}: acknowledgement fault")
            require(stderr == b"cubikan-local: failed to acknowledge durable submission response\n" and case["durable_journal_state"] == "resolved_retained", f"{where}: acknowledgement contract")
        else:
            require(case["fault"] == "request_read" and emitted == b"" and all(case[member] == 0 for member in ("response_body_attempts", "newline_attempts", "flush_attempts", "acknowledge_attempts")), f"{where}: read fault")
        require(stderr.startswith(b"cubikan-local: failed to ") and stderr.endswith(b"\n"), f"{where}: stderr")
        data_by_id[case["id"]] = (modeled, emitted, stderr)
    require(data_by_id["mutation_acknowledgement_failure"][1] == data_by_id["mutation_acknowledgement_failure"][0], "ack failure emitted a second/different body")


def validate_source_mapping(value: Any) -> None:
    exact_keys(value, ("fixture_schema_version", "hash_algorithm", "public_error_variant_mappings", "private_proc_discovery_mappings"), "source_mapping")
    require(value["fixture_schema_version"] == 1 and value["hash_algorithm"] == "sha256", "source mapping header")
    entries = value["public_error_variant_mappings"]
    require(isinstance(entries, list) and len(entries) == 148, "source mapping cell count")
    require(len({entry["source"] for entry in entries}) == len(entries), "duplicate source mapping")
    mapping: dict[str, dict[str, Any]] = {}
    for index, entry in enumerate(entries):
        where = f"source_mapping.public[{index}]"
        if entry["source"] == "SubmissionErrorKind::AcknowledgementUnavailable":
            exact_keys(entry, ("source", "disposition", "outcome", "code", "exit_code", "stderr"), where)
            require(entry == {
                "source": "SubmissionErrorKind::AcknowledgementUnavailable",
                "disposition": "operational_failure_after_stdout_flush",
                "outcome": None,
                "code": None,
                "exit_code": 1,
                "stderr": "cubikan-local: failed to acknowledge durable submission response\n",
            }, f"{where}: acknowledgement mapping")
        else:
            exact_keys(entry, ("source", "disposition", "outcome", "code", "exit_code"), where)
            require(entry["disposition"] == "modeled_error" and entry["outcome"] == "error" and entry["code"] in ERROR_CODES and entry["exit_code"] in (1, 3, 4), f"{where}: mapped error")
        mapping[entry["source"]] = entry
    archive_direct = [source for source in mapping if source.startswith("ArchiveError::")]
    backend_direct = [source for source in mapping if source.startswith("BackendError::")]
    require(len(archive_direct) == 26 and len(backend_direct) == 17, "public source variant inventory")
    for source in archive_direct:
        direct = mapping[source]
        for wrapper in ("ProjectionError::Archive", "AttestationError::Archive"):
            nested = mapping.get(f"{wrapper}({source})")
            require(nested is not None and (nested["code"], nested["exit_code"]) == (direct["code"], direct["exit_code"]), f"{source}: archive recursion")
    for source in backend_direct:
        direct = mapping[source]
        for wrapper in ("ProjectionError::Backend", "AttestationError::Backend"):
            nested = mapping.get(f"{wrapper}({source})")
            require(nested is not None and (nested["code"], nested["exit_code"]) == (direct["code"], direct["exit_code"]), f"{source}: backend recursion")
    exact_submission = {
        "SubmissionErrorKind::UnsupportedPlatform": ("unsupported_platform", 4),
        "SubmissionErrorKind::InsecureProjectionPath": ("insecure_projection_path", 4),
        "SubmissionErrorKind::SubmissionLaneCorrupt": ("submission_lane_corrupt", 1),
        "SubmissionErrorKind::ArchiveRpcUnavailable": ("archive_rpc_unavailable", 4),
        "SubmissionErrorKind::ArchiveHistoryUnavailable": ("archive_history_unavailable", 4),
        "SubmissionErrorKind::DeploymentMismatch": ("deployment_mismatch", 4),
        "SubmissionErrorKind::RuntimeMismatch": ("runtime_mismatch", 4),
        "SubmissionErrorKind::DevSignerUnavailable": ("dev_signer_unavailable", 1),
        "SubmissionErrorKind::ArithmeticOverflow": ("runtime_mismatch", 4),
    }
    for source, expected in exact_submission.items():
        require((mapping[source]["code"], mapping[source]["exit_code"]) == expected, f"{source}: submission mapping")
    proc = value["private_proc_discovery_mappings"]
    require(len(proc) == 5 and [entry["source"] for entry in proc] == [
        "ProcDiscovery::NonLinux", "ProcDiscovery::UnreadableRoot",
        "ProcDiscovery::ZeroSuccessfulArchiveProcesses",
        "ProcDiscovery::MultipleSuccessfulArchiveProcesses",
        "ProcDiscovery::ExactlyOneSuccessfulArchiveProcess",
    ], "proc mapping inventory")
    require([entry.get("code") for entry in proc] == ["unsupported_platform", "unsupported_platform", "archive_rpc_unavailable", "archive_rpc_unavailable", None], "proc mapping codes")


schema_bytes = artifact_hash(SCHEMA, EXPECTED_SCHEMA_SHA256, "schema")
manifest_bytes = artifact_hash(MANIFEST, EXPECTED_MANIFEST_SHA256, "manifest")
inventory_bytes = artifact_hash(INVENTORY, EXPECTED_INVENTORY_SHA256, "inventory")
process_bytes = artifact_hash(PROCESS, EXPECTED_PROCESS_SHA256, "process")
io_bytes = artifact_hash(IO_ORACLE, EXPECTED_IO_SHA256, "I/O oracle")
source_mapping_bytes = artifact_hash(SOURCE_MAPPING, EXPECTED_SOURCE_MAPPING_SHA256, "source mapping")
schema = load_json(schema_bytes)
manifest = load_json(manifest_bytes)
inventory = load_json(inventory_bytes)
process = load_json(process_bytes)
io_oracle = load_json(io_bytes)
source_mapping = load_json(source_mapping_bytes)
assert_schema(schema)

exact_keys(manifest, ("fixture_schema_version", "hash_algorithm", "schema", "cases"), "manifest")
require(manifest["fixture_schema_version"] == 1 and manifest["hash_algorithm"] == "sha256", "manifest header")
schema_path, referenced_schema = validate_ref(manifest["schema"], "manifest.schema")
require(schema_path == SCHEMA and referenced_schema == schema_bytes, "manifest schema ref")

exact_keys(
    inventory,
    (
        "fixture_schema_version", "hash_algorithm", "authority", "schema", "manifest", "controls",
        "public_schema", "error_registry", "semantic_spine", "structural_corpus", "process_corpus",
        "io_corpus", "source_mapping_corpus", "private_context_contract", "errata", "safety_nonclaims",
    ),
    "inventory",
)
require(inventory["fixture_schema_version"] == 1 and inventory["hash_algorithm"] == "sha256" and inventory["authority"] == "independently_authored_synthetic_oracle", "inventory header")
for member, expected_path, expected_bytes in (
    ("schema", SCHEMA, schema_bytes),
    ("manifest", MANIFEST, manifest_bytes),
):
    path, data = validate_ref(inventory[member], f"inventory.{member}")
    require(path == expected_path and data == expected_bytes, f"inventory.{member}: wrong control")
exact_keys(inventory["controls"], ("process", "io", "source_mapping"), "inventory.controls")
for member, expected_path, expected_bytes in (
    ("process", PROCESS, process_bytes),
    ("io", IO_ORACLE, io_bytes),
    ("source_mapping", SOURCE_MAPPING, source_mapping_bytes),
):
    path, data = validate_ref(inventory["controls"][member], f"inventory.controls.{member}")
    require(path == expected_path and data == expected_bytes, f"inventory.controls.{member}: wrong control")

public = inventory["public_schema"]
exact_keys(public, ("root_union_count", "definition_count", "definition_names", "operation_count", "operations", "result_tags", "mutation_outcomes", "accepted_effect_tags", "command_schema_version", "caller_selectable_command_schema_version"), "inventory.public_schema")
require(public["root_union_count"] == 10 and public["definition_count"] == 96 and public["definition_names"] == list(DEFINITION_NAMES), "inventory schema counts")
require(public["operation_count"] == 15 and [entry["name"] for entry in public["operations"]] == list(OPERATIONS), "inventory operation registry")
for entry in public["operations"]:
    exact_keys(entry, ("name", "kind", "fields"), f"inventory.operation.{entry['name']}")
    require(entry["kind"] == ("mutation" if entry["name"] in MUTATIONS else "read"), f"inventory.operation.{entry['name']}: kind")
    properties, required = OPERATION_SCHEMA_FIELDS[entry["name"]]
    expected_fields = [member if member in required else f"{member}?" for member in properties]
    require(entry["fields"] == expected_fields, f"inventory.operation.{entry['name']}: fields")
require(public["result_tags"] == list(RESULT_TAGS) and public["mutation_outcomes"] == list(MUTATION_OUTCOMES), "inventory result/outcome registry")
require(public["accepted_effect_tags"] == list(EFFECT_BY_OPERATION.values()), "inventory effect registry")
require(public["command_schema_version"] == 1 and public["caller_selectable_command_schema_version"] is False, "command schema ownership")

require(len(inventory["error_registry"]) == 74, "error registry count")
for code, entry in zip(ERROR_CODES, inventory["error_registry"], strict=True):
    exact_keys(entry, ("code", "message", "field_policy", "revision_pair", "legal_envelopes"), f"inventory.error.{code}")
    require(entry == {
        "code": code,
        "message": MESSAGES[code],
        "field_policy": "required" if code in FIELD_CODES else "forbidden",
        "revision_pair": "required" if code == "revision_conflict" else "forbidden",
        "legal_envelopes": expected_legality(code),
    }, f"inventory.error.{code}: registry drift")

spine = inventory["semantic_spine"]
exact_keys(spine, ("positive_operation_cells", "error_code_cells", "total", "case_ids"), "inventory.semantic_spine")
require((spine["positive_operation_cells"], spine["error_code_cells"], spine["total"]) == (15, 79, 94), "semantic spine cardinality")
structural = inventory["structural_corpus"]
exact_keys(structural, ("total", "case_ids"), "inventory.structural_corpus")
require(structural["total"] == 187, "structural case count")
require(len(manifest["cases"]) == 281, "manifest case count")
require([case["id"] for case in manifest["cases"]] == spine["case_ids"] + structural["case_ids"], "manifest case order/inventory")
require(len(set(spine["case_ids"] + structural["case_ids"])) == 281, "duplicate manifest case ID")
require(inventory["process_corpus"] == {"total": 72, "case_ids": [case["id"] for case in process["cases"]]}, "process inventory drift")
require(inventory["io_corpus"] == {"total": 8, "case_ids": [case["id"] for case in io_oracle["cases"]]}, "I/O inventory drift")
require(inventory["source_mapping_corpus"] == {"public_error_variant_cells": 148, "private_proc_discovery_cells": 5}, "source mapping inventory drift")
require(inventory["errata"] == [{
    "criterion": "T-1113-E1 bad coordinate request",
    "status": "not_applicable",
    "reason": "the exact fifteen-operation request inventory has no coordinate member",
    "codec_coverage_case": "error_invalid_coordinate_codec",
    "unknown_input_case": "shape_unknown_root_member",
    "must_not_add_request_field": True,
}], "coordinate erratum drift")
require(inventory["private_context_contract"]["public_selector"] is False and inventory["private_context_contract"]["signature_authority"] == "oracle_only_not_production_byte_equality", "private fixture seam nonclaim")

REFERENCED.update((MANIFEST, INVENTORY, PROCESS, IO_ORACLE, SOURCE_MAPPING))
case_by_id: dict[str, dict[str, Any]] = {}
request_by_id: dict[str, bytes] = {}
stdout_by_id: dict[str, bytes] = {}
response_by_id: dict[str, dict[str, Any]] = {}
spine_codes: list[str] = []
spine_operations: list[str] = []
spine_result_tags: list[str] = []
spine_outcomes: list[str] = []

malformed_ids = {
    "error_malformed_json", "shape_empty_input", "shape_whitespace_input",
    "shape_trailing_json", "shape_invalid_utf8", "shape_nonfinite_nan",
    "shape_nonfinite_positive_infinity", "shape_nonfinite_negative_infinity",
}
for index, case in enumerate(manifest["cases"]):
    where = f"manifest.cases[{index}]"
    allowed_keys = ("id", "request", "stdout", "exit_code")
    if "context" in case:
        allowed_keys = ("id", "request", "context", "stdout", "exit_code")
    exact_keys(case, allowed_keys, where)
    case_id = case["id"]
    require(isinstance(case_id, str), f"{where}: ID")
    _, request_bytes = validate_ref(case["request"], f"{where}.request")
    _, stdout = validate_ref(case["stdout"], f"{where}.stdout")
    require(type(case["exit_code"]) is int and case["exit_code"] in (0, 1, 2, 3, 4), f"{where}: exit")
    require(stdout.endswith(b"\n") and not stdout.endswith(b"\n\n"), f"{case_id}: stdout line")
    response = load_json(stdout[:-1])
    require(canonical(response) + b"\n" == stdout, f"{case_id}: canonical stdout")
    code, result_tag = assert_response(response, case_id, case["exit_code"])
    parsed_request: dict[str, Any] | None = None
    if case_id.startswith("shape_duplicate_"):
        try:
            load_json(request_bytes)
        except DuplicateMember:
            pass
        else:
            fail(f"{case_id}: duplicate key lost")
    elif case_id in malformed_ids:
        try:
            load_json(request_bytes)
        except (UnicodeDecodeError, json.JSONDecodeError, NonFiniteNumber):
            pass
        else:
            fail(f"{case_id}: malformed bytes became JSON")
    else:
        candidate = load_json(request_bytes)
        if isinstance(candidate, dict):
            parsed_request = candidate
    loaded_context: dict[str, Any] = {}
    if "context" in case:
        loaded_context = assert_manifest_context(case["context"], parsed_request, response, case_id)
    if index < 94:
        spine_outcomes.append(response["outcome"])
        if code is not None:
            spine_codes.append(code)
        if result_tag is not None:
            spine_result_tags.append(result_tag)
        if code is None:
            if response.get("operation") in MUTATIONS:
                spine_operations.append(response["operation"])
            elif response.get("outcome") == "success" and parsed_request is not None:
                spine_operations.append(parsed_request["operation"]["type"])
    should_be_valid = (
        case_id.startswith("success_")
        or case_id.startswith("size_exact_")
        or case_id.startswith("shape_valid_")
        or case_id.startswith("scalar_valid_")
        or case_id.startswith("workflow_valid_")
        or case_id in {"error_invalid_coordinate_codec", "error_invalid_rpc_endpoint"}
        or case_id.endswith("_generic")
        or "read_miss" in case_id
        or case_id.startswith("error_submission_")
        or case_id in {"error_submission_lane_unresolved", "error_expired_not_included", "error_finalized_invariant_failed"}
        or case_id.startswith("error_finalized_dispatch_rejected_")
        or case_id.startswith("error_delivery_indeterminate_")
    )
    if should_be_valid:
        require(parsed_request is not None, f"{case_id}: expected valid request JSON")
        operation = assert_valid_request(parsed_request, f"{case_id}.request")
        if response.get("operation") is not None:
            require(response["operation"] == operation, f"{case_id}: persisted operation drift")
        if response.get("outcome") == "success":
            require(operation in READ_RESULT_BY_OPERATION, f"{case_id}: non-read returned read success")
            expected_tag, expected_direction = READ_RESULT_BY_OPERATION[operation]
            result = response["result"]
            require(result["type"] == expected_tag, f"{case_id}: operation/result tag drift")
            actual_direction = result.get("direction") if result["type"] == "association_page" else None
            require(actual_direction == expected_direction, f"{case_id}: operation/result direction drift")
    forbidden_surface_keys = {"seed", "private_key", "secret", "owner", "author", "source_body", "provider_body", "sql", "rpc", "database", "capability", "journal_path"}
    def inspect_surface(item: Any) -> None:
        if isinstance(item, dict):
            require(forbidden_surface_keys.isdisjoint(item), f"{case_id}: unsafe serialized member")
            for child in item.values():
                inspect_surface(child)
        elif isinstance(item, list):
            for child in item:
                inspect_surface(child)
    inspect_surface(response)
    case_by_id[case_id] = case
    request_by_id[case_id] = request_bytes
    stdout_by_id[case_id] = stdout
    response_by_id[case_id] = response

expected_code_counts = Counter(ERROR_CODES)
for overlap in ("runtime_mismatch", "intent_unit_not_found", "relationship_definition_not_found", "nonce_conflict", "transaction_invalid"):
    expected_code_counts[overlap] += 1
require(Counter(spine_codes) == expected_code_counts and len(spine_codes) == 79, "79-cell code/envelope spine drift")
require(Counter(spine_operations) == Counter({operation: 1 for operation in OPERATIONS}), "15-operation positive spine drift")
require(set(spine_result_tags) == set(RESULT_TAGS) and len(spine_result_tags) == 7, "read result spine drift")
require(set(spine_outcomes) == {"success", "error", *MUTATION_OUTCOMES}, "top-level outcome spine drift")

# Exact omission/null, duplicate/unknown, codec, cursor, and byte-boundary evidence.
omitted_create = load_json(request_by_id["success_create_intent_unit"])
require("id" not in omitted_create["operation"]["intent_unit"] and case_by_id["success_create_intent_unit"]["context"]["generated_uuid"] == "00112233-4455-4677-8899-aabbccddeeff", "omitted-ID oracle")
for case_id in structural["case_ids"]:
    if case_id.startswith("shape_null_optional_"):
        parsed = load_json(request_by_id[case_id])
        require(b":null" in request_by_id[case_id] and response_by_id[case_id]["error"]["code"] == "invalid_request", f"{case_id}: null optional")
for case_id, operation in (
    ("shape_valid_list_intent_units_after", "list_intent_units"),
    ("shape_valid_list_relationships_after", "list_relationships"),
    ("shape_valid_project_intent_units_v1_after", "project_intent_units_v1"),
    ("shape_valid_list_associations_by_unit_after", "list_associations_by_unit"),
    ("shape_valid_list_associations_by_reference_after", "list_associations_by_reference"),
):
    request = load_json(request_by_id[case_id])
    require(request["operation"]["type"] == operation and "after" in request["operation"], f"{case_id}: valid cursor selector")
    require(response_by_id[case_id]["error"]["code"] == "projection_busy", f"{case_id}: cursor did not pass decode into projection")
unknown_coordinate = load_json(request_by_id["shape_unknown_root_member"])
require("coordinate" in unknown_coordinate and response_by_id["shape_unknown_root_member"]["error"] == {"code": "invalid_request", "message": MESSAGES["invalid_request"], "field": "/coordinate"}, "coordinate input must remain unknown")
require(response_by_id["error_invalid_coordinate_codec"]["error"]["code"] == "invalid_coordinate", "coordinate codec legality")
for case_id, raw_member, escaped_pointer in (
    (
        "shape_unknown_member_tilde_pointer_escape",
        "tilde~member",
        "/operation/tilde~0member",
    ),
    (
        "shape_unknown_member_slash_pointer_escape",
        "slash/member",
        "/operation/slash~1member",
    ),
):
    request = load_json(request_by_id[case_id])
    require(request["operation"].get(raw_member) is True, f"{case_id}: raw metacharacter member")
    require(
        response_by_id[case_id]["error"]
        == {
            "code": "invalid_request",
            "message": MESSAGES["invalid_request"],
            "field": escaped_pointer,
        },
        f"{case_id}: exact RFC 6901 escaped pointer",
    )
escaping_case = "success_canonical_escaping_and_utf8"
escaping_stdout = stdout_by_id[escaping_case]
escaping_species = response_by_id[escaping_case]["result"]["intent_unit"]["species"]
require(escaping_species == 'tâche "locale" \\\n\t\u001f', "canonical escaping semantic value")
require(b"t\xc3\xa2che" in escaping_stdout and b"\\u00e2" not in escaping_stdout, "canonical stdout must preserve UTF-8 rather than ASCII-escape it")
for escaped in (b'\\"locale\\"', b"\\\\", b"\\n", b"\\t", b"\\u001f"):
    require(escaped in escaping_stdout, f"canonical stdout lacks JSON escape {escaped!r}")
require(all(byte >= 0x20 for byte in escaping_stdout[:-1]), "canonical stdout contains an unescaped control byte")
for size, case_id in ((1_048_575, "size_exact_1048575"), (1_048_576, "size_exact_1048576"), (1_048_577, "error_request_too_large")):
    require(len(request_by_id[case_id]) == size, f"{case_id}: byte boundary")
require(load_json(request_by_id["shape_valid_present_generated_id"])["operation"]["intent_unit"]["id"] == "00112233-4455-4677-8899-aabbccddeeff", "present ID shape")
require("source_species" not in load_json(request_by_id["shape_valid_omitted_definition_species"])["operation"] and "target_species" not in load_json(request_by_id["shape_valid_omitted_definition_species"])["operation"], "omitted definition constraints")
maximal_workflow = load_json(request_by_id["workflow_valid_collection_maxima"])["operation"]["workflow"]
require((len(maximal_workflow["phases"]), len(maximal_workflow["edges"]), len(maximal_workflow["completion_phases"])) == (32, 128, 32), "workflow collection maxima evidence")
minimal_workflow = load_json(request_by_id["workflow_valid_zero_edges_and_completions"])["operation"]["workflow"]
require((len(minimal_workflow["phases"]), minimal_workflow["edges"], minimal_workflow["completion_phases"]) == (1, [], []), "workflow zero collection evidence")

# All eight accepted mutation vectors are distinct, signature-valid, and bound to stdout.
accepted_hashes = []
for operation in MUTATIONS:
    case_id = f"success_{operation}"
    context = case_by_id[case_id]["context"]
    _, signer = load_context_ref(context["signer"], f"{case_id}.context.signer.recheck")
    request = load_json(request_by_id[case_id])
    expected_hash = verify_signer_vector(signer, request, context.get("generated_uuid"), f"{case_id}.signer.recheck")
    require(response_by_id[case_id]["coordinate"]["extrinsic_hash"] == expected_hash, f"{case_id}: signer/coordinate hash")
    accepted_hashes.append(expected_hash)
require(len(set(accepted_hashes)) == 8, "accepted signer vectors are not operation-distinct")

# E6: both optional relationship species constraints reach SCALE Option::None,
# an independently verified signed extrinsic, accepted output, and an omitted
# (never null) projected representation bound to the same coordinate hash.
option_none_case = "success_create_relationship_definition_option_none"
option_none_request = load_json(request_by_id[option_none_case])
option_none_operation = option_none_request["operation"]
require(
    "source_species" not in option_none_operation
    and "target_species" not in option_none_operation,
    "Option::None request must omit both species constraints",
)
_, option_none_signer = load_context_ref(
    case_by_id[option_none_case]["context"]["signer"],
    f"{option_none_case}.context.signer.e6",
)
option_none_hash = verify_signer_vector(
    option_none_signer,
    option_none_request,
    None,
    f"{option_none_case}.signer.e6",
)
option_none_call = bytes.fromhex(option_none_signer["call_scale"].removeprefix("0x"))
require(
    len(option_none_call) == 28
    and option_none_call[23:28] == b"\x00\x00\x00\x01\x01",
    "Option::None definition call layout",
)
require(
    option_none_call
    == bytes.fromhex(
        "3204010028646570656e64735f6f6e07000000000000000000000101"
    ),
    "Option::None definition call exact bytes",
)
require(
    response_by_id[option_none_case]["coordinate"]["extrinsic_hash"]
    == option_none_hash,
    "Option::None accepted coordinate is not signed vector hash",
)
omitted_result_case = "success_get_relationship_definition_omitted_species"
omitted_definition = response_by_id[omitted_result_case]["result"]["definition"]
require(
    tuple(omitted_definition)
    == (
        "key",
        "directed",
        "self_policy",
        "cycle_policy",
        "created_coordinate",
    )
    and omitted_definition["created_coordinate"]["extrinsic_hash"] == option_none_hash,
    "projected Option::None species must be omitted and signed-hash-bound",
)
complete = response_by_id["success_complete_intent_unit"]
require(complete["outcome"] == "finalized_accepted" and complete["projection"] == {"status": "lagging", "checkpoint": None}, "projection failure masked canonical acceptance")

validate_process_oracle(process, case_by_id)
validate_io_oracle(io_oracle)
validate_source_mapping(source_mapping)

# No unreferenced file, symlink, FIFO, or hidden alternate oracle may coexist.
actual: set[Path] = set()
for path in FIXTURES.rglob("*"):
    require(not path.is_symlink(), f"fixture symlink forbidden: {path}")
    if path.is_file():
        actual.add(path)
    else:
        require(path.is_dir(), f"non-file fixture node forbidden: {path}")
require(actual == REFERENCED, f"fixture closure drift: missing={sorted(str(p) for p in REFERENCED-actual)[:3]} extra={sorted(str(p) for p in actual-REFERENCED)[:3]}")

print(
    "verified cubikan-local protocol v2: "
    f"schema={EXPECTED_SCHEMA_SHA256} manifest={EXPECTED_MANIFEST_SHA256} "
    f"inventory={EXPECTED_INVENTORY_SHA256} cases=281 semantic=94 structural=187 "
    "process_cases=72 io_cases=8 source_cells=153 files=703"
)
LOCAL_PY
