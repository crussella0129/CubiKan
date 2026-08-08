# Sprint 2 Unit and Repository Verification

- **Intent:** [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md)
- **Preserved intent:** [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Tested head:** `b99ba8e3285b65d931cb06f1a7f5c961750596fb`
- **Primary command:** `cargo test --workspace --all-targets`
- **CLI unit result:** 24 passed, 0 failed, 24 total
- **Workspace regression result:** 85 passed, 0 failed, 85 total across all all-target test binaries

## Locked EARS confirmations

| EARS | Named verification | Executed assertion | Result |
|------|--------------------|--------------------|--------|
| T-201-E2 | `test_protocol_serializes_request_too_large_error` | Compared the complete version 1 envelope, exact locked message/code, and absence of field, operation number, and Intent Unit state; exhaustive code serialization also includes `request_too_large`. | pass |
| T-201-E3 | `test_no_new_runtime_dependencies_or_core_changes` | Cargo metadata/tree retained exactly `cubikan-core`, `serde`, and `serde_json`; `git diff --quiet main...HEAD -- Cargo.toml Cargo.lock crates/cubikan-core` confirmed no dependency or core-source change. | pass |
| T-202-E1 | `test_run_preserves_below_limit_result_classes` | Representative success, unsupported-version setup rejection, and disallowed-transition lifecycle rejection retained their status and code after bounded buffering. | pass |
| T-202-E2 | `test_run_accepts_valid_json_at_exact_limit` | Required final root `}` was byte `1_048_576`; successful decoding proves the semantic boundary byte was retained rather than disposable padding being truncated. | pass |
| T-202-E3 | `test_run_rejects_oversize_before_json_classification` | Valid JSON with final `}` at byte `1_048_577` and a malformed input of the same size both returned the identical exact oversize response before parsing. | pass |
| T-202-E3 | `test_run_consumes_at_most_limit_plus_one` | A counting reader with additional available data recorded exactly `1_048_577` bytes consumed. | pass |
| T-202-E3–E4 | `test_run_preserves_boundary_io_precedence` | An error after exactly `MAX` bytes surfaced as `RunError::Read(io::Error)`; byte `MAX + 1` conclusively selected oversize without observing the later reader error. | pass |
| T-202-E4 | `test_runner_exposes_io_read_error_payload`, `test_run_propagates_input_and_output_io_failures` | An external integration crate binds the public `RunError::Read` payload as `io::Error` and checks its kind/message; immediate read, JSON-body write, and trailing-newline failures remained operational on ordinary and oversized-result paths. | pass |
| T-202-E4 | `test_process_shell_maps_operational_failure_to_exit_1` | The production process shell retained exit 1 and newline-terminated stderr diagnostics for read and incomplete-output failures. | pass |
| T-203-E3 | `test_workspace_regression_suite` | All 85 CLI/core all-target tests plus the core doctest remained green, including realized success/setup/lifecycle/process behavior. | pass |
| T-204-E1 | `test_cli_docs_define_raw_request_limit` | Root and CLI guides state 1 MiB / `1_048_576`, raw whitespace counting, `request_too_large`, exit 2, source-level configuration, and explicit nonclaims for total memory and production readiness. | pass |
| T-204-E2 | `test_cli_docs_preserve_nonproduction_exclusions` | Both guides preserve persistence/session, networking/deployment, authorization/concurrency, UI/blockchain, timeout/rate/quota, and cross-version compatibility boundaries. | pass |
| T-204-E3 | `test_realized_adapter_intent_records_bounded_follow_on` | INT-0002 records unbounded input as historical Sprint 1 state, links INT-0003, and remains realized with unchanged acceptance criteria. | pass |
| T-204-E4 | `test_book_v2_validation` | Installed `check-book.sh` reported `valid v2 Book (3 intent chapters)` with INT-0003 and Sprint 2 reachable in navigation. | pass |

## Quality confirmations

- `cargo metadata --no-deps --format-version 1` — pass; unchanged two-crate workspace and `cubikan` binary resolved
- `cargo tree -p cubikan-cli --depth 1 -e normal,build,dev` — pass; exactly `cubikan-core`, `serde`, and `serde_json`
- `cargo fmt --all -- --check` — pass
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — pass
- `RUSTFLAGS="-D warnings" cargo check --workspace --all-targets` — pass
- `cargo test --doc --workspace` — pass (1 core doctest, 0 failures; CLI has no doctests)
- T-201–T-204 task hashes resolve, are ancestors of the tested head, and have matching `sprint-2: T-20N` subjects — pass
- `.github/workflows/` is absent; CI is not configured, so these are authoritative local confirmations.

## Stubs

Deterministic in-memory `CountingReader`, `ErrorAfterReader`, `FailingReader`,
`FailingWriter`, and `NewlineFailingWriter` values implement only standard
`Read`/`Write` contracts. They expose byte counts and I/O errors without
mirroring JSON, limit, response, core, or process behavior. No core, service,
filesystem, clock, network, or actual-process mock is used.
