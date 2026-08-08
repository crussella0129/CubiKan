//! Stateless JSON adapter for the caller-configured CubiKan lifecycle.

#![forbid(unsafe_code)]

use std::io::{Read, Write};

mod execution;
mod protocol;
mod runner;

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

    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("fixture operational failure"))
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
    }
}
