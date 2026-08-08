//! Stateless JSON adapter for the caller-configured CubiKan lifecycle.

#![forbid(unsafe_code)]

mod execution;
mod protocol;
mod runner;

pub use runner::{RunError, RunStatus, run};
