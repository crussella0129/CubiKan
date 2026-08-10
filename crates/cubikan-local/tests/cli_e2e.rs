use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

use cubikan_backend::{CreateIntentUnit, SqliteBackend};
use cubikan_core::{IntentSpecies, PhaseId, Workflow, WorkflowEdge, WorkflowId};
use serde_json::{Map, Value, json};

const REQUEST_FIXTURE: &[u8] = include_bytes!("fixtures/durable-lifecycle-v1.json");
const ID_01: &str = "00000000-0000-0000-0000-000000000001";
const ID_02: &str = "00000000-0000-0000-0000-000000000002";

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory {
    root: PathBuf,
}

impl TestDirectory {
    fn new(label: &str) -> Self {
        for _ in 0..100 {
            let ordinal = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "cubikan-local-e2e-{label}-{}-{ordinal}",
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

fn requests() -> Value {
    serde_json::from_slice(REQUEST_FIXTURE).expect("named request fixture should be valid JSON")
}

fn named_request(requests: &Value, name: &str) -> Vec<u8> {
    serde_json::to_vec(
        requests
            .get(name)
            .unwrap_or_else(|| panic!("fixture request `{name}` should exist")),
    )
    .expect("named request should serialize")
}

fn invoke(database: &Path, request: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cubikan-local"))
        .arg("--database")
        .arg(database)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("cubikan-local process should start");
    let mut stdin = child.stdin.take().expect("child stdin should be piped");
    stdin
        .write_all(request)
        .expect("named request should be written to child stdin");
    drop(stdin);
    child
        .wait_with_output()
        .expect("cubikan-local process should finish")
}

fn invoke_named(database: &Path, requests: &Value, name: &str) -> (Output, Value) {
    let output = invoke(database, &named_request(requests, name));
    let response = response(&output);
    (output, response)
}

fn response(output: &Output) -> Value {
    assert!(output.stderr.is_empty(), "modeled stderr must remain empty");
    assert_eq!(output.stdout.last(), Some(&b'\n'));
    assert_eq!(
        output.stdout.iter().filter(|byte| **byte == b'\n').count(),
        1,
        "stdout must contain exactly one compact JSON line"
    );
    serde_json::from_slice(&output.stdout).expect("stdout should contain one JSON value")
}

fn workflow_json() -> Value {
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

fn unit_json(id: &str, phase: &str, status: &str, revision: &str, history: Value) -> Value {
    json!({
        "id": id,
        "species": "feature",
        "workflow": workflow_json(),
        "phase": phase,
        "status": status,
        "revision": revision,
        "history": history
    })
}

fn unit_success(unit: Value) -> Value {
    json!({
        "protocol_version": 1,
        "outcome": "success",
        "result": {"type": "unit", "intent_unit": unit}
    })
}

fn mutation_success(revision: &str, unit: Value) -> Value {
    json!({
        "protocol_version": 1,
        "outcome": "success",
        "result": {
            "type": "mutation",
            "committed_revision": revision,
            "intent_unit": unit
        }
    })
}

fn transition_one() -> Value {
    json!([{"type":"transition","sequence":1,"from":"queued","to":"doing"}])
}

fn transition_two() -> Value {
    json!([
        {"type":"transition","sequence":1,"from":"queued","to":"doing"},
        {"type":"transition","sequence":2,"from":"doing","to":"done"}
    ])
}

fn completed_history() -> Value {
    json!([
        {"type":"transition","sequence":1,"from":"queued","to":"doing"},
        {"type":"transition","sequence":2,"from":"doing","to":"done"},
        {"type":"completion","sequence":3,"phase":"done"}
    ])
}

#[test]
fn test_cubikan_local_persists_paginates_and_completes_across_processes() {
    let directory = TestDirectory::new("journey");
    let database = directory.path("cubikan.sqlite3");
    let requests = requests();

    let (output, created_02) = invoke_named(&database, &requests, "create_02");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        created_02,
        unit_success(unit_json(ID_02, "queued", "active", "0", json!([])))
    );
    let (output, created_01) = invoke_named(&database, &requests, "create_01");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        created_01,
        unit_success(unit_json(ID_01, "queued", "active", "0", json!([])))
    );

    for (name, id) in [("get_01", ID_01), ("get_02", ID_02)] {
        let (output, fetched) = invoke_named(&database, &requests, name);
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(
            fetched,
            unit_success(unit_json(id, "queued", "active", "0", json!([])))
        );
    }

    let expected_summary = |id: &str| {
        json!({
            "id": id,
            "species": "feature",
            "workflow_id": "delivery",
            "phase": "queued",
            "status": "active",
            "revision": "0"
        })
    };
    let (output, first_page) = invoke_named(&database, &requests, "list_all_filters_page_1");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        first_page,
        json!({
            "protocol_version": 1,
            "outcome": "success",
            "result": {
                "type": "page",
                "items": [expected_summary(ID_01)],
                "next_cursor": ID_01
            }
        })
    );
    let (output, second_page) = invoke_named(&database, &requests, "list_all_filters_page_2");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        second_page,
        json!({
            "protocol_version": 1,
            "outcome": "success",
            "result": {
                "type": "page",
                "items": [expected_summary(ID_02)],
                "next_cursor": null
            }
        })
    );

    let (output, transitioned) = invoke_named(&database, &requests, "transition_01_doing_r0");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        transitioned,
        mutation_success(
            "1",
            unit_json(ID_01, "doing", "active", "1", transition_one())
        )
    );

    // Completion from `doing` is ineligible, so this stale request proves the
    // locked stale-before-domain precedence across an actual process boundary.
    let (output, conflict) = invoke_named(&database, &requests, "stale_complete_01_r0");
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(conflict["protocol_version"], 1);
    assert_eq!(conflict["outcome"], "failure");
    assert_eq!(conflict["error"]["code"], "revision_conflict");
    assert_eq!(conflict["error"]["expected_revision"], "0");
    assert_eq!(conflict["error"]["actual_revision"], "1");
    assert!(conflict["error"]["message"].is_string());
    assert!(conflict["error"].get("field").is_none());
    assert_eq!(
        conflict["error"].as_object().map(Map::len),
        Some(4),
        "conflict optional members must be exact"
    );

    let (output, refreshed) = invoke_named(&database, &requests, "get_01");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        refreshed,
        unit_success(unit_json(ID_01, "doing", "active", "1", transition_one()))
    );

    let (output, transitioned) = invoke_named(&database, &requests, "transition_01_done_r1");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        transitioned,
        mutation_success(
            "2",
            unit_json(ID_01, "done", "active", "2", transition_two())
        )
    );
    let (output, completed) = invoke_named(&database, &requests, "complete_01_r2");
    assert_eq!(output.status.code(), Some(0));
    let final_unit = unit_json(ID_01, "done", "completed", "3", completed_history());
    assert_eq!(completed, mutation_success("3", final_unit.clone()));

    let (output, final_get) = invoke_named(&database, &requests, "get_01");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(final_get, unit_success(final_unit));
}

#[test]
fn test_cubikan_local_rejects_unknown_and_malformed_schema_without_mutation() {
    let directory = TestDirectory::new("schema-rejection");
    let requests = requests();

    let version_two = directory.path("version-two.sqlite3");
    seed_owned_database(&version_two);
    let mut version_two_bytes = fs::read(&version_two).expect("seed database should be readable");
    assert!(version_two_bytes.len() >= 64);
    assert_eq!(&version_two_bytes[60..64], &1_u32.to_be_bytes());
    version_two_bytes[60..64].copy_from_slice(&2_u32.to_be_bytes());
    fs::write(&version_two, &version_two_bytes).expect("version fixture should be written");
    assert_storage_rejection(
        &version_two,
        &requests,
        "unsupported_schema_version",
        &version_two_bytes,
    );

    let malformed = directory.path("malformed-v1.sqlite3");
    seed_owned_database(&malformed);
    let mut malformed_bytes = fs::read(&malformed).expect("seed database should be readable");
    replace_exactly_once(
        &mut malformed_bytes,
        b"intent_units(status,id)",
        b"intent_units(statuX,id)",
    );
    fs::write(&malformed, &malformed_bytes).expect("malformed fixture should be written");
    assert_storage_rejection(&malformed, &requests, "corrupt_schema", &malformed_bytes);
}

fn seed_owned_database(path: &Path) {
    let mut backend = SqliteBackend::open(path).expect("fixture database should initialize");
    backend
        .create(CreateIntentUnit::new(
            Some(ID_01.parse().expect("fixture ID should parse")),
            IntentSpecies::new("feature").expect("fixture species should be valid"),
            core_workflow(),
        ))
        .expect("fixture Intent Unit should create");
}

fn core_workflow() -> Workflow {
    let queued = PhaseId::new("queued").expect("fixture phase should be valid");
    let doing = PhaseId::new("doing").expect("fixture phase should be valid");
    let done = PhaseId::new("done").expect("fixture phase should be valid");
    Workflow::new(
        WorkflowId::new("delivery").expect("fixture workflow ID should be valid"),
        vec![queued.clone(), doing.clone(), done.clone()],
        queued.clone(),
        vec![
            WorkflowEdge::new(queued, doing.clone()),
            WorkflowEdge::new(doing, done.clone()),
        ],
        vec![done],
    )
    .expect("fixture workflow should be valid")
}

fn assert_storage_rejection(
    path: &Path,
    requests: &Value,
    expected_code: &str,
    expected_bytes: &[u8],
) {
    let output = invoke(path, &named_request(requests, "get_01"));
    let response = response(&output);
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(response["protocol_version"], 1);
    assert_eq!(response["outcome"], "failure");
    assert_eq!(response["error"]["code"], expected_code);
    assert!(response["error"]["message"].is_string());
    assert_eq!(
        response["error"].as_object().map(Map::len),
        Some(2),
        "storage failures must not expose validation or conflict members"
    );
    assert_eq!(
        fs::read(path).expect("rejected database should remain readable"),
        expected_bytes,
        "rejected storage bytes must remain unchanged"
    );
}

fn replace_exactly_once(bytes: &mut [u8], original: &[u8], replacement: &[u8]) {
    assert_eq!(original.len(), replacement.len());
    let positions = bytes
        .windows(original.len())
        .enumerate()
        .filter_map(|(index, candidate)| (candidate == original).then_some(index))
        .collect::<Vec<_>>();
    assert_eq!(positions.len(), 1, "schema text should occur exactly once");
    let start = positions[0];
    bytes[start..start + replacement.len()].copy_from_slice(replacement);
}
