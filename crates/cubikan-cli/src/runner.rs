use std::{error::Error, fmt, io, io::Read, io::Write};

use cubikan_core::IntentUnitId;

use crate::{
    MAX_REQUEST_BYTES,
    execution::{SimulationOutcome, simulate},
    protocol::{ErrorDetail, ProtocolResponse, decode_request},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunStatus {
    Success,
    RequestRejected,
    OperationRejected,
}

#[derive(Debug)]
pub enum RunError {
    Read(io::Error),
    WriteResponse(serde_json::Error),
    WriteNewline(io::Error),
    FlushResponse(io::Error),
}

impl fmt::Display for RunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(formatter, "failed to read request: {error}"),
            Self::WriteResponse(error) => {
                write!(formatter, "failed to write response body: {error}")
            }
            Self::WriteNewline(error) => {
                write!(formatter, "failed to write response newline: {error}")
            }
            Self::FlushResponse(error) => write!(formatter, "failed to flush response: {error}"),
        }
    }
}

impl Error for RunError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read(error) => Some(error),
            Self::WriteResponse(error) => Some(error),
            Self::WriteNewline(error) => Some(error),
            Self::FlushResponse(error) => Some(error),
        }
    }
}

pub fn run<R: Read, W: Write>(reader: R, writer: W) -> Result<RunStatus, RunError> {
    let mut generate_id = IntentUnitId::generate;
    run_with_id_generator(reader, writer, &mut generate_id)
}

fn run_with_id_generator<R, W, F>(
    reader: R,
    mut writer: W,
    generate_id: &mut F,
) -> Result<RunStatus, RunError>
where
    R: Read,
    W: Write,
    F: FnMut() -> IntentUnitId,
{
    let mut input = Vec::new();
    let mut bounded_reader = reader.take((MAX_REQUEST_BYTES + 1) as u64);
    bounded_reader
        .read_to_end(&mut input)
        .map_err(RunError::Read)?;

    let (response, status) = if input.len() > MAX_REQUEST_BYTES {
        (
            ProtocolResponse::setup_error(ErrorDetail::request_too_large()),
            RunStatus::RequestRejected,
        )
    } else {
        match decode_request(&input) {
            Err(error) => (
                ProtocolResponse::setup_error(error),
                RunStatus::RequestRejected,
            ),
            Ok(request) => match simulate(request, generate_id) {
                Err(error) => (
                    ProtocolResponse::setup_error(error),
                    RunStatus::RequestRejected,
                ),
                Ok(SimulationOutcome::Success(response)) => (response, RunStatus::Success),
                Ok(SimulationOutcome::OperationRejected(response)) => {
                    (response, RunStatus::OperationRejected)
                }
            },
        }
    };
    write_response(&mut writer, &response, status)
}

fn write_response<W: Write>(
    writer: &mut W,
    response: &ProtocolResponse,
    status: RunStatus,
) -> Result<RunStatus, RunError> {
    serde_json::to_writer(&mut *writer, response).map_err(RunError::WriteResponse)?;
    writer.write_all(b"\n").map_err(RunError::WriteNewline)?;
    writer.flush().map_err(RunError::FlushResponse)?;
    Ok(status)
}

#[cfg(test)]
mod tests {
    use std::{
        error::Error as _,
        fs,
        io::{self, Cursor, Read, Write},
        path::{Path, PathBuf},
        process::Command,
    };

    use cubikan_core::IntentUnitId;
    use serde::Deserialize;
    use serde_json::Value;

    use super::*;

    const MANIFEST_PATH: &str = "tests/fixtures/protocol-v2/cubikan/manifest-v1.json";
    const IO_MANIFEST_PATH: &str = "tests/fixtures/protocol-v2/cubikan/io-v1.json";
    const SCHEMA_SHA256: &str = "309697fe6e718c78ef8802861d60a660500a985c05b5a94aaba35a28fb2cb4a3";
    const VERIFIER_STDOUT: &[u8] = b"verified cubikan protocol v2: schema=309697fe6e718c78ef8802861d60a660500a985c05b5a94aaba35a28fb2cb4a3 manifest=46eab998ec22d8c806c7f8ac347aa89efb4f69578c7a34f6ee4737fc24e97c75 cases=96 io_cases=4\n";

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Manifest {
        fixture_schema_version: u8,
        hash_algorithm: String,
        schema: Artifact,
        cases: Vec<Case>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Case {
        id: String,
        request: Artifact,
        #[serde(default)]
        context: Option<CaseContext>,
        stdout: Artifact,
        exit_code: u8,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct CaseContext {
        generated_uuid: String,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Artifact {
        path: String,
        bytes: usize,
        sha256: String,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct IoManifest {
        fixture_schema_version: u8,
        hash_algorithm: String,
        cases: Vec<IoCase>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct IoCase {
        id: String,
        request: Artifact,
        fault: IoFault,
        stdout: Artifact,
        stderr: Artifact,
        exit_code: u8,
        response_attempts: usize,
        newline_attempts: usize,
        flush_attempts: usize,
        expected_source_chain: Vec<String>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct IoFault {
        stage: String,
        after_bytes: usize,
        io_kind: String,
        source_message: String,
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

    #[test]
    fn test_stateless_v2_is_strict_origin_required_simulation() {
        let manifest = load_manifest();
        assert_eq!(manifest.cases.len(), 96);

        for case in &manifest.cases {
            let request = read_artifact(&case.request);
            let expected_stdout = read_artifact(&case.stdout);
            let fixed_id = case
                .context
                .as_ref()
                .map(|context| parse_fixed_id(&context.generated_uuid));
            let mut generated = 0_usize;
            let mut generate_id = || {
                generated += 1;
                fixed_id.unwrap_or_else(|| {
                    panic!("{} unexpectedly requested an adapter-generated ID", case.id)
                })
            };
            let mut writer = RecordingWriter::default();

            let status = run_with_id_generator(request.as_slice(), &mut writer, &mut generate_id)
                .unwrap_or_else(|error| panic!("{} failed delivery: {error}", case.id));

            assert_eq!(status, status_for_exit(case.exit_code), "{}", case.id);
            assert_eq!(writer.bytes, expected_stdout, "{}", case.id);
            assert_eq!(writer.flush_offsets, [writer.bytes.len()], "{}", case.id);
            assert_eq!(
                generated,
                usize::from(case.context.is_some()),
                "{}",
                case.id
            );
            assert_eq!(writer.bytes.last(), Some(&b'\n'), "{}", case.id);
            assert_eq!(
                writer.bytes.iter().filter(|byte| **byte == b'\n').count(),
                1,
                "{}",
                case.id
            );

            let response: Value = serde_json::from_slice(&writer.bytes)
                .unwrap_or_else(|error| panic!("{} output is not JSON: {error}", case.id));
            assert_eq!(response["protocol_version"], 2, "{}", case.id);
            assert_eq!(response["authority"], "simulation_only", "{}", case.id);
            assert!(response.get("canonical").is_none(), "{}", case.id);
        }
    }

    #[test]
    fn test_stateless_protocol_preserves_ingestion_delivery_and_no_state() {
        let oracle = load_io_manifest();
        assert_eq!(oracle.fixture_schema_version, 1);
        assert_eq!(oracle.hash_algorithm, "sha256");
        assert_eq!(oracle.cases.len(), 4);

        let request = read_artifact(&oracle.cases[0].request);
        let expected_success =
            read_path("tests/fixtures/protocol-v2/cubikan/io/response-body.jsonl");
        for invocation in 0..2 {
            let mut writer = RecordingWriter::default();
            assert_eq!(
                run(request.as_slice(), &mut writer).expect("valid request should be delivered"),
                RunStatus::Success,
                "invocation {invocation}"
            );
            assert_eq!(writer.bytes, expected_success, "invocation {invocation}");
            assert_eq!(writer.flush_offsets, [writer.bytes.len()]);
        }

        let response_body_length =
            read_path("tests/fixtures/protocol-v2/cubikan/io/response-body.json").len();
        for case in &oracle.cases {
            assert_eq!(case.fault.io_kind, "other", "{}", case.id);
            let expected_stdout = read_artifact(&case.stdout);
            let expected_stderr = read_artifact(&case.stderr);

            let mut writer = FaultWriter::for_case(&case.fault, response_body_length);
            let mut stderr = Vec::new();
            let exit = if case.fault.stage == "read" {
                let reader = FailingReader::new(
                    &read_artifact(&case.request),
                    case.fault.after_bytes,
                    &case.fault.source_message,
                );
                crate::run_process(reader, &mut writer, &mut stderr)
            } else {
                crate::run_process(
                    read_artifact(&case.request).as_slice(),
                    &mut writer,
                    &mut stderr,
                )
            };

            assert_eq!(exit, case.exit_code, "{}", case.id);
            assert_eq!(writer.bytes, expected_stdout, "{}", case.id);
            assert_eq!(stderr, expected_stderr, "{}", case.id);
            assert_eq!(
                writer.response_attempts(),
                case.response_attempts,
                "{}",
                case.id
            );
            assert_eq!(
                writer.newline_attempts, case.newline_attempts,
                "{}",
                case.id
            );
            assert_eq!(writer.flush_attempts, case.flush_attempts, "{}", case.id);

            let error = reproduce_io_error(case, response_body_length);
            assert_eq!(case.expected_source_chain.len(), 2, "{}", case.id);
            assert!(
                error
                    .to_string()
                    .starts_with(&case.expected_source_chain[0]),
                "{}: {error}",
                case.id
            );
            assert_eq!(
                error.source().map(ToString::to_string).as_deref(),
                Some(case.expected_source_chain[1].as_str()),
                "{}",
                case.id
            );
        }
    }

    #[test]
    fn test_stateless_schema_and_fixture_hashes_are_independent() {
        let verification = Command::new("bash")
            .arg(workspace_path("protocol/v2/verify-fixtures.sh"))
            .arg("--locked")
            .current_dir(workspace_path("."))
            .output()
            .expect("locked fixture verifier should launch");
        assert!(
            verification.status.success(),
            "locked fixture verifier failed: stdout={} stderr={}",
            String::from_utf8_lossy(&verification.stdout),
            String::from_utf8_lossy(&verification.stderr)
        );
        assert_eq!(verification.stdout, VERIFIER_STDOUT);
        assert!(verification.stderr.is_empty());

        let manifest_bytes = read_path(MANIFEST_PATH);
        let io_manifest_bytes = read_path(IO_MANIFEST_PATH);

        let manifest: Manifest = serde_json::from_slice(&manifest_bytes)
            .expect("locked protocol manifest should decode");
        assert_eq!(manifest.fixture_schema_version, 1);
        assert_eq!(manifest.hash_algorithm, "sha256");
        assert_eq!(manifest.cases.len(), 96);
        assert_eq!(manifest.schema.sha256, SCHEMA_SHA256);
        verify_artifact(&manifest.schema);

        let context_cases: Vec<_> = manifest
            .cases
            .iter()
            .filter(|case| case.context.is_some())
            .collect();
        assert_eq!(context_cases.len(), 1);
        assert_eq!(context_cases[0].id, "success_omitted_id_uses_manifest_uuid");
        assert_eq!(
            context_cases[0]
                .context
                .as_ref()
                .expect("context should exist")
                .generated_uuid,
            "123e4567-e89b-42d3-a456-426614174000"
        );

        for case in &manifest.cases {
            verify_artifact(&case.request);
            verify_artifact(&case.stdout);
            assert!(matches!(case.exit_code, 0 | 2 | 3), "{}", case.id);
        }

        let io_manifest: IoManifest =
            serde_json::from_slice(&io_manifest_bytes).expect("locked I/O manifest should decode");
        assert_eq!(io_manifest.fixture_schema_version, 1);
        assert_eq!(io_manifest.hash_algorithm, "sha256");
        assert_eq!(io_manifest.cases.len(), 4);
        for case in &io_manifest.cases {
            verify_artifact(&case.request);
            verify_artifact(&case.stdout);
            verify_artifact(&case.stderr);
            assert_eq!(case.exit_code, 1, "{}", case.id);
        }
    }

    fn load_manifest() -> Manifest {
        serde_json::from_slice(&read_path(MANIFEST_PATH))
            .expect("locked protocol manifest should decode")
    }

    fn load_io_manifest() -> IoManifest {
        serde_json::from_slice(&read_path(IO_MANIFEST_PATH))
            .expect("locked I/O manifest should decode")
    }

    fn status_for_exit(exit_code: u8) -> RunStatus {
        match exit_code {
            0 => RunStatus::Success,
            2 => RunStatus::RequestRejected,
            3 => RunStatus::OperationRejected,
            other => panic!("fixture uses unexpected modeled exit {other}"),
        }
    }

    fn parse_fixed_id(value: &str) -> IntentUnitId {
        value.parse().expect("fixture UUID should parse")
    }

    fn workspace_path(path: impl AsRef<Path>) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
    }

    fn read_path(path: impl AsRef<Path>) -> Vec<u8> {
        let path = workspace_path(path);
        fs::read(&path).unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
    }

    fn read_artifact(artifact: &Artifact) -> Vec<u8> {
        read_path(&artifact.path)
    }

    fn verify_artifact(artifact: &Artifact) {
        let bytes = read_artifact(artifact);
        assert_eq!(bytes.len(), artifact.bytes, "{}", artifact.path);
        assert_eq!(artifact.sha256.len(), 64, "{}", artifact.path);
        assert!(
            artifact
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
            "{}",
            artifact.path
        );
    }

    fn reproduce_io_error(case: &IoCase, response_body_length: usize) -> RunError {
        if case.fault.stage == "read" {
            return run(
                FailingReader::new(
                    &read_artifact(&case.request),
                    case.fault.after_bytes,
                    &case.fault.source_message,
                ),
                Vec::new(),
            )
            .expect_err("read fault must reject operationally");
        }

        let mut writer = FaultWriter::for_case(&case.fault, response_body_length);
        run(read_artifact(&case.request).as_slice(), &mut writer)
            .expect_err("write fault must reject operationally")
    }

    struct FailingReader {
        prefix: Cursor<Vec<u8>>,
        source_message: String,
    }

    impl FailingReader {
        fn new(request: &[u8], after_bytes: usize, source_message: &str) -> Self {
            Self {
                prefix: Cursor::new(request[..after_bytes].to_vec()),
                source_message: source_message.to_owned(),
            }
        }
    }

    impl Read for FailingReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if self.prefix.position() < self.prefix.get_ref().len() as u64 {
                self.prefix.read(buffer)
            } else {
                Err(io::Error::other(self.source_message.clone()))
            }
        }
    }

    #[derive(Clone, Copy)]
    enum WriterFault {
        Body { after_bytes: usize },
        Newline,
        Flush,
    }

    struct FaultWriter {
        bytes: Vec<u8>,
        fault: WriterFault,
        source_message: String,
        response_body_length: usize,
        response_attempted: bool,
        newline_attempts: usize,
        flush_attempts: usize,
    }

    impl FaultWriter {
        fn for_case(fault: &IoFault, response_body_length: usize) -> Self {
            let writer_fault = match fault.stage.as_str() {
                "body" => WriterFault::Body {
                    after_bytes: fault.after_bytes,
                },
                "newline" => WriterFault::Newline,
                "flush" => WriterFault::Flush,
                "read" => WriterFault::Flush,
                stage => panic!("unknown fixture I/O stage {stage}"),
            };
            Self {
                bytes: Vec::new(),
                fault: writer_fault,
                source_message: fault.source_message.clone(),
                response_body_length,
                response_attempted: false,
                newline_attempts: 0,
                flush_attempts: 0,
            }
        }

        const fn response_attempts(&self) -> usize {
            self.response_attempted as usize
        }
    }

    impl Write for FaultWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            let is_newline = bytes == b"\n" && self.bytes.len() == self.response_body_length;
            if is_newline {
                self.newline_attempts += 1;
            } else if !bytes.is_empty() {
                self.response_attempted = true;
            }

            match self.fault {
                WriterFault::Body { after_bytes } => {
                    if self.bytes.len() >= after_bytes {
                        return Err(io::Error::other(self.source_message.clone()));
                    }
                    let accepted = (after_bytes - self.bytes.len()).min(bytes.len());
                    self.bytes.extend_from_slice(&bytes[..accepted]);
                    Ok(accepted)
                }
                WriterFault::Newline if is_newline => {
                    Err(io::Error::other(self.source_message.clone()))
                }
                WriterFault::Newline | WriterFault::Flush => {
                    self.bytes.extend_from_slice(bytes);
                    Ok(bytes.len())
                }
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flush_attempts += 1;
            if matches!(self.fault, WriterFault::Flush) {
                Err(io::Error::other(self.source_message.clone()))
            } else {
                Ok(())
            }
        }
    }
}
