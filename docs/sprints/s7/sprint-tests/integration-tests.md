# Sprint 7 Integration Test Results

- **Primary intent:** [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md)
- **Preserved intents:** [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) and [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Build ledger head:** `071341d1632ca6cfe363a334b33ba0b77209401e`
- **Tested candidate head:** `55cbdea6a492e6b958f92fd9e6286f14bad737cb` (the critic-response test commit after the Build ledger head)
- **Local commands:** `cargo +stable test -p cubikan-core --test lifecycle`, `cargo +stable test -p cubikan-core --test serialization`, and `cargo +stable test --workspace --all-targets`
- **Public core result:** 16 passed, 0 failed, 0 ignored, 0 measured, 0 filtered out
- **Serialization result:** 5 passed, 0 failed, 0 ignored, 0 measured, 0 filtered out
- **Hosted result:** 116 all-target tests and 1 doctest passed on the exact tested candidate head
- **Conclusion:** pass

## T-701 and T-702 public core lifecycle contract

The external `crates/cubikan-core/tests/lifecycle.rs` target imports the new
revision token, conflict, conditioned errors, and lifecycle API only through
the public `cubikan_core` crate root. The locked plan's four high-level public
contract checks are realized by the following finer-grained compiled tests,
which avoid duplicate alias tests while retaining every planned assertion:

| Locked public check | Concrete compiled evidence |
|---------------------|----------------------------|
| `test_public_unconditioned_lifecycle_advances_revision` | `test_intent_unit_starts_at_zero_revision`, `test_unconditioned_mutations_advance_revision_once_per_record`, `test_failed_unconditioned_commands_preserve_revision_and_aggregate` |
| `test_public_revisioned_lifecycle_accepts_current_observer` | `test_guarded_transition_returns_exact_successor_revision`, `test_guarded_completion_returns_exact_successor_revision` |
| `test_public_revisioned_lifecycle_rejects_competing_observer_atomically` | `test_revision_conflict_exposes_expected_and_actual`, `test_competing_observers_reject_second_command_and_allow_refresh`, `test_stale_revision_rejects_otherwise_valid_command_atomically`, and the three stale-precedence matrix tests |
| `test_public_current_revision_preserves_error_taxonomy` | `test_current_revision_preserves_domain_errors_atomically` |

### Exact Sprint 7 lifecycle assertions

| Named test | EARS | Arrangement and assertions | Result |
|------------|------|----------------------------|--------|
| `test_intent_unit_starts_at_zero_revision` | T-701-E1 | Construct a public linear unit; require `revision() == IntentUnitRevision::INITIAL == IntentUnitRevision::new(0)`, numeric value and display text `0`, active status, phase `queued`, and empty history. | pass |
| `test_unconditioned_mutations_advance_revision_once_per_record` | T-701-E2 | Execute forward, reverse, declared self-edge, final transition, and completion. After each success require revision `n + 1`, history length `+1`, and latest one-based record sequence equal to the numeric revision; require terminal status after completion. | pass |
| `test_failed_unconditioned_commands_preserve_revision_and_aggregate` | T-701-E3 | Exercise unknown target, undeclared edge, ineligible completion, post-completion transition, and repeated completion; require each exact original error and full aggregate equality against its pre-command clone. | pass |
| `test_guarded_transition_returns_exact_successor_revision` | T-702-E1 | Submit each current revision across declared forward `queued -> doing`, reverse `doing -> queued`, and self `queued -> queued` edges. For every success, require the returned/accessor revision and appended record sequence to advance exactly once (`1`, `2`, `3`), the record's exact `from`/`to` endpoints, preservation of the complete prior-history prefix, target phase with active status, and unchanged ID, species, and owned workflow. | pass |
| `test_guarded_completion_returns_exact_successor_revision` | T-702-E2 | Reach `done` at revision `2`, snapshot the complete prior history and immutable provenance, then complete with that observation. Require returned/accessor revision `3`, exactly one appended completion record at sequence `3` for the preserved final phase, the complete prior-history prefix unchanged, completed status, and unchanged ID, species, owned workflow, and aggregate phase. | pass |
| `test_revision_conflict_exposes_expected_and_actual` | T-702-E3 | Copy observation `0`, let one observer transition to revision `1`, then reuse `0`; require `RevisionedTransitionError::Conflict`, expected `0`, actual `1`, conflict as the error source, and exact display `revision conflict: expected 0, actual 1`. | pass |
| `test_competing_observers_reject_second_command_and_allow_refresh` | T-702-E3 | Let the first copied observation commit, require the second to conflict with full aggregate equality, refresh from `conflict.actual()`, and require the next valid transition to commit revision `2`, phase `done`, and history length `2`. | pass |
| `test_stale_revision_rejects_otherwise_valid_command_atomically` | T-702-E4 | Test an otherwise valid transition and an otherwise eligible completion with older observations; require typed transition/completion conflicts containing exact expected/actual revisions and full aggregate equality. | pass |
| `test_stale_revision_precedes_transition_errors_atomically` | T-702-E4 | At actual revision `1`, table-test stale revision `0` against unknown target `missing` and known but undeclared target `queued`; require conflict rather than `UnknownTarget` or `NotAllowed`, plus full equality. | pass |
| `test_stale_revision_precedes_completion_errors_atomically` | T-702-E4 | In ineligible phase `doing` at actual revision `1`, submit stale revision `0`; require conflict rather than `PhaseNotEligible` and full equality. | pass |
| `test_stale_revision_precedes_terminal_errors_atomically` | T-702-E4 | Complete at actual revision `3`, then use stale revision `2` for a known transition target, an unknown transition target, and repeated completion; require conflict before terminal or target evaluation and full equality after every case. | pass |
| `test_current_revision_preserves_domain_errors_atomically` | T-702-E5 | With current observations, require wrappers around exact `UnknownTarget`, `NotAllowed`, `PhaseNotEligible`, transition `AlreadyCompleted`, and completion `AlreadyCompleted` values; require each original domain error as `source()` and full aggregate equality. | pass |

The private checked-successor boundary required by T-701-E4 is recorded in the
unit ledger rather than duplicated here. Hosted and local all-target execution
both passed `test_revision_checked_next_rejects_maximum_without_wrap`.

### Exact guarded-success semantics

The tightened T-702 success evidence responds directly to critic concern
C-001. Each accepted command compares immutable provenance to its pre-command
snapshot and proves that the existing history is an element-for-element prefix
of the post-command history before inspecting the single new record.

| Command | Current revision | Returned/accessor revision | Appended record | Aggregate after success | Preserved state | Result |
|---------|-----------------:|---------------------------:|-----------------|-------------------------|-----------------|--------|
| declared forward `queued -> doing` | `0` | `1` | transition sequence `1`, exact `from = queued`, `to = doing` | phase `doing`, active | ID, species, workflow, empty history prefix | pass |
| declared reverse `doing -> queued` | `1` | `2` | transition sequence `2`, exact `from = doing`, `to = queued` | phase `queued`, active | ID, species, workflow, first-record history prefix | pass |
| declared self `queued -> queued` | `2` | `3` | transition sequence `3`, exact `from = queued`, `to = queued` | phase `queued`, active | ID, species, workflow, two-record history prefix | pass |
| completion in `done` after the fixture's two transitions | `2` | `3` | completion sequence `3`, exact final phase `done` | phase `done`, completed | ID, species, workflow, final phase, two-record history prefix | pass |

### Exact lifecycle rejection matrix

Every row clones and compares the complete derived-`Eq` `IntentUnit`, so an
atomic pass covers identity, species, owned workflow, phase, status, history,
and revision rather than only the field most directly affected.

| Revision relation | Command condition | Required public result | Aggregate | Result |
|-------------------|-------------------|------------------------|-----------|--------|
| unconditioned | unknown target `missing` | `TransitionError::UnknownTarget { target: "missing" }` | unchanged | pass |
| unconditioned | known undeclared edge `queued -> done` | `TransitionError::NotAllowed { from: "queued", to: "done" }` | unchanged | pass |
| unconditioned | completion from ineligible `queued` | `CompletionError::PhaseNotEligible { phase: "queued" }` | unchanged | pass |
| unconditioned | transition after completion | `TransitionError::AlreadyCompleted` | unchanged | pass |
| unconditioned | repeated completion | `CompletionError::AlreadyCompleted` | unchanged | pass |
| stale | otherwise valid `doing -> done` | transition `Conflict { expected: 0, actual: 1 }` | unchanged | pass |
| stale | otherwise eligible completion from `done` | completion `Conflict { expected: 1, actual: 2 }` | unchanged | pass |
| stale | unknown target `missing` | transition `Conflict { expected: 0, actual: 1 }` before `UnknownTarget` | unchanged | pass |
| stale | known undeclared edge `doing -> queued` | transition `Conflict { expected: 0, actual: 1 }` before `NotAllowed` | unchanged | pass |
| stale | completion from ineligible `doing` | completion `Conflict { expected: 0, actual: 1 }` before `PhaseNotEligible` | unchanged | pass |
| stale terminal | known transition target | transition `Conflict { expected: 2, actual: 3 }` before `AlreadyCompleted` | unchanged | pass |
| stale terminal | unknown transition target | transition `Conflict { expected: 2, actual: 3 }` before both terminal and target evaluation | unchanged | pass |
| stale terminal | repeated completion | completion `Conflict { expected: 2, actual: 3 }` before `AlreadyCompleted` | unchanged | pass |
| current | unknown target `missing` | `RevisionedTransitionError::Transition(UnknownTarget)` with exact source | unchanged | pass |
| current | known undeclared edge `queued -> done` | `RevisionedTransitionError::Transition(NotAllowed)` with exact source | unchanged | pass |
| current | completion from ineligible `queued` | `RevisionedCompletionError::Completion(PhaseNotEligible)` with exact source | unchanged | pass |
| current terminal | transition to unknown `missing` | `RevisionedTransitionError::Transition(AlreadyCompleted)` with exact source, preserving terminal-before-target precedence | unchanged | pass |
| current terminal | repeated completion | `RevisionedCompletionError::Completion(AlreadyCompleted)` with exact source | unchanged | pass |

### Preserved public lifecycle regressions

The same target passed all existing public-consumer journeys:

| Named test | Preserved assertion | Result |
|------------|---------------------|--------|
| `test_custom_workflow_configuration_composes_domain_values` | Arbitrary caller vocabulary, exact directed edge, and completion eligibility remain configurable. | pass |
| `test_intent_lifecycle_create_transition_complete` | Identity, species, workflow, ordered records `1..=3`, terminal status, and post-completion rejection remain unchanged. | pass |
| `test_failed_operations_are_atomic_and_recoverable` | Existing invalid operations preserve full state and do not prevent a later valid transition/completion journey. | pass |
| `test_explicit_rework_cycle_is_honored` | Declared forward/reverse topology and five-record completion journey remain honored. | pass |

## T-703 validated serialization contract

The external `crates/cubikan-core/tests/serialization.rs` target uses public
core values with production Serde serialization and validated deserialization.
It passed all five named tests:

| Named test | EARS | Arrangement and assertions | Result |
|------------|------|----------------------------|--------|
| `test_revision_round_trip_preserves_active_and_completed_units` | T-703-E1 | Round-trip an active unit at revision `1` and a completed unit at revision `3`; require exact revision preservation and full semantic equality in both cases. | pass |
| `test_restored_unit_continues_from_exact_revision` | T-703-E1 | Restore an active unit at revision `1`, condition `doing -> done` on the restored token, and require committed/accessor revision `2`, history length `2`, sequence `2`, source `doing`, and target `done`. | pass |
| `test_restore_rejects_missing_or_mismatched_revision` | T-703-E2 | Remove revision; supply negative, fractional, string, or greater-than-`u64` representations; and supply lower `0` or higher `2` values against a valid revision-`1` history. Require deserialization failure for every independent case. | pass |
| `test_restore_rejects_history_phase_or_status_disagreement` | T-703-E3 | Keep plausible stored revision `3` while independently corrupting sequence, transition source, completion phase, final aggregate phase, or final status. Require validated restoration failure for every case. | pass |
| `test_restore_rejects_invalid_workflow_topology` | T-703-E3 / preserved INT-0001 | Replace the owned workflow initial phase with unknown `missing`; require deserialization failure through normal workflow validation. | pass |

### Exact restoration rejection matrix

| Serialized mutation | Other lifecycle state | Required result | Result |
|---------------------|-----------------------|-----------------|--------|
| omit `revision` | otherwise valid active revision-`1` unit | reject; no default or inference | pass |
| revision `-1` | otherwise valid | reject non-`u64` | pass |
| revision `1.5` | otherwise valid | reject non-`u64` | pass |
| revision string `"1"` | otherwise valid | reject non-`u64` | pass |
| revision `18446744073709551616` | otherwise valid | reject value above `u64::MAX` | pass |
| revision `0` | history replays to `1` | reject lower mismatch | pass |
| revision `2` | history replays to `1` | reject higher mismatch | pass |
| first sequence `9` | stored revision remains plausible `3` | reject sequence disagreement | pass |
| first transition source `doing` | stored revision remains plausible `3` | reject history continuity disagreement | pass |
| completion-record final phase `doing` | stored revision remains plausible `3` | reject completion phase disagreement | pass |
| final aggregate phase `doing` | stored revision remains plausible `3` | reject replay/final phase disagreement | pass |
| final aggregate status `Active` | stored revision remains plausible `3` | reject replay/final status disagreement | pass |
| workflow initial phase `missing` | otherwise valid active unit | reject invalid owned topology | pass |

No mocks, service stubs, fake stores, clocks, actors, locks, or network doubles
participate in the public lifecycle or restoration evidence. Fixtures provide
deterministic IDs and caller-declared workflows only; commands execute the real
aggregate, errors come from the exported core types, and `serde_json` exercises
the production format-neutral Serde implementations.

## `test_hosted_sprint_seven_quality_run_succeeds`

GitHub received the exact critic-response test commit after the Build ledger
head through a real push to `dev`.
The existing `Rust CI` workflow completed attempt 1 successfully with its sole
`Rust quality gate` job:

| Field | Observed value |
|-------|----------------|
| Run | [31301197841 — Rust CI](https://github.com/crussella0129/CubiKan/actions/runs/31301197841) |
| Run number | `14` |
| Event / branch | `push` / `dev` |
| Attempt | `1` |
| Run status / conclusion | `completed` / `success` |
| Run created | `2026-08-09T07:27:45Z` |
| Run updated | `2026-08-09T07:28:18Z` |
| Job | [93214154471 — Rust quality gate](https://github.com/crussella0129/CubiKan/actions/runs/31301197841/job/93214154471) |
| Job status / conclusion | `completed` / `success` |
| Job created / started / completed | `2026-08-09T07:27:46Z` / `2026-08-09T07:27:48Z` / `2026-08-09T07:28:17Z` |
| Job duration | `29s` |
| Run, job, checkout, and remote `dev` SHA | `55cbdea6a492e6b958f92fd9e6286f14bad737cb` |
| Workflow blob at the tested candidate (unchanged from the Build head) | `96420136d282ef93bb60b0607dffac1d28427a8d` |

### Hosted steps and commands

| Hosted step | Command or boundary | Status / conclusion |
|-------------|---------------------|---------------------|
| Set up job | GitHub-hosted runner initialization | `completed` / `success` |
| Check out repository | Pinned `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1` with `persist-credentials: false` | `completed` / `success` |
| Install stable Rust | `rustup toolchain install stable --profile minimal --component rustfmt,clippy` | `completed` / `success` |
| Check formatting | `cargo +stable fmt --all -- --check` | `completed` / `success` |
| Run Clippy | `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings` | `completed` / `success` |
| Check workspace | `RUSTFLAGS="-D warnings" cargo +stable check --workspace --all-targets` | `completed` / `success` |
| Run workspace tests | `cargo +stable test --workspace --all-targets` | `completed` / `success` |
| Run workspace doctests | `cargo +stable test --doc --workspace` | `completed` / `success` |
| Post Check out repository | Checkout post-job cleanup | `completed` / `success` |
| Complete job | GitHub job finalization | `completed` / `success` |

The hosted all-target log recorded:

| Cargo target | Passed | Failed |
|--------------|-------:|-------:|
| `cubikan_cli` library unit tests | 32 | 0 |
| `cubikan` binary unit tests | 0 | 0 |
| `cli_e2e` integration target | 6 | 0 |
| `runner` integration target | 13 | 0 |
| `cubikan_core` library unit tests | 44 | 0 |
| `lifecycle` integration target | 16 | 0 |
| `serialization` integration target | 5 | 0 |
| **All targets** | **116** | **0** |

Every target reported zero ignored, measured, or filtered-out tests. The hosted
doctest step reported one passing `cubikan_core` doctest and no failing doctest.

### Hosted runtime, checkout, and permission provenance

The job log recorded runner `2.336.0`, Ubuntu `24.04.4`, image version
`20260720.247.2`, and stable `rustc 1.97.1`. These are observations from this
run, not a runner, operating-system, toolchain-matrix, or MSRV promise.

Checkout used the workflow's immutable action SHA and did not persist its
credential. Its fetch, `git rev-parse`, and `git log` evidence each identified
the exact tested candidate SHA `55cbdea6a492e6b958f92fd9e6286f14bad737cb`.
The built-in token had explicit `Contents: read` and implicit `Metadata: read`;
the workflow referenced no custom secret.

The completed GitHub run and job are the authoritative hosted integration
oracle. Local execution corroborates the named assertions, while the hosted
evidence proves that GitHub fetched and executed the critic-response test
commit after Build ledger head `071341d1632ca6cfe363a334b33ba0b77209401e`.
It does not claim product-level competing-client E2E behavior, database
isolation, branch protection, merge authorization, fixed future runner
versions, or a successful pull-request event.
