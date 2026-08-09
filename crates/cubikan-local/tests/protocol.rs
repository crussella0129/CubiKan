use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use cubikan_backend::SqliteBackend;
use cubikan_local::{ResponseClass, execute_request};
use serde_json::{Value, json};

const ID: &str = "70000000-0000-0000-0000-000000000007";
const MISSING_ID: &str = "70000000-0000-0000-0000-000000000099";

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory {
    root: PathBuf,
}

impl TestDirectory {
    fn new(label: &str) -> Self {
        for _ in 0..100 {
            let ordinal = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "cubikan-local-{label}-{}-{ordinal}",
                std::process::id()
            ));
            match fs::create_dir(&root) {
                Ok(()) => return Self { root },
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => panic!("test directory should be created: {error}"),
            }
        }
        panic!("could not allocate a unique test directory");
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn workflow() -> Value {
    json!({
        "id": "delivery",
        "phases": ["queued", "doing", "done"],
        "initial_phase": "queued",
        "edges": [
            {"from": "queued", "to": "doing"},
            {"from": "doing", "to": "done"}
        ],
        "completion_phases": ["done"]
    })
}

fn create_request(id: &str) -> Value {
    json!({
        "protocol_version": 1,
        "operation": {
            "type": "create",
            "intent_unit": {"id": id, "species": "feature"},
            "workflow": workflow()
        }
    })
}

fn get_request(id: &str) -> Value {
    json!({"protocol_version": 1, "operation": {"type": "get", "id": id}})
}

fn list_request() -> Value {
    json!({
        "protocol_version": 1,
        "operation": {"type": "list", "filters": {}, "limit": 100}
    })
}

fn transition_request(id: &str, target: &str, revision: Value) -> Value {
    json!({
        "protocol_version": 1,
        "operation": {
            "type": "transition",
            "id": id,
            "target": target,
            "expected_revision": revision
        }
    })
}

fn complete_request(id: &str, revision: Value) -> Value {
    json!({
        "protocol_version": 1,
        "operation": {"type": "complete", "id": id, "expected_revision": revision}
    })
}

fn execute_value(path: &Path, request: &Value) -> (ResponseClass, Value) {
    let bytes = serde_json::to_vec(request).expect("fixture request should serialize");
    execute_raw(path, &bytes)
}

fn execute_raw(path: &Path, request: &[u8]) -> (ResponseClass, Value) {
    let response = execute_request(path, request);
    let value = serde_json::from_slice(response.body()).expect("response should be valid JSON");
    (response.class(), value)
}

fn assert_failure(
    path: &Path,
    request: &[u8],
    expected_code: &str,
    expected_field: Option<&str>,
) -> Value {
    let (class, response) = execute_raw(path, request);
    assert_eq!(class, ResponseClass::RequestRejected);
    assert_eq!(response["protocol_version"], 1);
    assert_eq!(response["outcome"], "failure");
    assert_eq!(response["error"]["code"], expected_code);
    assert!(response["error"]["message"].is_string());
    assert_eq!(
        response["error"].get("field").and_then(Value::as_str),
        expected_field
    );
    assert!(response["error"].get("expected_revision").is_none());
    assert!(response["error"].get("actual_revision").is_none());
    response
}

#[test]
fn test_protocol_v1_decodes_all_locked_operations_strictly() {
    let directory = TestDirectory::new("strict");
    let database = directory.path("valid.sqlite3");

    let valid = [
        create_request(ID),
        get_request(ID),
        list_request(),
        transition_request(ID, "doing", json!("0")),
        complete_request(ID, json!("1")),
    ];
    let expected_classes = [
        ResponseClass::Success,
        ResponseClass::Success,
        ResponseClass::Success,
        ResponseClass::Success,
        ResponseClass::CommandRejected,
    ];
    for (request, expected_class) in valid.iter().zip(expected_classes) {
        assert_eq!(execute_value(&database, request).0, expected_class);
    }

    let sentinel = directory.path("sentinel.sqlite3");
    let structural_cases = vec![
        ("malformed", b"{".to_vec(), "malformed_json"),
        ("empty", Vec::new(), "malformed_json"),
        (
            "trailing",
            br#"{"protocol_version":1,"operation":{"type":"get","id":"70000000-0000-0000-0000-000000000007"}} null"#.to_vec(),
            "malformed_json",
        ),
        ("top-null", b"null".to_vec(), "invalid_request"),
        (
            "missing-version",
            serde_json::to_vec(&json!({"operation": {"type": "get", "id": ID}})).unwrap(),
            "invalid_request",
        ),
        (
            "wrong-version-type",
            serde_json::to_vec(&json!({"protocol_version": "1", "operation": {"type": "get", "id": ID}})).unwrap(),
            "invalid_request",
        ),
        (
            "missing-operation",
            serde_json::to_vec(&json!({"protocol_version": 1})).unwrap(),
            "invalid_request",
        ),
        (
            "wrong-operation-type",
            serde_json::to_vec(&json!({"protocol_version": 1, "operation": []})).unwrap(),
            "invalid_request",
        ),
        (
            "unknown-top-field",
            serde_json::to_vec(&json!({"protocol_version": 1, "operation": {"type": "get", "id": ID}, "extra": true})).unwrap(),
            "invalid_request",
        ),
        (
            "unknown-operation",
            serde_json::to_vec(&json!({"protocol_version": 1, "operation": {"type": "remove", "id": ID}})).unwrap(),
            "invalid_request",
        ),
        (
            "missing-create-member",
            serde_json::to_vec(&json!({"protocol_version": 1, "operation": {"type": "create", "workflow": workflow()}})).unwrap(),
            "invalid_request",
        ),
        (
            "unknown-intent-unit-field",
            serde_json::to_vec(&json!({"protocol_version": 1, "operation": {"type": "create", "intent_unit": {"species": "feature", "extra": 1}, "workflow": workflow()}})).unwrap(),
            "invalid_request",
        ),
        (
            "null-optional-create-id",
            serde_json::to_vec(&json!({"protocol_version": 1, "operation": {"type": "create", "intent_unit": {"id": null, "species": "feature"}, "workflow": workflow()}})).unwrap(),
            "invalid_request",
        ),
        (
            "unknown-workflow-field",
            serde_json::to_vec(&json!({"protocol_version": 1, "operation": {"type": "create", "intent_unit": {"species": "feature"}, "workflow": {"id": "flow", "phases": ["a"], "initial_phase": "a", "edges": [], "completion_phases": [], "extra": 1}}})).unwrap(),
            "invalid_request",
        ),
        (
            "unknown-edge-field",
            serde_json::to_vec(&json!({"protocol_version": 1, "operation": {"type": "create", "intent_unit": {"species": "feature"}, "workflow": {"id": "flow", "phases": ["a"], "initial_phase": "a", "edges": [{"from": "a", "to": "a", "extra": 1}], "completion_phases": []}}})).unwrap(),
            "invalid_request",
        ),
        (
            "missing-get-id",
            serde_json::to_vec(&json!({"protocol_version": 1, "operation": {"type": "get"}})).unwrap(),
            "invalid_request",
        ),
        (
            "null-get-id",
            serde_json::to_vec(&json!({"protocol_version": 1, "operation": {"type": "get", "id": null}})).unwrap(),
            "invalid_request",
        ),
        (
            "missing-list-filters",
            serde_json::to_vec(&json!({"protocol_version": 1, "operation": {"type": "list", "limit": 1}})).unwrap(),
            "invalid_request",
        ),
        (
            "null-list-after",
            serde_json::to_vec(&json!({"protocol_version": 1, "operation": {"type": "list", "filters": {}, "limit": 1, "after": null}})).unwrap(),
            "invalid_request",
        ),
        (
            "null-list-filter",
            serde_json::to_vec(&json!({"protocol_version": 1, "operation": {"type": "list", "filters": {"species": null}, "limit": 1}})).unwrap(),
            "invalid_request",
        ),
        (
            "unknown-filter-field",
            serde_json::to_vec(&json!({"protocol_version": 1, "operation": {"type": "list", "filters": {"extra": true}, "limit": 1}})).unwrap(),
            "invalid_request",
        ),
        (
            "numeric-transition-revision",
            serde_json::to_vec(&transition_request(ID, "doing", json!(0))).unwrap(),
            "invalid_request",
        ),
        (
            "null-complete-revision",
            serde_json::to_vec(&complete_request(ID, Value::Null)).unwrap(),
            "invalid_request",
        ),
        (
            "unknown-complete-field",
            serde_json::to_vec(&json!({"protocol_version": 1, "operation": {"type": "complete", "id": ID, "expected_revision": "0", "extra": 1}})).unwrap(),
            "invalid_request",
        ),
    ];
    for (name, request, expected_code) in structural_cases {
        assert_failure(&sentinel, &request, expected_code, None);
        assert!(!sentinel.exists(), "{name} must be rejected before open");
    }

    let unsupported =
        serde_json::to_vec(&json!({"protocol_version": 2, "operation": {"type": "get", "id": ID}}))
            .unwrap();
    assert_failure(
        &sentinel,
        &unsupported,
        "unsupported_protocol_version",
        None,
    );
    assert!(!sentinel.exists());
}

#[test]
fn test_protocol_v1_rejects_semantically_invalid_values_before_storage() {
    struct Case {
        name: &'static str,
        request: Value,
        code: &'static str,
        field: Option<&'static str>,
    }

    let directory = TestDirectory::new("semantic");
    let sentinel = directory.path("sentinel.sqlite3");
    let uppercase_id = "AAAAAAAA-AAAA-AAAA-AAAA-AAAAAAAAAAAA";
    let compact_id = "70000000000000000000000000000007";
    let invalid_topologies = [
        (
            "empty-phases",
            json!({"id":"flow","phases":[],"initial_phase":"a","edges":[],"completion_phases":[]}),
        ),
        (
            "duplicate-phase",
            json!({"id":"flow","phases":["a","a"],"initial_phase":"a","edges":[],"completion_phases":[]}),
        ),
        (
            "unknown-initial",
            json!({"id":"flow","phases":["a"],"initial_phase":"b","edges":[],"completion_phases":[]}),
        ),
        (
            "unknown-edge-source",
            json!({"id":"flow","phases":["a"],"initial_phase":"a","edges":[{"from":"b","to":"a"}],"completion_phases":[]}),
        ),
        (
            "unknown-edge-target",
            json!({"id":"flow","phases":["a"],"initial_phase":"a","edges":[{"from":"a","to":"b"}],"completion_phases":[]}),
        ),
        (
            "duplicate-edge",
            json!({"id":"flow","phases":["a"],"initial_phase":"a","edges":[{"from":"a","to":"a"},{"from":"a","to":"a"}],"completion_phases":[]}),
        ),
        (
            "unknown-completion",
            json!({"id":"flow","phases":["a"],"initial_phase":"a","edges":[],"completion_phases":["b"]}),
        ),
        (
            "duplicate-completion",
            json!({"id":"flow","phases":["a"],"initial_phase":"a","edges":[],"completion_phases":["a","a"]}),
        ),
    ];

    let mut cases = vec![
        Case {
            name: "malformed-id",
            request: get_request("not-a-uuid"),
            code: "invalid_intent_unit_id",
            field: Some("operation.id"),
        },
        Case {
            name: "uppercase-id",
            request: get_request(uppercase_id),
            code: "invalid_intent_unit_id",
            field: Some("operation.id"),
        },
        Case {
            name: "compact-create-id",
            request: create_request(compact_id),
            code: "invalid_intent_unit_id",
            field: Some("operation.intent_unit.id"),
        },
        Case {
            name: "blank-species",
            request: json!({"protocol_version":1,"operation":{"type":"create","intent_unit":{"species":" \t"},"workflow":workflow()}}),
            code: "invalid_species",
            field: Some("operation.intent_unit.species"),
        },
        Case {
            name: "blank-workflow-id",
            request: json!({"protocol_version":1,"operation":{"type":"create","intent_unit":{"species":"feature"},"workflow":{"id":" ","phases":["a"],"initial_phase":"a","edges":[],"completion_phases":[]}}}),
            code: "invalid_workflow_id",
            field: Some("operation.workflow.id"),
        },
        Case {
            name: "blank-phase",
            request: json!({"protocol_version":1,"operation":{"type":"create","intent_unit":{"species":"feature"},"workflow":{"id":"flow","phases":["a"," "],"initial_phase":"a","edges":[],"completion_phases":[]}}}),
            code: "invalid_phase_id",
            field: Some("operation.workflow.phases[1]"),
        },
        Case {
            name: "blank-initial-phase",
            request: json!({"protocol_version":1,"operation":{"type":"create","intent_unit":{"species":"feature"},"workflow":{"id":"flow","phases":["a"],"initial_phase":" ","edges":[],"completion_phases":[]}}}),
            code: "invalid_phase_id",
            field: Some("operation.workflow.initial_phase"),
        },
        Case {
            name: "blank-edge-source",
            request: json!({"protocol_version":1,"operation":{"type":"create","intent_unit":{"species":"feature"},"workflow":{"id":"flow","phases":["a"],"initial_phase":"a","edges":[{"from":" ","to":"a"}],"completion_phases":[]}}}),
            code: "invalid_phase_id",
            field: Some("operation.workflow.edges[0].from"),
        },
        Case {
            name: "blank-edge-target",
            request: json!({"protocol_version":1,"operation":{"type":"create","intent_unit":{"species":"feature"},"workflow":{"id":"flow","phases":["a"],"initial_phase":"a","edges":[{"from":"a","to":" "}],"completion_phases":[]}}}),
            code: "invalid_phase_id",
            field: Some("operation.workflow.edges[0].to"),
        },
        Case {
            name: "blank-completion-phase",
            request: json!({"protocol_version":1,"operation":{"type":"create","intent_unit":{"species":"feature"},"workflow":{"id":"flow","phases":["a"],"initial_phase":"a","edges":[],"completion_phases":[" "]}}}),
            code: "invalid_phase_id",
            field: Some("operation.workflow.completion_phases[0]"),
        },
        Case {
            name: "blank-transition-target",
            request: transition_request(ID, " ", json!("0")),
            code: "invalid_phase_id",
            field: Some("operation.target"),
        },
        Case {
            name: "zero-limit",
            request: json!({"protocol_version":1,"operation":{"type":"list","filters":{},"limit":0}}),
            code: "invalid_query",
            field: Some("operation.limit"),
        },
        Case {
            name: "negative-limit",
            request: json!({"protocol_version":1,"operation":{"type":"list","filters":{},"limit":-1}}),
            code: "invalid_query",
            field: Some("operation.limit"),
        },
        Case {
            name: "large-limit",
            request: json!({"protocol_version":1,"operation":{"type":"list","filters":{},"limit":101}}),
            code: "invalid_query",
            field: Some("operation.limit"),
        },
        Case {
            name: "float-limit",
            request: json!({"protocol_version":1,"operation":{"type":"list","filters":{},"limit":1.5}}),
            code: "invalid_request",
            field: None,
        },
        Case {
            name: "unknown-status",
            request: json!({"protocol_version":1,"operation":{"type":"list","filters":{"status":"Active"},"limit":1}}),
            code: "invalid_query",
            field: Some("operation.filters.status"),
        },
        Case {
            name: "malformed-cursor",
            request: json!({"protocol_version":1,"operation":{"type":"list","filters":{},"limit":1,"after":"nope"}}),
            code: "invalid_query",
            field: Some("operation.after"),
        },
        Case {
            name: "noncanonical-cursor",
            request: json!({"protocol_version":1,"operation":{"type":"list","filters":{},"limit":1,"after":uppercase_id}}),
            code: "invalid_query",
            field: Some("operation.after"),
        },
        Case {
            name: "blank-filter-workflow",
            request: json!({"protocol_version":1,"operation":{"type":"list","filters":{"workflow_id":" "},"limit":1}}),
            code: "invalid_workflow_id",
            field: Some("operation.filters.workflow_id"),
        },
        Case {
            name: "blank-filter-species",
            request: json!({"protocol_version":1,"operation":{"type":"list","filters":{"species":" "},"limit":1}}),
            code: "invalid_species",
            field: Some("operation.filters.species"),
        },
        Case {
            name: "blank-filter-phase",
            request: json!({"protocol_version":1,"operation":{"type":"list","filters":{"phase":" "},"limit":1}}),
            code: "invalid_phase_id",
            field: Some("operation.filters.phase"),
        },
    ];
    for (name, invalid_workflow) in invalid_topologies {
        cases.push(Case {
            name,
            request: json!({"protocol_version":1,"operation":{"type":"create","intent_unit":{"species":"feature"},"workflow":invalid_workflow}}),
            code: "invalid_workflow",
            field: Some("operation.workflow"),
        });
    }
    for invalid_revision in [
        "",
        "+1",
        "-1",
        " 1",
        "1 ",
        "01",
        "1.0",
        "18446744073709551616",
        "１２",
    ] {
        cases.push(Case {
            name: "invalid-revision-text",
            request: complete_request(ID, json!(invalid_revision)),
            code: "invalid_revision",
            field: Some("operation.expected_revision"),
        });
    }
    for wrong_revision in [json!(0), Value::Null, json!(true)] {
        cases.push(Case {
            name: "wrong-revision-type",
            request: complete_request(ID, wrong_revision),
            code: "invalid_request",
            field: None,
        });
    }

    for case in cases {
        let request = serde_json::to_vec(&case.request).unwrap();
        assert_failure(&sentinel, &request, case.code, case.field);
        assert!(
            !sentinel.exists(),
            "{} must be rejected before storage open",
            case.name
        );
    }
}

#[test]
fn test_protocol_v1_uses_decimal_strings_for_every_revision() {
    let directory = TestDirectory::new("revisions");
    let database = directory.path("cubikan.sqlite3");

    let (_, created) = execute_value(&database, &create_request(ID));
    assert_eq!(created["result"]["intent_unit"]["revision"], "0");
    assert!(created["result"]["intent_unit"]["revision"].is_string());

    let (_, page) = execute_value(&database, &list_request());
    assert_eq!(page["result"]["items"][0]["revision"], "0");
    assert!(page["result"]["items"][0]["revision"].is_string());

    let (class, conflict) = execute_value(
        &database,
        &transition_request(ID, "doing", json!(u64::MAX.to_string())),
    );
    assert_eq!(class, ResponseClass::CommandRejected);
    assert_eq!(conflict["error"]["code"], "revision_conflict");
    assert_eq!(conflict["error"]["expected_revision"], u64::MAX.to_string());
    assert_eq!(conflict["error"]["actual_revision"], "0");
    assert!(conflict["error"]["expected_revision"].is_string());
    assert!(conflict["error"]["actual_revision"].is_string());

    let (_, mutation) = execute_value(&database, &transition_request(ID, "doing", json!("0")));
    assert_eq!(mutation["result"]["committed_revision"], "1");
    assert_eq!(mutation["result"]["intent_unit"]["revision"], "1");
    assert!(mutation["result"]["committed_revision"].is_string());
    assert!(mutation["result"]["intent_unit"]["revision"].is_string());

    let (_, unit) = execute_value(&database, &get_request(ID));
    assert_eq!(unit["result"]["intent_unit"]["revision"], "1");
    assert!(unit["result"]["intent_unit"]["revision"].is_string());
}

#[test]
fn test_protocol_v1_serializes_exact_unit_page_and_mutation_results() {
    let directory = TestDirectory::new("results");
    let database = directory.path("cubikan.sqlite3");

    let (class, created) = execute_value(&database, &create_request(ID));
    assert_eq!(class, ResponseClass::Success);
    assert_eq!(
        created,
        json!({
            "protocol_version": 1,
            "outcome": "success",
            "result": {
                "type": "unit",
                "intent_unit": {
                    "id": ID,
                    "species": "feature",
                    "workflow": workflow(),
                    "phase": "queued",
                    "status": "active",
                    "revision": "0",
                    "history": []
                }
            }
        })
    );

    let (class, page) = execute_value(&database, &list_request());
    assert_eq!(class, ResponseClass::Success);
    assert_eq!(
        page,
        json!({
            "protocol_version": 1,
            "outcome": "success",
            "result": {
                "type": "page",
                "items": [{
                    "id": ID,
                    "species": "feature",
                    "workflow_id": "delivery",
                    "phase": "queued",
                    "status": "active",
                    "revision": "0"
                }],
                "next_cursor": null
            }
        })
    );

    let (class, transitioned) =
        execute_value(&database, &transition_request(ID, "doing", json!("0")));
    assert_eq!(class, ResponseClass::Success);
    assert_eq!(
        transitioned,
        json!({
            "protocol_version": 1,
            "outcome": "success",
            "result": {
                "type": "mutation",
                "committed_revision": "1",
                "intent_unit": {
                    "id": ID,
                    "species": "feature",
                    "workflow": workflow(),
                    "phase": "doing",
                    "status": "active",
                    "revision": "1",
                    "history": [{
                        "type": "transition",
                        "sequence": 1,
                        "from": "queued",
                        "to": "doing"
                    }]
                }
            }
        })
    );
}

#[test]
fn test_protocol_v1_maps_exact_error_code_taxonomy() {
    let request_codes = [
        "malformed_json",
        "request_too_large",
        "invalid_request",
        "unsupported_protocol_version",
        "invalid_intent_unit_id",
        "invalid_species",
        "invalid_workflow_id",
        "invalid_phase_id",
        "invalid_workflow",
        "invalid_query",
        "invalid_revision",
    ];
    let command_codes = [
        "duplicate_intent_unit",
        "intent_unit_not_found",
        "revision_conflict",
        "transition_already_completed",
        "transition_unknown_target",
        "transition_not_allowed",
        "completion_already_completed",
        "completion_phase_not_eligible",
    ];
    let storage_codes = [
        "storage_busy",
        "unowned_database",
        "unsupported_schema_version",
        "corrupt_schema",
        "unsupported_envelope_version",
        "corrupt_envelope",
        "projection_mismatch",
        "concurrent_storage_change",
        "storage_error",
    ];
    let all_codes = request_codes
        .into_iter()
        .chain(command_codes)
        .chain(storage_codes)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(all_codes.len(), 28, "the locked taxonomy must be unique");

    // The protocol module's same-named unit test constructs and serializes all
    // 28 private codes. This public-seam companion verifies each modeled class
    // is observable through execution without expanding the public API.
    let directory = TestDirectory::new("taxonomy");
    let database = directory.path("cubikan.sqlite3");
    let (class, malformed) = execute_raw(&database, b"{");
    assert_eq!(class, ResponseClass::RequestRejected);
    assert!(all_codes.contains(malformed["error"]["code"].as_str().unwrap()));

    assert_eq!(
        execute_value(&database, &create_request(ID)).0,
        ResponseClass::Success
    );
    let (class, duplicate) = execute_value(&database, &create_request(ID));
    assert_eq!(class, ResponseClass::CommandRejected);
    assert!(all_codes.contains(duplicate["error"]["code"].as_str().unwrap()));

    let unavailable = directory.path("missing-parent").join("cubikan.sqlite3");
    let (class, storage) = execute_value(&unavailable, &get_request(ID));
    assert_eq!(class, ResponseClass::StorageRejected);
    assert!(all_codes.contains(storage["error"]["code"].as_str().unwrap()));

    for response in [&malformed, &duplicate, &storage] {
        assert_eq!(response["protocol_version"], 1);
        assert_eq!(response["outcome"], "failure");
        assert!(response["error"]["message"].is_string());
        assert!(response["error"].get("field").is_none());
        assert!(response["error"].get("expected_revision").is_none());
        assert!(response["error"].get("actual_revision").is_none());
    }
}

#[test]
fn test_executor_preserves_backend_atomicity_on_modeled_failure() {
    let directory = TestDirectory::new("atomicity");
    let database = directory.path("cubikan.sqlite3");
    assert_eq!(
        execute_value(&database, &create_request(ID)).0,
        ResponseClass::Success
    );
    let id = ID.parse().expect("fixture ID should parse");

    let state = || {
        SqliteBackend::open(&database)
            .expect("database should reopen")
            .get(id)
            .expect("fixture unit should remain readable")
    };
    let initial = state();

    let mut duplicate = create_request(ID);
    duplicate["operation"]["intent_unit"]["species"] = json!("different");
    let (class, response) = execute_value(&database, &duplicate);
    assert_eq!(class, ResponseClass::CommandRejected);
    assert_eq!(response["outcome"], "failure");
    assert_eq!(response["error"]["code"], "duplicate_intent_unit");
    assert_eq!(state(), initial);

    let (class, response) = execute_value(&database, &get_request(MISSING_ID));
    assert_eq!(class, ResponseClass::CommandRejected);
    assert_eq!(response["error"]["code"], "intent_unit_not_found");
    assert_eq!(state(), initial);

    let (class, response) = execute_value(&database, &complete_request(ID, json!("0")));
    assert_eq!(class, ResponseClass::CommandRejected);
    assert_eq!(response["error"]["code"], "completion_phase_not_eligible");
    assert_eq!(state(), initial);

    let (class, response) = execute_value(&database, &transition_request(ID, "done", json!("0")));
    assert_eq!(class, ResponseClass::CommandRejected);
    assert_eq!(response["error"]["code"], "transition_not_allowed");
    assert_eq!(state(), initial);

    assert_eq!(
        execute_value(&database, &transition_request(ID, "doing", json!("0"))).0,
        ResponseClass::Success
    );
    let revision_one = state();
    assert_ne!(revision_one, initial);

    let (class, response) = execute_value(&database, &complete_request(ID, json!("0")));
    assert_eq!(class, ResponseClass::CommandRejected);
    assert_eq!(response["error"]["code"], "revision_conflict");
    assert_eq!(response["error"]["expected_revision"], "0");
    assert_eq!(response["error"]["actual_revision"], "1");
    assert_eq!(state(), revision_one);

    let unavailable = directory.path("missing-parent").join("cubikan.sqlite3");
    let (class, response) = execute_value(&unavailable, &get_request(ID));
    assert_eq!(class, ResponseClass::StorageRejected);
    assert_eq!(response["outcome"], "failure");
    assert_eq!(response["error"]["code"], "storage_error");
    assert!(!unavailable.exists());
    assert_eq!(state(), revision_one);
}
