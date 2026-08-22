//! Strict chain-backed local protocol-v2 process adapter.
//!
//! The crate intentionally exposes only its bounded process surface. Request
//! decoding, RPC access, projection access, signing, submission, and durable
//! acknowledgement remain closed implementation details.

#![forbid(unsafe_code)]

mod execution;
mod protocol;
mod runner;

pub use runner::{MAX_REQUEST_BYTES, run_process};
