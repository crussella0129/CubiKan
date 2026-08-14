use crate::protocol::{ErrorCode, ErrorDetail};

pub(crate) fn reject_unsupported_protocol(version: u64) -> ErrorDetail {
    ErrorDetail {
        code: ErrorCode::UnsupportedProtocolVersion,
        message: format!("protocol version {version} is unsupported"),
    }
}
