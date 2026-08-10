// Each integration test is compiled as a separate crate and intentionally uses
// only the fixture subset relevant to that boundary.
#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use cubikan_core::{
    IntentUnit, IntentUnitId, IntentUnitStatus, LifecycleRecord, PhaseId, Workflow, WorkflowEdge,
    WorkflowId,
};
use rusqlite::{Connection, params};
use serde_json::{Value, json};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

pub struct TestDatabase {
    directory: PathBuf,
    path: PathBuf,
}

impl TestDatabase {
    pub fn new(label: &str) -> Self {
        for _ in 0..100 {
            let ordinal = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let directory = std::env::temp_dir().join(format!(
                "cubikan-backend-{label}-{}-{ordinal}",
                std::process::id()
            ));
            match fs::create_dir(&directory) {
                Ok(()) => {
                    let path = directory.join("cubikan.sqlite3");
                    return Self { directory, path };
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => panic!("test directory should be created: {error}"),
            }
        }
        panic!("could not allocate a unique test directory");
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn connect(&self) -> Connection {
        Connection::open(&self.path).expect("test database should open through raw SQLite")
    }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

pub fn fixed_id(value: &str) -> IntentUnitId {
    value
        .parse()
        .expect("fixture Intent Unit ID should be valid")
}

pub fn phase(value: &str) -> PhaseId {
    PhaseId::new(value).expect("fixture phase should be valid")
}

pub fn linear_workflow(id: &str, initial: &str, terminal: &str) -> Workflow {
    let initial = phase(initial);
    let terminal = phase(terminal);
    Workflow::new(
        WorkflowId::new(id).expect("fixture workflow ID should be valid"),
        vec![terminal.clone(), initial.clone()],
        initial.clone(),
        vec![WorkflowEdge::new(initial, terminal.clone())],
        vec![terminal],
    )
    .expect("fixture workflow should be valid")
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredRowSnapshot {
    pub id: String,
    pub envelope_version: i64,
    pub envelope: String,
    pub workflow_id: String,
    pub species: String,
    pub phase: String,
    pub status: String,
    pub revision: Vec<u8>,
}

pub fn stored_rows(connection: &Connection) -> Vec<StoredRowSnapshot> {
    let mut statement = connection
        .prepare(
            "SELECT id, envelope_version, envelope, workflow_id, species, phase, status, revision
             FROM intent_units ORDER BY id",
        )
        .expect("stored rows should be readable");
    statement
        .query_map([], |row| {
            Ok(StoredRowSnapshot {
                id: row.get(0)?,
                envelope_version: row.get(1)?,
                envelope: row.get(2)?,
                workflow_id: row.get(3)?,
                species: row.get(4)?,
                phase: row.get(5)?,
                status: row.get(6)?,
                revision: row.get(7)?,
            })
        })
        .expect("stored row query should execute")
        .collect::<Result<Vec<_>, _>>()
        .expect("stored rows should decode")
}

/// Replaces an existing row with a complete envelope derived from a core unit.
///
/// This is deliberately an integration-test fixture rather than a backend API:
/// T-805 needs live phase/status membership before public mutations exist.
pub fn replace_stored_unit(connection: &Connection, unit: &IntentUnit) {
    let workflow = unit.workflow();
    let history = unit
        .history()
        .iter()
        .map(|record| match record {
            LifecycleRecord::Transition(record) => json!({
                "type": "transition",
                "sequence": record.sequence(),
                "from": record.from().as_str(),
                "to": record.to().as_str(),
            }),
            LifecycleRecord::Completion(record) => json!({
                "type": "completion",
                "sequence": record.sequence(),
                "phase": record.final_phase().as_str(),
            }),
        })
        .collect::<Vec<Value>>();
    let envelope = json!({
        "representation_version": 1,
        "id": unit.id().to_string(),
        "species": unit.species().as_str(),
        "phase": unit.phase().as_str(),
        "revision": unit.revision().value().to_string(),
        "status": status_text(unit.status()),
        "workflow": {
            "id": workflow.id().as_str(),
            "phases": workflow.phases().iter().map(PhaseId::as_str).collect::<Vec<_>>(),
            "initial_phase": workflow.initial_phase().as_str(),
            "edges": workflow.edges().iter().map(|edge| json!({
                "from": edge.from().as_str(),
                "to": edge.to().as_str(),
            })).collect::<Vec<_>>(),
            "completion_phases": workflow.completion_phases().iter()
                .map(PhaseId::as_str).collect::<Vec<_>>(),
        },
        "history": history,
    })
    .to_string();
    let changed = connection
        .execute(
            "UPDATE intent_units SET
                envelope_version=1,
                envelope=?1,
                workflow_id=?2,
                species=?3,
                phase=?4,
                status=?5,
                revision=?6
             WHERE id=?7",
            params![
                envelope,
                workflow.id().as_str(),
                unit.species().as_str(),
                unit.phase().as_str(),
                status_text(unit.status()),
                unit.revision().value().to_be_bytes(),
                unit.id().to_string(),
            ],
        )
        .expect("core-derived stored fixture should update");
    assert_eq!(changed, 1, "fixture must replace exactly one existing row");
}

const fn status_text(status: IntentUnitStatus) -> &'static str {
    match status {
        IntentUnitStatus::Active => "active",
        IntentUnitStatus::Completed => "completed",
    }
}
