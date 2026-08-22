use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use cubikan_local::MAX_REQUEST_BYTES;

const USAGE: &[u8] =
    include_bytes!("../../../tests/fixtures/protocol-v2/cubikan-local/process/usage.txt");
const REQUEST_TOO_LARGE: &[u8] = include_bytes!(
    "../../../tests/fixtures/protocol-v2/cubikan-local/stdout/error_request_too_large.jsonl"
);

fn database(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "cubikan-local-cli-{}-{label}.sqlite3",
        std::process::id()
    ))
}

fn invoke(database: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cubikan-local"))
        .arg("--database")
        .arg(database)
        .arg("--rpc")
        .arg("ws://127.0.0.1:9944/")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("cubikan-local should start");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(input)
        .expect("fixture should be written");
    child.wait_with_output().expect("process should finish")
}

#[test]
fn binary_rejects_structural_arguments_with_exact_usage() {
    let output = Command::new(env!("CARGO_BIN_EXE_cubikan-local"))
        .arg("--help")
        .output()
        .expect("cubikan-local should run");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, USAGE);
}

#[test]
fn binary_models_the_exact_one_byte_lookahead_rejection() {
    let database = database("oversized");
    let output = invoke(&database, &vec![b' '; MAX_REQUEST_BYTES + 1]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, REQUEST_TOO_LARGE);
    assert!(output.stderr.is_empty());
    assert!(!database.exists());
}
