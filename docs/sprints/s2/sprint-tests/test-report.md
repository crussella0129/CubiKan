# Sprint 2 Test Report

## Intent Verification

| Intent | Acceptance criterion | EARS / tests | Result | Intent evidence update |
|--------|----------------------|---------------|--------|------------------------|
| [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) | One public constant defines the documented 1 MiB raw request ceiling as a source-level engineering guardrail. | T-201-E1, T-204-E1 / `test_request_limit_is_one_mib`, `test_runner_accepts_exact_limit_request`, `test_cli_docs_define_raw_request_limit` | pass | Public visibility, the exact `usize` value, source-level configuration, and documentation are verified. |
| [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) | Retain at most ceiling plus one; pass complete at-or-below input unchanged to strict decoding; reject overflow before JSON classification. | T-202-E1–E3 / `test_run_accepts_valid_json_at_exact_limit`, `test_run_rejects_oversize_before_json_classification`, `test_run_consumes_at_most_limit_plus_one` | pass | Required boundary syntax is retained at the ceiling; valid and malformed overflow share size-first rejection; the reader consumes exactly the ceiling plus one when more data is available. |
| [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) | Oversize emits one newline-terminated version 1 `request_too_large` response without state and exits 2. | T-201-E2, T-202-E3, T-203-E2 / protocol, public-runner, and `test_cli_reports_oversized_request_with_exit_2` assertions | pass | The complete adapter envelope and actual process status/stdout/stderr behavior are verified. |
| [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) | Genuine I/O remains exit 1; all realized result classes and response semantics remain unchanged. | T-202-E1, T-202-E4, T-203-E3 / `test_runner_exposes_io_read_error_payload`, I/O propagation/process-shell tests, and existing runner/process regressions | pass | The public payload is compile-time bound as `io::Error`; input/body/newline failures remain operational and success/setup/lifecycle results retain their classifications. |
| [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) | Automated coverage includes below, exact, one-over, malformed-overflow, deterministic I/O, and actual-process overflow behavior. | T-202-E2–E4, T-203-E1–E3 / unit, integration, E2E, and canonical workspace suites | pass | Every requested boundary and negative path ran with deterministic fixtures at the recorded head. |
| [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) | Consumer documentation explains counting, ceiling, error, and all retained local/non-production exclusions. | T-204-E1–E4 / documentation reviews, INT-0002 follow-on review, and `check-book.sh` | pass | Both consumer guides and Book authority state the exact contract, nonclaims, and exclusions. |
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | The realized one-shot protocol, core delegation, typed outcomes, and actual-process lifecycle remain satisfied. | T-202-E1, T-203-E3 / existing CLI and full core regression suites | pass | Bounded ingestion preserves the realized adapter semantics without dependency or core-source drift. |

## Summary

- Unit tests: 24 passed / 0 failed / 24 total
- Integration tests: 8 passed / 0 failed / 8 total
- E2E tests: 4 passed / 0 failed / 4 total
- Workspace regression: 85 passed / 0 failed / 85 total across all all-target binaries
- Doctests: 1 passed / 0 failed / 1 total
- CI status: not-configured

## CI Confirmation

- **Head SHA:** `b99ba8e3285b65d931cb06f1a7f5c961750596fb`
- **CI run:** CI not configured — local confirmations only
- **Conclusion:** success
- **Confirmations:** [unit and repository checks](unit-tests.md), [integration tests](integration-tests.md), and [actual-process E2E tests](e2e-tests.md)

The committed head passed Cargo metadata and direct-dependency inspection,
`cargo fmt --all -- --check`, Clippy with warnings denied, warnings-denied
all-target checking, all-target tests, workspace doctests, Book v2 validation,
the no-core/no-dependency-drift check, and the documented CLI fixture command.

## Failures

None.

## Technical Debt Identified

- No follow-up intent was opened. The fixed source-level ceiling bounds retained
  raw request bytes only; total allocation, timeouts, rate limits, concurrent
  quotas, and production network exposure remain explicitly outside INT-0003.
- Persistence, sessions, service/API, UI, and blockchain outcomes still require
  separate product intents rather than being inferred from this hardening sprint.

## Coverage Observations

- Exact-limit fixtures make the final root `}` the boundary byte, so a silent
  truncation cannot pass as disposable-whitespace loss.
- Size classification is verified before JSON classification for both valid and
  malformed overflow, with ceiling-plus-one consumption and deterministic
  read-error precedence at the probe boundary.
- The Test Critic's initial weak-assertion concern was closed by an external
  integration test that binds `RunError::Read` to `io::Error`; the final critic
  verdict is clean.
- Actual-process E2E covers bounded success preservation, malformed input,
  lifecycle rejection, and oversized rejection with exact exit/status streams.
