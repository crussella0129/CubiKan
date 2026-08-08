# Sprint 3 Test Report

## Intent Verification

| Intent | Acceptance criterion | EARS / tests | Result | Intent evidence update |
|--------|----------------------|---------------|--------|------------------------|
| [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) | Serialize one response, write its terminating newline, call the supplied writer's flush exactly once, then return the existing modeled status. | T-301-E1, T-301-E4 / `test_run_flushes_each_modeled_response_once_after_newline` and existing complete-response tests | pass | Five separately classified response paths record their sole flush after the newline and return the unchanged status only after it succeeds. |
| [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) | Public `FlushResponse(io::Error)` preserves error kind, message, source, and deterministic diagnostic. | T-301-E2, T-302-E1 / `test_run_preserves_flush_error_payload_display_and_source`, `test_runner_surfaces_buffered_sink_failure_on_explicit_flush` | pass | Both the unit seam and external integration crate destructure the public variant and assert the complete error contract. |
| [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) | Body, newline, and flush stages have deterministic first-error precedence. | T-301-E3 / `test_run_preserves_response_output_error_precedence` | pass | Body failure suppresses newline/flush, newline failure suppresses flush, and flush-only failure returns no modeled status. |
| [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) | Success, malformed JSON, setup rejection, lifecycle rejection, and oversize each flush once; realistic buffering exposes the prior gap. | T-301-E1, T-302-E1 / five-case flush table and real-`BufWriter` integration | pass | Every response class is covered, and a 4096-byte `BufWriter` proves the drain failure appears only when explicit flush is called. |
| [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) | The process shell maps flush failure to exit 1 while ordinary actual-process behavior remains unchanged. | T-302-E2, T-302-E3 / two process-shell tests and four Cargo-built E2Es | pass | Injected flush failure preserves exit 1 and best-effort diagnostics; actual processes retain exact exits, complete JSON, newline, and stderr behavior. |
| [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) | Documentation states the writer-flush check without promising atomic, durable, acknowledged, or stable cross-version delivery. | T-303-E1, T-303-E2 / `test_cli_docs_define_explicit_response_flush`, `test_cli_docs_preserve_flush_boundary_nonclaims` | pass | Both guides state the exact sequence and operational failure behavior and retain every required nonclaim. |
| [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) | Genuine output failure remains operational exit 1 and oversized input remains a typed exit-2 rejection. | T-301-E1, T-301-E4, T-302-E3 / oversized unit, integration, and actual-process cases plus flush-failure process tests | pass | The new output stage does not alter bounded-ingestion classification or its complete response. |
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | One-response and actual-process lifecycle semantics remain satisfied. | T-301-E4, T-302-E3 / public runner regression and four complete-JSON E2Es | pass | Success, request rejection, and lifecycle rejection retain the realized adapter contract. |
| [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) | Core/domain and dependency boundaries remain unchanged. | T-303-E3 / path-scoped Git diff, direct-dependency inspection, and full core regression | pass | No core, manifest, lockfile, or protocol DTO/error-code source changed; all 49 core tests and the core doctest pass. |

## Summary

- CLI unit tests: 29 passed / 0 failed / 29 total
- CLI integration tests: 9 passed / 0 failed / 9 total
- CLI E2E tests: 4 passed / 0 failed / 4 total
- Focused CLI all-target suite: 42 passed / 0 failed / 42 total
- Workspace regression: 91 passed / 0 failed / 91 total across all all-target binaries
- Doctests: 1 passed / 0 failed / 1 total
- CI status: not-configured

## CI Confirmation

- **Head SHA:** `f6883cccfdb0008b1c6a0b3d37ac27bced00c3e8`
- **CI run:** CI not configured — local committed-head confirmations only
- **Conclusion:** success
- **Confirmations:** [unit and repository checks](unit-tests.md), [integration tests](integration-tests.md), and [actual-process E2E tests](e2e-tests.md)

The committed head passed Cargo metadata and direct-dependency inspection,
formatting, workspace Clippy with warnings denied, warnings-denied all-target
checking, focused and workspace all-target tests, workspace doctests, Book v2
validation, the path-scoped no-core/no-dependency/no-protocol-drift check, and
the documented CLI lifecycle fixture.

## Failures

None.

## Technical Debt Identified

- No follow-up intent was opened. Supplied-writer flush success does not prove
  OS/kernel delivery, close success, external-reader acknowledgement, stream
  atomicity/rollback, `fsync` durability, persistence, retries, or network
  semantics; each remains outside INT-0004.
- `RunError::FlushResponse` intentionally changes exhaustive matching for the
  unpublished experimental Rust API. Stable compatibility remains a separate
  future decision rather than an implicit Sprint 3 promise.

## Coverage Observations

- The first Test Critic run found that the repository-drift evidence abbreviated
  a path-scoped command as unscoped. The artifact now records the exact passing
  command, and the follow-up critic verdict is clean.
- The five modeled response classes are separately exercised rather than folded
  into a generic success/rejection claim.
- The real `BufWriter` test reproduces the motivating gap: body and newline fit
  in memory, and only explicit flush reaches the failing sink.
- Four actual-process E2Es prove ordinary standard-stream regression behavior.
  The flush-only negative path remains deterministic at the public `run` and
  injected `run_process` seams instead of relying on platform-specific failures.
