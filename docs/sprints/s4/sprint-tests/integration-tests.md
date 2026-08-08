# Sprint 4 GitHub Workflow Integration Verification

- **Intent:** [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md)
- **Tested Build head:** `85aaa99e6cbe375129475feb445319f2fd94beda`
- **Integration boundary:** GitHub workflow and job APIs; no stub or mock
- **Conclusion:** pass

## `test_ci_workflow_is_registered_on_github`

**Coverage:** T-401-E1 and T-402-E3; INT-0005 workflow registration and event
scope.

GitHub returned one active workflow with these values:

| Field | Observed value |
|-------|----------------|
| Workflow ID | `330204114` |
| Name | `Rust CI` |
| Path | `.github/workflows/ci.yml` |
| State | `active` |
| Remote branch | `dev` |
| Remote `dev` head | `85aaa99e6cbe375129475feb445319f2fd94beda` |
| Remote workflow blob | `96420136d282ef93bb60b0607dffac1d28427a8d` |
| Local workflow blob | `96420136d282ef93bb60b0607dffac1d28427a8d` |

The matching blob IDs prove that GitHub registered the workflow definition from
the exact committed Build head rather than a different local or remote file.
The committed YAML declares pull requests targeting `dev` or `main` and pushes
to `dev` or `main`; the hosted push execution is confirmed in the E2E result.

## `test_hosted_quality_job_exposes_all_steps`

**Coverage:** T-401-E4 and T-402-E1–E3; INT-0005 stable-toolchain,
separate-gate, fail-fast, and hosted-execution criteria.

The registered workflow produced exactly one job:

| Field | Observed value |
|-------|----------------|
| Run | [31281268756](https://github.com/crussella0129/CubiKan/actions/runs/31281268756) |
| Job | [93162907989 — Rust quality gate](https://github.com/crussella0129/CubiKan/actions/runs/31281268756/job/93162907989) |
| Job status / conclusion | `completed` / `success` |
| Started | `2026-08-08T22:15:30Z` |
| Completed | `2026-08-08T22:16:02Z` |

| Hosted step | Status / conclusion |
|-------------|---------------------|
| Check out repository | `completed` / `success` |
| Install stable Rust | `completed` / `success` |
| Check formatting | `completed` / `success` |
| Run Clippy | `completed` / `success` |
| Check workspace | `completed` / `success` |
| Run workspace tests | `completed` / `success` |
| Run workspace doctests | `completed` / `success` |

The workflow contains no failure override, and the command log shows
`/usr/bin/bash -e {0}` for setup and each Cargo gate, so a nonzero command keeps
GitHub's default failing behavior. No synthetic failing hosted run was
introduced.

## Hosted runtime and checkout provenance

The actual job log records runner `2.336.0`, Ubuntu `24.04.4`, image
`ubuntu-24.04` version `20260720.247.2`, and current stable resolving to
`rustc 1.97.1 (8bab26f4f 2026-07-14)` with `rustfmt` and `clippy` current. These
are observations for this run, not a fixed runner, cross-platform, or MSRV
promise.

GitHub downloaded the pinned
`actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1`, and the checkout
log recorded `persist-credentials: false`. It fetched
`+85aaa99e6cbe375129475feb445319f2fd94beda:refs/remotes/origin/dev`, then both
`git rev-parse refs/remotes/origin/dev` and `git log -1 --format=%H` returned
that same SHA. This independently ties the API run, remote ref, and executed
worktree to the tested Build head.

The workflow explicitly configures only `contents: read`; GitHub's hosted log
reported `Contents: read` plus implicit `Metadata: read` for the built-in
masked `GITHUB_TOKEN`. There is no custom secret reference. No mock or stub was
used: GitHub's workflow API, remote contents API, run/job API, and job log are
the external integration boundary, while local YAML checks only verify the
committed definition.

## Reproduction

```sh
gh api repos/crussella0129/CubiKan/actions/workflows/ci.yml
gh api 'repos/crussella0129/CubiKan/contents/.github/workflows/ci.yml?ref=dev'
gh run view 31281268756 --json databaseId,url,event,headBranch,headSha,status,conclusion,jobs
gh run view 31281268756 --job 93162907989 --log
```

This evidence proves workflow registration and execution. It does not claim
that GitHub branch protection requires the status, that a merge is authorized,
or that a pull-request event has already run; human approval and the later
remote checkpoint remain separate.
