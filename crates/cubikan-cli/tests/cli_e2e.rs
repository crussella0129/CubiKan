use std::{
    io::Write,
    process::{Command, Output, Stdio},
};

use cubikan_cli::MAX_REQUEST_BYTES;
use serde_json::{Value, json};

const V1_FIXTURE: &[u8] = include_bytes!("fixtures/lifecycle-success-v1.json");

fn invoke(input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cubikan"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("cubikan process should start");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(input)
        .expect("fixture should be written");
    child.wait_with_output().expect("process should finish")
}

fn response(output: &Output) -> Value {
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout.last(), Some(&b'\n'));
    assert_eq!(
        output.stdout.iter().filter(|byte| **byte == b'\n').count(),
        1
    );
    serde_json::from_slice(&output.stdout).expect("stdout should contain one JSON value")
}

fn padded_fixture(length: usize) -> Vec<u8> {
    let mut input = V1_FIXTURE.to_vec();
    while input.last().is_some_and(u8::is_ascii_whitespace) {
        input.pop();
    }
    assert_eq!(input.pop(), Some(b'}'));
    assert!(input.len() < length);
    input.resize(length - 1, b' ');
    input.push(b'}');
    input
}

#[test]
fn protocol_v1_fixture_is_an_unsupported_only_bridge() {
    let output = invoke(V1_FIXTURE);

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        response(&output),
        json!({
            "outcome": "error",
            "protocol_version": 1,
            "error": {
                "code": "unsupported_protocol_version",
                "message": "protocol version 1 is unsupported"
            }
        })
    );
}

#[test]
fn binary_preserves_bounded_ingestion_and_request_precedence() {
    let exact = invoke(&padded_fixture(MAX_REQUEST_BYTES));
    assert_eq!(exact.status.code(), Some(2));
    assert_eq!(
        response(&exact)["error"]["code"],
        "unsupported_protocol_version"
    );

    let oversized = invoke(&padded_fixture(MAX_REQUEST_BYTES + 1));
    assert_eq!(oversized.status.code(), Some(2));
    assert_eq!(response(&oversized)["error"]["code"], "request_too_large");
}

#[test]
fn binary_keeps_malformed_json_as_a_request_rejection() {
    let output = invoke(b"{");

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(response(&output)["error"]["code"], "invalid_json");
}
