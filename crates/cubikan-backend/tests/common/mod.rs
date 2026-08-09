// Each integration test is compiled as a separate crate and intentionally uses
// only the fixture subset relevant to that boundary.
#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use cubikan_core::{IntentUnitId, PhaseId, Workflow, WorkflowEdge, WorkflowId};
use rusqlite::Connection;

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
