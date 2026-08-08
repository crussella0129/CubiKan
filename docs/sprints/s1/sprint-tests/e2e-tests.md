# Sprint 1 End-to-End Test Results

- **Status:** possible and executed
- **Intent:** [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Tested head:** `4a4d5bf999cd6eddcf76bf92950aeeb224a59811`
- **Command:** `cargo test --workspace --all-targets`
- **Result:** 3 passed, 0 failed, 3 total

## Process confirmations

| Named test | EARS | Actual-process assertion | Result |
|------------|------|--------------------------|--------|
| `test_cli_configure_create_transition_complete` | T-106-E1 | Spawned Cargo's `CARGO_BIN_EXE_cubikan`, piped the checked-in v1 fixture, closed stdin, and asserted exit 0, empty stderr, exactly one newline-terminated success JSON, fixed ID, `done`/`completed`, and the exact transition 1, transition 2, completion 3 history. | pass |
| `test_cli_reports_malformed_request_with_exit_2` | T-106-E2 | Spawned the executable with malformed JSON and asserted exit 2, empty stderr, one v1 `invalid_json` response, and no snapshot. | pass |
| `test_cli_reports_lifecycle_rejection_with_exit_3` | T-106-E3 | Spawned the executable with one valid transition then an undeclared reverse edge and asserted exit 3, empty stderr, `transition_not_allowed`, operation 2, and the complete fixed ID/species/workflow/active `doing` snapshot with its exact first transition record. | pass |

The tests use `std::process::Command` and real piped standard streams with no process helper, service, persistence, clock, or network dependency. The fixed UUID and in-repository fixture remove randomness from externally asserted state.

## Documented command confirmation

`cargo run --quiet -p cubikan-cli --bin cubikan < crates/cubikan-cli/tests/fixtures/lifecycle-success-v1.json` also exited 0 at the tested head and emitted the documented completed response. This is supporting confirmation; the three named Cargo-built process tests are the automated E2E oracle.
