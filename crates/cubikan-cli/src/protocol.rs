use std::fmt;

use cubikan_core::{IntentUnit, IntentUnitStatus, LifecycleRecord};
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{MapAccess, SeqAccess, Visitor},
};

pub(crate) const PROTOCOL_VERSION: u8 = 2;

const AUTHORITY: &str = "simulation_only";
const INVALID_REQUEST_MESSAGE: &str = "request does not match the stateless protocol v2 schema";
const INVALID_EXTERNAL_REFERENCE_MESSAGE: &str =
    "origin must be an exact bounded external reference";
const INVALID_PHASE_ID_MESSAGE: &str =
    "phase identifier must be nonblank NUL-free UTF-8 of at most 256 bytes";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Request {
    pub(crate) workflow: WorkflowInput,
    pub(crate) intent_unit: IntentUnitInput,
    pub(crate) operations: Vec<OperationInput>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkflowInput {
    pub(crate) id: String,
    pub(crate) phases: Vec<String>,
    pub(crate) initial_phase: String,
    pub(crate) edges: Vec<WorkflowEdgeInput>,
    pub(crate) completion_phases: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkflowEdgeInput {
    pub(crate) from: String,
    pub(crate) to: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IntentUnitInput {
    pub(crate) id: Option<String>,
    pub(crate) origin: ExternalReferenceInput,
    pub(crate) species: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExternalReferenceInput {
    pub(crate) namespace: String,
    pub(crate) scope: String,
    pub(crate) value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum OperationInput {
    Transition { target: String },
    Complete,
}

pub(crate) fn decode_request(bytes: &[u8]) -> Result<Request, ErrorDetail> {
    let root = decode_json(bytes)?;
    let root_entries = match &root {
        RawJson::Object(entries) => entries.as_slice(),
        _ => return Err(ErrorDetail::invalid_request("")),
    };

    probe_protocol_version(root_entries)?;
    let root_entries = closed_object(
        &root,
        "",
        &["protocol_version", "workflow", "intent_unit", "operations"],
    )?;

    let workflow = parse_workflow(required_member(root_entries, "workflow", "")?)?;
    let intent_unit = parse_intent_unit(required_member(root_entries, "intent_unit", "")?)?;
    let operations = parse_operations(required_member(root_entries, "operations", "")?)?;

    Ok(Request {
        workflow,
        intent_unit,
        operations,
    })
}

fn decode_json(bytes: &[u8]) -> Result<RawJson, ErrorDetail> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = RawJson::deserialize(&mut deserializer)
        .and_then(|value| deserializer.end().map(|()| value));
    value.map_err(|_| ErrorDetail::malformed_json())
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
        RawJson::Unsigned(value) if *value == u64::from(PROTOCOL_VERSION) => Ok(()),
        RawJson::Signed(value) if *value == i64::from(PROTOCOL_VERSION) => Ok(()),
        RawJson::Unsigned(_) | RawJson::Signed(_) => {
            Err(ErrorDetail::unsupported_protocol_version())
        }
        _ => Err(ErrorDetail::invalid_request("/protocol_version")),
    }
}

fn parse_workflow(value: &RawJson) -> Result<WorkflowInput, ErrorDetail> {
    const PATH: &str = "/workflow";
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
    let id = required_string(entries, "id", PATH)?;
    let phases = parse_string_array(
        required_member(entries, "phases", PATH)?,
        "/workflow/phases",
    )?;
    let initial_phase = required_string(entries, "initial_phase", PATH)?;
    let edges = parse_edges(required_member(entries, "edges", PATH)?)?;
    let completion_phases = parse_string_array(
        required_member(entries, "completion_phases", PATH)?,
        "/workflow/completion_phases",
    )?;

    Ok(WorkflowInput {
        id,
        phases,
        initial_phase,
        edges,
        completion_phases,
    })
}

fn parse_edges(value: &RawJson) -> Result<Vec<WorkflowEdgeInput>, ErrorDetail> {
    let RawJson::Array(values) = value else {
        return Err(ErrorDetail::invalid_request("/workflow/edges"));
    };
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let path = format!("/workflow/edges/{index}");
            let entries = closed_object(value, &path, &["from", "to"])?;
            Ok(WorkflowEdgeInput {
                from: required_string(entries, "from", &path)?,
                to: required_string(entries, "to", &path)?,
            })
        })
        .collect()
}

fn parse_intent_unit(value: &RawJson) -> Result<IntentUnitInput, ErrorDetail> {
    const PATH: &str = "/intent_unit";
    let entries = closed_object(value, PATH, &["id", "origin", "species"])?;
    let id = match member(entries, "id") {
        None => None,
        Some(RawJson::String(value)) => Some(value.clone()),
        Some(_) => return Err(ErrorDetail::invalid_intent_unit_id()),
    };
    let origin = match member(entries, "origin") {
        Some(value) => parse_external_reference(value)?,
        None => {
            return Err(ErrorDetail::invalid_external_reference(
                "/intent_unit/origin",
            ));
        }
    };
    let species = match member(entries, "species") {
        Some(RawJson::String(value)) => value.clone(),
        Some(_) | None => return Err(ErrorDetail::invalid_species()),
    };

    Ok(IntentUnitInput {
        id,
        origin,
        species,
    })
}

fn parse_external_reference(value: &RawJson) -> Result<ExternalReferenceInput, ErrorDetail> {
    const PATH: &str = "/intent_unit/origin";
    let RawJson::Object(_) = value else {
        return Err(ErrorDetail::invalid_external_reference(PATH));
    };
    let entries = closed_object(value, PATH, &["namespace", "scope", "value"])?;
    let reference_member = |name: &str| {
        let path = pointer_member(PATH, name);
        match member(entries, name) {
            Some(RawJson::String(value)) => Ok(value.clone()),
            Some(_) | None => Err(ErrorDetail::invalid_external_reference(path)),
        }
    };

    Ok(ExternalReferenceInput {
        namespace: reference_member("namespace")?,
        scope: reference_member("scope")?,
        value: reference_member("value")?,
    })
}

fn parse_operations(value: &RawJson) -> Result<Vec<OperationInput>, ErrorDetail> {
    let RawJson::Array(values) = value else {
        return Err(ErrorDetail::invalid_request("/operations"));
    };
    if values.len() > 256 {
        return Err(ErrorDetail::invalid_request("/operations"));
    }
    values
        .iter()
        .enumerate()
        .map(|(index, value)| parse_operation(value, index))
        .collect()
}

fn parse_operation(value: &RawJson, index: usize) -> Result<OperationInput, ErrorDetail> {
    let path = format!("/operations/{index}");
    let RawJson::Object(entries) = value else {
        return Err(ErrorDetail::invalid_request(path));
    };
    let type_path = pointer_member(&path, "type");
    ensure_unique_member(entries, "type", &type_path)?;
    let operation_type = match member(entries, "type") {
        Some(RawJson::String(value)) => value.as_str(),
        Some(_) | None => return Err(ErrorDetail::invalid_request(type_path)),
    };

    match operation_type {
        "transition" => {
            let entries = closed_object(value, &path, &["type", "target"])?;
            Ok(OperationInput::Transition {
                target: required_string(entries, "target", &path)?,
            })
        }
        "complete" => {
            closed_object(value, &path, &["type"])?;
            Ok(OperationInput::Complete)
        }
        _ => Err(ErrorDetail::invalid_request(type_path)),
    }
}

fn parse_string_array(value: &RawJson, path: &str) -> Result<Vec<String>, ErrorDetail> {
    let RawJson::Array(values) = value else {
        return Err(ErrorDetail::invalid_request(path));
    };
    values
        .iter()
        .enumerate()
        .map(|(index, value)| match value {
            RawJson::String(value) => Ok(value.clone()),
            _ => Err(ErrorDetail::invalid_request(format!("{path}/{index}"))),
        })
        .collect()
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
    parent_path: &str,
) -> Result<&'a RawJson, ErrorDetail> {
    member(entries, name)
        .ok_or_else(|| ErrorDetail::invalid_request(pointer_member(parent_path, name)))
}

fn required_string(
    entries: &[(String, RawJson)],
    name: &str,
    parent_path: &str,
) -> Result<String, ErrorDetail> {
    let path = pointer_member(parent_path, name);
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

#[derive(Debug, Serialize)]
pub(crate) struct ErrorDetail {
    code: ErrorCode,
    message: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation_number: Option<usize>,
}

impl ErrorDetail {
    pub(crate) fn malformed_json() -> Self {
        Self::setup(
            ErrorCode::MalformedJson,
            "request is not valid RFC 8259 JSON",
            None,
        )
    }

    pub(crate) fn request_too_large() -> Self {
        Self::setup(
            ErrorCode::RequestTooLarge,
            "request exceeds the 1048576-byte limit",
            None,
        )
    }

    fn invalid_request(field: impl Into<String>) -> Self {
        Self::setup(
            ErrorCode::InvalidRequest,
            INVALID_REQUEST_MESSAGE,
            Some(field.into()),
        )
    }

    fn unsupported_protocol_version() -> Self {
        Self::setup(
            ErrorCode::UnsupportedProtocolVersion,
            "protocol_version must be 2",
            Some("/protocol_version".to_owned()),
        )
    }

    pub(crate) fn invalid_intent_unit_id() -> Self {
        Self::setup(
            ErrorCode::InvalidIntentUnitId,
            "intent_unit.id must be a lowercase hyphenated RFC 4122 UUID",
            Some("/intent_unit/id".to_owned()),
        )
    }

    pub(crate) fn invalid_external_reference(field: impl Into<String>) -> Self {
        Self::setup(
            ErrorCode::InvalidExternalReference,
            INVALID_EXTERNAL_REFERENCE_MESSAGE,
            Some(field.into()),
        )
    }

    pub(crate) fn invalid_species() -> Self {
        Self::setup(
            ErrorCode::InvalidSpecies,
            "intent_unit.species must be nonblank NUL-free UTF-8 of at most 256 bytes",
            Some("/intent_unit/species".to_owned()),
        )
    }

    pub(crate) fn invalid_workflow_id() -> Self {
        Self::setup(
            ErrorCode::InvalidWorkflowId,
            "workflow.id must be nonblank NUL-free UTF-8 of at most 256 bytes",
            Some("/workflow/id".to_owned()),
        )
    }

    pub(crate) fn invalid_phase_id(field: impl Into<String>) -> Self {
        Self::setup(
            ErrorCode::InvalidPhaseId,
            INVALID_PHASE_ID_MESSAGE,
            Some(field.into()),
        )
    }

    pub(crate) fn invalid_workflow(field: impl Into<String>) -> Self {
        Self::setup(
            ErrorCode::InvalidWorkflow,
            "workflow topology is invalid",
            Some(field.into()),
        )
    }

    pub(crate) fn transition_already_completed(operation_number: usize) -> Self {
        Self::operation(
            ErrorCode::TransitionAlreadyCompleted,
            "cannot transition a completed intent unit",
            operation_number,
        )
    }

    pub(crate) fn transition_unknown_target(operation_number: usize) -> Self {
        Self::operation(
            ErrorCode::TransitionUnknownTarget,
            "transition target is not declared by the workflow",
            operation_number,
        )
    }

    pub(crate) fn transition_not_allowed(operation_number: usize) -> Self {
        Self::operation(
            ErrorCode::TransitionNotAllowed,
            "workflow does not allow this transition",
            operation_number,
        )
    }

    pub(crate) fn completion_already_completed(operation_number: usize) -> Self {
        Self::operation(
            ErrorCode::CompletionAlreadyCompleted,
            "cannot complete an already completed intent unit",
            operation_number,
        )
    }

    pub(crate) fn completion_phase_not_eligible(operation_number: usize) -> Self {
        Self::operation(
            ErrorCode::CompletionPhaseNotEligible,
            "current phase is not eligible for completion",
            operation_number,
        )
    }

    fn setup(code: ErrorCode, message: &'static str, field: Option<String>) -> Self {
        Self {
            code,
            message,
            field,
            operation_number: None,
        }
    }

    fn operation(code: ErrorCode, message: &'static str, operation_number: usize) -> Self {
        Self {
            code,
            message,
            field: None,
            operation_number: Some(operation_number),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ErrorCode {
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
    TransitionAlreadyCompleted,
    TransitionUnknownTarget,
    TransitionNotAllowed,
    CompletionAlreadyCompleted,
    CompletionPhaseNotEligible,
}

pub(crate) enum ProtocolResponse {
    Success(SuccessResponse),
    SetupError(SetupErrorResponse),
    OperationError(OperationErrorResponse),
}

impl ProtocolResponse {
    pub(crate) fn success(intent_unit: &IntentUnit) -> Self {
        Self::Success(SuccessResponse {
            protocol_version: PROTOCOL_VERSION,
            authority: AUTHORITY,
            outcome: "success",
            result: SimulationResult {
                result_type: "simulation",
                intent_unit: UnitView::from(intent_unit),
            },
        })
    }

    pub(crate) fn setup_error(error: ErrorDetail) -> Self {
        Self::SetupError(SetupErrorResponse {
            protocol_version: PROTOCOL_VERSION,
            authority: AUTHORITY,
            outcome: "error",
            error,
        })
    }

    pub(crate) fn operation_error(error: ErrorDetail, intent_unit: &IntentUnit) -> Self {
        Self::OperationError(OperationErrorResponse {
            protocol_version: PROTOCOL_VERSION,
            authority: AUTHORITY,
            outcome: "error",
            error,
            intent_unit: UnitView::from(intent_unit),
        })
    }
}

impl Serialize for ProtocolResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Success(response) => response.serialize(serializer),
            Self::SetupError(response) => response.serialize(serializer),
            Self::OperationError(response) => response.serialize(serializer),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct SuccessResponse {
    protocol_version: u8,
    authority: &'static str,
    outcome: &'static str,
    result: SimulationResult,
}

#[derive(Serialize)]
struct SimulationResult {
    #[serde(rename = "type")]
    result_type: &'static str,
    intent_unit: UnitView,
}

#[derive(Serialize)]
pub(crate) struct SetupErrorResponse {
    protocol_version: u8,
    authority: &'static str,
    outcome: &'static str,
    error: ErrorDetail,
}

#[derive(Serialize)]
pub(crate) struct OperationErrorResponse {
    protocol_version: u8,
    authority: &'static str,
    outcome: &'static str,
    error: ErrorDetail,
    intent_unit: UnitView,
}

#[derive(Serialize)]
struct UnitView {
    id: String,
    origin: ExternalReferenceView,
    species: String,
    workflow: WorkflowView,
    phase: String,
    status: StatusView,
    revision: String,
    history: Vec<HistoryView>,
}

impl From<&IntentUnit> for UnitView {
    fn from(unit: &IntentUnit) -> Self {
        Self {
            id: unit.id().to_string(),
            origin: ExternalReferenceView {
                namespace: unit.origin().namespace().as_str().to_owned(),
                scope: unit.origin().scope().as_str().to_owned(),
                value: unit.origin().value().as_str().to_owned(),
            },
            species: unit.species().as_str().to_owned(),
            workflow: WorkflowView {
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
                    .map(|edge| WorkflowEdgeView {
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
                IntentUnitStatus::Active => StatusView::Active,
                IntentUnitStatus::Completed => StatusView::Completed,
            },
            revision: unit.revision().value().to_string(),
            history: unit.history().iter().map(HistoryView::from).collect(),
        }
    }
}

#[derive(Serialize)]
struct ExternalReferenceView {
    namespace: String,
    scope: String,
    value: String,
}

#[derive(Serialize)]
struct WorkflowView {
    id: String,
    phases: Vec<String>,
    initial_phase: String,
    edges: Vec<WorkflowEdgeView>,
    completion_phases: Vec<String>,
}

#[derive(Serialize)]
struct WorkflowEdgeView {
    from: String,
    to: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum StatusView {
    Active,
    Completed,
}

#[derive(Serialize)]
#[serde(untagged)]
enum HistoryView {
    Transition(TransitionHistoryView),
    Completion(CompletionHistoryView),
}

impl From<&LifecycleRecord> for HistoryView {
    fn from(record: &LifecycleRecord) -> Self {
        match record {
            LifecycleRecord::Transition(record) => Self::Transition(TransitionHistoryView {
                record_type: "transition",
                sequence: record.sequence().to_string(),
                from: record.from().as_str().to_owned(),
                to: record.to().as_str().to_owned(),
            }),
            LifecycleRecord::Completion(record) => Self::Completion(CompletionHistoryView {
                record_type: "completion",
                sequence: record.sequence().to_string(),
                phase: record.final_phase().as_str().to_owned(),
            }),
        }
    }
}

#[derive(Serialize)]
struct TransitionHistoryView {
    #[serde(rename = "type")]
    record_type: &'static str,
    sequence: String,
    from: String,
    to: String,
}

#[derive(Serialize)]
struct CompletionHistoryView {
    #[serde(rename = "type")]
    record_type: &'static str,
    sequence: String,
    phase: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nul_unknown_member_uses_valid_ancestor_pointer() {
        let request = br#"{"protocol_version":2,"workflow":{"id":"w","phases":["p"],"initial_phase":"p","edges":[],"completion_phases":[]},"intent_unit":{"origin":{"namespace":"a","scope":"s","value":"v"},"species":"x"},"operations":[],"\u0000":true}"#;
        let error = decode_request(request).expect_err("NUL member name must reject");
        let response = serde_json::to_vec(&ProtocolResponse::setup_error(error))
            .expect("error response should serialize");

        assert_eq!(
            response,
            br#"{"protocol_version":2,"authority":"simulation_only","outcome":"error","error":{"code":"invalid_request","message":"request does not match the stateless protocol v2 schema","field":""}}"#
        );
        assert!(!response.contains(&0));
    }
}
