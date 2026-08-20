use cubikan_core::{
    BoundedWorkflowError, CompletionError, ExternalReference, IntentSpecies, IntentUnit,
    IntentUnitId, PhaseId, ReferenceNamespace, ReferenceText, TransitionError, Workflow,
    WorkflowEdge, WorkflowError, WorkflowId,
};

use crate::protocol::{
    ErrorDetail, OperationInput, ProtocolResponse, Request, WorkflowEdgeInput, WorkflowInput,
};

pub(crate) enum SimulationOutcome {
    Success(ProtocolResponse),
    OperationRejected(ProtocolResponse),
}

pub(crate) fn simulate<F>(
    request: Request,
    generate_id: &mut F,
) -> Result<SimulationOutcome, ErrorDetail>
where
    F: FnMut() -> IntentUnitId,
{
    let workflow = convert_workflow(&request.workflow)?;
    let origin = convert_origin(&request.intent_unit.origin)?;
    let species = IntentSpecies::from_bytes(request.intent_unit.species.as_bytes())
        .map_err(|_| ErrorDetail::invalid_species())?;
    let id = match request.intent_unit.id {
        Some(id) => parse_intent_unit_id(&id)?,
        None => generate_id(),
    };
    let operations = request
        .operations
        .into_iter()
        .enumerate()
        .map(|(index, operation)| convert_operation(operation, index))
        .collect::<Result<Vec<_>, _>>()?;

    let mut intent_unit = IntentUnit::new(id, origin, species, workflow);
    for (operation_number, operation) in operations.iter().enumerate() {
        let error = match operation {
            DomainOperation::Transition(target) => intent_unit
                .transition_to(target)
                .err()
                .map(|error| transition_error(error, operation_number)),
            DomainOperation::Complete => intent_unit
                .complete()
                .err()
                .map(|error| completion_error(error, operation_number)),
        };
        if let Some(error) = error {
            return Ok(SimulationOutcome::OperationRejected(
                ProtocolResponse::operation_error(error, &intent_unit),
            ));
        }
    }

    Ok(SimulationOutcome::Success(ProtocolResponse::success(
        &intent_unit,
    )))
}

fn convert_origin(
    input: &crate::protocol::ExternalReferenceInput,
) -> Result<ExternalReference, ErrorDetail> {
    let namespace = ReferenceNamespace::from_bytes(input.namespace.as_bytes())
        .map_err(|_| ErrorDetail::invalid_external_reference("/intent_unit/origin/namespace"))?;
    let scope = ReferenceText::from_bytes(input.scope.as_bytes())
        .map_err(|_| ErrorDetail::invalid_external_reference("/intent_unit/origin/scope"))?;
    let value = ReferenceText::from_bytes(input.value.as_bytes())
        .map_err(|_| ErrorDetail::invalid_external_reference("/intent_unit/origin/value"))?;
    Ok(ExternalReference::new(namespace, scope, value))
}

fn parse_intent_unit_id(value: &str) -> Result<IntentUnitId, ErrorDetail> {
    if !is_canonical_uuid(value) {
        return Err(ErrorDetail::invalid_intent_unit_id());
    }
    value
        .parse()
        .map_err(|_| ErrorDetail::invalid_intent_unit_id())
}

fn is_canonical_uuid(value: &str) -> bool {
    if value.len() != 36 || !value.is_ascii() {
        return false;
    }
    value.bytes().enumerate().all(|(index, byte)| {
        if matches!(index, 8 | 13 | 18 | 23) {
            byte == b'-'
        } else {
            byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
        }
    })
}

fn convert_workflow(input: &WorkflowInput) -> Result<Workflow, ErrorDetail> {
    let id = WorkflowId::from_bytes(input.id.as_bytes())
        .map_err(|_| ErrorDetail::invalid_workflow_id())?;
    if distinct_count(&input.phases) > cubikan_core::MAX_WORKFLOW_PHASES {
        return Err(ErrorDetail::invalid_workflow("/workflow/phases"));
    }
    if input.edges.len() > cubikan_core::MAX_WORKFLOW_EDGES {
        return Err(ErrorDetail::invalid_workflow("/workflow/edges"));
    }
    if input.completion_phases.len() > cubikan_core::MAX_COMPLETION_PHASES {
        return Err(ErrorDetail::invalid_workflow("/workflow/completion_phases"));
    }
    if input.phases.len() > cubikan_core::MAX_WORKFLOW_PHASES {
        return Err(ErrorDetail::invalid_workflow("/workflow/phases"));
    }
    let phases = input
        .phases
        .iter()
        .enumerate()
        .map(|(index, phase)| phase_id(phase, format!("/workflow/phases/{index}")))
        .collect::<Result<Vec<_>, _>>()?;
    let initial_phase = phase_id(&input.initial_phase, "/workflow/initial_phase")?;
    let edges = input
        .edges
        .iter()
        .enumerate()
        .map(|(index, edge)| convert_edge(edge, index))
        .collect::<Result<Vec<_>, _>>()?;
    let completion_phases = input
        .completion_phases
        .iter()
        .enumerate()
        .map(|(index, phase)| phase_id(phase, format!("/workflow/completion_phases/{index}")))
        .collect::<Result<Vec<_>, _>>()?;

    Workflow::new_bounded(
        id,
        phases.clone(),
        initial_phase,
        edges.clone(),
        completion_phases.clone(),
    )
    .map_err(|error| workflow_error(error, &phases, &edges, &completion_phases))
}

fn distinct_count<T: Eq>(values: &[T]) -> usize {
    values
        .iter()
        .enumerate()
        .filter(|(index, value)| !values[..*index].contains(value))
        .count()
}

fn convert_edge(input: &WorkflowEdgeInput, index: usize) -> Result<WorkflowEdge, ErrorDetail> {
    Ok(WorkflowEdge::new(
        phase_id(&input.from, format!("/workflow/edges/{index}/from"))?,
        phase_id(&input.to, format!("/workflow/edges/{index}/to"))?,
    ))
}

fn phase_id(value: &str, field: impl Into<String>) -> Result<PhaseId, ErrorDetail> {
    PhaseId::from_bytes(value.as_bytes()).map_err(|_| ErrorDetail::invalid_phase_id(field))
}

fn workflow_error(
    error: BoundedWorkflowError,
    phases: &[PhaseId],
    edges: &[WorkflowEdge],
    completion_phases: &[PhaseId],
) -> ErrorDetail {
    match error {
        BoundedWorkflowError::TooManyPhases { .. } => {
            ErrorDetail::invalid_workflow("/workflow/phases")
        }
        BoundedWorkflowError::TooManyEdges { .. } => {
            ErrorDetail::invalid_workflow("/workflow/edges")
        }
        BoundedWorkflowError::TooManyCompletionPhases { .. } => {
            ErrorDetail::invalid_workflow("/workflow/completion_phases")
        }
        BoundedWorkflowError::Topology(error) => match error {
            WorkflowError::EmptyPhases => ErrorDetail::invalid_workflow("/workflow/phases"),
            WorkflowError::DuplicatePhase { phase } => ErrorDetail::invalid_workflow(format!(
                "/workflow/phases/{}",
                duplicate_index(phases, &phase)
            )),
            WorkflowError::UnknownInitialPhase { .. } => {
                ErrorDetail::invalid_workflow("/workflow/initial_phase")
            }
            WorkflowError::UnknownEdgeSource { phase } => ErrorDetail::invalid_workflow(format!(
                "/workflow/edges/{}/from",
                edges
                    .iter()
                    .position(|edge| edge.from() == &phase)
                    .unwrap_or(0)
            )),
            WorkflowError::UnknownEdgeTarget { phase } => ErrorDetail::invalid_workflow(format!(
                "/workflow/edges/{}/to",
                edges
                    .iter()
                    .position(|edge| edge.to() == &phase)
                    .unwrap_or(0)
            )),
            WorkflowError::DuplicateEdge { edge } => ErrorDetail::invalid_workflow(format!(
                "/workflow/edges/{}",
                duplicate_index(edges, &edge)
            )),
            WorkflowError::UnknownCompletionPhase { phase } => {
                ErrorDetail::invalid_workflow(format!(
                    "/workflow/completion_phases/{}",
                    completion_phases
                        .iter()
                        .position(|candidate| candidate == &phase)
                        .unwrap_or(0)
                ))
            }
            WorkflowError::DuplicateCompletionPhase { phase } => {
                ErrorDetail::invalid_workflow(format!(
                    "/workflow/completion_phases/{}",
                    duplicate_index(completion_phases, &phase)
                ))
            }
        },
    }
}

fn duplicate_index<T: Eq>(values: &[T], duplicate: &T) -> usize {
    values
        .iter()
        .enumerate()
        .filter(|(_, value)| *value == duplicate)
        .nth(1)
        .map_or(0, |(index, _)| index)
}

fn convert_operation(
    operation: OperationInput,
    operation_number: usize,
) -> Result<DomainOperation, ErrorDetail> {
    match operation {
        OperationInput::Transition { target } => Ok(DomainOperation::Transition(phase_id(
            &target,
            format!("/operations/{operation_number}/target"),
        )?)),
        OperationInput::Complete => Ok(DomainOperation::Complete),
    }
}

fn transition_error(error: TransitionError, operation_number: usize) -> ErrorDetail {
    match error {
        TransitionError::AlreadyCompleted => {
            ErrorDetail::transition_already_completed(operation_number)
        }
        TransitionError::UnknownTarget { .. } => {
            ErrorDetail::transition_unknown_target(operation_number)
        }
        TransitionError::NotAllowed { .. }
        | TransitionError::LifecycleHistoryCapacityExceeded { .. } => {
            ErrorDetail::transition_not_allowed(operation_number)
        }
    }
}

fn completion_error(error: CompletionError, operation_number: usize) -> ErrorDetail {
    match error {
        CompletionError::AlreadyCompleted => {
            ErrorDetail::completion_already_completed(operation_number)
        }
        CompletionError::PhaseNotEligible { .. }
        | CompletionError::LifecycleHistoryCapacityExceeded { .. } => {
            ErrorDetail::completion_phase_not_eligible(operation_number)
        }
    }
}

enum DomainOperation {
    Transition(PhaseId),
    Complete,
}
