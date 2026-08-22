use std::{
    error::Error,
    ffi::{OsStr, OsString},
    fmt,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use crate::{
    execution::{Execution, execute_request},
    protocol::request_too_large_response,
};

/// Maximum number of raw request bytes accepted by the local runner.
pub const MAX_REQUEST_BYTES: usize = 1_048_576;

const MAX_REQUEST_BYTES_WITH_LOOKAHEAD: usize = MAX_REQUEST_BYTES + 1;
const EXIT_OPERATIONAL: u8 = 1;
const EXIT_REQUEST: u8 = 2;
const USAGE: &[u8] =
    b"usage: cubikan-local --database PATH --rpc URL [--dev-signer charlie|dave]\n";

#[derive(Debug)]
enum RunError {
    ReadRequest(io::Error),
    WriteResponseBody(io::Error),
    WriteResponseNewline(io::Error),
    FlushResponse(io::Error),
    AcknowledgeResponse(Box<dyn Error + 'static>),
}

impl fmt::Display for RunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadRequest(error) => write!(formatter, "failed to read request: {error}"),
            Self::WriteResponseBody(error) => {
                write!(formatter, "failed to write response body: {error}")
            }
            Self::WriteResponseNewline(error) => {
                write!(formatter, "failed to write response newline: {error}")
            }
            Self::FlushResponse(error) => write!(formatter, "failed to flush response: {error}"),
            Self::AcknowledgeResponse(_) => {
                formatter.write_str("failed to acknowledge durable submission response")
            }
        }
    }
}

impl Error for RunError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadRequest(error)
            | Self::WriteResponseBody(error)
            | Self::WriteResponseNewline(error)
            | Self::FlushResponse(error) => Some(error),
            Self::AcknowledgeResponse(error) => Some(error.as_ref()),
        }
    }
}

struct ProcessArguments {
    database: PathBuf,
    rpc: OsString,
    signer: Option<OsString>,
}

trait DeliverableResponse {
    fn body(&self) -> &[u8];
    fn exit_code(&self) -> u8;
    fn acknowledge_response_durable(self) -> Result<(), Box<dyn Error + 'static>>;
}

impl DeliverableResponse for Execution {
    fn body(&self) -> &[u8] {
        Execution::body(self)
    }

    fn exit_code(&self) -> u8 {
        Execution::class(self).exit_code()
    }

    fn acknowledge_response_durable(self) -> Result<(), Box<dyn Error + 'static>> {
        Execution::acknowledge_response_durable(self)
            .map_err(|error| Box::new(error) as Box<dyn Error + 'static>)
    }
}

/// Runs the injectable process shell and returns its exact process exit code.
///
/// This is the only library entry point. It retains at most one byte beyond the
/// one-MiB ingress bound, emits exactly one compact response line, and does not
/// acknowledge a resolved submission journal until that line has been flushed.
pub async fn run_process<I, R, W, E>(args_os: I, reader: R, writer: W, mut stderr: E) -> u8
where
    I: IntoIterator<Item = OsString>,
    R: Read,
    W: Write,
    E: Write,
{
    let arguments = match parse_process_arguments(args_os) {
        Ok(arguments) => arguments,
        Err(()) => {
            write_usage(&mut stderr);
            return EXIT_REQUEST;
        }
    };

    let request = match read_request(reader) {
        Ok(request) => request,
        Err(error) => {
            write_operational_error(&mut stderr, &error);
            return EXIT_OPERATIONAL;
        }
    };

    let execution = if request.len() > MAX_REQUEST_BYTES {
        Execution::modeled(request_too_large_response())
    } else {
        match execute_request(
            &arguments.database,
            arguments.rpc.as_os_str(),
            arguments.signer.as_deref(),
            &request,
        )
        .await
        {
            Ok(execution) => execution,
            Err(_) => {
                write_usage(&mut stderr);
                return EXIT_REQUEST;
            }
        }
    };

    match deliver_response(execution, writer) {
        Ok(exit_code) => exit_code,
        Err(error) => {
            write_operational_error(&mut stderr, &error);
            EXIT_OPERATIONAL
        }
    }
}

fn parse_process_arguments<I>(args_os: I) -> Result<ProcessArguments, ()>
where
    I: IntoIterator<Item = OsString>,
{
    let mut arguments = args_os.into_iter();
    let _program = arguments.next().ok_or(())?;

    if arguments.next().as_deref() != Some(OsStr::new("--database")) {
        return Err(());
    }
    let database = arguments.next().ok_or(())?;
    if database.is_empty() || Path::new(&database) == Path::new(":memory:") {
        return Err(());
    }

    if arguments.next().as_deref() != Some(OsStr::new("--rpc")) {
        return Err(());
    }
    let rpc = arguments.next().ok_or(())?;

    let signer = match arguments.next() {
        None => None,
        Some(flag) if flag == OsStr::new("--dev-signer") => Some(arguments.next().ok_or(())?),
        Some(_) => return Err(()),
    };
    if arguments.next().is_some() {
        return Err(());
    }

    Ok(ProcessArguments {
        database: PathBuf::from(database),
        rpc,
        signer,
    })
}

fn read_request(reader: impl Read) -> Result<Vec<u8>, RunError> {
    let read_limit = u64::try_from(MAX_REQUEST_BYTES_WITH_LOOKAHEAD)
        .expect("the request bound plus one byte must fit u64");
    let mut request = Vec::with_capacity(MAX_REQUEST_BYTES_WITH_LOOKAHEAD);
    reader
        .take(read_limit)
        .read_to_end(&mut request)
        .map_err(RunError::ReadRequest)?;
    Ok(request)
}

fn deliver_response<T, W>(response: T, mut writer: W) -> Result<u8, RunError>
where
    T: DeliverableResponse,
    W: Write,
{
    let exit_code = response.exit_code();
    writer
        .write_all(response.body())
        .map_err(RunError::WriteResponseBody)?;
    writer
        .write_all(b"\n")
        .map_err(RunError::WriteResponseNewline)?;
    writer.flush().map_err(RunError::FlushResponse)?;
    response
        .acknowledge_response_durable()
        .map_err(RunError::AcknowledgeResponse)?;
    Ok(exit_code)
}

fn write_usage(stderr: &mut impl Write) {
    let _ = stderr.write_all(USAGE);
    let _ = stderr.flush();
}

fn write_operational_error(stderr: &mut impl Write, error: &RunError) {
    let _ = writeln!(stderr, "cubikan-local: {error}");
    let _ = stderr.flush();
}

#[cfg(test)]
mod tests {
    use std::{
        cell::Cell,
        collections::BTreeSet,
        io::{Cursor, Read, Write},
        rc::Rc,
    };

    use serde_json::Value;

    use super::*;

    const PROCESS_ORACLE: &str =
        include_str!("../../../tests/fixtures/protocol-v2/cubikan-local/process-v1.json");
    const IO_ORACLE: &str =
        include_str!("../../../tests/fixtures/protocol-v2/cubikan-local/io-v1.json");
    const PUBLIC_SCHEMA: &str = include_str!("../../../protocol/v2/cubikan-local.schema.json");
    const USAGE_ORACLE: &[u8] =
        include_bytes!("../../../tests/fixtures/protocol-v2/cubikan-local/process/usage.txt");
    const EMPTY: &[u8] =
        include_bytes!("../../../tests/fixtures/protocol-v2/cubikan-local/process/empty.bin");
    const REQUEST_TOO_LARGE_LINE: &[u8] = include_bytes!(
        "../../../tests/fixtures/protocol-v2/cubikan-local/stdout/error_request_too_large.jsonl"
    );
    const READ_REQUEST: &[u8] = include_bytes!(
        "../../../tests/fixtures/protocol-v2/cubikan-local/requests/success_get_intent_unit.json"
    );
    const MUTATION_REQUEST: &[u8] = include_bytes!(
        "../../../tests/fixtures/protocol-v2/cubikan-local/requests/success_create_intent_unit.json"
    );
    const SIZE_1048575_REQUEST: &[u8] = include_bytes!(
        "../../../tests/fixtures/protocol-v2/cubikan-local/requests/size_exact_1048575.json"
    );
    const SIZE_1048576_REQUEST: &[u8] = include_bytes!(
        "../../../tests/fixtures/protocol-v2/cubikan-local/requests/size_exact_1048576.json"
    );
    const SIZE_1048577_REQUEST: &[u8] = include_bytes!(
        "../../../tests/fixtures/protocol-v2/cubikan-local/requests/error_request_too_large.json"
    );
    const READ_BODY: &[u8] =
        include_bytes!("../../../tests/fixtures/protocol-v2/cubikan-local/io/stdout-read_body.bin");
    const READ_BODY_PREFIX_17: &[u8] = include_bytes!(
        "../../../tests/fixtures/protocol-v2/cubikan-local/io/stdout-read_body_prefix17.bin"
    );
    const READ_LINE: &[u8] =
        include_bytes!("../../../tests/fixtures/protocol-v2/cubikan-local/io/stdout-read_line.bin");
    const MUTATION_BODY: &[u8] = include_bytes!(
        "../../../tests/fixtures/protocol-v2/cubikan-local/io/stdout-mutation_body.bin"
    );
    const MUTATION_BODY_PREFIX_17: &[u8] = include_bytes!(
        "../../../tests/fixtures/protocol-v2/cubikan-local/io/stdout-mutation_body_prefix17.bin"
    );
    const MUTATION_LINE: &[u8] = include_bytes!(
        "../../../tests/fixtures/protocol-v2/cubikan-local/io/stdout-mutation_line.bin"
    );
    const STDERR_READ: &[u8] = include_bytes!(
        "../../../tests/fixtures/protocol-v2/cubikan-local/io/stderr-read_request.txt"
    );
    const STDERR_BODY: &[u8] =
        include_bytes!("../../../tests/fixtures/protocol-v2/cubikan-local/io/stderr-body.txt");
    const STDERR_NEWLINE: &[u8] =
        include_bytes!("../../../tests/fixtures/protocol-v2/cubikan-local/io/stderr-newline.txt");
    const STDERR_FLUSH: &[u8] =
        include_bytes!("../../../tests/fixtures/protocol-v2/cubikan-local/io/stderr-flush.txt");
    const STDERR_ACKNOWLEDGE: &[u8] = include_bytes!(
        "../../../tests/fixtures/protocol-v2/cubikan-local/io/stderr-acknowledge.txt"
    );

    struct PanicReader;

    impl Read for PanicReader {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            panic!("structurally invalid arguments must reject before stdin")
        }
    }

    #[tokio::test]
    async fn test_local_process_arguments_and_serialized_surface_are_safe() {
        let oracle: Value =
            serde_json::from_str(PROCESS_ORACLE).expect("process oracle must be JSON");
        let cases = oracle["cases"]
            .as_array()
            .expect("process oracle cases must be an array");
        assert_eq!(cases.len(), 72);
        assert_eq!(USAGE, USAGE_ORACLE);
        assert_eq!(oracle["usage"]["bytes"], USAGE.len());

        for case in cases {
            let id = case["id"].as_str().expect("process case id");
            let arguments = oracle_arguments(case);
            let consumed = case["stdin_bytes_consumed"]
                .as_u64()
                .expect("stdin byte count");
            let parsed = parse_process_arguments(arguments.clone());
            assert_eq!(
                parsed.is_ok(),
                consumed != 0,
                "{id}: structural parse/ingress ordering drift"
            );

            if consumed == 0 {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                let exit = run_process(arguments, PanicReader, &mut stdout, &mut stderr).await;
                assert_eq!(exit, EXIT_REQUEST, "{id}: structural exit");
                assert_eq!(stdout, EMPTY, "{id}: structural stdout");
                assert_eq!(stderr, USAGE_ORACLE, "{id}: structural stderr");
            }
        }

        let memory_arguments = [
            "cubikan-local",
            "--database",
            ":memory:",
            "--rpc",
            "ws://127.0.0.1:9944/",
        ]
        .map(OsString::from);
        assert!(parse_process_arguments(memory_arguments.clone()).is_err());
        let mut stderr = Vec::new();
        assert_eq!(
            run_process(memory_arguments, PanicReader, Vec::new(), &mut stderr).await,
            EXIT_REQUEST
        );
        assert_eq!(stderr, USAGE_ORACLE);

        for id in [
            "argv_raw_seed",
            "argv_private_key",
            "argv_journal_path",
            "argv_pid_forbidden",
        ] {
            let case = oracle_case(&oracle, id);
            assert_eq!(case["stdin_bytes_consumed"], 0, "{id}: security ordering");
            assert!(parse_process_arguments(oracle_arguments(case)).is_err());
        }

        let read = parse_process_arguments(oracle_arguments(oracle_case(
            &oracle,
            "url_accept_ipv4_min",
        )))
        .expect("read arguments must be structurally accepted");
        assert!(read.signer.is_none());
        let mutation = parse_process_arguments(oracle_arguments(oracle_case(
            &oracle,
            "signer_accept_charlie_mutation",
        )))
        .expect("mutation arguments must be structurally accepted");
        assert_eq!(mutation.signer.as_deref(), Some(OsStr::new("charlie")));

        for (id, request) in [
            ("argv_read_with_signer", READ_REQUEST),
            ("argv_mutation_without_signer", MUTATION_REQUEST),
            ("argv_unknown_signer", MUTATION_REQUEST),
            ("argv_uppercase_signer", MUTATION_REQUEST),
        ] {
            let case = oracle_case(&oracle, id);
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let exit = run_process(
                oracle_arguments(case),
                Cursor::new(request),
                &mut stdout,
                &mut stderr,
            )
            .await;
            assert_eq!(exit, EXIT_REQUEST, "{id}: post-decode usage exit");
            assert!(stdout.is_empty(), "{id}: post-decode usage stdout");
            assert_eq!(stderr, USAGE_ORACLE, "{id}: post-decode usage stderr");
        }

        let public_surface = include_str!("lib.rs");
        assert_eq!(public_surface.matches("pub use").count(), 1);
        assert!(public_surface.contains("pub use runner::{MAX_REQUEST_BYTES, run_process};"));
        for forbidden in [
            "pub use execution",
            "pub use protocol",
            "execute_request;",
            "ExecutedRequest",
            "ResponseClass",
            "RunError",
        ] {
            assert!(
                !public_surface.contains(forbidden),
                "raw local authority escaped through public surface: {forbidden}"
            );
        }

        let schema: Value =
            serde_json::from_str(PUBLIC_SCHEMA).expect("public schema must be JSON");
        let mut serialized_members = BTreeSet::new();
        collect_property_names(&schema, &mut serialized_members);
        for forbidden in [
            "secret",
            "seed",
            "private_key",
            "owner",
            "author",
            "source_body",
            "source_payload",
            "provider",
            "rpc",
            "signer",
            "journal",
        ] {
            assert!(
                !serialized_members.contains(forbidden),
                "forbidden serialized member escaped: {forbidden}"
            );
        }

        crate::execution::tests::assert_strict_url_dial_contract_for_e4().await;
    }

    #[test]
    fn test_local_protocol_preserves_one_mib_and_delivery_contract() {
        assert_eq!(MAX_REQUEST_BYTES, 1_048_576);
        assert_eq!(MAX_REQUEST_BYTES_WITH_LOOKAHEAD, 1_048_577);

        let process_oracle: Value =
            serde_json::from_str(PROCESS_ORACLE).expect("process oracle must be JSON");
        for (id, size, exit, acknowledgements) in [
            ("ingress_accept_1048575", 1_048_575, 0, 1),
            ("ingress_accept_1048576", 1_048_576, 0, 1),
            ("ingress_reject_1048577", 1_048_577, 2, 0),
        ] {
            let case = oracle_case(&process_oracle, id);
            assert_eq!(case["request"]["bytes"], size, "{id}: request bytes");
            assert_eq!(case["stdin_bytes_consumed"], size, "{id}: ingress bytes");
            assert_eq!(case["exit_code"], exit, "{id}: exit");
            assert_eq!(
                case["acknowledge_count"], acknowledgements,
                "{id}: acknowledgement count"
            );
        }

        for (id, artifact, size) in [
            ("size_exact_1048575", SIZE_1048575_REQUEST, 1_048_575),
            ("size_exact_1048576", SIZE_1048576_REQUEST, 1_048_576),
            ("error_request_too_large", SIZE_1048577_REQUEST, 1_048_577),
        ] {
            assert_eq!(artifact.len(), size, "{id}: frozen artifact size");
            let request = read_request(Cursor::new(artifact))
                .unwrap_or_else(|error| panic!("{id}: bounded read failed: {error}"));
            assert_eq!(request, artifact, "{id}: bounded reader changed bytes");
            assert!(request.len() <= MAX_REQUEST_BYTES_WITH_LOOKAHEAD);
        }

        let request = read_request(Cursor::new(vec![b' '; 1_048_578]))
            .expect("bounded reader must retain at most one lookahead byte");
        assert_eq!(request.len(), MAX_REQUEST_BYTES_WITH_LOOKAHEAD);

        let mut oversized_stdout = Vec::new();
        let oversized = Execution::modeled(request_too_large_response());
        assert_eq!(
            deliver_response(oversized, &mut oversized_stdout)
                .expect("modeled oversize response must deliver"),
            EXIT_REQUEST
        );
        assert_eq!(oversized_stdout, REQUEST_TOO_LARGE_LINE);

        let io_oracle: Value = serde_json::from_str(IO_ORACLE).expect("I/O oracle must be JSON");
        let expected_io = [
            ("request_read_failure", 0, 0, 0, 0, "not_applicable"),
            ("read_response_body_failure", 1, 0, 0, 0, "not_applicable"),
            (
                "read_response_newline_failure",
                1,
                1,
                0,
                0,
                "not_applicable",
            ),
            ("read_response_flush_failure", 1, 1, 1, 0, "not_applicable"),
            (
                "mutation_response_body_failure",
                1,
                0,
                0,
                0,
                "resolved_retained",
            ),
            (
                "mutation_response_newline_failure",
                1,
                1,
                0,
                0,
                "resolved_retained",
            ),
            (
                "mutation_response_flush_failure",
                1,
                1,
                1,
                0,
                "resolved_retained",
            ),
            (
                "mutation_acknowledgement_failure",
                1,
                1,
                1,
                1,
                "resolved_retained",
            ),
        ];
        assert_eq!(
            io_oracle["cases"].as_array().expect("I/O cases").len(),
            expected_io.len()
        );
        for (id, body, newline, flush, acknowledge, journal) in expected_io {
            let case = oracle_case(&io_oracle, id);
            assert_eq!(case["exit_code"], EXIT_OPERATIONAL, "{id}: exit");
            assert_eq!(case["response_body_attempts"], body, "{id}: body");
            assert_eq!(case["newline_attempts"], newline, "{id}: newline");
            assert_eq!(case["flush_attempts"], flush, "{id}: flush");
            assert_eq!(
                case["acknowledge_attempts"], acknowledge,
                "{id}: acknowledge"
            );
            assert_eq!(case["durable_journal_state"], journal, "{id}: journal");
        }

        let read_error = read_request(FixtureReadFailure).expect_err("read must fail");
        assert_error_stderr(&read_error, STDERR_READ);

        assert_delivery_failure(
            READ_BODY,
            DeliveryFault::BodyAfter17,
            READ_BODY_PREFIX_17,
            STDERR_BODY,
        );
        assert_delivery_failure(READ_BODY, DeliveryFault::Newline, READ_BODY, STDERR_NEWLINE);
        assert_delivery_failure(READ_BODY, DeliveryFault::Flush, READ_LINE, STDERR_FLUSH);
        assert_delivery_failure(
            MUTATION_BODY,
            DeliveryFault::BodyAfter17,
            MUTATION_BODY_PREFIX_17,
            STDERR_BODY,
        );
        assert_delivery_failure(
            MUTATION_BODY,
            DeliveryFault::Newline,
            MUTATION_BODY,
            STDERR_NEWLINE,
        );
        assert_delivery_failure(
            MUTATION_BODY,
            DeliveryFault::Flush,
            MUTATION_LINE,
            STDERR_FLUSH,
        );

        let acknowledgements = Rc::new(Cell::new(0));
        let response = FakeResponse {
            body: MUTATION_BODY,
            exit_code: 0,
            acknowledge_ok: false,
            acknowledgements: Rc::clone(&acknowledgements),
        };
        let mut writer = FaultWriter::new(DeliveryFault::None);
        let error = deliver_response(response, &mut writer).expect_err("acknowledgement must fail");
        assert_eq!(writer.bytes, MUTATION_LINE);
        assert_eq!(acknowledgements.get(), 1);
        assert_error_stderr(&error, STDERR_ACKNOWLEDGE);
    }

    fn oracle_arguments(case: &Value) -> Vec<OsString> {
        case["argv"]
            .as_array()
            .expect("argv must be an array")
            .iter()
            .map(|argument| OsString::from(argument.as_str().expect("argv string")))
            .collect()
    }

    fn collect_property_names(value: &Value, names: &mut BTreeSet<String>) {
        match value {
            Value::Object(object) => {
                if let Some(Value::Object(properties)) = object.get("properties") {
                    names.extend(properties.keys().cloned());
                }
                for nested in object.values() {
                    collect_property_names(nested, names);
                }
            }
            Value::Array(array) => {
                for nested in array {
                    collect_property_names(nested, names);
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        }
    }

    fn oracle_case<'a>(oracle: &'a Value, id: &str) -> &'a Value {
        oracle["cases"]
            .as_array()
            .expect("oracle cases")
            .iter()
            .find(|case| case["id"] == id)
            .unwrap_or_else(|| panic!("missing oracle case {id}"))
    }

    struct FixtureReadFailure;

    impl Read for FixtureReadFailure {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("fixture read failure"))
        }
    }

    struct FakeResponse {
        body: &'static [u8],
        exit_code: u8,
        acknowledge_ok: bool,
        acknowledgements: Rc<Cell<usize>>,
    }

    impl DeliverableResponse for FakeResponse {
        fn body(&self) -> &[u8] {
            self.body
        }

        fn exit_code(&self) -> u8 {
            self.exit_code
        }

        fn acknowledge_response_durable(self) -> Result<(), Box<dyn Error + 'static>> {
            self.acknowledgements
                .set(self.acknowledgements.get().saturating_add(1));
            if self.acknowledge_ok {
                Ok(())
            } else {
                Err(Box::new(io::Error::other(
                    "fixture acknowledgement failure",
                )))
            }
        }
    }

    #[derive(Clone, Copy)]
    enum DeliveryFault {
        None,
        BodyAfter17,
        Newline,
        Flush,
    }

    struct FaultWriter {
        fault: DeliveryFault,
        bytes: Vec<u8>,
    }

    impl FaultWriter {
        fn new(fault: DeliveryFault) -> Self {
            Self {
                fault,
                bytes: Vec::new(),
            }
        }
    }

    impl Write for FaultWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            match self.fault {
                DeliveryFault::BodyAfter17 if self.bytes.len() >= 17 => {
                    Err(io::Error::other("fixture body failure"))
                }
                DeliveryFault::BodyAfter17 => {
                    let accepted = bytes.len().min(17 - self.bytes.len());
                    self.bytes.extend_from_slice(&bytes[..accepted]);
                    Ok(accepted)
                }
                DeliveryFault::Newline if bytes == b"\n" => {
                    Err(io::Error::other("fixture newline failure"))
                }
                _ => {
                    self.bytes.extend_from_slice(bytes);
                    Ok(bytes.len())
                }
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            if matches!(self.fault, DeliveryFault::Flush) {
                Err(io::Error::other("fixture flush failure"))
            } else {
                Ok(())
            }
        }
    }

    fn assert_delivery_failure(
        body: &'static [u8],
        fault: DeliveryFault,
        expected_stdout: &[u8],
        expected_stderr: &[u8],
    ) {
        let acknowledgements = Rc::new(Cell::new(0));
        let response = FakeResponse {
            body,
            exit_code: 0,
            acknowledge_ok: true,
            acknowledgements: Rc::clone(&acknowledgements),
        };
        let mut writer = FaultWriter::new(fault);
        let error = deliver_response(response, &mut writer).expect_err("delivery must fail");
        assert_eq!(writer.bytes, expected_stdout);
        assert_eq!(acknowledgements.get(), 0);
        assert_error_stderr(&error, expected_stderr);
    }

    fn assert_error_stderr(error: &RunError, expected: &[u8]) {
        let mut stderr = Vec::new();
        write_operational_error(&mut stderr, error);
        assert_eq!(stderr, expected);
    }
}
