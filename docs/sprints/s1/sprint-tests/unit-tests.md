# Sprint 1 Unit and Repository Verification

- **Intent:** [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Tested head:** `4a4d5bf999cd6eddcf76bf92950aeeb224a59811`
- **Primary command:** `cargo test --workspace --all-targets`
- **Sprint 1 unit result:** 18 passed, 0 failed, 18 total
- **Workspace regression result:** 74 passed, 0 failed, 74 total across all all-target test binaries

## Locked EARS confirmations

| EARS | Named verification | Executed assertion | Result |
|------|--------------------|--------------------|--------|
| T-101-E1 | `test_cli_workspace_member_resolves` | `cargo metadata --no-deps --format-version 1` resolved both `cubikan-core` and `cubikan-cli`; the `cubikan` binary target was present. | pass |
| T-101-E2 | `test_cli_direct_dependency_boundary` | `cargo tree -p cubikan-cli --depth 1 -e normal,build,dev` listed exactly `cubikan-core`, `serde`, and `serde_json`, with no direct dev/build dependency. | pass |
| T-101-E2 | `test_cli_all_targets_compile_with_warnings_denied` | `RUSTFLAGS="-D warnings" cargo check --workspace --all-targets` completed without warnings. | pass |
| T-102-E1 | `test_protocol_decodes_complete_v1_scenario_strictly` | Preserved every caller field, exact directed edges, and ordered operations; rejected unknown fields at the root, workflow, edge, Intent Unit, transition, and completion DTO boundaries. | pass |
| T-102-E2 | `test_protocol_serializes_versioned_adapter_envelopes` | Compared the complete success, setup-error, and lifecycle-error JSON objects, covering every snapshot/history/error field and every omitted optional arm. | pass |
| T-103-E1 | `test_fixed_id_scenario_constructs_core_state` | Public core constructors preserved a fixed non-v4 UUID value, whitespace-bearing workflow ID, Unicode phase, and Unicode species. | pass |
| T-103-E2 | `test_omitted_id_generates_non_nil_v4` | Omitted ID produced a non-nil UUID with version number 4. | pass |
| T-103-E3 | `test_unsupported_version_and_scalar_failures_are_typed` | Unsupported version, malformed ID, and every blank scalar location returned its locked code/field with no operation number or state-producing path. | pass |
| T-103-E4 | `test_workflow_errors_map_exhaustively` | Eight constructor fixtures exercised and mapped all current `WorkflowError` variants to the exact locked codes. | pass |
| T-104-E1 | `test_executor_returns_completed_adapter_snapshot` | Two transitions plus completion returned stable identity/species/workflow, final phase/status, and exact sequences 1–3. | pass |
| T-104-E2 | `test_executor_returns_active_snapshot_for_empty_operations` | Empty operations returned the initial active phase with empty history. | pass |
| T-104-E3 | `test_executor_honors_declared_reverse_and_self_edges` | Explicit forward, self, and reverse edges all executed with exact ordered history. | pass |
| T-104-E4 | `test_lifecycle_errors_map_exhaustively` | All three transition and two completion variants mapped to distinct locked codes with supplied operation numbers. | pass |
| T-104-E4 | `test_executor_reports_atomic_failure_with_prior_state` | Transition and completion failures at operation 2 retained the exact fixed ID/species/workflow/active state and operation-1 history, omitted the rejected mutation, and skipped operation 3. | pass |
| T-104-E5 | `test_executor_reports_operation_after_completion` | Transition and duplicate completion after terminal completion returned distinct already-completed codes and snapshots equal to the terminal baseline. | pass |
| T-105-E1 | `test_run_writes_one_success_document` | The stream runner returned success and wrote one compact parseable document plus exactly one newline. | pass |
| T-105-E2 | `test_run_classifies_json_syntax_and_shape_failures` | Syntax/EOF mapped to `invalid_json`; wrong shape/unknown field mapped to `invalid_request`; all omitted state. | pass |
| T-105-E3 | `test_run_writes_setup_rejection_without_state` | Unsupported version emitted one typed setup rejection and request-rejected classification without state. | pass |
| T-105-E4 | `test_run_writes_lifecycle_rejection_with_prior_state` | Lifecycle rejection emitted operation 2, the complete exact operation-1 snapshot/history, and lifecycle-rejected classification. | pass |
| T-105-E5 | `test_run_propagates_input_and_output_io_failures` | Deterministic stubs separately exercised input failure, response-body write failure, and trailing-newline write failure as operational errors. | pass |
| T-106-E4 | `test_process_shell_maps_operational_failure_to_exit_1` | The same shell mapping called by `main` mapped both input and trailing-newline output failures to exit 1 with newline-terminated stderr diagnostics. | pass |
| T-107-E1 | `test_cli_docs_explain_protocol_and_failure_contract` | Root/crate README review confirmed invocation, complete v1 shapes, exits 0/1/2/3, all error codes, fail-fast partial state, and lifecycle example. | pass |
| T-107-E2 | `test_cli_docs_preserve_stateless_boundary_and_exclusions` | Docs state one-process/in-memory/experimental behavior, unbounded local stdin and hardening deferral, plus every planned platform/policy exclusion. | pass |
| T-107-E2 | `test_research_records_one_shot_cli_decision` | Research report and INT-0002 select the one-shot CLI and retain broader alternatives/deferrals. | pass |
| T-107-E2 | `test_book_v2_validation` | Installed `check-book.sh` reported `valid v2 Book (2 intent chapters)`. | pass |

## Quality confirmations

- `cargo fmt --all -- --check` — pass
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — pass
- `RUSTFLAGS="-D warnings" cargo check --workspace --all-targets` — pass
- `cargo test --doc --workspace` — pass (1 core doctest, 0 failures; CLI has no doctests)
- All seven Sprint 1 task hashes resolve, are ancestors of the tested head, and have matching `sprint-1: T-10N` subjects — pass
- `.github/workflows/` is absent; CI is not configured, so these are authoritative local confirmations.

## Stubs

Only deterministic in-memory `FailingReader`, `FailingWriter`, and `NewlineFailingWriter` implementations replace operating-system I/O for T-105-E5/T-106-E4. They implement the standard `Read`/`Write` failure contract and do not mirror domain behavior. No core, clock, store, network, or process mock is used.
