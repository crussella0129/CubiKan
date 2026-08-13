use frame_support::{traits::ConstU32, BoundedVec};
use parity_scale_codec::{Compact, Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde_json::Value;

use crate::{
    conformance::{
        accepted_event_bound_is_valid, decode_before_dispatch, definition_id, definition_version,
        external_reference, namespace, phase_id, projected_query_limit, reference_value,
        require_association_capacity, require_authorized, require_authorized_submitter_capacity,
        require_lifecycle_capacity, require_relationship_capacity, scope, species,
        workflow as validate_workflow, workflow_id, BoundaryCounters, BoundedLimitError,
        ChainOnlyError, ConformanceError, StructuralDecodeError, TextField, WorkflowInput,
    },
    types::{
        AssociationKey, AssociationSubject, CreateUnitPayload, DefinitionKey, DomainPayload,
        ExternalReference, IntentSpecies, IntentUnitId, IntentUnitState, IntentUnitStatus,
        LifecycleError, LifecycleRecord, Namespace, PhaseId, ReferenceScope, ReferenceValue,
        RelationshipDefinition, RelationshipDirection, RelationshipKey, RelationshipPolicy,
        Text256, Workflow, WorkflowEdge, WorkflowId, MAX_ACCEPTED_EVENT_BYTES,
        MAX_ACTIVE_ASSOCIATIONS, MAX_AUTHORIZED_SUBMITTERS, MAX_COMPLETION_PHASES,
        MAX_LIFECYCLE_RECORDS, MAX_NAMESPACE_BYTES, MAX_RELATIONSHIP_EDGES, MAX_TEXT_BYTES,
        MAX_WORKFLOW_EDGES, MAX_WORKFLOW_PHASES,
    },
};

const CORPUS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../tests/fixtures/chain-conformance-v1.json"
));

fn corpus() -> Value {
    serde_json::from_str(CORPUS_JSON).expect("the checked conformance corpus must be valid JSON")
}

fn field<'a>(value: &'a Value, key: &str, context: &str) -> &'a Value {
    value
        .get(key)
        .unwrap_or_else(|| panic!("{context} is missing `{key}`"))
}

fn text<'a>(value: &'a Value, key: &str, context: &str) -> &'a str {
    field(value, key, context)
        .as_str()
        .unwrap_or_else(|| panic!("{context}.{key} must be text"))
}

fn number(value: &Value, key: &str, context: &str) -> usize {
    usize::try_from(
        field(value, key, context)
            .as_u64()
            .unwrap_or_else(|| panic!("{context}.{key} must be an unsigned integer")),
    )
    .expect("fixture integers must fit usize")
}

fn values<'a>(value: &'a Value, key: &str, context: &str) -> &'a [Value] {
    field(value, key, context)
        .as_array()
        .unwrap_or_else(|| panic!("{context}.{key} must be an array"))
}

fn decode_hex(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0, "hex input must contain whole bytes");
    assert!(
        hex.bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "fixture hex must be lowercase without a prefix"
    );
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digits = core::str::from_utf8(pair).expect("hex digits are ASCII");
            u8::from_str_radix(digits, 16).expect("fixture hex must decode")
        })
        .collect()
}

fn input_bytes(case: &Value) -> Vec<u8> {
    let case_id = text(case, "id", "value case");
    let input = field(case, "input", case_id);
    match (input.get("utf8"), input.get("hex")) {
        (Some(utf8), None) => utf8
            .as_str()
            .unwrap_or_else(|| panic!("{case_id}.input.utf8 must be text"))
            .as_bytes()
            .to_vec(),
        (None, Some(hex)) => decode_hex(
            hex.as_str()
                .unwrap_or_else(|| panic!("{case_id}.input.hex must be text")),
        ),
        _ => panic!("{case_id} must supply exactly one of utf8 or hex"),
    }
}

fn expected_error(case: &Value) -> &Value {
    let case_id = text(case, "id", "case");
    let expected = field(case, "expected", case_id);
    assert_eq!(text(expected, "outcome", case_id), "error", "{case_id}");
    field(expected, "error", case_id)
}

fn expected_path(field: TextField) -> &'static str {
    match field {
        TextField::Namespace => "/namespace",
        TextField::Scope => "/scope",
        TextField::Value => "/value",
        TextField::WorkflowId => "/workflow/id",
        TextField::Phase => "/workflow/phases/0",
        TextField::Species => "/species",
        TextField::DefinitionId => "/key/id",
    }
}

fn assert_conformance_error(error: ConformanceError, expected: &Value, case_id: &str) {
    match error {
        ConformanceError::Empty { field } => {
            assert_eq!(text(expected, "kind", case_id), "empty", "{case_id}");
            assert_eq!(
                text(expected, "path", case_id),
                expected_path(field),
                "{case_id}"
            );
            assert_eq!(
                number(expected, "input_byte_length", case_id),
                0,
                "{case_id}"
            );
            assert_eq!(number(expected, "minimum", case_id), 1, "{case_id}");
        }
        ConformanceError::TooLong {
            field,
            length,
            maximum,
        } => {
            assert_eq!(text(expected, "kind", case_id), "too_long", "{case_id}");
            assert_eq!(
                text(expected, "path", case_id),
                expected_path(field),
                "{case_id}"
            );
            assert_eq!(
                number(expected, "input_byte_length", case_id),
                length,
                "{case_id}"
            );
            assert_eq!(number(expected, "maximum", case_id), maximum, "{case_id}");
        }
        ConformanceError::InvalidUtf8 {
            field,
            index,
            length,
        } => {
            assert_eq!(text(expected, "kind", case_id), "invalid_utf8", "{case_id}");
            assert_eq!(
                text(expected, "path", case_id),
                expected_path(field),
                "{case_id}"
            );
            assert_eq!(number(expected, "byte_index", case_id), index, "{case_id}");
            assert_eq!(
                number(expected, "invalid_byte_length", case_id),
                length,
                "{case_id}"
            );
        }
        ConformanceError::Nul { field, index } => {
            assert_eq!(text(expected, "kind", case_id), "contains_nul", "{case_id}");
            assert_eq!(
                text(expected, "path", case_id),
                expected_path(field),
                "{case_id}"
            );
            assert_eq!(number(expected, "byte_index", case_id), index, "{case_id}");
        }
        ConformanceError::Blank { field } => {
            assert_eq!(text(expected, "kind", case_id), "blank", "{case_id}");
            assert_eq!(
                text(expected, "path", case_id),
                expected_path(field),
                "{case_id}"
            );
        }
        ConformanceError::InvalidNamespaceStart { field, index, byte } => {
            assert_eq!(
                text(expected, "kind", case_id),
                "invalid_namespace_start",
                "{case_id}"
            );
            assert_eq!(
                text(expected, "path", case_id),
                expected_path(field),
                "{case_id}"
            );
            assert_eq!(number(expected, "byte_index", case_id), index, "{case_id}");
            assert_eq!(
                number(expected, "byte_value", case_id),
                usize::from(byte),
                "{case_id}"
            );
        }
        ConformanceError::InvalidNamespaceByte { field, index, byte } => {
            assert_eq!(
                text(expected, "kind", case_id),
                "invalid_namespace_byte",
                "{case_id}"
            );
            assert_eq!(
                text(expected, "path", case_id),
                expected_path(field),
                "{case_id}"
            );
            assert_eq!(number(expected, "byte_index", case_id), index, "{case_id}");
            assert_eq!(
                number(expected, "byte_value", case_id),
                usize::from(byte),
                "{case_id}"
            );
        }
        ConformanceError::DefinitionVersionZero => {
            assert_eq!(
                text(expected, "kind", case_id),
                "zero_definition_version",
                "{case_id}"
            );
            assert_eq!(text(expected, "path", case_id), "/key/version", "{case_id}");
            assert_eq!(number(expected, "value", case_id), 0, "{case_id}");
        }
        ConformanceError::EmptyPhases => {
            assert_eq!(
                text(expected, "kind", case_id),
                "too_few_phases",
                "{case_id}"
            );
            assert_eq!(text(expected, "path", case_id), "/phases", "{case_id}");
            assert_eq!(number(expected, "length", case_id), 0, "{case_id}");
        }
        ConformanceError::TooManyPhases { length, maximum } => {
            assert_eq!(
                text(expected, "kind", case_id),
                "too_many_phases",
                "{case_id}"
            );
            assert_eq!(number(expected, "length", case_id), length, "{case_id}");
            assert_eq!(number(expected, "maximum", case_id), maximum, "{case_id}");
        }
        ConformanceError::TooManyWorkflowEdges { length, maximum } => {
            assert_eq!(
                text(expected, "kind", case_id),
                "too_many_workflow_edges",
                "{case_id}"
            );
            assert_eq!(number(expected, "length", case_id), length, "{case_id}");
            assert_eq!(number(expected, "maximum", case_id), maximum, "{case_id}");
        }
        ConformanceError::TooManyCompletionPhases { length, maximum } => {
            assert_eq!(
                text(expected, "kind", case_id),
                "too_many_completion_phases",
                "{case_id}"
            );
            assert_eq!(number(expected, "length", case_id), length, "{case_id}");
            assert_eq!(number(expected, "maximum", case_id), maximum, "{case_id}");
        }
        ConformanceError::DuplicatePhase { first, duplicate } => {
            assert_eq!(
                text(expected, "kind", case_id),
                "duplicate_phase",
                "{case_id}"
            );
            assert_eq!(number(expected, "first_index", case_id), first, "{case_id}");
            assert_eq!(
                number(expected, "duplicate_index", case_id),
                duplicate,
                "{case_id}"
            );
        }
        ConformanceError::UnknownInitialPhase => {
            assert_eq!(
                text(expected, "kind", case_id),
                "unknown_initial_phase",
                "{case_id}"
            );
        }
        ConformanceError::UnknownEdgeSource { edge } => {
            assert_eq!(
                text(expected, "kind", case_id),
                "unknown_edge_source",
                "{case_id}"
            );
            assert_eq!(number(expected, "edge_index", case_id), edge, "{case_id}");
        }
        ConformanceError::UnknownEdgeTarget { edge } => {
            assert_eq!(
                text(expected, "kind", case_id),
                "unknown_edge_target",
                "{case_id}"
            );
            assert_eq!(number(expected, "edge_index", case_id), edge, "{case_id}");
        }
        ConformanceError::DuplicateWorkflowEdge { first, duplicate } => {
            assert_eq!(
                text(expected, "kind", case_id),
                "duplicate_workflow_edge",
                "{case_id}"
            );
            assert_eq!(number(expected, "first_index", case_id), first, "{case_id}");
            assert_eq!(
                number(expected, "duplicate_index", case_id),
                duplicate,
                "{case_id}"
            );
        }
        ConformanceError::UnknownCompletionPhase { completion } => {
            assert_eq!(
                text(expected, "kind", case_id),
                "unknown_completion_phase",
                "{case_id}"
            );
            assert_eq!(
                number(expected, "completion_index", case_id),
                completion,
                "{case_id}"
            );
        }
        ConformanceError::DuplicateCompletionPhase { first, duplicate } => {
            assert_eq!(
                text(expected, "kind", case_id),
                "duplicate_completion_phase",
                "{case_id}"
            );
            assert_eq!(number(expected, "first_index", case_id), first, "{case_id}");
            assert_eq!(
                number(expected, "duplicate_index", case_id),
                duplicate,
                "{case_id}"
            );
        }
    }
}

#[test]
fn test_reference_and_origin_bounds_are_exact() {
    let fixture = corpus();
    assert_eq!(number(&fixture, "fixture_schema_version", "corpus"), 1);
    let cases = values(&fixture, "value_cases", "corpus");
    assert_eq!(
        cases.len(),
        51,
        "the complete value corpus must be consumed"
    );

    for case in cases {
        let case_id = text(case, "id", "value case");
        let target = text(case, "target", case_id);
        let bytes = input_bytes(case);
        let observed = match target {
            "namespace" => namespace(&bytes).map(|value| value.as_bytes().to_vec()),
            "scope" => scope(&bytes).map(|value| value.as_bytes().to_vec()),
            "value" => reference_value(&bytes).map(|value| value.as_bytes().to_vec()),
            "species" => species(&bytes).map(|value| value.as_bytes().to_vec()),
            "workflow_id" => workflow_id(&bytes).map(|value| value.as_bytes().to_vec()),
            "phase_id" => phase_id(&bytes).map(|value| value.as_bytes().to_vec()),
            _ => panic!("unsupported value target `{target}` in {case_id}"),
        };
        let expected = field(case, "expected", case_id);
        match text(expected, "outcome", case_id) {
            "ok" => assert_eq!(
                observed.unwrap_or_else(|error| panic!("{case_id} unexpectedly failed: {error:?}")),
                bytes,
                "{case_id} changed bytes"
            ),
            "error" => {
                let error = observed
                    .err()
                    .unwrap_or_else(|| panic!("{case_id} unexpectedly succeeded"));
                assert_eq!(
                    number(
                        field(expected, "error", case_id),
                        "input_byte_length",
                        case_id
                    ),
                    bytes.len(),
                    "{case_id} input length"
                );
                assert_conformance_error(error, field(expected, "error", case_id), case_id);
            }
            outcome => panic!("unsupported outcome `{outcome}` in {case_id}"),
        }
    }

    let origin = field(
        field(&fixture, "shared_values", "corpus"),
        "origin",
        "shared_values",
    );
    let parsed = external_reference(
        text(origin, "namespace", "shared origin").as_bytes(),
        text(origin, "scope", "shared origin").as_bytes(),
        text(origin, "value", "shared origin").as_bytes(),
    )
    .expect("the independently authored required origin must satisfy all exact bounds");
    assert_eq!(
        parsed.namespace().as_str(),
        text(origin, "namespace", "shared origin")
    );
    assert_eq!(
        parsed.scope().as_str(),
        text(origin, "scope", "shared origin")
    );
    assert_eq!(
        parsed.value().as_str(),
        text(origin, "value", "shared origin")
    );
}

fn expand_text_values(value: &Value, context: &str) -> Vec<String> {
    if let Some(items) = value.as_array() {
        return items
            .iter()
            .map(|item| {
                item.as_str()
                    .unwrap_or_else(|| panic!("{context} entries must be text"))
                    .to_owned()
            })
            .collect();
    }
    let generator = text(value, "generator", context);
    match generator {
        "indexed_text" => {
            let prefix = text(value, "prefix", context);
            let count = number(value, "count", context);
            let width = number(value, "decimal_width", context);
            (0..count)
                .map(|index| format!("{prefix}{index:0width$}"))
                .collect()
        }
        "repeat" => {
            let repeated = text(value, "value", context);
            vec![repeated.to_owned(); number(value, "count", context)]
        }
        _ => panic!("unsupported text generator `{generator}` in {context}"),
    }
}

fn expand_edges(value: &Value, context: &str) -> Vec<(String, String)> {
    if let Some(items) = value.as_array() {
        return items
            .iter()
            .map(|item| {
                (
                    text(item, "from", context).to_owned(),
                    text(item, "to", context).to_owned(),
                )
            })
            .collect();
    }
    assert_eq!(
        text(value, "generator", context),
        "indexed_edges",
        "{context}"
    );
    let prefix = text(value, "phase_prefix", context);
    let phase_count = number(value, "phase_count", context);
    let edge_count = number(value, "edge_count", context);
    let width = number(value, "decimal_width", context);
    (0..edge_count)
        .map(|index| {
            let source = (index / phase_count) % phase_count;
            let target = index % phase_count;
            (
                format!("{prefix}{source:0width$}"),
                format!("{prefix}{target:0width$}"),
            )
        })
        .collect()
}

fn parse_workflow(value: &Value, context: &str) -> Result<Workflow, ConformanceError> {
    let id = WorkflowId::try_from(text(value, "id", context))
        .unwrap_or_else(|error| panic!("{context} has invalid workflow ID: {error:?}"));
    let phase_text = expand_text_values(field(value, "phases", context), context);
    let phases: Vec<_> = phase_text
        .iter()
        .map(|item| PhaseId::try_from(item.as_str()).expect("fixture phase text must be valid"))
        .collect();
    let initial_phase = PhaseId::try_from(text(value, "initial_phase", context))
        .expect("fixture initial phase text must be valid");
    let edge_text = expand_edges(field(value, "edges", context), context);
    let edges: Vec<_> = edge_text
        .iter()
        .map(|(from, to)| {
            WorkflowEdge::new(
                PhaseId::try_from(from.as_str()).expect("fixture edge source must be valid text"),
                PhaseId::try_from(to.as_str()).expect("fixture edge target must be valid text"),
            )
        })
        .collect();
    let completion_text = expand_text_values(field(value, "completion_phases", context), context);
    let completion: Vec<_> = completion_text
        .iter()
        .map(|item| {
            PhaseId::try_from(item.as_str()).expect("fixture completion phase must be valid")
        })
        .collect();
    validate_workflow(WorkflowInput {
        id,
        phases: &phases,
        initial_phase,
        edges: &edges,
        completion_phases: &completion,
    })
}

fn assert_workflow_preserved(workflow: &Workflow, input: &Value, context: &str) {
    let expected_phases = expand_text_values(field(input, "phases", context), context);
    let expected_edges = expand_edges(field(input, "edges", context), context);
    let expected_completion =
        expand_text_values(field(input, "completion_phases", context), context);
    assert_eq!(
        workflow.id().as_str(),
        text(input, "id", context),
        "{context}"
    );
    assert_eq!(
        workflow
            .phases()
            .iter()
            .map(PhaseId::as_str)
            .collect::<Vec<_>>(),
        expected_phases
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        "{context} phase order"
    );
    assert_eq!(
        workflow.initial_phase().as_str(),
        text(input, "initial_phase", context)
    );
    assert_eq!(
        workflow
            .edges()
            .iter()
            .map(|edge| (edge.from().as_str(), edge.to().as_str()))
            .collect::<Vec<_>>(),
        expected_edges
            .iter()
            .map(|(from, to)| (from.as_str(), to.as_str()))
            .collect::<Vec<_>>(),
        "{context} edge order"
    );
    assert_eq!(
        workflow
            .completion_phases()
            .iter()
            .map(PhaseId::as_str)
            .collect::<Vec<_>>(),
        expected_completion
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        "{context} completion order"
    );
}

fn parse_uuid(value: &str) -> Option<[u8; 16]> {
    if value.len() != 36
        || ![8, 13, 18, 23]
            .into_iter()
            .all(|index| value.as_bytes()[index] == b'-')
    {
        return None;
    }
    let compact: String = value
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if compact.len() != 32
        || !compact
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    let mut bytes = [0_u8; 16];
    for (index, pair) in compact.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] = u8::from_str_radix(core::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    Some(bytes)
}

fn shared<'a>(fixture: &'a Value, key: &str) -> &'a Value {
    field(
        field(fixture, "shared_values", "corpus"),
        key,
        "shared_values",
    )
}

fn unit_id_from_ref(fixture: &Value, key: &str) -> IntentUnitId {
    let value = shared(fixture, key)
        .as_str()
        .unwrap_or_else(|| panic!("shared unit ID {key} must be text"));
    IntentUnitId::from_bytes(parse_uuid(value).expect("shared unit IDs must be canonical UUIDs"))
}

fn reference_from_value(value: &Value, context: &str) -> ExternalReference {
    external_reference(
        text(value, "namespace", context).as_bytes(),
        text(value, "scope", context).as_bytes(),
        text(value, "value", context).as_bytes(),
    )
    .unwrap_or_else(|error| panic!("{context} contains an invalid reference: {error:?}"))
}

#[derive(Clone, Debug)]
enum RecordSpec {
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

fn expand_history(value: &Value, context: &str) -> Vec<RecordSpec> {
    if let Some(records) = value.as_array() {
        return records
            .iter()
            .map(|record| {
                let sequence = field(record, "sequence", context)
                    .as_u64()
                    .expect("history sequence must be an integer");
                match text(record, "type", context) {
                    "transition" => RecordSpec::Transition {
                        sequence,
                        from: text(record, "from", context).to_owned(),
                        to: text(record, "to", context).to_owned(),
                    },
                    "completion" => RecordSpec::Completion {
                        sequence,
                        phase: text(record, "phase", context).to_owned(),
                    },
                    kind => panic!("unsupported history record `{kind}` in {context}"),
                }
            })
            .collect();
    }
    assert_eq!(
        text(value, "generator", context),
        "alternating_transition_history",
        "{context}"
    );
    let first = text(value, "first", context);
    let second = text(value, "second", context);
    let count = number(value, "count", context);
    (0..count)
        .map(|index| {
            let (from, to) = if index % 2 == 0 {
                (first, second)
            } else {
                (second, first)
            };
            RecordSpec::Transition {
                sequence: u64::try_from(index + 1).expect("history length fits u64"),
                from: from.to_owned(),
                to: to.to_owned(),
            }
        })
        .collect()
}

#[derive(Clone)]
struct StateSpec {
    id: IntentUnitId,
    origin: ExternalReference,
    species: IntentSpecies,
    workflow: Workflow,
    phase: PhaseId,
    status: IntentUnitStatus,
    history: Vec<RecordSpec>,
    revision: u64,
}

fn state_spec(fixture: &Value, value: &Value, context: &str) -> StateSpec {
    let id = unit_id_from_ref(fixture, text(value, "id_ref", context));
    let origin = reference_from_value(shared(fixture, text(value, "origin_ref", context)), context);
    let species = IntentSpecies::try_from(text(value, "species", context))
        .expect("fixture species must be valid");
    let workflow = parse_workflow(
        shared(fixture, text(value, "workflow_ref", context)),
        context,
    )
    .expect("shared lifecycle workflow must be valid");
    let phase = PhaseId::try_from(text(value, "phase", context))
        .expect("fixture state phase must be valid");
    let status = match text(value, "status", context) {
        "active" => IntentUnitStatus::Active,
        "completed" => IntentUnitStatus::Completed,
        status => panic!("unsupported status `{status}` in {context}"),
    };
    let history = expand_history(field(value, "history", context), context);
    let revision = field(value, "revision", context)
        .as_u64()
        .expect("fixture revision must be an integer");
    StateSpec {
        id,
        origin,
        species,
        workflow,
        phase,
        status,
        history,
        revision,
    }
}

#[derive(Encode)]
struct WireTransition {
    sequence: u64,
    from: PhaseId,
    to: PhaseId,
}

#[derive(Encode)]
struct WireCompletion {
    sequence: u64,
    phase: PhaseId,
}

#[derive(Encode)]
enum WireRecord {
    #[codec(index = 0)]
    Transition(WireTransition),
    #[codec(index = 1)]
    Completion(WireCompletion),
}

#[derive(Encode)]
struct WireState {
    id: IntentUnitId,
    origin: ExternalReference,
    species: IntentSpecies,
    workflow: Workflow,
    phase: PhaseId,
    status: IntentUnitStatus,
    history: Vec<WireRecord>,
    revision: u64,
}

fn encode_state(spec: &StateSpec) -> Vec<u8> {
    let history = spec
        .history
        .iter()
        .map(|record| match record {
            RecordSpec::Transition { sequence, from, to } => {
                WireRecord::Transition(WireTransition {
                    sequence: *sequence,
                    from: PhaseId::try_from(from.as_str()).expect("history source must be valid"),
                    to: PhaseId::try_from(to.as_str()).expect("history target must be valid"),
                })
            }
            RecordSpec::Completion { sequence, phase } => WireRecord::Completion(WireCompletion {
                sequence: *sequence,
                phase: PhaseId::try_from(phase.as_str()).expect("history phase must be valid"),
            }),
        })
        .collect();
    WireState {
        id: spec.id,
        origin: spec.origin.clone(),
        species: spec.species.clone(),
        workflow: spec.workflow.clone(),
        phase: spec.phase.clone(),
        status: spec.status,
        history,
        revision: spec.revision,
    }
    .encode()
}

fn decode_state(spec: &StateSpec) -> Result<IntentUnitState, parity_scale_codec::Error> {
    let encoded = encode_state(spec);
    let mut input = encoded.as_slice();
    let state = IntentUnitState::decode(&mut input)?;
    if !input.is_empty() {
        return Err("lifecycle state left trailing bytes".into());
    }
    Ok(state)
}

fn replay_state(spec: &StateSpec) -> IntentUnitState {
    let mut state = IntentUnitState::new(
        spec.id,
        spec.origin.clone(),
        spec.species.clone(),
        spec.workflow.clone(),
    );
    for record in &spec.history {
        match record {
            RecordSpec::Transition { sequence, from, to } => {
                assert_eq!(state.phase().as_str(), from);
                let committed = state
                    .transition_to(
                        &PhaseId::try_from(to.as_str()).expect("history target must be valid"),
                        state.revision(),
                    )
                    .expect("fixture history transitions must replay");
                assert_eq!(committed, *sequence);
            }
            RecordSpec::Completion { sequence, phase } => {
                assert_eq!(state.phase().as_str(), phase);
                let committed = state
                    .complete(state.revision())
                    .expect("fixture completion history must replay");
                assert_eq!(committed, *sequence);
            }
        }
    }
    state
}

fn status_name(status: IntentUnitStatus) -> &'static str {
    match status {
        IntentUnitStatus::Active => "active",
        IntentUnitStatus::Completed => "completed",
    }
}

fn assert_history(actual: &[LifecycleRecord], expected: &[Value], context: &str) {
    assert_eq!(actual.len(), expected.len(), "{context} history length");
    for (record, expected) in actual.iter().zip(expected) {
        match record {
            LifecycleRecord::Transition(record) => {
                assert_eq!(text(expected, "type", context), "transition", "{context}");
                assert_eq!(
                    record.sequence(),
                    field(expected, "sequence", context).as_u64().unwrap()
                );
                assert_eq!(record.from().as_str(), text(expected, "from", context));
                assert_eq!(record.to().as_str(), text(expected, "to", context));
            }
            LifecycleRecord::Completion(record) => {
                assert_eq!(text(expected, "type", context), "completion", "{context}");
                assert_eq!(
                    record.sequence(),
                    field(expected, "sequence", context).as_u64().unwrap()
                );
                assert_eq!(record.phase().as_str(), text(expected, "phase", context));
            }
        }
    }
}

fn assert_state_projection(state: &IntentUnitState, expected: &Value, context: &str) {
    assert_eq!(
        state.phase().as_str(),
        text(expected, "phase", context),
        "{context}"
    );
    assert_eq!(
        status_name(state.status()),
        text(expected, "status", context),
        "{context}"
    );
    assert_eq!(
        state.revision(),
        field(expected, "revision", context).as_u64().unwrap()
    );
    assert_history(
        state.history(),
        values(expected, "history", context),
        context,
    );
}

fn assert_lifecycle_error(error: LifecycleError, expected: &Value, operation: &str, case_id: &str) {
    match error {
        LifecycleError::RevisionConflict {
            expected: stale,
            actual,
        } => {
            assert_eq!(
                text(expected, "kind", case_id),
                "revision_conflict",
                "{case_id}"
            );
            assert_eq!(
                field(expected, "expected_revision", case_id).as_u64(),
                Some(stale)
            );
            assert_eq!(
                field(expected, "actual_revision", case_id).as_u64(),
                Some(actual)
            );
        }
        LifecycleError::HistoryCapacityExceeded { length, maximum } => {
            assert_eq!(
                text(expected, "kind", case_id),
                "lifecycle_history_capacity_exceeded",
                "{case_id}"
            );
            assert_eq!(number(expected, "length", case_id), length);
            assert_eq!(number(expected, "maximum", case_id), maximum);
        }
        LifecycleError::AlreadyCompleted => {
            let kind = if operation == "transition" {
                "transition_already_completed"
            } else {
                "completion_already_completed"
            };
            assert_eq!(text(expected, "kind", case_id), kind, "{case_id}");
        }
        LifecycleError::UnknownTarget { target } => {
            assert_eq!(
                text(expected, "kind", case_id),
                "transition_unknown_target",
                "{case_id}"
            );
            assert_eq!(text(expected, "value", case_id), target.as_str());
        }
        LifecycleError::TransitionNotAllowed { from, to } => {
            assert_eq!(
                text(expected, "kind", case_id),
                "transition_not_allowed",
                "{case_id}"
            );
            assert_eq!(text(expected, "from", case_id), from.as_str());
            assert_eq!(text(expected, "to", case_id), to.as_str());
        }
        LifecycleError::CompletionPhaseNotEligible { phase } => {
            assert_eq!(
                text(expected, "kind", case_id),
                "completion_phase_not_eligible",
                "{case_id}"
            );
            assert_eq!(text(expected, "value", case_id), phase.as_str());
        }
        LifecycleError::RevisionExhausted => panic!("{case_id} unexpectedly exhausted revisions"),
    }
}

#[derive(Debug, Eq, PartialEq)]
enum RestoreError {
    TooManyRecords {
        length: usize,
    },
    RevisionMismatch {
        expected: u64,
        actual: u64,
    },
    PhaseMismatch {
        expected: String,
        actual: String,
    },
    StatusMismatch {
        expected: &'static str,
        actual: &'static str,
    },
}

fn classify_restore(spec: &StateSpec) -> Result<(), RestoreError> {
    if spec.history.len() > MAX_LIFECYCLE_RECORDS {
        return Err(RestoreError::TooManyRecords {
            length: spec.history.len(),
        });
    }
    let expected_revision = u64::try_from(spec.history.len()).expect("history length fits u64");
    if spec.revision != expected_revision {
        return Err(RestoreError::RevisionMismatch {
            expected: expected_revision,
            actual: spec.revision,
        });
    }
    let replay = replay_state(spec);
    if spec.phase != *replay.phase() {
        return Err(RestoreError::PhaseMismatch {
            expected: replay.phase().as_str().to_owned(),
            actual: spec.phase.as_str().to_owned(),
        });
    }
    if spec.status != replay.status() {
        return Err(RestoreError::StatusMismatch {
            expected: status_name(replay.status()),
            actual: status_name(spec.status),
        });
    }
    Ok(())
}

fn assert_restore_error(error: &RestoreError, expected: &Value, case_id: &str) {
    match error {
        RestoreError::TooManyRecords { length } => {
            assert_eq!(
                text(expected, "kind", case_id),
                "too_many_lifecycle_records"
            );
            assert_eq!(number(expected, "length", case_id), *length);
            assert_eq!(number(expected, "maximum", case_id), MAX_LIFECYCLE_RECORDS);
        }
        RestoreError::RevisionMismatch {
            expected: revision,
            actual,
        } => {
            assert_eq!(text(expected, "kind", case_id), "revision_history_mismatch");
            assert_eq!(
                field(expected, "expected_revision", case_id).as_u64(),
                Some(*revision)
            );
            assert_eq!(
                field(expected, "actual_revision", case_id).as_u64(),
                Some(*actual)
            );
        }
        RestoreError::PhaseMismatch {
            expected: phase,
            actual,
        } => {
            assert_eq!(text(expected, "kind", case_id), "phase_history_mismatch");
            assert_eq!(text(expected, "expected", case_id), phase);
            assert_eq!(text(expected, "actual", case_id), actual);
        }
        RestoreError::StatusMismatch {
            expected: status,
            actual,
        } => {
            assert_eq!(text(expected, "kind", case_id), "status_history_mismatch");
            assert_eq!(text(expected, "expected", case_id), *status);
            assert_eq!(text(expected, "actual", case_id), *actual);
        }
    }
}

#[derive(Debug)]
enum AdapterError {
    Conformance(ConformanceError),
    InvalidPolicy {
        path: &'static str,
    },
    InvalidUnitId {
        path: &'static str,
        length: usize,
    },
    InvalidSubject {
        path: &'static str,
        reason: &'static str,
    },
}

fn parse_policy(value: &str, path: &'static str) -> Result<RelationshipPolicy, AdapterError> {
    match value {
        "allow" => Ok(RelationshipPolicy::Allow),
        "reject" => Ok(RelationshipPolicy::Reject),
        _ => Err(AdapterError::InvalidPolicy { path }),
    }
}

fn parse_definition(value: &Value, context: &str) -> Result<RelationshipDefinition, AdapterError> {
    let key = field(value, "key", context);
    let id =
        definition_id(text(key, "id", context).as_bytes()).map_err(AdapterError::Conformance)?;
    let version_value = field(key, "version", context)
        .as_u64()
        .expect("definition version must be an integer");
    let version = definition_version(version_value).map_err(AdapterError::Conformance)?;
    assert_eq!(
        field(value, "directed", context).as_bool(),
        Some(true),
        "{context}"
    );
    let source_species = value.get("source_species").map(|item| {
        IntentSpecies::try_from(item.as_str().expect("source species must be text"))
            .expect("source species must satisfy text bounds")
    });
    let target_species = value.get("target_species").map(|item| {
        IntentSpecies::try_from(item.as_str().expect("target species must be text"))
            .expect("target species must satisfy text bounds")
    });
    let self_policy = parse_policy(text(value, "self_policy", context), "/self_policy")?;
    let cycle_policy = parse_policy(text(value, "cycle_policy", context), "/cycle_policy")?;
    Ok(RelationshipDefinition::new(
        DefinitionKey::new(id, version),
        source_species,
        target_species,
        self_policy,
        cycle_policy,
    ))
}

fn parse_id_input(
    fixture: &Value,
    value: &Value,
    name: &'static str,
    context: &str,
) -> Result<IntentUnitId, AdapterError> {
    let reference_name = format!("{name}_ref");
    let raw = if let Some(reference) = value.get(&reference_name) {
        shared(
            fixture,
            reference
                .as_str()
                .unwrap_or_else(|| panic!("{context}.{reference_name} must be text")),
        )
        .as_str()
        .expect("shared UUID must be text")
    } else {
        text(value, name, context)
    };
    let path = if name == "source_id" {
        "/source_id"
    } else {
        "/target_id"
    };
    parse_uuid(raw)
        .map(IntentUnitId::from_bytes)
        .ok_or(AdapterError::InvalidUnitId {
            path,
            length: raw.len(),
        })
}

fn parse_relationship_key(
    fixture: &Value,
    value: &Value,
    context: &str,
) -> Result<RelationshipKey, AdapterError> {
    let definition = field(value, "definition", context);
    let id = definition_id(text(definition, "id", context).as_bytes())
        .map_err(AdapterError::Conformance)?;
    let version = definition_version(
        field(definition, "version", context)
            .as_u64()
            .expect("relationship version must be an integer"),
    )
    .map_err(AdapterError::Conformance)?;
    Ok(RelationshipKey::new(
        DefinitionKey::new(id, version),
        parse_id_input(fixture, value, "source_id", context)?,
        parse_id_input(fixture, value, "target_id", context)?,
    ))
}

fn parse_association(
    fixture: &Value,
    value: &Value,
    context: &str,
) -> Result<AssociationKey, AdapterError> {
    let unit_ref = text(value, "unit_id_ref", context);
    let unit_id = unit_id_from_ref(fixture, unit_ref);
    let subject_value = field(value, "subject", context);
    let subject = match text(subject_value, "type", context) {
        "whole_unit" if subject_value.get("revision").is_none() => AssociationSubject::WholeUnit,
        "whole_unit" => {
            return Err(AdapterError::InvalidSubject {
                path: "/subject/revision",
                reason: "unexpected_value",
            })
        }
        "revision" => AssociationSubject::Revision(
            subject_value
                .get("revision")
                .and_then(Value::as_u64)
                .ok_or(AdapterError::InvalidSubject {
                    path: "/subject/revision",
                    reason: "missing_required_value",
                })?,
        ),
        _ => {
            return Err(AdapterError::InvalidSubject {
                path: "/subject/type",
                reason: "invalid_tag",
            })
        }
    };
    let reference = reference_from_value(
        shared(fixture, text(value, "reference_ref", context)),
        context,
    );
    Ok(AssociationKey::new(unit_id, subject, reference))
}

fn assert_adapter_error(error: AdapterError, expected: &Value, case_id: &str) {
    match error {
        AdapterError::Conformance(error) => assert_conformance_error(error, expected, case_id),
        AdapterError::InvalidPolicy { path } => {
            assert_eq!(
                text(expected, "kind", case_id),
                "invalid_relationship_policy"
            );
            assert_eq!(text(expected, "path", case_id), path);
            assert_eq!(
                values(expected, "allowed", case_id)
                    .iter()
                    .map(|value| value.as_str().unwrap())
                    .collect::<Vec<_>>(),
                ["allow", "reject"]
            );
        }
        AdapterError::InvalidUnitId { path, length } => {
            assert_eq!(text(expected, "kind", case_id), "invalid_intent_unit_id");
            assert_eq!(text(expected, "path", case_id), path);
            assert_eq!(number(expected, "input_byte_length", case_id), length);
        }
        AdapterError::InvalidSubject { path, reason } => {
            assert_eq!(
                text(expected, "kind", case_id),
                "invalid_association_subject"
            );
            assert_eq!(text(expected, "path", case_id), path);
            if reason == "invalid_tag" {
                assert_eq!(text(expected, "value", case_id), "latest");
                assert_eq!(
                    values(expected, "allowed", case_id)
                        .iter()
                        .map(|value| value.as_str().unwrap())
                        .collect::<Vec<_>>(),
                    ["whole_unit", "revision"]
                );
            } else {
                assert_eq!(text(expected, "reason", case_id), reason);
            }
        }
    }
}

fn assert_scale_round_trip<T>(value: &T)
where
    T: Clone + core::fmt::Debug + Decode + Encode + Eq,
{
    let encoded = value.encode();
    let mut input = encoded.as_slice();
    let decoded = T::decode(&mut input).expect("valid bounded value must SCALE-decode");
    assert!(input.is_empty(), "valid bounded value left SCALE bytes");
    assert_eq!(&decoded, value);
}

fn evaluate_workflow_cases(fixture: &Value) {
    let cases = values(fixture, "workflow_cases", "corpus");
    assert_eq!(cases.len(), 14);
    for case in cases {
        let case_id = text(case, "id", "workflow case");
        let input = field(case, "input", case_id);
        let observed = parse_workflow(input, case_id);
        let expected = field(case, "expected", case_id);
        match text(expected, "outcome", case_id) {
            "ok" => {
                let workflow = observed
                    .unwrap_or_else(|error| panic!("{case_id} unexpectedly failed: {error:?}"));
                assert_workflow_preserved(&workflow, input, case_id);
                assert_scale_round_trip(&workflow);
            }
            "error" => assert_conformance_error(
                observed
                    .err()
                    .unwrap_or_else(|| panic!("{case_id} unexpectedly succeeded")),
                expected_error(case),
                case_id,
            ),
            outcome => panic!("unsupported workflow outcome `{outcome}`"),
        }
    }
}

fn evaluate_lifecycle_cases(fixture: &Value) {
    let cases = values(fixture, "lifecycle_cases", "corpus");
    assert_eq!(cases.len(), 15);
    for case in cases {
        let case_id = text(case, "id", "lifecycle case");
        let spec = state_spec(fixture, field(case, "state", case_id), case_id);
        let operation = field(case, "operation", case_id);
        let operation_type = text(operation, "type", case_id);
        let expected = field(case, "expected", case_id);
        if operation_type == "validate_restore" {
            let classification = classify_restore(&spec);
            let decoded = decode_state(&spec);
            match text(expected, "outcome", case_id) {
                "ok" => {
                    classification.expect("valid restoration must classify as valid");
                    let decoded = decoded.expect("valid restoration must pass production Decode");
                    assert_eq!(decoded, replay_state(&spec), "{case_id}");
                    assert_eq!(
                        decoded.encode(),
                        encode_state(&spec),
                        "{case_id} preservation"
                    );
                }
                "error" => {
                    let classification = classification
                        .err()
                        .unwrap_or_else(|| panic!("{case_id} unexpectedly classified as valid"));
                    assert_restore_error(&classification, expected_error(case), case_id);
                    assert!(decoded.is_err(), "{case_id} passed production Decode");
                }
                outcome => panic!("unsupported restoration outcome `{outcome}`"),
            }
            continue;
        }

        classify_restore(&spec).expect("operation prestate must be internally consistent");
        let mut state =
            decode_state(&spec).expect("operation prestate must pass production Decode");
        let original = state.clone();
        let result = match operation_type {
            "transition" => state.transition_to(
                &PhaseId::try_from(text(operation, "target", case_id))
                    .expect("operation target must satisfy text bounds"),
                field(operation, "expected_revision", case_id)
                    .as_u64()
                    .unwrap(),
            ),
            "complete" => state.complete(
                field(operation, "expected_revision", case_id)
                    .as_u64()
                    .expect("expected revision must be an integer"),
            ),
            kind => panic!("unsupported lifecycle operation `{kind}` in {case_id}"),
        };
        match text(expected, "outcome", case_id) {
            "ok" => {
                result.unwrap_or_else(|error| panic!("{case_id} unexpectedly failed: {error:?}"));
                assert_state_projection(&state, field(expected, "state", case_id), case_id);
                assert_eq!(state.id(), original.id(), "{case_id} id changed");
                assert_eq!(
                    state.origin(),
                    original.origin(),
                    "{case_id} origin changed"
                );
                assert_eq!(
                    state.species(),
                    original.species(),
                    "{case_id} species changed"
                );
                assert_eq!(
                    state.workflow(),
                    original.workflow(),
                    "{case_id} workflow changed"
                );
            }
            "error" => {
                assert_lifecycle_error(
                    result
                        .err()
                        .unwrap_or_else(|| panic!("{case_id} unexpectedly succeeded")),
                    expected_error(case),
                    operation_type,
                    case_id,
                );
                assert_eq!(state, original, "{case_id} mutated rejected state");
            }
            outcome => panic!("unsupported lifecycle outcome `{outcome}`"),
        }
    }
}

fn evaluate_relationship_cases(fixture: &Value) {
    let cases = values(fixture, "relationship_cases", "corpus");
    assert_eq!(cases.len(), 8);
    for case in cases {
        let case_id = text(case, "id", "relationship case");
        let kind = text(case, "kind", case_id);
        let input = if let Some(reference) = case.get("input_ref") {
            shared(fixture, reference.as_str().expect("input_ref must be text"))
        } else {
            field(case, "input", case_id)
        };
        let expected = field(case, "expected", case_id);
        match kind {
            "definition" => match (
                parse_definition(input, case_id),
                text(expected, "outcome", case_id),
            ) {
                (Ok(definition), "ok") => {
                    assert_eq!(definition.direction(), RelationshipDirection::Directed);
                    assert_scale_round_trip(&definition);
                }
                (Err(error), "error") => assert_adapter_error(error, expected_error(case), case_id),
                (Ok(_), "error") => panic!("{case_id} unexpectedly succeeded"),
                (Err(error), "ok") => panic!("{case_id} unexpectedly failed: {error:?}"),
                (_, outcome) => panic!("unsupported relationship outcome `{outcome}`"),
            },
            "edge" => match (
                parse_relationship_key(fixture, input, case_id),
                text(expected, "outcome", case_id),
            ) {
                (Ok(key), "ok") => assert_scale_round_trip(&key),
                (Err(error), "error") => assert_adapter_error(error, expected_error(case), case_id),
                (Ok(_), "error") => panic!("{case_id} unexpectedly succeeded"),
                (Err(error), "ok") => panic!("{case_id} unexpectedly failed: {error:?}"),
                (_, outcome) => panic!("unsupported relationship outcome `{outcome}`"),
            },
            _ => panic!("unsupported relationship kind `{kind}`"),
        }
    }
}

fn evaluate_provenance_cases(fixture: &Value) {
    let cases = values(fixture, "provenance_cases", "corpus");
    assert_eq!(cases.len(), 7);
    let mut whole_unit = None;
    for case in cases {
        let case_id = text(case, "id", "provenance case");
        let expected = field(case, "expected", case_id);
        match (
            parse_association(fixture, field(case, "input", case_id), case_id),
            text(expected, "outcome", case_id),
        ) {
            (Ok(key), "ok") => {
                assert_scale_round_trip(&key);
                if case_id == "association-whole-unit-valid" {
                    whole_unit = Some(key);
                } else if case_id == "association-exact-revision-zero-valid-and-distinct" {
                    assert_ne!(Some(&key), whole_unit.as_ref());
                    assert_eq!(key.subject(), AssociationSubject::Revision(0));
                }
            }
            (Err(error), "error") => assert_adapter_error(error, expected_error(case), case_id),
            (Ok(_), "error") => panic!("{case_id} unexpectedly succeeded"),
            (Err(error), "ok") => panic!("{case_id} unexpectedly failed: {error:?}"),
            (_, outcome) => panic!("unsupported provenance outcome `{outcome}`"),
        }
    }
}

fn evaluate_chain_only_cases(fixture: &Value) {
    let cases = values(fixture, "chain_only_cases", "corpus");
    assert_eq!(cases.len(), 8);
    for case in cases {
        let case_id = text(case, "id", "chain-only case");
        let core = field(case, "core", case_id);
        assert_eq!(
            text(core, "outcome", case_id),
            "not_applicable",
            "{case_id}"
        );
        let operation = field(case, "operation", case_id);
        let chain = field(case, "chain", case_id);
        match text(operation, "type", case_id) {
            "create_intent_unit" => {
                assert_eq!(text(core, "reason", case_id), "authorization_is_chain_only");
                assert_eq!(
                    require_authorized(false),
                    Err(ChainOnlyError::UnauthorizedSubmitter)
                );
                assert_eq!(
                    text(field(chain, "error", case_id), "kind", case_id),
                    "unauthorized_submitter"
                );
                assert_eq!(number(chain, "events", case_id), 0);
            }
            "create_relationship" => {
                assert_eq!(
                    text(core, "reason", case_id),
                    "storage_capacity_is_chain_only"
                );
                let count = number(
                    field(field(case, "prestate", case_id), "edges", case_id),
                    "count",
                    case_id,
                );
                match text(chain, "outcome", case_id) {
                    "ok" => {
                        require_relationship_capacity(count)
                            .expect("127 edges must admit one more");
                        assert_eq!(number(chain, "resulting_edge_count", case_id), count + 1);
                        assert_eq!(number(chain, "events", case_id), 1);
                    }
                    "error" => {
                        assert_eq!(
                            require_relationship_capacity(count),
                            Err(ChainOnlyError::RelationshipEdgeCapacityExceeded {
                                length: count,
                                maximum: MAX_RELATIONSHIP_EDGES,
                            })
                        );
                        assert_eq!(
                            number(field(chain, "error", case_id), "length", case_id),
                            count
                        );
                        assert_eq!(number(chain, "events", case_id), 0);
                    }
                    outcome => panic!("unsupported chain outcome `{outcome}`"),
                }
            }
            "record_association" => {
                assert_eq!(
                    text(core, "reason", case_id),
                    "storage_capacity_is_chain_only"
                );
                let count = number(
                    field(field(case, "prestate", case_id), "associations", case_id),
                    "count",
                    case_id,
                );
                match text(chain, "outcome", case_id) {
                    "ok" => {
                        require_association_capacity(count)
                            .expect("127 associations must admit one more");
                        assert_eq!(
                            number(chain, "resulting_association_count", case_id),
                            count + 1
                        );
                        assert_eq!(number(chain, "events", case_id), 1);
                    }
                    "error" => {
                        assert_eq!(
                            require_association_capacity(count),
                            Err(ChainOnlyError::ActiveAssociationCapacityExceeded {
                                length: count,
                                maximum: MAX_ACTIVE_ASSOCIATIONS,
                            })
                        );
                        assert_eq!(
                            number(field(chain, "error", case_id), "length", case_id),
                            count
                        );
                        assert_eq!(number(chain, "events", case_id), 0);
                    }
                    outcome => panic!("unsupported chain outcome `{outcome}`"),
                }
            }
            "replace_authorized_submitters" => {
                assert_eq!(text(core, "reason", case_id), "authorization_is_chain_only");
                let submitters = field(operation, "submitters", case_id);
                let count = if let Some(items) = submitters.as_array() {
                    items.len()
                } else {
                    assert_eq!(
                        text(submitters, "generator", case_id),
                        "indexed_account_id32",
                        "{case_id}"
                    );
                    let start = number(submitters, "start", case_id);
                    let count = number(submitters, "count", case_id);
                    assert!(start > 0, "{case_id} account sequence must be nonzero");
                    assert!(
                        start.checked_add(count).is_some(),
                        "{case_id} account sequence must not overflow"
                    );
                    count
                };
                match text(chain, "outcome", case_id) {
                    "ok" => {
                        assert_eq!(require_authorized_submitter_capacity(count), Ok(()));
                        assert_eq!(
                            number(chain, "resulting_authorized_submitter_count", case_id),
                            count
                        );
                        assert_eq!(number(chain, "administrative_events", case_id), 1);
                        assert_eq!(number(chain, "domain_events", case_id), 0);
                    }
                    "error" => {
                        assert_eq!(
                            require_authorized_submitter_capacity(count),
                            Err(ChainOnlyError::AuthorizedSubmitterCapacityExceeded {
                                length: count,
                                maximum: MAX_AUTHORIZED_SUBMITTERS,
                            })
                        );
                        let error = field(chain, "error", case_id);
                        assert_eq!(
                            text(error, "kind", case_id),
                            "authorized_submitter_capacity_exceeded"
                        );
                        assert_eq!(text(error, "path", case_id), "/submitters");
                        assert_eq!(number(error, "length", case_id), count);
                        assert_eq!(number(error, "maximum", case_id), MAX_AUTHORIZED_SUBMITTERS);
                        assert_eq!(number(chain, "administrative_events", case_id), 0);
                        assert_eq!(number(chain, "domain_events", case_id), 0);
                    }
                    outcome => panic!("unsupported allowlist outcome `{outcome}`"),
                }
            }
            kind => panic!("unsupported chain-only operation `{kind}`"),
        }
    }
}

fn evaluate_bounded_limit_cases(fixture: &Value) {
    let cases = values(fixture, "bounded_limit_cases", "corpus");
    assert_eq!(cases.len(), 4);
    for case in cases {
        let case_id = text(case, "id", "bounded limit case");
        assert_eq!(text(case, "target", case_id), "projected_query_limit");
        assert_eq!(text(case, "path", case_id), "/limit");
        let input = usize::try_from(
            field(case, "input", case_id)
                .as_u64()
                .expect("query limit input must be an integer"),
        )
        .expect("query limit must fit usize");
        let expected = field(case, "expected", case_id);
        match text(expected, "outcome", case_id) {
            "ok" => {
                assert_eq!(projected_query_limit(input), Ok(input), "{case_id}");
                assert_eq!(number(expected, "value", case_id), input, "{case_id}");
            }
            "error" => {
                let error = field(expected, "error", case_id);
                assert_eq!(text(error, "kind", case_id), "query_limit_out_of_range");
                assert_eq!(text(error, "path", case_id), "/limit");
                assert_eq!(number(error, "value", case_id), input);
                let minimum = number(error, "minimum", case_id);
                let maximum = number(error, "maximum", case_id);
                assert_eq!(
                    projected_query_limit(input),
                    Err(BoundedLimitError::QueryLimitOutOfRange {
                        value: input,
                        minimum,
                        maximum,
                    }),
                    "{case_id}"
                );
            }
            outcome => panic!("unsupported bounded-limit outcome `{outcome}`"),
        }
    }
}

#[test]
fn test_core_chain_conformance_corpus_matches() {
    let fixture = corpus();
    evaluate_workflow_cases(&fixture);
    evaluate_lifecycle_cases(&fixture);
    evaluate_relationship_cases(&fixture);
    evaluate_provenance_cases(&fixture);
    evaluate_chain_only_cases(&fixture);
    evaluate_bounded_limit_cases(&fixture);
}

fn phase(value: &str) -> PhaseId {
    PhaseId::try_from(value).expect("test phase must satisfy the bounded text contract")
}

fn maximal_text(prefix: &str) -> String {
    let mut value = String::from(prefix);
    value.extend(core::iter::repeat_n('x', MAX_TEXT_BYTES - value.len()));
    value
}

fn maximal_workflow() -> Workflow {
    let phases: Vec<_> = (0..MAX_WORKFLOW_PHASES)
        .map(|index| phase(&maximal_text(&format!("p{index:02}"))))
        .collect();
    let edges: Vec<_> = (0..MAX_WORKFLOW_EDGES)
        .map(|index| {
            WorkflowEdge::new(
                phases[index / MAX_WORKFLOW_PHASES].clone(),
                phases[index % MAX_WORKFLOW_PHASES].clone(),
            )
        })
        .collect();
    let completion: Vec<_> = phases.iter().take(MAX_COMPLETION_PHASES).cloned().collect();
    Workflow::try_new(
        WorkflowId::try_from(maximal_text("workflow").as_str())
            .expect("maximal workflow ID must be valid"),
        &phases,
        phases[0].clone(),
        &edges,
        &completion,
    )
    .expect("maximal workflow must be valid")
}

fn maximal_reference() -> ExternalReference {
    ExternalReference::new(
        Namespace::try_from("a23456789012345678901234567890123456789012345678901234567890123")
            .expect("64-byte namespace must be valid"),
        ReferenceScope::try_from(maximal_text("scope").as_str())
            .expect("maximal scope must be valid"),
        ReferenceValue::try_from(maximal_text("value").as_str())
            .expect("maximal value must be valid"),
    )
}

fn assert_bounded_type<T>()
where
    T: Decode + DecodeWithMemTracking + Encode + MaxEncodedLen + TypeInfo,
{
    assert!(
        T::max_encoded_len() <= MAX_ACCEPTED_EVENT_BYTES,
        "{} has an excessive maximum SCALE length of {}",
        core::any::type_name::<T>(),
        T::max_encoded_len()
    );
}

#[test]
fn test_chain_types_are_no_std_bounded_and_dependency_clean() {
    let fixture = corpus();
    let limits = field(&fixture, "limits", "corpus");
    let maximum = |name: &str| number(field(limits, name, "limits"), "maximum", name);
    assert_eq!(maximum("namespace_bytes"), MAX_NAMESPACE_BYTES);
    assert_eq!(maximum("text_bytes"), MAX_TEXT_BYTES);
    assert_eq!(maximum("workflow_phases"), MAX_WORKFLOW_PHASES);
    assert_eq!(maximum("workflow_edges"), MAX_WORKFLOW_EDGES);
    assert_eq!(maximum("workflow_completion_phases"), MAX_COMPLETION_PHASES);
    assert_eq!(maximum("lifecycle_records"), MAX_LIFECYCLE_RECORDS);
    assert_eq!(
        maximum("relationship_edges_per_definition"),
        MAX_RELATIONSHIP_EDGES
    );
    assert_eq!(
        maximum("active_associations_per_unit"),
        MAX_ACTIVE_ASSOCIATIONS
    );
    assert_eq!(maximum("authorized_submitters"), MAX_AUTHORIZED_SUBMITTERS);
    assert_eq!(
        maximum("accepted_event_scale_bytes"),
        MAX_ACCEPTED_EVENT_BYTES
    );

    assert_bounded_type::<Text256>();
    assert_bounded_type::<Namespace>();
    assert_bounded_type::<ExternalReference>();
    assert_bounded_type::<Workflow>();
    assert_bounded_type::<IntentUnitState>();
    assert_bounded_type::<RelationshipDefinition>();
    assert_bounded_type::<RelationshipKey>();
    assert_bounded_type::<AssociationKey>();
    assert_bounded_type::<CreateUnitPayload>();
    assert_bounded_type::<DomainPayload>();
    assert!(accepted_event_bound_is_valid(
        DomainPayload::max_encoded_len()
    ));

    let unit_id = IntentUnitId::from_bytes([0xff; 16]);
    let origin = maximal_reference();
    let workflow = maximal_workflow();
    let definition = DefinitionKey::new(
        Namespace::try_from("a23456789012345678901234567890123456789012345678901234567890123")
            .expect("maximal definition ID must be valid"),
        crate::types::DefinitionVersion::try_new(u64::MAX)
            .expect("maximum definition version is nonzero"),
    );
    let relationship = RelationshipKey::new(definition.clone(), unit_id, unit_id);
    let association = AssociationKey::new(
        unit_id,
        AssociationSubject::Revision(u64::MAX),
        origin.clone(),
    );
    let fixtures = [
        DomainPayload::UnitCreated(CreateUnitPayload {
            command_schema_version: 1,
            id: unit_id,
            origin,
            species: IntentSpecies::try_from(maximal_text("species").as_str())
                .expect("maximal species must be valid"),
            workflow,
        }),
        DomainPayload::UnitTransitioned {
            unit_id,
            committed_revision: u64::MAX,
            from: phase(&maximal_text("from")),
            to: phase(&maximal_text("to")),
        },
        DomainPayload::UnitCompleted {
            unit_id,
            committed_revision: u64::MAX,
            phase: phase(&maximal_text("phase")),
        },
        DomainPayload::RelationshipDefinitionCreated(RelationshipDefinition::new(
            definition,
            Some(
                IntentSpecies::try_from(maximal_text("source").as_str())
                    .expect("maximal source species must be valid"),
            ),
            Some(
                IntentSpecies::try_from(maximal_text("target").as_str())
                    .expect("maximal target species must be valid"),
            ),
            RelationshipPolicy::Reject,
            RelationshipPolicy::Reject,
        )),
        DomainPayload::RelationshipCreated(relationship.clone()),
        DomainPayload::RelationshipDeleted(relationship),
        DomainPayload::AssociationRecorded(association.clone()),
        DomainPayload::AssociationRevoked(association),
    ];
    for payload in fixtures {
        assert!(payload.encode().len() <= MAX_ACCEPTED_EVENT_BYTES);
    }

    let source = include_str!("../types.rs");
    for forbidden in [
        "std::",
        "String",
        "uuid",
        "filesystem",
        "SystemTime",
        "Instant",
        "rpc",
        "AccountId",
    ] {
        assert!(
            !source.contains(forbidden),
            "bounded chain types contain forbidden dependency surface `{forbidden}`"
        );
    }
    let manifest = include_str!("../../Cargo.toml");
    for forbidden in ["uuid", "tokio", "subxt", "jsonrpsee"] {
        assert!(!manifest.contains(forbidden));
    }
}

fn reference() -> ExternalReference {
    ExternalReference::new(
        Namespace::try_from("book.intent").expect("test namespace must be valid"),
        ReferenceScope::try_from("CubiKan").expect("test scope must be valid"),
        ReferenceValue::try_from("INT-0014").expect("test value must be valid"),
    )
}

fn workflow() -> Workflow {
    let queued = phase("queued");
    let done = phase("done");
    Workflow::try_new(
        WorkflowId::try_from("delivery").expect("test workflow ID must be valid"),
        &[queued.clone(), done.clone()],
        queued.clone(),
        &[WorkflowEdge::new(queued, done.clone())],
        &[done],
    )
    .expect("test workflow must be valid")
}

fn create_payload() -> CreateUnitPayload {
    CreateUnitPayload {
        command_schema_version: 1,
        id: IntentUnitId::from_bytes([0x11; 16]),
        origin: reference(),
        species: IntentSpecies::try_from("work-item").expect("test species must be valid"),
        workflow: workflow(),
    }
}

fn assert_structural_rejection<T: Decode>(bytes: &[u8]) -> StructuralDecodeError {
    let mut counters = BoundaryCounters::default();
    let error = decode_before_dispatch::<T>(bytes, &mut counters)
        .err()
        .unwrap_or_else(|| panic!("malformed SCALE reached dispatch"));
    assert_eq!(counters, BoundaryCounters::default());
    error
}

fn assert_scale_reason(case: &Value, bytes: &[u8]) {
    let case_id = text(case, "id", "SCALE case");
    let target = text(case, "target_type", case_id);
    let expected = expected_error(case);
    match text(expected, "kind", case_id) {
        "unexpected_eof" => {
            let mut input = bytes;
            assert!(u16::decode(&mut input).is_err(), "{case_id}");
            assert_eq!(number(expected, "byte_index", case_id), bytes.len());
        }
        "missing_required_origin" => {
            let mut input = bytes;
            u16::decode(&mut input).expect("schema version prefix must decode");
            IntentUnitId::decode(&mut input).expect("caller UUID prefix must decode");
            assert!(input.is_empty(), "{case_id} must end exactly before origin");
            assert!(ExternalReference::decode(&mut input).is_err(), "{case_id}");
            assert_eq!(number(expected, "byte_index", case_id), bytes.len());
        }
        "invalid_variant" => {
            assert_eq!(usize::from(bytes[0]), number(expected, "variant", case_id));
            assert_eq!(number(expected, "byte_index", case_id), 0);
        }
        "malformed_compact_length" => {
            let mut input = bytes;
            assert!(Compact::<u32>::decode(&mut input).is_err(), "{case_id}");
            assert_eq!(number(expected, "byte_index", case_id), bytes.len());
        }
        "over_bound_collection" if target == "workflow_phase_collection" => {
            let mut input = bytes;
            let length = Compact::<u32>::decode(&mut input)
                .expect("over-bound vector length prefix must be structurally valid")
                .0;
            assert_eq!(
                usize::try_from(length).unwrap(),
                number(expected, "length", case_id)
            );
            assert_eq!(number(expected, "maximum", case_id), MAX_WORKFLOW_PHASES);
            assert_eq!(number(expected, "byte_index", case_id), 0);
        }
        "over_bound_collection" if target == "external_reference" => {
            let mut input = bytes;
            Namespace::decode(&mut input).expect("namespace prefix must decode");
            let consumed = bytes.len() - input.len();
            let length = Compact::<u32>::decode(&mut input)
                .expect("scope length prefix must be structurally valid")
                .0;
            assert_eq!(
                usize::try_from(length).unwrap(),
                number(expected, "length", case_id)
            );
            assert_eq!(number(expected, "maximum", case_id), MAX_TEXT_BYTES);
            assert_eq!(number(expected, "byte_index", case_id), consumed);
        }
        "invalid_utf8" | "contains_nul" => {
            let mut input = bytes;
            let raw = Vec::<u8>::decode(&mut input).expect("raw text vector must decode");
            assert!(input.is_empty(), "{case_id} must have no trailing bytes");
            let error = Text256::try_from_bytes(&raw)
                .err()
                .unwrap_or_else(|| panic!("{case_id} unexpectedly passed semantic preflight"));
            let mapped = match error {
                crate::types::TextError::InvalidUtf8 { index, length } => {
                    ConformanceError::InvalidUtf8 {
                        field: TextField::Value,
                        index,
                        length,
                    }
                }
                crate::types::TextError::Nul { index } => ConformanceError::Nul {
                    field: TextField::Value,
                    index,
                },
                other => panic!("{case_id} produced the wrong preflight error: {other:?}"),
            };
            assert_conformance_error(mapped, expected, case_id);
        }
        kind => panic!("unsupported SCALE reason `{kind}` for {case_id}"),
    }
}

#[test]
fn test_scale_structural_rejections_never_enter_dispatch() {
    let fixture = corpus();
    let cases = values(&fixture, "scale_preflight_cases", "corpus");
    assert_eq!(cases.len(), 8);
    for case in cases {
        let case_id = text(case, "id", "SCALE case");
        let target = text(case, "target_type", case_id);
        let bytes = decode_hex(text(case, "input_hex", case_id));
        let error = match target {
            "create_unit_payload" => assert_structural_rejection::<CreateUnitPayload>(&bytes),
            "domain_payload" => assert_structural_rejection::<DomainPayload>(&bytes),
            "external_reference" => assert_structural_rejection::<ExternalReference>(&bytes),
            "bounded_text_256" => assert_structural_rejection::<Text256>(&bytes),
            "workflow_phase_collection" => {
                type Phases = BoundedVec<PhaseId, ConstU32<32>>;
                assert_structural_rejection::<Phases>(&bytes)
            }
            _ => panic!("unsupported SCALE target `{target}` in {case_id}"),
        };
        assert_eq!(error, StructuralDecodeError::Codec, "{case_id}");
        let expected = field(case, "expected", case_id);
        assert_eq!(text(expected, "outcome", case_id), "error");
        assert!(matches!(
            text(expected, "stage", case_id),
            "codec" | "preflight"
        ));
        assert_eq!(number(expected, "dispatch_entries", case_id), 0);
        assert_eq!(number(expected, "domain_reads", case_id), 0);
        assert_eq!(number(expected, "mutations", case_id), 0);
        assert_eq!(number(expected, "events", case_id), 0);
        assert_scale_reason(case, &bytes);
    }

    let encoded = create_payload().encode();
    let mut counters = BoundaryCounters::default();
    let decoded = decode_before_dispatch::<CreateUnitPayload>(&encoded, &mut counters)
        .expect("valid SCALE must reach the dispatch boundary");
    assert_eq!(decoded, create_payload());
    assert_eq!(counters.pallet_entries(), 1);
    assert_eq!(counters.domain_reads(), 0);
    assert_eq!(counters.mutations(), 0);
    assert_eq!(counters.accepted_events(), 0);
    assert_eq!(
        require_lifecycle_capacity(MAX_LIFECYCLE_RECORDS),
        Err(ChainOnlyError::LifecycleHistoryCapacityExceeded {
            length: MAX_LIFECYCLE_RECORDS,
            maximum: MAX_LIFECYCLE_RECORDS,
        })
    );
}
