# Sprint 4 Unit and Repository Verification

- **Primary intent:** [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md)
- **Preserved intents:** [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md), [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md), [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md), and [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md)
- **Accepted base:** `a68aa776e3433b08abea0edd3b5139754329e28a`
- **Tested Build head:** `85aaa99e6cbe375129475feb445319f2fd94beda`
- **Initial worktree:** clean
- **Local stable toolchain:** `rustc 1.95.0`; `cargo 1.95.0`
- **Local conclusion:** pass; hosted GitHub evidence is recorded separately in [integration](integration-tests.md) and [E2E](e2e-tests.md) results

## Named contract checks

| Named check | EARS / acceptance boundary | Confirmation | Result |
|-------------|----------------------------|--------------|--------|
| `test_ci_workflow_matches_event_and_job_scope` | T-401-E1; INT-0005 event/job criterion | Parsed the committed `Rust CI` workflow and found pull-request bases `{dev, main}`, push branches `{dev, main}`, exactly one `quality` job named `Rust quality gate`, `ubuntu-latest`, and timeout `15`. All four routes are configuration assertions; only `push` on `dev` was dynamically observed. | pass |
| `test_ci_workflow_is_read_only_and_bounded` | T-401-E1–E3; INT-0005 least-privilege criterion | Confirmed explicit permissions equal only `contents: read`; group `${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}` with cancellation makes PR numbers N/M distinct and pushed `refs/heads/dev`/`refs/heads/main` distinct; and no `pull_request_target`, `workflow_run`, write permission, custom secret reference, cache, artifact, release, deployment, service, or second job. No cancellation event was induced. | pass |
| `test_ci_workflow_pins_checkout_and_drops_credentials` | T-401-E2 | Confirmed the sole action reference is `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1` with the v7.0.1 annotation and `persist-credentials: false`. | pass |
| `test_ci_workflow_provisions_current_stable_components` | T-401-E4; INT-0005 toolchain criterion | Confirmed the sole toolchain setup is `rustup toolchain install stable --profile minimal --component rustfmt,clippy`; all five Cargo commands explicitly select `+stable`, with no numbered Rust/MSRV pin or third-party setup action. | pass |
| `test_ci_workflow_runs_five_canonical_gates_in_order` | T-402-E1–E2; INT-0005 five-gate criterion | Compared five separately named commands, flags, and check-step environment with the locked order. The executed structural negative check found no `continue-on-error`, retry wrapper, `if: always()`, failure mask, later-step override, or `--locked`; hosted command logs use `/usr/bin/bash -e {0}`. No synthetic failing hosted run was introduced. | pass |
| `test_readme_documents_ci_contract_and_nonclaims` | T-403-E1–E2; INT-0005 documentation criterion | Confirmed the workflow link, both event branch sets, current-stable hosted-Ubuntu scope, exact five commands in order, status-only automation, retained human approval, and explicit exclusions for caches, artifacts, coverage/security scanners, secrets, releases/deployment, auto-merge, MSRV, and OS/toolchain matrices. The existing product-exclusion suffix is unchanged. | pass |
| `test_ci_scope_has_no_product_or_dependency_drift` | T-402-E4; INT-0005 scope criterion; preserved INT-0001–INT-0004 | `git diff --quiet main...HEAD -- Cargo.toml Cargo.lock crates` passed; the changed paths contain only the workflow, README, and Book/ledger evidence. Metadata and the depth-one dependency tree remain the accepted two-crate graph, and the complete existing Rust suite remains green. | pass |
| `test_book_v2_validation` | T-403-E3; INT-0005 durable-evidence criterion | The installed validator reported exactly `check-book: valid v2 Book (5 intent chapters)` with INT-0005 and Sprint 4 reachable from `SUMMARY.md`. | pass |

The workflow inspection used Python 3 with PyYAML 6.0.3 only as a one-off YAML
parser. It added no repository dependency or product test harness.

## Five canonical local gates

The following commands ran in locked order at the tested Build head:

| Gate | Command | Result |
|------|---------|--------|
| Formatting | `cargo +stable fmt --all -- --check` | pass |
| Clippy | `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings` | pass |
| Warnings-denied check | `RUSTFLAGS="-D warnings" cargo +stable check --workspace --all-targets` | pass |
| All-target tests | `cargo +stable test --workspace --all-targets` | pass — 91 passed, 0 failed |
| Doctests | `cargo +stable test --doc --workspace` | pass — 1 core doctest passed, 0 failed; CLI has 0 doctests |

## Regression breakdown and preserved-intent assertions

| Intent | Executed evidence | Result |
|--------|-------------------|--------|
| [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) | 43 core unit tests, 4 lifecycle integration tests, 2 serialization integration tests, and the core doctest cover explicit topology, atomic lifecycle failures, terminal completion, ordered history, and tamper-rejecting restore. | pass |
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | `test_cli_configure_create_transition_complete`, malformed exit-2 and lifecycle exit-3 actual-process cases, plus public runner success/prior-state tests preserve the versioned one-shot adapter behavior. | pass |
| [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) | Exact-limit, one-byte-over, oversize-before-JSON, read/write precedence, and actual-process oversized exit-2 cases preserve bounded ingestion and typed classification. | pass |
| [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) | Five-response exactly-once flush, output-stage precedence, real-`BufWriter` drain failure, process-shell exit 1, and ordinary actual-process regressions preserve the supplied-writer flush contract. | pass |

The all-target total is 29 CLI unit + 9 CLI public integration + 4 CLI
actual-process + 43 core unit + 4 core lifecycle + 2 core serialization = 91.
There were 0 failures, ignored, measured, or filtered tests. Workspace doctests
were 1 core pass and 0 CLI tests. Together with the quiet crate diff, these
named outcomes preserve INT-0001 through INT-0004 without changing their
realized semantics.

## Repository and provenance commands

```sh
git diff --quiet main...85aaa99e6cbe375129475feb445319f2fd94beda -- Cargo.toml Cargo.lock crates
git diff --check main...85aaa99e6cbe375129475feb445319f2fd94beda
cargo metadata --no-deps --format-version 1
cargo tree --workspace --depth 1 -e normal,build,dev
bash /mnt/c/Users/charl/.codex/plugins/cache/sprint-loops/sprint-loop/local/skills/sprint-loop/scripts/check-book.sh
```

All passed. The completion ledger records T-401 at
`567e3d5f496cb9bd27830052c4fecbd56d06d36f`, T-402 at
`f70ee3d34f633023a633aad6e7377108cebf571d`, and T-403 at
`c4489cd35bfdce36600925918d73c215b0b2a891`; each is an ancestor of the tested
Build head.

The implementation commits above are distinct from their ledger-backfill
commits `53114d9`, `a0661fc`, and `85aaa99`; the final backfill is also the
tested Build head. All six Sprint 4 commits descend from the accepted base.
