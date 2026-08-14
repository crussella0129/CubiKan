use std::str::FromStr;

use cubikan_core::{
    ExternalReference, IntentSpecies, IntentUnit, IntentUnitId, IntentUnitRevision,
    IntentUnitStatus, LifecycleRecord, MAX_COMPLETION_PHASES, MAX_LIFECYCLE_RECORDS,
    MAX_WORKFLOW_EDGES, MAX_WORKFLOW_PHASES, PhaseId, ReferenceNamespace, ReferenceText, Workflow,
    WorkflowEdge, WorkflowId,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::BackendError;

pub(crate) const ENVELOPE_VERSION: u64 = 2;
pub(crate) const MAX_ENVELOPE_BYTES: usize = 2_097_152;

#[derive(Debug, Deserialize)]
struct VersionProbe {
    representation_version: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct U64Text(u64);

impl Serialize for U64Text {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for U64Text {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        let value = text
            .parse::<u64>()
            .map_err(|_| de::Error::custom("u64 text is malformed"))?;
        if value.to_string() != text {
            return Err(de::Error::custom("u64 text is not canonical"));
        }
        Ok(Self(value))
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredEnvelopeV2 {
    representation_version: u64,
    id: String,
    origin: StoredExternalReferenceV2,
    species: String,
    workflow: StoredWorkflowV2,
    phase: String,
    status: StoredStatusV2,
    revision: U64Text,
    history: Vec<StoredLifecycleRecordV2>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredExternalReferenceV2 {
    namespace: String,
    scope: String,
    value: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredWorkflowV2 {
    id: String,
    phases: Vec<String>,
    initial_phase: String,
    edges: Vec<StoredWorkflowEdgeV2>,
    completion_phases: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredWorkflowEdgeV2 {
    from: String,
    to: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum StoredStatusV2 {
    Active,
    Completed,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase", tag = "type")]
enum StoredLifecycleRecordV2 {
    Transition {
        sequence: U64Text,
        from: String,
        to: String,
    },
    Completion {
        sequence: U64Text,
        phase: String,
    },
}

impl StoredEnvelopeV2 {
    fn from_unit(unit: &IntentUnit) -> Self {
        Self {
            representation_version: ENVELOPE_VERSION,
            id: unit.id().to_string(),
            origin: StoredExternalReferenceV2 {
                namespace: unit.origin().namespace().as_str().to_owned(),
                scope: unit.origin().scope().as_str().to_owned(),
                value: unit.origin().value().as_str().to_owned(),
            },
            species: unit.species().as_str().to_owned(),
            workflow: StoredWorkflowV2 {
                id: unit.workflow().id().as_str().to_owned(),
                phases: unit
                    .workflow()
                    .phases()
                    .iter()
                    .map(|phase| phase.as_str().to_owned())
                    .collect(),
                initial_phase: unit.workflow().initial_phase().as_str().to_owned(),
                edges: unit
                    .workflow()
                    .edges()
                    .iter()
                    .map(|edge| StoredWorkflowEdgeV2 {
                        from: edge.from().as_str().to_owned(),
                        to: edge.to().as_str().to_owned(),
                    })
                    .collect(),
                completion_phases: unit
                    .workflow()
                    .completion_phases()
                    .iter()
                    .map(|phase| phase.as_str().to_owned())
                    .collect(),
            },
            phase: unit.phase().as_str().to_owned(),
            status: match unit.status() {
                IntentUnitStatus::Active => StoredStatusV2::Active,
                IntentUnitStatus::Completed => StoredStatusV2::Completed,
            },
            revision: U64Text(unit.revision().value()),
            history: unit
                .history()
                .iter()
                .map(|record| match record {
                    LifecycleRecord::Transition(record) => StoredLifecycleRecordV2::Transition {
                        sequence: U64Text(
                            u64::try_from(record.sequence())
                                .expect("bounded lifecycle sequence fits u64"),
                        ),
                        from: record.from().as_str().to_owned(),
                        to: record.to().as_str().to_owned(),
                    },
                    LifecycleRecord::Completion(record) => StoredLifecycleRecordV2::Completion {
                        sequence: U64Text(
                            u64::try_from(record.sequence())
                                .expect("bounded lifecycle sequence fits u64"),
                        ),
                        phase: record.final_phase().as_str().to_owned(),
                    },
                })
                .collect(),
        }
    }

    fn replay(self) -> Result<IntentUnit, BackendError> {
        if self.representation_version != ENVELOPE_VERSION
            || self.history.len() > MAX_LIFECYCLE_RECORDS
        {
            return Err(BackendError::CorruptEnvelope);
        }

        let id = IntentUnitId::from_str(&self.id).map_err(|_| BackendError::CorruptEnvelope)?;
        if id.to_string() != self.id {
            return Err(BackendError::CorruptEnvelope);
        }
        let origin = ExternalReference::new(
            ReferenceNamespace::from_bytes(self.origin.namespace.as_bytes())
                .map_err(|_| BackendError::CorruptEnvelope)?,
            ReferenceText::from_bytes(self.origin.scope.as_bytes())
                .map_err(|_| BackendError::CorruptEnvelope)?,
            ReferenceText::from_bytes(self.origin.value.as_bytes())
                .map_err(|_| BackendError::CorruptEnvelope)?,
        );
        let species = IntentSpecies::from_bytes(self.species.as_bytes())
            .map_err(|_| BackendError::CorruptEnvelope)?;
        let workflow_id = WorkflowId::from_bytes(self.workflow.id.as_bytes())
            .map_err(|_| BackendError::CorruptEnvelope)?;
        let phases = self
            .workflow
            .phases
            .iter()
            .map(|phase| {
                PhaseId::from_bytes(phase.as_bytes()).map_err(|_| BackendError::CorruptEnvelope)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let initial_phase = PhaseId::from_bytes(self.workflow.initial_phase.as_bytes())
            .map_err(|_| BackendError::CorruptEnvelope)?;
        let edges = self
            .workflow
            .edges
            .iter()
            .map(|edge| {
                Ok(WorkflowEdge::new(
                    PhaseId::from_bytes(edge.from.as_bytes())
                        .map_err(|_| BackendError::CorruptEnvelope)?,
                    PhaseId::from_bytes(edge.to.as_bytes())
                        .map_err(|_| BackendError::CorruptEnvelope)?,
                ))
            })
            .collect::<Result<Vec<_>, BackendError>>()?;
        let completion_phases = self
            .workflow
            .completion_phases
            .iter()
            .map(|phase| {
                PhaseId::from_bytes(phase.as_bytes()).map_err(|_| BackendError::CorruptEnvelope)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let workflow =
            Workflow::new_bounded(workflow_id, phases, initial_phase, edges, completion_phases)
                .map_err(|_| BackendError::CorruptEnvelope)?;

        let mut unit = IntentUnit::new(id, origin, species, workflow);
        for (offset, record) in self.history.iter().enumerate() {
            let expected_sequence =
                u64::try_from(offset + 1).expect("bounded lifecycle sequence fits unsigned text");
            match record {
                StoredLifecycleRecordV2::Transition { sequence, from, to } => {
                    if sequence.0 != expected_sequence || unit.phase().as_str() != from {
                        return Err(BackendError::CorruptEnvelope);
                    }
                    let target = PhaseId::from_bytes(to.as_bytes())
                        .map_err(|_| BackendError::CorruptEnvelope)?;
                    unit.transition_to(&target)
                        .map_err(|_| BackendError::CorruptEnvelope)?;
                }
                StoredLifecycleRecordV2::Completion { sequence, phase } => {
                    if sequence.0 != expected_sequence || unit.phase().as_str() != phase {
                        return Err(BackendError::CorruptEnvelope);
                    }
                    unit.complete().map_err(|_| BackendError::CorruptEnvelope)?;
                }
            }
        }

        let expected_status = match self.status {
            StoredStatusV2::Active => IntentUnitStatus::Active,
            StoredStatusV2::Completed => IntentUnitStatus::Completed,
        };
        if unit.phase().as_str() != self.phase
            || unit.status() != expected_status
            || unit.revision().value() != self.revision.0
            || unit.history().len() != self.history.len()
        {
            return Err(BackendError::CorruptEnvelope);
        }
        Ok(unit)
    }
}

/// Serializes a current unit as the closed canonical envelope-v2 representation.
pub(crate) fn encode_envelope(unit: &IntentUnit) -> Result<String, BackendError> {
    validate_envelope_source(unit)?;
    let envelope = StoredEnvelopeV2::from_unit(unit);
    let bytes = serde_json::to_vec(&envelope).map_err(|_| BackendError::CorruptEnvelope)?;
    if bytes.is_empty() || bytes.len() > MAX_ENVELOPE_BYTES || bytes.contains(&b'\n') {
        return Err(BackendError::CorruptEnvelope);
    }
    String::from_utf8(bytes).map_err(|_| BackendError::CorruptEnvelope)
}

fn validate_envelope_source(unit: &IntentUnit) -> Result<(), BackendError> {
    if unit.history().len() > MAX_LIFECYCLE_RECORDS
        || unit.workflow().phases().len() > MAX_WORKFLOW_PHASES
        || unit.workflow().edges().len() > MAX_WORKFLOW_EDGES
        || unit.workflow().completion_phases().len() > MAX_COMPLETION_PHASES
        || IntentSpecies::from_bytes(unit.species().as_str().as_bytes()).is_err()
        || WorkflowId::from_bytes(unit.workflow_id().as_str().as_bytes()).is_err()
        || PhaseId::from_bytes(unit.phase().as_str().as_bytes()).is_err()
        || PhaseId::from_bytes(unit.workflow().initial_phase().as_str().as_bytes()).is_err()
        || unit
            .workflow()
            .phases()
            .iter()
            .any(|phase| PhaseId::from_bytes(phase.as_str().as_bytes()).is_err())
        || unit.workflow().edges().iter().any(|edge| {
            PhaseId::from_bytes(edge.from().as_str().as_bytes()).is_err()
                || PhaseId::from_bytes(edge.to().as_str().as_bytes()).is_err()
        })
        || unit
            .workflow()
            .completion_phases()
            .iter()
            .any(|phase| PhaseId::from_bytes(phase.as_str().as_bytes()).is_err())
    {
        return Err(BackendError::CorruptEnvelope);
    }
    Ok(())
}

/// Parses only canonical envelope-v2 bytes and restores state by checked core replay.
pub(crate) fn decode_envelope(bytes: &[u8]) -> Result<IntentUnit, BackendError> {
    if bytes.is_empty() || bytes.len() > MAX_ENVELOPE_BYTES {
        return Err(BackendError::CorruptEnvelope);
    }
    let version = serde_json::from_slice::<VersionProbe>(bytes)
        .map_err(|_| BackendError::CorruptEnvelope)?
        .representation_version;
    if version != ENVELOPE_VERSION {
        return Err(BackendError::UnsupportedEnvelopeVersion { found: version });
    }
    let envelope = serde_json::from_slice::<StoredEnvelopeV2>(bytes)
        .map_err(|_| BackendError::CorruptEnvelope)?;
    let canonical = serde_json::to_vec(&envelope).map_err(|_| BackendError::CorruptEnvelope)?;
    if canonical != bytes {
        return Err(BackendError::CorruptEnvelope);
    }
    envelope.replay()
}

pub(crate) fn decode_versioned_envelope(
    bytes: &[u8],
    row_envelope_version: i64,
) -> Result<IntentUnit, BackendError> {
    let row_envelope_version =
        u64::try_from(row_envelope_version).map_err(|_| BackendError::ProjectionMismatch)?;
    if row_envelope_version != ENVELOPE_VERSION {
        return Err(BackendError::UnsupportedEnvelopeVersion {
            found: row_envelope_version,
        });
    }
    decode_envelope(bytes)
}

/// Scalar columns selected beside an envelope-v2 value.
pub(crate) struct UnitProjection<'a> {
    pub(crate) id: &'a str,
    pub(crate) envelope_version: i64,
    pub(crate) origin_namespace: &'a str,
    pub(crate) origin_scope: &'a str,
    pub(crate) origin_value: &'a str,
    pub(crate) workflow_id: &'a str,
    pub(crate) species: &'a str,
    pub(crate) phase: &'a str,
    pub(crate) status: &'a str,
    pub(crate) revision: &'a [u8],
    pub(crate) last_global_sequence: &'a [u8],
    pub(crate) accepted_global_sequence: u64,
}

/// Replays an envelope and proves every selected SQL scalar and event coordinate agrees.
pub(crate) fn decode_projected_envelope(
    bytes: &[u8],
    projection: &UnitProjection<'_>,
) -> Result<IntentUnit, BackendError> {
    let unit = decode_versioned_envelope(bytes, projection.envelope_version)?;
    let revision = decode_revision_blob(projection.revision)?;
    let global_sequence = decode_u64_blob(projection.last_global_sequence)
        .map_err(|_| BackendError::ProjectionMismatch)?;
    let expected_status = match unit.status() {
        IntentUnitStatus::Active => "active",
        IntentUnitStatus::Completed => "completed",
    };
    if projection.id != unit.id().to_string()
        || projection.origin_namespace != unit.origin().namespace().as_str()
        || projection.origin_scope != unit.origin().scope().as_str()
        || projection.origin_value != unit.origin().value().as_str()
        || projection.workflow_id != unit.workflow_id().as_str()
        || projection.species != unit.species().as_str()
        || projection.phase != unit.phase().as_str()
        || projection.status != expected_status
        || revision != unit.revision()
        || global_sequence != projection.accepted_global_sequence
    {
        return Err(BackendError::ProjectionMismatch);
    }
    Ok(unit)
}

/// Keeps the checked row/envelope bridge reachable until the T-1110 projector
/// becomes the production caller.
pub(crate) fn retain_stored_projection_symbols() {
    let _ = decode_versioned_envelope;
    let _ = decode_projected_envelope;
    let _ = std::mem::size_of::<UnitProjection<'static>>();
}

pub(crate) fn encode_revision_text(revision: IntentUnitRevision) -> String {
    revision.value().to_string()
}

pub(crate) fn decode_revision_text(value: &str) -> Result<IntentUnitRevision, BackendError> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| BackendError::CorruptEnvelope)?;
    if parsed.to_string() != value {
        return Err(BackendError::CorruptEnvelope);
    }
    Ok(IntentUnitRevision::new(parsed))
}

pub(crate) const fn encode_revision_blob(revision: IntentUnitRevision) -> [u8; 8] {
    encode_u64_blob(revision.value())
}

pub(crate) fn decode_revision_blob(bytes: &[u8]) -> Result<IntentUnitRevision, BackendError> {
    decode_u64_blob(bytes)
        .map(IntentUnitRevision::new)
        .map_err(|_| BackendError::ProjectionMismatch)
}

pub(crate) const fn encode_u64_blob(value: u64) -> [u8; 8] {
    value.to_be_bytes()
}

pub(crate) fn decode_u64_blob(bytes: &[u8]) -> Result<u64, BackendError> {
    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| BackendError::ProjectionMismatch)?;
    Ok(u64::from_be_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use std::{fmt::Write as _, fs, path::PathBuf};

    use sha2::{Digest, Sha256};

    use super::*;

    const LEGACY_ENVELOPE: &[u8] = br#"{"representation_version":1,"id":"67e55044-10b1-426f-9247-bb680e5fe0c8","species":"feature","workflow":{"id":"delivery","phases":["queued"],"initial_phase":"queued","edges":[],"completion_phases":[]},"phase":"queued","status":"active","revision":"0","history":[]}"#;

    fn unit() -> IntentUnit {
        let queued = PhaseId::new("queued").expect("phase");
        IntentUnit::new(
            IntentUnitId::from_str("67e55044-10b1-426f-9247-bb680e5fe0c8").expect("id"),
            ExternalReference::new(
                ReferenceNamespace::new("book.intent").expect("namespace"),
                ReferenceText::new("scope\"; PRAGMA").expect("scope"),
                ReferenceText::new("value/* ATTACH */").expect("value"),
            ),
            IntentSpecies::new("feature").expect("species"),
            Workflow::new(
                WorkflowId::new("delivery").expect("workflow"),
                [queued.clone()],
                queued,
                [],
                [],
            )
            .expect("topology"),
        )
    }

    #[derive(Deserialize)]
    struct FixtureManifest {
        fixture_format: String,
        inventory_counts: FixtureInventory,
        formula_evidence: FixtureFile,
        cases: Vec<ManifestCase>,
    }

    #[derive(Deserialize)]
    struct FixtureInventory {
        case_files: usize,
        accept: usize,
        reject: usize,
        formula_files: usize,
    }

    #[derive(Deserialize)]
    struct FixtureFile {
        file: String,
        file_bytes: usize,
        file_sha256: String,
    }

    #[derive(Deserialize)]
    struct ManifestCase {
        case_id: String,
        file: String,
        file_bytes: usize,
        file_sha256: String,
        payload_bytes: usize,
        payload_sha256: String,
        sqlite_envelope_version: i64,
        expectation: FixtureExpectation,
    }

    #[derive(Deserialize)]
    struct FixtureCase {
        fixture_format: String,
        case_id: String,
        payload_encoding: String,
        payload_utf8_segments: Vec<String>,
        payload_bytes: usize,
        payload_sha256: String,
        sqlite_envelope_version: i64,
        expectation: FixtureExpectation,
    }

    #[derive(Deserialize)]
    struct FixtureExpectation {
        result: String,
    }

    fn fixture_directory() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/envelope-v2")
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        let digest = Sha256::digest(bytes);
        let mut text = String::with_capacity(64);
        for byte in digest {
            write!(&mut text, "{byte:02x}").expect("writing to String cannot fail");
        }
        text
    }

    #[test]
    fn envelope_v2_round_trips_canonical_bytes() {
        let unit = unit();
        let encoded = encode_envelope(&unit).expect("encode");
        assert!(!encoded.contains('\n'));
        assert_eq!(decode_envelope(encoded.as_bytes()), Ok(unit));
    }

    #[test]
    fn envelope_v1_is_rejected_without_rewriting_input() {
        let bytes = LEGACY_ENVELOPE.to_vec();
        let before = bytes.clone();
        assert_eq!(
            decode_envelope(&bytes),
            Err(BackendError::UnsupportedEnvelopeVersion { found: 1 })
        );
        assert_eq!(bytes, before);
    }

    #[test]
    fn revision_codecs_cover_complete_unsigned_range() {
        for value in [0, i64::MAX as u64 + 1, u64::MAX] {
            let revision = IntentUnitRevision::new(value);
            let text = encode_revision_text(revision);
            assert_eq!(decode_revision_text(&text), Ok(revision));
            let blob = encode_revision_blob(revision);
            assert_eq!(decode_revision_blob(&blob), Ok(revision));
        }
    }

    #[test]
    fn test_envelope_replay_bounds_and_generation_rejection_are_exact() {
        let directory = fixture_directory();
        let manifest_bytes =
            fs::read(directory.join("manifest-v1.json")).expect("read locked manifest");
        let manifest: FixtureManifest =
            serde_json::from_slice(&manifest_bytes).expect("parse locked manifest");
        assert_eq!(manifest.fixture_format, "cubikan-envelope-v2-manifest-v1");
        assert_eq!(manifest.inventory_counts.case_files, 18);
        assert_eq!(manifest.inventory_counts.accept, 4);
        assert_eq!(manifest.inventory_counts.reject, 14);
        assert_eq!(manifest.inventory_counts.formula_files, 1);
        assert_eq!(manifest.cases.len(), 18);

        let formula_bytes = fs::read(directory.join(&manifest.formula_evidence.file))
            .expect("read locked formula evidence");
        assert_eq!(formula_bytes.len(), manifest.formula_evidence.file_bytes);
        assert_eq!(
            sha256_hex(&formula_bytes),
            manifest.formula_evidence.file_sha256
        );

        let mut accepted = 0;
        let mut rejected = 0;
        for manifest_case in manifest.cases {
            let wrapper_bytes =
                fs::read(directory.join(&manifest_case.file)).expect("read locked case wrapper");
            assert_eq!(wrapper_bytes.len(), manifest_case.file_bytes);
            assert_eq!(sha256_hex(&wrapper_bytes), manifest_case.file_sha256);
            let wrapper: FixtureCase =
                serde_json::from_slice(&wrapper_bytes).expect("parse locked case wrapper");
            assert_eq!(wrapper.fixture_format, "cubikan-envelope-v2-case-v1");
            assert_eq!(
                wrapper.payload_encoding,
                "utf8-concatenate-json-string-segments"
            );
            assert_eq!(wrapper.case_id, manifest_case.case_id);
            assert_eq!(wrapper.payload_bytes, manifest_case.payload_bytes);
            assert_eq!(wrapper.payload_sha256, manifest_case.payload_sha256);
            assert_eq!(
                wrapper.sqlite_envelope_version,
                manifest_case.sqlite_envelope_version
            );
            assert_eq!(wrapper.expectation.result, manifest_case.expectation.result);

            let payload = wrapper.payload_utf8_segments.concat().into_bytes();
            assert_eq!(payload.len(), manifest_case.payload_bytes);
            assert_eq!(sha256_hex(&payload), manifest_case.payload_sha256);

            match manifest_case.expectation.result.as_str() {
                "accept" => {
                    accepted += 1;
                    assert_eq!(manifest_case.sqlite_envelope_version, 2);
                    assert!(!payload.ends_with(b"\n"));
                    let unit =
                        decode_versioned_envelope(&payload, manifest_case.sqlite_envelope_version)
                            .expect("accepted fixture must replay");
                    assert_eq!(
                        encode_envelope(&unit)
                            .expect("re-encode accepted fixture")
                            .as_bytes(),
                        payload
                    );

                    let id = unit.id().to_string();
                    let revision = encode_revision_blob(unit.revision());
                    let accepted_global_sequence = unit.revision().value() + 1;
                    let last_global_sequence = encode_u64_blob(accepted_global_sequence);
                    let projection = UnitProjection {
                        id: &id,
                        envelope_version: 2,
                        origin_namespace: unit.origin().namespace().as_str(),
                        origin_scope: unit.origin().scope().as_str(),
                        origin_value: unit.origin().value().as_str(),
                        workflow_id: unit.workflow_id().as_str(),
                        species: unit.species().as_str(),
                        phase: unit.phase().as_str(),
                        status: match unit.status() {
                            IntentUnitStatus::Active => "active",
                            IntentUnitStatus::Completed => "completed",
                        },
                        revision: &revision,
                        last_global_sequence: &last_global_sequence,
                        accepted_global_sequence,
                    };
                    assert_eq!(
                        decode_projected_envelope(&payload, &projection),
                        Ok(unit.clone())
                    );

                    if manifest_case.case_id == "valid-maximal-256" {
                        assert_eq!(payload.len(), 1_304_742);
                        assert_eq!(MAX_ENVELOPE_BYTES - payload.len(), 792_410);
                        assert_eq!(unit.history().len(), MAX_LIFECYCLE_RECORDS);
                        assert_eq!(unit.workflow().phases().len(), MAX_WORKFLOW_PHASES);
                        assert_eq!(unit.workflow().edges().len(), MAX_WORKFLOW_EDGES);
                        assert_eq!(
                            unit.workflow().completion_phases().len(),
                            MAX_COMPLETION_PHASES
                        );
                    }
                }
                "reject" => {
                    rejected += 1;
                    let result =
                        decode_versioned_envelope(&payload, manifest_case.sqlite_envelope_version);
                    assert!(
                        result.is_err(),
                        "rejection fixture {} was accepted",
                        manifest_case.case_id
                    );
                    if manifest_case.case_id == "reject-over-ceiling" {
                        assert_eq!(payload.len(), MAX_ENVELOPE_BYTES + 1);
                        assert_eq!(result, Err(BackendError::CorruptEnvelope));
                    }
                    if manifest_case.case_id == "reject-lifecycle-257" {
                        assert_eq!(result, Err(BackendError::CorruptEnvelope));
                    }
                }
                other => panic!("unknown fixture expectation {other}"),
            }
        }
        assert_eq!((accepted, rejected), (4, 14));
    }
}
