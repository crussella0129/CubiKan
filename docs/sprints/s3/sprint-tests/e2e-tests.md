# Sprint 3 End-to-End Test Results

- **Status:** possible and executed for ordinary standard streams
- **Primary intent:** [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md)
- **Preserved intents:** [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) and [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md)
- **Tested head:** `f6883cccfdb0008b1c6a0b3d37ac27bced00c3e8`
- **Command:** `cargo test -p cubikan-cli --all-targets`
- **Result:** 4 passed, 0 failed, 4 total

## Actual-process confirmations

| Named test | EARS | Cargo-built process assertion | Result |
|------------|------|-------------------------------|--------|
| `test_cli_configure_create_transition_complete` | T-302-E3 | The checked-in lifecycle request exited 0, wrote no stderr, wrote exactly one newline, and produced the complete expected version 1 success JSON with fixed ID, final phase, completed status, and exact three-record history. | pass |
| `test_cli_reports_malformed_request_with_exit_2` | T-302-E3 | Malformed JSON exited 2, wrote no stderr, wrote exactly one newline, and produced the complete expected `invalid_json` envelope without state. | pass |
| `test_cli_reports_oversized_request_with_exit_2` | T-301-E1, T-302-E3 | A valid request whose required final `}` is byte `1_048_577` exited 2, wrote no stderr, wrote exactly one newline, and produced the complete expected `request_too_large` envelope without state. | pass |
| `test_cli_reports_lifecycle_rejection_with_exit_3` | T-302-E3 | An undeclared second transition exited 3, wrote no stderr, wrote exactly one newline, and produced the complete expected typed error plus prior-state snapshot. | pass |

These tests spawn Cargo's `CARGO_BIN_EXE_cubikan` and use real piped standard
streams. Inputs and IDs are fixed; there is no timing, retry, shared state,
filesystem, network, or process-helper dependency.

## Flush-failure boundary

An actual child-process flush-only failure is not portable or isolatable at the
current executable seam: a closed pipe or platform device can fail during body,
newline, flush, or teardown. The negative acceptance outcome is therefore proved
deterministically at public `run` with a real `BufWriter` and at the injectable
`run_process` shell. Introducing a test-only process hook or platform-specific
sink would expand rather than strengthen the accepted boundary; no unlocking
intent is required while that outcome remains covered at those observable seams.

## Documented command confirmation

`cargo run --quiet -p cubikan-cli --bin cubikan < crates/cubikan-cli/tests/fixtures/lifecycle-success-v1.json`
also exited 0 at the tested head and emitted the documented completed response
with the fixed ID, `done` phase, completed status, and three ordered records.
