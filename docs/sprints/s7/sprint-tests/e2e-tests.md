# Sprint 7 End-to-End Test Results

- **Primary intent:** [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md)
- **Tested committed head:** `55cbdea6a492e6b958f92fd9e6286f14bad737cb`
- **Actual-process CLI regression:** pass, 6/6
- **Exact-head hosted quality regression:** pass
- **Product concurrency E2E:** not yet possible; unlocked by [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md)

## Product concurrency E2E boundary

INT-0009's product-level competing-observer journey is not executable through
the current adapter. The version 1 CLI accepts one complete scenario, creates
one aggregate in that process's memory, executes that request's operations in
order, emits one response, and exits. It provides no operation for retrieving
the same aggregate in a later process, no shared durable state against which two
independent clients can retain the same observation, and no revision token or
conditioned mutation in its request/response protocol. A second CLI invocation
therefore creates a different in-memory aggregate rather than acting as a stale
writer against the first invocation's aggregate.

Expanding that deliberately one-shot protocol only to manufacture a test would
violate Sprint 7's compatibility boundary. The required E2E becomes possible
when INT-0010 realizes an adapter-owned, versioned durable boundary that can
create and retrieve one stable unit across separate requests, expose its current
revision, and atomically preserve durable state when a conditional mutation is
stale. A real test can then let two clients observe revision `n`, accept the
first client's mutation to `n + 1`, reject the second client's stale command,
retrieve the unchanged committed aggregate, refresh, and continue. INT-0010 is
still `proposed`; no mock store, speculative service, or altered CLI protocol is
introduced in this sprint.

## Existing actual-process CLI regression

- **Named check:** `test_existing_cli_actual_process_contract_is_unchanged`
- **Command:** `cargo +stable test -p cubikan-cli --test cli_e2e`
- **Result:** pass — 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
- **Duration reported by Cargo:** 0.01 seconds
- **Mocks/stubs:** none; each test launches the Cargo-built `cubikan` executable

| Actual-process test | Expected process exit | Result |
|---------------------|-----------------------|--------|
| `test_cli_configure_create_transition_complete` | `0` | pass |
| `test_cli_generates_id_when_member_is_omitted` | `0` | pass |
| `test_cli_reports_explicit_null_id_with_exit_2` | `2` | pass |
| `test_cli_reports_malformed_request_with_exit_2` | `2` | pass |
| `test_cli_reports_lifecycle_rejection_with_exit_3` | `3` | pass |
| `test_cli_reports_oversized_request_with_exit_2` | `2` | pass |

The accepted-base-to-tested-head diff changes no file under
`crates/cubikan-cli/`, no workspace or crate manifest, no `Cargo.lock`, and no
`.github/workflows/ci.yml`. The exact tested head differs from the original
committed Build head `071341d1632ca6cfe363a334b33ba0b77209401e` in one file,
`crates/cubikan-core/tests/lifecycle.rs`, whose additional public-consumer
assertions close Test critic concern C-001. Together with the six real
child-process results, this is regression evidence that the one-shot CLI's
success, request-rejection, lifecycle-rejection, and bounded-input exit
contracts did not drift. It is not revision proof: the CLI does not expose
`IntentUnitRevision`, accept an expected revision, call the new guarded methods,
or coordinate two observations of one aggregate.

## Exact-head hosted quality regression

- **Named check:** `test_hosted_sprint_seven_quality_run_succeeds`
- **Run:** [31301197841](https://github.com/crussella0129/CubiKan/actions/runs/31301197841)
- **Job:** [93214154471](https://github.com/crussella0129/CubiKan/actions/runs/31301197841/job/93214154471)
- **Result:** pass

Three observations identify the same committed tested revision:

| Provenance observation | SHA | Result |
|------------------------|-----|--------|
| Local committed tested `HEAD` | `55cbdea6a492e6b958f92fd9e6286f14bad737cb` | match |
| Local remote-tracking `origin/dev` | `55cbdea6a492e6b958f92fd9e6286f14bad737cb` | match |
| GitHub run and job API `head_sha` | `55cbdea6a492e6b958f92fd9e6286f14bad737cb` | match |

| Hosted assertion | Observed value | Result |
|------------------|----------------|--------|
| Workflow / run number | `Rust CI` / `14` | pass |
| Trigger | `push` on `dev` | pass |
| Run attempt / previous attempt | `1` / none | pass |
| Run status / conclusion | `completed` / `success` | pass |
| Run interval | `2026-08-09T07:27:45Z`–`2026-08-09T07:28:18Z` | pass |
| Sole job | `Rust quality gate` (ID `93214154471`) on `ubuntu-latest` | pass |
| Job status / conclusion | `completed` / `success` | pass |
| Job interval | `2026-08-09T07:27:48Z`–`2026-08-09T07:28:17Z` | pass |
| Configured timeout | 15 minutes; observed job completed in 29 seconds | pass |

The setup, pinned repository checkout, current-stable Rust installation, `Check
formatting`, `Run Clippy`, `Check workspace`, `Run workspace tests`, `Run
workspace doctests`, post-checkout, and completion steps each reported
`completed` / `success`. This was one real push run at the exact tested SHA. It
used attempt 1, had no prior attempt or retry, and the checked-in workflow
declares no dependency or build-cache step.

## External, flake, and claim boundary

No mock or stub supports the actual-process or hosted results. The hosted gate
crosses the real GitHub Actions service, its floating `ubuntu-latest` runner
image, a pinned `actions/checkout` action, Rustup's moving `stable` toolchain,
and crates.io index/download availability. The repository does not control
those services or the future contents of the floating runner and toolchain, so
availability and reproducibility remain external flake boundaries. Absence of a
workflow cache step does not claim that GitHub's runner infrastructure performs
no internal caching.

The hosted success is a quality and delivery regression oracle, not a product
E2E result and not independent evidence of INT-0009's competing-observer
semantics. One successful Linux run does not establish ongoing CI availability,
deterministic future runner or Rust versions, an MSRV, Windows or macOS support,
load or performance characteristics, database isolation, durable atomicity,
cross-unit atomicity, locking, delivery idempotency, security or supply-chain
certification, release behavior, or deployment behavior. A later pull-request
run is a separate handoff checkpoint and is not this exact-tested-head oracle.
