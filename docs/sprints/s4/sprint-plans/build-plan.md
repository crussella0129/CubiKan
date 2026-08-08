Finalized - DO NOT EDIT

# Sprint 4 Build Plan

## Intents

- [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) — state: planned; acceptance criteria covered: GitHub event scope, least-privilege bounded execution, current-stable Rust provisioning, five canonical gates, successful hosted-run evidence, contributor documentation, and no crate/runtime drift.
- [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) — state: realized and preserved; affected criterion: buffered-writer, process-shell, exact response, and exit behavior remain green under the automated suite.
- [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) — state: realized and preserved; affected criterion: exact request limits and I/O classification remain green without new quota policy.
- [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) — state: realized and preserved; affected criterion: actual-process lifecycle behavior remains green without adapter changes.
- [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) — state: realized and preserved; affected criterion: the warning-free core, full domain suite, and doctest remain unchanged.

## Schema Tree

- Automate the accepted Rust quality contract on GitHub
  - Hosted execution boundary
    - T-401: Establish the least-privilege workflow shell and toolchain
  - Quality contract
    - T-402: Add the five canonical Rust gates and hosted-run requirement
  - Contributor boundary
    - T-403: Document automation, reproduction, and explicit nonclaims

## Execution Sequence

### T-401: Establish the bounded Rust CI workflow shell

- **Intent:** [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md)
- **Touches:** `.github/workflows/ci.yml`
- **Depends on:** (none)
- **Acceptance criterion:** The workflow covers work/corpus pushes and dependency/sprint pull requests while using a single bounded, read-only Ubuntu job with immutable checkout and current stable Rust tooling.
- **Success criterion (EARS):**
  - **T-401-E1 — WHEN** a pull request targets `dev` or `main`, or a commit is pushed to `dev` or `main`, **THEN** `Rust CI` **SHALL** define dispatch to exactly one `quality` job on `ubuntu-latest` with `timeout-minutes: 15`.
  - **T-401-E2 — WHEN** the workflow token and checkout boundary are inspected, **THEN** the workflow **SHALL** grant only `contents: read`, use official checkout v7.0.1 at immutable commit `3d3c42e5aac5ba805825da76410c181273ba90b1`, set `persist-credentials: false`, and contain no write permission, secret, privileged trigger, cache, artifact, release, or deployment step.
  - **T-401-E3 — WHEN** a newer run for the same workflow and pull request or pushed ref begins, **THEN** workflow concurrency **SHALL** group by workflow plus `github.event.pull_request.number || github.ref` and cancel the superseded in-progress run without coupling distinct pull requests, fork heads, or refs.
  - **T-401-E4 — WHEN** the job prepares Rust tooling, **THEN** it **SHALL** install `stable` through Rustup with the minimal profile, `rustfmt`, and `clippy`, and later Cargo commands **SHALL** explicitly select `+stable` without defining an MSRV.
- **Notes:** Quote the YAML `on` key for generic parser compatibility. Use ordinary `pull_request`, never `pull_request_target`. The first task commit also records the initialized Sprint 4 Book/intent/plan scaffolding required by the task helper. Commit as `sprint-4: T-401 establish bounded Rust CI`.

### T-402: Add the canonical Rust quality gates and hosted proof boundary

- **Intent:** [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md)
- **Touches:** `.github/workflows/ci.yml`
- **Depends on:** T-401
- **Acceptance criterion:** The hosted job executes the accepted quality contract as separate fail-fast steps, changes no product/runtime surface, and produces a successful real run for a committed Sprint 4 `dev` head before realization.
- **Success criterion (EARS):**
  - **T-402-E1 — WHEN** the `quality` job evaluates a revision, **THEN** it **SHALL** execute in order `cargo +stable fmt --all -- --check`, workspace/all-target/all-feature Clippy with `-D warnings`, `cargo +stable check --workspace --all-targets` under `RUSTFLAGS=-D warnings`, `cargo +stable test --workspace --all-targets`, and `cargo +stable test --doc --workspace` as five separately named steps.
  - **T-402-E2 — WHEN** any setup or quality command returns nonzero, **THEN** the job **SHALL** retain GitHub's default failure behavior with no `continue-on-error`, retry, or later-step override that could report a false successful gate.
  - **T-402-E3 — WHEN** the completed Sprint 4 Build head is pushed to `dev`, **THEN** GitHub Actions **SHALL** register `.github/workflows/ci.yml`, dispatch a `push` run for that exact SHA, and report the `quality` job and workflow conclusion as `success` before Test claims realization.
  - **T-402-E4 — WHEN** the Sprint 4 diff is inspected, **THEN** it **SHALL** contain no Cargo manifest/lockfile or `crates/` change and **SHALL** preserve all existing Rust tests, protocol shapes/error codes, process exits, and realized intent semantics.
- **Notes:** Do not add `--locked`; the accepted local contract does not currently require lockfile-strict execution. After T-403 is committed, push `dev` under the declared remote profile solely to obtain T-402-E3 Test evidence; do not open the sprint PR early. If the hosted run is unavailable or fails, INT-0005 remains unrealized and the sprint cannot close successfully. Commit as `sprint-4: T-402 add canonical Rust CI gates`.

### T-403: Document the automated quality boundary

- **Intent:** [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md)
- **Touches:** `README.md`
- **Depends on:** T-402
- **Acceptance criterion:** Contributors can reproduce the five gates and understand the workflow's triggers, hosted scope, and non-enforcement boundaries.
- **Success criterion (EARS):**
  - **T-403-E1 — WHEN** a contributor reads the Development section, **THEN** it **SHALL** link `Rust CI`, name pull-request targets and push branches, state current-stable GitHub-hosted Ubuntu coverage, and list the same five local gates in workflow order.
  - **T-403-E2 — WHEN** CI authority and scope are reviewed, **THEN** the documentation **SHALL** distinguish status-producing automation from required branch protection and human merge approval, and **SHALL** explicitly exclude caches, artifacts, coverage/security scanners, secrets, releases/deployment, automatic merge, MSRV, and OS/toolchain matrices.
  - **T-403-E3 — WHEN** the Sprint 4 Project Book is validated after INT-0005 and its sprint evidence are linked, **THEN** the installed validator **SHALL** report a valid Book v2 with five reachable intent chapters.
- **Notes:** Keep product exclusions unchanged. A passing workflow is evidence, not authorization to merge. Commit as `sprint-4: T-403 document automated Rust quality gate`.
