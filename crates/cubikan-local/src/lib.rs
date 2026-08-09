//! Versioned one-request local adapter for the durable CubiKan backend.
//!
//! This library owns protocol v1 decoding, semantic validation, execution, and
//! response encoding. Process argument, input-size, output, and exit handling
//! remain a separate runner boundary.

#![forbid(unsafe_code)]

mod execution;
mod protocol;

pub use execution::execute_request;
pub use protocol::{ExecutedRequest, PROTOCOL_VERSION, ResponseClass};
