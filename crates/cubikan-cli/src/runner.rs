use std::{error::Error, fmt, io, io::Read, io::Write};

use serde_json::error::Category;

use crate::{
    MAX_REQUEST_BYTES,
    execution::{execute, prepare},
    protocol::{ErrorCode, ErrorDetail, ProtocolRequest, ProtocolResponse},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunStatus {
    Success,
    RequestRejected,
    LifecycleRejected,
}

#[derive(Debug)]
pub enum RunError {
    Read(io::Error),
    WriteResponse(serde_json::Error),
    WriteNewline(io::Error),
}

impl fmt::Display for RunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(formatter, "failed to read request: {error}"),
            Self::WriteResponse(error) => write!(formatter, "failed to write response: {error}"),
            Self::WriteNewline(error) => {
                write!(formatter, "failed to finish response line: {error}")
            }
        }
    }
}

impl Error for RunError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read(error) => Some(error),
            Self::WriteResponse(error) => Some(error),
            Self::WriteNewline(error) => Some(error),
        }
    }
}

pub fn run<R: Read, W: Write>(reader: R, mut writer: W) -> Result<RunStatus, RunError> {
    let mut input = Vec::new();
    let mut bounded_reader = reader.take((MAX_REQUEST_BYTES + 1) as u64);
    bounded_reader
        .read_to_end(&mut input)
        .map_err(RunError::Read)?;

    if input.len() > MAX_REQUEST_BYTES {
        let response = ProtocolResponse::error(
            ErrorDetail {
                code: ErrorCode::RequestTooLarge,
                message: format!("request exceeds maximum size of {MAX_REQUEST_BYTES} bytes"),
                field: None,
                operation_number: None,
            },
            None,
        );
        return write_response(&mut writer, &response, RunStatus::RequestRejected);
    }

    let request = match serde_json::from_slice::<ProtocolRequest>(&input) {
        Ok(request) => request,
        Err(error) => {
            let code = match error.classify() {
                Category::Syntax | Category::Eof => ErrorCode::InvalidJson,
                Category::Data => ErrorCode::InvalidRequest,
                Category::Io => unreachable!("slice decoding cannot produce an I/O error"),
            };
            let response = ProtocolResponse::error(
                ErrorDetail {
                    code,
                    message: error.to_string(),
                    field: None,
                    operation_number: None,
                },
                None,
            );
            return write_response(&mut writer, &response, RunStatus::RequestRejected);
        }
    };

    let prepared = match prepare(request) {
        Ok(prepared) => prepared,
        Err(error) => {
            let response = ProtocolResponse::error(error, None);
            return write_response(&mut writer, &response, RunStatus::RequestRejected);
        }
    };
    let (response, status) = match execute(prepared) {
        Ok(intent_unit) => (ProtocolResponse::success(intent_unit), RunStatus::Success),
        Err(rejection) => (
            ProtocolResponse::error(rejection.error, Some(*rejection.intent_unit)),
            RunStatus::LifecycleRejected,
        ),
    };
    write_response(&mut writer, &response, status)
}

fn write_response<W: Write>(
    writer: &mut W,
    response: &ProtocolResponse,
    status: RunStatus,
) -> Result<RunStatus, RunError> {
    serde_json::to_writer(&mut *writer, response).map_err(RunError::WriteResponse)?;
    writer.write_all(b"\n").map_err(RunError::WriteNewline)?;
    Ok(status)
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, io::Cursor, rc::Rc};

    use serde_json::{Value, json};

    use super::*;

    fn request(operations: Value) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "protocol_version": 1,
            "workflow": {
                "id": "delivery",
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

    fn response(output: &[u8]) -> Value {
        assert_eq!(output.last(), Some(&b'\n'));
        assert_eq!(output.iter().filter(|byte| **byte == b'\n').count(), 1);
        serde_json::from_slice(output).expect("response should be valid JSON")
    }

    fn request_with_final_brace_at(length: usize, operations: Value) -> Vec<u8> {
        let mut input = request(operations);
        assert_eq!(input.pop(), Some(b'}'));
        assert!(input.len() < length);
        input.resize(length - 1, b' ');
        input.push(b'}');
        assert_eq!(input.len(), length);
        assert_eq!(input[length - 1], b'}');
        input
    }

    fn assert_request_too_large(status: RunStatus, output: &[u8]) {
        assert_eq!(status, RunStatus::RequestRejected);
        assert_eq!(
            response(output),
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

    struct CountingReader {
        remaining: usize,
        consumed: Rc<Cell<usize>>,
    }

    impl Read for CountingReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if buffer.is_empty() || self.remaining == 0 {
                return Ok(0);
            }

            let count = buffer.len().min(self.remaining);
            buffer[..count].fill(b'!');
            self.remaining -= count;
            self.consumed.set(self.consumed.get() + count);
            Ok(count)
        }
    }

    struct ErrorAfterReader {
        remaining_before_error: usize,
        consumed: Rc<Cell<usize>>,
        error_observed: Rc<Cell<bool>>,
    }

    impl Read for ErrorAfterReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if buffer.is_empty() {
                return Ok(0);
            }
            if self.remaining_before_error == 0 {
                self.error_observed.set(true);
                return Err(io::Error::other("fixture boundary read failure"));
            }

            let count = buffer.len().min(self.remaining_before_error);
            buffer[..count].fill(b'!');
            self.remaining_before_error -= count;
            self.consumed.set(self.consumed.get() + count);
            Ok(count)
        }
    }

    #[test]
    fn test_run_writes_one_success_document() {
        let input = request(json!([
            {"type": "transition", "target": "doing"},
            {"type": "transition", "target": "done"},
            {"type": "complete"}
        ]));
        let mut output = Vec::new();

        let status = run(input.as_slice(), &mut output).expect("runner should succeed");
        let response = response(&output);

        assert_eq!(status, RunStatus::Success);
        assert_eq!(response["outcome"], "success");
        assert_eq!(response["intent_unit"]["status"], "completed");
    }

    #[test]
    fn test_run_classifies_json_syntax_and_shape_failures() {
        let cases = [
            (br#"{"protocol_version":"#.as_slice(), "invalid_json"),
            (
                br#"{"protocol_version":"one"}"#.as_slice(),
                "invalid_request",
            ),
            (
                br#"{"protocol_version":1,"unexpected":true}"#.as_slice(),
                "invalid_request",
            ),
        ];

        for (input, expected_code) in cases {
            let mut output = Vec::new();
            let status = run(input, &mut output).expect("modeled failure should serialize");
            let response = response(&output);
            assert_eq!(status, RunStatus::RequestRejected);
            assert_eq!(response["error"]["code"], expected_code);
            assert!(response.get("intent_unit").is_none());
        }
    }

    #[test]
    fn test_run_writes_setup_rejection_without_state() {
        let mut value: Value = serde_json::from_slice(&request(json!([])))
            .expect("fixture request should deserialize");
        value["protocol_version"] = json!(2);
        let input = serde_json::to_vec(&value).expect("fixture should serialize");
        let mut output = Vec::new();

        let status = run(input.as_slice(), &mut output).expect("failure should serialize");
        let response = response(&output);

        assert_eq!(status, RunStatus::RequestRejected);
        assert_eq!(response["error"]["code"], "unsupported_protocol_version");
        assert!(response.get("intent_unit").is_none());
    }

    #[test]
    fn test_run_writes_lifecycle_rejection_with_prior_state() {
        let input = request(json!([
            {"type": "transition", "target": "doing"},
            {"type": "transition", "target": "queued"},
            {"type": "transition", "target": "done"}
        ]));
        let mut output = Vec::new();

        let status = run(input.as_slice(), &mut output).expect("failure should serialize");
        let response = response(&output);

        assert_eq!(status, RunStatus::LifecycleRejected);
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

    #[test]
    fn test_run_preserves_below_limit_result_classes() {
        let success = request(json!([]));
        let mut output = Vec::new();
        assert_eq!(
            run(success.as_slice(), &mut output).expect("success should serialize"),
            RunStatus::Success
        );

        let mut setup: Value =
            serde_json::from_slice(&request(json!([]))).expect("fixture should decode");
        setup["protocol_version"] = json!(2);
        let setup = serde_json::to_vec(&setup).expect("fixture should serialize");
        let mut output = Vec::new();
        assert_eq!(
            run(setup.as_slice(), &mut output).expect("setup rejection should serialize"),
            RunStatus::RequestRejected
        );
        assert_eq!(
            response(&output)["error"]["code"],
            "unsupported_protocol_version"
        );

        let lifecycle = request(json!([
            {"type": "transition", "target": "doing"},
            {"type": "transition", "target": "queued"}
        ]));
        let mut output = Vec::new();
        assert_eq!(
            run(lifecycle.as_slice(), &mut output).expect("lifecycle rejection should serialize"),
            RunStatus::LifecycleRejected
        );
        assert_eq!(response(&output)["error"]["code"], "transition_not_allowed");
    }

    #[test]
    fn test_run_accepts_valid_json_at_exact_limit() {
        let input = request_with_final_brace_at(MAX_REQUEST_BYTES, json!([]));
        let mut output = Vec::new();

        let status = run(input.as_slice(), &mut output).expect("exact-limit input should run");

        assert_eq!(status, RunStatus::Success);
        assert_eq!(response(&output)["outcome"], "success");
    }

    #[test]
    fn test_run_rejects_oversize_before_json_classification() {
        let cases = [
            request_with_final_brace_at(MAX_REQUEST_BYTES + 1, json!([])),
            vec![b'!'; MAX_REQUEST_BYTES + 1],
        ];

        for input in cases {
            let mut output = Vec::new();
            let status =
                run(input.as_slice(), &mut output).expect("oversized rejection should serialize");
            assert_request_too_large(status, &output);
        }
    }

    #[test]
    fn test_run_consumes_at_most_limit_plus_one() {
        let consumed = Rc::new(Cell::new(0));
        let reader = CountingReader {
            remaining: MAX_REQUEST_BYTES + 4_096,
            consumed: Rc::clone(&consumed),
        };
        let mut output = Vec::new();

        let status = run(reader, &mut output).expect("oversized rejection should serialize");

        assert_request_too_large(status, &output);
        assert_eq!(consumed.get(), MAX_REQUEST_BYTES + 1);
    }

    #[test]
    fn test_run_preserves_boundary_io_precedence() {
        let consumed = Rc::new(Cell::new(0));
        let error_observed = Rc::new(Cell::new(false));
        let reader = ErrorAfterReader {
            remaining_before_error: MAX_REQUEST_BYTES,
            consumed: Rc::clone(&consumed),
            error_observed: Rc::clone(&error_observed),
        };
        assert!(matches!(
            run(reader, Cursor::new(Vec::new())),
            Err(RunError::Read(_))
        ));
        assert_eq!(consumed.get(), MAX_REQUEST_BYTES);
        assert!(error_observed.get());

        let consumed = Rc::new(Cell::new(0));
        let error_observed = Rc::new(Cell::new(false));
        let reader = ErrorAfterReader {
            remaining_before_error: MAX_REQUEST_BYTES + 1,
            consumed: Rc::clone(&consumed),
            error_observed: Rc::clone(&error_observed),
        };
        let mut output = Vec::new();
        let status = run(reader, &mut output).expect("overflow should precede later I/O error");
        assert_request_too_large(status, &output);
        assert_eq!(consumed.get(), MAX_REQUEST_BYTES + 1);
        assert!(!error_observed.get());
    }

    #[test]
    fn test_run_propagates_input_and_output_io_failures() {
        struct FailingReader;
        impl Read for FailingReader {
            fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::other("fixture read failure"))
            }
        }

        struct FailingWriter;
        impl Write for FailingWriter {
            fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
                Err(io::Error::other("fixture write failure"))
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        struct NewlineFailingWriter;
        impl Write for NewlineFailingWriter {
            fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
                if buffer == b"\n" {
                    Err(io::Error::other("fixture newline failure"))
                } else {
                    Ok(buffer.len())
                }
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        assert!(matches!(
            run(FailingReader, Cursor::new(Vec::new())),
            Err(RunError::Read(_))
        ));
        assert!(matches!(
            run(request(json!([])).as_slice(), FailingWriter),
            Err(RunError::WriteResponse(_))
        ));
        assert!(matches!(
            run(request(json!([])).as_slice(), NewlineFailingWriter),
            Err(RunError::WriteNewline(_))
        ));

        let oversized = request_with_final_brace_at(MAX_REQUEST_BYTES + 1, json!([]));
        assert!(matches!(
            run(oversized.as_slice(), FailingWriter),
            Err(RunError::WriteResponse(_))
        ));
        assert!(matches!(
            run(oversized.as_slice(), NewlineFailingWriter),
            Err(RunError::WriteNewline(_))
        ));
    }
}
