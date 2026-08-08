# Sprint 4 Test Report

## Intent Verification

| Intent | Acceptance criterion | EARS / tests | Result | Intent evidence update |
|--------|----------------------|---------------|--------|------------------------|
| [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) | PRs targeting `dev`/`main` and pushes to `dev`/`main` configure one quality job. | T-401-E1 / `test_ci_workflow_matches_event_and_job_scope`, `test_ci_workflow_is_registered_on_github`, hosted `dev` push | pass | Exact event filters and sole job are asserted statically; GitHub registered the workflow and dynamically exercised `push` on `dev`. |
| [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) | Explicit permission is only `contents: read`; checkout is immutable and credential persistence is disabled; superseded PR/ref runs are bounded and the sole Ubuntu job has a timeout. | T-401-E1–E3 / `test_ci_workflow_is_read_only_and_bounded`, `test_ci_workflow_pins_checkout_and_drops_credentials` | pass | YAML assertions cover exact permission/concurrency/job shape and noncollision; hosted logs confirm pinned checkout and disabled credential persistence. |
| [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) | Current stable minimal Rust plus rustfmt/clippy executes five separate fail-fast gates. | T-401-E4, T-402-E1–E2 / `test_ci_workflow_provisions_current_stable_components`, `test_ci_workflow_runs_five_canonical_gates_in_order`, `test_hosted_quality_job_exposes_all_steps` | pass | Exact command/order/environment checks and the real job show toolchain setup plus formatting, Clippy, warnings-denied check, 91 tests, and one doctest all succeeding. |
| [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) | A real GitHub Actions run succeeds at an exact committed Sprint 4 `dev` head with recorded provenance. | T-402-E3 / `test_hosted_dev_push_quality_run_succeeds` | pass | Push run [31281268756](https://github.com/crussella0129/CubiKan/actions/runs/31281268756), attempt 1, and sole job [93162907989](https://github.com/crussella0129/CubiKan/actions/runs/31281268756/job/93162907989) completed `success` at `85aaa99e6cbe375129475feb445319f2fd94beda`. |
| [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) | Contributor documentation names automation, triggers, commands, hosted scope, and all nonclaims. | T-403-E1–E2 / `test_readme_documents_ci_contract_and_nonclaims` | pass | README links the workflow, reproduces all five gates, preserves human approval, and denies branch-protection, release, deployment, cache, artifact, scanner, secret, auto-merge, MSRV, and matrix claims. |
| [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) | No crate dependency, Rust source, runtime protocol, domain behavior, or process-exit meaning changes. | T-402-E4 / `test_ci_scope_has_no_product_or_dependency_drift`, full regression | pass | Scoped Git diff, metadata/tree inspection, 91 tests, and the doctest confirm the sprint contains only workflow, README, and Book/ledger evidence changes. |
| [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) | Sprint intent and evidence remain valid and reachable in Book v2. | T-403-E3 / `test_book_v2_validation` | pass | Installed validation reports a valid Book v2 with five reachable intent chapters. |
| [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) | Realized core lifecycle and validation semantics remain satisfied. | T-402-E4 / 43 core unit, 4 lifecycle, 2 serialization tests, core doctest | pass | Explicit topology, atomic lifecycle, terminal completion, ordered history, and validated restore remain green without crate changes. |
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | Realized one-shot adapter and actual-process lifecycle behavior remain satisfied. | T-402-E4 / public runner and four actual-process regressions | pass | Configure/create/transition/complete and typed request/lifecycle failures retain exact public behavior. |
| [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) | Exact-limit, oversize, classification, and operational-I/O semantics remain satisfied. | T-402-E4 / bounded-ingestion unit, public, and process regressions | pass | Exact-limit and one-over cases, oversize-before-JSON, I/O precedence, and process exit 2 remain green. |
| [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) | Supplied-writer exactly-once flush, precedence, buffered failure, and process mapping remain satisfied. | T-402-E4 / flush unit, `BufWriter`, process-shell, and actual-process regressions | pass | Five response classes, staged output failures, real buffered drain failure, exit 1, and ordinary output remain green. |

## Summary

- Unit/repository contract checks: 8 passed / 0 failed / 8 total
- GitHub integration checks: 2 passed / 0 failed / 2 total
- Hosted E2E checks: 1 passed / 0 failed / 1 total
- Workspace all-target regression: 91 passed / 0 failed / 91 total
- Workspace doctests: 1 passed / 0 failed / 1 total; CLI has no doctests
- CI status: green

## CI Confirmation

- **Head SHA:** `85aaa99e6cbe375129475feb445319f2fd94beda`
- **CI run:** [Rust CI run 31281268756](https://github.com/crussella0129/CubiKan/actions/runs/31281268756), event `push`, branch `dev`, attempt `1`
- **Job:** [Rust quality gate 93162907989](https://github.com/crussella0129/CubiKan/actions/runs/31281268756/job/93162907989)
- **Conclusion:** success
- **Confirmations:** [unit and repository checks](unit-tests.md), [GitHub integration checks](integration-tests.md), [hosted E2E](e2e-tests.md), and [clean Test Critique](critique.md)

The local five-gate reproduction ran at the same clean Build SHA with local
stable Rust `1.95.0`. The hosted job independently checked out that SHA and ran
on observed Ubuntu `24.04.4` with current stable Rust `1.97.1`; those versions
are provenance, not fixed support commitments.

## Failures

None.

## Technical Debt Identified

- No follow-up intent was opened. Branch protection, required-check repository
  settings, caching, artifacts, coverage/security scanners, releases,
  deployment, auto-merge, an MSRV, and OS/toolchain matrices remain explicit
  INT-0005 nonclaims rather than incomplete Sprint 4 work.
- The workflow deliberately follows floating `ubuntu-latest` and Rust `stable`;
  future changes are compatibility feedback, not a fixed-version guarantee.

## Coverage Observations

- Configuration checks prove all four declared event filters; the real hosted
  run dynamically proves only `push` on `dev`. The later draft `dev → main` PR
  run is a remote-checkpoint confirmation, not the realization oracle.
- Fail-fast behavior is proved structurally by absence of failure masks and by
  hosted `/usr/bin/bash -e {0}` command shells. No artificial failing hosted run
  or retry was introduced.
- Workflow permission is explicitly `contents: read`; GitHub additionally
  reports implicit `Metadata: read` for its masked built-in token. No custom
  secret is referenced.
- The gate intentionally omits `--locked`; lockfile-strict CI was not part of
  the accepted contract.
