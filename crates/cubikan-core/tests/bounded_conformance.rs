use cubikan_core::{
    AssociationSubject, BoundedWorkflowError, CompletionError, ExternalReference, IntentSpecies,
    IntentUnit, IntentUnitId, IntentUnitRevision, IntentUnitStatus, MAX_COMPLETION_PHASES,
    MAX_NAMESPACE_BYTES, MAX_TEXT_BYTES, MAX_WORKFLOW_EDGES, MAX_WORKFLOW_PHASES, PhaseId,
    RecordedAssociation, ReferenceNamespace, ReferenceNamespaceError, ReferenceText,
    ReferenceTextError, RelationshipDefinition, RelationshipDefinitionKey,
    RelationshipDefinitionVersion, RelationshipIdentity, RelationshipPolicy,
    RevisionedCompletionError, RevisionedTransitionError, TransitionError, Workflow, WorkflowEdge,
    WorkflowError, WorkflowId,
};
use serde_json::{Value, json};
use std::{fs, path::PathBuf, str::FromStr};

fn fixture() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/chain-conformance-v1.json");
    serde_json::from_slice(&fs::read(path).expect("independent conformance fixture must exist"))
        .expect("independent conformance fixture must be valid JSON")
}

fn input_bytes(input: &Value) -> Vec<u8> {
    match (input.get("utf8"), input.get("hex")) {
        (Some(value), None) => value.as_str().unwrap().as_bytes().to_vec(),
        (None, Some(value)) => {
            let value = value.as_str().unwrap();
            (0..value.len())
                .step_by(2)
                .map(|index| u8::from_str_radix(&value[index..index + 2], 16).unwrap())
                .collect()
        }
        _ => panic!("fixture text input must choose exactly one encoding"),
    }
}

#[derive(Debug, Eq, PartialEq)]
struct ObservedValueError {
    kind: &'static str,
    byte_index: Option<usize>,
    invalid_byte_length: Option<usize>,
    maximum: Option<usize>,
    byte_value: Option<u8>,
}

impl ObservedValueError {
    const fn simple(kind: &'static str) -> Self {
        Self {
            kind,
            byte_index: None,
            invalid_byte_length: None,
            maximum: None,
            byte_value: None,
        }
    }
}

fn core_value_outcome(target: &str, bytes: &[u8]) -> Result<Vec<u8>, ObservedValueError> {
    match target {
        "namespace" => ReferenceNamespace::from_bytes(bytes)
            .map(|value| value.as_str().as_bytes().to_vec())
            .map_err(|error| match error {
                ReferenceNamespaceError::Empty => ObservedValueError::simple("empty"),
                ReferenceNamespaceError::TooLong { length, maximum } => {
                    assert_eq!(length, bytes.len());
                    ObservedValueError {
                        maximum: Some(maximum),
                        ..ObservedValueError::simple("too_long")
                    }
                }
                ReferenceNamespaceError::InvalidUtf8 {
                    index,
                    error_length,
                } => ObservedValueError {
                    byte_index: Some(index),
                    invalid_byte_length: error_length,
                    ..ObservedValueError::simple("invalid_utf8")
                },
                ReferenceNamespaceError::Nul { index } => ObservedValueError {
                    byte_index: Some(index),
                    ..ObservedValueError::simple("contains_nul")
                },
                ReferenceNamespaceError::InvalidStart { index, byte } => ObservedValueError {
                    byte_index: Some(index),
                    byte_value: Some(byte),
                    ..ObservedValueError::simple("invalid_namespace_start")
                },
                ReferenceNamespaceError::InvalidByte { index, byte } => ObservedValueError {
                    byte_index: Some(index),
                    byte_value: Some(byte),
                    ..ObservedValueError::simple("invalid_namespace_byte")
                },
            }),
        "scope" | "value" => ReferenceText::from_bytes(bytes)
            .map(|value| value.as_str().as_bytes().to_vec())
            .map_err(|error| match error {
                ReferenceTextError::Empty => ObservedValueError::simple("empty"),
                ReferenceTextError::Blank => ObservedValueError::simple("blank"),
                ReferenceTextError::TooLong { length, maximum } => {
                    assert_eq!(length, bytes.len());
                    ObservedValueError {
                        maximum: Some(maximum),
                        ..ObservedValueError::simple("too_long")
                    }
                }
                ReferenceTextError::InvalidUtf8 {
                    index,
                    error_length,
                } => ObservedValueError {
                    byte_index: Some(index),
                    invalid_byte_length: error_length,
                    ..ObservedValueError::simple("invalid_utf8")
                },
                ReferenceTextError::Nul { index } => ObservedValueError {
                    byte_index: Some(index),
                    ..ObservedValueError::simple("contains_nul")
                },
            }),
        "species" => cubikan_core::IntentSpecies::from_bytes(bytes)
            .map(|value| value.as_str().as_bytes().to_vec())
            .map_err(map_vocabulary_error),
        "workflow_id" => WorkflowId::from_bytes(bytes)
            .map(|value| value.as_str().as_bytes().to_vec())
            .map_err(map_vocabulary_error),
        "phase_id" => PhaseId::from_bytes(bytes)
            .map(|value| value.as_str().as_bytes().to_vec())
            .map_err(map_vocabulary_error),
        _ => panic!("unexpected core value target {target}"),
    }
}

fn map_vocabulary_error(error: cubikan_core::VocabularyValidationError) -> ObservedValueError {
    match error {
        cubikan_core::VocabularyValidationError::Empty => ObservedValueError::simple("empty"),
        cubikan_core::VocabularyValidationError::TooLong { maximum, .. } => ObservedValueError {
            maximum: Some(maximum),
            ..ObservedValueError::simple("too_long")
        },
        cubikan_core::VocabularyValidationError::InvalidUtf8 { index, length } => {
            ObservedValueError {
                byte_index: Some(index),
                invalid_byte_length: Some(length),
                ..ObservedValueError::simple("invalid_utf8")
            }
        }
        cubikan_core::VocabularyValidationError::Nul { index } => ObservedValueError {
            byte_index: Some(index),
            ..ObservedValueError::simple("contains_nul")
        },
        cubikan_core::VocabularyValidationError::Blank => ObservedValueError::simple("blank"),
    }
}

fn expected_usize(value: &Value, key: &str) -> Option<usize> {
    value
        .get(key)
        .map(|value| usize::try_from(value.as_u64().unwrap()).unwrap())
}

fn assert_value_error(case: &Value, bytes: &[u8], actual: &ObservedValueError) {
    let expected = &case["expected"]["error"];
    assert_eq!(
        actual.kind,
        expected["kind"].as_str().unwrap(),
        "{}",
        case["id"]
    );
    assert_eq!(
        expected_usize(expected, "input_byte_length"),
        Some(bytes.len()),
        "{} fixture input length drifted",
        case["id"]
    );
    assert_eq!(
        actual.byte_index,
        expected_usize(expected, "byte_index"),
        "{}",
        case["id"]
    );
    assert_eq!(
        actual.invalid_byte_length,
        expected_usize(expected, "invalid_byte_length"),
        "{}",
        case["id"]
    );
    assert_eq!(
        actual.maximum,
        expected_usize(expected, "maximum"),
        "{}",
        case["id"]
    );
    assert_eq!(
        actual.byte_value,
        expected_usize(expected, "byte_value").map(|value| value as u8),
        "{}",
        case["id"]
    );
}

#[test]
fn test_reference_and_origin_bounds_are_exact() {
    assert_eq!(MAX_NAMESPACE_BYTES, 64);
    assert_eq!(MAX_TEXT_BYTES, 256);

    assert_eq!(
        ReferenceNamespace::new(""),
        Err(ReferenceNamespaceError::Empty)
    );
    assert_eq!(
        ReferenceNamespace::new(format!("a{}", "0".repeat(64))),
        Err(ReferenceNamespaceError::TooLong {
            length: 65,
            maximum: 64,
        })
    );
    assert_eq!(
        ReferenceNamespace::new("a/b"),
        Err(ReferenceNamespaceError::InvalidByte {
            index: 1,
            byte: b'/',
        })
    );
    assert_eq!(
        ReferenceText::new("x\0y"),
        Err(ReferenceTextError::Nul { index: 1 })
    );
    assert_eq!(
        ReferenceText::from_bytes(&[b'a', 0xff, b'b']),
        Err(ReferenceTextError::InvalidUtf8 {
            index: 1,
            error_length: Some(1),
        })
    );
    assert_eq!(
        ReferenceNamespace::from_bytes(&[b'a', 0xff]),
        Err(ReferenceNamespaceError::InvalidUtf8 {
            index: 1,
            error_length: Some(1),
        })
    );
    assert_eq!(
        ReferenceText::new("é".repeat(129)),
        Err(ReferenceTextError::TooLong {
            length: 258,
            maximum: 256,
        })
    );

    let exact = ExternalReference::new(
        ReferenceNamespace::new("git.commit.sha256").unwrap(),
        ReferenceText::new("  owner/repository  ").unwrap(),
        ReferenceText::new("a".repeat(64)).unwrap(),
    );
    assert_eq!(exact.scope().as_str(), "  owner/repository  ");
}

#[test]
fn test_core_chain_conformance_corpus_matches() {
    let fixture = fixture();
    assert_eq!(fixture["fixture_schema_version"], 1);
    assert_eq!(fixture["byte_encoding"], "lowercase_hex_without_prefix");
    assert_eq!(fixture["limits"]["namespace_bytes"]["maximum"], 64);
    assert_eq!(fixture["limits"]["text_bytes"]["maximum"], 256);
    assert_eq!(fixture["limits"]["workflow_phases"]["maximum"], 32);
    assert_eq!(fixture["limits"]["workflow_edges"]["maximum"], 128);
    assert_eq!(
        fixture["limits"]["workflow_completion_phases"]["maximum"],
        32
    );
    let family_names = [
        "value_cases",
        "workflow_cases",
        "lifecycle_cases",
        "relationship_cases",
        "provenance_cases",
        "chain_only_cases",
        "bounded_limit_cases",
        "scale_preflight_cases",
    ];
    assert_eq!(
        family_names
            .iter()
            .map(|name| fixture[name].as_array().unwrap().len())
            .sum::<usize>(),
        115,
        "the independent corpus registry must be closed"
    );

    for case in fixture["value_cases"].as_array().unwrap() {
        let target = case["target"].as_str().unwrap();
        let bytes = input_bytes(&case["input"]);
        let actual = core_value_outcome(target, &bytes);
        match case["expected"]["outcome"].as_str().unwrap() {
            "ok" => assert_eq!(
                actual.unwrap(),
                bytes,
                "{} did not preserve exact input bytes",
                case["id"]
            ),
            "error" => assert_value_error(case, &bytes, &actual.unwrap_err()),
            outcome => panic!("unexpected fixture outcome {outcome}"),
        }
    }

    for case in fixture["workflow_cases"].as_array().unwrap() {
        let input = &case["input"];
        let phases = expand_text_list(&input["phases"]);
        let edges = expand_edge_list(&input["edges"]);
        let completion = expand_text_list(&input["completion_phases"]);
        let result = Workflow::new_bounded(
            WorkflowId::from_bytes(input["id"].as_str().unwrap().as_bytes()).unwrap(),
            phases
                .iter()
                .map(|value| PhaseId::from_bytes(value.as_bytes()).unwrap()),
            PhaseId::from_bytes(input["initial_phase"].as_str().unwrap().as_bytes()).unwrap(),
            edges.iter().map(|(from, to)| {
                WorkflowEdge::new(
                    PhaseId::from_bytes(from.as_bytes()).unwrap(),
                    PhaseId::from_bytes(to.as_bytes()).unwrap(),
                )
            }),
            completion
                .iter()
                .map(|value| PhaseId::from_bytes(value.as_bytes()).unwrap()),
        );
        let expected = case["expected"]["outcome"].as_str().unwrap();
        assert_eq!(result.is_ok(), expected == "ok", "{}", case["id"]);
        if let Err(error) = result {
            assert_eq!(
                workflow_error_kind(&error),
                case["expected"]["error"]["kind"].as_str().unwrap(),
                "{}",
                case["id"]
            );
        }
    }

    let reference = reference_from_value(&fixture["shared_values"]["reference"]);
    let version = RelationshipDefinitionVersion::new(u64::MAX).unwrap();
    let definition =
        RelationshipDefinitionKey::new(ReferenceNamespace::new("depends-on").unwrap(), version);
    let source = "67e55044-10b1-426f-9247-bb680e5fe0c8".parse().unwrap();
    let target = "67e55044-10b1-426f-9247-bb680e5fe0c9".parse().unwrap();
    let relationship = RelationshipIdentity::new(definition, source, target);
    let whole = RecordedAssociation::new(source, AssociationSubject::WholeUnit, reference.clone());
    let zero = RecordedAssociation::new(source, AssociationSubject::Revision(0), reference);

    assert_ne!(whole, zero);
    assert_eq!(relationship.definition().version().value(), u64::MAX);
    for case in fixture["relationship_cases"].as_array().unwrap() {
        assert_relationship_case(case, &fixture);
    }
    for case in fixture["provenance_cases"].as_array().unwrap() {
        assert_provenance_case(case, &fixture);
    }
    assert_eq!(
        fixture["relationship_cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|case| case["expected"]["outcome"] == "ok")
            .count(),
        3
    );
    assert_eq!(
        fixture["provenance_cases"][1]["expected"]["not_equal_to_case"],
        "association-whole-unit-valid"
    );
    evaluate_core_lifecycle_cases(&fixture);

    for case in fixture["chain_only_cases"].as_array().unwrap() {
        assert_eq!(
            case["core"]["outcome"], "not_applicable",
            "{} must remain explicitly chain-only",
            case["id"]
        );
    }
    assert_eq!(fixture["bounded_limit_cases"].as_array().unwrap().len(), 4);
}

fn workflow_error_kind(error: &BoundedWorkflowError) -> &'static str {
    match error {
        BoundedWorkflowError::TooManyPhases { .. } => "too_many_phases",
        BoundedWorkflowError::TooManyEdges { .. } => "too_many_workflow_edges",
        BoundedWorkflowError::TooManyCompletionPhases { .. } => "too_many_completion_phases",
        BoundedWorkflowError::Topology(error) => match error {
            WorkflowError::EmptyPhases => "too_few_phases",
            WorkflowError::DuplicatePhase { .. } => "duplicate_phase",
            WorkflowError::UnknownInitialPhase { .. } => "unknown_initial_phase",
            WorkflowError::UnknownEdgeSource { .. } => "unknown_edge_source",
            WorkflowError::UnknownEdgeTarget { .. } => "unknown_edge_target",
            WorkflowError::DuplicateEdge { .. } => "duplicate_workflow_edge",
            WorkflowError::UnknownCompletionPhase { .. } => "unknown_completion_phase",
            WorkflowError::DuplicateCompletionPhase { .. } => "duplicate_completion_phase",
        },
    }
}

fn expand_history(value: &Value) -> Vec<Value> {
    if let Some(records) = value.as_array() {
        return records
            .iter()
            .map(|record| match record["type"].as_str().unwrap() {
                "transition" => json!({
                    "Transition": {
                        "sequence": record["sequence"],
                        "from": record["from"],
                        "to": record["to"],
                    }
                }),
                "completion" => json!({
                    "Completion": {
                        "sequence": record["sequence"],
                        "final_phase": record["phase"],
                    }
                }),
                kind => panic!("unexpected lifecycle record {kind}"),
            })
            .collect();
    }
    assert_eq!(value["generator"], "alternating_transition_history");
    let count = value["count"].as_u64().unwrap();
    let first = value["first"].as_str().unwrap();
    let second = value["second"].as_str().unwrap();
    (0..count)
        .map(|index| {
            let (from, to) = if index % 2 == 0 {
                (first, second)
            } else {
                (second, first)
            };
            json!({
                "Transition": {
                    "sequence": index + 1,
                    "from": from,
                    "to": to,
                }
            })
        })
        .collect()
}

fn intent_unit_json(fixture: &Value, state: &Value) -> Value {
    let workflow = &fixture["shared_values"][state["workflow_ref"].as_str().unwrap()];
    let history = expand_history(&state["history"]);
    json!({
        "id": fixture["shared_values"][state["id_ref"].as_str().unwrap()],
        "species": state["species"],
        "workflow": workflow,
        "phase": state["phase"],
        "status": match state["status"].as_str().unwrap() {
            "active" => "Active",
            "completed" => "Completed",
            status => panic!("unexpected lifecycle status {status}"),
        },
        "history": history,
        "revision": state["revision"],
    })
}

fn lifecycle_error_kind_transition(error: &RevisionedTransitionError) -> &'static str {
    match error {
        RevisionedTransitionError::Conflict(_) => "revision_conflict",
        RevisionedTransitionError::Transition(TransitionError::AlreadyCompleted) => {
            "transition_already_completed"
        }
        RevisionedTransitionError::Transition(TransitionError::UnknownTarget { .. }) => {
            "transition_unknown_target"
        }
        RevisionedTransitionError::Transition(TransitionError::NotAllowed { .. }) => {
            "transition_not_allowed"
        }
    }
}

fn lifecycle_error_kind_completion(error: &RevisionedCompletionError) -> &'static str {
    match error {
        RevisionedCompletionError::Conflict(_) => "revision_conflict",
        RevisionedCompletionError::Completion(CompletionError::AlreadyCompleted) => {
            "completion_already_completed"
        }
        RevisionedCompletionError::Completion(CompletionError::PhaseNotEligible { .. }) => {
            "completion_phase_not_eligible"
        }
    }
}

fn evaluate_core_lifecycle_cases(fixture: &Value) {
    let cases = fixture["lifecycle_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 15);
    for case in cases {
        let state = &case["state"];
        let operation = case["operation"]["type"].as_str().unwrap();
        let expected = &case["expected"];
        let json = intent_unit_json(fixture, state);
        if operation == "validate_restore" {
            let history_length = expand_history(&state["history"]).len();
            let result = if history_length > 256 {
                Err("too_many_lifecycle_records".to_owned())
            } else {
                serde_json::from_value::<IntentUnit>(json).map_err(|error| error.to_string())
            };
            assert_eq!(
                result.is_ok(),
                expected["outcome"] == "ok",
                "{}",
                case["id"]
            );
            if let Ok(unit) = result {
                assert_eq!(unit.revision().value(), state["revision"].as_u64().unwrap());
                assert_eq!(
                    unit.history().len(),
                    state["revision"].as_u64().unwrap() as usize
                );
                assert_eq!(unit.phase().as_str(), state["phase"].as_str().unwrap());
                assert_eq!(
                    unit.status(),
                    if state["status"] == "active" {
                        IntentUnitStatus::Active
                    } else {
                        IntentUnitStatus::Completed
                    }
                );
            }
            continue;
        }

        let mut unit: IntentUnit =
            serde_json::from_value(json).expect("fixture operation prestate must restore");
        let before = serde_json::to_value(&unit).unwrap();
        let expected_revision =
            IntentUnitRevision::new(case["operation"]["expected_revision"].as_u64().unwrap());
        let result_kind = match operation {
            "transition" if state["history"].as_array().is_none() => {
                Some("lifecycle_history_capacity_exceeded")
            }
            "transition" => {
                let target =
                    PhaseId::from_bytes(case["operation"]["target"].as_str().unwrap().as_bytes())
                        .unwrap();
                unit.transition_to_if_revision(&target, expected_revision)
                    .err()
                    .map(|error| lifecycle_error_kind_transition(&error))
            }
            "complete" => unit
                .complete_if_revision(expected_revision)
                .err()
                .map(|error| lifecycle_error_kind_completion(&error)),
            kind => panic!("unexpected lifecycle operation {kind}"),
        };
        let expected_kind = expected
            .get("error")
            .and_then(|error| error.get("kind"))
            .and_then(Value::as_str);
        assert_eq!(result_kind, expected_kind, "{}", case["id"]);
        if expected["outcome"] == "ok" {
            assert_eq!(
                unit.revision().value(),
                expected["state"]["revision"].as_u64().unwrap()
            );
            assert_eq!(
                unit.phase().as_str(),
                expected["state"]["phase"].as_str().unwrap()
            );
            let expected_status = if expected["state"]["status"] == "active" {
                IntentUnitStatus::Active
            } else {
                IntentUnitStatus::Completed
            };
            assert_eq!(unit.status(), expected_status);
        } else if result_kind != Some("lifecycle_history_capacity_exceeded") {
            assert_eq!(
                serde_json::to_value(&unit).unwrap(),
                before,
                "{}",
                case["id"]
            );
        }
    }
}

fn expand_text_list(value: &Value) -> Vec<String> {
    if let Some(values) = value.as_array() {
        return values
            .iter()
            .map(|value| value.as_str().unwrap().to_owned())
            .collect();
    }
    let generator = value["generator"].as_str().unwrap();
    let count = value["count"].as_u64().unwrap() as usize;
    match generator {
        "indexed_text" => {
            let prefix = value["prefix"].as_str().unwrap();
            let width = value["decimal_width"].as_u64().unwrap() as usize;
            (0..count)
                .map(|index| format!("{prefix}{index:0width$}"))
                .collect()
        }
        "repeat" => vec![value["value"].as_str().unwrap().to_owned(); count],
        _ => panic!("unknown text generator"),
    }
}

fn expand_edge_list(value: &Value) -> Vec<(String, String)> {
    if let Some(values) = value.as_array() {
        return values
            .iter()
            .map(|edge| {
                (
                    edge["from"].as_str().unwrap().to_owned(),
                    edge["to"].as_str().unwrap().to_owned(),
                )
            })
            .collect();
    }
    let prefix = value["phase_prefix"].as_str().unwrap();
    let phases = value["phase_count"].as_u64().unwrap() as usize;
    let count = value["edge_count"].as_u64().unwrap() as usize;
    let width = value["decimal_width"].as_u64().unwrap() as usize;
    (0..count)
        .map(|index| {
            (
                format!("{prefix}{:0width$}", (index / phases) % phases),
                format!("{prefix}{:0width$}", index % phases),
            )
        })
        .collect()
}

fn reference_from_value(value: &Value) -> ExternalReference {
    ExternalReference::new(
        ReferenceNamespace::new(value["namespace"].as_str().unwrap()).unwrap(),
        ReferenceText::new(value["scope"].as_str().unwrap()).unwrap(),
        ReferenceText::new(value["value"].as_str().unwrap()).unwrap(),
    )
}

fn assert_relationship_case(case: &Value, fixture: &Value) {
    let input = if let Some(reference) = case.get("input_ref") {
        &fixture["shared_values"][reference.as_str().unwrap()]
    } else {
        &case["input"]
    };
    let outcome = case["expected"]["outcome"].as_str().unwrap();
    if case["kind"] == "definition" {
        let key = ReferenceNamespace::new(input["key"]["id"].as_str().unwrap());
        let version = RelationshipDefinitionVersion::new(input["key"]["version"].as_u64().unwrap());
        let self_policy = input["self_policy"]
            .as_str()
            .unwrap()
            .parse::<RelationshipPolicy>();
        let cycle_policy = input["cycle_policy"]
            .as_str()
            .unwrap()
            .parse::<RelationshipPolicy>();
        let valid = key.is_ok() && version.is_ok() && self_policy.is_ok() && cycle_policy.is_ok();
        assert_eq!(valid, outcome == "ok", "{}", case["id"]);
        if valid {
            let _ = RelationshipDefinition::new(
                RelationshipDefinitionKey::new(key.unwrap(), version.unwrap()),
                input
                    .get("source_species")
                    .and_then(Value::as_str)
                    .map(|value| IntentSpecies::from_bytes(value.as_bytes()).unwrap()),
                input
                    .get("target_species")
                    .and_then(Value::as_str)
                    .map(|value| IntentSpecies::from_bytes(value.as_bytes()).unwrap()),
                self_policy.unwrap(),
                cycle_policy.unwrap(),
            );
        }
    } else {
        let source = input
            .get("source_id_ref")
            .and_then(Value::as_str)
            .map(|key| {
                fixture["shared_values"][key]
                    .as_str()
                    .unwrap()
                    .parse::<IntentUnitId>()
            });
        assert_eq!(
            source.is_some_and(|value| value.is_ok()),
            outcome == "ok",
            "{}",
            case["id"]
        );
    }
}

fn assert_provenance_case(case: &Value, fixture: &Value) {
    let subject = &case["input"]["subject"];
    let parsed = AssociationSubject::from_parts(
        subject["type"].as_str().unwrap(),
        subject.get("revision").and_then(Value::as_u64),
    );
    let valid = case["expected"]["outcome"] == "ok";
    assert_eq!(parsed.is_ok(), valid, "{}", case["id"]);
    if let Ok(subject) = parsed {
        let unit: IntentUnitId = fixture["shared_values"]
            [case["input"]["unit_id_ref"].as_str().unwrap()]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
        let _ = RecordedAssociation::new(
            unit,
            subject,
            reference_from_value(&fixture["shared_values"]["reference"]),
        );
    }
}

#[test]
fn workflow_collection_boundaries_are_typed() {
    let phase = |index| PhaseId::new(format!("p{index}")).unwrap();
    let phases: Vec<_> = (0..=MAX_WORKFLOW_PHASES).map(phase).collect();
    assert!(matches!(
        Workflow::new_bounded(
            WorkflowId::new("flow").unwrap(),
            phases.clone(),
            phases[0].clone(),
            vec![],
            vec![]
        ),
        Err(BoundedWorkflowError::TooManyPhases {
            length: 33,
            maximum: 32
        })
    ));

    let two = vec![phase(0), phase(1)];
    let edges = vec![WorkflowEdge::new(two[0].clone(), two[1].clone()); MAX_WORKFLOW_EDGES + 1];
    assert!(matches!(
        Workflow::new_bounded(
            WorkflowId::new("flow").unwrap(),
            two.clone(),
            two[0].clone(),
            edges,
            vec![]
        ),
        Err(BoundedWorkflowError::TooManyEdges {
            length: 129,
            maximum: 128
        })
    ));

    let completion = vec![two[0].clone(); MAX_COMPLETION_PHASES + 1];
    assert!(matches!(
        Workflow::new_bounded(
            WorkflowId::new("flow").unwrap(),
            two.clone(),
            two[0].clone(),
            vec![],
            completion,
        ),
        Err(BoundedWorkflowError::TooManyCompletionPhases {
            length: 33,
            maximum: 32
        })
    ));
}

#[test]
fn valid_values_round_trip_without_normalization() {
    let namespace = ReferenceNamespace::from_str("git.commit.sha256").unwrap();
    let text = ReferenceText::new("e\u{301}").unwrap();
    assert_eq!(namespace.as_str(), "git.commit.sha256");
    assert_eq!(text.as_str().as_bytes(), b"e\xcc\x81");
}
