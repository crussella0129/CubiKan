# Sprint 3 Integration Test Results

- **Primary intent:** [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md)
- **Preserved intents:** [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) and [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md)
- **Tested head:** `f6883cccfdb0008b1c6a0b3d37ac27bced00c3e8`
- **Command:** `cargo test -p cubikan-cli --all-targets`
- **Adapter integration result:** 9 passed, 0 failed, 9 total

## Public boundary confirmations

| Named test | EARS | Composed boundary and assertion | Result |
|------------|------|---------------------------------|--------|
| `test_runner_surfaces_buffered_sink_failure_on_explicit_flush` | T-302-E1 | Public `run` wrote a small complete response into a real 4096-byte `std::io::BufWriter`; the underlying sink rejected its first drain write only when explicit flush ran. The external crate matched public `FlushResponse`, verified `BrokenPipe` kind/message/display/source, one drain attempt, zero inner-sink flush calls after that failed drain, retained newline-terminated JSON, and controlled disassembly without drop retry. | pass |
| `test_runner_executes_configure_create_transition_complete` | T-301-E4, T-302-E3 | The public runner retained fixed identity, workflow, completed phase/status, and exact transition/transition/completion history. | pass |
| `test_runner_returns_request_failure_without_unit_state` | T-301-E4, T-302-E3 | Setup rejection remained typed and contained no Intent Unit state. | pass |
| `test_runner_preserves_prior_successes_on_lifecycle_failure` | T-301-E4, T-302-E3 | Lifecycle rejection retained its operation number and exact atomic prior-state snapshot. | pass |
| `test_runner_propagates_output_io_failure` | T-301-E3, T-301-E4 | Existing response-body output failure remained `WriteResponse` rather than becoming a modeled result. | pass |
| `test_runner_exposes_io_read_error_payload` | T-301-E4 | Existing input I/O error kind/message propagation remained unchanged. | pass |
| `test_request_limit_is_one_mib` | T-301-E4 | The public compile-time request ceiling remained exactly `1_048_576` bytes. | pass |
| `test_runner_accepts_exact_limit_request` | T-301-E4 | A valid request whose required final `}` is byte `MAX` retained its complete successful lifecycle result. | pass |
| `test_runner_rejects_one_byte_over_limit` | T-301-E1, T-301-E4 | A request whose required final `}` is byte `MAX + 1` retained the exact one-line `request_too_large` rejection and no state. | pass |

The new failure proof composes the public crate boundary with Rust's real
`BufWriter`; only the drain-rejecting sink is deterministic test infrastructure.
It does not mirror JSON, flushing policy, status mapping, or domain behavior.
The remaining cases compose the real `cubikan-core` and public `cubikan-cli`
seams with fixed in-memory requests and no external services.
