use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

use cubikan_local::MAX_REQUEST_BYTES;
use serde_json::{Value, json};

const V1_FIXTURE: &[u8] = include_bytes!("fixtures/durable-lifecycle-v1.json");
static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        for _ in 0..100 {
            let ordinal = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "cubikan-local-unsupported-e2e-{}-{ordinal}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => panic!("create test directory: {error}"),
            }
        }
        panic!("could not allocate test directory")
    }

    fn database(&self) -> PathBuf {
        self.0.join("requested.sqlite3")
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn create_request() -> Vec<u8> {
    let fixture: Value = serde_json::from_slice(V1_FIXTURE).expect("fixture should be JSON");
    serde_json::to_vec(&fixture["create_01"]).expect("named v1 request should serialize")
}

fn invoke(database: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cubikan-local"))
        .arg("--database")
        .arg(database)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("cubikan-local should start");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(input)
        .expect("fixture should be written");
    child.wait_with_output().expect("process should finish")
}

fn response(output: &Output) -> Value {
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout.last(), Some(&b'\n'));
    assert_eq!(
        output.stdout.iter().filter(|byte| **byte == b'\n').count(),
        1
    );
    serde_json::from_slice(&output.stdout).expect("stdout should contain one JSON value")
}

fn padded_request(length: usize) -> Vec<u8> {
    let mut request = create_request();
    assert_eq!(request.pop(), Some(b'}'));
    assert!(request.len() < length);
    request.resize(length - 1, b' ');
    request.push(b'}');
    request
}

#[test]
fn binary_rejects_v1_before_creating_the_requested_database() {
    let directory = TestDirectory::new();
    let database = directory.database();
    let output = invoke(&database, &create_request());

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        response(&output),
        json!({
            "protocol_version": 1,
            "outcome": "failure",
            "error": {
                "code": "unsupported_protocol_version",
                "message": "protocol version 1 is unsupported"
            }
        })
    );
    assert!(!database.exists());
}

#[test]
fn binary_preserves_the_one_mib_ingestion_bound_without_database_access() {
    let directory = TestDirectory::new();
    let database = directory.database();

    let exact = invoke(&database, &padded_request(MAX_REQUEST_BYTES));
    assert_eq!(exact.status.code(), Some(2));
    assert_eq!(
        response(&exact)["error"]["code"],
        "unsupported_protocol_version"
    );
    assert!(!database.exists());

    let oversized = invoke(&database, &padded_request(MAX_REQUEST_BYTES + 1));
    assert_eq!(oversized.status.code(), Some(2));
    assert_eq!(response(&oversized)["error"]["code"], "request_too_large");
    assert!(!database.exists());
}
