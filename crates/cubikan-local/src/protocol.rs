use std::{fmt, str::FromStr};

use cubikan_backend::{
    AssociationDirection, AssociationPage, DirectRelationshipPredicate, GetIntentUnit,
    LedgerCoordinate, ListAssociationsByReference, ListAssociationsByUnit, ListCursor, ListFilters,
    ListIntentUnits, ListRelationships, PageLimit, ProjectedAssociation, ProjectedDefinition,
    ProjectedDefinitionResult, ProjectedProjectionPage, ProjectedRelationship,
    ProjectedRelationshipPage, ProjectedUnit, ProjectedUnitPage, ProjectedUnitResult,
    ProjectedUnitSummary, ProjectionCheckpoint, ProjectionQueryV1, RelationshipCursor,
    RelationshipDefinitionId as BackendDefinitionId,
    RelationshipDefinitionKey as BackendDefinitionKey,
    RelationshipDefinitionVersion as BackendDefinitionVersion,
    RelationshipIdentity as BackendRelationship,
};
use cubikan_chain_client::{
    AcceptedCoordinate, AcceptedEffect, DeploymentIdentity, FinalizedExtrinsic, MortalEra,
    Mutation, MutationOperation, SubmissionFailureCode, SubmissionOutcome, SubmissionOutcomeKind,
};
use cubikan_core::{
    AssociationSubject, BoundedWorkflowError, ExternalReference, IntentSpecies, IntentUnitId,
    IntentUnitStatus, LifecycleRecord, PhaseId, RecordedAssociation, ReferenceNamespace,
    ReferenceText, RelationshipDefinition, RelationshipDefinitionKey as CoreDefinitionKey,
    RelationshipDefinitionVersion as CoreDefinitionVersion,
    RelationshipIdentity as CoreRelationship, RelationshipPolicy, Workflow, WorkflowEdge,
    WorkflowError, WorkflowId,
};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{MapAccess, SeqAccess, Visitor},
};

pub(crate) const PROTOCOL_VERSION: u8 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponseClass {
    Success,
    OperationalFailure,
    RequestRejected,
    DomainRejected,
    EnvironmentFailure,
}

impl ResponseClass {
    #[must_use]
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::OperationalFailure => 1,
            Self::RequestRejected => 2,
            Self::DomainRejected => 3,
            Self::EnvironmentFailure => 4,
        }
    }
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

    fn encode(class: ResponseClass, response: &impl Serialize) -> Self {
        Self {
            class,
            body: serde_json::to_vec(response)
                .expect("adapter-owned protocol responses must always serialize"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DecodedRequest {
    pub(crate) operation: DecodedOperation,
}

impl DecodedRequest {
    #[must_use]
    pub(crate) const fn operation(&self) -> &DecodedOperation {
        &self.operation
    }

    #[must_use]
    pub(crate) fn into_operation(self) -> DecodedOperation {
        self.operation
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DecodedOperation {
    Mutation(Mutation),
    GetIntentUnit(GetIntentUnit),
    ListIntentUnits(ListIntentUnits),
    GetRelationshipDefinition(BackendDefinitionKey),
    ListRelationships(ListRelationships),
    ProjectIntentUnitsV1(ProjectionQueryV1),
    ListAssociationsByUnit(ListAssociationsByUnit),
    ListAssociationsByReference(ListAssociationsByReference),
}

impl DecodedOperation {
    #[must_use]
    pub(crate) const fn is_mutation(&self) -> bool {
        matches!(self, Self::Mutation(_))
    }
}

trait IdSource {
    fn generate(&mut self) -> IntentUnitId;
}

struct ProductionIdSource;

impl IdSource for ProductionIdSource {
    fn generate(&mut self) -> IntentUnitId {
        IntentUnitId::generate()
    }
}

pub(crate) fn decode_request(bytes: &[u8]) -> Result<DecodedRequest, ErrorDetail> {
    decode_request_from_source(bytes, &mut ProductionIdSource)
}

fn decode_request_from_source(
    bytes: &[u8],
    id_source: &mut impl IdSource,
) -> Result<DecodedRequest, ErrorDetail> {
    let root = decode_json(bytes)?;
    let root_entries = match &root {
        RawJson::Object(entries) => entries.as_slice(),
        _ => return Err(ErrorDetail::invalid_request("")),
    };
    if root_entries.is_empty() {
        return Err(ErrorDetail::invalid_request(""));
    }
    probe_protocol_version(root_entries)?;
    let root_entries = closed_object(&root, "", &["protocol_version", "operation"])?;
    let operation = required_member(root_entries, "operation", "")?;
    Ok(DecodedRequest {
        operation: parse_operation(operation, id_source)?,
    })
}

fn decode_json(bytes: &[u8]) -> Result<RawJson, ErrorDetail> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    RawJson::deserialize(&mut deserializer)
        .and_then(|value| deserializer.end().map(|()| value))
        .map_err(|_| ErrorDetail::plain(ErrorCode::MalformedJson))
}

fn probe_protocol_version(entries: &[(String, RawJson)]) -> Result<(), ErrorDetail> {
    let mut versions = entries
        .iter()
        .filter(|(name, _)| name == "protocol_version")
        .map(|(_, value)| value);
    let Some(version) = versions.next() else {
        return Err(ErrorDetail::invalid_request("/protocol_version"));
    };
    if versions.next().is_some() {
        return Err(ErrorDetail::invalid_request("/protocol_version"));
    }
    match version {
        RawJson::Unsigned(2) | RawJson::Signed(2) => Ok(()),
        RawJson::Unsigned(_) | RawJson::Signed(_) => Err(ErrorDetail::with_field(
            ErrorCode::UnsupportedProtocolVersion,
            "/protocol_version",
        )),
        _ => Err(ErrorDetail::invalid_request("/protocol_version")),
    }
}

fn parse_operation(
    value: &RawJson,
    id_source: &mut impl IdSource,
) -> Result<DecodedOperation, ErrorDetail> {
    const PATH: &str = "/operation";
    let RawJson::Object(entries) = value else {
        return Err(ErrorDetail::invalid_request(PATH));
    };
    ensure_unique_member(entries, "type", "/operation/type")?;
    let operation_type = match member(entries, "type") {
        Some(RawJson::String(value)) => value.as_str(),
        Some(_) | None => return Err(ErrorDetail::invalid_request("/operation/type")),
    };
    match operation_type {
        "create_intent_unit" => parse_create_unit(value, id_source),
        "get_intent_unit" => parse_get_unit(value),
        "list_intent_units" => parse_list_units(value),
        "transition_intent_unit" => parse_transition_unit(value),
        "complete_intent_unit" => parse_complete_unit(value),
        "create_relationship_definition" => parse_create_definition(value),
        "get_relationship_definition" => parse_get_definition(value),
        "create_relationship" => parse_mutate_relationship(value, false),
        "delete_relationship" => parse_mutate_relationship(value, true),
        "list_relationships" => parse_list_relationships(value),
        "project_intent_units_v1" => parse_projection(value),
        "record_association" => parse_mutate_association(value, false),
        "revoke_association" => parse_mutate_association(value, true),
        "list_associations_by_unit" => parse_associations_by_unit(value),
        "list_associations_by_reference" => parse_associations_by_reference(value),
        _ => Err(ErrorDetail::invalid_request("/operation/type")),
    }
}

fn parse_create_unit(
    value: &RawJson,
    id_source: &mut impl IdSource,
) -> Result<DecodedOperation, ErrorDetail> {
    let entries = closed_object(value, "/operation", &["type", "intent_unit", "workflow"])?;
    let unit_value = required_member(entries, "intent_unit", "/operation")?;
    let unit_entries = closed_object(
        unit_value,
        "/operation/intent_unit",
        &["id", "origin", "species"],
    )?;
    let id = match member(unit_entries, "id") {
        None => None,
        Some(RawJson::String(value)) => Some(parse_uuid(
            value,
            "/operation/intent_unit/id",
            ErrorCode::InvalidIntentUnitId,
        )?),
        Some(_) => return Err(ErrorDetail::invalid_request("/operation/intent_unit/id")),
    };
    let origin = parse_external_reference(
        required_member(unit_entries, "origin", "/operation/intent_unit")?,
        "/operation/intent_unit/origin",
        false,
    )?;
    let species = parse_species_member(unit_entries, "species", "/operation/intent_unit")?;
    let workflow = parse_workflow(required_member(entries, "workflow", "/operation")?)?;
    let id = id.unwrap_or_else(|| id_source.generate());
    Ok(DecodedOperation::Mutation(Mutation::CreateUnit {
        id,
        origin,
        species,
        workflow,
    }))
}

fn parse_get_unit(value: &RawJson) -> Result<DecodedOperation, ErrorDetail> {
    let entries = closed_object(value, "/operation", &["type", "id"])?;
    let id = parse_required_uuid(entries, "id", "/operation", ErrorCode::InvalidIntentUnitId)?;
    Ok(DecodedOperation::GetIntentUnit(GetIntentUnit::new(id)))
}

fn parse_list_units(value: &RawJson) -> Result<DecodedOperation, ErrorDetail> {
    let entries = closed_object(value, "/operation", &["type", "filters", "limit", "after"])?;
    let filters = parse_filters(required_member(entries, "filters", "/operation")?)?;
    let limit = parse_page_limit(required_member(entries, "limit", "/operation")?)?;
    let after = parse_optional_cursor_uuid(entries, "after")?;
    Ok(DecodedOperation::ListIntentUnits(ListIntentUnits::new(
        filters, limit, after,
    )))
}

fn parse_transition_unit(value: &RawJson) -> Result<DecodedOperation, ErrorDetail> {
    let entries = closed_object(
        value,
        "/operation",
        &["type", "id", "target", "expected_revision"],
    )?;
    let id = parse_required_uuid(entries, "id", "/operation", ErrorCode::InvalidIntentUnitId)?;
    let target = required_string(entries, "target", "/operation")?;
    let target = PhaseId::from_bytes(target.as_bytes())
        .map_err(|_| ErrorDetail::with_field(ErrorCode::InvalidPhaseId, "/operation/target"))?;
    let expected_revision = parse_u64_text_member(
        entries,
        "expected_revision",
        "/operation",
        ErrorCode::InvalidRevision,
        false,
    )?;
    Ok(DecodedOperation::Mutation(Mutation::TransitionUnit {
        id,
        target,
        expected_revision,
    }))
}

fn parse_complete_unit(value: &RawJson) -> Result<DecodedOperation, ErrorDetail> {
    let entries = closed_object(value, "/operation", &["type", "id", "expected_revision"])?;
    let id = parse_required_uuid(entries, "id", "/operation", ErrorCode::InvalidIntentUnitId)?;
    let expected_revision = parse_u64_text_member(
        entries,
        "expected_revision",
        "/operation",
        ErrorCode::InvalidRevision,
        false,
    )?;
    Ok(DecodedOperation::Mutation(Mutation::CompleteUnit {
        id,
        expected_revision,
    }))
}

fn parse_create_definition(value: &RawJson) -> Result<DecodedOperation, ErrorDetail> {
    let entries = closed_object(
        value,
        "/operation",
        &[
            "type",
            "definition",
            "source_species",
            "target_species",
            "self_policy",
            "cycle_policy",
        ],
    )?;
    let (id, version) = parse_definition_parts(
        required_member(entries, "definition", "/operation")?,
        "/operation/definition",
        false,
    )?;
    let source_species = parse_optional_species(entries, "source_species", "/operation")?;
    let target_species = parse_optional_species(entries, "target_species", "/operation")?;
    let self_policy = parse_policy(entries, "self_policy")?;
    let cycle_policy = parse_policy(entries, "cycle_policy")?;
    let definition = RelationshipDefinition::new(
        CoreDefinitionKey::new(
            id,
            CoreDefinitionVersion::new(version).expect("a decoded definition version is nonzero"),
        ),
        source_species,
        target_species,
        self_policy,
        cycle_policy,
    );
    Ok(DecodedOperation::Mutation(
        Mutation::CreateRelationshipDefinition(definition),
    ))
}

fn parse_get_definition(value: &RawJson) -> Result<DecodedOperation, ErrorDetail> {
    let entries = closed_object(value, "/operation", &["type", "definition"])?;
    let definition = parse_backend_definition(
        required_member(entries, "definition", "/operation")?,
        "/operation/definition",
        false,
    )?;
    Ok(DecodedOperation::GetRelationshipDefinition(definition))
}

fn parse_mutate_relationship(
    value: &RawJson,
    delete: bool,
) -> Result<DecodedOperation, ErrorDetail> {
    let entries = closed_object(value, "/operation", &["type", "relationship"])?;
    let relationship = parse_core_relationship(
        required_member(entries, "relationship", "/operation")?,
        "/operation/relationship",
        false,
    )?;
    Ok(DecodedOperation::Mutation(if delete {
        Mutation::DeleteRelationship(relationship)
    } else {
        Mutation::CreateRelationship(relationship)
    }))
}

fn parse_list_relationships(value: &RawJson) -> Result<DecodedOperation, ErrorDetail> {
    let entries = closed_object(
        value,
        "/operation",
        &[
            "type",
            "definition",
            "source_id",
            "target_id",
            "limit",
            "after",
        ],
    )?;
    let definition = parse_backend_definition(
        required_member(entries, "definition", "/operation")?,
        "/operation/definition",
        false,
    )?;
    let source = parse_optional_uuid(entries, "source_id", ErrorCode::InvalidIntentUnitId)?;
    let target = parse_optional_uuid(entries, "target_id", ErrorCode::InvalidIntentUnitId)?;
    let limit = parse_page_limit(required_member(entries, "limit", "/operation")?)?;
    let after = match member(entries, "after") {
        None => None,
        Some(RawJson::Null) => return Err(ErrorDetail::invalid_request("/operation/after")),
        Some(value) => Some(RelationshipCursor::new(parse_backend_relationship(
            value,
            "/operation/after",
            true,
        )?)),
    };
    let query = ListRelationships::new(definition, source, target, limit, after)
        .map_err(|_| ErrorDetail::with_field(ErrorCode::InvalidCursor, "/operation/after"))?;
    Ok(DecodedOperation::ListRelationships(query))
}

fn parse_projection(value: &RawJson) -> Result<DecodedOperation, ErrorDetail> {
    let entries = closed_object(
        value,
        "/operation",
        &[
            "type",
            "query_version",
            "filters",
            "predicate",
            "limit",
            "after",
        ],
    )?;
    match required_member(entries, "query_version", "/operation")? {
        RawJson::Unsigned(1) | RawJson::Signed(1) => {}
        RawJson::Unsigned(_) | RawJson::Signed(_) => {
            return Err(ErrorDetail::with_field(
                ErrorCode::InvalidQuery,
                "/operation/query_version",
            ));
        }
        _ => return Err(ErrorDetail::invalid_request("/operation/query_version")),
    }
    let filters = parse_filters(required_member(entries, "filters", "/operation")?)?;
    let predicate = match member(entries, "predicate") {
        None => None,
        Some(RawJson::Null) => return Err(ErrorDetail::invalid_request("/operation/predicate")),
        Some(value) => Some(parse_predicate(value)?),
    };
    let limit = parse_page_limit(required_member(entries, "limit", "/operation")?)?;
    let after = parse_optional_cursor_uuid(entries, "after")?;
    Ok(DecodedOperation::ProjectIntentUnitsV1(
        ProjectionQueryV1::new(filters, predicate, limit, after),
    ))
}

fn parse_mutate_association(
    value: &RawJson,
    revoke: bool,
) -> Result<DecodedOperation, ErrorDetail> {
    let entries = closed_object(value, "/operation", &["type", "association"])?;
    let association = parse_association(
        required_member(entries, "association", "/operation")?,
        "/operation/association",
        false,
    )?;
    Ok(DecodedOperation::Mutation(if revoke {
        Mutation::RevokeAssociation(association)
    } else {
        Mutation::RecordAssociation(association)
    }))
}

fn parse_associations_by_unit(value: &RawJson) -> Result<DecodedOperation, ErrorDetail> {
    let entries = closed_object(
        value,
        "/operation",
        &["type", "unit_id", "subject", "limit", "after"],
    )?;
    let unit_id = parse_required_uuid(
        entries,
        "unit_id",
        "/operation",
        ErrorCode::InvalidIntentUnitId,
    )?;
    let subject = match member(entries, "subject") {
        None => None,
        Some(RawJson::Null) => return Err(ErrorDetail::invalid_request("/operation/subject")),
        Some(value) => Some(parse_subject(value, "/operation/subject", false)?),
    };
    let limit = parse_page_limit(required_member(entries, "limit", "/operation")?)?;
    let after = match member(entries, "after") {
        None => None,
        Some(RawJson::Null) => return Err(ErrorDetail::invalid_request("/operation/after")),
        Some(value) => Some(parse_association(value, "/operation/after", true)?),
    };
    let query = ListAssociationsByUnit::new(unit_id, subject, limit, after)
        .map_err(|_| ErrorDetail::with_field(ErrorCode::InvalidCursor, "/operation/after"))?;
    Ok(DecodedOperation::ListAssociationsByUnit(query))
}

fn parse_associations_by_reference(value: &RawJson) -> Result<DecodedOperation, ErrorDetail> {
    let entries = closed_object(
        value,
        "/operation",
        &["type", "reference", "limit", "after"],
    )?;
    let reference = parse_external_reference(
        required_member(entries, "reference", "/operation")?,
        "/operation/reference",
        false,
    )?;
    let limit = parse_page_limit(required_member(entries, "limit", "/operation")?)?;
    let after = match member(entries, "after") {
        None => None,
        Some(RawJson::Null) => return Err(ErrorDetail::invalid_request("/operation/after")),
        Some(value) => Some(parse_association(value, "/operation/after", true)?),
    };
    let query = ListAssociationsByReference::new(reference, limit, after)
        .map_err(|_| ErrorDetail::with_field(ErrorCode::InvalidCursor, "/operation/after"))?;
    Ok(DecodedOperation::ListAssociationsByReference(query))
}

fn parse_external_reference(
    value: &RawJson,
    path: &str,
    cursor: bool,
) -> Result<ExternalReference, ErrorDetail> {
    let entries = closed_object(value, path, &["namespace", "scope", "value"])?;
    let namespace_text = required_string(entries, "namespace", path)?;
    let scope_text = required_string(entries, "scope", path)?;
    let value_text = required_string(entries, "value", path)?;
    let invalid = |member_name: &str| {
        ErrorDetail::with_field(
            if cursor {
                ErrorCode::InvalidCursor
            } else {
                ErrorCode::InvalidExternalReference
            },
            pointer_member(path, member_name),
        )
    };
    let namespace = ReferenceNamespace::from_bytes(namespace_text.as_bytes())
        .map_err(|_| invalid("namespace"))?;
    let scope = ReferenceText::from_bytes(scope_text.as_bytes()).map_err(|_| invalid("scope"))?;
    let reference_value =
        ReferenceText::from_bytes(value_text.as_bytes()).map_err(|_| invalid("value"))?;
    let oid_length = match namespace.as_str() {
        "git.commit.sha1" => Some(40),
        "git.commit.sha256" => Some(64),
        _ => None,
    };
    if let Some(length) = oid_length
        && (value_text.len() != length
            || !value_text
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
    {
        return Err(invalid("value"));
    }
    Ok(ExternalReference::new(namespace, scope, reference_value))
}

fn parse_workflow(value: &RawJson) -> Result<Workflow, ErrorDetail> {
    const PATH: &str = "/operation/workflow";
    let entries = closed_object(
        value,
        PATH,
        &[
            "id",
            "phases",
            "initial_phase",
            "edges",
            "completion_phases",
        ],
    )?;
    let id_text = required_string(entries, "id", PATH)?;
    let id = WorkflowId::from_bytes(id_text.as_bytes()).map_err(|_| {
        ErrorDetail::with_field(ErrorCode::InvalidWorkflowId, "/operation/workflow/id")
    })?;
    let phases_value = required_member(entries, "phases", PATH)?;
    let RawJson::Array(phase_values) = phases_value else {
        return Err(ErrorDetail::invalid_request("/operation/workflow/phases"));
    };
    let phase_texts = parse_string_array(phase_values, "/operation/workflow/phases")?;
    if phase_texts.len() > cubikan_core::MAX_WORKFLOW_PHASES {
        return Err(ErrorDetail::with_field(
            ErrorCode::InvalidWorkflow,
            "/operation/workflow/phases",
        ));
    }
    let phases = phase_texts
        .iter()
        .enumerate()
        .map(|(index, phase)| parse_phase(phase, format!("/operation/workflow/phases/{index}")))
        .collect::<Result<Vec<_>, _>>()?;
    let initial_text = required_string(entries, "initial_phase", PATH)?;
    let initial_phase = parse_phase(&initial_text, "/operation/workflow/initial_phase")?;
    let edges_value = required_member(entries, "edges", PATH)?;
    let RawJson::Array(edge_values) = edges_value else {
        return Err(ErrorDetail::invalid_request("/operation/workflow/edges"));
    };
    if edge_values.len() > cubikan_core::MAX_WORKFLOW_EDGES {
        return Err(ErrorDetail::with_field(
            ErrorCode::InvalidWorkflow,
            "/operation/workflow/edges",
        ));
    }
    let edges = edge_values
        .iter()
        .enumerate()
        .map(|(index, edge)| {
            let path = format!("/operation/workflow/edges/{index}");
            let entries = closed_object(edge, &path, &["from", "to"])?;
            let from = required_string(entries, "from", &path)?;
            let to = required_string(entries, "to", &path)?;
            Ok(WorkflowEdge::new(
                parse_phase(&from, pointer_member(&path, "from"))?,
                parse_phase(&to, pointer_member(&path, "to"))?,
            ))
        })
        .collect::<Result<Vec<_>, ErrorDetail>>()?;
    let completions_value = required_member(entries, "completion_phases", PATH)?;
    let RawJson::Array(completion_values) = completions_value else {
        return Err(ErrorDetail::invalid_request(
            "/operation/workflow/completion_phases",
        ));
    };
    let completion_texts =
        parse_string_array(completion_values, "/operation/workflow/completion_phases")?;
    if completion_texts.len() > cubikan_core::MAX_COMPLETION_PHASES {
        return Err(ErrorDetail::with_field(
            ErrorCode::InvalidWorkflow,
            "/operation/workflow/completion_phases",
        ));
    }
    let completion_phases = completion_texts
        .iter()
        .enumerate()
        .map(|(index, phase)| {
            parse_phase(
                phase,
                format!("/operation/workflow/completion_phases/{index}"),
            )
        })
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

fn parse_string_array<'a>(values: &'a [RawJson], path: &str) -> Result<Vec<&'a str>, ErrorDetail> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| match value {
            RawJson::String(value) => Ok(value.as_str()),
            _ => Err(ErrorDetail::invalid_request(format!("{path}/{index}"))),
        })
        .collect()
}

fn parse_phase(value: &str, path: impl Into<String>) -> Result<PhaseId, ErrorDetail> {
    let path = path.into();
    PhaseId::from_bytes(value.as_bytes())
        .map_err(|_| ErrorDetail::with_field(ErrorCode::InvalidPhaseId, path))
}

fn workflow_error(
    error: BoundedWorkflowError,
    phases: &[PhaseId],
    edges: &[WorkflowEdge],
    completion_phases: &[PhaseId],
) -> ErrorDetail {
    let path = match error {
        BoundedWorkflowError::TooManyPhases { .. } => "/operation/workflow/phases".to_owned(),
        BoundedWorkflowError::TooManyEdges { .. } => "/operation/workflow/edges".to_owned(),
        BoundedWorkflowError::TooManyCompletionPhases { .. } => {
            "/operation/workflow/completion_phases".to_owned()
        }
        BoundedWorkflowError::Topology(error) => match error {
            WorkflowError::EmptyPhases => "/operation/workflow/phases".to_owned(),
            WorkflowError::DuplicatePhase { phase } => format!(
                "/operation/workflow/phases/{}",
                duplicate_index(phases, &phase)
            ),
            WorkflowError::UnknownInitialPhase { .. } => {
                "/operation/workflow/initial_phase".to_owned()
            }
            WorkflowError::UnknownEdgeSource { phase } => format!(
                "/operation/workflow/edges/{}/from",
                edges
                    .iter()
                    .position(|edge| edge.from() == &phase)
                    .unwrap_or(0)
            ),
            WorkflowError::UnknownEdgeTarget { phase } => format!(
                "/operation/workflow/edges/{}/to",
                edges
                    .iter()
                    .position(|edge| edge.to() == &phase)
                    .unwrap_or(0)
            ),
            WorkflowError::DuplicateEdge { edge } => format!(
                "/operation/workflow/edges/{}",
                duplicate_index(edges, &edge)
            ),
            WorkflowError::UnknownCompletionPhase { phase } => format!(
                "/operation/workflow/completion_phases/{}",
                completion_phases
                    .iter()
                    .position(|candidate| candidate == &phase)
                    .unwrap_or(0)
            ),
            WorkflowError::DuplicateCompletionPhase { phase } => format!(
                "/operation/workflow/completion_phases/{}",
                duplicate_index(completion_phases, &phase)
            ),
        },
    };
    ErrorDetail::with_field(ErrorCode::InvalidWorkflow, path)
}

fn duplicate_index<T: Eq>(values: &[T], duplicate: &T) -> usize {
    values
        .iter()
        .enumerate()
        .filter(|(_, value)| *value == duplicate)
        .nth(1)
        .map_or(0, |(index, _)| index)
}

fn parse_filters(value: &RawJson) -> Result<ListFilters, ErrorDetail> {
    const PATH: &str = "/operation/filters";
    let entries = closed_object(value, PATH, &["workflow_id", "species", "phase", "status"])?;
    let workflow_id = parse_optional_text(entries, "workflow_id", PATH, |value| {
        WorkflowId::from_bytes(value.as_bytes()).map_err(|_| {
            ErrorDetail::with_field(
                ErrorCode::InvalidWorkflowId,
                "/operation/filters/workflow_id",
            )
        })
    })?;
    let species = parse_optional_text(entries, "species", PATH, |value| {
        IntentSpecies::from_bytes(value.as_bytes()).map_err(|_| {
            ErrorDetail::with_field(ErrorCode::InvalidSpecies, "/operation/filters/species")
        })
    })?;
    let phase = parse_optional_text(entries, "phase", PATH, |value| {
        PhaseId::from_bytes(value.as_bytes()).map_err(|_| {
            ErrorDetail::with_field(ErrorCode::InvalidPhaseId, "/operation/filters/phase")
        })
    })?;
    let status = match member(entries, "status") {
        None => None,
        Some(RawJson::String(value)) => Some(match value.as_str() {
            "active" => IntentUnitStatus::Active,
            "completed" => IntentUnitStatus::Completed,
            _ => {
                return Err(ErrorDetail::with_field(
                    ErrorCode::InvalidQuery,
                    "/operation/filters/status",
                ));
            }
        }),
        Some(_) => return Err(ErrorDetail::invalid_request("/operation/filters/status")),
    };
    Ok(ListFilters::new(workflow_id, species, phase, status))
}

fn parse_optional_text<T>(
    entries: &[(String, RawJson)],
    name: &str,
    path: &str,
    convert: impl FnOnce(&str) -> Result<T, ErrorDetail>,
) -> Result<Option<T>, ErrorDetail> {
    match member(entries, name) {
        None => Ok(None),
        Some(RawJson::String(value)) => convert(value).map(Some),
        Some(_) => Err(ErrorDetail::invalid_request(pointer_member(path, name))),
    }
}

fn parse_predicate(value: &RawJson) -> Result<DirectRelationshipPredicate, ErrorDetail> {
    const PATH: &str = "/operation/predicate";
    let RawJson::Object(entries) = value else {
        return Err(ErrorDetail::invalid_request(PATH));
    };
    ensure_unique_member(entries, "type", "/operation/predicate/type")?;
    let kind = match member(entries, "type") {
        Some(RawJson::String(value)) => value.as_str(),
        Some(_) | None => return Err(ErrorDetail::invalid_request("/operation/predicate/type")),
    };
    if !matches!(kind, "outgoing" | "incoming") {
        return Err(ErrorDetail::with_field(
            ErrorCode::InvalidQuery,
            "/operation/predicate/type",
        ));
    }
    let entries = closed_object(value, PATH, &["type", "definition", "anchor_id"])?;
    let definition = parse_backend_definition(
        required_member(entries, "definition", PATH)?,
        "/operation/predicate/definition",
        false,
    )?;
    let anchor = parse_required_uuid(entries, "anchor_id", PATH, ErrorCode::InvalidIntentUnitId)?;
    Ok(if kind == "outgoing" {
        DirectRelationshipPredicate::Outgoing { definition, anchor }
    } else {
        DirectRelationshipPredicate::Incoming { definition, anchor }
    })
}

fn parse_definition_parts(
    value: &RawJson,
    path: &str,
    cursor: bool,
) -> Result<(ReferenceNamespace, u64), ErrorDetail> {
    let entries = closed_object(value, path, &["id", "version"])?;
    let id_text = required_string(entries, "id", path)?;
    let invalid_id = if cursor {
        ErrorCode::InvalidCursor
    } else {
        ErrorCode::InvalidDefinitionId
    };
    let id = ReferenceNamespace::from_bytes(id_text.as_bytes())
        .map_err(|_| ErrorDetail::with_field(invalid_id, pointer_member(path, "id")))?;
    let version = parse_u64_text_member(
        entries,
        "version",
        path,
        if cursor {
            ErrorCode::InvalidCursor
        } else {
            ErrorCode::InvalidDefinitionVersion
        },
        true,
    )?;
    Ok((id, version))
}

fn parse_backend_definition(
    value: &RawJson,
    path: &str,
    cursor: bool,
) -> Result<BackendDefinitionKey, ErrorDetail> {
    let (id, version) = parse_definition_parts(value, path, cursor)?;
    Ok(BackendDefinitionKey::new(
        BackendDefinitionId::new(id.as_str())
            .expect("the backend and core definition ID grammars are identical"),
        BackendDefinitionVersion::new(version).expect("a decoded definition version is nonzero"),
    ))
}

fn parse_core_relationship(
    value: &RawJson,
    path: &str,
    cursor: bool,
) -> Result<CoreRelationship, ErrorDetail> {
    let entries = closed_object(value, path, &["definition", "source_id", "target_id"])?;
    let (id, version) = parse_definition_parts(
        required_member(entries, "definition", path)?,
        &pointer_member(path, "definition"),
        cursor,
    )?;
    let error = if cursor {
        ErrorCode::InvalidCursor
    } else {
        ErrorCode::InvalidIntentUnitId
    };
    let source = parse_uuid_value(
        required_member(entries, "source_id", path)?,
        &pointer_member(path, "source_id"),
        error,
    )?;
    let target = parse_uuid_value(
        required_member(entries, "target_id", path)?,
        &pointer_member(path, "target_id"),
        error,
    )?;
    Ok(CoreRelationship::new(
        CoreDefinitionKey::new(
            id,
            CoreDefinitionVersion::new(version).expect("decoded version is nonzero"),
        ),
        source,
        target,
    ))
}

fn parse_backend_relationship(
    value: &RawJson,
    path: &str,
    cursor: bool,
) -> Result<BackendRelationship, ErrorDetail> {
    let entries = closed_object(value, path, &["definition", "source_id", "target_id"])?;
    let definition = parse_backend_definition(
        required_member(entries, "definition", path)?,
        &pointer_member(path, "definition"),
        cursor,
    )?;
    let error = if cursor {
        ErrorCode::InvalidCursor
    } else {
        ErrorCode::InvalidIntentUnitId
    };
    let source = parse_uuid_value(
        required_member(entries, "source_id", path)?,
        &pointer_member(path, "source_id"),
        error,
    )?;
    let target = parse_uuid_value(
        required_member(entries, "target_id", path)?,
        &pointer_member(path, "target_id"),
        error,
    )?;
    Ok(BackendRelationship::new(definition, source, target))
}

fn parse_association(
    value: &RawJson,
    path: &str,
    cursor: bool,
) -> Result<RecordedAssociation, ErrorDetail> {
    let entries = closed_object(value, path, &["unit_id", "subject", "reference"])?;
    let error = if cursor {
        ErrorCode::InvalidCursor
    } else {
        ErrorCode::InvalidIntentUnitId
    };
    let unit_id = parse_uuid_value(
        required_member(entries, "unit_id", path)?,
        &pointer_member(path, "unit_id"),
        error,
    )?;
    let subject_path = pointer_member(path, "subject");
    let subject = parse_subject(
        required_member(entries, "subject", path)?,
        &subject_path,
        cursor,
    )?;
    let reference_path = pointer_member(path, "reference");
    let reference = parse_external_reference(
        required_member(entries, "reference", path)?,
        &reference_path,
        cursor,
    )?;
    Ok(RecordedAssociation::new(unit_id, subject, reference))
}

fn parse_subject(
    value: &RawJson,
    path: &str,
    cursor: bool,
) -> Result<AssociationSubject, ErrorDetail> {
    let RawJson::Object(entries) = value else {
        return Err(ErrorDetail::invalid_request(path));
    };
    let type_path = pointer_member(path, "type");
    ensure_unique_member(entries, "type", &type_path)?;
    let kind = match member(entries, "type") {
        Some(RawJson::String(value)) => value.as_str(),
        Some(_) | None => return Err(ErrorDetail::invalid_request(type_path)),
    };
    let semantic_code = if cursor {
        ErrorCode::InvalidCursor
    } else {
        ErrorCode::InvalidAssociationSubject
    };
    match kind {
        "whole_unit" => {
            closed_object(value, path, &["type"])?;
            Ok(AssociationSubject::WholeUnit)
        }
        "revision" => {
            let entries = closed_object(value, path, &["type", "revision"])?;
            let revision_path = pointer_member(path, "revision");
            let revision = match required_member(entries, "revision", path)? {
                RawJson::String(value) => {
                    parse_u64_text(value, &revision_path, semantic_code, false)?
                }
                _ => return Err(ErrorDetail::with_field(semantic_code, revision_path)),
            };
            Ok(AssociationSubject::Revision(revision))
        }
        _ => Err(ErrorDetail::with_field(semantic_code, type_path)),
    }
}

fn parse_policy(
    entries: &[(String, RawJson)],
    name: &str,
) -> Result<RelationshipPolicy, ErrorDetail> {
    let path = pointer_member("/operation", name);
    match member(entries, name) {
        Some(RawJson::String(value)) => match value.as_str() {
            "allow" => Ok(RelationshipPolicy::Allow),
            "reject" => Ok(RelationshipPolicy::Reject),
            _ => Err(ErrorDetail::with_field(
                ErrorCode::InvalidRelationshipPolicy,
                path,
            )),
        },
        Some(_) | None => Err(ErrorDetail::invalid_request(path)),
    }
}

fn parse_species_member(
    entries: &[(String, RawJson)],
    name: &str,
    parent: &str,
) -> Result<IntentSpecies, ErrorDetail> {
    let path = pointer_member(parent, name);
    let value = required_string(entries, name, parent)?;
    IntentSpecies::from_bytes(value.as_bytes())
        .map_err(|_| ErrorDetail::with_field(ErrorCode::InvalidSpecies, path))
}

fn parse_optional_species(
    entries: &[(String, RawJson)],
    name: &str,
    parent: &str,
) -> Result<Option<IntentSpecies>, ErrorDetail> {
    let path = pointer_member(parent, name);
    match member(entries, name) {
        None => Ok(None),
        Some(RawJson::String(value)) => IntentSpecies::from_bytes(value.as_bytes())
            .map(Some)
            .map_err(|_| ErrorDetail::with_field(ErrorCode::InvalidSpecies, path)),
        Some(_) => Err(ErrorDetail::invalid_request(path)),
    }
}

fn parse_page_limit(value: &RawJson) -> Result<PageLimit, ErrorDetail> {
    let integer = match value {
        RawJson::Unsigned(value) => usize::try_from(*value).ok(),
        RawJson::Signed(value) => usize::try_from(*value).ok(),
        _ => return Err(ErrorDetail::invalid_request("/operation/limit")),
    };
    integer
        .and_then(|value| PageLimit::new(value).ok())
        .ok_or_else(|| ErrorDetail::with_field(ErrorCode::InvalidQuery, "/operation/limit"))
}

fn parse_optional_cursor_uuid(
    entries: &[(String, RawJson)],
    name: &str,
) -> Result<Option<ListCursor>, ErrorDetail> {
    let path = pointer_member("/operation", name);
    match member(entries, name) {
        None => Ok(None),
        Some(RawJson::String(value)) => ListCursor::from_str(value)
            .map(Some)
            .map_err(|_| ErrorDetail::with_field(ErrorCode::InvalidCursor, path)),
        Some(_) => Err(ErrorDetail::invalid_request(path)),
    }
}

fn parse_required_uuid(
    entries: &[(String, RawJson)],
    name: &str,
    parent: &str,
    semantic_error: ErrorCode,
) -> Result<IntentUnitId, ErrorDetail> {
    let path = pointer_member(parent, name);
    parse_uuid_value(
        required_member(entries, name, parent)?,
        &path,
        semantic_error,
    )
}

fn parse_optional_uuid(
    entries: &[(String, RawJson)],
    name: &str,
    semantic_error: ErrorCode,
) -> Result<Option<IntentUnitId>, ErrorDetail> {
    let path = pointer_member("/operation", name);
    match member(entries, name) {
        None => Ok(None),
        Some(RawJson::String(value)) => parse_uuid(value, &path, semantic_error).map(Some),
        Some(_) => Err(ErrorDetail::invalid_request(path)),
    }
}

fn parse_uuid_value(
    value: &RawJson,
    path: &str,
    semantic_error: ErrorCode,
) -> Result<IntentUnitId, ErrorDetail> {
    match value {
        RawJson::String(value) => parse_uuid(value, path, semantic_error),
        _ => Err(ErrorDetail::invalid_request(path)),
    }
}

fn parse_uuid(
    value: &str,
    path: &str,
    semantic_error: ErrorCode,
) -> Result<IntentUnitId, ErrorDetail> {
    if value.len() != 36
        || !value.is_ascii()
        || !value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
            }
        })
    {
        return Err(ErrorDetail::with_field(semantic_error, path));
    }
    value
        .parse()
        .map_err(|_| ErrorDetail::with_field(semantic_error, path))
}

fn parse_u64_text_member(
    entries: &[(String, RawJson)],
    name: &str,
    parent: &str,
    semantic_error: ErrorCode,
    nonzero: bool,
) -> Result<u64, ErrorDetail> {
    let path = pointer_member(parent, name);
    match member(entries, name) {
        Some(RawJson::String(value)) => parse_u64_text(value, &path, semantic_error, nonzero),
        Some(_) | None => Err(ErrorDetail::invalid_request(path)),
    }
}

fn parse_u64_text(
    value: &str,
    path: &str,
    semantic_error: ErrorCode,
    nonzero: bool,
) -> Result<u64, ErrorDetail> {
    let canonical = value == "0"
        || (!value.is_empty()
            && !value.starts_with('0')
            && value.bytes().all(|byte| byte.is_ascii_digit()));
    let parsed = canonical.then(|| value.parse::<u64>().ok()).flatten();
    match parsed {
        Some(0) if nonzero => Err(ErrorDetail::with_field(semantic_error, path)),
        Some(value) => Ok(value),
        None => Err(ErrorDetail::with_field(semantic_error, path)),
    }
}

fn closed_object<'a>(
    value: &'a RawJson,
    path: &str,
    allowed: &[&str],
) -> Result<&'a [(String, RawJson)], ErrorDetail> {
    let RawJson::Object(entries) = value else {
        return Err(ErrorDetail::invalid_request(path));
    };
    let mut seen = Vec::with_capacity(entries.len());
    for (name, _) in entries {
        let member_path = pointer_member(path, name);
        if !allowed.contains(&name.as_str()) || seen.contains(&name) {
            return Err(ErrorDetail::invalid_request(member_path));
        }
        seen.push(name);
    }
    Ok(entries)
}

fn ensure_unique_member(
    entries: &[(String, RawJson)],
    name: &str,
    path: &str,
) -> Result<(), ErrorDetail> {
    if entries
        .iter()
        .filter(|(member_name, _)| member_name == name)
        .count()
        > 1
    {
        Err(ErrorDetail::invalid_request(path))
    } else {
        Ok(())
    }
}

fn required_member<'a>(
    entries: &'a [(String, RawJson)],
    name: &str,
    parent: &str,
) -> Result<&'a RawJson, ErrorDetail> {
    member(entries, name).ok_or_else(|| ErrorDetail::invalid_request(pointer_member(parent, name)))
}

fn required_string(
    entries: &[(String, RawJson)],
    name: &str,
    parent: &str,
) -> Result<String, ErrorDetail> {
    let path = pointer_member(parent, name);
    match member(entries, name) {
        Some(RawJson::String(value)) => Ok(value.clone()),
        Some(_) | None => Err(ErrorDetail::invalid_request(path)),
    }
}

fn member<'a>(entries: &'a [(String, RawJson)], name: &str) -> Option<&'a RawJson> {
    entries
        .iter()
        .find(|(member_name, _)| member_name == name)
        .map(|(_, value)| value)
}

fn pointer_member(parent: &str, name: &str) -> String {
    if name.contains('\0') {
        return parent.to_owned();
    }
    let escaped = name.replace('~', "~0").replace('/', "~1");
    let pointer = format!("{parent}/{escaped}");
    if pointer.len() <= 256 {
        pointer
    } else {
        parent.to_owned()
    }
}

#[derive(Debug)]
enum RawJson {
    Null,
    Bool,
    Signed(i64),
    Unsigned(u64),
    Float,
    String(String),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
}

impl<'de> Deserialize<'de> for RawJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(RawJsonVisitor)
    }
}

struct RawJsonVisitor;

impl<'de> Visitor<'de> for RawJsonVisitor {
    type Value = RawJson;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an RFC 8259 JSON value")
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(RawJson::Bool)
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(RawJson::Signed(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(RawJson::Unsigned(value))
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(RawJson::Float)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(RawJson::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(RawJson::String(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(RawJson::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(RawJson::Null)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::with_capacity(sequence.size_hint().unwrap_or(0));
        while let Some(value) = sequence.next_element()? {
            values.push(value);
        }
        Ok(RawJson::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut entries = Vec::with_capacity(map.size_hint().unwrap_or(0));
        while let Some(name) = map.next_key()? {
            entries.push((name, map.next_value()?));
        }
        Ok(RawJson::Object(entries))
    }
}

/// Closed local-v2 error vocabulary.  The wire spelling is fixed by the
/// schema and deliberately cannot be supplied by callers.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ErrorCode {
    MalformedJson,
    RequestTooLarge,
    InvalidRequest,
    UnsupportedProtocolVersion,
    InvalidIntentUnitId,
    InvalidExternalReference,
    InvalidSpecies,
    InvalidWorkflowId,
    InvalidPhaseId,
    InvalidWorkflow,
    InvalidRevision,
    InvalidDefinitionId,
    InvalidDefinitionVersion,
    InvalidRelationshipPolicy,
    InvalidAssociationSubject,
    InvalidQuery,
    InvalidCursor,
    // Frozen T-1113 erratum: coordinates are response-only, so callers cannot
    // construct this closed codec value through any request member.
    #[allow(dead_code)]
    InvalidCoordinate,
    InvalidRpcEndpoint,
    UnsupportedPlatform,
    InsecureProjectionPath,
    ProjectionBusy,
    UnsupportedSchemaVersion,
    UnsupportedEnvelopeVersion,
    CorruptSchema,
    CorruptEnvelope,
    ProjectionMismatch,
    RefreshRequired,
    ArchiveRpcUnavailable,
    ArchiveHistoryUnavailable,
    DeploymentMismatch,
    RuntimeMismatch,
    UnsupportedEventSchemaVersion,
    ConflictingFinalizedBlock,
    ProjectionError,
    DevSignerUnavailable,
    SubmissionLaneCorrupt,
    SubmissionLaneUnresolved,
    NonceConflict,
    InsufficientBalance,
    TransactionInvalid,
    RpcSubmissionRejected,
    SubmissionWatchLost,
    SubmissionTimeout,
    FinalizedInvariantFailed,
    ExpiredNotIncluded,
    UnsupportedCommandSchemaVersion,
    UnsignedCall,
    UnauthorizedSubmitter,
    DuplicateIntentUnit,
    IntentUnitNotFound,
    RevisionConflict,
    LifecycleHistoryCapacityExceeded,
    TransitionAlreadyCompleted,
    TransitionUnknownTarget,
    TransitionNotAllowed,
    CompletionAlreadyCompleted,
    CompletionPhaseNotEligible,
    GlobalSequenceExhausted,
    RelationshipDefinitionAlreadyExists,
    RelationshipDefinitionNotFound,
    RelationshipSourceNotFound,
    RelationshipTargetNotFound,
    RelationshipSourceSpeciesMismatch,
    RelationshipTargetSpeciesMismatch,
    SelfRelationshipRejected,
    DuplicateRelationship,
    CycleRejected,
    RelationshipCapacityExceeded,
    RelationshipNotFound,
    AssociationRevisionOutOfRange,
    DuplicateAssociation,
    AssociationCapacityExceeded,
    AssociationNotFound,
}

impl ErrorCode {
    #[cfg(test)]
    const ALL: [Self; 74] = [
        Self::MalformedJson,
        Self::RequestTooLarge,
        Self::InvalidRequest,
        Self::UnsupportedProtocolVersion,
        Self::InvalidIntentUnitId,
        Self::InvalidExternalReference,
        Self::InvalidSpecies,
        Self::InvalidWorkflowId,
        Self::InvalidPhaseId,
        Self::InvalidWorkflow,
        Self::InvalidRevision,
        Self::InvalidDefinitionId,
        Self::InvalidDefinitionVersion,
        Self::InvalidRelationshipPolicy,
        Self::InvalidAssociationSubject,
        Self::InvalidQuery,
        Self::InvalidCursor,
        Self::InvalidCoordinate,
        Self::InvalidRpcEndpoint,
        Self::UnsupportedPlatform,
        Self::InsecureProjectionPath,
        Self::ProjectionBusy,
        Self::UnsupportedSchemaVersion,
        Self::UnsupportedEnvelopeVersion,
        Self::CorruptSchema,
        Self::CorruptEnvelope,
        Self::ProjectionMismatch,
        Self::RefreshRequired,
        Self::ArchiveRpcUnavailable,
        Self::ArchiveHistoryUnavailable,
        Self::DeploymentMismatch,
        Self::RuntimeMismatch,
        Self::UnsupportedEventSchemaVersion,
        Self::ConflictingFinalizedBlock,
        Self::ProjectionError,
        Self::DevSignerUnavailable,
        Self::SubmissionLaneCorrupt,
        Self::SubmissionLaneUnresolved,
        Self::NonceConflict,
        Self::InsufficientBalance,
        Self::TransactionInvalid,
        Self::RpcSubmissionRejected,
        Self::SubmissionWatchLost,
        Self::SubmissionTimeout,
        Self::FinalizedInvariantFailed,
        Self::ExpiredNotIncluded,
        Self::UnsupportedCommandSchemaVersion,
        Self::UnsignedCall,
        Self::UnauthorizedSubmitter,
        Self::DuplicateIntentUnit,
        Self::IntentUnitNotFound,
        Self::RevisionConflict,
        Self::LifecycleHistoryCapacityExceeded,
        Self::TransitionAlreadyCompleted,
        Self::TransitionUnknownTarget,
        Self::TransitionNotAllowed,
        Self::CompletionAlreadyCompleted,
        Self::CompletionPhaseNotEligible,
        Self::GlobalSequenceExhausted,
        Self::RelationshipDefinitionAlreadyExists,
        Self::RelationshipDefinitionNotFound,
        Self::RelationshipSourceNotFound,
        Self::RelationshipTargetNotFound,
        Self::RelationshipSourceSpeciesMismatch,
        Self::RelationshipTargetSpeciesMismatch,
        Self::SelfRelationshipRejected,
        Self::DuplicateRelationship,
        Self::CycleRejected,
        Self::RelationshipCapacityExceeded,
        Self::RelationshipNotFound,
        Self::AssociationRevisionOutOfRange,
        Self::DuplicateAssociation,
        Self::AssociationCapacityExceeded,
        Self::AssociationNotFound,
    ];

    #[cfg(test)]
    const fn requires_field(self) -> bool {
        matches!(
            self,
            Self::InvalidRequest
                | Self::UnsupportedProtocolVersion
                | Self::InvalidIntentUnitId
                | Self::InvalidExternalReference
                | Self::InvalidSpecies
                | Self::InvalidWorkflowId
                | Self::InvalidPhaseId
                | Self::InvalidWorkflow
                | Self::InvalidRevision
                | Self::InvalidDefinitionId
                | Self::InvalidDefinitionVersion
                | Self::InvalidRelationshipPolicy
                | Self::InvalidAssociationSubject
                | Self::InvalidQuery
                | Self::InvalidCursor
                | Self::InvalidCoordinate
                | Self::InvalidRpcEndpoint
        )
    }

    const fn message(self) -> &'static str {
        match self {
            Self::MalformedJson => "request is not valid RFC 8259 JSON",
            Self::RequestTooLarge => "request exceeds the 1048576-byte limit",
            Self::InvalidRequest => "request does not match the local protocol v2 schema",
            Self::UnsupportedProtocolVersion => "protocol_version must be 2",
            Self::InvalidIntentUnitId => {
                "intent unit identifier must be a lowercase hyphenated RFC 4122 UUID"
            }
            Self::InvalidExternalReference => {
                "external reference must contain an exact bounded namespace, scope, and value"
            }
            Self::InvalidSpecies => "species must be nonblank NUL-free UTF-8 of at most 256 bytes",
            Self::InvalidWorkflowId => {
                "workflow identifier must be nonblank NUL-free UTF-8 of at most 256 bytes"
            }
            Self::InvalidPhaseId => {
                "phase identifier must be nonblank NUL-free UTF-8 of at most 256 bytes"
            }
            Self::InvalidWorkflow => "workflow topology is invalid",
            Self::InvalidRevision => "revision must be canonical unsigned decimal text within u64",
            Self::InvalidDefinitionId => {
                "definition identifier must use the canonical namespace grammar"
            }
            Self::InvalidDefinitionVersion => {
                "definition version must be canonical nonzero unsigned decimal text within u64"
            }
            Self::InvalidRelationshipPolicy => "relationship policy must be allow or reject",
            Self::InvalidAssociationSubject => {
                "association subject must be whole_unit or an exact revision"
            }
            Self::InvalidQuery => "query does not match the supported bounded query contract",
            Self::InvalidCursor => "cursor does not belong to the requested ordered query",
            Self::InvalidCoordinate => {
                "ledger coordinate is structurally invalid or fails its joined hash invariant"
            }
            Self::InvalidRpcEndpoint => {
                "RPC endpoint must be a canonical loopback ws URL with an explicit nondefault port"
            }
            Self::UnsupportedPlatform => "the required local platform boundary is unsupported",
            Self::InsecureProjectionPath => "projection path failed the local security boundary",
            Self::ProjectionBusy => "projection storage is busy",
            Self::UnsupportedSchemaVersion => "projection schema version is unsupported",
            Self::UnsupportedEnvelopeVersion => "stored envelope version is unsupported",
            Self::CorruptSchema => "projection schema is corrupt",
            Self::CorruptEnvelope => "stored envelope is corrupt",
            Self::ProjectionMismatch => "projection does not match the attested finalized archive",
            Self::RefreshRequired => "projection changed before the verified read was pinned",
            Self::ArchiveRpcUnavailable => {
                "archive RPC is unavailable or lacks unique local process evidence"
            }
            Self::ArchiveHistoryUnavailable => "required finalized archive history is unavailable",
            Self::DeploymentMismatch => {
                "archive deployment identity does not match the pinned deployment"
            }
            Self::RuntimeMismatch => {
                "archive runtime identity or response does not match the pinned runtime"
            }
            Self::UnsupportedEventSchemaVersion => "finalized event schema version is unsupported",
            Self::ConflictingFinalizedBlock => "finalized block history conflicts at one height",
            Self::ProjectionError => {
                "projection processing failed without a more specific safe classification"
            }
            Self::DevSignerUnavailable => "the selected named development signer is unavailable",
            Self::SubmissionLaneCorrupt => "the derived signer submission lane is corrupt",
            Self::SubmissionLaneUnresolved => {
                "the derived signer submission lane remains unresolved"
            }
            Self::NonceConflict => "canonical signer nonce could not be used safely",
            Self::InsufficientBalance => "canonical signer balance cannot pay the transaction",
            Self::TransactionInvalid => "transaction validity could not be proven",
            Self::RpcSubmissionRejected => "submission RPC failed after the send boundary",
            Self::SubmissionWatchLost => "submission watcher ended before a finalized outcome",
            Self::SubmissionTimeout => {
                "submission did not reach a finalized outcome within the bounded wait"
            }
            Self::FinalizedInvariantFailed => {
                "finalized inclusion violated the exact accepted-event invariant"
            }
            Self::ExpiredNotIncluded => {
                "the exact extrinsic hash was absent throughout its finalized mortal era"
            }
            Self::UnsupportedCommandSchemaVersion => {
                "runtime rejected the fixed command schema version"
            }
            Self::UnsignedCall => "runtime rejected an unsigned mutation call",
            Self::UnauthorizedSubmitter => "runtime rejected the development submitter",
            Self::DuplicateIntentUnit => "intent unit already exists",
            Self::IntentUnitNotFound => "intent unit was not found",
            Self::RevisionConflict => "expected revision does not match canonical parent state",
            Self::LifecycleHistoryCapacityExceeded => {
                "lifecycle history reached its fixed capacity"
            }
            Self::TransitionAlreadyCompleted => "cannot transition a completed intent unit",
            Self::TransitionUnknownTarget => "transition target is not declared by the workflow",
            Self::TransitionNotAllowed => "workflow does not allow this transition",
            Self::CompletionAlreadyCompleted => "cannot complete an already completed intent unit",
            Self::CompletionPhaseNotEligible => "current phase is not eligible for completion",
            Self::GlobalSequenceExhausted => "global event sequence is exhausted",
            Self::RelationshipDefinitionAlreadyExists => "relationship definition already exists",
            Self::RelationshipDefinitionNotFound => "relationship definition was not found",
            Self::RelationshipSourceNotFound => "relationship source intent unit was not found",
            Self::RelationshipTargetNotFound => "relationship target intent unit was not found",
            Self::RelationshipSourceSpeciesMismatch => {
                "relationship source species violates its definition"
            }
            Self::RelationshipTargetSpeciesMismatch => {
                "relationship target species violates its definition"
            }
            Self::SelfRelationshipRejected => "relationship definition rejects self relationships",
            Self::DuplicateRelationship => "relationship already exists",
            Self::CycleRejected => "relationship definition rejects the resulting cycle",
            Self::RelationshipCapacityExceeded => {
                "relationship definition reached its fixed edge capacity"
            }
            Self::RelationshipNotFound => "relationship was not found",
            Self::AssociationRevisionOutOfRange => {
                "association revision is outside the intent unit history"
            }
            Self::DuplicateAssociation => "association already exists",
            Self::AssociationCapacityExceeded => {
                "intent unit reached its fixed association capacity"
            }
            Self::AssociationNotFound => "association was not found",
        }
    }

    const fn generic_class(self) -> ResponseClass {
        match self {
            Self::MalformedJson
            | Self::RequestTooLarge
            | Self::InvalidRequest
            | Self::UnsupportedProtocolVersion
            | Self::InvalidIntentUnitId
            | Self::InvalidExternalReference
            | Self::InvalidSpecies
            | Self::InvalidWorkflowId
            | Self::InvalidPhaseId
            | Self::InvalidWorkflow
            | Self::InvalidRevision
            | Self::InvalidDefinitionId
            | Self::InvalidDefinitionVersion
            | Self::InvalidRelationshipPolicy
            | Self::InvalidAssociationSubject
            | Self::InvalidQuery
            | Self::InvalidCursor
            | Self::InvalidCoordinate
            | Self::InvalidRpcEndpoint => ResponseClass::RequestRejected,
            Self::DevSignerUnavailable | Self::SubmissionLaneCorrupt => {
                ResponseClass::OperationalFailure
            }
            Self::IntentUnitNotFound | Self::RelationshipDefinitionNotFound => {
                ResponseClass::DomainRejected
            }
            _ => ResponseClass::EnvironmentFailure,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ErrorDetail {
    code: ErrorCode,
    message: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actual_revision: Option<String>,
}

impl ErrorDetail {
    #[must_use]
    pub(crate) fn plain(code: ErrorCode) -> Self {
        Self {
            code,
            message: code.message(),
            field: None,
            expected_revision: None,
            actual_revision: None,
        }
    }

    #[must_use]
    pub(crate) fn with_field(code: ErrorCode, field: impl Into<String>) -> Self {
        Self {
            field: Some(field.into()),
            ..Self::plain(code)
        }
    }

    #[must_use]
    pub(crate) fn revision_conflict(expected_revision: u64, actual_revision: u64) -> Self {
        Self {
            code: ErrorCode::RevisionConflict,
            message: ErrorCode::RevisionConflict.message(),
            field: None,
            expected_revision: Some(expected_revision.to_string()),
            actual_revision: Some(actual_revision.to_string()),
        }
    }

    fn invalid_request(field: impl Into<String>) -> Self {
        Self::with_field(ErrorCode::InvalidRequest, field)
    }

    #[must_use]
    pub(crate) const fn code(&self) -> ErrorCode {
        self.code
    }
}

#[derive(Serialize)]
struct GenericErrorResponse<'a> {
    protocol_version: u8,
    outcome: &'static str,
    error: &'a ErrorDetail,
}

#[must_use]
pub(crate) fn generic_error_response(error: ErrorDetail) -> ExecutedRequest {
    let class = error.code().generic_class();
    ExecutedRequest::encode(
        class,
        &GenericErrorResponse {
            protocol_version: PROTOCOL_VERSION,
            outcome: "error",
            error: &error,
        },
    )
}

#[must_use]
pub(crate) fn request_too_large_response() -> ExecutedRequest {
    generic_error_response(ErrorDetail::plain(ErrorCode::RequestTooLarge))
}

#[derive(Serialize)]
struct ReadSuccessResponse<R> {
    protocol_version: u8,
    outcome: &'static str,
    result: R,
}

fn read_success(result: ReadResultWire<'_>) -> ExecutedRequest {
    ExecutedRequest::encode(
        ResponseClass::Success,
        &ReadSuccessResponse {
            protocol_version: PROTOCOL_VERSION,
            outcome: "success",
            result,
        },
    )
}

#[derive(Serialize)]
#[serde(untagged)]
enum ReadResultWire<'a> {
    IntentUnit(IntentUnitResultWire<'a>),
    IntentUnitPage(IntentUnitPageResultWire<'a>),
    RelationshipDefinition(RelationshipDefinitionResultWire<'a>),
    RelationshipPage(RelationshipPageResultWire),
    ProjectionPage(ProjectionPageResultWire<'a>),
    AssociationPage(AssociationPageResultWire<'a>),
}

#[derive(Serialize)]
struct IntentUnitResultWire<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    intent_unit: ProjectedUnitWire<'a>,
    checkpoint: ProjectionCheckpointWire,
}

#[derive(Serialize)]
struct IntentUnitPageResultWire<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    items: Vec<UnitSummaryWire<'a>>,
    next_cursor: Option<String>,
    checkpoint: ProjectionCheckpointWire,
}

#[derive(Serialize)]
struct RelationshipDefinitionResultWire<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    definition: ProjectedDefinitionWire<'a>,
    checkpoint: ProjectionCheckpointWire,
}

#[derive(Serialize)]
struct RelationshipPageResultWire {
    #[serde(rename = "type")]
    kind: &'static str,
    items: Vec<ProjectedRelationshipWire>,
    next_cursor: Option<RelationshipKeyWire>,
    checkpoint: ProjectionCheckpointWire,
}

#[derive(Serialize)]
struct ProjectionPageResultWire<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    query_version: u8,
    items: Vec<UnitSummaryWire<'a>>,
    next_cursor: Option<String>,
    checkpoint: ProjectionCheckpointWire,
}

#[derive(Serialize)]
struct AssociationPageResultWire<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    direction: &'static str,
    items: Vec<ProjectedAssociationWire<'a>>,
    next_cursor: Option<AssociationKeyWire<'a>>,
    checkpoint: ProjectionCheckpointWire,
}

#[derive(Serialize)]
struct ProjectedUnitWire<'a> {
    id: String,
    origin: ExternalReferenceWire<'a>,
    species: &'a str,
    workflow: WorkflowWire<'a>,
    phase: &'a str,
    status: &'static str,
    revision: String,
    history: Vec<HistoryRecordWire<'a>>,
    last_coordinate: LedgerCoordinateWire,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum HistoryRecordWire<'a> {
    Transition {
        sequence: String,
        from: &'a str,
        to: &'a str,
    },
    Completion {
        sequence: String,
        phase: &'a str,
    },
}

#[derive(Serialize)]
struct UnitSummaryWire<'a> {
    id: String,
    origin: ExternalReferenceWire<'a>,
    species: &'a str,
    workflow_id: &'a str,
    phase: &'a str,
    status: &'static str,
    revision: String,
    last_coordinate: LedgerCoordinateWire,
}

#[derive(Serialize)]
struct DefinitionKeyWire {
    id: String,
    version: String,
}

#[derive(Serialize)]
struct ProjectedDefinitionWire<'a> {
    key: DefinitionKeyWire,
    directed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_species: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_species: Option<&'a str>,
    self_policy: &'static str,
    cycle_policy: &'static str,
    created_coordinate: LedgerCoordinateWire,
}

#[derive(Serialize)]
struct RelationshipKeyWire {
    definition: DefinitionKeyWire,
    source_id: String,
    target_id: String,
}

#[derive(Serialize)]
struct ProjectedRelationshipWire {
    key: RelationshipKeyWire,
    created_coordinate: LedgerCoordinateWire,
}

#[derive(Serialize)]
struct AssociationSubjectWire {
    #[serde(rename = "type")]
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    revision: Option<String>,
}

#[derive(Serialize)]
struct AssociationKeyWire<'a> {
    unit_id: String,
    subject: AssociationSubjectWire,
    reference: ExternalReferenceWire<'a>,
}

#[derive(Serialize)]
struct ExternalReferenceWire<'a> {
    namespace: &'a str,
    scope: &'a str,
    value: &'a str,
}

#[derive(Serialize)]
struct WorkflowWire<'a> {
    id: &'a str,
    phases: Vec<&'a str>,
    initial_phase: &'a str,
    edges: Vec<WorkflowEdgeWire<'a>>,
    completion_phases: Vec<&'a str>,
}

#[derive(Serialize)]
struct WorkflowEdgeWire<'a> {
    from: &'a str,
    to: &'a str,
}

#[derive(Serialize)]
struct ProjectedAssociationWire<'a> {
    key: AssociationKeyWire<'a>,
    created_coordinate: LedgerCoordinateWire,
}

#[derive(Serialize)]
struct LedgerCoordinateWire {
    parachain_genesis_hash: String,
    deployment_id: String,
    block_number: String,
    block_hash: String,
    extrinsic_index: u32,
    extrinsic_hash: String,
    system_event_index: u32,
    global_sequence: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct ProjectionCheckpointWire {
    block_number: String,
    block_hash: String,
    last_global_sequence: Option<String>,
    runtime_spec_version: u32,
    runtime_code_hash: String,
}

impl<'a> From<&'a ProjectedUnit> for ProjectedUnitWire<'a> {
    fn from(projected: &'a ProjectedUnit) -> Self {
        let unit = projected.intent_unit();
        Self {
            id: unit.id().to_string(),
            origin: external_reference_wire(unit.origin()),
            species: unit.species().as_str(),
            workflow: workflow_wire(unit.workflow()),
            phase: unit.phase().as_str(),
            status: status_name(unit.status()),
            revision: unit.revision().value().to_string(),
            history: unit.history().iter().map(HistoryRecordWire::from).collect(),
            last_coordinate: projected.last_coordinate().into(),
        }
    }
}

impl<'a> From<&'a LifecycleRecord> for HistoryRecordWire<'a> {
    fn from(record: &'a LifecycleRecord) -> Self {
        match record {
            LifecycleRecord::Transition(record) => Self::Transition {
                sequence: record.sequence().to_string(),
                from: record.from().as_str(),
                to: record.to().as_str(),
            },
            LifecycleRecord::Completion(record) => Self::Completion {
                sequence: record.sequence().to_string(),
                phase: record.final_phase().as_str(),
            },
        }
    }
}

impl<'a> From<&'a ProjectedUnitSummary> for UnitSummaryWire<'a> {
    fn from(unit: &'a ProjectedUnitSummary) -> Self {
        Self {
            id: unit.id().to_string(),
            origin: external_reference_wire(unit.origin()),
            species: unit.species().as_str(),
            workflow_id: unit.workflow_id().as_str(),
            phase: unit.phase().as_str(),
            status: status_name(unit.status()),
            revision: unit.revision().value().to_string(),
            last_coordinate: unit.last_coordinate().into(),
        }
    }
}

impl<'a> From<&'a ProjectedDefinition> for ProjectedDefinitionWire<'a> {
    fn from(projected: &'a ProjectedDefinition) -> Self {
        let definition = projected.definition();
        Self {
            key: definition_key_wire(definition.key()),
            directed: projected.directed(),
            source_species: definition.source_species().map(IntentSpecies::as_str),
            target_species: definition.target_species().map(IntentSpecies::as_str),
            self_policy: relationship_policy_name(definition.self_policy()),
            cycle_policy: relationship_policy_name(definition.cycle_policy()),
            created_coordinate: projected.created_coordinate().into(),
        }
    }
}

impl From<&ProjectedRelationship> for ProjectedRelationshipWire {
    fn from(projected: &ProjectedRelationship) -> Self {
        Self {
            key: core_relationship_wire(projected.key()),
            created_coordinate: projected.created_coordinate().into(),
        }
    }
}

impl<'a> From<&'a RecordedAssociation> for AssociationKeyWire<'a> {
    fn from(association: &'a RecordedAssociation) -> Self {
        Self {
            unit_id: association.unit_id().to_string(),
            subject: association_subject_wire(association.subject()),
            reference: external_reference_wire(association.reference()),
        }
    }
}

impl<'a> From<&'a ProjectedAssociation> for ProjectedAssociationWire<'a> {
    fn from(projected: &'a ProjectedAssociation) -> Self {
        Self {
            key: projected.key().into(),
            created_coordinate: projected.created_coordinate().into(),
        }
    }
}

impl From<&LedgerCoordinate> for LedgerCoordinateWire {
    fn from(coordinate: &LedgerCoordinate) -> Self {
        Self {
            parachain_genesis_hash: hex32(coordinate.parachain_genesis_hash()),
            deployment_id: hex32(coordinate.deployment_id()),
            block_number: coordinate.block_number().to_string(),
            block_hash: hex32(coordinate.block_hash()),
            extrinsic_index: coordinate.extrinsic_index(),
            extrinsic_hash: hex32(coordinate.extrinsic_hash()),
            system_event_index: coordinate.system_event_index(),
            global_sequence: coordinate.global_sequence().get().to_string(),
        }
    }
}

impl From<&ProjectionCheckpoint> for ProjectionCheckpointWire {
    fn from(checkpoint: &ProjectionCheckpoint) -> Self {
        Self {
            block_number: checkpoint.block_number().to_string(),
            block_hash: hex32(checkpoint.block_hash()),
            last_global_sequence: checkpoint
                .last_global_sequence()
                .map(|sequence| sequence.get().to_string()),
            runtime_spec_version: checkpoint.runtime_spec_version(),
            runtime_code_hash: hex32(checkpoint.runtime_code_hash()),
        }
    }
}

#[must_use]
pub(crate) fn intent_unit_response(result: &ProjectedUnitResult) -> ExecutedRequest {
    read_success(ReadResultWire::IntentUnit(IntentUnitResultWire {
        kind: "intent_unit",
        intent_unit: result.intent_unit().into(),
        checkpoint: result.checkpoint().into(),
    }))
}

#[must_use]
pub(crate) fn intent_unit_page_response(result: &ProjectedUnitPage) -> ExecutedRequest {
    read_success(ReadResultWire::IntentUnitPage(IntentUnitPageResultWire {
        kind: "intent_unit_page",
        items: result.items().iter().map(UnitSummaryWire::from).collect(),
        next_cursor: result.next_cursor().map(|cursor| cursor.to_string()),
        checkpoint: result.checkpoint().into(),
    }))
}

#[must_use]
pub(crate) fn relationship_definition_response(
    result: &ProjectedDefinitionResult,
) -> ExecutedRequest {
    read_success(ReadResultWire::RelationshipDefinition(
        RelationshipDefinitionResultWire {
            kind: "relationship_definition",
            definition: result.definition().into(),
            checkpoint: result.checkpoint().into(),
        },
    ))
}

#[must_use]
pub(crate) fn relationship_page_response(result: &ProjectedRelationshipPage) -> ExecutedRequest {
    read_success(ReadResultWire::RelationshipPage(
        RelationshipPageResultWire {
            kind: "relationship_page",
            items: result
                .items()
                .iter()
                .map(ProjectedRelationshipWire::from)
                .collect(),
            next_cursor: result
                .next_cursor()
                .map(|cursor| backend_relationship_wire(cursor.relationship())),
            checkpoint: result.checkpoint().into(),
        },
    ))
}

#[must_use]
pub(crate) fn projection_page_response(result: &ProjectedProjectionPage) -> ExecutedRequest {
    read_success(ReadResultWire::ProjectionPage(ProjectionPageResultWire {
        kind: "projection_v1_page",
        query_version: 1,
        items: result.items().iter().map(UnitSummaryWire::from).collect(),
        next_cursor: result.next_cursor().map(|cursor| cursor.to_string()),
        checkpoint: result.checkpoint().into(),
    }))
}

#[must_use]
pub(crate) fn association_page_response(result: &AssociationPage) -> ExecutedRequest {
    read_success(ReadResultWire::AssociationPage(AssociationPageResultWire {
        kind: "association_page",
        direction: match result.direction() {
            AssociationDirection::ByUnit => "by_unit",
            AssociationDirection::ByReference => "by_reference",
        },
        items: result
            .items()
            .iter()
            .map(ProjectedAssociationWire::from)
            .collect(),
        next_cursor: result.next_cursor().map(AssociationKeyWire::from),
        checkpoint: result.checkpoint().into(),
    }))
}

fn status_name(status: IntentUnitStatus) -> &'static str {
    match status {
        IntentUnitStatus::Active => "active",
        IntentUnitStatus::Completed => "completed",
    }
}

fn relationship_policy_name(policy: RelationshipPolicy) -> &'static str {
    match policy {
        RelationshipPolicy::Allow => "allow",
        RelationshipPolicy::Reject => "reject",
    }
}

fn definition_key_wire(key: &CoreDefinitionKey) -> DefinitionKeyWire {
    DefinitionKeyWire {
        id: key.id().as_str().to_owned(),
        version: key.version().value().to_string(),
    }
}

fn backend_definition_key_wire(key: &BackendDefinitionKey) -> DefinitionKeyWire {
    DefinitionKeyWire {
        id: key.id().as_str().to_owned(),
        version: key.version().value().to_string(),
    }
}

fn core_relationship_wire(relationship: &CoreRelationship) -> RelationshipKeyWire {
    RelationshipKeyWire {
        definition: definition_key_wire(relationship.definition()),
        source_id: relationship.source().to_string(),
        target_id: relationship.target().to_string(),
    }
}

fn backend_relationship_wire(relationship: &BackendRelationship) -> RelationshipKeyWire {
    RelationshipKeyWire {
        definition: backend_definition_key_wire(relationship.definition()),
        source_id: relationship.source().to_string(),
        target_id: relationship.target().to_string(),
    }
}

fn external_reference_wire(reference: &ExternalReference) -> ExternalReferenceWire<'_> {
    ExternalReferenceWire {
        namespace: reference.namespace().as_str(),
        scope: reference.scope().as_str(),
        value: reference.value().as_str(),
    }
}

fn workflow_wire(workflow: &Workflow) -> WorkflowWire<'_> {
    WorkflowWire {
        id: workflow.id().as_str(),
        phases: workflow.phases().iter().map(PhaseId::as_str).collect(),
        initial_phase: workflow.initial_phase().as_str(),
        edges: workflow
            .edges()
            .iter()
            .map(|edge| WorkflowEdgeWire {
                from: edge.from().as_str(),
                to: edge.to().as_str(),
            })
            .collect(),
        completion_phases: workflow
            .completion_phases()
            .iter()
            .map(PhaseId::as_str)
            .collect(),
    }
}

fn association_subject_wire(subject: AssociationSubject) -> AssociationSubjectWire {
    match subject {
        AssociationSubject::WholeUnit => AssociationSubjectWire {
            kind: "whole_unit",
            revision: None,
        },
        AssociationSubject::Revision(revision) => AssociationSubjectWire {
            kind: "revision",
            revision: Some(revision.to_string()),
        },
    }
}

fn hex32(bytes: &[u8; 32]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(66);
    encoded.push_str("0x");
    for byte in bytes {
        encoded.push(DIGITS[usize::from(byte >> 4)] as char);
        encoded.push(DIGITS[usize::from(byte & 0x0f)] as char);
    }
    encoded
}

/// Projection state reported after an accepted mutation.  Construction is
/// closed so the wire always contains either a checkpoint-backed `caught_up`
/// state or an explicitly nullable `lagging` state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AcceptedProjection {
    CaughtUp(ProjectionCheckpoint),
    Lagging(Option<ProjectionCheckpoint>),
}

impl AcceptedProjection {
    #[must_use]
    pub(crate) const fn caught_up(checkpoint: ProjectionCheckpoint) -> Self {
        Self::CaughtUp(checkpoint)
    }

    #[must_use]
    pub(crate) const fn lagging(checkpoint: Option<ProjectionCheckpoint>) -> Self {
        Self::Lagging(checkpoint)
    }
}

#[derive(Serialize)]
struct SubmissionRejectedResponse<'a> {
    protocol_version: u8,
    outcome: &'static str,
    operation: &'static str,
    error: &'a ErrorDetail,
}

#[derive(Serialize)]
struct PendingSubmissionResponse<'a> {
    protocol_version: u8,
    outcome: &'static str,
    operation: &'static str,
    expected_extrinsic_hash: String,
    era: MortalEraWire,
    error: &'a ErrorDetail,
}

#[derive(Serialize)]
struct FinalizedRejectedResponse<'a> {
    protocol_version: u8,
    outcome: &'static str,
    operation: &'static str,
    finalized_extrinsic: FinalizedExtrinsicWire,
    error: &'a ErrorDetail,
}

#[derive(Serialize)]
struct FinalizedAcceptedResponse<'a> {
    protocol_version: u8,
    outcome: &'static str,
    operation: &'static str,
    coordinate: LedgerCoordinateWire,
    effect: AcceptedEffectWire<'a>,
    projection: AcceptedProjectionWire,
}

#[derive(Serialize)]
#[serde(untagged)]
enum SubmissionResponseWire<'a> {
    SubmissionRejected(SubmissionRejectedResponse<'a>),
    Pending(PendingSubmissionResponse<'a>),
    FinalizedRejected(FinalizedRejectedResponse<'a>),
    FinalizedAccepted(Box<FinalizedAcceptedResponse<'a>>),
}

fn encode_submission_response(
    class: ResponseClass,
    response: SubmissionResponseWire<'_>,
) -> ExecutedRequest {
    ExecutedRequest::encode(class, &response)
}

#[derive(Serialize)]
struct MortalEraWire {
    birth: String,
    death: String,
}

#[derive(Serialize)]
struct FinalizedExtrinsicWire {
    parachain_genesis_hash: String,
    deployment_id: String,
    block_number: String,
    block_hash: String,
    extrinsic_index: u32,
    extrinsic_hash: String,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AcceptedEffectWire<'a> {
    UnitCreated {
        unit_id: String,
        committed_revision: String,
    },
    UnitTransitioned {
        unit_id: String,
        committed_revision: String,
    },
    UnitCompleted {
        unit_id: String,
        committed_revision: String,
    },
    RelationshipDefinitionCreated {
        definition: DefinitionKeyWire,
    },
    RelationshipCreated {
        relationship: RelationshipKeyWire,
    },
    RelationshipDeleted {
        relationship: RelationshipKeyWire,
    },
    AssociationRecorded {
        association: AssociationKeyWire<'a>,
    },
    AssociationRevoked {
        association: AssociationKeyWire<'a>,
    },
}

#[derive(Serialize)]
struct AcceptedProjectionWire {
    status: &'static str,
    checkpoint: Option<ProjectionCheckpointWire>,
}

impl From<MortalEra> for MortalEraWire {
    fn from(era: MortalEra) -> Self {
        Self {
            birth: era.birth().to_string(),
            death: era.death().to_string(),
        }
    }
}

impl<'a> From<&'a AcceptedEffect> for AcceptedEffectWire<'a> {
    fn from(effect: &'a AcceptedEffect) -> Self {
        match effect {
            AcceptedEffect::UnitCreated {
                unit_id,
                committed_revision,
            } => Self::UnitCreated {
                unit_id: unit_id.to_string(),
                committed_revision: committed_revision.to_string(),
            },
            AcceptedEffect::UnitTransitioned {
                unit_id,
                committed_revision,
            } => Self::UnitTransitioned {
                unit_id: unit_id.to_string(),
                committed_revision: committed_revision.to_string(),
            },
            AcceptedEffect::UnitCompleted {
                unit_id,
                committed_revision,
            } => Self::UnitCompleted {
                unit_id: unit_id.to_string(),
                committed_revision: committed_revision.to_string(),
            },
            AcceptedEffect::RelationshipDefinitionCreated(definition) => {
                Self::RelationshipDefinitionCreated {
                    definition: definition_key_wire(definition.key()),
                }
            }
            AcceptedEffect::RelationshipCreated(relationship) => Self::RelationshipCreated {
                relationship: core_relationship_wire(relationship),
            },
            AcceptedEffect::RelationshipDeleted(relationship) => Self::RelationshipDeleted {
                relationship: core_relationship_wire(relationship),
            },
            AcceptedEffect::AssociationRecorded(association) => Self::AssociationRecorded {
                association: association.into(),
            },
            AcceptedEffect::AssociationRevoked(association) => Self::AssociationRevoked {
                association: association.into(),
            },
        }
    }
}

impl From<AcceptedProjection> for AcceptedProjectionWire {
    fn from(projection: AcceptedProjection) -> Self {
        match projection {
            AcceptedProjection::CaughtUp(checkpoint) => Self {
                status: "caught_up",
                checkpoint: Some((&checkpoint).into()),
            },
            AcceptedProjection::Lagging(checkpoint) => Self {
                status: "lagging",
                checkpoint: checkpoint.as_ref().map(ProjectionCheckpointWire::from),
            },
        }
    }
}

#[must_use]
pub(crate) fn submission_response(
    identity: &DeploymentIdentity,
    outcome: &SubmissionOutcome,
    projection: Option<AcceptedProjection>,
) -> ExecutedRequest {
    let operation = mutation_operation_name(outcome.operation());
    match outcome.kind() {
        SubmissionOutcomeKind::SubmissionRejected => {
            let error = submission_error_detail(outcome);
            encode_submission_response(
                ResponseClass::DomainRejected,
                SubmissionResponseWire::SubmissionRejected(SubmissionRejectedResponse {
                    protocol_version: PROTOCOL_VERSION,
                    outcome: "submission_rejected",
                    operation,
                    error: &error,
                }),
            )
        }
        SubmissionOutcomeKind::SubmissionLaneUnresolved => pending_submission_response(
            "submission_lane_unresolved",
            operation,
            outcome,
            ErrorCode::SubmissionLaneUnresolved,
        ),
        SubmissionOutcomeKind::ExpiredNotIncluded => pending_submission_response(
            "expired_not_included",
            operation,
            outcome,
            ErrorCode::ExpiredNotIncluded,
        ),
        SubmissionOutcomeKind::FinalizedDispatchRejected => {
            let error = submission_error_detail(outcome);
            encode_submission_response(
                ResponseClass::DomainRejected,
                SubmissionResponseWire::FinalizedRejected(FinalizedRejectedResponse {
                    protocol_version: PROTOCOL_VERSION,
                    outcome: "finalized_dispatch_rejected",
                    operation,
                    finalized_extrinsic: finalized_extrinsic_wire(
                        identity,
                        outcome
                            .finalized_extrinsic()
                            .expect("dispatch rejection must retain its finalized extrinsic"),
                    ),
                    error: &error,
                }),
            )
        }
        SubmissionOutcomeKind::FinalizedInvariantFailed => {
            let error = ErrorDetail::plain(ErrorCode::FinalizedInvariantFailed);
            encode_submission_response(
                ResponseClass::OperationalFailure,
                SubmissionResponseWire::FinalizedRejected(FinalizedRejectedResponse {
                    protocol_version: PROTOCOL_VERSION,
                    outcome: "finalized_invariant_failed",
                    operation,
                    finalized_extrinsic: finalized_extrinsic_wire(
                        identity,
                        outcome
                            .finalized_extrinsic()
                            .expect("finalized invariant failure must retain its extrinsic"),
                    ),
                    error: &error,
                }),
            )
        }
        SubmissionOutcomeKind::DeliveryIndeterminate => {
            let error = submission_error_detail(outcome);
            pending_submission_response_with_error(
                "delivery_indeterminate",
                operation,
                outcome,
                &error,
            )
        }
        SubmissionOutcomeKind::FinalizedAccepted => {
            let coordinate = outcome
                .coordinate()
                .expect("accepted outcome must retain its accepted coordinate");
            let effect = outcome
                .effect()
                .expect("accepted outcome must retain its accepted effect");
            encode_submission_response(
                ResponseClass::Success,
                SubmissionResponseWire::FinalizedAccepted(Box::new(FinalizedAcceptedResponse {
                    protocol_version: PROTOCOL_VERSION,
                    outcome: "finalized_accepted",
                    operation,
                    coordinate: accepted_coordinate_wire(identity, coordinate),
                    effect: effect.into(),
                    projection: projection
                        .unwrap_or_else(|| AcceptedProjection::lagging(None))
                        .into(),
                })),
            )
        }
    }
}

fn pending_submission_response(
    outcome_name: &'static str,
    operation: &'static str,
    outcome: &SubmissionOutcome,
    code: ErrorCode,
) -> ExecutedRequest {
    let error = ErrorDetail::plain(code);
    pending_submission_response_with_error(outcome_name, operation, outcome, &error)
}

fn pending_submission_response_with_error(
    outcome_name: &'static str,
    operation: &'static str,
    outcome: &SubmissionOutcome,
    error: &ErrorDetail,
) -> ExecutedRequest {
    encode_submission_response(
        ResponseClass::OperationalFailure,
        SubmissionResponseWire::Pending(PendingSubmissionResponse {
            protocol_version: PROTOCOL_VERSION,
            outcome: outcome_name,
            operation,
            expected_extrinsic_hash: hex32(
                &outcome
                    .expected_extrinsic_hash()
                    .expect("pending outcome must retain its extrinsic hash"),
            ),
            era: outcome
                .era()
                .expect("pending outcome must retain its mortal era")
                .into(),
            error,
        }),
    )
}

fn submission_error_detail(outcome: &SubmissionOutcome) -> ErrorDetail {
    if let Some(detail) = outcome.revision_conflict() {
        return ErrorDetail::revision_conflict(
            detail.expected_revision(),
            detail.actual_revision(),
        );
    }
    ErrorDetail::plain(submission_failure_code(
        outcome
            .failure_code()
            .expect("failed submission outcome must retain its error code"),
    ))
}

fn submission_failure_code(code: SubmissionFailureCode) -> ErrorCode {
    match code {
        SubmissionFailureCode::NonceConflict => ErrorCode::NonceConflict,
        SubmissionFailureCode::InsufficientBalance => ErrorCode::InsufficientBalance,
        SubmissionFailureCode::TransactionInvalid => ErrorCode::TransactionInvalid,
        SubmissionFailureCode::RpcSubmissionRejected => ErrorCode::RpcSubmissionRejected,
        SubmissionFailureCode::SubmissionWatchLost => ErrorCode::SubmissionWatchLost,
        SubmissionFailureCode::SubmissionTimeout => ErrorCode::SubmissionTimeout,
        SubmissionFailureCode::SubmissionLaneUnresolved => ErrorCode::SubmissionLaneUnresolved,
        SubmissionFailureCode::FinalizedInvariantFailed => ErrorCode::FinalizedInvariantFailed,
        SubmissionFailureCode::ExpiredNotIncluded => ErrorCode::ExpiredNotIncluded,
        SubmissionFailureCode::RuntimeMismatch => ErrorCode::RuntimeMismatch,
        SubmissionFailureCode::UnsupportedCommandSchemaVersion => {
            ErrorCode::UnsupportedCommandSchemaVersion
        }
        SubmissionFailureCode::UnsignedCall => ErrorCode::UnsignedCall,
        SubmissionFailureCode::UnauthorizedSubmitter => ErrorCode::UnauthorizedSubmitter,
        SubmissionFailureCode::DuplicateIntentUnit => ErrorCode::DuplicateIntentUnit,
        SubmissionFailureCode::IntentUnitNotFound => ErrorCode::IntentUnitNotFound,
        SubmissionFailureCode::RevisionConflict => ErrorCode::RevisionConflict,
        SubmissionFailureCode::LifecycleHistoryCapacityExceeded => {
            ErrorCode::LifecycleHistoryCapacityExceeded
        }
        SubmissionFailureCode::TransitionAlreadyCompleted => ErrorCode::TransitionAlreadyCompleted,
        SubmissionFailureCode::TransitionUnknownTarget => ErrorCode::TransitionUnknownTarget,
        SubmissionFailureCode::TransitionNotAllowed => ErrorCode::TransitionNotAllowed,
        SubmissionFailureCode::CompletionAlreadyCompleted => ErrorCode::CompletionAlreadyCompleted,
        SubmissionFailureCode::CompletionPhaseNotEligible => ErrorCode::CompletionPhaseNotEligible,
        SubmissionFailureCode::GlobalSequenceExhausted => ErrorCode::GlobalSequenceExhausted,
        SubmissionFailureCode::RelationshipDefinitionAlreadyExists => {
            ErrorCode::RelationshipDefinitionAlreadyExists
        }
        SubmissionFailureCode::RelationshipDefinitionNotFound => {
            ErrorCode::RelationshipDefinitionNotFound
        }
        SubmissionFailureCode::RelationshipSourceNotFound => ErrorCode::RelationshipSourceNotFound,
        SubmissionFailureCode::RelationshipTargetNotFound => ErrorCode::RelationshipTargetNotFound,
        SubmissionFailureCode::RelationshipSourceSpeciesMismatch => {
            ErrorCode::RelationshipSourceSpeciesMismatch
        }
        SubmissionFailureCode::RelationshipTargetSpeciesMismatch => {
            ErrorCode::RelationshipTargetSpeciesMismatch
        }
        SubmissionFailureCode::SelfRelationshipRejected => ErrorCode::SelfRelationshipRejected,
        SubmissionFailureCode::DuplicateRelationship => ErrorCode::DuplicateRelationship,
        SubmissionFailureCode::CycleRejected => ErrorCode::CycleRejected,
        SubmissionFailureCode::RelationshipCapacityExceeded => {
            ErrorCode::RelationshipCapacityExceeded
        }
        SubmissionFailureCode::RelationshipNotFound => ErrorCode::RelationshipNotFound,
        SubmissionFailureCode::AssociationRevisionOutOfRange => {
            ErrorCode::AssociationRevisionOutOfRange
        }
        SubmissionFailureCode::DuplicateAssociation => ErrorCode::DuplicateAssociation,
        SubmissionFailureCode::AssociationCapacityExceeded => {
            ErrorCode::AssociationCapacityExceeded
        }
        SubmissionFailureCode::AssociationNotFound => ErrorCode::AssociationNotFound,
    }
}

fn mutation_operation_name(operation: MutationOperation) -> &'static str {
    match operation {
        MutationOperation::CreateUnit => "create_intent_unit",
        MutationOperation::TransitionUnit => "transition_intent_unit",
        MutationOperation::CompleteUnit => "complete_intent_unit",
        MutationOperation::CreateRelationshipDefinition => "create_relationship_definition",
        MutationOperation::CreateRelationship => "create_relationship",
        MutationOperation::DeleteRelationship => "delete_relationship",
        MutationOperation::RecordAssociation => "record_association",
        MutationOperation::RevokeAssociation => "revoke_association",
    }
}

fn finalized_extrinsic_wire(
    identity: &DeploymentIdentity,
    finalized: FinalizedExtrinsic,
) -> FinalizedExtrinsicWire {
    FinalizedExtrinsicWire {
        parachain_genesis_hash: hex32(identity.parachain_genesis_hash()),
        deployment_id: hex32(identity.deployment_id()),
        block_number: finalized.block_number().to_string(),
        block_hash: hex32(&finalized.block_hash()),
        extrinsic_index: finalized.extrinsic_index(),
        extrinsic_hash: hex32(&finalized.extrinsic_hash()),
    }
}

fn accepted_coordinate_wire(
    identity: &DeploymentIdentity,
    coordinate: AcceptedCoordinate,
) -> LedgerCoordinateWire {
    let finalized = coordinate.finalized_extrinsic();
    LedgerCoordinateWire {
        parachain_genesis_hash: hex32(identity.parachain_genesis_hash()),
        deployment_id: hex32(identity.deployment_id()),
        block_number: finalized.block_number().to_string(),
        block_hash: hex32(&finalized.block_hash()),
        extrinsic_index: finalized.extrinsic_index(),
        extrinsic_hash: hex32(&finalized.extrinsic_hash()),
        system_event_index: coordinate.system_event_index(),
        global_sequence: coordinate.global_sequence().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, collections::BTreeSet, fs, path::Path};

    use serde_json::Value;

    use super::*;

    const MANIFEST_PATH: &str = "tests/fixtures/protocol-v2/cubikan-local/manifest-v1.json";
    const INVENTORY_PATH: &str = "tests/fixtures/protocol-v2/cubikan-local/inventory-v1.json";

    struct ManifestIdSource {
        id: IntentUnitId,
        calls: Cell<usize>,
    }

    impl ManifestIdSource {
        fn new(id: IntentUnitId) -> Self {
            Self {
                id,
                calls: Cell::new(0),
            }
        }
    }

    impl IdSource for ManifestIdSource {
        fn generate(&mut self) -> IntentUnitId {
            self.calls.set(self.calls.get() + 1);
            self.id
        }
    }

    #[test]
    fn local_v2_semantic_response_dtos_are_byte_exact() {
        let root = workspace_root();
        let inventory: Value = serde_json::from_slice(
            &fs::read(root.join(INVENTORY_PATH)).expect("read frozen local-v2 inventory"),
        )
        .expect("parse frozen local-v2 inventory");
        let manifest: Value = serde_json::from_slice(
            &fs::read(root.join(MANIFEST_PATH)).expect("read frozen local-v2 manifest"),
        )
        .expect("parse frozen local-v2 manifest");
        let semantic = &inventory["semantic_spine"];
        let ids = array(semantic, "case_ids");
        assert_eq!(semantic["positive_operation_cells"].as_u64(), Some(15));
        assert_eq!(semantic["error_code_cells"].as_u64(), Some(79));
        assert_eq!(semantic["total"].as_u64(), Some(94));
        assert_eq!(ids.len(), 94, "semantic spine case count drifted");
        assert_production_error_registry(&inventory);

        let manifest_cases = array(&manifest, "cases");
        let mut read_responses = 0_usize;
        let mut generic_errors = 0_usize;
        let mut mutation_responses = 0_usize;
        let mut mutation_outcomes = BTreeSet::new();
        let mut invalid_coordinate = 0_usize;

        for id in ids {
            let id = id.as_str().expect("semantic case ID must be text");
            let (value, expected) = stdout_fixture(id);
            let response = match text(&value, "outcome") {
                "success" => {
                    read_responses += 1;
                    read_success(read_result_fixture(&value["result"]))
                }
                "error" => {
                    generic_errors += 1;
                    if text(&value["error"], "code") == "invalid_coordinate" {
                        invalid_coordinate += 1;
                    }
                    generic_error_response(error_fixture(&value["error"]))
                }
                outcome => {
                    mutation_responses += 1;
                    mutation_outcomes.insert(outcome.to_owned());
                    let (class, wire) = submission_fixture(&value);
                    encode_submission_response(class, wire)
                }
            };
            let expected_exit = manifest_case(manifest_cases, id)["exit_code"]
                .as_u64()
                .expect("manifest exit code must be u64") as u8;
            assert_eq!(
                response.class().exit_code(),
                expected_exit,
                "{id}: response class"
            );
            assert_eq!(response.body(), expected.as_slice(), "{id}: response bytes");
        }

        assert_eq!(read_responses, 7, "all read result tags must be replayed");
        assert_eq!(
            generic_errors, 39,
            "all generic error cells must be replayed"
        );
        assert_eq!(
            mutation_responses, 48,
            "all mutation response cells must be replayed"
        );
        assert_eq!(
            mutation_outcomes,
            BTreeSet::from(
                [
                    "delivery_indeterminate",
                    "expired_not_included",
                    "finalized_accepted",
                    "finalized_dispatch_rejected",
                    "finalized_invariant_failed",
                    "submission_lane_unresolved",
                    "submission_rejected",
                ]
                .map(str::to_owned)
            ),
            "all seven mutation envelopes must be replayed"
        );
        assert_eq!(
            invalid_coordinate, 1,
            "response-only invalid_coordinate must remain a closed codec cell"
        );
    }

    fn assert_production_error_registry(inventory: &Value) {
        let registry = array(inventory, "error_registry");
        assert_eq!(registry.len(), ErrorCode::ALL.len());
        for code in ErrorCode::ALL {
            let wire_name = error_code_name(code);
            let cell = registry
                .iter()
                .find(|cell| text(cell, "code") == wire_name)
                .unwrap_or_else(|| panic!("missing frozen error registry cell {wire_name}"));
            assert_eq!(
                text(cell, "message"),
                code.message(),
                "{wire_name}: production message drifted"
            );
            assert_eq!(
                text(cell, "field_policy") == "required",
                code.requires_field(),
                "{wire_name}: field legality drifted"
            );
            assert_eq!(
                text(cell, "revision_pair") == "required",
                code == ErrorCode::RevisionConflict,
                "{wire_name}: revision-pair legality drifted"
            );
        }
    }

    fn stdout_fixture(id: &str) -> (Value, Vec<u8>) {
        let mut bytes = fs::read(
            workspace_root()
                .join("tests/fixtures/protocol-v2/cubikan-local/stdout")
                .join(format!("{id}.jsonl")),
        )
        .unwrap_or_else(|error| panic!("read {id} stdout fixture: {error}"));
        assert_eq!(bytes.pop(), Some(b'\n'), "{id}: fixture must end in LF");
        let value = serde_json::from_slice(&bytes)
            .unwrap_or_else(|error| panic!("parse {id} stdout fixture: {error}"));
        (value, bytes)
    }

    fn read_result_fixture(result: &Value) -> ReadResultWire<'_> {
        match text(result, "type") {
            "intent_unit" => ReadResultWire::IntentUnit(IntentUnitResultWire {
                kind: "intent_unit",
                intent_unit: projected_unit_fixture(&result["intent_unit"]),
                checkpoint: checkpoint_fixture(&result["checkpoint"]),
            }),
            "intent_unit_page" => ReadResultWire::IntentUnitPage(IntentUnitPageResultWire {
                kind: "intent_unit_page",
                items: array(result, "items").iter().map(summary_fixture).collect(),
                next_cursor: optional_text(result, "next_cursor").map(str::to_owned),
                checkpoint: checkpoint_fixture(&result["checkpoint"]),
            }),
            "relationship_definition" => {
                ReadResultWire::RelationshipDefinition(RelationshipDefinitionResultWire {
                    kind: "relationship_definition",
                    definition: projected_definition_fixture(&result["definition"]),
                    checkpoint: checkpoint_fixture(&result["checkpoint"]),
                })
            }
            "relationship_page" => ReadResultWire::RelationshipPage(RelationshipPageResultWire {
                kind: "relationship_page",
                items: array(result, "items")
                    .iter()
                    .map(projected_relationship_fixture)
                    .collect(),
                next_cursor: optional_value(result, "next_cursor").map(relationship_key_fixture),
                checkpoint: checkpoint_fixture(&result["checkpoint"]),
            }),
            "projection_v1_page" => ReadResultWire::ProjectionPage(ProjectionPageResultWire {
                kind: "projection_v1_page",
                query_version: u8::try_from(number(result, "query_version"))
                    .expect("query version fits u8"),
                items: array(result, "items").iter().map(summary_fixture).collect(),
                next_cursor: optional_text(result, "next_cursor").map(str::to_owned),
                checkpoint: checkpoint_fixture(&result["checkpoint"]),
            }),
            "association_page" => ReadResultWire::AssociationPage(AssociationPageResultWire {
                kind: "association_page",
                direction: fixed_direction(text(result, "direction")),
                items: array(result, "items")
                    .iter()
                    .map(projected_association_fixture)
                    .collect(),
                next_cursor: optional_value(result, "next_cursor").map(association_key_fixture),
                checkpoint: checkpoint_fixture(&result["checkpoint"]),
            }),
            kind => panic!("unexpected fixture read result {kind}"),
        }
    }

    fn projected_unit_fixture(unit: &Value) -> ProjectedUnitWire<'_> {
        ProjectedUnitWire {
            id: text(unit, "id").to_owned(),
            origin: external_reference_fixture(&unit["origin"]),
            species: text(unit, "species"),
            workflow: workflow_fixture(&unit["workflow"]),
            phase: text(unit, "phase"),
            status: fixed_status(text(unit, "status")),
            revision: text(unit, "revision").to_owned(),
            history: array(unit, "history").iter().map(history_fixture).collect(),
            last_coordinate: coordinate_fixture(&unit["last_coordinate"]),
        }
    }

    fn history_fixture(record: &Value) -> HistoryRecordWire<'_> {
        match text(record, "type") {
            "transition" => HistoryRecordWire::Transition {
                sequence: text(record, "sequence").to_owned(),
                from: text(record, "from"),
                to: text(record, "to"),
            },
            "completion" => HistoryRecordWire::Completion {
                sequence: text(record, "sequence").to_owned(),
                phase: text(record, "phase"),
            },
            kind => panic!("unexpected history kind {kind}"),
        }
    }

    fn summary_fixture(unit: &Value) -> UnitSummaryWire<'_> {
        UnitSummaryWire {
            id: text(unit, "id").to_owned(),
            origin: external_reference_fixture(&unit["origin"]),
            species: text(unit, "species"),
            workflow_id: text(unit, "workflow_id"),
            phase: text(unit, "phase"),
            status: fixed_status(text(unit, "status")),
            revision: text(unit, "revision").to_owned(),
            last_coordinate: coordinate_fixture(&unit["last_coordinate"]),
        }
    }

    fn projected_definition_fixture(definition: &Value) -> ProjectedDefinitionWire<'_> {
        ProjectedDefinitionWire {
            key: definition_key_fixture(&definition["key"]),
            directed: definition["directed"]
                .as_bool()
                .expect("directed must be boolean"),
            source_species: optional_text(definition, "source_species"),
            target_species: optional_text(definition, "target_species"),
            self_policy: fixed_policy(text(definition, "self_policy")),
            cycle_policy: fixed_policy(text(definition, "cycle_policy")),
            created_coordinate: coordinate_fixture(&definition["created_coordinate"]),
        }
    }

    fn projected_relationship_fixture(value: &Value) -> ProjectedRelationshipWire {
        ProjectedRelationshipWire {
            key: relationship_key_fixture(&value["key"]),
            created_coordinate: coordinate_fixture(&value["created_coordinate"]),
        }
    }

    fn projected_association_fixture(value: &Value) -> ProjectedAssociationWire<'_> {
        ProjectedAssociationWire {
            key: association_key_fixture(&value["key"]),
            created_coordinate: coordinate_fixture(&value["created_coordinate"]),
        }
    }

    fn submission_fixture(value: &Value) -> (ResponseClass, SubmissionResponseWire<'_>) {
        let operation = fixed_operation(text(value, "operation"));
        match text(value, "outcome") {
            "submission_rejected" => {
                let error = Box::leak(Box::new(error_fixture(&value["error"])));
                (
                    ResponseClass::DomainRejected,
                    SubmissionResponseWire::SubmissionRejected(SubmissionRejectedResponse {
                        protocol_version: PROTOCOL_VERSION,
                        outcome: "submission_rejected",
                        operation,
                        error,
                    }),
                )
            }
            outcome @ ("submission_lane_unresolved"
            | "expired_not_included"
            | "delivery_indeterminate") => {
                let error = Box::leak(Box::new(error_fixture(&value["error"])));
                (
                    ResponseClass::OperationalFailure,
                    SubmissionResponseWire::Pending(PendingSubmissionResponse {
                        protocol_version: PROTOCOL_VERSION,
                        outcome: fixed_pending_outcome(outcome),
                        operation,
                        expected_extrinsic_hash: text(value, "expected_extrinsic_hash").to_owned(),
                        era: MortalEraWire {
                            birth: text(&value["era"], "birth").to_owned(),
                            death: text(&value["era"], "death").to_owned(),
                        },
                        error,
                    }),
                )
            }
            outcome @ ("finalized_dispatch_rejected" | "finalized_invariant_failed") => {
                let error = Box::leak(Box::new(error_fixture(&value["error"])));
                let class = if outcome == "finalized_dispatch_rejected" {
                    ResponseClass::DomainRejected
                } else {
                    ResponseClass::OperationalFailure
                };
                (
                    class,
                    SubmissionResponseWire::FinalizedRejected(FinalizedRejectedResponse {
                        protocol_version: PROTOCOL_VERSION,
                        outcome: fixed_finalized_outcome(outcome),
                        operation,
                        finalized_extrinsic: finalized_extrinsic_fixture(
                            &value["finalized_extrinsic"],
                        ),
                        error,
                    }),
                )
            }
            "finalized_accepted" => (
                ResponseClass::Success,
                SubmissionResponseWire::FinalizedAccepted(Box::new(FinalizedAcceptedResponse {
                    protocol_version: PROTOCOL_VERSION,
                    outcome: "finalized_accepted",
                    operation,
                    coordinate: coordinate_fixture(&value["coordinate"]),
                    effect: accepted_effect_fixture(&value["effect"]),
                    projection: accepted_projection_fixture(&value["projection"]),
                })),
            ),
            outcome => panic!("unexpected fixture submission outcome {outcome}"),
        }
    }

    fn error_fixture(value: &Value) -> ErrorDetail {
        let code = fixture_error_code(text(value, "code"));
        assert_eq!(
            text(value, "message"),
            code.message(),
            "{}: fixture must use the production message",
            error_code_name(code)
        );
        let field = optional_text(value, "field");
        let expected_revision = optional_text(value, "expected_revision");
        let actual_revision = optional_text(value, "actual_revision");

        if code == ErrorCode::RevisionConflict {
            assert!(field.is_none(), "revision_conflict forbids field");
            return ErrorDetail::revision_conflict(
                expected_revision
                    .expect("revision_conflict requires expected_revision")
                    .parse()
                    .expect("expected_revision must be u64 text"),
                actual_revision
                    .expect("revision_conflict requires actual_revision")
                    .parse()
                    .expect("actual_revision must be u64 text"),
            );
        }

        assert!(
            expected_revision.is_none() && actual_revision.is_none(),
            "{} forbids revision members",
            error_code_name(code)
        );
        if code.requires_field() {
            ErrorDetail::with_field(code, field.expect("field error requires field"))
        } else {
            assert!(field.is_none(), "{} forbids field", error_code_name(code));
            ErrorDetail::plain(code)
        }
    }

    fn accepted_effect_fixture(effect: &Value) -> AcceptedEffectWire<'_> {
        match text(effect, "type") {
            "unit_created" => AcceptedEffectWire::UnitCreated {
                unit_id: text(effect, "unit_id").to_owned(),
                committed_revision: text(effect, "committed_revision").to_owned(),
            },
            "unit_transitioned" => AcceptedEffectWire::UnitTransitioned {
                unit_id: text(effect, "unit_id").to_owned(),
                committed_revision: text(effect, "committed_revision").to_owned(),
            },
            "unit_completed" => AcceptedEffectWire::UnitCompleted {
                unit_id: text(effect, "unit_id").to_owned(),
                committed_revision: text(effect, "committed_revision").to_owned(),
            },
            "relationship_definition_created" => {
                AcceptedEffectWire::RelationshipDefinitionCreated {
                    definition: definition_key_fixture(&effect["definition"]),
                }
            }
            "relationship_created" => AcceptedEffectWire::RelationshipCreated {
                relationship: relationship_key_fixture(&effect["relationship"]),
            },
            "relationship_deleted" => AcceptedEffectWire::RelationshipDeleted {
                relationship: relationship_key_fixture(&effect["relationship"]),
            },
            "association_recorded" => AcceptedEffectWire::AssociationRecorded {
                association: association_key_fixture(&effect["association"]),
            },
            "association_revoked" => AcceptedEffectWire::AssociationRevoked {
                association: association_key_fixture(&effect["association"]),
            },
            kind => panic!("unexpected accepted effect {kind}"),
        }
    }

    fn accepted_projection_fixture(value: &Value) -> AcceptedProjectionWire {
        AcceptedProjectionWire {
            status: fixed_projection_status(text(value, "status")),
            checkpoint: optional_value(value, "checkpoint").map(checkpoint_fixture),
        }
    }

    fn external_reference_fixture(value: &Value) -> ExternalReferenceWire<'_> {
        ExternalReferenceWire {
            namespace: text(value, "namespace"),
            scope: text(value, "scope"),
            value: text(value, "value"),
        }
    }

    fn workflow_fixture(value: &Value) -> WorkflowWire<'_> {
        WorkflowWire {
            id: text(value, "id"),
            phases: array(value, "phases")
                .iter()
                .map(|phase| phase.as_str().expect("phase must be text"))
                .collect(),
            initial_phase: text(value, "initial_phase"),
            edges: array(value, "edges")
                .iter()
                .map(|edge| WorkflowEdgeWire {
                    from: text(edge, "from"),
                    to: text(edge, "to"),
                })
                .collect(),
            completion_phases: array(value, "completion_phases")
                .iter()
                .map(|phase| phase.as_str().expect("completion phase must be text"))
                .collect(),
        }
    }

    fn definition_key_fixture(value: &Value) -> DefinitionKeyWire {
        DefinitionKeyWire {
            id: text(value, "id").to_owned(),
            version: text(value, "version").to_owned(),
        }
    }

    fn relationship_key_fixture(value: &Value) -> RelationshipKeyWire {
        RelationshipKeyWire {
            definition: definition_key_fixture(&value["definition"]),
            source_id: text(value, "source_id").to_owned(),
            target_id: text(value, "target_id").to_owned(),
        }
    }

    fn association_key_fixture(value: &Value) -> AssociationKeyWire<'_> {
        AssociationKeyWire {
            unit_id: text(value, "unit_id").to_owned(),
            subject: association_subject_fixture(&value["subject"]),
            reference: external_reference_fixture(&value["reference"]),
        }
    }

    fn association_subject_fixture(value: &Value) -> AssociationSubjectWire {
        AssociationSubjectWire {
            kind: fixed_subject(text(value, "type")),
            revision: optional_text(value, "revision").map(str::to_owned),
        }
    }

    fn coordinate_fixture(value: &Value) -> LedgerCoordinateWire {
        LedgerCoordinateWire {
            parachain_genesis_hash: text(value, "parachain_genesis_hash").to_owned(),
            deployment_id: text(value, "deployment_id").to_owned(),
            block_number: text(value, "block_number").to_owned(),
            block_hash: text(value, "block_hash").to_owned(),
            extrinsic_index: u32::try_from(number(value, "extrinsic_index"))
                .expect("extrinsic index fits u32"),
            extrinsic_hash: text(value, "extrinsic_hash").to_owned(),
            system_event_index: u32::try_from(number(value, "system_event_index"))
                .expect("event index fits u32"),
            global_sequence: text(value, "global_sequence").to_owned(),
        }
    }

    fn finalized_extrinsic_fixture(value: &Value) -> FinalizedExtrinsicWire {
        FinalizedExtrinsicWire {
            parachain_genesis_hash: text(value, "parachain_genesis_hash").to_owned(),
            deployment_id: text(value, "deployment_id").to_owned(),
            block_number: text(value, "block_number").to_owned(),
            block_hash: text(value, "block_hash").to_owned(),
            extrinsic_index: u32::try_from(number(value, "extrinsic_index"))
                .expect("extrinsic index fits u32"),
            extrinsic_hash: text(value, "extrinsic_hash").to_owned(),
        }
    }

    fn checkpoint_fixture(value: &Value) -> ProjectionCheckpointWire {
        ProjectionCheckpointWire {
            block_number: text(value, "block_number").to_owned(),
            block_hash: text(value, "block_hash").to_owned(),
            last_global_sequence: optional_text(value, "last_global_sequence").map(str::to_owned),
            runtime_spec_version: u32::try_from(number(value, "runtime_spec_version"))
                .expect("runtime spec version fits u32"),
            runtime_code_hash: text(value, "runtime_code_hash").to_owned(),
        }
    }

    fn text<'a>(value: &'a Value, field: &str) -> &'a str {
        value[field]
            .as_str()
            .unwrap_or_else(|| panic!("{field} must be text"))
    }

    fn optional_text<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
        optional_value(value, field).map(|value| {
            value
                .as_str()
                .unwrap_or_else(|| panic!("{field} must be text or null"))
        })
    }

    fn optional_value<'a>(value: &'a Value, field: &str) -> Option<&'a Value> {
        value.get(field).filter(|value| !value.is_null())
    }

    fn array<'a>(value: &'a Value, field: &str) -> &'a [Value] {
        value[field]
            .as_array()
            .unwrap_or_else(|| panic!("{field} must be an array"))
    }

    fn number(value: &Value, field: &str) -> u64 {
        value[field]
            .as_u64()
            .unwrap_or_else(|| panic!("{field} must be u64"))
    }

    fn fixed_status(value: &str) -> &'static str {
        match value {
            "active" => "active",
            "completed" => "completed",
            _ => panic!("unexpected status {value}"),
        }
    }

    fn fixed_policy(value: &str) -> &'static str {
        match value {
            "allow" => "allow",
            "reject" => "reject",
            _ => panic!("unexpected policy {value}"),
        }
    }

    fn fixed_direction(value: &str) -> &'static str {
        match value {
            "by_unit" => "by_unit",
            "by_reference" => "by_reference",
            _ => panic!("unexpected direction {value}"),
        }
    }

    fn fixed_subject(value: &str) -> &'static str {
        match value {
            "whole_unit" => "whole_unit",
            "revision" => "revision",
            _ => panic!("unexpected subject {value}"),
        }
    }

    fn fixed_projection_status(value: &str) -> &'static str {
        match value {
            "caught_up" => "caught_up",
            "lagging" => "lagging",
            _ => panic!("unexpected projection status {value}"),
        }
    }

    fn fixed_operation(value: &str) -> &'static str {
        match value {
            "create_intent_unit" => "create_intent_unit",
            "transition_intent_unit" => "transition_intent_unit",
            "complete_intent_unit" => "complete_intent_unit",
            "create_relationship_definition" => "create_relationship_definition",
            "create_relationship" => "create_relationship",
            "delete_relationship" => "delete_relationship",
            "record_association" => "record_association",
            "revoke_association" => "revoke_association",
            _ => panic!("unexpected operation {value}"),
        }
    }

    fn fixed_pending_outcome(value: &str) -> &'static str {
        match value {
            "submission_lane_unresolved" => "submission_lane_unresolved",
            "expired_not_included" => "expired_not_included",
            "delivery_indeterminate" => "delivery_indeterminate",
            _ => panic!("unexpected pending outcome {value}"),
        }
    }

    fn fixed_finalized_outcome(value: &str) -> &'static str {
        match value {
            "finalized_dispatch_rejected" => "finalized_dispatch_rejected",
            "finalized_invariant_failed" => "finalized_invariant_failed",
            _ => panic!("unexpected finalized outcome {value}"),
        }
    }

    fn fixture_error_code(value: &str) -> ErrorCode {
        ErrorCode::ALL
            .into_iter()
            .find(|code| error_code_name(*code) == value)
            .unwrap_or_else(|| panic!("unexpected semantic fixture error code {value}"))
    }

    fn error_code_name(code: ErrorCode) -> String {
        serde_json::to_value(code)
            .expect("closed ErrorCode must serialize")
            .as_str()
            .expect("closed ErrorCode must serialize as text")
            .to_owned()
    }

    fn manifest_case<'a>(cases: &'a [Value], id: &str) -> &'a Value {
        cases
            .iter()
            .find(|case| case["id"] == id)
            .unwrap_or_else(|| panic!("manifest is missing semantic case {id}"))
    }

    #[test]
    fn test_local_v2_rejects_invalid_shape_before_any_state_access() {
        const POSITIVE_CURSOR_CASES: [&str; 5] = [
            "shape_valid_list_intent_units_after",
            "shape_valid_list_relationships_after",
            "shape_valid_project_intent_units_v1_after",
            "shape_valid_list_associations_by_unit_after",
            "shape_valid_list_associations_by_reference_after",
        ];

        let root = workspace_root();
        let inventory: Value = serde_json::from_slice(
            &fs::read(root.join(INVENTORY_PATH)).expect("read frozen local-v2 inventory"),
        )
        .expect("parse frozen local-v2 inventory");
        let manifest: Value = serde_json::from_slice(
            &fs::read(root.join(MANIFEST_PATH)).expect("read frozen local-v2 manifest"),
        )
        .expect("parse frozen local-v2 manifest");
        let cases = manifest["cases"]
            .as_array()
            .expect("manifest cases must be an array");
        let expected_case_count = inventory["semantic_spine"]["total"]
            .as_u64()
            .expect("semantic total must be u64")
            + inventory["structural_corpus"]["total"]
                .as_u64()
                .expect("structural total must be u64");
        assert_eq!(
            u64::try_from(cases.len()).expect("manifest length fits u64"),
            expected_case_count,
            "manifest and inventory totals drifted"
        );

        let mut replayed = 0usize;
        let mut decoder_rejections = 0usize;
        let expected_positive_cursors = BTreeSet::from(POSITIVE_CURSOR_CASES);
        let mut decoded_positive_cursors = BTreeSet::new();
        for case in cases {
            let id = case["id"].as_str().expect("case ID must be text");
            if id == "error_request_too_large" {
                // This one is rejected by bounded ingress before the decoder.
                continue;
            }

            let request_path = case["request"]["path"]
                .as_str()
                .expect("request path must be text");
            let stdout_path = case["stdout"]["path"]
                .as_str()
                .expect("stdout path must be text");
            let request = fs::read(root.join(request_path)).expect("read frozen request bytes");
            let expected_stdout =
                fs::read(root.join(stdout_path)).expect("read frozen stdout bytes");
            let expected_value: Value =
                serde_json::from_slice(&expected_stdout).expect("stdout must contain JSON and LF");
            let expected_code = expected_value["error"]["code"].as_str();
            let must_reject_in_decoder = expected_value["outcome"] == "error"
                && expected_code.is_some_and(is_decoder_error_code);

            let generated_id = case["context"]["generated_uuid"]
                .as_str()
                .unwrap_or("00112233-4455-4677-8899-aabbccddeeff")
                .parse()
                .expect("manifest generated UUID must be valid");
            let mut ids = ManifestIdSource::new(generated_id);
            let decoded = decode_request_from_source(&request, &mut ids);

            if must_reject_in_decoder {
                let error = decoded.unwrap_err_or_else(|| {
                    panic!("{id}: decoder accepted a frozen rejection request")
                });
                let response = generic_error_response(error);
                assert_eq!(
                    response.class().exit_code(),
                    case["exit_code"].as_u64().expect("exit code must be u64") as u8,
                    "{id}: response class drifted"
                );
                assert_eq!(
                    response.body(),
                    expected_stdout
                        .strip_suffix(b"\n")
                        .expect("stdout oracle must end in exactly one LF"),
                    "{id}: byte-exact decoder response drifted"
                );
                assert_eq!(ids.calls.get(), 0, "{id}: generated an ID after rejection");
                decoder_rejections += 1;
            } else {
                decoded.unwrap_or_else(|error| {
                    panic!("{id}: valid request rejected as {:?}", error.code())
                });
                if expected_positive_cursors.contains(id) {
                    decoded_positive_cursors.insert(id);
                }
            }
            replayed += 1;
        }

        assert_eq!(
            replayed,
            cases.len() - 1,
            "every non-oversize request must be decoded"
        );
        assert_eq!(
            decoder_rejections, 184,
            "frozen decoder-rejection registry drifted"
        );
        assert_eq!(
            decoded_positive_cursors, expected_positive_cursors,
            "all five structurally valid cursor requests must reach typed decoding"
        );
    }

    fn is_decoder_error_code(code: &str) -> bool {
        matches!(
            code,
            "malformed_json"
                | "invalid_request"
                | "unsupported_protocol_version"
                | "invalid_intent_unit_id"
                | "invalid_external_reference"
                | "invalid_species"
                | "invalid_workflow_id"
                | "invalid_phase_id"
                | "invalid_workflow"
                | "invalid_revision"
                | "invalid_definition_id"
                | "invalid_definition_version"
                | "invalid_relationship_policy"
                | "invalid_association_subject"
                | "invalid_query"
                | "invalid_cursor"
        )
    }

    fn workspace_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("local crate must live at workspace/crates/cubikan-local")
    }

    trait ResultTestExt<T, E> {
        fn unwrap_err_or_else(self, on_ok: impl FnOnce() -> E) -> E;
    }

    impl<T, E> ResultTestExt<T, E> for Result<T, E> {
        fn unwrap_err_or_else(self, on_ok: impl FnOnce() -> E) -> E {
            match self {
                Ok(_) => on_ok(),
                Err(error) => error,
            }
        }
    }
}
