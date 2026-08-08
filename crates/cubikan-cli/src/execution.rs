use std::str::FromStr;

use cubikan_core::{
    CompletionError, IntentSpecies, IntentUnit, IntentUnitId, IntentUnitStatus, LifecycleRecord,
    PhaseId, TransitionError, VocabularyError, Workflow, WorkflowEdge, WorkflowError, WorkflowId,
};

use crate::protocol::{
    EdgeInput, ErrorCode, ErrorDetail, HistoryEntry, IntentUnitSnapshot, OperationInput,
    PROTOCOL_VERSION, ProtocolRequest, SnapshotStatus, TransitionInput, WorkflowInput,
};

#[derive(Debug)]
pub(crate) struct PreparedScenario {
    pub(crate) intent_unit: IntentUnit,
    pub(crate) operations: Vec<PreparedOperation>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PreparedOperation {
    Transition { target: PhaseId },
    Complete,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LifecycleRejection {
    pub(crate) error: ErrorDetail,
    pub(crate) intent_unit: Box<IntentUnitSnapshot>,
}

pub(crate) fn prepare(request: ProtocolRequest) -> Result<PreparedScenario, ErrorDetail> {
    if request.protocol_version != PROTOCOL_VERSION {
        return Err(setup_error(
            ErrorCode::UnsupportedProtocolVersion,
            format!(
                "unsupported protocol version {}; expected {PROTOCOL_VERSION}",
                request.protocol_version
            ),
            None,
        ));
    }

    let WorkflowInput {
        id,
        phases,
        initial_phase,
        edges,
        completion_phases,
    } = request.workflow;
    let workflow_id =
        WorkflowId::new(id).map_err(|error| vocabulary_error("workflow.id", error))?;
    let phases = phases
        .into_iter()
        .enumerate()
        .map(|(index, phase)| {
            PhaseId::new(phase)
                .map_err(|error| vocabulary_error(format!("workflow.phases[{index}]"), error))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let initial_phase = PhaseId::new(initial_phase)
        .map_err(|error| vocabulary_error("workflow.initial_phase", error))?;
    let edges = edges
        .into_iter()
        .enumerate()
        .map(|(index, edge)| prepare_edge(index, edge))
        .collect::<Result<Vec<_>, _>>()?;
    let completion_phases = completion_phases
        .into_iter()
        .enumerate()
        .map(|(index, phase)| {
            PhaseId::new(phase).map_err(|error| {
                vocabulary_error(format!("workflow.completion_phases[{index}]"), error)
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let workflow = Workflow::new(workflow_id, phases, initial_phase, edges, completion_phases)
        .map_err(workflow_error)?;

    let species = IntentSpecies::new(request.intent_unit.species)
        .map_err(|error| vocabulary_error("intent_unit.species", error))?;
    let id = match request.intent_unit.id {
        Some(value) => IntentUnitId::from_str(&value).map_err(|error| {
            setup_error(
                ErrorCode::InvalidIntentUnitId,
                error.to_string(),
                Some("intent_unit.id".to_owned()),
            )
        })?,
        None => IntentUnitId::generate(),
    };
    let operations = request
        .operations
        .into_iter()
        .enumerate()
        .map(|(index, operation)| match operation {
            OperationInput::Transition(TransitionInput { target }) => PhaseId::new(target)
                .map(|target| PreparedOperation::Transition { target })
                .map_err(|error| vocabulary_error(format!("operations[{index}].target"), error)),
            OperationInput::Complete(_) => Ok(PreparedOperation::Complete),
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(PreparedScenario {
        intent_unit: IntentUnit::new(id, species, workflow),
        operations,
    })
}

pub(crate) fn execute(
    mut prepared: PreparedScenario,
) -> Result<IntentUnitSnapshot, LifecycleRejection> {
    for (index, operation) in prepared.operations.into_iter().enumerate() {
        let operation_number = index + 1;
        let result = match operation {
            PreparedOperation::Transition { target } => prepared
                .intent_unit
                .transition_to(&target)
                .map_err(|error| transition_error(error, operation_number)),
            PreparedOperation::Complete => prepared
                .intent_unit
                .complete()
                .map_err(|error| completion_error(error, operation_number)),
        };

        if let Err(error) = result {
            return Err(LifecycleRejection {
                error,
                intent_unit: Box::new(snapshot(&prepared.intent_unit)),
            });
        }
    }

    Ok(snapshot(&prepared.intent_unit))
}

fn prepare_edge(index: usize, edge: EdgeInput) -> Result<WorkflowEdge, ErrorDetail> {
    let from = PhaseId::new(edge.from)
        .map_err(|error| vocabulary_error(format!("workflow.edges[{index}].from"), error))?;
    let to = PhaseId::new(edge.to)
        .map_err(|error| vocabulary_error(format!("workflow.edges[{index}].to"), error))?;
    Ok(WorkflowEdge::new(from, to))
}

fn vocabulary_error(field: impl Into<String>, error: VocabularyError) -> ErrorDetail {
    let code = match error {
        VocabularyError::Blank => ErrorCode::BlankValue,
    };
    setup_error(code, error.to_string(), Some(field.into()))
}

fn workflow_error(error: WorkflowError) -> ErrorDetail {
    let code = match &error {
        WorkflowError::EmptyPhases => ErrorCode::WorkflowEmptyPhases,
        WorkflowError::DuplicatePhase { .. } => ErrorCode::WorkflowDuplicatePhase,
        WorkflowError::UnknownInitialPhase { .. } => ErrorCode::WorkflowUnknownInitialPhase,
        WorkflowError::UnknownEdgeSource { .. } => ErrorCode::WorkflowUnknownEdgeSource,
        WorkflowError::UnknownEdgeTarget { .. } => ErrorCode::WorkflowUnknownEdgeTarget,
        WorkflowError::DuplicateEdge { .. } => ErrorCode::WorkflowDuplicateEdge,
        WorkflowError::UnknownCompletionPhase { .. } => ErrorCode::WorkflowUnknownCompletionPhase,
        WorkflowError::DuplicateCompletionPhase { .. } => {
            ErrorCode::WorkflowDuplicateCompletionPhase
        }
    };
    setup_error(code, error.to_string(), None)
}

fn setup_error(code: ErrorCode, message: String, field: Option<String>) -> ErrorDetail {
    ErrorDetail {
        code,
        message,
        field,
        operation_number: None,
    }
}

fn transition_error(error: TransitionError, operation_number: usize) -> ErrorDetail {
    let code = match &error {
        TransitionError::AlreadyCompleted => ErrorCode::TransitionAlreadyCompleted,
        TransitionError::UnknownTarget { .. } => ErrorCode::TransitionUnknownTarget,
        TransitionError::NotAllowed { .. } => ErrorCode::TransitionNotAllowed,
    };
    lifecycle_error(code, error.to_string(), operation_number)
}

fn completion_error(error: CompletionError, operation_number: usize) -> ErrorDetail {
    let code = match &error {
        CompletionError::AlreadyCompleted => ErrorCode::CompletionAlreadyCompleted,
        CompletionError::PhaseNotEligible { .. } => ErrorCode::CompletionPhaseNotEligible,
    };
    lifecycle_error(code, error.to_string(), operation_number)
}

fn lifecycle_error(code: ErrorCode, message: String, operation_number: usize) -> ErrorDetail {
    ErrorDetail {
        code,
        message,
        field: None,
        operation_number: Some(operation_number),
    }
}

fn snapshot(intent_unit: &IntentUnit) -> IntentUnitSnapshot {
    let status = match intent_unit.status() {
        IntentUnitStatus::Active => SnapshotStatus::Active,
        IntentUnitStatus::Completed => SnapshotStatus::Completed,
    };
    let history = intent_unit
        .history()
        .iter()
        .map(|record| match record {
            LifecycleRecord::Transition(record) => HistoryEntry::Transition {
                sequence: record.sequence(),
                from: record.from().as_str().to_owned(),
                to: record.to().as_str().to_owned(),
            },
            LifecycleRecord::Completion(record) => HistoryEntry::Completion {
                sequence: record.sequence(),
                phase: record.final_phase().as_str().to_owned(),
            },
        })
        .collect();

    IntentUnitSnapshot {
        id: intent_unit.id().to_string(),
        species: intent_unit.species().as_str().to_owned(),
        workflow_id: intent_unit.workflow_id().as_str().to_owned(),
        phase: intent_unit.phase().as_str().to_owned(),
        status,
        history,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{CompleteInput, IntentUnitInput};

    fn edge(from: &str, to: &str) -> EdgeInput {
        EdgeInput {
            from: from.to_owned(),
            to: to.to_owned(),
        }
    }

    fn transition(target: &str) -> OperationInput {
        OperationInput::Transition(TransitionInput {
            target: target.to_owned(),
        })
    }

    fn complete() -> OperationInput {
        OperationInput::Complete(CompleteInput {})
    }

    fn request() -> ProtocolRequest {
        ProtocolRequest {
            protocol_version: PROTOCOL_VERSION,
            workflow: WorkflowInput {
                id: "delivery".to_owned(),
                phases: vec!["queued".to_owned(), "doing".to_owned(), "done".to_owned()],
                initial_phase: "queued".to_owned(),
                edges: vec![edge("queued", "doing"), edge("doing", "done")],
                completion_phases: vec!["done".to_owned()],
            },
            intent_unit: IntentUnitInput {
                id: Some("6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_owned()),
                species: "feature".to_owned(),
            },
            operations: vec![
                OperationInput::Transition(TransitionInput {
                    target: "doing".to_owned(),
                }),
                OperationInput::Complete(CompleteInput {}),
            ],
        }
    }

    fn assert_setup_error(
        request: ProtocolRequest,
        expected_code: ErrorCode,
        expected_field: Option<&str>,
    ) {
        let error = prepare(request).expect_err("fixture should be rejected");
        assert_eq!(error.code, expected_code);
        assert_eq!(error.field.as_deref(), expected_field);
        assert_eq!(error.operation_number, None);
    }

    #[test]
    fn test_fixed_id_scenario_constructs_core_state() {
        let mut request = request();
        request.workflow.id = "  delivery  ".to_owned();
        request.workflow.phases[0] = "待機中".to_owned();
        request.workflow.initial_phase = "待機中".to_owned();
        request.workflow.edges[0].from = "待機中".to_owned();
        request.intent_unit.species = "機能".to_owned();

        let prepared = prepare(request).expect("valid custom scenario should prepare");

        assert_eq!(
            prepared.intent_unit.id().to_string(),
            "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
        );
        assert_eq!(prepared.intent_unit.workflow_id().as_str(), "  delivery  ");
        assert_eq!(prepared.intent_unit.phase().as_str(), "待機中");
        assert_eq!(prepared.intent_unit.species().as_str(), "機能");
        assert_eq!(prepared.operations.len(), 2);
    }

    #[test]
    fn test_omitted_id_generates_non_nil_v4() {
        let mut request = request();
        request.intent_unit.id = None;

        let prepared = prepare(request).expect("valid scenario should prepare");

        assert!(!prepared.intent_unit.id().as_uuid().is_nil());
        assert_eq!(prepared.intent_unit.id().as_uuid().get_version_num(), 4);
    }

    #[test]
    fn test_unsupported_version_and_scalar_failures_are_typed() {
        let mut unsupported = request();
        unsupported.protocol_version = 2;
        assert_setup_error(unsupported, ErrorCode::UnsupportedProtocolVersion, None);

        let mut invalid_id = request();
        invalid_id.intent_unit.id = Some("not-a-uuid".to_owned());
        assert_setup_error(
            invalid_id,
            ErrorCode::InvalidIntentUnitId,
            Some("intent_unit.id"),
        );

        type Mutation = fn(&mut ProtocolRequest);
        let cases: [(Mutation, &str); 8] = [
            (|value| value.workflow.id.clear(), "workflow.id"),
            (
                |value| value.workflow.phases[1].clear(),
                "workflow.phases[1]",
            ),
            (
                |value| value.workflow.initial_phase.clear(),
                "workflow.initial_phase",
            ),
            (
                |value| value.workflow.edges[0].from.clear(),
                "workflow.edges[0].from",
            ),
            (
                |value| value.workflow.edges[0].to.clear(),
                "workflow.edges[0].to",
            ),
            (
                |value| value.workflow.completion_phases[0].clear(),
                "workflow.completion_phases[0]",
            ),
            (
                |value| value.intent_unit.species.clear(),
                "intent_unit.species",
            ),
            (
                |value| {
                    let OperationInput::Transition(operation) = &mut value.operations[0] else {
                        panic!("fixture operation should be a transition");
                    };
                    operation.target.clear();
                },
                "operations[0].target",
            ),
        ];
        for (mutate, field) in cases {
            let mut value = request();
            mutate(&mut value);
            assert_setup_error(value, ErrorCode::BlankValue, Some(field));
        }
    }

    #[test]
    fn test_workflow_errors_map_exhaustively() {
        let mut empty = request();
        empty.workflow.phases.clear();

        let mut duplicate_phase = request();
        duplicate_phase.workflow.phases.push("queued".to_owned());

        let mut unknown_initial = request();
        unknown_initial.workflow.initial_phase = "missing".to_owned();

        let mut unknown_source = request();
        unknown_source.workflow.edges[0].from = "missing".to_owned();

        let mut unknown_target = request();
        unknown_target.workflow.edges[0].to = "missing".to_owned();

        let mut duplicate_edge = request();
        duplicate_edge.workflow.edges.push(edge("queued", "doing"));

        let mut unknown_completion = request();
        unknown_completion.workflow.completion_phases[0] = "missing".to_owned();

        let mut duplicate_completion = request();
        duplicate_completion
            .workflow
            .completion_phases
            .push("done".to_owned());

        let cases = [
            (empty, ErrorCode::WorkflowEmptyPhases),
            (duplicate_phase, ErrorCode::WorkflowDuplicatePhase),
            (unknown_initial, ErrorCode::WorkflowUnknownInitialPhase),
            (unknown_source, ErrorCode::WorkflowUnknownEdgeSource),
            (unknown_target, ErrorCode::WorkflowUnknownEdgeTarget),
            (duplicate_edge, ErrorCode::WorkflowDuplicateEdge),
            (
                unknown_completion,
                ErrorCode::WorkflowUnknownCompletionPhase,
            ),
            (
                duplicate_completion,
                ErrorCode::WorkflowDuplicateCompletionPhase,
            ),
        ];

        for (request, expected_code) in cases {
            assert_setup_error(request, expected_code, None);
        }
    }

    #[test]
    fn test_executor_returns_completed_adapter_snapshot() {
        let mut request = request();
        request.operations = vec![transition("doing"), transition("done"), complete()];

        let snapshot = execute(prepare(request).expect("scenario should prepare"))
            .expect("lifecycle should succeed");

        assert_eq!(snapshot.id, "6ba7b810-9dad-11d1-80b4-00c04fd430c8");
        assert_eq!(snapshot.species, "feature");
        assert_eq!(snapshot.workflow_id, "delivery");
        assert_eq!(snapshot.phase, "done");
        assert_eq!(snapshot.status, SnapshotStatus::Completed);
        assert_eq!(
            snapshot.history,
            [
                HistoryEntry::Transition {
                    sequence: 1,
                    from: "queued".to_owned(),
                    to: "doing".to_owned(),
                },
                HistoryEntry::Transition {
                    sequence: 2,
                    from: "doing".to_owned(),
                    to: "done".to_owned(),
                },
                HistoryEntry::Completion {
                    sequence: 3,
                    phase: "done".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn test_executor_returns_active_snapshot_for_empty_operations() {
        let mut request = request();
        request.operations.clear();

        let snapshot = execute(prepare(request).expect("scenario should prepare"))
            .expect("empty lifecycle should succeed");

        assert_eq!(snapshot.phase, "queued");
        assert_eq!(snapshot.status, SnapshotStatus::Active);
        assert!(snapshot.history.is_empty());
    }

    #[test]
    fn test_executor_honors_declared_reverse_and_self_edges() {
        let mut request = request();
        request.workflow.phases = vec!["queued".to_owned(), "doing".to_owned()];
        request.workflow.edges = vec![
            edge("queued", "doing"),
            edge("doing", "doing"),
            edge("doing", "queued"),
        ];
        request.workflow.completion_phases.clear();
        request.operations = vec![
            transition("doing"),
            transition("doing"),
            transition("queued"),
        ];

        let snapshot = execute(prepare(request).expect("scenario should prepare"))
            .expect("declared reverse and self edges should succeed");

        assert_eq!(snapshot.phase, "queued");
        assert_eq!(snapshot.status, SnapshotStatus::Active);
        assert_eq!(snapshot.history.len(), 3);
        assert_eq!(
            snapshot.history[1],
            HistoryEntry::Transition {
                sequence: 2,
                from: "doing".to_owned(),
                to: "doing".to_owned(),
            }
        );
        assert_eq!(
            snapshot.history[2],
            HistoryEntry::Transition {
                sequence: 3,
                from: "doing".to_owned(),
                to: "queued".to_owned(),
            }
        );
    }

    #[test]
    fn test_lifecycle_errors_map_exhaustively() {
        let phase = || PhaseId::new("phase").expect("fixture phase should be valid");
        let transition_cases = [
            (
                TransitionError::AlreadyCompleted,
                ErrorCode::TransitionAlreadyCompleted,
            ),
            (
                TransitionError::UnknownTarget { target: phase() },
                ErrorCode::TransitionUnknownTarget,
            ),
            (
                TransitionError::NotAllowed {
                    from: phase(),
                    to: PhaseId::new("other").expect("fixture phase should be valid"),
                },
                ErrorCode::TransitionNotAllowed,
            ),
        ];
        for (error, expected_code) in transition_cases {
            let error = transition_error(error, 7);
            assert_eq!(error.code, expected_code);
            assert_eq!(error.field, None);
            assert_eq!(error.operation_number, Some(7));
        }

        let completion_cases = [
            (
                CompletionError::AlreadyCompleted,
                ErrorCode::CompletionAlreadyCompleted,
            ),
            (
                CompletionError::PhaseNotEligible { phase: phase() },
                ErrorCode::CompletionPhaseNotEligible,
            ),
        ];
        for (error, expected_code) in completion_cases {
            let error = completion_error(error, 8);
            assert_eq!(error.code, expected_code);
            assert_eq!(error.field, None);
            assert_eq!(error.operation_number, Some(8));
        }
    }

    #[test]
    fn test_executor_reports_atomic_failure_with_prior_state() {
        let mut rejected_transition = request();
        rejected_transition.operations = vec![
            transition("doing"),
            transition("queued"),
            transition("done"),
        ];
        let transition_error =
            execute(prepare(rejected_transition).expect("transition scenario should prepare"))
                .expect_err("undeclared reverse edge should fail");
        assert_eq!(transition_error.error.code, ErrorCode::TransitionNotAllowed);
        assert_eq!(transition_error.error.operation_number, Some(2));
        assert_eq!(transition_error.intent_unit.phase, "doing");
        assert_eq!(transition_error.intent_unit.status, SnapshotStatus::Active);
        assert_eq!(transition_error.intent_unit.history.len(), 1);

        let mut rejected_completion = request();
        rejected_completion.operations = vec![transition("doing"), complete(), transition("done")];
        let completion_error =
            execute(prepare(rejected_completion).expect("completion scenario should prepare"))
                .expect_err("ineligible completion should fail");
        assert_eq!(
            completion_error.error.code,
            ErrorCode::CompletionPhaseNotEligible
        );
        assert_eq!(completion_error.error.operation_number, Some(2));
        assert_eq!(completion_error.intent_unit.phase, "doing");
        assert_eq!(completion_error.intent_unit.status, SnapshotStatus::Active);
        assert_eq!(completion_error.intent_unit.history.len(), 1);
    }

    #[test]
    fn test_executor_reports_operation_after_completion() {
        fn terminal_request(operations: Vec<OperationInput>) -> ProtocolRequest {
            let mut request = request();
            request.workflow.phases = vec!["queued".to_owned()];
            request.workflow.edges = vec![edge("queued", "queued")];
            request.workflow.completion_phases = vec!["queued".to_owned()];
            request.operations = operations;
            request
        }

        let baseline =
            execute(prepare(terminal_request(vec![complete()])).expect("baseline should prepare"))
                .expect("baseline completion should succeed");

        let transition_error = execute(
            prepare(terminal_request(vec![complete(), transition("queued")]))
                .expect("transition scenario should prepare"),
        )
        .expect_err("transition after completion should fail");
        assert_eq!(
            transition_error.error.code,
            ErrorCode::TransitionAlreadyCompleted
        );
        assert_eq!(transition_error.error.operation_number, Some(2));
        assert_eq!(*transition_error.intent_unit, baseline);

        let completion_error = execute(
            prepare(terminal_request(vec![complete(), complete()]))
                .expect("completion scenario should prepare"),
        )
        .expect_err("second completion should fail");
        assert_eq!(
            completion_error.error.code,
            ErrorCode::CompletionAlreadyCompleted
        );
        assert_eq!(completion_error.error.operation_number, Some(2));
        assert_eq!(*completion_error.intent_unit, baseline);
    }
}
