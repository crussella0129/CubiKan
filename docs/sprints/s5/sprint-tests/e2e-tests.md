# Sprint 5 End-to-End Test Results

- **Status:** possible, executed, and passed
- **Primary intent:** [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md)
- **Preserved intents:** [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md), [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md), [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md), [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md), and [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md)
- **Tested Build head:** `6979ca2217ac1b838c406bf21821e32b3a4f6227`
- **Actual-process command:** `cargo test -p cubikan-cli --test cli_e2e`
- **Workspace command:** `cargo test --workspace --all-targets`
- **Doctest command:** `cargo test --doc --workspace`
- **Local result:** 6 actual-process tests passed; 100 all-target tests passed; 1 doctest passed; 0 failed

## Actual-process confirmations

All six tests spawn Cargo's `CARGO_BIN_EXE_cubikan`, write the request through
real piped standard input, close stdin, and inspect the child process's real exit
status, stdout, and stderr. Their shared response assertion requires empty
stderr, exactly one terminating newline, and exactly one complete JSON document.

| Named test | EARS | Actual-process assertion | Result |
|------------|------|--------------------------|--------|
| `test_cli_generates_id_when_member_is_omitted` | T-503-E1 | Removes the `intent_unit.id` object member rather than assigning `null`; the process exits `0`, emits one version 1 success line with empty stderr, and returns a dynamically generated ID that parses through `cubikan_core::IntentUnitId` as non-nil UUID v4. The complete response is asserted with species `feature`, workflow `delivery`, phase `done`, completed status, and the exact three-record transition/transition/completion history. | pass |
| `test_cli_reports_explicit_null_id_with_exit_2` | T-503-E2 | Places JSON `null` at the present `intent_unit.id` member; the process exits `2`, emits one version 1 `invalid_request` line with a nonempty human message and empty stderr, and includes no `intent_unit`, `field`, or `operation_number`. Exact diagnostic prose is intentionally not pinned. | pass |
| `test_cli_configure_create_transition_complete` | T-503-E3 | The unchanged checked-in fixed-ID scenario exits `0` and preserves the exact version 1 completed lifecycle response, including fixed ID, final phase, completed status, and three ordered history records. | pass |
| `test_cli_reports_malformed_request_with_exit_2` | T-503-E3 | Malformed JSON exits `2` and preserves the exact `invalid_json` response without state. | pass |
| `test_cli_reports_lifecycle_rejection_with_exit_3` | T-503-E3 | The undeclared second transition exits `3` and preserves the exact `transition_not_allowed` error, one-based operation number, and prior-success state snapshot. | pass |
| `test_cli_reports_oversized_request_with_exit_2` | T-503-E3 | A request whose required final root brace is byte `1_048_577` exits `2` and preserves the exact `request_too_large` response without state. | pass |

The omission and explicit-null cases are otherwise derived from the same valid
completed-lifecycle fixture. Their divergent exit and response assertions
therefore isolate member absence from a present non-string representation at the
actual executable boundary rather than only at an internal decoder seam.

## Exact-head hosted E2E

- **Named E2E:** `test_hosted_sprint_five_quality_run_succeeds`
- **Coverage:** T-504-E3, with T-501 through T-503 and all preserved intents exercised by the hosted workspace gates
- **Run:** [31285064082](https://github.com/crussella0129/CubiKan/actions/runs/31285064082)
- **Job:** [93172288024](https://github.com/crussella0129/CubiKan/actions/runs/31285064082/job/93172288024)

The final committed Sprint 5 Build head was pushed to `dev`. Three independent
observations identify the same revision:

| Provenance observation | SHA | Result |
|------------------------|-----|--------|
| Local committed Build `HEAD` | `6979ca2217ac1b838c406bf21821e32b3a4f6227` | match |
| Local remote-tracking `origin/dev` | `6979ca2217ac1b838c406bf21821e32b3a4f6227` | match |
| GitHub run `head_sha` | `6979ca2217ac1b838c406bf21821e32b3a4f6227` | match |

| Hosted assertion | Observed value | Result |
|------------------|----------------|--------|
| Workflow | `Rust CI` (ID `330204114`) | pass |
| Event | `push` | pass |
| Head branch | `dev` | pass |
| Run status / conclusion | `completed` / `success` | pass |
| Run attempt | `1` (no retry or rerun) | pass |
| Sole job | `Rust quality gate` (ID `93172288024`) | pass |
| Job status / conclusion | `completed` / `success` | pass |

- **Run interval:** `2026-08-08T23:55:40Z` through `2026-08-08T23:56:07Z`
- **Job interval:** `2026-08-08T23:55:43Z` through `2026-08-08T23:56:06Z`

| Hosted quality step | Interval | Status / conclusion |
|---------------------|----------|---------------------|
| Check formatting | `2026-08-08T23:55:46Z`–`2026-08-08T23:55:47Z` | `completed` / `success` |
| Run Clippy | `2026-08-08T23:55:47Z`–`2026-08-08T23:55:53Z` | `completed` / `success` |
| Check workspace | `2026-08-08T23:55:53Z`–`2026-08-08T23:55:58Z` | `completed` / `success` |
| Run workspace tests | `2026-08-08T23:55:58Z`–`2026-08-08T23:56:04Z` | `completed` / `success` — 100 passed, 0 failed |
| Run workspace doctests | `2026-08-08T23:56:04Z`–`2026-08-08T23:56:04Z` | `completed` / `success` — 1 core pass, 0 failed; CLI had 0 |

The matching local and hosted all-target totals are 32 CLI unit, 13 CLI public
integration, 6 CLI actual-process, 43 core unit, 4 core lifecycle integration,
and 2 core serialization tests: 100 passed and 0 failed. Both environments also
recorded the one core doctest passing, with no CLI doctests. The hosted result is
therefore the exact-Build-head realization evidence rather than a static workflow
inspection or local emulation.

## External and flake boundary

The six actual-process cases are deterministic local child processes with no
network, retry, shared service, durable state, or timing oracle. The hosted
result is one successful attempt with no retry, rerun, or dependency cache and
the workflow's existing 15-minute job timeout. It exercises GitHub Actions,
floating `ubuntu-latest`, Rustup's current floating `stable` channel, the
crates.io index/download boundary, and the immutable pinned checkout action.
Observed runner and toolchain versions are provenance for this attempt, not an
MSRV, fixed-OS promise, future availability guarantee, Windows/macOS support,
coverage or security certification, release/deployment evidence, or
branch-protection proof.

## Reproduction

```sh
cargo test -p cubikan-cli --test cli_e2e
cargo test --workspace --all-targets
cargo test --doc --workspace
gh run view 31285064082 --json attempt,conclusion,createdAt,event,headBranch,headSha,jobs,status,updatedAt,url
gh api repos/crussella0129/CubiKan/actions/jobs/93172288024
```

## Remote-checkpoint and authority boundary

This successful `dev` push run is the Sprint 5 realization oracle. The later
single draft `dev → main` pull request will trigger a separate candidate run at
the post-Loop remote checkpoint; that later run confirms the handoff candidate
but is not realization evidence because recording it in this sprint would create
a new evidence head and recursively require another run.

The successful workflow status is review evidence only. It does not configure
or prove required branch protection, authorize a merge, add automatic merge
behavior, replace the declared human approval mark, or weaken the standing
one-PR-per-sprint checkpoint.
