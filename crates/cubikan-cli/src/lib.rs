//! Stateless JSON adapter for the caller-configured CubiKan lifecycle.

#![forbid(unsafe_code)]

use std::io::{Read, Write};

mod execution;
mod protocol;
mod runner;

/// Maximum number of raw bytes accepted for one standard-input request.
pub const MAX_REQUEST_BYTES: usize = 1_048_576;

pub use runner::{RunError, RunStatus, run};

pub fn run_process<R: Read, W: Write, E: Write>(reader: R, writer: W, mut stderr: E) -> u8 {
    match run(reader, writer) {
        Ok(RunStatus::Success) => 0,
        Ok(RunStatus::RequestRejected) => 2,
        Ok(RunStatus::LifecycleRejected) => 3,
        Err(error) => {
            let _ = writeln!(stderr, "cubikan: {error}");
            1
        }
    }
}

#[cfg(test)]
mod process_tests {
    use std::io;

    use super::*;

    const VALID_REQUEST: &[u8] = br#"{
        "protocol_version": 1,
        "workflow": {
            "id": "delivery",
            "phases": ["queued"],
            "initial_phase": "queued",
            "edges": [],
            "completion_phases": []
        },
        "intent_unit": {"id": null, "species": "feature"},
        "operations": []
    }"#;

    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("fixture operational failure"))
        }
    }

    struct NewlineFailingWriter;

    impl Write for NewlineFailingWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            if buffer == b"\n" {
                Err(io::Error::other("fixture newline failure"))
            } else {
                Ok(buffer.len())
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct FlushOnlyFailingWriter {
        bytes: Vec<u8>,
        flush_attempts: usize,
    }

    impl Write for FlushOnlyFailingWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            self.bytes.extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flush_attempts += 1;
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "fixture flush failure",
            ))
        }
    }

    #[derive(Default)]
    struct FailingDiagnosticWriter {
        write_attempts: usize,
    }

    impl Write for FailingDiagnosticWriter {
        fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
            self.write_attempts += 1;
            Err(io::Error::other("fixture diagnostic failure"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("fixture diagnostic failure"))
        }
    }

    #[test]
    fn test_process_shell_maps_operational_failure_to_exit_1() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run_process(FailingReader, &mut stdout, &mut stderr);

        assert_eq!(exit, 1);
        assert!(stdout.is_empty());
        let diagnostic = String::from_utf8(stderr).expect("diagnostic should be UTF-8");
        assert!(diagnostic.contains("failed to read request"));
        assert!(diagnostic.ends_with('\n'));

        let mut stderr = Vec::new();
        let exit = run_process(VALID_REQUEST, NewlineFailingWriter, &mut stderr);

        assert_eq!(exit, 1);
        let diagnostic = String::from_utf8(stderr).expect("diagnostic should be UTF-8");
        assert!(diagnostic.contains("failed to finish response line"));
        assert!(diagnostic.ends_with('\n'));
    }

    #[test]
    fn test_process_shell_maps_flush_failure_to_exit_1() {
        let mut stdout = FlushOnlyFailingWriter::default();
        let mut stderr = Vec::new();

        let exit = run_process(VALID_REQUEST, &mut stdout, &mut stderr);

        assert_eq!(exit, 1);
        assert_eq!(stdout.flush_attempts, 1);
        assert_eq!(stdout.bytes.last(), Some(&b'\n'));
        let response: serde_json::Value = serde_json::from_slice(&stdout.bytes)
            .expect("accepted stdout bytes should form a complete response line");
        assert_eq!(response["outcome"], "success");
        assert_eq!(
            stderr,
            b"cubikan: failed to flush response: fixture flush failure\n"
        );
    }

    #[test]
    fn test_process_shell_keeps_exit_1_when_flush_diagnostic_fails() {
        let mut stdout = FlushOnlyFailingWriter::default();
        let mut stderr = FailingDiagnosticWriter::default();

        let exit = run_process(VALID_REQUEST, &mut stdout, &mut stderr);

        assert_eq!(exit, 1);
        assert_eq!(stdout.flush_attempts, 1);
        assert_eq!(stdout.bytes.last(), Some(&b'\n'));
        assert!(stderr.write_attempts > 0);
    }
}
