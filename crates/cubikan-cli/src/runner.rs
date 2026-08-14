use std::{error::Error, fmt, io, io::Read, io::Write};

use crate::{
    MAX_REQUEST_BYTES,
    execution::reject_unsupported_protocol,
    protocol::{ErrorCode, ErrorDetail, ProtocolResponse, decode_protocol_version},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunStatus {
    RequestRejected,
}

#[derive(Debug)]
pub enum RunError {
    Read(io::Error),
    WriteResponse(serde_json::Error),
    WriteNewline(io::Error),
    FlushResponse(io::Error),
}

impl fmt::Display for RunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(formatter, "failed to read request: {error}"),
            Self::WriteResponse(error) => write!(formatter, "failed to write response: {error}"),
            Self::WriteNewline(error) => {
                write!(formatter, "failed to finish response line: {error}")
            }
            Self::FlushResponse(error) => write!(formatter, "failed to flush response: {error}"),
        }
    }
}

impl Error for RunError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read(error) => Some(error),
            Self::WriteResponse(error) => Some(error),
            Self::WriteNewline(error) => Some(error),
            Self::FlushResponse(error) => Some(error),
        }
    }
}

pub fn run<R: Read, W: Write>(reader: R, mut writer: W) -> Result<RunStatus, RunError> {
    let mut input = Vec::new();
    let mut bounded_reader = reader.take((MAX_REQUEST_BYTES + 1) as u64);
    bounded_reader
        .read_to_end(&mut input)
        .map_err(RunError::Read)?;

    let error = if input.len() > MAX_REQUEST_BYTES {
        ErrorDetail {
            code: ErrorCode::RequestTooLarge,
            message: format!("request exceeds maximum size of {MAX_REQUEST_BYTES} bytes"),
        }
    } else {
        match decode_protocol_version(&input) {
            Ok(probe) => reject_unsupported_protocol(probe.version()),
            Err(error) => error,
        }
    };
    write_response(
        &mut writer,
        &ProtocolResponse::error(error),
        RunStatus::RequestRejected,
    )
}

fn write_response<W: Write>(
    writer: &mut W,
    response: &ProtocolResponse,
    status: RunStatus,
) -> Result<RunStatus, RunError> {
    serde_json::to_writer(&mut *writer, response).map_err(RunError::WriteResponse)?;
    writer.write_all(b"\n").map_err(RunError::WriteNewline)?;
    writer.flush().map_err(RunError::FlushResponse)?;
    Ok(status)
}
