use std::{
    ffi::OsString,
    fs,
    io::{self, Cursor, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use cubikan_local::{MAX_REQUEST_BYTES, ResponseClass, run, run_process};
use serde_json::{Value, json};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        for _ in 0..100 {
            let ordinal = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "cubikan-local-unsupported-runner-{}-{ordinal}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
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

#[derive(Default)]
struct RecordingWriter {
    bytes: Vec<u8>,
    flush_offsets: Vec<usize>,
}

impl Write for RecordingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_offsets.push(self.bytes.len());
        Ok(())
    }
}

fn request() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "protocol_version": 1,
        "operation": {
            "type": "create",
            "intent_unit": {"species": "feature"},
            "workflow": {"removed": true}
        }
    }))
    .expect("fixture should serialize")
}

fn args(path: &Path) -> Vec<OsString> {
    vec![
        OsString::from("cubikan-local"),
        OsString::from("--database"),
        path.as_os_str().to_owned(),
    ]
}

fn response(writer: &RecordingWriter) -> Value {
    assert_eq!(writer.bytes.last(), Some(&b'\n'));
    assert_eq!(
        writer.bytes.iter().filter(|byte| **byte == b'\n').count(),
        1
    );
    assert_eq!(writer.flush_offsets, [writer.bytes.len()]);
    serde_json::from_slice(&writer.bytes).expect("response should be JSON")
}

fn padded_request(length: usize) -> Vec<u8> {
    let mut bytes = request();
    assert_eq!(bytes.pop(), Some(b'}'));
    assert!(bytes.len() < length);
    bytes.resize(length - 1, b' ');
    bytes.push(b'}');
    bytes
}

#[test]
fn runner_delivers_one_flushed_rejection_without_creating_the_path() {
    let directory = TestDirectory::new();
    let database = directory.database();
    let mut writer = RecordingWriter::default();

    let class = run(&database, Cursor::new(request()), &mut writer)
        .expect("modeled rejection should be delivered");

    assert_eq!(class, ResponseClass::RequestRejected);
    assert_eq!(
        response(&writer)["error"]["code"],
        "unsupported_protocol_version"
    );
    assert!(!database.exists());
}

#[test]
fn runner_preserves_bound_precedence_without_creating_the_path() {
    assert_eq!(MAX_REQUEST_BYTES, 1_048_576);
    let directory = TestDirectory::new();
    let database = directory.database();
    let mut exact_writer = RecordingWriter::default();
    assert_eq!(
        run(
            &database,
            Cursor::new(padded_request(MAX_REQUEST_BYTES)),
            &mut exact_writer,
        )
        .expect("exact request should respond"),
        ResponseClass::RequestRejected
    );
    assert_eq!(
        response(&exact_writer)["error"]["code"],
        "unsupported_protocol_version"
    );

    let mut oversized_writer = RecordingWriter::default();
    assert_eq!(
        run(
            &database,
            Cursor::new(padded_request(MAX_REQUEST_BYTES + 1)),
            &mut oversized_writer,
        )
        .expect("oversized request should respond"),
        ResponseClass::RequestRejected
    );
    assert_eq!(
        response(&oversized_writer)["error"]["code"],
        "request_too_large"
    );
    assert!(!database.exists());
}

#[test]
fn process_maps_v1_to_exit_two_and_rejects_bad_args_before_reading() {
    struct PanicReader;
    impl Read for PanicReader {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            panic!("invalid arguments must reject before reading stdin")
        }
    }

    let directory = TestDirectory::new();
    let database = directory.database();
    let mut stdout = RecordingWriter::default();
    let mut stderr = Vec::new();
    assert_eq!(
        run_process(
            args(&database),
            Cursor::new(request()),
            &mut stdout,
            &mut stderr,
        ),
        2
    );
    assert!(stderr.is_empty());
    assert_eq!(
        response(&stdout)["error"]["code"],
        "unsupported_protocol_version"
    );
    assert!(!database.exists());

    let mut stderr = Vec::new();
    assert_eq!(
        run_process(
            vec![OsString::from("cubikan-local")],
            PanicReader,
            Vec::new(),
            &mut stderr,
        ),
        2
    );
    assert_eq!(stderr, b"usage: cubikan-local --database PATH\n");
}
