Finalized - DO NOT EDIT

# Sprint 5 Test Plan

## Intent Traceability

| Intent | Acceptance criterion | Build task / EARS clause | Verification |
|--------|----------------------|--------------------------|--------------|
| [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md) | The decoder distinguishes absence from a present string and preserves each representation. | T-501-E1 | `test_protocol_distinguishes_absent_string_and_null_id` |
| [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md) | Explicit `null`, Boolean, number, array, and object ID values are structural failures with no constructed state. | T-501-E2, T-502-E2 | `test_protocol_distinguishes_absent_string_and_null_id`, `test_run_rejects_present_non_string_ids_without_creating_state` |
| [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md) | Omission generates non-nil UUID v4; valid fixed and malformed string semantics remain distinct and stable. | T-502-E1, T-502-E3, T-503-E1 | `test_run_generates_id_when_member_is_omitted`, `test_run_preserves_id_string_validation_taxonomy`, `test_cli_generates_id_when_member_is_omitted`, `test_fixed_id_scenario_constructs_core_state`, `test_omitted_id_generates_non_nil_v4`, `test_unsupported_version_and_scalar_failures_are_typed` |
| [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md) | Public explicit-null rejection is one newline-terminated, writer-flush-checked version 1 response with stable shape and status. | T-502-E2 | `test_run_rejects_present_non_string_ids_without_creating_state`, `test_run_flushes_each_modeled_response_once_after_newline`, `test_run_preserves_response_output_error_precedence` |
| [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md) | The actual process reports null with exit `2` and omission with generated-ID success. | T-503-E1–E2 | `test_cli_generates_id_when_member_is_omitted`, `test_cli_reports_explicit_null_id_with_exit_2` |
| [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md) | Documentation defines the optional-member/string/null and structural-versus-semantic failure rules. | T-504-E1–E2 | `test_cli_guide_documents_id_presence_contract` |
| [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md) | No dependency, core, version, response, error-code, request-limit, flush, or unrelated product-policy drift occurs. | T-501-E3–E4, T-502-E4, T-503-E3, T-504-E3 | `test_protocol_preserves_required_and_unknown_field_strictness`, process-shell exit regressions, `test_run_preserves_oversize_before_explicit_null_classification`, actual-process regressions, `test_sprint_scope_has_no_dependency_core_or_output_contract_drift`, full workspace and hosted regression |
| [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md), [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md), [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md), [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md), [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) | Realized core, adapter, bounded-input, output-flush, and automated quality outcomes remain satisfied. | T-501-E4, T-502-E2–E4, T-503-E3, T-504-E3 | existing core/CLI unit, integration, process, doctest, and hosted quality regressions; `test_hosted_sprint_five_quality_run_succeeds` |

## Unit and Repository Contract Checks

### T-501 protocol decoder tests

- **Intent:** [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md), preserving [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- `test_protocol_distinguishes_absent_string_and_null_id` [T-501-E1–E2]: starting from one otherwise valid `serde_json::Value`, remove `intent_unit.id` and assert decoded `None`; set a fixed UUID string and assert exact `Some`; then table-test `null`, Boolean, number, array, and object values and assert each decode fails with Serde category `Data`.
- `test_protocol_preserves_required_and_unknown_field_strictness` [T-501-E3]: assert absent `intent_unit.id` alone decodes, while missing `intent_unit.species` and unknown members at the existing root/workflow/edge/intent/action boundaries remain rejected. Existing `test_protocol_decodes_complete_v1_scenario_strictly` remains a regression oracle for preserved values and tagged operations.
- Stubs/mocks: none. Tests decode adapter-owned JSON values directly through the private DTO used in production.

### T-501 process-shell unit regressions

- **Intent:** [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md), preserving [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) and [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md)
- `test_process_shell_fixture_uses_true_omission` [T-501-E4]: decode `VALID_REQUEST` through the production request DTO and assert `intent_unit.id == None`, proving that the fixture contains no `id` member rather than JSON `null`.
- `test_process_shell_maps_operational_failure_to_exit_1`, `test_process_shell_maps_flush_failure_to_exit_1`, and `test_process_shell_keeps_exit_1_when_flush_diagnostic_fails` [T-501-E4]: run the existing read/newline, flush, and diagnostic-write cases with the corrected fixture and preserve their exact exit and diagnostic expectations.

### T-504 documentation, scope, and Book checks

- **Intent:** [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md), preserving [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md), [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md), [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md), [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md), and [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md)
- `test_cli_guide_documents_id_presence_contract` [T-504-E1–E2]: inspect the CLI guide and assert it names `intent_unit.id` as the only optional request member, defines omission, requires a present string, rejects explicit `null`/non-string values as `invalid_request`, preserves malformed-string `invalid_intent_unit_id` plus field path, and retains experimental/one-shot nonclaims without pinning diagnostic prose.
- `test_sprint_scope_has_no_dependency_core_or_output_contract_drift` [T-504-E3]: compare accepted base to the committed Build head; assert no `Cargo.toml`, `Cargo.lock`, `.github/`, `crates/cubikan-core/`, CLI execution/lifecycle logic, protocol version, response DTO, error-code, request-ceiling, or output-precedence change. The only runtime behavior diff must be ID field decoding, with test-only process fixture and evidence/docs alongside it.
- `test_book_v2_validation` [T-504-E3]: installed `check-book.sh` reports a valid Book with INT-0006 and Sprint 5 reachable from `SUMMARY.md`, and all Markdown file links resolve.
- These named documentation/repository checks are one-off Test Phase inspections recorded under `docs/sprints/s5/sprint-tests/`; no implementation-mirroring test harness or new dependency is added.

## Integration Tests

### T-502 public runner identity contract

- **Intents:** [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md), preserving [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md), [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md), and [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md)
- `test_run_generates_id_when_member_is_omitted` [T-502-E1]: remove the ID member from serialized request bytes, call public `run`, assert `Success`, exactly one response line, and parse the snapshot ID through the already-dependent `cubikan_core::IntentUnitId` API as non-nil UUID version 4.
- `test_run_rejects_present_non_string_ids_without_creating_state` [T-502-E2]: table-test `null`, Boolean, number, array, and object request values through public `run` and a test-owned recording writer; for every case assert request-rejection status, one newline, exactly one flush after the newline, protocol version `1`, `invalid_request`, nonempty message, and absence of state, field, and operation number.
- `test_run_preserves_id_string_validation_taxonomy` [T-502-E3]: table-test a valid fixed string and malformed UUID string; assert the fixed ID is exact, while malformed text yields request rejection, `invalid_intent_unit_id`, field `intent_unit.id`, no state, and a nonempty message.
- `test_run_preserves_oversize_before_explicit_null_classification` [T-502-E4]: construct an explicit-null request whose required final root brace is byte `MAX_REQUEST_BYTES + 1`; assert exactly the existing `request_too_large` response. Existing `test_run_consumes_at_most_limit_plus_one` remains the independent consumption-bound oracle.
- Stubs/mocks: no domain or service mocks. The recording writer and deterministic bounded reader are test doubles for the public `Read`/`Write` seams and assert externally observable I/O contracts.

### Existing component regressions

- **Intents:** [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md), [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md), [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md), [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md)
- `test_fixed_id_scenario_constructs_core_state`, `test_omitted_id_generates_non_nil_v4`, and `test_unsupported_version_and_scalar_failures_are_typed` [T-502-E1, T-502-E3, T-504-E3] preserve fixed, omitted, and malformed-string execution semantics beneath the wire boundary.
- `test_runner_accepts_exact_limit_request`, `test_runner_rejects_one_byte_over_limit`, and `test_run_consumes_at_most_limit_plus_one` [T-502-E4, T-504-E3] preserve the realized bounded-ingestion contract.
- `test_run_flushes_each_modeled_response_once_after_newline`, `test_run_preserves_response_output_error_precedence`, and `test_runner_surfaces_buffered_sink_failure_on_explicit_flush` [T-502-E2, T-504-E3] preserve the realized supplied-writer flush and first-error contract.
- All core lifecycle/serialization tests and remaining CLI unit, public-runner, and actual-process tests run unchanged [T-503-E3, T-504-E3]. Representative domain regressions include configure/create/transition/complete, atomic lifecycle failure, semantic restore, and the core doctest.

### Hosted quality integration

- **Intent:** [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md), preserving all other reviewed intents
- After the final Build commit is pushed to `dev`, query GitHub's existing `Rust CI` workflow and sole `Rust quality gate` job [T-504-E3]. Assert the remote branch and checked-out revision equal the exact committed Build SHA and each of the five configured quality steps completes successfully. Static/local checks are not substitutes for hosted registration and execution evidence.

## End-to-End Tests

- **Status:** possible and required.
- `test_cli_generates_id_when_member_is_omitted` [T-503-E1]: remove the ID member from the checked-in valid scenario, invoke `env!("CARGO_BIN_EXE_cubikan")`, close stdin, and assert exit `0`, empty stderr, exactly one newline-terminated success document, the expected completed lifecycle, and an ID parsed through `cubikan_core::IntentUnitId` as non-nil UUID v4.
- `test_cli_reports_explicit_null_id_with_exit_2` [T-503-E2]: set the member to JSON `null`, invoke the Cargo-built process, and assert exit `2`, empty stderr, exactly one response line, protocol version `1`, code `invalid_request`, nonempty message, and absence of `intent_unit`, `field`, and `operation_number`.
- Existing `test_cli_configure_create_transition_complete`, `test_cli_reports_malformed_request_with_exit_2`, `test_cli_reports_lifecycle_rejection_with_exit_3`, and `test_cli_reports_oversized_request_with_exit_2` [T-503-E3] retain exact fixed-ID response, stderr, and process-exit expectations.
- `test_hosted_sprint_five_quality_run_succeeds` [T-504-E3]: push the exact committed Sprint 5 Build head to `dev`; assert the GitHub run event is `push`, branch is `dev`, head SHA matches, and workflow/job conclusions plus all five quality steps are `success`. Record the run and job URLs. The later draft `dev → main` PR run is a remote-checkpoint confirmation, not the realization oracle.
- External/flake boundary: actual-process cases are local deterministic child processes. Hosted evidence is one GitHub attempt bounded by the existing 15-minute timeout and depends on GitHub, floating `ubuntu-latest`, current stable Rust, Rustup, crates.io, and the pinned checkout action; observed versions are provenance rather than support promises. If hosted execution is unavailable or fails, do not realize INT-0006 from local evidence alone.

## Test Artifact Locations

- Protocol, process-shell, documentation, scope, local quality, and Book confirmations: `docs/sprints/s5/sprint-tests/unit-tests.md`.
- Public runner and hosted job integration evidence: `docs/sprints/s5/sprint-tests/integration-tests.md`.
- Actual-process and hosted push-run evidence: `docs/sprints/s5/sprint-tests/e2e-tests.md`.
- Reviewed intent verification and exact committed-head provenance: `docs/sprints/s5/sprint-tests/test-report.md`.

## Final Quality Gates

- `cargo metadata --no-deps --format-version 1` resolves the unchanged workspace.
- `cargo tree --workspace --edges normal` confirms no dependency drift.
- `cargo fmt --all -- --check` passes.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- `RUSTFLAGS="-D warnings" cargo check --workspace --all-targets` passes.
- `cargo test --workspace --all-targets` passes every protocol, public-runner, process-shell, actual-process, core, and regression test.
- `cargo test --doc --workspace` passes the core doctest.
- The existing hosted `dev` push workflow and sole quality job succeed at the exact committed Build head.
- Installed `check-book.sh` reports a valid Book v2 tree with six reachable intents, and all Markdown file links resolve.
- `git diff --check` and the exact scoped no-dependency/core/output-contract-drift review pass.
