use std::io::{self, BufWriter, Write};

use cubikan_cli::{MAX_REQUEST_BYTES, RunError, RunStatus, run};
use serde_json::{Value, json};

fn request(operations: Value) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "protocol_version": 1,
        "workflow": {
            "id": "custom-delivery",
            "phases": ["queued", "doing", "done"],
            "initial_phase": "queued",
            "edges": [
                {"from": "queued", "to": "doing"},
                {"from": "doing", "to": "done"}
            ],
            "completion_phases": ["done"]
        },
        "intent_unit": {
            "id": "67e55044-10b1-426f-9247-bb680e5fe0c8",
            "species": "feature"
        },
        "operations": operations
    }))
    .expect("fixture should serialize")
}

fn invoke(input: &[u8]) -> (RunStatus, Value) {
    let mut output = Vec::new();
    let status = run(input, &mut output).expect("modeled result should serialize");
    assert_eq!(output.last(), Some(&b'\n'));
    assert_eq!(output.iter().filter(|byte| **byte == b'\n').count(), 1);
    let response = serde_json::from_slice(&output).expect("response should be valid JSON");
    (status, response)
}

fn request_with_final_brace_at(length: usize, operations: Value) -> Vec<u8> {
    let mut input = request(operations);
    assert_eq!(input.pop(), Some(b'}'));
    assert!(input.len() < length);
    input.resize(length - 1, b' ');
    input.push(b'}');
    assert_eq!(input.len(), length);
    input
}

#[test]
fn test_request_limit_is_one_mib() {
    let _: usize = MAX_REQUEST_BYTES;
    assert_eq!(MAX_REQUEST_BYTES, 1_048_576);
}

#[test]
fn test_runner_exposes_io_read_error_payload() {
    struct FailingReader;

    impl io::Read for FailingReader {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("fixture read failure"))
        }
    }

    let error: io::Error = match run(FailingReader, Vec::new()) {
        Err(RunError::Read(error)) => error,
        other => panic!("expected public I/O read error payload, got {other:?}"),
    };

    assert_eq!(error.kind(), io::ErrorKind::Other);
    assert_eq!(error.to_string(), "fixture read failure");
}

#[test]
fn test_runner_accepts_exact_limit_request() {
    let operations = json!([
        {"type": "transition", "target": "doing"},
        {"type": "transition", "target": "done"},
        {"type": "complete"}
    ]);
    let below_limit = request(operations.clone());
    let exact_limit = request_with_final_brace_at(MAX_REQUEST_BYTES, operations);

    let expected = invoke(&below_limit);
    let actual = invoke(&exact_limit);

    assert_eq!(actual, expected);
}

#[test]
fn test_runner_rejects_one_byte_over_limit() {
    let input = request_with_final_brace_at(MAX_REQUEST_BYTES + 1, json!([]));

    let (status, response) = invoke(&input);

    assert_eq!(status, RunStatus::RequestRejected);
    assert_eq!(
        response,
        json!({
            "outcome": "error",
            "protocol_version": 1,
            "error": {
                "code": "request_too_large",
                "message": "request exceeds maximum size of 1048576 bytes"
            }
        })
    );
}

#[test]
fn test_runner_executes_configure_create_transition_complete() {
    let input = request(json!([
        {"type": "transition", "target": "doing"},
        {"type": "transition", "target": "done"},
        {"type": "complete"}
    ]));

    let (status, response) = invoke(&input);

    assert_eq!(status, RunStatus::Success);
    assert_eq!(response["outcome"], "success");
    assert_eq!(response["intent_unit"]["workflow_id"], "custom-delivery");
    assert_eq!(response["intent_unit"]["phase"], "done");
    assert_eq!(response["intent_unit"]["status"], "completed");
    assert_eq!(
        response["intent_unit"]["history"].as_array().map(Vec::len),
        Some(3)
    );
}

#[test]
fn test_runner_returns_request_failure_without_unit_state() {
    let mut value: Value =
        serde_json::from_slice(&request(json!([]))).expect("fixture should decode");
    value["workflow"]["phases"] = json!([]);
    let input = serde_json::to_vec(&value).expect("fixture should serialize");

    let (status, response) = invoke(&input);

    assert_eq!(status, RunStatus::RequestRejected);
    assert_eq!(response["error"]["code"], "workflow_empty_phases");
    assert!(response.get("intent_unit").is_none());
}

#[test]
fn test_runner_preserves_prior_successes_on_lifecycle_failure() {
    let input = request(json!([
        {"type": "transition", "target": "doing"},
        {"type": "transition", "target": "queued"},
        {"type": "transition", "target": "done"}
    ]));

    let (status, response) = invoke(&input);

    assert_eq!(status, RunStatus::LifecycleRejected);
    assert_eq!(response["error"]["operation_number"], 2);
    assert_eq!(
        response["intent_unit"],
        json!({
            "id": "67e55044-10b1-426f-9247-bb680e5fe0c8",
            "species": "feature",
            "workflow_id": "custom-delivery",
            "phase": "doing",
            "status": "active",
            "history": [{
                "type": "transition",
                "sequence": 1,
                "from": "queued",
                "to": "doing"
            }]
        })
    );
}

#[test]
fn test_runner_propagates_output_io_failure() {
    struct FailingWriter;

    impl io::Write for FailingWriter {
        fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("fixture output failure"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let result = run(request(json!([])).as_slice(), FailingWriter);
    assert!(matches!(result, Err(RunError::WriteResponse(_))));
}

#[test]
fn test_runner_surfaces_buffered_sink_failure_on_explicit_flush() {
    #[derive(Default)]
    struct DrainFailingSink {
        write_attempts: usize,
        flush_attempts: usize,
    }

    impl Write for DrainFailingSink {
        fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
            self.write_attempts += 1;
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "fixture buffered drain failure",
            ))
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flush_attempts += 1;
            Ok(())
        }
    }

    let input = request(json!([]));
    let mut writer = BufWriter::with_capacity(4_096, DrainFailingSink::default());
    let error = run(input.as_slice(), &mut writer)
        .expect_err("explicit flush should expose the buffered sink failure");

    let RunError::FlushResponse(payload) = &error else {
        panic!("expected public flush response error, got {error:?}");
    };
    assert_eq!(payload.kind(), io::ErrorKind::BrokenPipe);
    assert_eq!(payload.to_string(), "fixture buffered drain failure");
    assert_eq!(
        error.to_string(),
        "failed to flush response: fixture buffered drain failure"
    );
    let source = std::error::Error::source(&error)
        .expect("flush response error should expose its source")
        .downcast_ref::<io::Error>()
        .expect("flush response source should remain an I/O error");
    assert_eq!(source.kind(), io::ErrorKind::BrokenPipe);
    assert_eq!(source.to_string(), "fixture buffered drain failure");

    assert_eq!(writer.get_ref().write_attempts, 1);
    assert_eq!(writer.get_ref().flush_attempts, 0);
    assert!(writer.buffer().len() < writer.capacity());
    assert_eq!(writer.buffer().last(), Some(&b'\n'));

    let (sink, buffered) = writer.into_parts();
    let buffered = buffered.expect("fixture writer should not panic");
    assert_eq!(sink.write_attempts, 1);
    assert_eq!(sink.flush_attempts, 0);
    assert_eq!(buffered.last(), Some(&b'\n'));
    let response: Value =
        serde_json::from_slice(&buffered).expect("retained response line should be valid JSON");
    assert_eq!(response["outcome"], "success");
}
