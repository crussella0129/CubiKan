use serde::{Deserialize, Serialize, de::IgnoredAny};

pub(crate) const PROTOCOL_VERSION: u64 = 1;

#[derive(Debug, Deserialize)]
pub(crate) struct ProtocolVersionProbe {
    protocol_version: u64,
}

impl ProtocolVersionProbe {
    pub(crate) const fn version(&self) -> u64 {
        self.protocol_version
    }
}

pub(crate) fn decode_protocol_version(bytes: &[u8]) -> Result<ProtocolVersionProbe, ErrorDetail> {
    let mut syntax = serde_json::Deserializer::from_slice(bytes);
    IgnoredAny::deserialize(&mut syntax)
        .and_then(|_| syntax.end())
        .map_err(|error| ErrorDetail {
            code: ErrorCode::InvalidJson,
            message: error.to_string(),
        })?;

    serde_json::from_slice(bytes).map_err(|_| ErrorDetail {
        code: ErrorCode::InvalidRequest,
        message: "request must be an object containing an integer protocol_version".to_owned(),
    })
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub(crate) enum ProtocolResponse {
    Error {
        protocol_version: u64,
        error: ErrorDetail,
    },
}

impl ProtocolResponse {
    pub(crate) const fn error(error: ErrorDetail) -> Self {
        Self::Error {
            protocol_version: PROTOCOL_VERSION,
            error,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ErrorDetail {
    pub(crate) code: ErrorCode,
    pub(crate) message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ErrorCode {
    InvalidJson,
    InvalidRequest,
    RequestTooLarge,
    UnsupportedProtocolVersion,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn version_probe_ignores_removed_v1_authority_fields() {
        let bytes = serde_json::to_vec(&json!({
            "protocol_version": 1,
            "intent_unit": {"id": null, "origin": null},
            "workflow": "not interpreted",
            "operations": [{"type": "removed"}]
        }))
        .expect("fixture should serialize");

        let probe = decode_protocol_version(&bytes).expect("version probe should succeed");

        assert_eq!(probe.version(), 1);
    }

    #[test]
    fn version_probe_keeps_json_and_request_taxonomy() {
        assert_eq!(
            decode_protocol_version(b"{")
                .expect_err("malformed JSON must reject")
                .code,
            ErrorCode::InvalidJson
        );
        assert_eq!(
            decode_protocol_version(br#"{"protocol_version":"1"}"#)
                .expect_err("non-integer version must reject")
                .code,
            ErrorCode::InvalidRequest
        );
    }
}
