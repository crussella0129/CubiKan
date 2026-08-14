#!/usr/bin/env python3
"""Fail-closed verification of CubiKan's generated weights and raw evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import re
import sys
import tomllib
from typing import NoReturn

EXPECTED_FUNCTIONS = (
    "create_unit",
    "transition_unit",
    "complete_unit",
    "replace_authorized_submitters",
    "create_relationship_definition",
    "create_relationship",
    "delete_relationship",
    "record_association",
    "revoke_association",
)
SDK_REVISION = "8ae9775dc43c0d8cdd0f6d87700596e14278b1e1"
BENCHMARK_SOURCE_PATH = "chain/pallets/cubikan/src/benchmarking.rs"
BENCHMARK_SOURCE_SHA256 = "59f79352896709ff8deb6f79d6f01308f402edb3f8908ef88d1a4ba5b8692ce4"
WEIGHTS_PATH = "chain/runtime/src/weights/pallet_cubikan.rs"
WEIGHTS_SHA256 = "5300fec791e7d352be42abdfbf8a7168beafa736bc7f665883ab03e1eac3e1f8"
EVIDENCE_PATH = "chain/artifacts/benchmarks/cubikan-pallet-v1.json"
EVIDENCE_SHA256 = "7951a5292d45ebca80080118a598a3b3a2de2187ea4649c718be386580e59c3d"
BENCHMARK_WASM_PATH = (
    "chain/artifacts/benchmarks/cubikan-runtime-v1.runtime-benchmarks.compact.compressed.wasm"
)
BENCHMARK_WASM_SHA256 = "27461a4a473e6f87057b215c8320a52b8a766488c9f79af2079259a6ae0d2370"
GENERATOR_ASSET = "assets.frame-omni-bencher"
GENERATOR_VERSION = "frame-omni-bencher 0.22.0"
GENERATOR_SHA256 = "501f92ba8f1dd7eabfe84aa3990f517fd448c3d5e0de6f408b29656933e39576"
CLI_VERSION = "58.0.0"

EXPECTED_DATABASE = {
    "create_unit": (5, 2),
    "transition_unit": (5, 2),
    "complete_unit": (5, 2),
    "replace_authorized_submitters": (0, 1),
    "create_relationship_definition": (5, 2),
    "create_relationship": (8, 2),
    "delete_relationship": (8, 2),
    "record_association": (6, 2),
    "revoke_association": (6, 2),
}

EXPECTED_STORAGE = {
    "create_unit": (
        ("AuthorizedSubmitters", 1, 0),
        ("IntentUnits", 1, 1),
        ("GlobalSequence", 1, 1),
        ("DeploymentAnchor", 1, 0),
        ("EventSchemaVersion", 1, 0),
    ),
    "transition_unit": (
        ("AuthorizedSubmitters", 1, 0),
        ("IntentUnits", 1, 1),
        ("GlobalSequence", 1, 1),
        ("DeploymentAnchor", 1, 0),
        ("EventSchemaVersion", 1, 0),
    ),
    "complete_unit": (
        ("AuthorizedSubmitters", 1, 0),
        ("IntentUnits", 1, 1),
        ("GlobalSequence", 1, 1),
        ("DeploymentAnchor", 1, 0),
        ("EventSchemaVersion", 1, 0),
    ),
    "replace_authorized_submitters": (("AuthorizedSubmitters", 0, 1),),
    "create_relationship_definition": (
        ("AuthorizedSubmitters", 1, 0),
        ("RelationshipDefinitions", 1, 1),
        ("GlobalSequence", 1, 1),
        ("DeploymentAnchor", 1, 0),
        ("EventSchemaVersion", 1, 0),
    ),
    "create_relationship": (
        ("AuthorizedSubmitters", 1, 0),
        ("RelationshipDefinitions", 1, 0),
        ("IntentUnits", 2, 0),
        ("RelationshipEdges", 1, 1),
        ("GlobalSequence", 1, 1),
        ("DeploymentAnchor", 1, 0),
        ("EventSchemaVersion", 1, 0),
    ),
    "delete_relationship": (
        ("AuthorizedSubmitters", 1, 0),
        ("RelationshipDefinitions", 1, 0),
        ("IntentUnits", 2, 0),
        ("RelationshipEdges", 1, 1),
        ("GlobalSequence", 1, 1),
        ("DeploymentAnchor", 1, 0),
        ("EventSchemaVersion", 1, 0),
    ),
    "record_association": (
        ("AuthorizedSubmitters", 1, 0),
        ("IntentUnits", 1, 0),
        ("ActiveAssociations", 1, 1),
        ("GlobalSequence", 1, 1),
        ("DeploymentAnchor", 1, 0),
        ("EventSchemaVersion", 1, 0),
    ),
    "revoke_association": (
        ("AuthorizedSubmitters", 1, 0),
        ("IntentUnits", 1, 0),
        ("ActiveAssociations", 1, 1),
        ("GlobalSequence", 1, 1),
        ("DeploymentAnchor", 1, 0),
        ("EventSchemaVersion", 1, 0),
    ),
}


def fail(message: str) -> NoReturn:
    raise ValueError(message)


def regular_file(root: pathlib.Path, relative: str) -> pathlib.Path:
    path = root / relative
    if path.is_symlink() or not path.is_file():
        fail(f"missing or symbolic regular file: {relative}")
    return path


def sha256(path: pathlib.Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def exact_keys(value: object, expected: set[str], label: str) -> dict[str, object]:
    if not isinstance(value, dict):
        fail(f"{label} is not an object")
    if set(value) != expected:
        fail(f"{label} keys differ: expected {sorted(expected)}, got {sorted(value)}")
    return value


def require_pin(table: dict[str, object], key: str, expected: object) -> None:
    if table.get(key) != expected:
        fail(f"runtime_weights.{key} differs from the locked value")


def verify_pins(
    root: pathlib.Path,
    benchmark_source_path: pathlib.Path,
    weights_path: pathlib.Path,
    evidence_path: pathlib.Path,
    benchmark_wasm_path: pathlib.Path,
) -> None:
    pins = tomllib.loads(regular_file(root, "chain/pins.toml").read_text(encoding="utf-8"))
    table = exact_keys(
        pins.get("runtime_weights"),
        {
            "generator",
            "generator_asset",
            "generator_version",
            "generator_sha256",
            "cli_version",
            "pallet",
            "steps",
            "repeat",
            "wasm_execution",
            "output_analysis",
            "output_pov_analysis",
            "default_pov_mode",
            "benchmark_source_path",
            "benchmark_source_sha256",
            "weights_path",
            "weights_sha256",
            "benchmark_wasm_path",
            "benchmark_wasm_sha256",
            "evidence_path",
            "evidence_sha256",
        },
        "runtime_weights",
    )
    expected = {
        "generator": "frame-omni-bencher",
        "generator_asset": GENERATOR_ASSET,
        "generator_version": GENERATOR_VERSION,
        "generator_sha256": GENERATOR_SHA256,
        "cli_version": CLI_VERSION,
        "pallet": "pallet_cubikan",
        "steps": 50,
        "repeat": 20,
        "wasm_execution": "compiled",
        "output_analysis": "max",
        "output_pov_analysis": "max",
        "default_pov_mode": "measured",
        "benchmark_source_path": BENCHMARK_SOURCE_PATH,
        "benchmark_source_sha256": BENCHMARK_SOURCE_SHA256,
        "weights_path": WEIGHTS_PATH,
        "weights_sha256": WEIGHTS_SHA256,
        "benchmark_wasm_path": BENCHMARK_WASM_PATH,
        "benchmark_wasm_sha256": BENCHMARK_WASM_SHA256,
        "evidence_path": EVIDENCE_PATH,
        "evidence_sha256": EVIDENCE_SHA256,
    }
    for key, value in expected.items():
        require_pin(table, key, value)
    if sha256(benchmark_source_path) != BENCHMARK_SOURCE_SHA256:
        fail("benchmark source digest differs from runtime_weights")
    if sha256(weights_path) != WEIGHTS_SHA256:
        fail("generated runtime weight digest differs from runtime_weights")
    if sha256(evidence_path) != EVIDENCE_SHA256:
        fail("raw benchmark evidence digest differs from runtime_weights")
    if sha256(benchmark_wasm_path) != BENCHMARK_WASM_SHA256:
        fail("retained benchmark Wasm digest differs from runtime_weights")

    sdk = pins.get("polkadot_sdk")
    if not isinstance(sdk, dict) or sdk.get("revision") != SDK_REVISION:
        fail("runtime weights are not bound to the pinned Polkadot SDK revision")

    generator = pins.get("assets", {}).get("frame-omni-bencher")
    if not isinstance(generator, dict):
        fail("pins.toml lacks assets.frame-omni-bencher")
    require_pin(table, "generator_version", generator.get("version"))
    require_pin(table, "generator_sha256", generator.get("sha256"))


def command_argv(weights: str) -> list[str]:
    match = re.search(
        r"(?ms)^// Executed Command:\n(?P<body>(?:// [^\n]+\n)+)\n#!\[",
        weights,
    )
    if match is None:
        fail("standard generated Executed Command block is absent")
    return [line.removeprefix("// ") for line in match.group("body").splitlines()]


def verify_header_and_command(weights: str, root: pathlib.Path) -> None:
    header = re.match(
        r"(?s)^\n?//! Autogenerated weights for `pallet_cubikan`\n//!\n"
        r"//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 58\.0\.0\n"
        r"//! DATE: 2026-08-14, STEPS: `50`, REPEAT: `20`, LOW RANGE: `\[\]`, HIGH RANGE: `\[\]`\n"
        r"//! WORST CASE MAP SIZE: `1000000`\n"
        r"//! HOSTNAME: `cubikan-t1106`, CPU: `AMD Ryzen 9 5900X 12-Core Processor`\n"
        r"//! WASM-EXECUTION: `Compiled`, CHAIN: `None`, DB CACHE: 1024\n\n",
        weights,
    )
    if header is None:
        fail("standard generated header or locked generation facts differ")
    if any(marker in weights.lower() for marker in ("provisional", "fake-marker", "placeholder")):
        fail("provisional or fake generated-weight marker is present")

    argv = command_argv(weights)
    expected_prefix = [
        "frame-omni-bencher",
        "v1",
        "benchmark",
        "pallet",
        "--runtime",
    ]
    expected_middle = [
        "--genesis-builder",
        "runtime",
        "--genesis-builder-preset",
        "development",
        "--pallets",
        "pallet_cubikan",
        "--extrinsic",
        "*",
        "--steps",
        "50",
        "--repeat",
        "20",
        "--min-duration",
        "0",
        "--wasm-execution",
        "compiled",
        "--output-analysis",
        "max",
        "--output-pov-analysis",
        "max",
        "--default-pov-mode",
        "measured",
        "--hostname-override",
        "cubikan-t1106",
        "--json-file",
    ]
    expected_suffix = [
        "--output",
    ]
    if (
        len(argv) != len(expected_prefix) + 1 + len(expected_middle) + 1 + len(expected_suffix) + 1
        or argv[: len(expected_prefix)] != expected_prefix
        or argv[len(expected_prefix) + 1 : len(expected_prefix) + 1 + len(expected_middle)]
        != expected_middle
        or argv[-(len(expected_suffix) + 1) : -1] != expected_suffix
    ):
        fail("generated Executed Command argv differs from the locked benchmark invocation")

    runtime_arg = pathlib.Path(argv[len(expected_prefix)])
    json_arg = pathlib.Path(argv[len(expected_prefix) + 1 + len(expected_middle)])
    output_arg = pathlib.Path(argv[-1])
    expected_paths = (
        (
            runtime_arg,
            pathlib.PurePosixPath(
                BENCHMARK_WASM_PATH
            ),
        ),
        (json_arg, pathlib.PurePosixPath(EVIDENCE_PATH)),
        (output_arg, pathlib.PurePosixPath("chain/runtime/src/weights")),
    )
    for actual, expected_path in expected_paths:
        normalized = pathlib.PurePosixPath(actual.as_posix().rstrip("/"))
        if actual.is_absolute():
            matches = actual.resolve(strict=False) == (root / expected_path).resolve(strict=False)
        else:
            matches = normalized == expected_path
        if not matches:
            fail(f"generated benchmark argv path is not the project artifact: {actual}")


def function_sections(weights: str) -> dict[str, str]:
    matches = list(re.finditer(r"(?m)^\s*fn\s+([a-z0-9_]+)\s*\(", weights))
    names = [match.group(1) for match in matches]
    if names != list(EXPECTED_FUNCTIONS):
        fail(f"generated function set/order differs: {names}")
    sections: dict[str, str] = {}
    previous_function_end = weights.find("impl<")
    for index, match in enumerate(matches):
        storage_start = weights.find("\t/// Storage:", previous_function_end, match.start())
        start = storage_start if storage_start >= 0 else match.start()
        function_end = weights.find("\n\t}", match.end())
        if function_end < 0:
            fail(f"generated function {match.group(1)} has no closing brace")
        end = function_end + len("\n\t}")
        sections[match.group(1)] = weights[start:end]
        previous_function_end = end
    return sections


def weight_at_component(body: str, component: int) -> tuple[int, int]:
    total_ref_time = 0
    total_proof = 0
    for ref_time_text, proof_text, component_name in re.findall(
        r"Weight::from_parts\(\s*([0-9_]+)\s*,\s*([0-9_]+)\s*\)"
        r"(?:\.saturating_mul\((a)\.into\(\)\))?",
        body,
    ):
        multiplier = component if component_name == "a" else 1
        total_ref_time += int(ref_time_text.replace("_", "")) * multiplier
        total_proof += int(proof_text.replace("_", "")) * multiplier
    return total_ref_time, total_proof


def verify_weight_source(weights: str, root: pathlib.Path) -> dict[str, int]:
    verify_header_and_command(weights, root)
    sections = function_sections(weights)
    if "/// The range of component `a` is `[0, 16]`." not in sections["replace_authorized_submitters"]:
        fail("replace_authorized_submitters component a range is not exactly [0, 16]")
    if not re.search(r"fn replace_authorized_submitters\(a: u32,\s*\)", weights):
        fail("replace_authorized_submitters lacks generated component a")

    storage_pattern = re.compile(
        r"/// Storage: `Cubikan::([A-Za-z0-9]+)` \(r:(\d+) w:(\d+)\)"
    )
    generated_proofs: dict[str, int] = {}
    for name, body in sections.items():
        if "Weight::zero" in body:
            fail(f"zero placeholder in generated function {name}")
        parts = [
            (int(ref_time.replace("_", "")), int(proof.replace("_", "")))
            for ref_time, proof in re.findall(
                r"Weight::from_parts\(\s*([0-9_]+)\s*,\s*([0-9_]+)\s*\)", body
            )
        ]
        if not parts or not any(ref_time > 0 for ref_time, _ in parts):
            fail(f"generated function {name} has no finite nonzero reference time")
        minimum = re.search(r"Minimum execution time: ([0-9_]+) picoseconds", body)
        measured = re.search(r"Measured:\s+`([0-9_]+)`", body)
        estimated = re.search(r"Estimated: `([0-9_]+)`", body)
        if minimum is None or int(minimum.group(1).replace("_", "")) <= 0:
            fail(f"generated function {name} lacks a finite nonzero minimum time")
        if measured is None or estimated is None:
            fail(f"generated function {name} lacks proof-size analysis annotations")
        annotated_proof = int(estimated.group(1).replace("_", ""))
        generated_ref_time, generated_proof = weight_at_component(
            body, 16 if name == "replace_authorized_submitters" else 0
        )
        minimum_ref_time = int(minimum.group(1).replace("_", ""))
        if generated_ref_time < minimum_ref_time:
            fail(
                f"emitted ref time undercounts parsed minimum for {name}: "
                f"{generated_ref_time} < {minimum_ref_time}"
            )
        if generated_proof != annotated_proof:
            fail(
                f"emitted proof weight differs from Estimated annotation for {name}: "
                f"{generated_proof} != {annotated_proof}"
            )
        generated_proofs[name] = generated_proof
        observed_storage = tuple(
            (storage, int(reads), int(writes))
            for storage, reads, writes in storage_pattern.findall(body)
        )
        if observed_storage != EXPECTED_STORAGE[name]:
            fail(f"generated storage annotations differ for {name}: {observed_storage}")
        proof_annotations = re.findall(
            r"/// Proof: `Cubikan::([A-Za-z0-9]+)` \([^\n]*mode: `([^`]+)`\)", body
        )
        expected_proofs = [(storage, "Measured") for storage, _, _ in EXPECTED_STORAGE[name]]
        if proof_annotations != expected_proofs:
            fail(f"generated measured-proof annotations differ for {name}: {proof_annotations}")
        reads, writes = EXPECTED_DATABASE[name]
        actual_reads = sum(int(value) for value in re.findall(r"\.reads\((\d+)\)", body))
        actual_writes = sum(int(value) for value in re.findall(r"\.writes\((\d+)\)", body))
        if (actual_reads, actual_writes) != (reads, writes):
            fail(f"generated database accounting differs for {name}")
        if int(measured.group(1).replace("_", "")) > 0 and not any(
            proof > 0 for _, proof in parts
        ):
            fail(f"generated function {name} drops its nonzero proof-size weight")
    return generated_proofs


def result_object(result: object, benchmark: str, group: str) -> dict[str, object]:
    return exact_keys(
        result,
        {
            "components",
            "extrinsic_time",
            "storage_root_time",
            "reads",
            "repeat_reads",
            "writes",
            "repeat_writes",
            "proof_size",
        },
        f"{benchmark}.{group} result",
    )


def raw_tuple(result: dict[str, object]) -> tuple[int, int, int, int, int]:
    return (
        result["reads"],
        result["repeat_reads"],
        result["writes"],
        result["repeat_writes"],
        result["proof_size"],
    )


def verify_evidence(evidence_path: pathlib.Path) -> dict[str, int]:
    evidence = json.loads(evidence_path.read_text(encoding="utf-8"))
    if not isinstance(evidence, list) or len(evidence) != 9:
        fail("raw benchmark evidence is not an exact nine-entry array")
    names: list[str] = []
    raw_proof_maxima: dict[str, int] = {}
    for entry_value in evidence:
        entry = exact_keys(
            entry_value,
            {"pallet", "instance", "benchmark", "time_results", "db_results"},
            "benchmark entry",
        )
        benchmark = entry.get("benchmark")
        if not isinstance(benchmark, str):
            fail("benchmark name is not a string")
        names.append(benchmark)
        if entry.get("pallet") != "pallet_cubikan" or entry.get("instance") != "Cubikan":
            fail(f"raw evidence identity differs for {benchmark}")
        time_values = entry.get("time_results")
        database_values = entry.get("db_results")
        if not isinstance(time_values, list) or not isinstance(database_values, list):
            fail(f"raw evidence results are not arrays for {benchmark}")
        expected_time_count = 1000 if benchmark == "replace_authorized_submitters" else 20
        expected_database_count = 50 if benchmark == "replace_authorized_submitters" else 1
        if len(time_values) != expected_time_count or len(database_values) != expected_database_count:
            fail(f"raw evidence sample counts differ for {benchmark}")
        time_results = [result_object(value, benchmark, "time") for value in time_values]
        database_results = [result_object(value, benchmark, "database") for value in database_values]
        for result in time_results + database_results:
            for field in (
                "extrinsic_time",
                "storage_root_time",
                "reads",
                "repeat_reads",
                "writes",
                "repeat_writes",
                "proof_size",
            ):
                value = result.get(field)
                if type(value) is not int or value < 0:
                    fail(f"raw evidence {benchmark}.{field} is not a finite nonnegative integer")
        if max(result["extrinsic_time"] for result in time_results) <= 0:
            fail(f"raw evidence has no nonzero time maximum for {benchmark}")
        if max(result["storage_root_time"] for result in time_results) <= 0:
            fail(f"raw evidence has no nonzero storage-root maximum for {benchmark}")
        reads, writes = EXPECTED_DATABASE[benchmark]
        proof_maximum = max(result["proof_size"] for result in time_results + database_results)
        raw_proof_maxima[benchmark] = proof_maximum
        if benchmark != "replace_authorized_submitters" and proof_maximum <= 0:
            fail(f"raw evidence has no nonzero proof maximum for {benchmark}")
        if benchmark == "replace_authorized_submitters":
            expected_time_counts = {value: 60 for value in range(17)}
            expected_time_counts[0] = 80
            expected_time_counts[16] = 20
            expected_database_counts = {value: 3 for value in range(17)}
            expected_database_counts[0] = 4
            expected_database_counts[16] = 1
            for group, results, expected_counts, expected_tuple in (
                ("time", time_results, expected_time_counts, (0, 0, 0, 0, 0)),
                ("database", database_results, expected_database_counts, (0, 0, 1, 0, 0)),
            ):
                counts = {value: 0 for value in range(17)}
                for result in results:
                    components = result["components"]
                    if (
                        not isinstance(components, list)
                        or len(components) != 1
                        or not isinstance(components[0], list)
                        or len(components[0]) != 2
                        or components[0][0] != "a"
                        or type(components[0][1]) is not int
                        or components[0][1] not in counts
                    ):
                        fail(f"raw {group} component tuple differs for {benchmark}")
                    counts[components[0][1]] += 1
                    if raw_tuple(result) != expected_tuple:
                        fail(f"raw {group} accounting tuple differs for {benchmark}")
                if counts != expected_counts:
                    fail(f"raw {group} component-a sample distribution differs for {benchmark}")
        else:
            expected_time_tuple = (0, 0, 0, 0, proof_maximum)
            expected_database_tuple = (reads, 0, writes, 0, proof_maximum)
            if any(
                result["components"] != [] or raw_tuple(result) != expected_time_tuple
                for result in time_results
            ):
                fail(f"raw time tuple differs for {benchmark}")
            if any(
                result["components"] != [] or raw_tuple(result) != expected_database_tuple
                for result in database_results
            ):
                fail(f"raw database tuple differs for {benchmark}")
    if names != list(EXPECTED_FUNCTIONS):
        fail(f"raw benchmark name set/order differs: {names}")
    return raw_proof_maxima


def verify(root: pathlib.Path) -> None:
    runtime = regular_file(root, "chain/runtime/src/lib.rs").read_text(encoding="utf-8")
    configs = regular_file(root, "chain/runtime/src/configs.rs").read_text(encoding="utf-8")
    benchmark_source_path = regular_file(root, BENCHMARK_SOURCE_PATH)
    weights_path = regular_file(root, WEIGHTS_PATH)
    evidence_path = regular_file(root, EVIDENCE_PATH)
    benchmark_wasm_path = regular_file(root, BENCHMARK_WASM_PATH)
    weights = weights_path.read_text(encoding="utf-8")

    selected = re.search(
        r"type\s+WeightInfo\s*=\s*(?:crate::)?weights::pallet_cubikan::"
        r"WeightInfo\s*<\s*Runtime\s*>\s*;",
        configs,
    )
    if selected is None:
        fail("runtime does not select its generated CubiKan WeightInfo")
    cubikan_config = re.search(
        r"impl\s+pallet_cubikan::Config\s+for\s+Runtime\s*\{(?P<body>.*?)\n\}",
        configs,
        re.DOTALL,
    )
    if cubikan_config is None:
        fail("runtime lacks pallet_cubikan::Config")
    if re.search(
        r"type\s+WeightInfo\s*=.*(?:SubstrateWeight|\(\))",
        cubikan_config.group("body"),
    ):
        fail("runtime selects a pallet fallback WeightInfo")
    if "mod weights" not in runtime:
        fail("runtime does not compile its generated weights module")

    generated_proofs = verify_weight_source(weights, root)
    raw_proof_maxima = verify_evidence(evidence_path)
    for name in EXPECTED_FUNCTIONS:
        if generated_proofs[name] < raw_proof_maxima[name]:
            fail(
                f"generated proof weight undercounts raw measured maximum for {name}: "
                f"{generated_proofs[name]} < {raw_proof_maxima[name]}"
            )
    verify_pins(
        root,
        benchmark_source_path,
        weights_path,
        evidence_path,
        benchmark_wasm_path,
    )

    pallet_fallback = regular_file(
        root, "chain/pallets/cubikan/src/weights.rs"
    ).read_text(encoding="utf-8")
    if "impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T>" not in pallet_fallback:
        fail("pallet fallback weight contract is missing")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=("locked", "test-static"))
    parser.add_argument("root")
    args = parser.parse_args()
    try:
        root = pathlib.Path(args.root).resolve(strict=True)
        verify(root)
    except (
        OSError,
        UnicodeError,
        ValueError,
        json.JSONDecodeError,
        tomllib.TOMLDecodeError,
    ) as error:
        print(f"verify-weights: {error}", file=sys.stderr)
        return 1
    print("verify-weights: generated runtime weights and raw evidence verified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
