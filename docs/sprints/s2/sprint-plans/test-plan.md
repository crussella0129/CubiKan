Finalized - DO NOT EDIT

# Sprint 2 Test Plan

## Intent Traceability

| Intent | Acceptance criterion | Build task / EARS clause | Verification |
|--------|----------------------|--------------------------|--------------|
| [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) | One public constant defines the documented 1 MiB raw-byte ceiling as a source-level engineering guardrail. | T-201-E1, T-204-E1 | `test_request_limit_is_one_mib`, exact-limit runner tests, `test_cli_docs_define_raw_request_limit` |
| [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) | Buffer at most ceiling plus one; unchanged bytes at/below the ceiling reach strict decoding and overflow precedes JSON classification. | T-202-E1–E3 | `test_run_accepts_valid_json_at_exact_limit`, `test_run_rejects_oversize_before_json_classification`, `test_run_consumes_at_most_limit_plus_one` |
| [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) | Oversize emits one version 1 `request_too_large` line without state and exits 2. | T-201-E2, T-202-E3, T-203-E2 | `test_protocol_serializes_request_too_large_error`, `test_runner_rejects_one_byte_over_limit`, `test_cli_reports_oversized_request_with_exit_2` |
| [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) | Genuine I/O remains exit 1 and every realized result class remains unchanged. | T-202-E1, T-202-E4, T-203-E3 | `test_run_propagates_input_and_output_io_failures`, `test_process_shell_maps_operational_failure_to_exit_1`, existing runner/process regression suite |
| [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) | Automated tests cover below, exact, one-over, malformed-overflow, I/O, and actual-process behavior. | T-202-E2–E4, T-203-E1–E3 | unit, integration, E2E, and canonical workspace gates below |
| [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) | Documentation states counting, ceiling, error, local-only posture, retained exclusions, and stable Book authority. | T-204-E1–E4 | `test_cli_docs_define_raw_request_limit`, `test_cli_docs_preserve_nonproduction_exclusions`, `test_realized_adapter_intent_records_bounded_follow_on`, `test_book_v2_validation` |
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | The realized one-shot protocol, core delegation, typed results, and process E2E remain satisfied. | T-202-E1, T-203-E3 | existing 25 CLI unit/integration/E2E tests plus full core regression suite |

## Unit Tests

### T-201 protocol contract tests

- **Intent:** [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md)
- `test_protocol_serializes_request_too_large_error` [T-201-E2]: compare the complete version 1 error JSON and include `request_too_large` in exhaustive error-code serialization coverage.
- Stubs: none; fixed protocol values only.

### T-202 bounded runner tests

- **Intent:** [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md)
- `test_run_preserves_below_limit_result_classes` [T-202-E1]: representative success, setup rejection, and lifecycle rejection remain unchanged through bounded buffering.
- `test_run_accepts_valid_json_at_exact_limit` [T-202-E2]: remove a compact valid request's final root `}`, insert whitespace, restore `}` as byte `MAX`, assert exact length, and require ordinary success so truncation would fail decoding.
- `test_run_rejects_oversize_before_json_classification` [T-202-E3]: one-byte-over valid and malformed-prefix inputs both produce the exact typed request rejection with no state.
- `test_run_consumes_at_most_limit_plus_one` [T-202-E3]: a deterministic counting reader with additional available bytes records exactly ceiling-plus-one bytes consumed.
- `test_run_preserves_boundary_io_precedence` [T-202-E3–E4]: a reader error after exactly `MAX` bytes remains `RunError::Read(io::Error)`, while retaining byte `MAX + 1` proves overflow and prevents a later error/read.
- `test_run_propagates_input_and_output_io_failures` [T-202-E4]: retain read, response-body write, and response-newline write operational variants after the ingestion refactor, including body and newline failures while emitting the new oversize response.
- `test_process_shell_maps_operational_failure_to_exit_1` [T-202-E4]: retain exit 1 and stderr diagnostics for real runner I/O errors.
- Stubs: deterministic in-memory counting/failing `Read` and failing `Write` implementations; no core, service, filesystem, or process mock.

## Integration Tests

### Public runner boundary

- **Intents:** [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md), [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- `test_request_limit_is_one_mib` [T-201-E1]: an external integration crate imports `cubikan_cli::MAX_REQUEST_BYTES` as a `usize` and asserts exactly `1_048_576`, proving public visibility.
- `test_runner_accepts_exact_limit_request` [T-203-E1]: public `run` receives valid JSON whose required final `}` is byte `MAX` and emits the same completed response as the below-limit request.
- `test_runner_rejects_one_byte_over_limit` [T-202-E3]: public `run` receives valid JSON whose required final `}` is byte `MAX + 1` and emits one `request_too_large` response with no state.
- Existing `test_runner_executes_configure_create_transition_complete`, request failure, lifecycle partial-state, and output-error tests [T-202-E1, T-202-E4, T-203-E3]: all remain green without assertion weakening.
- Fixtures: adapter-owned request bytes with JSON whitespace inserted immediately before the required final root `}`; no external files, clocks, networks, or services.

## End-to-End Tests

- **Status:** possible
- `test_cli_reports_oversized_request_with_exit_2` [T-203-E2]: spawn `env!("CARGO_BIN_EXE_cubikan")`, pipe valid version 1 JSON whose required final `}` is byte `MAX_REQUEST_BYTES + 1`, close stdin, and require exit 2, empty stderr, exactly one newline-terminated error JSON, code `request_too_large`, and no Intent Unit snapshot.
- Existing actual-process success, malformed-input exit 2, and lifecycle-rejection exit 3 tests [T-203-E3] remain unchanged and green.
- Fixtures: construct the oversized request deterministically from the checked-in success fixture; no process helper or external service.

## Documentation and Repository Checks

- `test_cli_docs_define_raw_request_limit` [T-204-E1]: root and CLI README review finds `1 MiB`, `1_048_576`, raw-byte/whitespace counting, `request_too_large`, exit 2, compile-time/source-level configuration, and explicit nonclaims for total memory and production readiness.
- `test_cli_docs_preserve_nonproduction_exclusions` [T-204-E2]: docs retain persistence/session, networking/deployment, authorization/concurrency, UI/blockchain, timeout/rate/quota, and stable-compatibility exclusions.
- `test_no_new_runtime_dependencies_or_core_changes` [T-201-E3]: Cargo metadata/tree and Git diff confirm no dependency declaration or `cubikan-core` source change.
- `test_realized_adapter_intent_records_bounded_follow_on` [T-204-E3]: INT-0002 describes unbounded input as historical Sprint 1 state, links INT-0003, and retains its realized state and acceptance criteria.
- `test_book_v2_validation` [T-204-E4]: installed `check-book.sh` reports a valid Book with INT-0003 reachable from `SUMMARY.md` and valid Sprint 2 intent/plan links.

## Test Artifact Locations

- Protocol unit tests: `crates/cubikan-cli/src/protocol.rs` under `#[cfg(test)]`.
- Bounded reader and process-shell unit tests: `crates/cubikan-cli/src/runner.rs` and `crates/cubikan-cli/src/lib.rs` under `#[cfg(test)]`.
- Public constant and runner integration: `crates/cubikan-cli/tests/runner.rs`.
- Process E2E: `crates/cubikan-cli/tests/cli_e2e.rs` and the existing success fixture.
- Repository/documentation/Book evidence: results recorded under `docs/sprints/s2/sprint-tests/`.

## Final Quality Gates

- `cargo metadata --no-deps --format-version 1` resolves the unchanged two-crate workspace and one `cubikan` binary.
- `cargo tree -p cubikan-cli --depth 1 -e normal,build,dev` still lists exactly `cubikan-core`, `serde`, and `serde_json`.
- `cargo fmt --all -- --check` passes.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- `RUSTFLAGS="-D warnings" cargo check --workspace --all-targets` passes.
- `cargo test --workspace --all-targets` passes, including the new actual-process E2E.
- `cargo test --doc --workspace` passes.
- `check-book.sh` reports a valid Book v2 tree.
- The tested Git head matches the head recorded in the Sprint 2 Test Report.
