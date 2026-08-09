# INT-0005 — Automated Rust quality gate

<!-- sprint-loop-intent-v2 -->
- **Intent ID:** INT-0005
- **State:** realized
- **Work evidence:** [Sprint 4 build plan](../sprints/s4/sprint-plans/build-plan.md)
- **Completion evidence:** [T-401–T-403 completion ledger](../work/completed-tasks.md#t-401-sprint-4)
- **Code evidence:** [Rust CI workflow](../../.github/workflows/ci.yml)
- **Test evidence:** [Sprint 4 test report](../sprints/s4/sprint-tests/test-report.md), [Sprint 5 regression report](../sprints/s5/sprint-tests/test-report.md)
- **Documentation evidence:** [CubiKan development guide](../../README.md)

## Intent

Run CubiKan's existing Rust quality contract automatically on GitHub-hosted
Linux for changes entering the accepted `main` corpus and for work on `dev`.
One least-privilege workflow will validate pull requests targeting `dev` or
`main` and pushes to `dev` or `main` with current stable Rust, using the same
formatting, linting, warnings-denied compilation, all-target test, and doctest
gates already used as local sprint evidence.

This is an engineering feedback boundary, not a product or deployment surface.
It does not configure branch protection, merge automation, releases, secrets,
artifact publication, security certification, an MSRV, or a cross-platform
support promise.

## Acceptance criteria

- A workflow under `.github/workflows/` is triggered by pull requests targeting
  `dev` or `main` and pushes to `dev` or `main`, covering both dependency-update
  intake and the standing sprint `dev → main` checkpoint.
- The workflow grants only `contents: read`, checks out through an immutable
  official `actions/checkout` release with persisted credentials disabled,
  cancels superseded runs for the same workflow and pull request or pushed ref
  without coupling unrelated fork heads, and gives its single job a finite
  timeout on a GitHub-hosted Ubuntu runner.
- The job installs current stable Rust with the minimal profile plus `rustfmt`
  and `clippy`, then executes formatting, workspace Clippy with warnings denied,
  warnings-denied all-target compilation, workspace all-target tests, and
  workspace doctests as separate fail-fast steps.
- An actual GitHub Actions run for a committed Sprint 4 `dev` head completes
  successfully before realization, and its run URL, event, head SHA, conclusion,
  and job result are recorded alongside reproducible local verification.
- Consumer documentation names the automated gate, its triggers and commands,
  and its current-stable Ubuntu scope while explicitly excluding caches,
  artifacts, coverage/security scanners, secrets, releases/deployment, branch
  protection, automatic merge, MSRV, and OS/toolchain matrices.
- The sprint adds no crate dependency and changes no `cubikan-core` or
  `cubikan-cli` source, runtime protocol, domain behavior, or process exit
  meaning.

## Rationale

Every completed sprint has relied on authoritative local checks because hosted
CI was not configured. The repository now has a stable two-crate quality
contract, a GitHub `dev → main` remote profile, and Dependabot updates targeting
`dev`, so the same gates can run remotely without selecting unresolved product
policy. A successful hosted run is required because static YAML inspection alone
cannot prove GitHub dispatch, runner provisioning, or job execution.

## Alternatives

Keeping local-only verification would preserve the current process but continue
the repeatedly recorded automation gap. A larger OS, toolchain, MSRV, coverage,
audit, cache, or release matrix would introduce support, security, and
maintenance commitments that current evidence does not require. A third-party
toolchain setup action is unnecessary because GitHub-hosted runners include
`rustup`; the official checkout action plus Rust's own toolchain manager keeps
the executable trust surface small. Persistence, service, Electron, blockchain,
and new domain features remain separate intents because each requires unresolved
product or platform decisions.

## Consequences

The gate consumes GitHub Actions time and depends on GitHub-hosted Ubuntu,
network access, the current stable Rust channel, and the pinned checkout action.
Current stable and `ubuntu-latest` may expose future compiler or runner changes;
that is deliberate ongoing compatibility feedback, not an MSRV or fixed-OS
promise. Dependabot can propose checkout updates against `dev`. A green workflow
is review evidence, but it does not make the check required or authorize merging.

## Transition history

- 2026-08-08: created as `proposed` after Sprint 4 research selected the repeatedly recorded hosted-CI gap as the strongest bounded outcome that requires no new product policy.
- 2026-08-08: moved to `planned` when Sprint 4 decomposed the workflow, hosted-run proof, and contributor boundary into T-401–T-403 with intent-to-EARS-to-test traceability.
- 2026-08-08: moved to `active` immediately before Build began T-401 under the finalized Sprint 4 plans.
- 2026-08-08: moved to `realized` after T-401–T-403 completed, the final Test Critic returned `clean`, all local gates passed, and GitHub Actions push run 31281268756 succeeded at exact Build head `85aaa99e6cbe375129475feb445319f2fd94beda`.
