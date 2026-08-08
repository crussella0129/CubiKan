use std::{error::Error, fmt, io, io::Read, io::Write};

use serde_json::error::Category;

use crate::{
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
    Read(serde_json::Error),
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
            Self::Read(error) | Self::WriteResponse(error) => Some(error),
            Self::WriteNewline(error) => Some(error),
        }
    }
}

pub fn run<R: Read, W: Write>(reader: R, mut writer: W) -> Result<RunStatus, RunError> {
    let request = match serde_json::from_reader::<_, ProtocolRequest>(reader) {
        Ok(request) => request,
        Err(error) if error.classify() == Category::Io => return Err(RunError::Read(error)),
        Err(error) => {
            let code = match error.classify() {
                Category::Syntax | Category::Eof => ErrorCode::InvalidJson,
                Category::Data => ErrorCode::InvalidRequest,
                Category::Io => unreachable!("I/O errors return before protocol mapping"),
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
    use std::io::Cursor;

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
        assert_eq!(response["intent_unit"]["phase"], "doing");
        assert_eq!(
            response["intent_unit"]["history"].as_array().map(Vec::len),
            Some(1)
        );
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

        assert!(matches!(
            run(FailingReader, Cursor::new(Vec::new())),
            Err(RunError::Read(_))
        ));
        assert!(matches!(
            run(request(json!([])).as_slice(), FailingWriter),
            Err(RunError::WriteResponse(_))
        ));
    }
}
