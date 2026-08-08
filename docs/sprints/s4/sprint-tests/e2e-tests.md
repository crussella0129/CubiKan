# Sprint 4 Hosted End-to-End Verification

- **Intent:** [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md)
- **Named E2E:** `test_hosted_dev_push_quality_run_succeeds`
- **Coverage:** T-402-E3, with T-401-E1/E4 and T-402-E1/E2 observed through the hosted path
- **Status:** executed and passed

## Exact-head hosted run

The committed Sprint 4 Build head was pushed to `dev`, exercising the real
GitHub event, workflow registration, hosted Ubuntu runner, Rustup/network
boundary, checkout action, and five Cargo gates without a mock.

| Assertion | Observed value | Result |
|-----------|----------------|--------|
| Workflow | `Rust CI` (ID `330204114`) | pass |
| Event | `push` | pass |
| Head branch | `dev` | pass |
| Head SHA | `85aaa99e6cbe375129475feb445319f2fd94beda` | pass |
| Run status | `completed` | pass |
| Run conclusion | `success` | pass |
| Run attempt | `1` (no rerun) | pass |
| Sole job | `Rust quality gate` (ID `93162907989`) | pass |
| Job conclusion | `success` | pass |

- **Run:** [31281268756](https://github.com/crussella0129/CubiKan/actions/runs/31281268756)
- **Job:** [93162907989](https://github.com/crussella0129/CubiKan/actions/runs/31281268756/job/93162907989)
- **Run interval:** `2026-08-08T22:15:26Z` through `2026-08-08T22:16:02Z`
- **Job interval:** `2026-08-08T22:15:30Z` through `2026-08-08T22:16:02Z`

| Hosted gate | Status / conclusion |
|-------------|---------------------|
| Check formatting | `completed` / `success` |
| Run Clippy | `completed` / `success` |
| Check workspace | `completed` / `success` |
| Run workspace tests | `completed` / `success` — 91 passed, 0 failed |
| Run workspace doctests | `completed` / `success` — 1 core pass, 0 failed; CLI had 0 |

The 91 hosted all-target tests comprise 29 CLI unit, 9 CLI public integration,
4 CLI actual-process, 43 core unit, 4 core lifecycle, and 2 core serialization
tests. The job log independently records a fetch mapping from the asserted SHA
to `refs/remotes/origin/dev`; `git rev-parse` of that ref and `git log -1` both
returned the same `85aaa99e6cbe375129475feb445319f2fd94beda` reported by the run
API.

This is the realization oracle required by T-402-E3: the run is a real
`push` event for the exact clean Build SHA, not a local emulation or a run for a
later evidence-only commit.

## External and flake boundary

This result is one successful attempt with no retry, rerun, or cache and a
15-minute job timeout. It exercised GitHub Actions, floating `ubuntu-latest`,
Rustup's current `stable` channel, the crates.io index/download boundary, and
the pinned checkout action. The observed Ubuntu `24.04.4` image and Rust
`1.97.1` are provenance for this run only; they do not establish deterministic
future runner versions, ongoing reliability, an MSRV, Windows/macOS support,
coverage or security certification, release/deployment behavior, or any
branch-protection guarantee.

## Reproduction

```sh
gh run view 31281268756 --json databaseId,url,event,headBranch,headSha,status,conclusion,jobs
gh api repos/crussella0129/CubiKan/actions/jobs/93162907989
```

## Remote-checkpoint boundary

No sprint pull request was open when this realization run completed. The final
`dev → main` draft PR will create a separate pull-request candidate run at the
post-Loop remote checkpoint and that run will be checked before handoff. It is
not the realization oracle because recording its result in the sprint would
create a new head and recursively require another run.

The successful workflow status is review evidence only. This E2E does not
configure or prove required branch protection, authorize a merge, add automatic
merge behavior, or replace the declared human-approval checkpoint.
