use std::{
    io::Write,
    process::{Command, Output, Stdio},
};

const SUCCESS_REQUEST: &[u8] =
    include_bytes!("../../../tests/fixtures/protocol-v2/cubikan/requests/success_completion.json");
const SUCCESS_STDOUT: &[u8] =
    include_bytes!("../../../tests/fixtures/protocol-v2/cubikan/stdout/success_completion.jsonl");
const SETUP_ERROR_REQUEST: &[u8] = include_bytes!(
    "../../../tests/fixtures/protocol-v2/cubikan/requests/error_invalid_external_reference_missing.json"
);
const SETUP_ERROR_STDOUT: &[u8] = include_bytes!(
    "../../../tests/fixtures/protocol-v2/cubikan/stdout/error_invalid_external_reference_missing.jsonl"
);
const OPERATION_ERROR_REQUEST: &[u8] = include_bytes!(
    "../../../tests/fixtures/protocol-v2/cubikan/requests/error_transition_not_allowed.json"
);
const OPERATION_ERROR_STDOUT: &[u8] = include_bytes!(
    "../../../tests/fixtures/protocol-v2/cubikan/stdout/error_transition_not_allowed.jsonl"
);

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

fn assert_invocation(input: &[u8], expected_stdout: &[u8], exit_code: i32) {
    let output = invoke(input);
    assert_eq!(output.status.code(), Some(exit_code));
    assert_eq!(output.stdout, expected_stdout);
    assert!(output.stderr.is_empty());
}

#[test]
fn binary_uses_the_locked_success_and_failure_exits() {
    assert_invocation(SUCCESS_REQUEST, SUCCESS_STDOUT, 0);
    assert_invocation(SETUP_ERROR_REQUEST, SETUP_ERROR_STDOUT, 2);
    assert_invocation(OPERATION_ERROR_REQUEST, OPERATION_ERROR_STDOUT, 3);
}

#[test]
fn separate_processes_share_no_lifecycle_state() {
    assert_invocation(SUCCESS_REQUEST, SUCCESS_STDOUT, 0);
    assert_invocation(SUCCESS_REQUEST, SUCCESS_STDOUT, 0);
}
