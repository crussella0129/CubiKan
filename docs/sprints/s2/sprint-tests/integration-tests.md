# Sprint 2 Integration Test Results

- **Intent:** [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md)
- **Preserved intent:** [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Tested head:** `b99ba8e3285b65d931cb06f1a7f5c961750596fb`
- **Command:** `cargo test --workspace --all-targets`
- **Adapter integration result:** 8 passed, 0 failed, 8 total
- **Core regression integration result:** 6 passed, 0 failed, 6 total

## Public boundary confirmations

| Named test | EARS | Composed boundary and assertion | Result |
|------------|------|---------------------------------|--------|
| `test_request_limit_is_one_mib` | T-201-E1 | An external integration crate imported `cubikan_cli::MAX_REQUEST_BYTES` as `usize` and asserted exactly `1_048_576`, proving public visibility. | pass |
| `test_runner_exposes_io_read_error_payload` | T-202-E4 | An external integration crate destructured public `RunError::Read`, bound its payload as `io::Error`, and asserted the propagated kind/message. | pass |
| `test_runner_accepts_exact_limit_request` | T-201-E1, T-203-E1 | Public `run` received JSON whose required final `}` was byte `MAX`; its status and complete response equaled the below-limit completed lifecycle result. | pass |
| `test_runner_rejects_one_byte_over_limit` | T-202-E3 | Public `run` received valid JSON whose required final `}` was byte `MAX + 1` and returned the exact single `request_too_large` envelope without state. | pass |
| `test_runner_executes_configure_create_transition_complete` | T-202-E1, T-203-E3 | The realized public configure/create/transition/complete pipeline retained fixed identity, workflow, completed phase/status, and three records. | pass |
| `test_runner_returns_request_failure_without_unit_state` | T-202-E1, T-203-E3 | Core topology rejection remained typed request rejection without state. | pass |
| `test_runner_preserves_prior_successes_on_lifecycle_failure` | T-202-E1, T-203-E3 | Lifecycle rejection retained operation number and the exact atomic prior-state snapshot. | pass |
| `test_runner_propagates_output_io_failure` | T-202-E4, T-203-E3 | The public seam still returned `RunError::WriteResponse` instead of a false modeled result. | pass |

The exact-limit fixtures insert JSON whitespace immediately before the required
final root `}` and assert their lengths before invocation. Tests use the real
`cubikan-core` and public `cubikan-cli` seam with no external service or
implementation-mirroring mock.
