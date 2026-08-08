# Sprint 1 Integration Test Results

- **Intent:** [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Tested head:** `4a4d5bf999cd6eddcf76bf92950aeeb224a59811`
- **Command:** `cargo test --workspace --all-targets`
- **Adapter integration result:** 4 passed, 0 failed, 4 total
- **Core regression integration result:** 6 passed, 0 failed, 6 total

## Adapter pipeline confirmations

| Named test | EARS | Composed boundary and assertion | Result |
|------------|------|---------------------------------|--------|
| `test_runner_executes_configure_create_transition_complete` | T-102-E1, T-103-E1, T-104-E1, T-105-E1 | Public `run` composed strict JSON decoding, validated core construction, two transitions, completion, adapter snapshot mapping, and one response; asserted fixed ID, custom workflow ID, final phase/status, and three records. | pass |
| `test_runner_returns_request_failure_without_unit_state` | T-103-E3–E4, T-105-E2–E3 | An empty phase set crossed protocol and core validation, returned `workflow_empty_phases`, request classification, and no snapshot. | pass |
| `test_runner_preserves_prior_successes_on_lifecycle_failure` | T-104-E4, T-105-E4 | A valid first transition followed by an undeclared reverse edge returned operation 2 and the complete fixed ID/species/workflow/active `doing` snapshot with the exact first transition record and no third-operation effect. | pass |
| `test_runner_propagates_output_io_failure` | T-105-E5 | The public runner propagated a standard failing writer as `RunError::WriteResponse` rather than a protocol success. | pass |

These tests use the public library seam and real `cubikan-core`; they do not repeat private helper calls or introduce external services. Fixed JSON and UUID fixtures make the results deterministic.
