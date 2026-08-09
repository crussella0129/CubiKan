use std::str::FromStr;

use cubikan_backend::{
    CompleteIntentUnit, CreateIntentUnit, IntentUnitPage, IntentUnitSummary, IntentUnitView,
    ListCursor, ListFilters, ListIntentUnits, MutationResult, PageLimit, TransitionIntentUnit,
};
use cubikan_core::{
    IntentSpecies, IntentUnitId, IntentUnitRevision, IntentUnitStatus, LifecycleRecord, PhaseId,
    Workflow, WorkflowEdge, WorkflowId,
};
use serde::{Deserialize, Deserializer, Serialize, de::IgnoredAny};

/// Version selected by the local JSON request and response contract.
pub const PROTOCOL_VERSION: u64 = 1;

/// Modeled outcome class used by the process runner's eventual exit mapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponseClass {
    /// The requested backend operation succeeded.
    Success,
    /// JSON, structure, version, or semantic field validation rejected input.
    RequestRejected,
    /// The command was valid but the aggregate rejected it.
    CommandRejected,
    /// The local durable store rejected or could not perform the operation.
    StorageRejected,
}

/// One already-encoded modeled response and its outcome class.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutedRequest {
    class: ResponseClass,
    body: Vec<u8>,
}

impl ExecutedRequest {
    /// Returns the modeled outcome class.
    #[must_use]
    pub const fn class(&self) -> ResponseClass {
        self.class
    }

    /// Borrows the compact JSON response body without a trailing newline.
    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Consumes the response and returns its compact JSON body.
    #[must_use]
    pub fn into_body(self) -> Vec<u8> {
        self.body
    }
}

#[derive(Deserialize)]
struct ProtocolVersionProbe {
    protocol_version: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestV1 {
    protocol_version: u64,
    operation: RawOperationV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase", tag = "type")]
enum RawOperationV1 {
    Create {
        intent_unit: RawCreateIntentUnitV1,
        workflow: RawWorkflowV1,
    },
    Get {
        id: String,
    },
    List {
        filters: RawListFiltersV1,
        limit: RawInteger,
        #[serde(default, deserialize_with = "present")]
        after: Option<String>,
    },
    Transition {
        id: String,
        target: String,
        expected_revision: String,
    },
    Complete {
        id: String,
        expected_revision: String,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCreateIntentUnitV1 {
    #[serde(default, deserialize_with = "present")]
    id: Option<String>,
    species: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawWorkflowV1 {
    id: String,
    phases: Vec<String>,
    initial_phase: String,
    edges: Vec<RawWorkflowEdgeV1>,
    completion_phases: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawWorkflowEdgeV1 {
    from: String,
    to: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawListFiltersV1 {
    #[serde(default, deserialize_with = "present")]
    workflow_id: Option<String>,
    #[serde(default, deserialize_with = "present")]
    species: Option<String>,
    #[serde(default, deserialize_with = "present")]
    phase: Option<String>,
    #[serde(default, deserialize_with = "present")]
    status: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawInteger {
    Unsigned(u64),
    Signed(i64),
}

fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

pub(crate) enum ValidatedOperation {
    Create(CreateIntentUnit),
    Get(IntentUnitId),
    List(ListIntentUnits),
    Transition(TransitionIntentUnit),
    Complete(CompleteIntentUnit),
}

pub(crate) fn decode_request(bytes: &[u8]) -> Result<ValidatedOperation, ProtocolFailure> {
    let mut syntax = serde_json::Deserializer::from_slice(bytes);
    IgnoredAny::deserialize(&mut syntax)
        .map(|_| ())
        .and_then(|()| syntax.end())
        .map_err(|_| ProtocolFailure::plain(ErrorCode::MalformedJson, "request is not JSON"))?;

    let probe: ProtocolVersionProbe = serde_json::from_slice(bytes).map_err(|_| {
        ProtocolFailure::plain(
            ErrorCode::InvalidRequest,
            "request must contain an integer protocol_version",
        )
    })?;
    if probe.protocol_version != PROTOCOL_VERSION {
        return Err(ProtocolFailure::plain(
            ErrorCode::UnsupportedProtocolVersion,
            format!(
                "protocol version {} is not supported",
                probe.protocol_version
            ),
        ));
    }

    let request: RequestV1 = serde_json::from_slice(bytes).map_err(|_| {
        ProtocolFailure::plain(
            ErrorCode::InvalidRequest,
            "request does not match protocol version 1",
        )
    })?;
    debug_assert_eq!(request.protocol_version, PROTOCOL_VERSION);
    request.operation.validate()
}

impl RawOperationV1 {
    fn validate(self) -> Result<ValidatedOperation, ProtocolFailure> {
        match self {
            Self::Create {
                intent_unit,
                workflow,
            } => {
                let id = intent_unit
                    .id
                    .map(|value| parse_id(value, "operation.intent_unit.id"))
                    .transpose()?;
                let species = parse_species(intent_unit.species, "operation.intent_unit.species")?;
                let workflow = workflow.validate()?;
                Ok(ValidatedOperation::Create(CreateIntentUnit::new(
                    id, species, workflow,
                )))
            }
            Self::Get { id } => Ok(ValidatedOperation::Get(parse_id(id, "operation.id")?)),
            Self::List {
                filters,
                limit,
                after,
            } => {
                let filters = filters.validate()?;
                let limit = match limit {
                    RawInteger::Unsigned(value) => usize::try_from(value).ok(),
                    RawInteger::Signed(value) => usize::try_from(value).ok(),
                }
                .and_then(|value| PageLimit::new(value).ok())
                .ok_or_else(|| {
                    ProtocolFailure::field(
                        ErrorCode::InvalidQuery,
                        "list limit must be an integer from 1 through 100",
                        "operation.limit",
                    )
                })?;
                let after = after
                    .map(|value| {
                        ListCursor::from_str(&value).map_err(|_| {
                            ProtocolFailure::field(
                                ErrorCode::InvalidQuery,
                                "list cursor must be a canonical Intent Unit ID",
                                "operation.after",
                            )
                        })
                    })
                    .transpose()?;
                Ok(ValidatedOperation::List(ListIntentUnits::new(
                    filters, limit, after,
                )))
            }
            Self::Transition {
                id,
                target,
                expected_revision,
            } => Ok(ValidatedOperation::Transition(TransitionIntentUnit::new(
                parse_id(id, "operation.id")?,
                parse_phase(target, "operation.target")?,
                parse_revision(expected_revision, "operation.expected_revision")?,
            ))),
            Self::Complete {
                id,
                expected_revision,
            } => Ok(ValidatedOperation::Complete(CompleteIntentUnit::new(
                parse_id(id, "operation.id")?,
                parse_revision(expected_revision, "operation.expected_revision")?,
            ))),
        }
    }
}

impl RawWorkflowV1 {
    fn validate(self) -> Result<Workflow, ProtocolFailure> {
        let id = parse_workflow_id(self.id, "operation.workflow.id")?;
        let phases = self
            .phases
            .into_iter()
            .enumerate()
            .map(|(index, value)| parse_phase(value, format!("operation.workflow.phases[{index}]")))
            .collect::<Result<Vec<_>, _>>()?;
        let initial_phase = parse_phase(self.initial_phase, "operation.workflow.initial_phase")?;
        let edges = self
            .edges
            .into_iter()
            .enumerate()
            .map(|(index, edge)| {
                Ok(WorkflowEdge::new(
                    parse_phase(edge.from, format!("operation.workflow.edges[{index}].from"))?,
                    parse_phase(edge.to, format!("operation.workflow.edges[{index}].to"))?,
                ))
            })
            .collect::<Result<Vec<_>, ProtocolFailure>>()?;
        let completion_phases = self
            .completion_phases
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                parse_phase(
                    value,
                    format!("operation.workflow.completion_phases[{index}]"),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        Workflow::new(id, phases, initial_phase, edges, completion_phases).map_err(|error| {
            ProtocolFailure::field(
                ErrorCode::InvalidWorkflow,
                format!("workflow topology is invalid: {error}"),
                "operation.workflow",
            )
        })
    }
}

impl RawListFiltersV1 {
    fn validate(self) -> Result<ListFilters, ProtocolFailure> {
        let workflow_id = self
            .workflow_id
            .map(|value| parse_workflow_id(value, "operation.filters.workflow_id"))
            .transpose()?;
        let species = self
            .species
            .map(|value| parse_species(value, "operation.filters.species"))
            .transpose()?;
        let phase = self
            .phase
            .map(|value| parse_phase(value, "operation.filters.phase"))
            .transpose()?;
        let status = self
            .status
            .map(|value| match value.as_str() {
                "active" => Ok(IntentUnitStatus::Active),
                "completed" => Ok(IntentUnitStatus::Completed),
                _ => Err(ProtocolFailure::field(
                    ErrorCode::InvalidQuery,
                    "list status must be `active` or `completed`",
                    "operation.filters.status",
                )),
            })
            .transpose()?;
        Ok(ListFilters::new(workflow_id, species, phase, status))
    }
}

fn parse_id(value: String, field: impl Into<String>) -> Result<IntentUnitId, ProtocolFailure> {
    let field = field.into();
    let id = value.parse::<IntentUnitId>().map_err(|_| {
        ProtocolFailure::field(
            ErrorCode::InvalidIntentUnitId,
            "Intent Unit ID is malformed",
            field.clone(),
        )
    })?;
    if id.to_string() != value {
        return Err(ProtocolFailure::field(
            ErrorCode::InvalidIntentUnitId,
            "Intent Unit ID is not canonical lowercase hyphenated text",
            field,
        ));
    }
    Ok(id)
}

fn parse_species(
    value: String,
    field: impl Into<String>,
) -> Result<IntentSpecies, ProtocolFailure> {
    let field = field.into();
    IntentSpecies::new(value).map_err(|_| {
        ProtocolFailure::field(
            ErrorCode::InvalidSpecies,
            "Intent Unit species must not be blank",
            field,
        )
    })
}

fn parse_workflow_id(
    value: String,
    field: impl Into<String>,
) -> Result<WorkflowId, ProtocolFailure> {
    let field = field.into();
    WorkflowId::new(value).map_err(|_| {
        ProtocolFailure::field(
            ErrorCode::InvalidWorkflowId,
            "workflow ID must not be blank",
            field,
        )
    })
}

fn parse_phase(value: String, field: impl Into<String>) -> Result<PhaseId, ProtocolFailure> {
    let field = field.into();
    PhaseId::new(value).map_err(|_| {
        ProtocolFailure::field(
            ErrorCode::InvalidPhaseId,
            "phase ID must not be blank",
            field,
        )
    })
}

fn parse_revision(
    value: String,
    field: impl Into<String>,
) -> Result<IntentUnitRevision, ProtocolFailure> {
    let field = field.into();
    let bytes = value.as_bytes();
    let canonical_grammar = bytes == b"0"
        || matches!(bytes.first(), Some(b'1'..=b'9')) && bytes[1..].iter().all(u8::is_ascii_digit);
    if !canonical_grammar {
        return Err(ProtocolFailure::field(
            ErrorCode::InvalidRevision,
            "revision must be canonical unsigned decimal text",
            field,
        ));
    }
    let parsed = value.parse::<u64>().map_err(|_| {
        ProtocolFailure::field(
            ErrorCode::InvalidRevision,
            "revision must be canonical unsigned decimal text",
            field,
        )
    })?;
    Ok(IntentUnitRevision::new(parsed))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ErrorCode {
    MalformedJson,
    #[allow(dead_code, reason = "constructed by the locked T-808 bounded runner")]
    RequestTooLarge,
    InvalidRequest,
    UnsupportedProtocolVersion,
    InvalidIntentUnitId,
    InvalidSpecies,
    InvalidWorkflowId,
    InvalidPhaseId,
    InvalidWorkflow,
    InvalidQuery,
    InvalidRevision,
    DuplicateIntentUnit,
    IntentUnitNotFound,
    RevisionConflict,
    TransitionAlreadyCompleted,
    TransitionUnknownTarget,
    TransitionNotAllowed,
    CompletionAlreadyCompleted,
    CompletionPhaseNotEligible,
    StorageBusy,
    UnownedDatabase,
    UnsupportedSchemaVersion,
    CorruptSchema,
    UnsupportedEnvelopeVersion,
    CorruptEnvelope,
    ProjectionMismatch,
    ConcurrentStorageChange,
    StorageError,
}

impl ErrorCode {
    pub(crate) const fn class(self) -> ResponseClass {
        match self {
            Self::MalformedJson
            | Self::RequestTooLarge
            | Self::InvalidRequest
            | Self::UnsupportedProtocolVersion
            | Self::InvalidIntentUnitId
            | Self::InvalidSpecies
            | Self::InvalidWorkflowId
            | Self::InvalidPhaseId
            | Self::InvalidWorkflow
            | Self::InvalidQuery
            | Self::InvalidRevision => ResponseClass::RequestRejected,
            Self::DuplicateIntentUnit
            | Self::IntentUnitNotFound
            | Self::RevisionConflict
            | Self::TransitionAlreadyCompleted
            | Self::TransitionUnknownTarget
            | Self::TransitionNotAllowed
            | Self::CompletionAlreadyCompleted
            | Self::CompletionPhaseNotEligible => ResponseClass::CommandRejected,
            Self::StorageBusy
            | Self::UnownedDatabase
            | Self::UnsupportedSchemaVersion
            | Self::CorruptSchema
            | Self::UnsupportedEnvelopeVersion
            | Self::CorruptEnvelope
            | Self::ProjectionMismatch
            | Self::ConcurrentStorageChange
            | Self::StorageError => ResponseClass::StorageRejected,
        }
    }
}

pub(crate) struct ProtocolFailure {
    pub(crate) code: ErrorCode,
    message: String,
    field: Option<String>,
    expected_revision: Option<String>,
    actual_revision: Option<String>,
}

impl ProtocolFailure {
    pub(crate) fn plain(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            field: None,
            expected_revision: None,
            actual_revision: None,
        }
    }

    fn field(code: ErrorCode, message: impl Into<String>, field: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            field: Some(field.into()),
            expected_revision: None,
            actual_revision: None,
        }
    }

    pub(crate) fn conflict(
        message: impl Into<String>,
        expected: IntentUnitRevision,
        actual: IntentUnitRevision,
    ) -> Self {
        Self {
            code: ErrorCode::RevisionConflict,
            message: message.into(),
            field: None,
            expected_revision: Some(revision_text(expected)),
            actual_revision: Some(revision_text(actual)),
        }
    }
}

pub(crate) enum ProtocolResult {
    Unit(IntentUnitView),
    Page(IntentUnitPage),
    Mutation(MutationResult),
}

pub(crate) fn success_response(result: ProtocolResult) -> ExecutedRequest {
    let response = SuccessResponseV1 {
        protocol_version: PROTOCOL_VERSION,
        outcome: "success",
        result: ResultV1::from(result),
    };
    ExecutedRequest {
        class: ResponseClass::Success,
        body: serialize_response(&response),
    }
}

pub(crate) fn failure_response(error: ProtocolFailure) -> ExecutedRequest {
    let class = error.code.class();
    let response = FailureResponseV1 {
        protocol_version: PROTOCOL_VERSION,
        outcome: "failure",
        error: ErrorV1 {
            code: error.code,
            message: error.message,
            field: error.field,
            expected_revision: error.expected_revision,
            actual_revision: error.actual_revision,
        },
    };
    ExecutedRequest {
        class,
        body: serialize_response(&response),
    }
}

/// Builds the modeled response for T-808's bounded-ingestion rejection.
#[allow(dead_code, reason = "called by the locked T-808 bounded runner")]
pub(crate) fn request_too_large_response(max_bytes: usize) -> ExecutedRequest {
    failure_response(ProtocolFailure::plain(
        ErrorCode::RequestTooLarge,
        format!("request exceeds the {max_bytes}-byte limit"),
    ))
}

fn serialize_response(response: &impl Serialize) -> Vec<u8> {
    serde_json::to_vec(response)
        .expect("adapter-owned protocol response values must always serialize")
}

#[derive(Serialize)]
struct SuccessResponseV1 {
    protocol_version: u64,
    outcome: &'static str,
    result: ResultV1,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actual_revision: Option<String>,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase", tag = "type")]
enum ResultV1 {
    Unit {
        intent_unit: IntentUnitV1,
    },
    Page {
        items: Vec<IntentUnitSummaryV1>,
        next_cursor: Option<String>,
    },
    Mutation {
        committed_revision: String,
        intent_unit: IntentUnitV1,
    },
}

impl From<ProtocolResult> for ResultV1 {
    fn from(result: ProtocolResult) -> Self {
        match result {
            ProtocolResult::Unit(view) => Self::Unit {
                intent_unit: IntentUnitV1::from(&view),
            },
            ProtocolResult::Page(page) => Self::Page {
                items: page.items().iter().map(IntentUnitSummaryV1::from).collect(),
                next_cursor: page.next_cursor().map(|cursor| cursor.to_string()),
            },
            ProtocolResult::Mutation(result) => Self::Mutation {
                committed_revision: revision_text(result.committed_revision()),
                intent_unit: IntentUnitV1::from(result.intent_unit()),
            },
        }
    }
}

#[derive(Serialize)]
struct IntentUnitV1 {
    id: String,
    species: String,
    workflow: WorkflowV1,
    phase: String,
    status: StatusV1,
    revision: String,
    history: Vec<LifecycleRecordV1>,
}

impl From<&IntentUnitView> for IntentUnitV1 {
    fn from(view: &IntentUnitView) -> Self {
        Self {
            id: view.id().to_string(),
            species: view.species().as_str().to_owned(),
            workflow: WorkflowV1::from(view.workflow()),
            phase: view.phase().as_str().to_owned(),
            status: StatusV1::from(view.status()),
            revision: revision_text(view.revision()),
            history: view.history().iter().map(LifecycleRecordV1::from).collect(),
        }
    }
}

#[derive(Serialize)]
struct IntentUnitSummaryV1 {
    id: String,
    species: String,
    workflow_id: String,
    phase: String,
    status: StatusV1,
    revision: String,
}

impl From<&IntentUnitSummary> for IntentUnitSummaryV1 {
    fn from(summary: &IntentUnitSummary) -> Self {
        Self {
            id: summary.id().to_string(),
            species: summary.species().as_str().to_owned(),
            workflow_id: summary.workflow_id().as_str().to_owned(),
            phase: summary.phase().as_str().to_owned(),
            status: StatusV1::from(summary.status()),
            revision: revision_text(summary.revision()),
        }
    }
}

#[derive(Serialize)]
struct WorkflowV1 {
    id: String,
    phases: Vec<String>,
    initial_phase: String,
    edges: Vec<WorkflowEdgeV1>,
    completion_phases: Vec<String>,
}

impl From<&Workflow> for WorkflowV1 {
    fn from(workflow: &Workflow) -> Self {
        Self {
            id: workflow.id().as_str().to_owned(),
            phases: workflow
                .phases()
                .iter()
                .map(|phase| phase.as_str().to_owned())
                .collect(),
            initial_phase: workflow.initial_phase().as_str().to_owned(),
            edges: workflow.edges().iter().map(WorkflowEdgeV1::from).collect(),
            completion_phases: workflow
                .completion_phases()
                .iter()
                .map(|phase| phase.as_str().to_owned())
                .collect(),
        }
    }
}

#[derive(Serialize)]
struct WorkflowEdgeV1 {
    from: String,
    to: String,
}

impl From<&WorkflowEdge> for WorkflowEdgeV1 {
    fn from(edge: &WorkflowEdge) -> Self {
        Self {
            from: edge.from().as_str().to_owned(),
            to: edge.to().as_str().to_owned(),
        }
    }
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
enum StatusV1 {
    Active,
    Completed,
}

impl From<IntentUnitStatus> for StatusV1 {
    fn from(status: IntentUnitStatus) -> Self {
        match status {
            IntentUnitStatus::Active => Self::Active,
            IntentUnitStatus::Completed => Self::Completed,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase", tag = "type")]
enum LifecycleRecordV1 {
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

impl From<&LifecycleRecord> for LifecycleRecordV1 {
    fn from(record: &LifecycleRecord) -> Self {
        match record {
            LifecycleRecord::Transition(record) => Self::Transition {
                sequence: record.sequence(),
                from: record.from().as_str().to_owned(),
                to: record.to().as_str().to_owned(),
            },
            LifecycleRecord::Completion(record) => Self::Completion {
                sequence: record.sequence(),
                phase: record.final_phase().as_str().to_owned(),
            },
        }
    }
}

fn revision_text(revision: IntentUnitRevision) -> String {
    revision.value().to_string()
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    #[test]
    fn test_protocol_v1_maps_exact_error_code_taxonomy() {
        let cases = [
            (
                ErrorCode::MalformedJson,
                "malformed_json",
                ResponseClass::RequestRejected,
            ),
            (
                ErrorCode::RequestTooLarge,
                "request_too_large",
                ResponseClass::RequestRejected,
            ),
            (
                ErrorCode::InvalidRequest,
                "invalid_request",
                ResponseClass::RequestRejected,
            ),
            (
                ErrorCode::UnsupportedProtocolVersion,
                "unsupported_protocol_version",
                ResponseClass::RequestRejected,
            ),
            (
                ErrorCode::InvalidIntentUnitId,
                "invalid_intent_unit_id",
                ResponseClass::RequestRejected,
            ),
            (
                ErrorCode::InvalidSpecies,
                "invalid_species",
                ResponseClass::RequestRejected,
            ),
            (
                ErrorCode::InvalidWorkflowId,
                "invalid_workflow_id",
                ResponseClass::RequestRejected,
            ),
            (
                ErrorCode::InvalidPhaseId,
                "invalid_phase_id",
                ResponseClass::RequestRejected,
            ),
            (
                ErrorCode::InvalidWorkflow,
                "invalid_workflow",
                ResponseClass::RequestRejected,
            ),
            (
                ErrorCode::InvalidQuery,
                "invalid_query",
                ResponseClass::RequestRejected,
            ),
            (
                ErrorCode::InvalidRevision,
                "invalid_revision",
                ResponseClass::RequestRejected,
            ),
            (
                ErrorCode::DuplicateIntentUnit,
                "duplicate_intent_unit",
                ResponseClass::CommandRejected,
            ),
            (
                ErrorCode::IntentUnitNotFound,
                "intent_unit_not_found",
                ResponseClass::CommandRejected,
            ),
            (
                ErrorCode::RevisionConflict,
                "revision_conflict",
                ResponseClass::CommandRejected,
            ),
            (
                ErrorCode::TransitionAlreadyCompleted,
                "transition_already_completed",
                ResponseClass::CommandRejected,
            ),
            (
                ErrorCode::TransitionUnknownTarget,
                "transition_unknown_target",
                ResponseClass::CommandRejected,
            ),
            (
                ErrorCode::TransitionNotAllowed,
                "transition_not_allowed",
                ResponseClass::CommandRejected,
            ),
            (
                ErrorCode::CompletionAlreadyCompleted,
                "completion_already_completed",
                ResponseClass::CommandRejected,
            ),
            (
                ErrorCode::CompletionPhaseNotEligible,
                "completion_phase_not_eligible",
                ResponseClass::CommandRejected,
            ),
            (
                ErrorCode::StorageBusy,
                "storage_busy",
                ResponseClass::StorageRejected,
            ),
            (
                ErrorCode::UnownedDatabase,
                "unowned_database",
                ResponseClass::StorageRejected,
            ),
            (
                ErrorCode::UnsupportedSchemaVersion,
                "unsupported_schema_version",
                ResponseClass::StorageRejected,
            ),
            (
                ErrorCode::CorruptSchema,
                "corrupt_schema",
                ResponseClass::StorageRejected,
            ),
            (
                ErrorCode::UnsupportedEnvelopeVersion,
                "unsupported_envelope_version",
                ResponseClass::StorageRejected,
            ),
            (
                ErrorCode::CorruptEnvelope,
                "corrupt_envelope",
                ResponseClass::StorageRejected,
            ),
            (
                ErrorCode::ProjectionMismatch,
                "projection_mismatch",
                ResponseClass::StorageRejected,
            ),
            (
                ErrorCode::ConcurrentStorageChange,
                "concurrent_storage_change",
                ResponseClass::StorageRejected,
            ),
            (
                ErrorCode::StorageError,
                "storage_error",
                ResponseClass::StorageRejected,
            ),
        ];
        assert_eq!(cases.len(), 28);

        for (code, expected_code, expected_class) in cases {
            let is_field_validation = matches!(
                code,
                ErrorCode::InvalidIntentUnitId
                    | ErrorCode::InvalidSpecies
                    | ErrorCode::InvalidWorkflowId
                    | ErrorCode::InvalidPhaseId
                    | ErrorCode::InvalidWorkflow
                    | ErrorCode::InvalidQuery
                    | ErrorCode::InvalidRevision
            );
            let failure = if code == ErrorCode::RevisionConflict {
                ProtocolFailure::conflict("conflict", revision(0), revision(u64::MAX))
            } else if is_field_validation {
                ProtocolFailure::field(code, "invalid field", "operation.test")
            } else {
                ProtocolFailure::plain(code, "modeled failure")
            };
            let response = failure_response(failure);
            assert_eq!(response.class(), expected_class);
            let value: Value = serde_json::from_slice(response.body()).unwrap();
            assert_eq!(value["protocol_version"], PROTOCOL_VERSION);
            assert_eq!(value["outcome"], "failure");
            assert_eq!(value["error"]["code"], expected_code);
            assert!(value["error"]["message"].is_string());
            assert_eq!(value["error"].get("field").is_some(), is_field_validation);
            assert_eq!(
                value["error"].get("expected_revision").is_some(),
                code == ErrorCode::RevisionConflict
            );
            assert_eq!(
                value["error"].get("actual_revision").is_some(),
                code == ErrorCode::RevisionConflict
            );
        }

        let too_large = request_too_large_response(1_048_576);
        assert_eq!(too_large.class(), ResponseClass::RequestRejected);
        let value: Value = serde_json::from_slice(too_large.body()).unwrap();
        assert_eq!(value["error"]["code"], "request_too_large");
        assert!(value["error"].get("field").is_none());
    }

    #[test]
    fn test_response_revision_codec_preserves_full_u64_as_text() {
        for value in [0, i64::MAX as u64 + 1, u64::MAX] {
            let encoded = revision_text(revision(value));
            assert_eq!(encoded, value.to_string());
            assert!(serde_json::to_value(encoded).unwrap().is_string());
        }
    }

    const fn revision(value: u64) -> IntentUnitRevision {
        IntentUnitRevision::new(value)
    }
}
