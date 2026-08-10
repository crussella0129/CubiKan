use std::{
    error::Error,
    ffi::OsString,
    fs,
    io::{self, Cursor, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use cubikan_backend::SqliteBackend;
use cubikan_core::{IntentUnitRevision, IntentUnitStatus};
use cubikan_local::{MAX_REQUEST_BYTES, ResponseClass, RunError, run, run_process};
use serde_json::{Value, json};

const ID: &str = "80000000-0000-0000-0000-000000000008";
const MISSING_ID: &str = "80000000-0000-0000-0000-000000000099";

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory {
    root: PathBuf,
}

impl TestDirectory {
    fn new(label: &str) -> Self {
        for _ in 0..100 {
            let ordinal = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "cubikan-local-runner-{label}-{}-{ordinal}",
                std::process::id()
            ));
            match fs::create_dir(&root) {
                Ok(()) => return Self { root },
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
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
        "phases": ["queued", "done"],
        "initial_phase": "queued",
        "edges": [{"from": "queued", "to": "done"}],
        "completion_phases": ["done"]
    })
}

fn create_request() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "protocol_version": 1,
        "operation": {
            "type": "create",
            "intent_unit": {"id": ID, "species": "feature"},
            "workflow": workflow()
        }
    }))
    .unwrap()
}

fn get_request(id: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({"protocol_version":1,"operation":{"type":"get","id":id}})).unwrap()
}

fn list_request() -> Vec<u8> {
    serde_json::to_vec(
        &json!({"protocol_version":1,"operation":{"type":"list","filters":{},"limit":100}}),
    )
    .unwrap()
}

fn transition_request(expected_revision: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "protocol_version": 1,
        "operation": {
            "type": "transition",
            "id": ID,
            "target": "done",
            "expected_revision": expected_revision
        }
    }))
    .unwrap()
}

fn complete_request(expected_revision: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "protocol_version": 1,
        "operation": {
            "type": "complete",
            "id": ID,
            "expected_revision": expected_revision
        }
    }))
    .unwrap()
}

fn args(path: &Path) -> Vec<OsString> {
    vec![
        OsString::from("cubikan-local"),
        OsString::from("--database"),
        path.as_os_str().to_owned(),
    ]
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WriteStage {
    Body,
    Newline,
    Flush,
}

#[derive(Default)]
struct RecordingWriter {
    bytes: Vec<u8>,
    events: Vec<WriteStage>,
    fail_at: Option<WriteStage>,
}

impl RecordingWriter {
    fn failing(stage: WriteStage) -> Self {
        Self {
            fail_at: Some(stage),
            ..Self::default()
        }
    }
}

impl Write for RecordingWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let stage = if buffer == b"\n" {
            WriteStage::Newline
        } else {
            WriteStage::Body
        };
        self.events.push(stage);
        if self.fail_at == Some(stage) {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                match stage {
                    WriteStage::Body => "fixture body failure",
                    WriteStage::Newline => "fixture newline failure",
                    WriteStage::Flush => unreachable!(),
                },
            ));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.events.push(WriteStage::Flush);
        if self.fail_at == Some(WriteStage::Flush) {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "fixture flush failure",
            ));
        }
        Ok(())
    }
}

struct PanicReader;

impl Read for PanicReader {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        panic!("invalid arguments must be rejected before reading stdin")
    }
}

struct ErrorAfterReader {
    bytes: Vec<u8>,
    position: usize,
    fail_after: usize,
    message: &'static str,
}

impl ErrorAfterReader {
    fn new(bytes: Vec<u8>, fail_after: usize, message: &'static str) -> Self {
        Self {
            bytes,
            position: 0,
            fail_after,
            message,
        }
    }
}

impl Read for ErrorAfterReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.position >= self.fail_after {
            return Err(io::Error::other(self.message));
        }
        let available = self
            .bytes
            .len()
            .min(self.fail_after)
            .saturating_sub(self.position);
        if available == 0 {
            return Ok(0);
        }
        let count = available.min(buffer.len());
        buffer[..count].copy_from_slice(&self.bytes[self.position..self.position + count]);
        self.position += count;
        Ok(count)
    }
}

fn response_line(writer: &RecordingWriter) -> Value {
    assert_eq!(
        writer.events,
        [WriteStage::Body, WriteStage::Newline, WriteStage::Flush]
    );
    assert_eq!(writer.bytes.last(), Some(&b'\n'));
    assert_eq!(
        writer.bytes.iter().filter(|byte| **byte == b'\n').count(),
        1
    );
    serde_json::from_slice(&writer.bytes[..writer.bytes.len() - 1])
        .expect("compact response line should contain JSON")
}

fn run_recorded(path: &Path, request: &[u8], expected: ResponseClass) -> Value {
    let mut writer = RecordingWriter::default();
    let class = run(path, Cursor::new(request), &mut writer).expect("modeled run should deliver");
    let response = response_line(&writer);
    assert_eq!(class, expected, "unexpected modeled response: {response}");
    response
}

#[test]
fn test_process_shell_requires_exactly_one_explicit_database_path() {
    let directory = TestDirectory::new("args");
    let candidate = directory.path("candidate.sqlite3");
    let invalid = vec![
        Vec::new(),
        vec![OsString::from("cubikan-local")],
        vec![
            OsString::from("cubikan-local"),
            OsString::from("--database"),
        ],
        vec![
            OsString::from("cubikan-local"),
            OsString::from("--database"),
            OsString::new(),
        ],
        vec![
            OsString::from("cubikan-local"),
            OsString::from("--database"),
            OsString::from(":memory:"),
        ],
        vec![
            OsString::from("cubikan-local"),
            OsString::from("--other"),
            candidate.as_os_str().to_owned(),
        ],
        vec![
            OsString::from("cubikan-local"),
            candidate.as_os_str().to_owned(),
        ],
        vec![
            OsString::from("cubikan-local"),
            OsString::from("--database"),
            candidate.as_os_str().to_owned(),
            OsString::from("extra"),
        ],
        vec![
            OsString::from("cubikan-local"),
            OsString::from("--database"),
            candidate.as_os_str().to_owned(),
            OsString::from("--database"),
            candidate.as_os_str().to_owned(),
        ],
    ];

    for arguments in invalid {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = run_process(arguments, PanicReader, &mut stdout, &mut stderr);
        assert_eq!(exit, 2);
        assert!(stdout.is_empty(), "usage failures are not JSON responses");
        assert_eq!(stderr, b"usage: cubikan-local --database PATH\n");
        assert!(!candidate.exists());
    }

    let mut failing_stderr = RecordingWriter::failing(WriteStage::Body);
    assert_eq!(
        run_process(
            vec![OsString::from("cubikan-local")],
            PanicReader,
            Vec::new(),
            &mut failing_stderr,
        ),
        2,
        "usage diagnostics are best effort and must not change exit 2"
    );

    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;

        let mut opaque = directory.root.clone();
        opaque.push(OsString::from_vec(b"opaque-\x80.sqlite3".to_vec()));
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = run_process(
            args(&opaque),
            Cursor::new(create_request()),
            &mut stdout,
            &mut stderr,
        );
        assert_eq!(exit, 0);
        assert!(opaque.exists());
        assert!(stderr.is_empty());
    }
}

#[test]
fn test_runner_dispatches_one_command_and_flushes_one_response() {
    let directory = TestDirectory::new("dispatch");
    let database = directory.path("cubikan.sqlite3");

    let created = run_recorded(&database, &create_request(), ResponseClass::Success);
    assert_eq!(created["result"]["type"], "unit");
    let fetched = run_recorded(&database, &get_request(ID), ResponseClass::Success);
    assert_eq!(fetched["result"]["type"], "unit");
    let page = run_recorded(&database, &list_request(), ResponseClass::Success);
    assert_eq!(page["result"]["type"], "page");
    let transitioned = run_recorded(&database, &transition_request("0"), ResponseClass::Success);
    assert_eq!(transitioned["result"]["type"], "mutation");
    let completed = run_recorded(&database, &complete_request("1"), ResponseClass::Success);
    assert_eq!(completed["result"]["type"], "mutation");

    let rejected = run_recorded(&database, b"{", ResponseClass::RequestRejected);
    assert_eq!(rejected["error"]["code"], "malformed_json");
    let command = run_recorded(
        &database,
        &get_request(MISSING_ID),
        ResponseClass::CommandRejected,
    );
    assert_eq!(command["error"]["code"], "intent_unit_not_found");
    let unavailable = directory.path("missing-parent").join("cubikan.sqlite3");
    let storage = run_recorded(
        &unavailable,
        &get_request(ID),
        ResponseClass::StorageRejected,
    );
    assert_eq!(storage["error"]["code"], "storage_error");
}

#[test]
fn test_runner_enforces_one_mib_before_json_or_database() {
    let directory = TestDirectory::new("bound");
    let database = directory.path("exact.sqlite3");
    let prefix =
        br#"{"protocol_version":1,"operation":{"type":"create","intent_unit":{"species":""#;
    let suffix = br#""},"workflow":{"id":"flow","phases":["a"],"initial_phase":"a","edges":[],"completion_phases":[]}}}"#;
    let padding = MAX_REQUEST_BYTES
        .checked_sub(prefix.len() + suffix.len())
        .expect("fixture framing must fit inside the request bound");
    let mut exact = Vec::with_capacity(MAX_REQUEST_BYTES);
    exact.extend_from_slice(prefix);
    exact.extend(std::iter::repeat_n(b'x', padding));
    exact.extend_from_slice(suffix);
    assert_eq!(exact.len(), MAX_REQUEST_BYTES);
    assert_eq!(exact.last(), Some(&b'}'));

    let accepted = run_recorded(&database, &exact, ResponseClass::Success);
    assert_eq!(accepted["outcome"], "success");
    assert!(database.exists());

    let sentinel = directory.path("oversize.sqlite3");
    let mut oversize = exact;
    oversize.extend_from_slice(b"!ignored");
    let mut reader = ErrorAfterReader::new(
        oversize,
        MAX_REQUEST_BYTES + 1,
        "error after retained lookahead",
    );
    let mut writer = RecordingWriter::default();
    let class = run(&sentinel, &mut reader, &mut writer).expect("oversize is a modeled rejection");
    assert_eq!(class, ResponseClass::RequestRejected);
    assert_eq!(reader.position, MAX_REQUEST_BYTES + 1);
    let response = response_line(&writer);
    assert_eq!(response["error"]["code"], "request_too_large");
    assert!(!sentinel.exists(), "oversize input must not open storage");
}

#[test]
fn test_runner_preserves_first_io_error_precedence() {
    let directory = TestDirectory::new("io-errors");
    let database = directory.path("sentinel.sqlite3");
    let mut exact_bound_error_reader = ErrorAfterReader::new(
        vec![b' '; MAX_REQUEST_BYTES + 1],
        MAX_REQUEST_BYTES,
        "exact-bound read failure",
    );
    let mut exact_bound_writer = RecordingWriter::default();
    let exact_bound_error = run(
        &database,
        &mut exact_bound_error_reader,
        &mut exact_bound_writer,
    )
    .expect_err("an error before byte MAX+1 must outrank JSON and storage");
    assert_run_error(
        &exact_bound_error,
        WriteStageOrRead::Read,
        "exact-bound read failure",
    );
    assert_eq!(exact_bound_error_reader.position, MAX_REQUEST_BYTES);
    assert!(exact_bound_writer.events.is_empty());
    assert!(!database.exists());

    let mut reader = ErrorAfterReader::new(
        create_request(),
        create_request().len() - 1,
        "fixture read failure",
    );
    let mut untouched_writer = RecordingWriter::default();
    let read_error = run(&database, &mut reader, &mut untouched_writer)
        .expect_err("read failure must remain operational");
    assert_run_error(&read_error, WriteStageOrRead::Read, "fixture read failure");
    assert!(untouched_writer.events.is_empty());
    assert!(!database.exists());

    for (stage, expected) in [
        (WriteStage::Body, WriteStageOrRead::Body),
        (WriteStage::Newline, WriteStageOrRead::Newline),
        (WriteStage::Flush, WriteStageOrRead::Flush),
    ] {
        let mut writer = RecordingWriter::failing(stage);
        let error = run(&database, Cursor::new(b"{"), &mut writer)
            .expect_err("delivery failure must remain operational");
        let expected_message = match stage {
            WriteStage::Body => "fixture body failure",
            WriteStage::Newline => "fixture newline failure",
            WriteStage::Flush => "fixture flush failure",
        };
        assert_run_error(&error, expected, expected_message);
        let expected_events = match stage {
            WriteStage::Body => vec![WriteStage::Body],
            WriteStage::Newline => vec![WriteStage::Body, WriteStage::Newline],
            WriteStage::Flush => vec![WriteStage::Body, WriteStage::Newline, WriteStage::Flush],
        };
        assert_eq!(writer.events, expected_events);
        assert!(!database.exists());
    }
}

#[derive(Clone, Copy)]
enum WriteStageOrRead {
    Read,
    Body,
    Newline,
    Flush,
}

fn assert_run_error(error: &RunError, expected: WriteStageOrRead, message: &str) {
    let source = match (expected, error) {
        (WriteStageOrRead::Read, RunError::ReadRequest(source))
        | (WriteStageOrRead::Body, RunError::WriteResponse(source))
        | (WriteStageOrRead::Newline, RunError::WriteNewline(source))
        | (WriteStageOrRead::Flush, RunError::FlushResponse(source)) => source,
        _ => panic!("unexpected run error variant: {error:?}"),
    };
    let expected_kind = match expected {
        WriteStageOrRead::Read => io::ErrorKind::Other,
        WriteStageOrRead::Body | WriteStageOrRead::Newline | WriteStageOrRead::Flush => {
            io::ErrorKind::BrokenPipe
        }
    };
    assert_eq!(source.kind(), expected_kind);
    assert_eq!(source.to_string(), message);
    let chained = error
        .source()
        .expect("run error must retain its I/O source");
    assert_eq!(chained.to_string(), message);
}

#[test]
fn test_committed_mutation_survives_response_delivery_failures() {
    let directory = TestDirectory::new("delivery-uncertainty");
    for (ordinal, stage) in [WriteStage::Body, WriteStage::Newline, WriteStage::Flush]
        .into_iter()
        .enumerate()
    {
        let database = directory.path(&format!("case-{ordinal}.sqlite3"));
        let mut seed_output = Vec::new();
        assert_eq!(
            run(&database, Cursor::new(create_request()), &mut seed_output).unwrap(),
            ResponseClass::Success
        );

        let mut writer = RecordingWriter::failing(stage);
        let error = run(&database, Cursor::new(transition_request("0")), &mut writer)
            .expect_err("response delivery should fail after commit");
        assert!(
            error.to_string().contains("committed outcome is unknown"),
            "delivery diagnostic must communicate uncertainty"
        );

        let fresh = SqliteBackend::open(&database).expect("committed database should reopen");
        let view = fresh
            .get(ID.parse().expect("fixture ID should parse"))
            .expect("committed successor should remain readable");
        assert_eq!(view.revision(), IntentUnitRevision::new(1));
        assert_eq!(view.phase().as_str(), "done");
        assert_eq!(view.status(), IntentUnitStatus::Active);
        assert_eq!(view.history().len(), 1);
    }
}

#[test]
fn test_local_process_exit_and_stderr_mapping() {
    let directory = TestDirectory::new("exits");

    let success_database = directory.path("success.sqlite3");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    assert_eq!(
        run_process(
            args(&success_database),
            Cursor::new(create_request()),
            &mut stdout,
            &mut stderr,
        ),
        0
    );
    assert!(stderr.is_empty());
    assert!(stdout.ends_with(b"\n"));

    let request_database = directory.path("request.sqlite3");
    stdout.clear();
    stderr.clear();
    assert_eq!(
        run_process(
            args(&request_database),
            Cursor::new(b"{"),
            &mut stdout,
            &mut stderr,
        ),
        2
    );
    assert!(stderr.is_empty());
    assert_eq!(response_code(&stdout), "malformed_json");

    let command_database = directory.path("command.sqlite3");
    stdout.clear();
    stderr.clear();
    assert_eq!(
        run_process(
            args(&command_database),
            Cursor::new(get_request(MISSING_ID)),
            &mut stdout,
            &mut stderr,
        ),
        3
    );
    assert!(stderr.is_empty());
    assert_eq!(response_code(&stdout), "intent_unit_not_found");

    let storage_database = directory.path("missing-parent").join("cubikan.sqlite3");
    stdout.clear();
    stderr.clear();
    assert_eq!(
        run_process(
            args(&storage_database),
            Cursor::new(get_request(ID)),
            &mut stdout,
            &mut stderr,
        ),
        4
    );
    assert!(stderr.is_empty());
    assert_eq!(response_code(&stdout), "storage_error");

    let operational_database = directory.path("operational.sqlite3");
    stdout.clear();
    stderr.clear();
    let reader = ErrorAfterReader::new(Vec::new(), 0, "shell read failure");
    assert_eq!(
        run_process(
            args(&operational_database),
            reader,
            &mut stdout,
            &mut stderr,
        ),
        1
    );
    assert!(stdout.is_empty());
    let diagnostic = String::from_utf8(stderr.clone()).unwrap();
    assert!(diagnostic.contains("failed to read local request"));
    assert!(diagnostic.contains("shell read failure"));
    assert!(!operational_database.exists());

    let reader = ErrorAfterReader::new(Vec::new(), 0, "hidden diagnostic");
    let mut unavailable_stderr = RecordingWriter::failing(WriteStage::Body);
    assert_eq!(
        run_process(
            args(&operational_database),
            reader,
            Vec::new(),
            &mut unavailable_stderr,
        ),
        1,
        "operational diagnostics are best effort"
    );
}

fn response_code(stdout: &[u8]) -> String {
    let response: Value = serde_json::from_slice(stdout).expect("stdout should be one JSON line");
    response["error"]["code"]
        .as_str()
        .expect("failure should have a code")
        .to_owned()
}
