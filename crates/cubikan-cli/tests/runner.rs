use std::io;

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
