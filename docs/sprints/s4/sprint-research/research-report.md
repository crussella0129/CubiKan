# Sprint 4 Research Report

## Intents Reviewed

- [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) — created; relevance: owns the selected hosted Rust CI outcome and its actual-run evidence boundary; current state: `proposed`.
- [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) — selected; relevance: its realized writer and process behavior must remain unchanged under the automated regression suite; current state: `realized`.
- [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) — selected; relevance: its bounded-input and operational-error behavior must remain covered without new resource policy; current state: `realized`.
- [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) — selected; relevance: its actual-process lifecycle boundary supplies the existing adapter tests the gate will execute; current state: `realized`.
- [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) — selected; relevance: its warning-free Rust domain contract and doctest must remain unchanged; current state: `realized`.

## 1. Sprint Goal

Add one least-privilege GitHub Actions workflow that automatically executes
CubiKan's accepted Rust quality gates on pull requests targeting `dev` or `main`
and pushes to `dev` or `main`. Sprint 4 will prove the configuration with local
contract checks and an actual successful hosted run for a committed `dev` head,
then document the exact Linux/current-stable boundary. It will not change crate
code or dependencies, configure branch protection, merge automatically, publish
artifacts, release or deploy software, use secrets, or claim an MSRV or
cross-platform support matrix.

## 2. Existing Code Survey

| File | Relevance | Finding |
|------|-----------|---------|
| `.github/dependabot.yml` | high | Cargo and GitHub Actions updates already target `dev`; PR-triggered CI on `dev` is therefore required before updater intake can be classified as green. |
| `README.md` | high | Defines a current Rust toolchain and the canonical formatting, Clippy, all-target test, and doctest commands; prior Test plans additionally require warnings-denied all-target compilation. |
| `Cargo.toml` | high | The accepted repository is a Rust 2024 two-crate workspace with centralized dependencies and no MSRV or pinned toolchain policy. |
| `docs/work/remote-profile.md` | high | Declares GitHub, `dev → main`, and human-approved merging; workflow triggers should cover both work and corpus branches without changing this authority boundary. |
| `docs/sprints/s3/sprint-tests/test-report.md` | high | Records 91 passing all-target tests, one doctest, and `CI status: not-configured`; local committed-head checks are still the only authority. |
| `docs/sprints/s3/sprint-research/research-report.md` | medium | Previously deferred CI because a first hosted run could not be a pre-close oracle; a `push: dev` trigger lets Sprint 4 obtain that evidence before realization. |
| `docs/intents/INT-0001-chain-agnostic-intent-lifecycle-core.md` | medium | Requires a warning-free core and executable documentation while excluding platform policy; CI can preserve it without source changes. |
| `docs/intents/INT-0002-runnable-lifecycle-adapter.md` | medium | Supplies actual-process tests and preserves the no-persistence/no-service boundary. |
| `docs/intents/INT-0003-bounded-cli-request-ingestion.md` | medium | Supplies request-limit and deterministic I/O regression cases while keeping timeouts, quotas, and stable protocol policy deferred. |
| `docs/intents/INT-0004-explicit-cli-response-flush.md` | medium | Supplies buffered-writer and process-shell regressions and forbids stronger delivery claims; no follow-on code change is needed. |

## 3. External Sources

- [Workflow syntax for GitHub Actions](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax) — defines workflow location, pull-request target and push branch filters, token permissions, expression contexts, and finite job timeouts.
- [Control workflow concurrency](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency) — documents expression-based concurrency groups and `cancel-in-progress` for superseded runs.
- [Secure use reference](https://docs.github.com/en/actions/reference/security/secure-use) — recommends minimum token permissions and immutable full-commit action references for third-party or external executable code.
- [`actions/checkout` v7.0.1](https://github.com/actions/checkout/releases/tag/v7.0.1) — supplies the current official checkout release and the commit pinned by the workflow; persisted credentials can be disabled because the job never pushes.
- [Rustup profiles](https://rust-lang.github.io/rustup/concepts/profiles.html) — recommends the minimal profile for CI and identifies `rustfmt` and `clippy` as separately installable components.

## 4. Risks, Unknowns, Dependencies

- **Risk:** Static YAML or local Cargo success cannot prove GitHub dispatch,
  checkout provisioning, token permissions, or hosted execution. Build must push
  a committed workflow to `dev`, and Test must record the resulting run before
  INT-0005 can be realized.
- **Risk:** Push and pull-request events can overlap for the standing `dev →
  main` PR, while forks can reuse the same head branch name. A concurrency key
  must make superseded runs cancelable without coupling unrelated pull requests
  or pushed refs.
- **Risk:** A broad token, persisted checkout credential, `pull_request_target`,
  or write-capable step would execute unnecessary authority. The job needs only
  `contents: read` and ordinary `pull_request`/`push` events.
- **Risk:** Current stable Rust and `ubuntu-latest` float. This is useful
  compatibility feedback but must not be described as an MSRV, fixed Linux
  distribution, or cross-platform support promise.
- **Risk:** Adding cache, audit, coverage, artifact, release, or deployment steps
  would create new trust and maintenance surfaces. They remain explicit
  exclusions rather than silent CI extras.
- **Unknown:** Repository Actions policy, GitHub availability, or account limits
  could prevent the first hosted run. If no successful run is observable, Test
  must report failure and INT-0005 must remain unrealized.
- **Dependency:** The workflow relies on GitHub-hosted Ubuntu, the immutable
  official checkout commit, Rustup's stable channel, crates.io access, and the
  repository's existing `Cargo.lock`; it adds no repository dependency.

## 5. Recommended Approach

Create `.github/workflows/ci.yml` with `pull_request` filters for `dev` and
`main`, `push` filters for `dev` and `main`, workflow-level `contents: read`, and
a concurrency key based on workflow plus repository-local pull-request number
or full pushed ref with `cancel-in-progress: true`. Use one `ubuntu-latest` job
with a 15-minute timeout.
Pin official `actions/checkout` v7.0.1 to its full release commit, disable
persisted credentials, and install stable Rust through Rustup's minimal profile
with `rustfmt` and `clippy`.

Run five separate steps: `cargo +stable fmt --all -- --check`, workspace Clippy
for all targets/features with `-D warnings`, `RUSTFLAGS=-D warnings cargo
+stable check --workspace --all-targets`, `cargo +stable test --workspace
--all-targets`, and `cargo +stable test --doc --workspace`. Do not add
`--locked`: it is absent from the accepted local contract and lockfile
strictness is a distinct policy. Do not use `continue-on-error`, caches, or
third-party setup actions.

Verify the exact event/permission/concurrency/timeout/action/toolchain/command
shape locally, run the full workspace gates, then push the committed Build head
to `dev`. Record a successful GitHub Actions run and job at that exact SHA in
Test evidence before realization. Update the README with the automated scope,
triggers, commands, and nonclaims. Branch protection and required-check settings
remain a human-controlled repository decision.

Alternatives considered: retaining local-only checks leaves the repeatedly
recorded gap open. A larger OS/toolchain/MSRV/security/coverage/release matrix is
not justified by current evidence. Persistence, service, Electron, blockchain,
and additional resource controls all require unresolved product or workload
policy and remain separate future intents.

## Artifacts

- No separate artifacts were saved; repository findings are listed in the code
  survey and external evidence is linked above.
