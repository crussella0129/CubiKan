Finalized - DO NOT EDIT

# Sprint 3 Test Plan

## Intent Traceability

| Intent | Acceptance criterion | Build task / EARS clause | Verification |
|--------|----------------------|--------------------------|--------------|
| [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) | Serialize one envelope, write its newline, call supplied `flush` once, then return the existing modeled status. | T-301-E1, T-301-E4 | `test_run_flushes_each_modeled_response_once_after_newline`, existing complete-envelope runner tests |
| [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) | Public `FlushResponse(io::Error)` preserves kind, message, source, and deterministic diagnostic. | T-301-E2, T-302-E1 | `test_run_preserves_flush_error_payload_display_and_source`, `test_runner_surfaces_buffered_sink_failure_on_explicit_flush` |
| [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) | Body → newline → flush has deterministic first-error precedence. | T-301-E3 | `test_run_preserves_response_output_error_precedence` |
| [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) | Success, malformed request, setup rejection, lifecycle rejection, and oversize each flush once; real buffering exposes the original gap. | T-301-E1, T-302-E1 | five-case flush table; `test_runner_surfaces_buffered_sink_failure_on_explicit_flush` |
| [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) | Process shell maps flush failure to exit `1`; ordinary actual-process behavior remains unchanged. | T-302-E2, T-302-E3 | `test_process_shell_maps_flush_failure_to_exit_1`, `test_process_shell_keeps_exit_1_when_flush_diagnostic_fails`, four existing process E2Es |
| [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) | Documentation describes the writer-flush check and avoids stronger delivery/durability claims. | T-303-E1, T-303-E2 | `test_cli_docs_define_explicit_response_flush`, `test_cli_docs_preserve_flush_boundary_nonclaims` |
| [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) | Genuine output failure and oversize handling remain exit `1`/`2` as applicable. | T-301-E1, T-301-E4, T-302-E3 | oversized flush-table case, existing oversize runner/process tests, full regression suite |
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | One-response and actual-process lifecycle semantics remain satisfied. | T-301-E4, T-302-E3 | existing runner integration and four process E2Es |
| [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) | Core/domain and dependency boundaries remain unchanged. | T-303-E3 | `test_no_dependency_core_or_protocol_scope_change`, full core regression suite |

## Unit Tests

### T-301 runner sequencing and error tests

- **Intent:** [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md)
- `test_run_flushes_each_modeled_response_once_after_newline` [T-301-E1, T-301-E4]: a recording writer stores accepted bytes and flush offsets; table cases for success, malformed JSON, unsupported-version setup, lifecycle rejection, and one-byte-over input assert the expected status/code, one parseable response line, `flush_offsets == [bytes.len()]`, and newline at the recorded flush boundary.
- `test_run_preserves_flush_error_payload_display_and_source` [T-301-E2]: a fixed `BrokenPipe` flush failure asserts the exact public variant, `io::Error` kind/message, `RunError::to_string() == "failed to flush response: fixture flush failure"`, and a downcast `Error::source()` with the same values.
- `test_run_preserves_response_output_error_precedence` [T-301-E3]: a stage-configurable writer proves body failure returns `WriteResponse` with zero newline/flush attempts; newline failure returns `WriteNewline` with one newline and zero flush attempts; flush-only failure accepts the complete response line, attempts one flush, returns `FlushResponse`, and returns no modeled status.
- Stubs: deterministic `Write` implementations observe the public output contract only. They record accepted bytes, newline attempts, flush offsets, and fixed I/O failures without generating JSON, statuses, or core behavior.

### T-302 process-shell unit tests

- **Intent:** [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md)
- `test_process_shell_maps_flush_failure_to_exit_1` [T-302-E2]: a stdout writer accepts the complete JSON line and fails only on its sole flush; assert exit `1`, one flush attempt, accepted newline-terminated JSON, and the exact best-effort diagnostic on working stderr.
- `test_process_shell_keeps_exit_1_when_flush_diagnostic_fails` [T-302-E2]: the same stdout flush failure plus a failing stderr writer still returns exactly exit `1`.
- Existing `test_process_shell_maps_operational_failure_to_exit_1` remains green [T-301-E4, T-302-E3].

## Integration Tests

### Public buffered-writer boundary

- **Intents:** [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md), preserving [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) and [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md)
- `test_runner_surfaces_buffered_sink_failure_on_explicit_flush` [T-302-E1]: public `run` writes a small success response into a real `BufWriter` whose capacity is larger than the response and whose sink rejects its first drain write; assert public `FlushResponse` kind/message/display/source, exactly one underlying drain attempt caused by explicit flush, zero inner-sink flush calls after the failed drain, and retained buffered bytes ending in newline before disassembling without a drop retry.
- Existing eight runner integration tests [T-301-E4, T-302-E3] retain public request-limit, identity/lifecycle, response, and body-output-error behavior.
- Fixtures: fixed in-memory requests, real `std::io::BufWriter`, and a minimal drain-failing sink; no domain, protocol, filesystem, network, clock, or process mock.

## End-to-End Tests

- **Status:** possible and executed for ordinary standard streams; a portable flush-only OS failure is not isolatable at the current executable boundary.
- `test_cli_configure_create_transition_complete` [T-302-E3] is strengthened to compare the complete success response JSON and retains exit `0`, empty stderr, and one newline.
- `test_cli_reports_malformed_request_with_exit_2` [T-302-E3] is strengthened to compare the complete malformed-request response JSON and retains exit `2`, empty stderr, and one newline.
- Existing `test_cli_reports_oversized_request_with_exit_2` [T-302-E3] retains its complete typed-response equality, exit `2`, empty stderr, and one newline.
- `test_cli_reports_lifecycle_rejection_with_exit_3` [T-302-E3] is strengthened to compare the complete failure response JSON and retains exit `3`, empty stderr, and one newline.
- Negative flush-only proof is provided at the public `run` + real `BufWriter` integration seam and injected `run_process` unit seam. Closed pipes or platform devices cannot deterministically distinguish body, newline, explicit-flush, and teardown failures, so no test-only hook or platform-specific pseudo-E2E will be introduced.

## Documentation and Repository Checks

- `test_cli_docs_define_explicit_response_flush` [T-303-E1]: root and CLI READMEs state JSON → newline → one supplied-writer flush → modeled status, with flush error as operational exit `1` and best-effort stderr.
- `test_cli_docs_preserve_flush_boundary_nonclaims` [T-303-E2]: guides explicitly deny atomicity/rollback, durable `fsync`, OS delivery/close success, persistence, retries, external/network acknowledgement, and stable cross-version compatibility.
- `test_no_dependency_core_or_protocol_scope_change` [T-303-E3]: Cargo metadata/tree and Git diff confirm no dependency, `cubikan-core`, protocol DTO/error-code/JSON-shape, or exit-meaning change.
- `test_book_v2_validation` [T-303-E4]: installed `check-book.sh` reports a valid Book with INT-0004 and Sprint 3 reachable from `SUMMARY.md`.

## Test Artifact Locations

- Runner unit tests: `crates/cubikan-cli/src/runner.rs` under `#[cfg(test)]`.
- Process-shell unit tests: `crates/cubikan-cli/src/lib.rs` under `#[cfg(test)]`.
- Public buffered-writer integration: `crates/cubikan-cli/tests/runner.rs`.
- Actual-process regression: `crates/cubikan-cli/tests/cli_e2e.rs` and existing fixed fixture.
- Documentation/repository/Book evidence: results recorded under `docs/sprints/s3/sprint-tests/`.

## Final Quality Gates

- `cargo metadata --no-deps --format-version 1` resolves the unchanged two-crate workspace and `cubikan` binary.
- `cargo tree -p cubikan-cli --depth 1 -e normal,build,dev` still lists exactly `cubikan-core`, `serde`, and `serde_json`.
- `cargo fmt --all -- --check` passes.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- `RUSTFLAGS="-D warnings" cargo check --workspace --all-targets` passes.
- `cargo test -p cubikan-cli --all-targets` passes focused adapter coverage.
- `cargo test --workspace --all-targets` passes the full regression suite.
- `cargo test --doc --workspace` passes.
- Installed `check-book.sh` reports a valid Book v2 tree.
- `git diff --check` passes, and the tested committed head matches the head recorded in the Sprint 3 Test Report.
