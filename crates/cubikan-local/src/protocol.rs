use serde::{Deserialize, Serialize, de::IgnoredAny};

/// Version retained only as an unsupported compatibility bridge.
pub const PROTOCOL_VERSION: u64 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponseClass {
    RequestRejected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutedRequest {
    class: ResponseClass,
    body: Vec<u8>,
}

impl ExecutedRequest {
    #[must_use]
    pub const fn class(&self) -> ResponseClass {
        self.class
    }

    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    #[must_use]
    pub fn into_body(self) -> Vec<u8> {
        self.body
    }
}

#[derive(Debug, Deserialize)]
struct ProtocolVersionProbe {
    protocol_version: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ErrorCode {
    MalformedJson,
    RequestTooLarge,
    InvalidRequest,
    UnsupportedProtocolVersion,
}

#[derive(Serialize)]
struct FailureResponseV1 {
    protocol_version: u64,
    outcome: &'static str,
    error: ErrorV1,
}

#[derive(Serialize)]
struct ErrorV1 {
    code: ErrorCode,
    message: String,
}

pub(crate) fn unsupported_bridge_response(bytes: &[u8]) -> ExecutedRequest {
    let mut syntax = serde_json::Deserializer::from_slice(bytes);
    if let Err(error) = IgnoredAny::deserialize(&mut syntax).and_then(|_| syntax.end()) {
        return failure_response(ErrorCode::MalformedJson, error.to_string());
    }

    let probe: ProtocolVersionProbe = match serde_json::from_slice(bytes) {
        Ok(probe) => probe,
        Err(_) => {
            return failure_response(
                ErrorCode::InvalidRequest,
                "request must be an object containing an integer protocol_version",
            );
        }
    };
    failure_response(
        ErrorCode::UnsupportedProtocolVersion,
        format!("protocol version {} is unsupported", probe.protocol_version),
    )
}

pub(crate) fn request_too_large_response(max_bytes: usize) -> ExecutedRequest {
    failure_response(
        ErrorCode::RequestTooLarge,
        format!("request exceeds the {max_bytes}-byte limit"),
    )
}

fn failure_response(code: ErrorCode, message: impl Into<String>) -> ExecutedRequest {
    let response = FailureResponseV1 {
        protocol_version: PROTOCOL_VERSION,
        outcome: "failure",
        error: ErrorV1 {
            code,
            message: message.into(),
        },
    };
    ExecutedRequest {
        class: ResponseClass::RequestRejected,
        body: serde_json::to_vec(&response)
            .expect("adapter-owned failure responses must always serialize"),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    fn response(bytes: &[u8]) -> Value {
        serde_json::from_slice(unsupported_bridge_response(bytes).body())
            .expect("response should be JSON")
    }

    #[test]
    fn bridge_probes_only_the_version_before_rejecting_v1() {
        let bytes = serde_json::to_vec(&json!({
            "protocol_version": 1,
            "operation": {
                "type": "create",
                "intent_unit": {"id": null, "origin": null},
                "workflow": "not interpreted"
            }
        }))
        .expect("fixture should serialize");

        let response = response(&bytes);

        assert_eq!(response["error"]["code"], "unsupported_protocol_version");
        assert_eq!(
            response["error"]["message"],
            "protocol version 1 is unsupported"
        );
    }

    #[test]
    fn bridge_preserves_malformed_and_invalid_request_classes() {
        assert_eq!(response(b"{")["error"]["code"], "malformed_json");
        assert_eq!(
            response(br#"{"protocol_version":"1"}"#)["error"]["code"],
            "invalid_request"
        );
    }
}
