use std::{
    ffi::OsString,
    io::{self, Cursor, Read},
    path::{Path, PathBuf},
};

use cubikan_local::{MAX_REQUEST_BYTES, run_process};

const USAGE: &[u8] =
    include_bytes!("../../../tests/fixtures/protocol-v2/cubikan-local/process/usage.txt");
const REQUEST_TOO_LARGE: &[u8] = include_bytes!(
    "../../../tests/fixtures/protocol-v2/cubikan-local/stdout/error_request_too_large.jsonl"
);
const READ_REQUEST: &[u8] = include_bytes!(
    "../../../tests/fixtures/protocol-v2/cubikan-local/requests/success_get_intent_unit.json"
);
const MUTATION_REQUEST: &[u8] = include_bytes!(
    "../../../tests/fixtures/protocol-v2/cubikan-local/requests/success_create_intent_unit.json"
);

struct PanicReader;

impl Read for PanicReader {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        panic!("structural argument rejection must precede stdin")
    }
}

fn database(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "cubikan-local-runner-{}-{label}.sqlite3",
        std::process::id()
    ))
}

fn read_args(database: &Path) -> Vec<OsString> {
    vec![
        OsString::from("cubikan-local"),
        OsString::from("--database"),
        database.as_os_str().to_owned(),
        OsString::from("--rpc"),
        OsString::from("ws://127.0.0.1:9944/"),
    ]
}

#[tokio::test]
async fn public_process_rejects_structural_arguments_before_stdin() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit = run_process(
        [OsString::from("cubikan-local"), OsString::from("--help")],
        PanicReader,
        &mut stdout,
        &mut stderr,
    )
    .await;
    assert_eq!(exit, 2);
    assert!(stdout.is_empty());
    assert_eq!(stderr, USAGE);

    let mut memory_stderr = Vec::new();
    let exit = run_process(
        [
            OsString::from("cubikan-local"),
            OsString::from("--database"),
            OsString::from(":memory:"),
            OsString::from("--rpc"),
            OsString::from("ws://127.0.0.1:9944/"),
        ],
        PanicReader,
        Vec::new(),
        &mut memory_stderr,
    )
    .await;
    assert_eq!(exit, 2);
    assert_eq!(memory_stderr, USAGE);
}

#[tokio::test]
async fn signer_form_is_enforced_after_decode_and_before_rpc() {
    let read_database = database("read-with-signer");
    let mut read_arguments = read_args(&read_database);
    read_arguments.extend([OsString::from("--dev-signer"), OsString::from("charlie")]);
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    assert_eq!(
        run_process(
            read_arguments,
            Cursor::new(READ_REQUEST),
            &mut stdout,
            &mut stderr
        )
        .await,
        2
    );
    assert!(stdout.is_empty());
    assert_eq!(stderr, USAGE);
    assert!(!read_database.exists());

    let mutation_database = database("mutation-without-signer");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    assert_eq!(
        run_process(
            read_args(&mutation_database),
            Cursor::new(MUTATION_REQUEST),
            &mut stdout,
            &mut stderr
        )
        .await,
        2
    );
    assert!(stdout.is_empty());
    assert_eq!(stderr, USAGE);
    assert!(!mutation_database.exists());
}

#[tokio::test]
async fn oversized_request_is_modeled_without_state_access() {
    let database = database("oversized");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit = run_process(
        read_args(&database),
        Cursor::new(vec![b' '; MAX_REQUEST_BYTES + 1]),
        &mut stdout,
        &mut stderr,
    )
    .await;
    assert_eq!(exit, 2);
    assert_eq!(stdout, REQUEST_TOO_LARGE);
    assert!(stderr.is_empty());
    assert!(!database.exists());
}
