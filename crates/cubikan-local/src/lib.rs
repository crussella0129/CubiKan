//! Unsupported-only bridge for the retired durable JSON protocol v1.
//!
//! The requested database path is retained for process compatibility but is
//! never opened, created, read, or written by this bridge.

#![forbid(unsafe_code)]

mod execution;
mod protocol;
mod runner;

pub use execution::execute_request;
pub use protocol::{ExecutedRequest, PROTOCOL_VERSION, ResponseClass};
pub use runner::{MAX_REQUEST_BYTES, RunError, run, run_process};
