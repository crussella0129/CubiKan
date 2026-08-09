use cubikan_core::{
    IntentSpecies, IntentUnit, IntentUnitId, IntentUnitRevision, IntentUnitStatus, LifecycleRecord,
    PhaseId, Workflow, WorkflowEdge, WorkflowId,
};
use serde::{Deserialize, Serialize};

use crate::BackendError;

pub(crate) const ENVELOPE_VERSION: u64 = 1;

#[derive(Deserialize)]
struct RepresentationVersionProbe {
    representation_version: u64,
}

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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum StoredStatusV1 {
    Active,
    Completed,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredWorkflowV1 {
    id: String,
    phases: Vec<String>,
    initial_phase: String,
    edges: Vec<StoredWorkflowEdgeV1>,
    completion_phases: Vec<String>,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredWorkflowEdgeV1 {
    from: String,
    to: String,
}

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

/// Encodes a complete core aggregate into the adapter-owned envelope v1.
pub(crate) fn encode_envelope(unit: &IntentUnit) -> Result<String, BackendError> {
    let stored = StoredEnvelopeV1::from_intent_unit(unit)?;
    serde_json::to_string(&stored).map_err(|_| BackendError::CorruptEnvelope)
}

/// Decodes and replay-validates an adapter-owned envelope from its original bytes.
pub(crate) fn decode_envelope(bytes: &[u8]) -> Result<IntentUnit, BackendError> {
    // Probe the original input rather than normalizing through `serde_json::Value`.
    // This preserves duplicate-key and numeric-shape rejection in both passes.
    let probe: RepresentationVersionProbe =
        serde_json::from_slice(bytes).map_err(|_| BackendError::CorruptEnvelope)?;
    if probe.representation_version != ENVELOPE_VERSION {
        return Err(BackendError::UnsupportedEnvelopeVersion {
            found: probe.representation_version,
        });
    }

    let stored: StoredEnvelopeV1 =
        serde_json::from_slice(bytes).map_err(|_| BackendError::CorruptEnvelope)?;
    stored.replay()
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

impl StoredEnvelopeV1 {
    fn from_intent_unit(unit: &IntentUnit) -> Result<Self, BackendError> {
        let workflow = unit.workflow();
        let history = unit
            .history()
            .iter()
            .map(StoredLifecycleRecordV1::from_core)
            .collect::<Result<_, _>>()?;

        Ok(Self {
            representation_version: ENVELOPE_VERSION,
            id: unit.id().to_string(),
            species: unit.species().as_str().to_owned(),
            phase: unit.phase().as_str().to_owned(),
            revision: encode_revision_text(unit.revision()),
            status: StoredStatusV1::from_core(unit.status()),
            workflow: StoredWorkflowV1 {
                id: workflow.id().as_str().to_owned(),
                phases: workflow
                    .phases()
                    .iter()
                    .map(|phase| phase.as_str().to_owned())
                    .collect(),
                initial_phase: workflow.initial_phase().as_str().to_owned(),
                edges: workflow
                    .edges()
                    .iter()
                    .map(|edge| StoredWorkflowEdgeV1 {
                        from: edge.from().as_str().to_owned(),
                        to: edge.to().as_str().to_owned(),
                    })
                    .collect(),
                completion_phases: workflow
                    .completion_phases()
                    .iter()
                    .map(|phase| phase.as_str().to_owned())
                    .collect(),
            },
            history,
        })
    }

    fn replay(&self) -> Result<IntentUnit, BackendError> {
        let id = parse_canonical_id(&self.id)?;
        let species =
            IntentSpecies::new(self.species.clone()).map_err(|_| BackendError::CorruptEnvelope)?;
        let workflow = self.workflow.to_core()?;
        // Validate revision grammar before replay so malformed text is always
        // classified as corruption, including for a revision-zero aggregate.
        decode_revision_text(&self.revision)?;

        let mut unit = IntentUnit::new(id, species, workflow);
        for (index, record) in self.history.iter().enumerate() {
            let expected_sequence = index
                .checked_add(1)
                .and_then(|value| u64::try_from(value).ok())
                .ok_or(BackendError::CorruptEnvelope)?;
            record.replay(&mut unit, expected_sequence)?;
        }

        // Re-encoding the replayed aggregate compares every declared semantic
        // field and preserves the caller's workflow/history ordering.
        let replayed = Self::from_intent_unit(&unit)?;
        if replayed != *self {
            return Err(BackendError::CorruptEnvelope);
        }
        Ok(unit)
    }
}

impl StoredStatusV1 {
    const fn from_core(status: IntentUnitStatus) -> Self {
        match status {
            IntentUnitStatus::Active => Self::Active,
            IntentUnitStatus::Completed => Self::Completed,
        }
    }
}

impl StoredWorkflowV1 {
    fn to_core(&self) -> Result<Workflow, BackendError> {
        let id = WorkflowId::new(self.id.clone()).map_err(|_| BackendError::CorruptEnvelope)?;
        let phases = self
            .phases
            .iter()
            .map(|value| parse_phase(value))
            .collect::<Result<Vec<_>, _>>()?;
        let initial_phase = parse_phase(&self.initial_phase)?;
        let edges = self
            .edges
            .iter()
            .map(StoredWorkflowEdgeV1::to_core)
            .collect::<Result<Vec<_>, _>>()?;
        let completion_phases = self
            .completion_phases
            .iter()
            .map(|value| parse_phase(value))
            .collect::<Result<Vec<_>, _>>()?;

        Workflow::new(id, phases, initial_phase, edges, completion_phases)
            .map_err(|_| BackendError::CorruptEnvelope)
    }
}

impl StoredWorkflowEdgeV1 {
    fn to_core(&self) -> Result<WorkflowEdge, BackendError> {
        Ok(WorkflowEdge::new(
            parse_phase(&self.from)?,
            parse_phase(&self.to)?,
        ))
    }
}

impl StoredLifecycleRecordV1 {
    fn from_core(record: &LifecycleRecord) -> Result<Self, BackendError> {
        match record {
            LifecycleRecord::Transition(record) => Ok(Self::Transition {
                sequence: u64::try_from(record.sequence())
                    .map_err(|_| BackendError::CorruptEnvelope)?,
                from: record.from().as_str().to_owned(),
                to: record.to().as_str().to_owned(),
            }),
            LifecycleRecord::Completion(record) => Ok(Self::Completion {
                sequence: u64::try_from(record.sequence())
                    .map_err(|_| BackendError::CorruptEnvelope)?,
                phase: record.final_phase().as_str().to_owned(),
            }),
        }
    }

    fn replay(&self, unit: &mut IntentUnit, expected_sequence: u64) -> Result<(), BackendError> {
        match self {
            Self::Transition { sequence, from, to } => {
                if *sequence != expected_sequence {
                    return Err(BackendError::CorruptEnvelope);
                }
                let from = parse_phase(from)?;
                let to = parse_phase(to)?;
                if unit.phase() != &from {
                    return Err(BackendError::CorruptEnvelope);
                }
                unit.transition_to(&to)
                    .map_err(|_| BackendError::CorruptEnvelope)
            }
            Self::Completion { sequence, phase } => {
                if *sequence != expected_sequence {
                    return Err(BackendError::CorruptEnvelope);
                }
                let phase = parse_phase(phase)?;
                if unit.phase() != &phase {
                    return Err(BackendError::CorruptEnvelope);
                }
                unit.complete().map_err(|_| BackendError::CorruptEnvelope)
            }
        }
    }
}

fn parse_canonical_id(value: &str) -> Result<IntentUnitId, BackendError> {
    let id = value
        .parse::<IntentUnitId>()
        .map_err(|_| BackendError::CorruptEnvelope)?;
    if id.to_string() != value {
        return Err(BackendError::CorruptEnvelope);
    }
    Ok(id)
}

fn parse_phase(value: &str) -> Result<PhaseId, BackendError> {
    PhaseId::new(value.to_owned()).map_err(|_| BackendError::CorruptEnvelope)
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    fn phase(value: &str) -> PhaseId {
        PhaseId::new(value).expect("fixture phase should be valid")
    }

    fn fixture_workflow() -> Workflow {
        let queued = phase("queued");
        let doing = phase("doing");
        let done = phase("done");
        Workflow::new(
            WorkflowId::new("custom-delivery").expect("fixture workflow ID should be valid"),
            vec![doing.clone(), queued.clone(), done.clone()],
            queued.clone(),
            vec![
                WorkflowEdge::new(queued.clone(), doing.clone()),
                WorkflowEdge::new(doing.clone(), doing.clone()),
                WorkflowEdge::new(doing.clone(), queued.clone()),
                WorkflowEdge::new(doing.clone(), done.clone()),
            ],
            vec![done],
        )
        .expect("fixture workflow should be valid")
    }

    fn fixture_id() -> IntentUnitId {
        "67e55044-10b1-426f-9247-bb680e5fe0c8"
            .parse()
            .expect("fixture ID should be valid")
    }

    fn new_unit() -> IntentUnit {
        IntentUnit::new(
            fixture_id(),
            IntentSpecies::new("feature").expect("fixture species should be valid"),
            fixture_workflow(),
        )
    }

    fn active_unit_with_history() -> IntentUnit {
        let mut unit = new_unit();
        unit.transition_to(&phase("doing"))
            .expect("queued -> doing should succeed");
        unit.transition_to(&phase("doing"))
            .expect("doing -> doing should succeed");
        unit.transition_to(&phase("queued"))
            .expect("doing -> queued should succeed");
        unit
    }

    fn completed_unit() -> IntentUnit {
        let mut unit = active_unit_with_history();
        unit.transition_to(&phase("doing"))
            .expect("queued -> doing should succeed");
        unit.transition_to(&phase("done"))
            .expect("doing -> done should succeed");
        unit.complete().expect("done should be completion eligible");
        unit
    }

    fn encoded_value(unit: &IntentUnit) -> Value {
        serde_json::from_str(&encode_envelope(unit).expect("fixture should encode"))
            .expect("encoded fixture should be JSON")
    }

    fn assert_corrupt(value: &Value) {
        let bytes = serde_json::to_vec(value).expect("fixture should serialize");
        assert_eq!(decode_envelope(&bytes), Err(BackendError::CorruptEnvelope));
    }

    #[test]
    fn test_envelope_v1_round_trips_active_and_completed_units() {
        for unit in [active_unit_with_history(), completed_unit()] {
            let encoded = encode_envelope(&unit).expect("valid unit should encode");
            let decoded =
                decode_envelope(encoded.as_bytes()).expect("valid envelope should decode");
            assert_eq!(decoded, unit);

            let stored: Value = serde_json::from_str(&encoded).expect("envelope should be JSON");
            let direct = serde_json::to_value(&unit).expect("core unit should serialize");
            assert_eq!(stored["representation_version"], json!(1));
            assert!(stored["revision"].is_string());
            assert_ne!(stored, direct, "core Serde must not be storage authority");
        }
    }

    #[test]
    fn test_envelope_v1_rejects_malformed_or_unreplayable_lifecycle() {
        let active = encoded_value(&active_unit_with_history());
        let completed = encoded_value(&completed_unit());
        let mut cases = Vec::new();

        let mut sequence_zero = active.clone();
        sequence_zero["history"][0]["sequence"] = json!(0);
        cases.push(sequence_zero);

        let mut sequence_gap = active.clone();
        sequence_gap["history"][1]["sequence"] = json!(3);
        cases.push(sequence_gap);

        let mut duplicate_sequence = active.clone();
        duplicate_sequence["history"][1]["sequence"] = json!(1);
        cases.push(duplicate_sequence);

        let mut wrong_source = active.clone();
        wrong_source["history"][0]["from"] = json!("doing");
        cases.push(wrong_source);

        let mut undeclared_edge = active.clone();
        undeclared_edge["history"][0]["to"] = json!("done");
        cases.push(undeclared_edge);

        let mut ineligible_completion = active.clone();
        ineligible_completion["history"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "type": "completion",
                "sequence": 4,
                "phase": "queued"
            }));
        cases.push(ineligible_completion);

        let mut wrong_completion_phase = completed.clone();
        let last = wrong_completion_phase["history"]
            .as_array_mut()
            .unwrap()
            .last_mut()
            .unwrap();
        last["phase"] = json!("doing");
        cases.push(wrong_completion_phase);

        let mut record_after_completion = completed.clone();
        record_after_completion["history"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "type": "transition",
                "sequence": 7,
                "from": "done",
                "to": "done"
            }));
        cases.push(record_after_completion);

        for (field, replacement) in [
            ("phase", json!("doing")),
            ("status", json!("completed")),
            ("revision", json!("2")),
        ] {
            let mut mismatch = active.clone();
            mismatch[field] = replacement;
            cases.push(mismatch);
        }

        for case in cases {
            assert_corrupt(&case);
        }
    }

    #[test]
    fn test_envelope_v1_rejects_unknown_missing_invalid_and_unsupported_state() {
        let valid = encoded_value(&active_unit_with_history());

        for field in [
            "representation_version",
            "id",
            "species",
            "phase",
            "revision",
            "status",
            "workflow",
            "history",
        ] {
            let mut missing = valid.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert_corrupt(&missing);
        }

        for field in [
            "id",
            "phases",
            "initial_phase",
            "edges",
            "completion_phases",
        ] {
            let mut missing = valid.clone();
            missing["workflow"].as_object_mut().unwrap().remove(field);
            assert_corrupt(&missing);
        }

        for field in ["from", "to"] {
            let mut missing = valid.clone();
            missing["workflow"]["edges"][0]
                .as_object_mut()
                .unwrap()
                .remove(field);
            assert_corrupt(&missing);
        }

        for field in ["type", "sequence", "from", "to"] {
            let mut missing = valid.clone();
            missing["history"][0].as_object_mut().unwrap().remove(field);
            assert_corrupt(&missing);
        }

        let completed = encoded_value(&completed_unit());
        let completion_index = completed["history"]
            .as_array()
            .unwrap()
            .len()
            .checked_sub(1)
            .unwrap();
        for field in ["type", "sequence", "phase"] {
            let mut missing = completed.clone();
            missing["history"][completion_index]
                .as_object_mut()
                .unwrap()
                .remove(field);
            assert_corrupt(&missing);
        }

        let mut unknown_top = valid.clone();
        unknown_top["unknown"] = json!(true);
        assert_corrupt(&unknown_top);
        let mut unknown_workflow = valid.clone();
        unknown_workflow["workflow"]["unknown"] = json!(true);
        assert_corrupt(&unknown_workflow);
        let mut unknown_edge = valid.clone();
        unknown_edge["workflow"]["edges"][0]["unknown"] = json!(true);
        assert_corrupt(&unknown_edge);
        let mut unknown_record = valid.clone();
        unknown_record["history"][0]["unknown"] = json!(true);
        assert_corrupt(&unknown_record);
        let mut unknown_completion = completed.clone();
        unknown_completion["history"][completion_index]["unknown"] = json!(true);
        assert_corrupt(&unknown_completion);

        let mut invalid_cases = Vec::new();
        let mut noncanonical_id = valid.clone();
        noncanonical_id["id"] = json!(fixture_id().to_string().to_uppercase());
        invalid_cases.push(noncanonical_id);
        for path in ["species", "phase"] {
            let mut blank = valid.clone();
            blank[path] = json!("   ");
            invalid_cases.push(blank);
        }
        let mut blank_workflow_id = valid.clone();
        blank_workflow_id["workflow"]["id"] = json!("");
        invalid_cases.push(blank_workflow_id);
        let mut blank_declared_phase = valid.clone();
        blank_declared_phase["workflow"]["phases"][0] = json!("   ");
        invalid_cases.push(blank_declared_phase);
        let mut empty_phases = valid.clone();
        empty_phases["workflow"]["phases"] = json!([]);
        invalid_cases.push(empty_phases);
        let mut duplicate_phase = valid.clone();
        duplicate_phase["workflow"]["phases"] = json!(["queued", "queued", "done"]);
        invalid_cases.push(duplicate_phase);
        let mut unknown_initial = valid.clone();
        unknown_initial["workflow"]["initial_phase"] = json!("absent");
        invalid_cases.push(unknown_initial);
        let mut unknown_edge_source = valid.clone();
        unknown_edge_source["workflow"]["edges"][0]["from"] = json!("absent");
        invalid_cases.push(unknown_edge_source);
        let mut unknown_edge_target = valid.clone();
        unknown_edge_target["workflow"]["edges"][0]["to"] = json!("absent");
        invalid_cases.push(unknown_edge_target);
        let mut duplicate_edge = valid.clone();
        let edge = duplicate_edge["workflow"]["edges"][0].clone();
        duplicate_edge["workflow"]["edges"]
            .as_array_mut()
            .unwrap()
            .push(edge);
        invalid_cases.push(duplicate_edge);
        let mut unknown_completion = valid.clone();
        unknown_completion["workflow"]["completion_phases"] = json!(["absent"]);
        invalid_cases.push(unknown_completion);
        let mut duplicate_completion = valid.clone();
        duplicate_completion["workflow"]["completion_phases"] = json!(["done", "done"]);
        invalid_cases.push(duplicate_completion);
        let mut invalid_status = valid.clone();
        invalid_status["status"] = json!("terminal");
        invalid_cases.push(invalid_status);
        let mut numeric_revision = valid.clone();
        numeric_revision["revision"] = json!(3);
        invalid_cases.push(numeric_revision);
        for invalid in invalid_cases {
            assert_corrupt(&invalid);
        }

        for version in [0, 2] {
            let mut unsupported = valid.clone();
            unsupported["representation_version"] = json!(version);
            let bytes = serde_json::to_vec(&unsupported).unwrap();
            assert_eq!(
                decode_envelope(&bytes),
                Err(BackendError::UnsupportedEnvelopeVersion { found: version })
            );
        }

        let encoded = encode_envelope(&active_unit_with_history()).unwrap();
        let duplicate_cases = [
            encoded.replacen("\"id\":", "\"id\":\"other\",\"id\":", 1),
            encoded.replacen(
                "\"representation_version\":1",
                "\"representation_version\":1,\"representation_version\":1",
                1,
            ),
            encoded.replacen(
                "\"workflow\":{\"id\":",
                "\"workflow\":{\"id\":\"other\",\"id\":",
                1,
            ),
            encoded.replacen(
                "\"edges\":[{\"from\":",
                "\"edges\":[{\"from\":\"other\",\"from\":",
                1,
            ),
            encoded.replacen("\"sequence\":1", "\"sequence\":1,\"sequence\":1", 1),
        ];
        for duplicate in duplicate_cases {
            assert_eq!(
                decode_envelope(duplicate.as_bytes()),
                Err(BackendError::CorruptEnvelope)
            );
        }
    }

    #[test]
    fn test_revision_codecs_preserve_full_u64_and_reject_aliases() {
        for value in [0, i64::MAX as u64 + 1, u64::MAX] {
            let revision = IntentUnitRevision::new(value);
            let text = encode_revision_text(revision);
            assert_eq!(text, value.to_string());
            assert_eq!(
                serde_json::to_string(&text).unwrap(),
                format!("\"{value}\"")
            );
            assert_eq!(decode_revision_text(&text).unwrap(), revision);

            let blob = encode_revision_blob(revision);
            assert_eq!(blob, value.to_be_bytes());
            assert_eq!(decode_revision_blob(&blob).unwrap(), revision);
        }

        for invalid in [
            "",
            "+1",
            "-1",
            " 1",
            "1 ",
            "00",
            "01",
            "1.0",
            "18446744073709551616",
            "\u{ff11}",
        ] {
            assert_eq!(
                decode_revision_text(invalid),
                Err(BackendError::CorruptEnvelope)
            );
        }

        for invalid in [vec![], vec![0; 7], vec![0; 9]] {
            assert_eq!(
                decode_revision_blob(&invalid),
                Err(BackendError::ProjectionMismatch)
            );
        }
    }
}
