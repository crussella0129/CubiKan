use std::{
    io::Write,
    process::{Command, Output, Stdio},
};

use serde_json::{Value, json};

const SUCCESS_FIXTURE: &[u8] = include_bytes!("fixtures/lifecycle-success-v1.json");

fn invoke(input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cubikan"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("cubikan process should start");
    let mut stdin = child.stdin.take().expect("child stdin should be piped");
    stdin
        .write_all(input)
        .expect("fixture should be written to child stdin");
    drop(stdin);
    child
        .wait_with_output()
        .expect("cubikan process should finish")
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

#[test]
fn test_cli_configure_create_transition_complete() {
    let output = invoke(SUCCESS_FIXTURE);
    let response = response(&output);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(response["protocol_version"], 1);
    assert_eq!(response["outcome"], "success");
    assert_eq!(
        response["intent_unit"]["id"],
        "67e55044-10b1-426f-9247-bb680e5fe0c8"
    );
    assert_eq!(response["intent_unit"]["phase"], "done");
    assert_eq!(response["intent_unit"]["status"], "completed");
    assert_eq!(
        response["intent_unit"]["history"],
        json!([
            {
                "type": "transition",
                "sequence": 1,
                "from": "queued",
                "to": "doing"
            },
            {
                "type": "transition",
                "sequence": 2,
                "from": "doing",
                "to": "done"
            },
            {
                "type": "completion",
                "sequence": 3,
                "phase": "done"
            }
        ])
    );
}

#[test]
fn test_cli_reports_malformed_request_with_exit_2() {
    let output = invoke(br#"{"protocol_version":"#);
    let response = response(&output);

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(response["protocol_version"], 1);
    assert_eq!(response["outcome"], "error");
    assert_eq!(response["error"]["code"], "invalid_json");
    assert!(response.get("intent_unit").is_none());
}

#[test]
fn test_cli_reports_lifecycle_rejection_with_exit_3() {
    let mut request: Value =
        serde_json::from_slice(SUCCESS_FIXTURE).expect("fixture should be valid JSON");
    request["operations"] = json!([
        {"type": "transition", "target": "doing"},
        {"type": "transition", "target": "queued"},
        {"type": "transition", "target": "done"}
    ]);
    let input = serde_json::to_vec(&request).expect("rejection fixture should serialize");

    let output = invoke(&input);
    let response = response(&output);

    assert_eq!(output.status.code(), Some(3));
    assert_eq!(response["outcome"], "error");
    assert_eq!(response["error"]["code"], "transition_not_allowed");
    assert_eq!(response["error"]["operation_number"], 2);
    assert_eq!(
        response["intent_unit"],
        json!({
            "id": "67e55044-10b1-426f-9247-bb680e5fe0c8",
            "species": "feature",
            "workflow_id": "delivery",
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
