Finalized - DO NOT EDIT

# Sprint 4 Test Plan

## Intent Traceability

| Intent | Acceptance criterion | Build task / EARS clause | Verification |
|--------|----------------------|--------------------------|--------------|
| [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) | PRs targeting `dev`/`main` and pushes to `dev`/`main` dispatch one quality job. | T-401-E1 | `test_ci_workflow_matches_event_and_job_scope`, `test_ci_workflow_is_registered_on_github`, hosted `dev` push run |
| [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) | Read-only permission, immutable checkout without credentials, PR-number/full-ref concurrency cancellation, Ubuntu, and finite timeout. | T-401-E1–E3 | `test_ci_workflow_is_read_only_and_bounded`, `test_ci_workflow_pins_checkout_and_drops_credentials` |
| [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) | Current stable minimal Rust plus rustfmt/clippy executes five separate fail-fast quality steps. | T-401-E4, T-402-E1–E2 | `test_ci_workflow_provisions_current_stable_components`, `test_ci_workflow_runs_five_canonical_gates_in_order`, local five-gate execution, hosted job-step evidence |
| [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) | Actual hosted run at a committed Sprint 4 `dev` head succeeds and records exact provenance. | T-402-E3 | `test_hosted_dev_push_quality_run_succeeds` |
| [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) | Documentation defines triggers, commands, Ubuntu/current-stable scope, and explicit exclusions. | T-403-E1–E2 | `test_readme_documents_ci_contract_and_nonclaims` |
| [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) | Sprint 4 intent and evidence remain valid and reachable in the Project Book. | T-403-E3 | `test_book_v2_validation` |
| [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) | No dependency, crate source, protocol, domain, or process-exit drift. | T-402-E4 | `test_ci_scope_has_no_product_or_dependency_drift`, full workspace regression |
| [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md), [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md), [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md), [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) | Realized core/adapter/request/flush behavior remains satisfied. | T-402-E4 | 91 existing all-target tests, core doctest, exact actual-process regressions |

## Unit and Repository Contract Checks

### T-401 workflow shell checks

- **Intent:** [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md)
- `test_ci_workflow_matches_event_and_job_scope` [T-401-E1]: inspect the committed YAML and GitHub-registered workflow; assert exact PR bases `{dev, main}`, push branches `{dev, main}`, exactly one `quality` job, `ubuntu-latest`, and timeout `15`.
- `test_ci_workflow_is_read_only_and_bounded` [T-401-E1–E3]: assert top-level permission equals only `contents: read`, concurrency is workflow plus `github.event.pull_request.number || github.ref` with cancellation, distinct PR numbers and full pushed refs cannot collide, and no `pull_request_target`, write permission, secret, cache, artifact, release, deployment, service, or second job exists.
- `test_ci_workflow_pins_checkout_and_drops_credentials` [T-401-E2]: assert the sole action reference is exactly `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1`, the v7.0.1 comment is present, and `persist-credentials` is false.
- `test_ci_workflow_provisions_current_stable_components` [T-401-E4]: assert the setup command installs `stable` with profile `minimal`, components `rustfmt,clippy`, and every Cargo quality command uses `+stable`; assert no version/MSRV pin or third-party setup action.

### T-402 quality and scope checks

- **Intent:** [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md), preserving [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md), [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md), [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md), and [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md)
- `test_ci_workflow_runs_five_canonical_gates_in_order` [T-402-E1–E2]: extract the five quality `run` commands and compare their order and exact flags/environment with the locked list; assert five separately named steps and absence of `continue-on-error` or retry wrappers.
- `test_ci_scope_has_no_product_or_dependency_drift` [T-402-E4]: `git diff --quiet main...HEAD -- Cargo.toml Cargo.lock crates` passes, Cargo metadata/dependency tree remain unchanged, and all existing local gates remain green.

### T-403 documentation and Book checks

- **Intent:** [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md)
- `test_readme_documents_ci_contract_and_nonclaims` [T-403-E1–E2]: README links the workflow, names both event branch sets, lists all five local commands in order, scopes coverage to current-stable GitHub-hosted Ubuntu, preserves human merge approval, and contains every explicit nonclaim.
- `test_book_v2_validation` [T-403-E3]: installed `check-book.sh` reports a valid Book with INT-0005 and Sprint 4 reachable from `SUMMARY.md`.
- These named repository checks are one-off Test Phase inspections recorded under `docs/sprints/s4/sprint-tests/`; no parser/runtime dependency or implementation-mirroring test harness is added to the product workspace.

## Integration Tests

### GitHub workflow registration

- **Intent:** [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md)
- `test_ci_workflow_is_registered_on_github` [T-401-E1, T-402-E3]: after the committed Build head is pushed to `dev`, GitHub's workflow API recognizes `.github/workflows/ci.yml` as active with name `Rust CI`, and the returned YAML/path is the committed definition.
- `test_hosted_quality_job_exposes_all_steps` [T-401-E4, T-402-E1–E3]: the resulting run exposes one `Rust quality gate` job; checkout, toolchain, and all five quality steps complete successfully at the exact Build SHA.
- Stubs/mocks: none. GitHub's workflow and run APIs are the external integration boundary; local contract checks inspect committed repository text rather than reproducing the Actions evaluator.

## End-to-End Tests

- **Status:** possible and required.
- `test_hosted_dev_push_quality_run_succeeds` [T-402-E3]: push the committed Sprint 4 Build head to `dev`; assert the GitHub run event is `push`, head branch is `dev`, head SHA equals the recorded Build head, workflow/job conclusions are `success`, and record the run ID and URL.
- The final `dev → main` draft PR will trigger a second candidate run at the remote checkpoint and will be verified before handoff, but it is not the realization oracle because committing that result would recursively create another head. No PR is opened before Loop closes.
- Failure handling: if Actions is disabled, unavailable, or the hosted run cannot succeed, write failure evidence and leave INT-0005 unrealized; local YAML/Cargo success must not be mislabeled as hosted CI success.

## Test Artifact Locations

- Workflow/repository/local-gate confirmations: `docs/sprints/s4/sprint-tests/unit-tests.md`.
- GitHub workflow registration and job evidence: `docs/sprints/s4/sprint-tests/integration-tests.md`.
- Hosted push-run evidence and remote-checkpoint PR-run rationale: `docs/sprints/s4/sprint-tests/e2e-tests.md`.
- Reviewed intent verification and exact CI provenance: `docs/sprints/s4/sprint-tests/test-report.md`.

## Final Quality Gates

- `cargo metadata --no-deps --format-version 1` resolves the unchanged workspace.
- `cargo tree --workspace --edges normal` confirms no dependency drift.
- `cargo fmt --all -- --check` passes.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- `RUSTFLAGS="-D warnings" cargo check --workspace --all-targets` passes.
- `cargo test --workspace --all-targets` passes all existing unit/integration/E2E tests.
- `cargo test --doc --workspace` passes the core doctest.
- The hosted `dev` push workflow/job succeeds at the exact committed Build head.
- Installed `check-book.sh` reports a valid Book v2 tree.
- `git diff --check` and the exact scoped no-runtime/dependency-drift check pass.
