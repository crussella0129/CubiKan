//! Generated-weight completeness tests for T-1106.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use cubikan_runtime::Runtime;
use pallet_cubikan::{
    types::{
        MAX_ACTIVE_ASSOCIATIONS, MAX_AUTHORIZED_SUBMITTERS, MAX_LIFECYCLE_RECORDS,
        MAX_RELATIONSHIP_EDGES,
    },
    weights::WeightInfo,
};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("runtime crate is nested directly under chain/")
        .parent()
        .expect("chain workspace is nested under the repository root")
        .to_path_buf()
}

fn run_static_verifier(root: &Path) -> Output {
    Command::new("bash")
        .arg(root.join("chain/tools/verify-weights.sh"))
        .arg("--test-static")
        .arg(root)
        .output()
        .expect("run static generated-weight verifier")
}

fn copy_weight_contract(root: &Path, target: &Path) {
    for relative in [
        "chain/runtime/src/lib.rs",
        "chain/runtime/src/configs.rs",
        "chain/runtime/src/weights/pallet_cubikan.rs",
        "chain/pallets/cubikan/src/benchmarking.rs",
        "chain/pallets/cubikan/src/weights.rs",
        "chain/artifacts/benchmarks/cubikan-pallet-v1.json",
        "chain/artifacts/benchmarks/cubikan-runtime-v1.runtime-benchmarks.compact.compressed.wasm",
        "chain/pins.toml",
        "chain/tools/verify-weights.py",
        "chain/tools/verify-weights.sh",
    ] {
        let source = root.join(relative);
        let destination = target.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has a parent"))
            .expect("create fixture parent");
        fs::copy(&source, &destination).unwrap_or_else(|error| {
            panic!("copy {} into mutation fixture: {error}", source.display())
        });
    }
}

fn sha256_file(path: &Path) -> String {
    lowercase_hex(&sp_io::hashing::sha2_256(&fs::read(path).unwrap_or_else(
        |error| panic!("read {} for SHA-256: {error}", path.display()),
    )))
}

fn append_runtime_weight_pins(target: &Path) {
    let pins_path = target.join("chain/pins.toml");
    let mut pins = fs::read_to_string(&pins_path).expect("read copied pins");
    if pins.contains("[runtime_weights]") {
        return;
    }
    let weights_digest = sha256_file(&target.join("chain/runtime/src/weights/pallet_cubikan.rs"));
    let evidence_digest =
        sha256_file(&target.join("chain/artifacts/benchmarks/cubikan-pallet-v1.json"));
    pins.push_str(&format!(
        "\n[runtime_weights]\n\
         generator = \"frame-omni-bencher\"\n\
         generator_asset = \"assets.frame-omni-bencher\"\n\
         generator_version = \"frame-omni-bencher 0.22.0\"\n\
         generator_sha256 = \"501f92ba8f1dd7eabfe84aa3990f517fd448c3d5e0de6f408b29656933e39576\"\n\
         cli_version = \"58.0.0\"\n\
         pallet = \"pallet_cubikan\"\n\
         steps = 50\n\
         repeat = 20\n\
         wasm_execution = \"compiled\"\n\
         output_analysis = \"max\"\n\
         output_pov_analysis = \"max\"\n\
         default_pov_mode = \"measured\"\n\
         benchmark_source_path = \"chain/pallets/cubikan/src/benchmarking.rs\"\n\
         benchmark_source_sha256 = \"59f79352896709ff8deb6f79d6f01308f402edb3f8908ef88d1a4ba5b8692ce4\"\n\
         weights_path = \"chain/runtime/src/weights/pallet_cubikan.rs\"\n\
         weights_sha256 = \"{weights_digest}\"\n\
         benchmark_wasm_path = \"chain/artifacts/benchmarks/cubikan-runtime-v1.runtime-benchmarks.compact.compressed.wasm\"\n\
         benchmark_wasm_sha256 = \"27461a4a473e6f87057b215c8320a52b8a766488c9f79af2079259a6ae0d2370\"\n\
         evidence_path = \"chain/artifacts/benchmarks/cubikan-pallet-v1.json\"\n\
         evidence_sha256 = \"{evidence_digest}\"\n"
    ));
    fs::write(pins_path, pins).expect("append copied runtime-weight pins");
}

fn replace_pin_digest(pins: &mut String, key: &str, digest: &str) {
    let marker = format!("{key} = \"");
    let start = pins
        .find(&marker)
        .map(|index| index + marker.len())
        .unwrap_or_else(|| panic!("copied pins omit {key}"));
    let end = pins[start..]
        .find('"')
        .map(|index| start + index)
        .expect("copied digest pin is quoted");
    pins.replace_range(start..end, digest);
}

fn lowercase_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(DIGITS[(byte >> 4) as usize] as char);
        encoded.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    encoded
}

#[test]
fn test_generated_weights_cover_every_declared_maximum() {
    type Weights = cubikan_runtime::weights::pallet_cubikan::WeightInfo<Runtime>;

    let weights = [
        Weights::create_unit(),
        Weights::transition_unit(),
        Weights::complete_unit(),
        Weights::create_relationship_definition(),
        Weights::create_relationship(),
        Weights::delete_relationship(),
        Weights::record_association(),
        Weights::revoke_association(),
    ];
    for weight in weights {
        assert!(weight.ref_time() > 0, "dispatch has zero ref-time weight");
        assert!(
            weight.proof_size() > 0,
            "dispatch has zero proof-size weight"
        );
    }
    let bounded_allowlist =
        Weights::replace_authorized_submitters(MAX_AUTHORIZED_SUBMITTERS as u32);
    assert!(
        bounded_allowlist.ref_time() > 0,
        "allowlist dispatch has zero ref-time weight"
    );
    assert_eq!(
        bounded_allowlist.proof_size(),
        0,
        "the storage-value-only allowlist benchmark has no proof-size component"
    );

    assert_eq!(MAX_AUTHORIZED_SUBMITTERS, 16);
    assert_eq!(MAX_RELATIONSHIP_EDGES, 128);
    assert_eq!(MAX_ACTIVE_ASSOCIATIONS, 128);
    assert_eq!(MAX_LIFECYCLE_RECORDS, 256);

    let source = fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("runtime crate is nested under chain/")
            .join("runtime/src/weights/pallet_cubikan.rs"),
    )
    .expect("read generated weight source");
    for marker in [
        "AUTO-GENERATED",
        "frame-omni-bencher",
        "STEPS: `50`, REPEAT: `20`",
        "// --pallets\n// pallet_cubikan",
        "// --wasm-execution\n// compiled",
        "// --output-analysis\n// max",
        "// --output-pov-analysis\n// max",
        "// --default-pov-mode\n// measured",
    ] {
        assert!(source.contains(marker), "generated weights omit {marker}");
    }
}

#[test]
fn test_generated_weight_evidence_is_locked_and_mutations_fail() {
    let root = repository_root();
    let verified = run_static_verifier(&root);
    assert!(
        verified.status.success(),
        "static generated-weight verification failed:\n{}",
        String::from_utf8_lossy(&verified.stderr)
    );

    let fixture = std::env::temp_dir().join(format!(
        "cubikan-t1106-weight-mutation-{}",
        std::process::id()
    ));
    if fixture.exists() {
        fs::remove_dir_all(&fixture).expect("remove stale generated-weight mutation fixture");
    }
    copy_weight_contract(&root, &fixture);
    append_runtime_weight_pins(&fixture);

    let weights_path = fixture.join("chain/runtime/src/weights/pallet_cubikan.rs");
    let source = fs::read_to_string(&weights_path).expect("read fixture generated weights");
    let mutated = source.replacen(
        "//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION",
        "//! provisional fake-marker generated using CLI version",
        1,
    );
    assert_ne!(source, mutated, "mutation anchor was absent");
    fs::write(weights_path, mutated).expect("write mutated generated weights");

    let rejected = run_static_verifier(&fixture);
    assert!(
        !rejected.status.success(),
        "the verifier accepted a provisional/fake-marker weight file"
    );
    fs::remove_dir_all(fixture).expect("remove generated-weight mutation fixture");

    let proof_fixture = std::env::temp_dir().join(format!(
        "cubikan-t1106-weight-proof-mutation-{}",
        std::process::id()
    ));
    if proof_fixture.exists() {
        fs::remove_dir_all(&proof_fixture).expect("remove stale proof mutation fixture");
    }
    copy_weight_contract(&root, &proof_fixture);
    append_runtime_weight_pins(&proof_fixture);
    let proof_weights_path = proof_fixture.join("chain/runtime/src/weights/pallet_cubikan.rs");
    let original = fs::read_to_string(&proof_weights_path).expect("read proof fixture weights");
    let estimated = "//  Estimated: `459186`";
    let proof_term = "Weight::from_parts(0, 459186)";
    assert!(
        original.contains(estimated) && original.contains(proof_term),
        "measured proof mutation anchors are absent"
    );
    let mutated = original
        .replacen(estimated, "//  Estimated: `1`", 1)
        .replacen(proof_term, "Weight::from_parts(0, 1)", 1);
    fs::write(&proof_weights_path, &mutated).expect("write proof-underflow mutation");
    let mut pins =
        fs::read_to_string(proof_fixture.join("chain/pins.toml")).expect("read proof fixture pins");
    replace_pin_digest(
        &mut pins,
        "weights_sha256",
        &lowercase_hex(&sp_io::hashing::sha2_256(mutated.as_bytes())),
    );
    fs::write(proof_fixture.join("chain/pins.toml"), pins)
        .expect("update coherent proof fixture pin");
    let proof_rejected = run_static_verifier(&proof_fixture);
    assert!(
        !proof_rejected.status.success(),
        "the verifier accepted emitted proof weight below retained raw evidence"
    );
    fs::remove_dir_all(proof_fixture).expect("remove proof mutation fixture");

    let source_fixture = std::env::temp_dir().join(format!(
        "cubikan-t1106-weight-source-mutation-{}",
        std::process::id()
    ));
    if source_fixture.exists() {
        fs::remove_dir_all(&source_fixture).expect("remove stale source mutation fixture");
    }
    copy_weight_contract(&root, &source_fixture);
    append_runtime_weight_pins(&source_fixture);
    let benchmark_source_path = source_fixture.join("chain/pallets/cubikan/src/benchmarking.rs");
    let original =
        fs::read_to_string(&benchmark_source_path).expect("read benchmark source fixture");
    let mutated = original.replacen(
        "GlobalSequence::<T>::put(u64::MAX - 1);",
        "GlobalSequence::<T>::put(0);",
        1,
    );
    assert_ne!(
        original, mutated,
        "benchmark source mutation anchor was absent"
    );
    fs::write(&benchmark_source_path, &mutated).expect("write benchmark source mutation");
    let mut pins =
        fs::read_to_string(source_fixture.join("chain/pins.toml")).expect("read source pins");
    replace_pin_digest(
        &mut pins,
        "benchmark_source_sha256",
        &lowercase_hex(&sp_io::hashing::sha2_256(mutated.as_bytes())),
    );
    fs::write(source_fixture.join("chain/pins.toml"), pins)
        .expect("update coherent benchmark source pin");
    let source_rejected = run_static_verifier(&source_fixture);
    assert!(
        !source_rejected.status.success(),
        "the verifier accepted a coherently repinned benchmark source mutation"
    );
    fs::remove_dir_all(source_fixture).expect("remove benchmark source mutation fixture");
}
