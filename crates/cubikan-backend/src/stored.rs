use cubikan_core::{IntentUnit, IntentUnitRevision};
use serde::{Deserialize, Serialize};

use crate::BackendError;

/// Historical envelope identity. Envelope v1 remains immutable and unsupported.
pub(crate) const ENVELOPE_VERSION: u64 = 1;

// These declarations intentionally retain the exact historical envelope-v1
// shape. Required origin is not added to them: doing so would silently redefine
// stored bytes that already have a version identity.
#[allow(dead_code)]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredEnvelopeV1 {
    representation_version: u64,
    id: String,
    species: String,
    phase: String,
    revision: String,
    status: StoredStatusV1,
    workflow: StoredWorkflowV1,
    history: Vec<StoredLifecycleRecordV1>,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum StoredStatusV1 {
    Active,
    Completed,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredWorkflowV1 {
    id: String,
    phases: Vec<String>,
    initial_phase: String,
    edges: Vec<StoredWorkflowEdgeV1>,
    completion_phases: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredWorkflowEdgeV1 {
    from: String,
    to: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase", tag = "type")]
enum StoredLifecycleRecordV1 {
    Transition {
        sequence: u64,
        from: String,
        to: String,
    },
    Completion {
        sequence: u64,
        phase: String,
    },
}

/// Rejects encoding into the retired originless envelope generation.
pub(crate) fn encode_envelope(_unit: &IntentUnit) -> Result<String, BackendError> {
    Err(BackendError::UnsupportedEnvelopeVersion {
        found: ENVELOPE_VERSION,
    })
}

/// Identifies and rejects a retired envelope without replay or attribution.
pub(crate) fn decode_envelope(_bytes: &[u8]) -> Result<IntentUnit, BackendError> {
    Err(BackendError::UnsupportedEnvelopeVersion {
        found: ENVELOPE_VERSION,
    })
}

/// Renders a revision as its canonical unsigned-decimal JSON-boundary text.
pub(crate) fn encode_revision_text(revision: IntentUnitRevision) -> String {
    revision.value().to_string()
}

/// Parses only canonical unsigned-decimal revision text in the complete `u64` range.
pub(crate) fn decode_revision_text(value: &str) -> Result<IntentUnitRevision, BackendError> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| BackendError::CorruptEnvelope)?;
    if parsed.to_string() != value {
        return Err(BackendError::CorruptEnvelope);
    }
    Ok(IntentUnitRevision::new(parsed))
}

/// Encodes the SQL revision projection as exactly eight big-endian bytes.
pub(crate) const fn encode_revision_blob(revision: IntentUnitRevision) -> [u8; 8] {
    revision.value().to_be_bytes()
}

/// Decodes an exact eight-byte big-endian SQL revision projection.
pub(crate) fn decode_revision_blob(bytes: &[u8]) -> Result<IntentUnitRevision, BackendError> {
    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| BackendError::ProjectionMismatch)?;
    Ok(IntentUnitRevision::new(u64::from_be_bytes(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEGACY_ENVELOPE: &[u8] = br#"{
        "representation_version":1,
        "id":"67e55044-10b1-426f-9247-bb680e5fe0c8",
        "species":"feature",
        "phase":"queued",
        "revision":"0",
        "status":"active",
        "workflow":{
            "id":"delivery",
            "phases":["queued"],
            "initial_phase":"queued",
            "edges":[],
            "completion_phases":[]
        },
        "history":[]
    }"#;

    #[test]
    fn test_envelope_v1_decode_is_unsupported_without_rewriting_input() {
        let bytes = LEGACY_ENVELOPE.to_vec();
        let before = bytes.clone();
        assert_eq!(
            decode_envelope(&bytes),
            Err(BackendError::UnsupportedEnvelopeVersion { found: 1 })
        );
        assert_eq!(bytes, before);
    }

    #[test]
    fn test_revision_codecs_remain_exact_for_projection_boundaries() {
        for value in [0, i64::MAX as u64 + 1, u64::MAX] {
            let revision = IntentUnitRevision::new(value);
            let text = encode_revision_text(revision);
            assert_eq!(decode_revision_text(&text), Ok(revision));
            let blob = encode_revision_blob(revision);
            assert_eq!(decode_revision_blob(&blob), Ok(revision));
        }
    }
}
