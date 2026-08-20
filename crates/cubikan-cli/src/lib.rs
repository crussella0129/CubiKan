//! Strict one-shot adapter for CubiKan's simulation-only stateless protocol v2.

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
        Ok(RunStatus::OperationRejected) => 3,
        Err(error) => {
            let _ = writeln!(stderr, "cubikan: {error}");
            1
        }
    }
}
