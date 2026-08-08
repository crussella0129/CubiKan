use serde::{Deserialize, Serialize};

pub(crate) const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProtocolRequest {
    pub(crate) protocol_version: u32,
    pub(crate) workflow: WorkflowInput,
    pub(crate) intent_unit: IntentUnitInput,
    pub(crate) operations: Vec<OperationInput>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkflowInput {
    pub(crate) id: String,
    pub(crate) phases: Vec<String>,
    pub(crate) initial_phase: String,
    pub(crate) edges: Vec<EdgeInput>,
    pub(crate) completion_phases: Vec<String>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct EdgeInput {
    pub(crate) from: String,
    pub(crate) to: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct IntentUnitInput {
    pub(crate) id: Option<String>,
    pub(crate) species: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum OperationInput {
    Transition(TransitionInput),
    Complete(CompleteInput),
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct TransitionInput {
    pub(crate) target: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompleteInput {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub(crate) enum ProtocolResponse {
    Success {
        protocol_version: u32,
        intent_unit: IntentUnitSnapshot,
    },
    Error {
        protocol_version: u32,
        error: ErrorDetail,
        #[serde(skip_serializing_if = "Option::is_none")]
        intent_unit: Option<IntentUnitSnapshot>,
    },
}

impl ProtocolResponse {
    pub(crate) const fn success(intent_unit: IntentUnitSnapshot) -> Self {
        Self::Success {
            protocol_version: PROTOCOL_VERSION,
            intent_unit,
        }
    }

    pub(crate) const fn error(error: ErrorDetail, intent_unit: Option<IntentUnitSnapshot>) -> Self {
        Self::Error {
            protocol_version: PROTOCOL_VERSION,
            error,
            intent_unit,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ErrorDetail {
    pub(crate) code: ErrorCode,
    pub(crate) message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) operation_number: Option<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ErrorCode {
    InvalidJson,
    InvalidRequest,
    #[allow(dead_code, reason = "used by bounded request ingestion in T-202")]
    RequestTooLarge,
    UnsupportedProtocolVersion,
    BlankValue,
    InvalidIntentUnitId,
    WorkflowEmptyPhases,
    WorkflowDuplicatePhase,
    WorkflowUnknownInitialPhase,
    WorkflowUnknownEdgeSource,
    WorkflowUnknownEdgeTarget,
    WorkflowDuplicateEdge,
    WorkflowUnknownCompletionPhase,
    WorkflowDuplicateCompletionPhase,
    TransitionAlreadyCompleted,
    TransitionUnknownTarget,
    TransitionNotAllowed,
    CompletionAlreadyCompleted,
    CompletionPhaseNotEligible,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct IntentUnitSnapshot {
    pub(crate) id: String,
    pub(crate) species: String,
    pub(crate) workflow_id: String,
    pub(crate) phase: String,
    pub(crate) status: SnapshotStatus,
    pub(crate) history: Vec<HistoryEntry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SnapshotStatus {
    Active,
    Completed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum HistoryEntry {
    Transition {
        sequence: usize,
        from: String,
        to: String,
    },
    Completion {
        sequence: usize,
        phase: String,
    },
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn snapshot(status: SnapshotStatus) -> IntentUnitSnapshot {
        IntentUnitSnapshot {
            id: "67e55044-10b1-426f-9247-bb680e5fe0c8".to_owned(),
            species: "feature".to_owned(),
            workflow_id: "delivery".to_owned(),
            phase: "done".to_owned(),
            status,
            history: vec![
                HistoryEntry::Transition {
                    sequence: 1,
                    from: "doing".to_owned(),
                    to: "done".to_owned(),
                },
                HistoryEntry::Completion {
                    sequence: 2,
                    phase: "done".to_owned(),
                },
            ],
        }
    }

    #[test]
    fn test_protocol_decodes_complete_v1_scenario_strictly() {
        let request = json!({
            "protocol_version": 1,
            "workflow": {
                "id": "  delivery  ",
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
            "operations": [
                {"type": "transition", "target": "doing"},
                {"type": "complete"}
            ]
        });

        let decoded: ProtocolRequest =
            serde_json::from_value(request.clone()).expect("valid request should decode");

        assert_eq!(decoded.protocol_version, PROTOCOL_VERSION);
        assert_eq!(decoded.workflow.id, "  delivery  ");
        assert_eq!(decoded.workflow.phases, ["queued", "doing", "done"]);
        assert_eq!(decoded.workflow.initial_phase, "queued");
        assert_eq!(
            decoded.workflow.edges,
            [
                EdgeInput {
                    from: "queued".to_owned(),
                    to: "doing".to_owned(),
                },
                EdgeInput {
                    from: "doing".to_owned(),
                    to: "done".to_owned(),
                },
            ]
        );
        assert_eq!(decoded.workflow.completion_phases, ["done"]);
        assert_eq!(decoded.intent_unit.species, "feature");
        assert_eq!(
            decoded.intent_unit.id.as_deref(),
            Some("67e55044-10b1-426f-9247-bb680e5fe0c8")
        );
        assert_eq!(
            decoded.operations,
            [
                OperationInput::Transition(TransitionInput {
                    target: "doing".to_owned()
                }),
                OperationInput::Complete(CompleteInput {})
            ]
        );

        let mut unknown_root = request.clone();
        unknown_root["unexpected"] = json!(true);
        assert!(serde_json::from_value::<ProtocolRequest>(unknown_root).is_err());

        let mut unknown_workflow = request.clone();
        unknown_workflow["workflow"]["unexpected"] = json!(true);
        assert!(serde_json::from_value::<ProtocolRequest>(unknown_workflow).is_err());

        let mut unknown_edge = request.clone();
        unknown_edge["workflow"]["edges"][0]["unexpected"] = json!(true);
        assert!(serde_json::from_value::<ProtocolRequest>(unknown_edge).is_err());

        let mut unknown_intent_unit = request.clone();
        unknown_intent_unit["intent_unit"]["unexpected"] = json!(true);
        assert!(serde_json::from_value::<ProtocolRequest>(unknown_intent_unit).is_err());

        let mut unknown_transition = request.clone();
        unknown_transition["operations"][0]["unexpected"] = json!(true);
        assert!(serde_json::from_value::<ProtocolRequest>(unknown_transition).is_err());

        let mut unknown_complete = request;
        unknown_complete["operations"][1]["target"] = json!("done");
        assert!(serde_json::from_value::<ProtocolRequest>(unknown_complete).is_err());
    }

    #[test]
    fn test_protocol_serializes_versioned_adapter_envelopes() {
        let success = serde_json::to_value(ProtocolResponse::success(snapshot(
            SnapshotStatus::Completed,
        )))
        .expect("success should serialize");
        assert_eq!(
            success,
            json!({
                "outcome": "success",
                "protocol_version": 1,
                "intent_unit": {
                    "id": "67e55044-10b1-426f-9247-bb680e5fe0c8",
                    "species": "feature",
                    "workflow_id": "delivery",
                    "phase": "done",
                    "status": "completed",
                    "history": [
                        {
                            "type": "transition",
                            "sequence": 1,
                            "from": "doing",
                            "to": "done"
                        },
                        {
                            "type": "completion",
                            "sequence": 2,
                            "phase": "done"
                        }
                    ]
                }
            })
        );

        let setup_error = serde_json::to_value(ProtocolResponse::error(
            ErrorDetail {
                code: ErrorCode::BlankValue,
                message: "value must not be blank".to_owned(),
                field: Some("intent_unit.species".to_owned()),
                operation_number: None,
            },
            None,
        ))
        .expect("setup error should serialize");
        assert_eq!(
            setup_error,
            json!({
                "outcome": "error",
                "protocol_version": 1,
                "error": {
                    "code": "blank_value",
                    "message": "value must not be blank",
                    "field": "intent_unit.species"
                }
            })
        );

        let lifecycle_error = serde_json::to_value(ProtocolResponse::error(
            ErrorDetail {
                code: ErrorCode::TransitionNotAllowed,
                message: "transition is not declared".to_owned(),
                field: None,
                operation_number: Some(2),
            },
            Some(snapshot(SnapshotStatus::Active)),
        ))
        .expect("lifecycle error should serialize");
        assert_eq!(
            lifecycle_error,
            json!({
                "outcome": "error",
                "protocol_version": 1,
                "error": {
                    "code": "transition_not_allowed",
                    "message": "transition is not declared",
                    "operation_number": 2
                },
                "intent_unit": {
                    "id": "67e55044-10b1-426f-9247-bb680e5fe0c8",
                    "species": "feature",
                    "workflow_id": "delivery",
                    "phase": "done",
                    "status": "active",
                    "history": [
                        {
                            "type": "transition",
                            "sequence": 1,
                            "from": "doing",
                            "to": "done"
                        },
                        {
                            "type": "completion",
                            "sequence": 2,
                            "phase": "done"
                        }
                    ]
                }
            })
        );

        let codes = [
            ErrorCode::InvalidJson,
            ErrorCode::InvalidRequest,
            ErrorCode::RequestTooLarge,
            ErrorCode::UnsupportedProtocolVersion,
            ErrorCode::BlankValue,
            ErrorCode::InvalidIntentUnitId,
            ErrorCode::WorkflowEmptyPhases,
            ErrorCode::WorkflowDuplicatePhase,
            ErrorCode::WorkflowUnknownInitialPhase,
            ErrorCode::WorkflowUnknownEdgeSource,
            ErrorCode::WorkflowUnknownEdgeTarget,
            ErrorCode::WorkflowDuplicateEdge,
            ErrorCode::WorkflowUnknownCompletionPhase,
            ErrorCode::WorkflowDuplicateCompletionPhase,
            ErrorCode::TransitionAlreadyCompleted,
            ErrorCode::TransitionUnknownTarget,
            ErrorCode::TransitionNotAllowed,
            ErrorCode::CompletionAlreadyCompleted,
            ErrorCode::CompletionPhaseNotEligible,
        ];
        let expected_codes = [
            "invalid_json",
            "invalid_request",
            "request_too_large",
            "unsupported_protocol_version",
            "blank_value",
            "invalid_intent_unit_id",
            "workflow_empty_phases",
            "workflow_duplicate_phase",
            "workflow_unknown_initial_phase",
            "workflow_unknown_edge_source",
            "workflow_unknown_edge_target",
            "workflow_duplicate_edge",
            "workflow_unknown_completion_phase",
            "workflow_duplicate_completion_phase",
            "transition_already_completed",
            "transition_unknown_target",
            "transition_not_allowed",
            "completion_already_completed",
            "completion_phase_not_eligible",
        ];
        for (code, expected) in codes.into_iter().zip(expected_codes) {
            assert_eq!(
                serde_json::to_value(code).expect("error code should serialize"),
                expected
            );
        }

        for response in [success, setup_error, lifecycle_error] {
            let object = response.as_object().expect("response should be an object");
            let outcome_count = ["success", "error"]
                .iter()
                .filter(|outcome| object["outcome"].as_str() == Some(**outcome))
                .count();
            assert_eq!(outcome_count, 1);
        }
    }

    #[test]
    fn test_protocol_serializes_request_too_large_error() {
        let response = serde_json::to_value(ProtocolResponse::error(
            ErrorDetail {
                code: ErrorCode::RequestTooLarge,
                message: "request exceeds maximum size of 1048576 bytes".to_owned(),
                field: None,
                operation_number: None,
            },
            None,
        ))
        .expect("oversized-request rejection should serialize");

        assert_eq!(
            response,
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
}
