# Sprint 7 Unit and Repository Verification

- **Primary intent:** [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md)
- **Preserved intents:** [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) and [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md), with unchanged regressions for realized INT-0003 through INT-0006
- **Accepted base:** `ba635f6bc61b8194e65ff4761392993765f487c1`
- **Build task/ledger head:** `071341d1632ca6cfe363a334b33ba0b77209401e`
- **Tested critic-response candidate:** `55cbdea6a492e6b958f92fd9e6286f14bad737cb`
- **Local stable toolchain:** `rustc 1.95.0`; `cargo 1.95.0`
- **Conclusion:** pass; every locked unit, documentation, repository, and local-quality check below passed at the exact tested candidate

The public lifecycle checks live in `crates/cubikan-core/tests/lifecycle.rs`
because they exercise only the crate-root exports available to a consumer. The
private checked-increment seam is exercised by one library unit test. This
artifact records the locked T-701, T-702, and T-704 unit/repository evidence;
validated restoration under T-703 and the public/hosted integration boundary
are detailed in [integration-tests.md](integration-tests.md). No mock, stub,
clock, network, database, or implementation-mirroring test harness was added.

## Locked unit and repository EARS checks

### T-701 revision state and unconditioned mutation

#### `test_intent_unit_starts_at_zero_revision` — T-701-E1

- **Arrangement:** Construct the public three-phase `linear_unit` fixture and
  inspect its revision, status, phase, history, numeric representation, and
  display representation before any lifecycle command.
- **SHALL assertion and observation:** A new unit shall expose
  `IntentUnitRevision::INITIAL`, equal `IntentUnitRevision::new(0)`, return
  numeric value `0`, and display as `"0"`. It remained active in `queued` with
  empty history.
- **Result:** pass.

#### `test_unconditioned_mutations_advance_revision_once_per_record` — T-701-E2

- **Arrangement:** Build a caller-declared workflow containing forward
  `queued -> doing`, reverse `doing -> queued`, self `queued -> queued`, and
  final `queued -> done` edges, then invoke those four transitions followed by
  eligible completion. Snapshot revision and history length before every
  command.
- **SHALL assertion and observation:** Every accepted forward, reverse,
  self-edge, final transition, and completion shall advance revision from `n`
  to `n + 1` exactly once, append exactly one existing lifecycle record, and
  make the latest one-based record sequence numerically equal the new revision.
  All five mutations met that invariant, and the final aggregate was completed.
- **Result:** pass.

#### `test_failed_unconditioned_commands_preserve_revision_and_aggregate` — T-701-E3

- **Arrangement:** Against an active unit, separately submit an unknown target,
  an undeclared `queued -> done` edge, and ineligible completion. Then reach and
  complete `done`, and separately attempt a post-completion transition and a
  repeated completion. Clone the whole aggregate immediately before every
  rejected command.
- **SHALL assertion and observation:** Each rejection shall retain the exact
  pre-Sprint error—`UnknownTarget`, `NotAllowed`, `PhaseNotEligible`, transition
  `AlreadyCompleted`, or completion `AlreadyCompleted`—and leave the full
  aggregate, including revision, unchanged. Every exact error and derived-`Eq`
  aggregate comparison passed.
- **Result:** pass.

#### `test_revision_checked_next_rejects_maximum_without_wrap` — T-701-E4

- **Arrangement:** Invoke the private checked-successor seam on
  `IntentUnitRevision::new(u64::MAX)` in the core library test module.
- **SHALL assertion and observation:** The helper shall report no successor and
  shall not wrap to `IntentUnitRevision::INITIAL`; it returned `None` and was
  explicitly unequal to `Some(INITIAL)`.
- **Result:** pass.

### T-702 guarded command and precedence matrix

#### `test_guarded_transition_returns_exact_successor_revision` — T-702-E1

- **Arrangement:** Build a guarded workflow with declared forward
  `queued -> doing`, reverse `doing -> queued`, and self `queued -> queued`
  edges. Snapshot immutable identity, species, and owned workflow once; before
  each guarded command, record the current revision and copy the complete
  history prefix, then verify that the unit occupies the edge's expected
  source phase.
- **SHALL assertion and observation:** A current expected revision shall
  preserve normal transition behavior for forward, reverse, and declared
  self-edges. The three commands returned exact successor revisions `1`, `2`,
  and `3`; after each, the accessor agreed, history grew by exactly one, the
  complete prior history remained an equal prefix, and the appended transition
  record had the exact sequence, source, and target. Identity, species, owned
  workflow, and active status were unchanged, while phase matched each target.
- **Result:** pass.

#### `test_guarded_completion_returns_exact_successor_revision` — T-702-E2

- **Arrangement:** Reach completion-eligible `done` using two accepted
  transitions; snapshot identity, species, owned workflow, final phase,
  revision `2`, and a copy of the complete two-record history before invoking
  `complete_if_revision` with the current value.
- **SHALL assertion and observation:** Current-revision completion shall retain
  normal completion behavior and return one exact successor after one record
  and one revision advance. It returned revision `3`; the accessor agreed,
  identity, species, owned workflow, and final phase remained unchanged, the
  complete prior history remained an equal prefix, status became completed,
  and the sole new record was a sequence-`3` completion record for `done`.
- **Result:** pass.

#### `test_revision_conflict_exposes_expected_and_actual` — T-702-E3

- **Arrangement:** Give two observers revision `0`, let the first guarded
  transition commit revision `1`, and submit the second observer's stale
  transition.
- **SHALL assertion and observation:** The stale command shall return a typed
  `RevisionConflict` containing expected `0` and actual `1`. The public
  accessors returned those exact values, the wrapper's `Error::source`
  downcast to the stored conflict, and both conflict and wrapper displayed
  `revision conflict: expected 0, actual 1`.
- **Result:** pass.

#### `test_competing_observers_reject_second_command_and_allow_refresh` — T-702-E3

- **Arrangement:** Copy revision `0` for two observers, let the first move the
  unit to `doing`, clone the aggregate, reject the second observer's stale
  `doing -> done` request, then refresh from `RevisionConflict::actual()` and
  resubmit.
- **SHALL assertion and observation:** The losing command shall return the
  expected-`0`/actual-`1` conflict without mutation, while a caller that
  explicitly refreshes may issue a new decision. Full aggregate equality held
  across rejection; the refreshed request committed revision `2`, moved to
  `done`, and left exactly two records.
- **Result:** pass.

#### `test_stale_revision_rejects_otherwise_valid_command_atomically` — T-702-E4

- **Arrangement:** First make `doing -> done` domain-valid at actual revision
  `1` while retaining stale revision `0`; separately make completion valid at
  `done` revision `2` while retaining stale revision `1`. Clone each aggregate
  before the conditioned command.
- **SHALL assertion and observation:** A stale revision shall reject even an
  otherwise valid transition or completion before lifecycle evaluation and
  leave every aggregate field unchanged. Both paths returned typed conflicts
  with their exact expected/actual revisions, and both full-aggregate equality
  checks passed.
- **Result:** pass.

#### `test_stale_revision_precedes_transition_errors_atomically` — T-702-E4

- **Arrangement:** Advance a linear unit from revision `0` to revision `1` in
  `doing`, then table-test the retained stale revision with unknown target
  `missing` and known but undeclared reverse target `queued`; snapshot the
  aggregate for each case.
- **SHALL assertion and observation:** Revision comparison shall precede both
  target-existence and edge validation. Each case returned conflict expected
  `0`/actual `1`, never a wrapped `UnknownTarget` or `NotAllowed`, and preserved
  the complete aggregate.
- **Result:** pass.

#### `test_stale_revision_precedes_completion_errors_atomically` — T-702-E4

- **Arrangement:** Retain revision `0`, advance to completion-ineligible
  `doing` at revision `1`, clone the aggregate, and invoke
  `complete_if_revision` with the stale observation.
- **SHALL assertion and observation:** Revision comparison shall occur before
  completion eligibility. The method returned conflict expected `0`/actual
  `1`, not `PhaseNotEligible`, and the full aggregate remained equal to its
  snapshot.
- **Result:** pass.

#### `test_stale_revision_precedes_terminal_errors_atomically` — T-702-E4

- **Arrangement:** Reach `done` at revision `2`, retain that revision, complete
  the unit at revision `3`, and then use the stale value for transitions to
  both known `queued` and unknown `missing`, plus repeated completion. Snapshot
  before every rejection.
- **SHALL assertion and observation:** Stale comparison shall precede terminal,
  target, and repeated-completion validation. All three commands returned
  conflict expected `2`/actual `3`, rather than terminal or unknown-target
  errors, and every whole-aggregate comparison passed.
- **Result:** pass.

#### `test_current_revision_preserves_domain_errors_atomically` — T-702-E5

- **Arrangement:** With the revision current, separately request unknown
  `missing`, undeclared `queued -> done`, and ineligible completion from
  `queued`; on a completed unit, separately request a transition to `missing`
  and repeated completion. Clone before each request and inspect the wrapper's
  source.
- **SHALL assertion and observation:** Current expectations shall preserve
  normal domain evaluation and return separate wrappers containing the exact
  original `UnknownTarget`, `NotAllowed`, `PhaseNotEligible`, transition
  `AlreadyCompleted`, and completion `AlreadyCompleted` values. Each wrapper's
  source downcast to that stored typed error, and every full aggregate remained
  unchanged.
- **Result:** pass.

The full-aggregate assertions above cover identity, species, owned workflow,
phase, status, history, and revision through the aggregate's derived equality.
They therefore test atomicity without reconstructing production logic in a
mock.

### T-704 documentation and repository boundary

#### `test_public_revision_doctest_compiles` — T-704-E1

- **Arrangement:** Run `cargo +stable test --doc --workspace` against the
  crate-root example, which imports the revision surface solely from
  `cubikan_core`, observes a new unit, performs two guarded transitions and
  guarded completion, and checks returned/accessor revisions.
- **SHALL assertion and observation:** The public example shall compile and
  pass while demonstrating initial revision `0`, successful conditioned
  commands, an asserted first returned/accessor revision of `1`, reuse of the
  returned intermediate token, a final returned/accessor revision of `3`,
  three lifecycle records, and completed status. The one core doctest passed;
  `cubikan-cli` defines no doctest.
- **Result:** pass.

#### `test_readme_defines_revision_contract_and_nonclaims` — T-704-E2

- **Arrangement:** Inspect the root README's core-model,
  revision-conditioned-mutation, adapter, and explicit-exclusion sections
  together with the crate-level documentation at the exact tested candidate.
- **SHALL assertion and observation:** A consumer shall be told that revision
  starts at aggregate-local `0`, every accepted conditioned or unconditioned
  lifecycle mutation advances it exactly once, stale comparison occurs before
  domain evaluation, typed conflicts expose expected/actual values for an
  explicit reject-refresh-decide flow, and current expectations preserve typed
  lifecycle errors. The docs distinguish zero-based revision from one-based
  record sequence and explicitly deny clocks/global ordering, database
  isolation or durable compare-and-set, locking, cross-unit atomicity,
  idempotent delivery/retry safety, actor policy, and a revision-aware CLI v1.
  Every required statement and nonclaim was present.
- **Result:** pass.

#### `test_sprint_scope_preserves_cli_protocol_and_dependencies` — T-704-E3

- **Arrangement:** Compare accepted base
  `ba635f6bc61b8194e65ff4761392993765f487c1` to tested candidate
  `55cbdea6a492e6b958f92fd9e6286f14bad737cb`; inspect every changed path and
  diff, run `git diff --check`, inspect workspace metadata and the complete
  normal dependency tree, and perform quiet byte-level comparisons for all
  manifests, `Cargo.lock`, CI, the entire CLI, and unaffected core modules.
- **SHALL assertion and observation:** The sprint shall introduce no manifest,
  lockfile, dependency, CI/workflow, CLI protocol/request/response/error-code,
  fixture, persistence, transport, clock, actor, database, retry/lease, or
  unrelated product-policy change. The exact diff contains only the intended
  core revision implementation, strengthened lifecycle tests, docs, and Sprint
  7 Book/ledger evidence. The critic-response delta from the task/ledger head
  changes only `crates/cubikan-core/tests/lifecycle.rs`; protected paths were
  byte-identical, metadata still reported two Rust 2024 crates, the normal
  dependency tree was unchanged, and `git diff --check` passed.
- **Result:** pass.

#### `test_book_v2_validation` — T-704-E4

- **Arrangement:** Run the installed `check-book.sh` at the tested candidate,
  inspect INT-0009's state/evidence and Summary entry, then separately run a
  read-only `pathlib`/regular-expression Markdown resolver over filesystem
  targets and GitHub-style heading fragments.
- **SHALL assertion and observation:** Sprint 7 Book state shall be valid and
  every new local Markdown link shall resolve. The schema validator returned
  exactly `check-book: valid v2 Book (12 intent chapters)` with INT-0009 active,
  legally linked to its work evidence, and reachable from `docs/SUMMARY.md`.
  The separate resolver inspected 103 Markdown files, 668 Markdown links, 618
  local links, and 8 fragment targets with 0 errors.
- **Result:** pass.

`check-book.sh` is the Book v2 intent-schema, uniqueness, state, and evidence
oracle; it does not prove Markdown target or fragment reachability. The
read-only link resolver is the reachability oracle; it does not validate intent
lifecycle state. Neither result is used as a substitute for the other.

## Exact accepted-base diff evidence

The 17 accepted-base-to-tested-candidate changed paths were:

```text
README.md
crates/cubikan-core/src/intent_unit.rs
crates/cubikan-core/src/lib.rs
crates/cubikan-core/tests/lifecycle.rs
crates/cubikan-core/tests/serialization.rs
docs/SUMMARY.md
docs/intents/INT-0009-revisioned-lifecycle-commands.md
docs/sprints/s7/sprint-meta.md
docs/sprints/s7/sprint-plans/build-plan.md
docs/sprints/s7/sprint-plans/critique.md
docs/sprints/s7/sprint-plans/test-plan.md
docs/sprints/s7/sprint-research/research-report.md
docs/sprints/s7/sprint-tests/e2e-tests.md
docs/sprints/s7/sprint-tests/integration-tests.md
docs/sprints/s7/sprint-tests/test-report.md
docs/sprints/s7/sprint-tests/unit-tests.md
docs/work/completed-tasks.md
```

The four Sprint 7 test-artifact paths are initialized Build-phase placeholders;
this Test evidence populates them after the tested candidate SHA. `docs/work/tasks.md`
is empty and byte-identical to the accepted base after all four task commits.
There is no changed path under `crates/cubikan-cli`, `.github`, any Cargo
manifest or lockfile, or the core ID, vocabulary, or workflow modules.

Relative to the original task/ledger head, the tested critic-response candidate
changes only `crates/cubikan-core/tests/lifecycle.rs`. That test-only delta
strengthens the guarded success oracle; it changes no product source, Book
chapter, documentation, protocol, manifest, dependency, or workflow.

## Focused and canonical local gates

The focused core commands and five canonical commands ran at the exact clean
tested candidate. All completed successfully:

| Gate | Command | Exact result |
|------|---------|--------------|
| Core library | `cargo +stable test -p cubikan-core --lib` | pass; 44 passed, 0 failed |
| Public lifecycle | `cargo +stable test -p cubikan-core --test lifecycle` | pass; 16 passed, 0 failed |
| Validated serialization | `cargo +stable test -p cubikan-core --test serialization` | pass; 5 passed, 0 failed |
| Metadata | `cargo +stable metadata --no-deps --format-version 1` | pass; exactly `cubikan-core` and `cubikan-cli`, both Rust 2024 |
| Dependency tree | `cargo +stable tree --workspace --edges normal` | pass; identical to accepted base |
| Formatting | `cargo +stable fmt --all -- --check` | pass |
| Clippy | `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings` | pass; zero warnings |
| Warnings-denied check | `RUSTFLAGS="-D warnings" cargo +stable check --workspace --all-targets` | pass; zero warnings |
| All-target tests | `cargo +stable test --workspace --all-targets` | pass; 116 passed, 0 failed |
| Doctests | `cargo +stable test --doc --workspace` | pass; 1 core doctest, 0 CLI doctests |
| Whitespace/error diff | `git diff --check ba635f6bc61b8194e65ff4761392993765f487c1...55cbdea6a492e6b958f92fd9e6286f14bad737cb` | pass |
| Book v2 | installed `check-book.sh` | pass; 12 intent chapters |
| Markdown reachability | read-only path-and-fragment resolver | pass; 618 local links and 8 fragments, 0 errors |

## Exact all-target suite breakdown

| Suite | Passed | Failed | Ignored / measured / filtered |
|-------|-------:|-------:|-------------------------------:|
| `cubikan-cli` library unit tests | 32 | 0 | 0 |
| `cubikan-cli` actual-process E2E tests | 6 | 0 | 0 |
| `cubikan-cli` public-runner integration tests | 13 | 0 | 0 |
| `cubikan-core` library unit tests | 44 | 0 | 0 |
| `cubikan-core` lifecycle integration tests | 16 | 0 | 0 |
| `cubikan-core` serialization integration tests | 5 | 0 | 0 |
| **All-target total** | **116** | **0** | **0** |

The `cubikan` binary target contains 0 unit tests. Workspace doctests contain 1
passing core test and 0 CLI tests. No test failed, was ignored, measured, or
filtered.

## Public/core baseline and preserved behavior

| Intent/boundary | Executed evidence | Result |
|-----------------|-------------------|--------|
| [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) | The 12 new public lifecycle checks, private maximum-revision check, four revision-focused serialization checks, and public doctest cover initial state, exactly-once advance, stale-first conflicts, current-error preservation, atomicity, validated restore, and documentation scope. | pass at the local unit/repository boundary |
| [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) | All 44 core unit, 16 lifecycle, and 5 serialization tests retained opaque identity, caller-declared topology, owned workflow snapshots, declared rework/self edges, exact domain errors, ordered history, terminal completion, atomic rejection, and semantic replay. Named retained oracles include `test_intent_lifecycle_create_transition_complete`, `test_failed_operations_are_atomic_and_recoverable`, `test_explicit_rework_cycle_is_honored`, `test_active_intent_semantic_round_trip`, and `test_serialization_rejects_inconsistent_lifecycle_history`. | pass; realized behavior preserved |
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | The unchanged CLI contributed 32 library, 13 public-runner, and 6 actual-process passes. Existing strict v1 setup/lifecycle, one-shot execution, typed responses, and exit behavior remained green; the CLI source and protocol fixtures are byte-identical to the accepted base. | pass; realized behavior preserved |
| Realized INT-0003–INT-0006 regressions | The same 51 CLI tests retained the 1 MiB ceiling and precedence, exactly-once supplied-writer flush, unchanged five-gate automation contract, and omitted-versus-present ID behavior. No affected implementation or policy path changed. | pass; unchanged regression baseline |

The current CLI's actual-process tests are regression evidence only. Because
the CLI creates a fresh in-memory aggregate per process and exposes no revision
token, they are not claimed as product E2E proof for competing revision-aware
clients.

## Build and task provenance

| Task | Implementation commit | Ledger/evidence follow-up |
|------|-----------------------|---------------------------|
| T-701 | `380e2285ee1a25b37b12a09612dce32784a30319` | `9ee882230ddff8e5e13a169b35cc31c8b0a543c4` |
| T-702 | `536b83ee3a2c58781a937c7e876e85a5c315a0a4` | `f563763b2c9007250dc45b5dad469f117608478b` |
| T-703 | `6f894980523856ec5b06d2aa5577b6e74733cef5` | `54bec8c71fe67ba3fb7524d1ea16f6db88912604` |
| T-704 | `8124ffa76286b7df7ab30af3a1c0d924c9e32c64` | `071341d1632ca6cfe363a334b33ba0b77209401e` |

Every listed commit descends from the accepted base. The final T-704 ledger
follow-up is the original Build task/ledger head.

### Post-Build critic response

| Commit | Scope | Verification effect | Product effect |
|--------|-------|---------------------|----------------|
| `55cbdea6a492e6b958f92fd9e6286f14bad737cb` | Test-only change to `crates/cubikan-core/tests/lifecycle.rs` | Strengthens T-702-E1 with guarded forward, reverse, and self-edge record/prefix/immutable-field assertions and T-702-E2 with completion prefix/immutable-field assertions; exact focused and workspace gates remain green, and hosted [run 31301197841](https://github.com/crussella0129/CubiKan/actions/runs/31301197841) / [job 93214154471](https://github.com/crussella0129/CubiKan/actions/runs/31301197841/job/93214154471) passed the candidate's 116 tests plus 1 doctest. | none |

This distinct critic-response commit is the exact tested candidate. Test
artifacts written after that SHA record evidence rather than retroactively
claiming to be candidate product code.

## Reproduction commands

Run from the repository root with the tested candidate checked out and a clean
worktree:

```sh
git rev-parse HEAD
git status --porcelain=v1
git merge-base --is-ancestor ba635f6bc61b8194e65ff4761392993765f487c1 55cbdea6a492e6b958f92fd9e6286f14bad737cb
git merge-base --is-ancestor 071341d1632ca6cfe363a334b33ba0b77209401e 55cbdea6a492e6b958f92fd9e6286f14bad737cb
git diff --name-only ba635f6bc61b8194e65ff4761392993765f487c1...55cbdea6a492e6b958f92fd9e6286f14bad737cb
git diff --name-only 071341d1632ca6cfe363a334b33ba0b77209401e...55cbdea6a492e6b958f92fd9e6286f14bad737cb
git diff --check ba635f6bc61b8194e65ff4761392993765f487c1...55cbdea6a492e6b958f92fd9e6286f14bad737cb
git diff --quiet ba635f6bc61b8194e65ff4761392993765f487c1...55cbdea6a492e6b958f92fd9e6286f14bad737cb -- Cargo.toml Cargo.lock .github crates/cubikan-cli crates/cubikan-core/Cargo.toml crates/cubikan-core/src/id.rs crates/cubikan-core/src/vocabulary.rs crates/cubikan-core/src/workflow.rs
cargo +stable test -p cubikan-core --lib
cargo +stable test -p cubikan-core --test lifecycle
cargo +stable test -p cubikan-core --test serialization
cargo +stable metadata --no-deps --format-version 1
cargo +stable tree --workspace --edges normal
cargo +stable fmt --all -- --check
cargo +stable clippy --workspace --all-targets --all-features -- -D warnings
RUSTFLAGS="-D warnings" cargo +stable check --workspace --all-targets
cargo +stable test --workspace --all-targets
cargo +stable test --doc --workspace
bash /mnt/c/Users/charl/.codex/plugins/cache/sprint-loops/sprint-loop/local/skills/sprint-loop/scripts/check-book.sh
```

The Markdown reachability result additionally used the separate one-off,
read-only filesystem-target and GitHub-heading-fragment resolver described in
T-704-E4; it introduced no repository file or dependency.
