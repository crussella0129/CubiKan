# Sprint 2 End-to-End Test Results

- **Status:** possible and executed
- **Intent:** [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md)
- **Preserved intent:** [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Tested head:** `b99ba8e3285b65d931cb06f1a7f5c961750596fb`
- **Command:** `cargo test --workspace --all-targets`
- **Result:** 4 passed, 0 failed, 4 total

## Process confirmations

| Named test | EARS | Actual-process assertion | Result |
|------------|------|--------------------------|--------|
| `test_cli_reports_oversized_request_with_exit_2` | T-203-E2 | Spawned Cargo's `CARGO_BIN_EXE_cubikan`, piped valid JSON whose required final `}` was byte `1_048_577`, closed stdin, and asserted exit 2, empty stderr, exactly one newline-terminated version 1 `request_too_large` envelope, exact message, and no state. | pass |
| `test_cli_configure_create_transition_complete` | T-203-E3 | The existing actual-process success fixture retained exit 0, empty stderr, one response line, fixed ID, completed state, and exact transition/transition/completion history. | pass |
| `test_cli_reports_malformed_request_with_exit_2` | T-203-E3 | Malformed below-limit JSON retained exit 2, empty stderr, one `invalid_json` response, and no state. | pass |
| `test_cli_reports_lifecycle_rejection_with_exit_3` | T-203-E3 | A valid transition followed by an undeclared edge retained exit 3, exact typed failure/operation number, and exact prior-state snapshot. | pass |

The tests use `std::process::Command`, the Cargo-built executable, and real
piped standard streams. Inputs and IDs are fixed; there is no timing, retry,
shared state, filesystem, network, or process-helper dependency.

## Documented command confirmation

`cargo run --quiet -p cubikan-cli --bin cubikan < crates/cubikan-cli/tests/fixtures/lifecycle-success-v1.json`
also exited 0 at the tested head and emitted the documented completed response.
