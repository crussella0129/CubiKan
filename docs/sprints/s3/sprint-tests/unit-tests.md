# Sprint 3 Unit and Repository Verification

- **Primary intent:** [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md)
- **Preserved intents:** [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md), [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md), and [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md)
- **Tested head:** `f6883cccfdb0008b1c6a0b3d37ac27bced00c3e8`
- **Primary command:** `cargo test --workspace --all-targets`
- **CLI unit result:** 29 passed, 0 failed, 29 total
- **Workspace regression result:** 91 passed, 0 failed, 91 total across all all-target test binaries

## Locked EARS confirmations

| EARS | Named verification | Executed assertion | Result |
|------|--------------------|--------------------|--------|
| T-301-E1, T-301-E4 | `test_run_flushes_each_modeled_response_once_after_newline` | Separate success, malformed-JSON, unsupported-version setup, lifecycle-rejection, and oversized-request cases each produced the expected existing status/code and one parseable response line; the supplied writer recorded exactly one flush at the final byte offset after the newline. | pass |
| T-301-E2 | `test_run_preserves_flush_error_payload_display_and_source` | A fixed `BrokenPipe` flush error returned public `FlushResponse`, preserved kind/message/source, rendered exactly `failed to flush response: fixture flush failure`, and returned no modeled status. | pass |
| T-301-E3 | `test_run_preserves_response_output_error_precedence` | Body failure returned `WriteResponse` without newline or flush; newline failure returned `WriteNewline` without flush; flush-only failure accepted the complete response line, attempted one flush, and returned `FlushResponse`. | pass |
| T-302-E2 | `test_process_shell_maps_flush_failure_to_exit_1` | A stdout writer accepted a complete JSON line and failed on its sole flush; `run_process` returned exit 1 and wrote exactly `cubikan: failed to flush response: fixture flush failure\n` to working stderr. | pass |
| T-302-E2 | `test_process_shell_keeps_exit_1_when_flush_diagnostic_fails` | The same stdout flush failure retained exit 1 when best-effort stderr writing also failed. | pass |
| T-301-E4, T-302-E3 | `test_process_shell_maps_operational_failure_to_exit_1`, full CLI regression | Existing input/body/newline operational failures, request-size precedence, modeled statuses, and core-delegated lifecycle behavior remained green when ordinary writers flushed successfully. | pass |
| T-303-E1 | `test_cli_docs_define_explicit_response_flush` | Root and CLI guides state JSON serialization → newline → one supplied-writer flush → modeled outcome, with flush failure as exit 1 and a best-effort stderr diagnostic. | pass |
| T-303-E2 | `test_cli_docs_preserve_flush_boundary_nonclaims` | The guides deny stream atomicity/rollback, durable `fsync`, OS/kernel delivery, close success, persistence, retries, external-reader receipt, network acknowledgement, and cross-version compatibility. | pass |
| T-303-E3 | `test_no_dependency_core_or_protocol_scope_change` | Cargo metadata/tree retained only `cubikan-core`, `serde`, and `serde_json` as CLI direct dependencies; `git diff --quiet main...HEAD -- Cargo.toml Cargo.lock crates/cubikan-core crates/cubikan-cli/Cargo.toml crates/cubikan-cli/src/protocol.rs` passed, confirming no manifest, lockfile, `cubikan-core`, or CLI protocol DTO/error-code source change. Existing exit meanings and realized intent semantics remained green in regression and documentation review. | pass |
| T-303-E4 | `test_book_v2_validation` | Installed `check-book.sh` reported `valid v2 Book (4 intent chapters)` with INT-0004 and Sprint 3 reachable from `SUMMARY.md`. | pass |

## Quality confirmations

- `cargo metadata --no-deps --format-version 1` — pass; unchanged two-crate workspace and `cubikan` binary resolved
- `cargo tree -p cubikan-cli --depth 1 -e normal,build,dev` — pass; exactly `cubikan-core`, `serde`, and `serde_json`
- `cargo fmt --all -- --check` — pass
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — pass
- `RUSTFLAGS="-D warnings" cargo check --workspace --all-targets` — pass
- `cargo test -p cubikan-cli --all-targets` — pass; 42 tests, 0 failures
- `cargo test --workspace --all-targets` — pass; 91 tests, 0 failures
- `cargo test --doc --workspace` — pass; 1 core doctest, 0 failures; CLI has no doctests
- `git diff --check` — pass
- T-301–T-303 task hashes resolve, are ancestors of the tested head, and have matching `sprint-3: T-30N` subjects — pass
- `.github/workflows/` is absent and `.github/` contains only `dependabot.yml`; CI is not configured, so these committed-head local confirmations are authoritative.

## Test fixtures and stubs

The recording and stage-failing writers implement only the standard `Write`
contract. They expose accepted bytes, newline attempts, flush offsets, and fixed
I/O errors without constructing protocol responses, statuses, or core behavior.
The process-shell fixtures similarly inject only stdout/stderr behavior. No core,
filesystem, network, clock, retry, shared-state, or actual-process mock is used.
