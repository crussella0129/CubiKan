use std::{fs, path::PathBuf, process::Command};

use serde_json::Value;

#[test]
fn test_local_schema_and_fixture_hashes_are_independent() {
    let root = workspace_root();
    let output = Command::new("bash")
        .arg("protocol/v2/verify-fixtures.sh")
        .arg("--locked")
        .current_dir(&root)
        .output()
        .expect("run independently authored local-v2 fixture verifier");
    assert!(
        output.status.success(),
        "local-v2 verifier failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "verifier must keep stderr empty");
    assert_eq!(
        String::from_utf8(output.stdout).expect("verifier stdout must be UTF-8"),
        concat!(
            "verified cubikan protocol v2: ",
            "schema=309697fe6e718c78ef8802861d60a660500a985c05b5a94aaba35a28fb2cb4a3 ",
            "manifest=46eab998ec22d8c806c7f8ac347aa89efb4f69578c7a34f6ee4737fc24e97c75 ",
            "cases=96 io_cases=4\n",
            "verified cubikan-local protocol v2: ",
            "schema=869d30e3865cbaf80af60616fdf13d0e4b848997bb36f4bcc68f0e7cc2d7d4ff ",
            "manifest=b19ff73569f612cb3b365d2af83284e99e1cb2b7cc529685aa58267ea23289cc ",
            "inventory=1e74987aef0a03cdf3cf2039402ce6c361363f6cb5a710fc3a008b590d834b9a ",
            "cases=281 semantic=94 structural=187 process_cases=72 io_cases=8 ",
            "source_cells=153 files=703\n",
        )
    );

    let inventory: Value = serde_json::from_slice(
        &fs::read(root.join("tests/fixtures/protocol-v2/cubikan-local/inventory-v1.json"))
            .expect("read frozen local-v2 inventory"),
    )
    .expect("parse frozen local-v2 inventory");
    let manifest: Value = serde_json::from_slice(
        &fs::read(root.join("tests/fixtures/protocol-v2/cubikan-local/manifest-v1.json"))
            .expect("read frozen local-v2 manifest"),
    )
    .expect("parse frozen local-v2 manifest");

    let expected_case_count = inventory["semantic_spine"]["total"]
        .as_u64()
        .expect("semantic total must be u64")
        + inventory["structural_corpus"]["total"]
            .as_u64()
            .expect("structural total must be u64");
    assert_eq!(
        manifest["cases"]
            .as_array()
            .map(Vec::len)
            .and_then(|count| u64::try_from(count).ok()),
        Some(expected_case_count)
    );
    assert_eq!(inventory["public_schema"]["operation_count"], 15);
    assert_eq!(
        inventory["public_schema"]["result_tags"]
            .as_array()
            .map(Vec::len),
        Some(6)
    );
    assert_eq!(
        inventory["public_schema"]["mutation_outcomes"]
            .as_array()
            .map(Vec::len),
        Some(7)
    );
    assert_eq!(
        inventory["public_schema"]["accepted_effect_tags"]
            .as_array()
            .map(Vec::len),
        Some(8)
    );
    assert_eq!(
        inventory["error_registry"].as_array().map(Vec::len),
        Some(74)
    );
    assert_eq!(inventory["public_schema"]["command_schema_version"], 1);
    assert_eq!(
        inventory["public_schema"]["caller_selectable_command_schema_version"],
        false
    );
}

#[test]
fn local_crate_exposes_only_the_bounded_process_surface() {
    let root = workspace_root();
    let library = fs::read_to_string(root.join("crates/cubikan-local/src/lib.rs"))
        .expect("read local public surface");
    for forbidden in [
        "pub use execution",
        "pub use protocol",
        "pub fn decode_request",
        "pub fn submit",
        "pub fn open_projection",
    ] {
        assert!(
            !library.contains(forbidden),
            "local crate exposed forbidden authority: {forbidden}"
        );
    }
    assert!(library.contains("pub use runner::{MAX_REQUEST_BYTES, run_process};"));
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("local crate must live at workspace/crates/cubikan-local")
        .to_path_buf()
}
