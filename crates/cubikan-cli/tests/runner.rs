use std::io::{self, Write};

use cubikan_cli::{MAX_REQUEST_BYTES, RunError, RunStatus, run};
use serde_json::{Value, json};

const V1_FIXTURE: &[u8] = include_bytes!("fixtures/lifecycle-success-v1.json");

#[derive(Default)]
struct RecordingWriter {
    bytes: Vec<u8>,
    flush_offsets: Vec<usize>,
}

impl Write for RecordingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_offsets.push(self.bytes.len());
        Ok(())
    }
}

fn response(writer: &RecordingWriter) -> Value {
    assert_eq!(writer.bytes.last(), Some(&b'\n'));
    assert_eq!(
        writer.bytes.iter().filter(|byte| **byte == b'\n').count(),
        1
    );
    assert_eq!(writer.flush_offsets, [writer.bytes.len()]);
    serde_json::from_slice(&writer.bytes).expect("response should be JSON")
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
fn runner_rejects_v1_with_one_newline_and_one_flush() {
    let mut writer = RecordingWriter::default();

    let status = run(V1_FIXTURE, &mut writer).expect("response should be delivered");

    assert_eq!(status, RunStatus::RequestRejected);
    assert_eq!(
        response(&writer),
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
fn runner_preserves_the_one_mib_bound() {
    assert_eq!(MAX_REQUEST_BYTES, 1_048_576);
    let mut exact_writer = RecordingWriter::default();
    let exact = padded_fixture(MAX_REQUEST_BYTES);
    assert_eq!(
        run(exact.as_slice(), &mut exact_writer).expect("exact request should respond"),
        RunStatus::RequestRejected
    );
    assert_eq!(
        response(&exact_writer)["error"]["code"],
        "unsupported_protocol_version"
    );

    let mut oversized_writer = RecordingWriter::default();
    let oversized = padded_fixture(MAX_REQUEST_BYTES + 1);
    assert_eq!(
        run(oversized.as_slice(), &mut oversized_writer).expect("oversized request should respond"),
        RunStatus::RequestRejected
    );
    assert_eq!(
        response(&oversized_writer)["error"]["code"],
        "request_too_large"
    );
}

#[test]
fn runner_surfaces_read_failures_without_a_modeled_response() {
    struct FailingReader;
    impl io::Read for FailingReader {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("fixture read failure"))
        }
    }

    match run(FailingReader, Vec::new()) {
        Err(RunError::Read(error)) => assert_eq!(error.to_string(), "fixture read failure"),
        other => panic!("expected read failure, got {other:?}"),
    }
}

#[test]
fn removed_authority_symbols_are_absent_from_the_bridge() {
    let execution = include_str!("../src/execution.rs");
    let runner = include_str!("../src/runner.rs");
    for forbidden in [
        "IntentUnit::new",
        "IntentUnitId::generate",
        "SqliteBackend",
        "CreateIntentUnit",
        "synthetic_origin",
    ] {
        assert!(
            !execution.contains(forbidden),
            "execution retains {forbidden}"
        );
        assert!(!runner.contains(forbidden), "runner retains {forbidden}");
    }
}
