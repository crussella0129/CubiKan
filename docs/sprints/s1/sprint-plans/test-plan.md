Finalized - DO NOT EDIT

# Sprint 1 Test Plan

## Intent Traceability

| Intent | Acceptance criterion | Build task / EARS clause | Verification |
|--------|----------------------|--------------------------|--------------|
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | Research records why the one-shot batch JSON CLI is the smallest reversible boundary. | T-101-E1, T-107-E2 | `test_cli_workspace_member_resolves`, `test_research_records_one_shot_cli_decision` |
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | One versioned caller-defined scenario produces a versioned success or typed failure response. | T-102-E1–E2, T-103-E1–E4, T-105-E1–E4 | protocol, setup-mapping, and runner suites below |
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | A process-level E2E drives configure → create → transition → complete. | T-106-E1 | `test_cli_configure_create_transition_complete` |
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | Adapter behavior delegates lifecycle invariants to `cubikan-core`. | T-103-E1–E4, T-104-E1–E5 | constructor/error mapping and lifecycle execution suites below |
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | Persistence, networking, deployment, authorization, KPI, naming, blockchain, and UI policy remain outside scope. | T-101-E2, T-107-E2 | `test_cli_direct_dependency_boundary`, `test_cli_docs_preserve_stateless_boundary_and_exclusions` |

## Unit Tests

### T-101 workspace checks

- **Intent:** [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- `test_cli_workspace_member_resolves` [T-101-E1]: `cargo metadata --no-deps` reports `cubikan-core` and `cubikan-cli` as workspace members.
- `test_cli_direct_dependency_boundary` [T-101-E2]: metadata shows exactly `cubikan-core`, `serde`, and `serde_json` as all direct dependency declarations and no direct dev or build dependencies.
- `test_cli_all_targets_compile_with_warnings_denied` [T-101-E2]: `RUSTFLAGS="-D warnings" cargo check --workspace --all-targets` succeeds.
- Stubs: none; these are deterministic Test Phase command/metadata checks rather than recursive Cargo tests.

### T-102 protocol unit tests

- **Intent:** [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- `test_protocol_decodes_complete_v1_scenario_strictly` [T-102-E1]: a complete fixture preserves exact strings, topology, optional ID, and operation order; adding an unknown field fails decoding.
- `test_protocol_serializes_versioned_adapter_envelopes` [T-102-E2]: success and error variants contain one outcome, version 1, lowercase/tagged adapter fields, optional context only where applicable, and no embedded core aggregate layout.
- Stubs: fixed JSON values; no core or service mock.

### T-103 setup and conversion unit tests

- **Intent:** [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- `test_fixed_id_scenario_constructs_core_state` [T-103-E1]: exact custom/Unicode text and a fixed parseable UUID survive validated core construction.
- `test_omitted_id_generates_non_nil_v4` [T-103-E2]: an omitted ID yields a generated, non-nil UUID v4 without testing temporal ordering.
- `test_unsupported_version_and_scalar_failures_are_typed` [T-103-E3]: unsupported version, malformed ID, and blank workflow ID, phase, or species map to stable adapter codes with precise field context and no snapshot.
- `test_workflow_errors_map_exhaustively` [T-103-E4]: table-driven fixtures cover all eight current `WorkflowError` variants and assert their adapter-owned codes and absent state.
- Stubs: fixed UUIDs and caller-defined workflow DTO fixtures; no fake core implementation.

### T-104 lifecycle execution unit tests

- **Intent:** [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- `test_executor_returns_completed_adapter_snapshot` [T-104-E1]: two declared transitions and completion return the exact final adapter view and sequences 1–3.
- `test_executor_returns_active_snapshot_for_empty_operations` [T-104-E2]: no operations preserves the initial active phase and empty history.
- `test_executor_honors_declared_reverse_and_self_edges` [T-104-E3]: configured reverse and self transitions succeed while no forward-only adapter rule appears.
- `test_lifecycle_errors_map_exhaustively` [T-104-E4]: every `TransitionError` and `CompletionError` variant maps to its operation-specific machine code.
- `test_executor_reports_atomic_failure_with_prior_state` [T-104-E4]: a rejected second operation reports number 2, retains the first record and resulting phase, omits the failed mutation, and does not execute a later operation.
- `test_executor_reports_operation_after_completion` [T-104-E5]: a transition and a second completion attempted after completion produce their distinct already-completed codes and an unchanged completed snapshot.
- Stubs: deterministic workflow/request fixtures; the real in-memory core is used directly.

### T-105 stream runner unit tests

- **Intent:** [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- `test_run_writes_one_success_document` [T-105-E1]: an in-memory reader/writer receives exactly one parseable compact success value followed by one newline and returns success classification.
- `test_run_classifies_json_syntax_and_shape_failures` [T-105-E2]: syntax/EOF inputs produce `invalid_json`, data/unknown-field inputs produce `invalid_request`, both omit state, and each returns request-rejection classification.
- `test_run_writes_setup_rejection_without_state` [T-105-E3]: unsupported version and core setup failures emit one typed response without a snapshot and return request rejection.
- `test_run_writes_lifecycle_rejection_with_prior_state` [T-105-E4]: a core lifecycle failure emits its operation number and partial snapshot and returns lifecycle rejection.
- `test_run_propagates_input_and_output_io_failures` [T-105-E5]: deterministic failing `Read` and `Write` stubs surface operational errors rather than success or a truncated modeled response.
- Stubs: in-memory cursors plus minimal `FailingReader` and `FailingWriter` implementations.

### T-106 process-shell unit tests

- **Intent:** [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- `test_process_shell_maps_operational_failure_to_exit_1` [T-106-E4]: inject a deterministic runner I/O failure into the same shell mapping used by `main` and require exit `1`, a diagnostic on the supplied stderr writer, and no success classification.
- Stubs: the T-105 deterministic failing reader and an in-memory stderr writer.

## Integration Tests

### Adapter pipeline integration

- **Intents:** [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- `test_runner_executes_configure_create_transition_complete` [T-102-E1, T-103-E1, T-104-E1, T-105-E1]: the public stream runner composes strict decoding, core construction, ordered lifecycle execution, and adapter serialization into one completed response.
- `test_runner_returns_request_failure_without_unit_state` [T-103-E3–E4, T-105-E2–E3]: representative protocol and topology failures return typed request rejection without state.
- `test_runner_preserves_prior_successes_on_lifecycle_failure` [T-104-E4, T-105-E4]: a valid first transition followed by rejection returns only the earlier successful state and stops before the third operation.
- `test_runner_propagates_output_io_failure` [T-105-E5]: the public runner exposes a failed writer as an operational error.
- Fixtures: adapter-owned request JSON with fixed IDs and arbitrary workflow names; no external mocks, files, clocks, stores, or services.

## End-to-End Tests

- **Status:** possible
- `test_cli_configure_create_transition_complete` [T-106-E1]: spawn `env!("CARGO_BIN_EXE_cubikan")`, pipe the checked-in version 1 fixture, close stdin, and require exit `0`, empty stderr, one newline-terminated success value, the expected fixed ID, completed status, final phase, and ordered history.
- `test_cli_reports_malformed_request_with_exit_2` [T-106-E2]: spawn the binary with malformed JSON and require exit `2`, empty stderr, and one version 1 `invalid_json` response without state.
- `test_cli_reports_lifecycle_rejection_with_exit_3` [T-106-E3]: spawn the binary with a valid first transition followed by an undeclared edge and require exit `3`, empty stderr, operation number 2, active partial snapshot, and exactly one transition record.
- Fixtures: `crates/cubikan-cli/tests/fixtures/lifecycle-success-v1.json`; rejection and malformed inputs may remain inline when clearer.

## Documentation and Book Checks

- `test_cli_docs_explain_protocol_and_failure_contract` [T-107-E1]: review the root and crate READMEs for invocation, complete version 1 shapes, response/error semantics, exits 0/1/2/3, fail-fast partial state, and the lifecycle example.
- `test_cli_docs_preserve_stateless_boundary_and_exclusions` [T-107-E2]: require one-process/in-memory/experimental language, disclosure of unbounded local stdin with resource limiting deferred before production exposure, and explicit persistence, networking, deployment, authorization, KPI, naming, blockchain, and UI deferrals.
- `test_research_records_one_shot_cli_decision` [T-107-E2]: require the Sprint 1 research report and INT-0002 to record the selected CLI and rejected broader alternatives.
- `test_book_v2_validation` [T-107-E2]: `check-book.sh` succeeds with valid intent, task, sprint, and navigation links.
- Stubs: none; record deterministic review commands and paths in the Sprint 1 Test Report.

## Test Artifact Locations

- Protocol/setup/lifecycle unit tests: `crates/cubikan-cli/src/protocol.rs` and `crates/cubikan-cli/src/execution.rs` under `#[cfg(test)]`.
- Stream and process-shell unit tests: `crates/cubikan-cli/src/runner.rs` and `crates/cubikan-cli/src/lib.rs` under `#[cfg(test)]`.
- Public library integration: `crates/cubikan-cli/tests/runner.rs`.
- Process E2E: `crates/cubikan-cli/tests/cli_e2e.rs` and `crates/cubikan-cli/tests/fixtures/lifecycle-success-v1.json`.
- Repository, documentation, and Book checks: commands and results recorded in `docs/sprints/s1/sprint-tests/`.

## Final Quality Gates

- `cargo metadata --no-deps` must resolve both workspace packages and the `cubikan` binary target.
- `cargo fmt --all -- --check` must pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` must pass.
- `RUSTFLAGS="-D warnings" cargo check --workspace --all-targets` must pass.
- `cargo test --workspace --all-targets` must pass, including the actual-process E2E.
- `cargo test --doc --workspace` must pass.
- `check-book.sh` must report a valid Book v2 tree.
- The tested Git head must match the head recorded in the Sprint 1 Test Report.
