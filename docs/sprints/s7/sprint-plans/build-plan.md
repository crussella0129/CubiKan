Finalized - DO NOT EDIT

# Sprint 7 Build Plan

## Intents

- [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) — state: planned; acceptance criteria covered: documented initial revision, public revision observation, exactly-once advancement for every accepted mutation, current-revision lifecycle preservation, typed stale conflicts with stale-first precedence and full atomicity, current-plus-invalid domain errors, validated revision restoration, semantic round trips, and optimistic-conflict nonclaims.
- [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) — state: realized and preserved; affected criteria: caller-declared topology, existing transition/completion errors and precedence, one lifecycle record per accepted mutation, immutable aggregate identity, terminal behavior, and invariant-validating restoration remain unchanged.
- [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) — state: realized and preserved; affected criteria: the one-shot CLI continues using the existing unconditioned core mutators and retains its version 1 request, response, error, and exit contracts without exposing revision state.

## Schema Tree

- Add deterministic optimistic lifecycle conflict detection to the CubiKan core
  - Aggregate revision state
    - T-701: Add the explicit revision value and advance existing lifecycle mutations
  - Revision-conditioned commands
    - T-702: Add stale-first transition and completion commands with separate typed errors
  - Validated interchange
    - T-703: Persist and validate revision during semantic restoration
  - Public guidance and compatibility boundary
    - T-704: Document the revision contract, guarded usage, and explicit nonclaims

## Execution Sequence

### T-701: Add the explicit revision value and advance existing lifecycle mutations

- **Intent:** [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md), preserving [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) and [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Touches:** `crates/cubikan-core/src/intent_unit.rs`, `crates/cubikan-core/src/lib.rs`, `crates/cubikan-core/tests/lifecycle.rs`, `docs/SUMMARY.md`, `docs/intents/INT-0009-revisioned-lifecycle-commands.md`, `docs/sprints/s7/**`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Depends on:** (none)
- **Acceptance criterion:** A new Intent Unit exposes documented revision `0`; every accepted unconditioned transition or completion advances the explicit revision exactly once with its existing single lifecycle record, while existing rejected-command behavior and public mutator signatures remain unchanged.
- **Success criterion (EARS):**
  - **T-701-E1 — WHEN** `IntentUnit::new` constructs a unit, **THEN** the aggregate **SHALL** expose `IntentUnitRevision::INITIAL` whose numeric value is `0`, while remaining active at the workflow initial phase with empty history.
  - **T-701-E2 — WHEN** `transition_to` accepts a declared transition, including a declared reverse or self-edge, or `complete` accepts an eligible completion, **THEN** the aggregate **SHALL** append exactly one existing lifecycle record and advance revision from `n` to `n + 1` exactly once.
  - **T-701-E3 — WHEN** existing unconditioned transition or completion validation rejects a command, **THEN** the method **SHALL** return the same `TransitionError` or `CompletionError` as before and leave the entire aggregate, including revision, unchanged.
  - **T-701-E4 — WHEN** the revision increment helper evaluates `u64::MAX`, **THEN** it **SHALL** report that no next revision exists and **SHALL NOT** wrap to `0`.
- **Notes:** Add an opaque, copyable `IntentUnitRevision(u64)` with private representation, `INITIAL`, `new(u64)`, `value()`, transparent Serde representation, and `Display`. Store it explicitly on `IntentUnit` and expose it through `revision()`. Keep `transition_to(&PhaseId) -> Result<(), TransitionError>` and `complete() -> Result<(), CompletionError>` source-compatible; do not add conflict or exhaustion variants to either existing error. Calculate the next revision with checked arithmetic before mutating phase, status, or history, and centralize successful mutation so record and revision cannot diverge. A constructible valid aggregate cannot reach revision exhaustion without already containing `u64::MAX` records; prove the non-wrapping primitive directly without inventing a reachable domain failure. Do not change CLI source, protocol DTOs, dependencies, clocks, actors, or workflow policy. The first task commit also records the initialized Sprint 7 Book, finalized plans, planned-to-active intent transition, sprint metadata, and queued task ledger required by the task helper. Commit as `sprint-7: T-701 add lifecycle revisions`.

### T-702: Add stale-first transition and completion commands with separate typed errors

- **Intent:** [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md), preserving [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) and [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Touches:** `crates/cubikan-core/src/intent_unit.rs`, `crates/cubikan-core/src/lib.rs`, `crates/cubikan-core/tests/lifecycle.rs`
- **Depends on:** T-701
- **Acceptance criterion:** Public conditioned transition and completion commands accept the current observed revision and return the resulting revision, reject a mismatched revision as a typed conflict before lifecycle validation, preserve existing domain errors when the revision is current, and leave all aggregate fields unchanged on either rejection path.
- **Success criterion (EARS):**
  - **T-702-E1 — WHEN** `transition_to_if_revision` receives the current revision and a declared target, **THEN** it **SHALL** preserve normal transition behavior and return the newly committed revision after exactly one record and one revision advance.
  - **T-702-E2 — WHEN** `complete_if_revision` receives the current revision in a completion-eligible phase, **THEN** it **SHALL** preserve normal completion behavior and return the newly committed revision after exactly one completion record and one revision advance.
  - **T-702-E3 — WHEN** two callers submit commands using the same observed revision and the first command succeeds, **THEN** the second command **SHALL** return a typed `RevisionConflict` containing that expected revision and the aggregate's newer actual revision without mutation.
  - **T-702-E4 — WHEN** either conditioned command receives a stale revision with an otherwise valid or independently invalid lifecycle command, **THEN** it **SHALL** return the revision-conflict variant before terminal, target, edge, or completion-eligibility evaluation and leave identity, species, workflow, phase, status, history, and revision unchanged.
  - **T-702-E5 — WHEN** either conditioned command receives the current revision with a terminal, unknown-target, undeclared-edge, or completion-ineligible command, **THEN** it **SHALL** return its separate domain-error wrapper containing the unchanged existing typed error and leave the aggregate unchanged.
- **Notes:** Expose `transition_to_if_revision(&mut self, target: &PhaseId, expected_revision: IntentUnitRevision) -> Result<IntentUnitRevision, RevisionedTransitionError>` and `complete_if_revision(&mut self, expected_revision: IntentUnitRevision) -> Result<IntentUnitRevision, RevisionedCompletionError>`. Add a shared `RevisionConflict` with immutable `expected()` and `actual()` accessors. Use `RevisionedTransitionError::{Conflict(RevisionConflict), Transition(TransitionError)}` and `RevisionedCompletionError::{Conflict(RevisionConflict), Completion(CompletionError)}`; implement `Display` and `Error`, exposing the stored conflict or domain error through `source()`, without changing the original error enums. Conditioned methods compare revision first, then delegate to the existing unconditioned mutation path so success behavior cannot drift. Return the post-mutation revision, not the caller's expectation. Commit as `sprint-7: T-702 add revision-conditioned commands`.

### T-703: Persist and validate revision during semantic restoration

- **Intent:** [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md), preserving [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md)
- **Touches:** `crates/cubikan-core/src/intent_unit.rs`, `crates/cubikan-core/tests/serialization.rs`
- **Depends on:** T-702
- **Acceptance criterion:** Format-neutral aggregate serialization stores the explicit revision, valid active and completed aggregates restore with the exact revision, and restoration rejects any missing, malformed, or mismatched revision in addition to existing history, phase, and status inconsistencies.
- **Success criterion (EARS):**
  - **T-703-E1 — WHEN** a valid active or completed Intent Unit is serialized, restored, and then continued where lifecycle state permits, **THEN** restoration **SHALL** preserve its exact revision and the next accepted mutation **SHALL** advance from that restored value exactly once.
  - **T-703-E2 — WHEN** serialized aggregate input omits revision, supplies a non-`u64` representation, or supplies a revision lower or higher than the revision derived by replaying lifecycle history, **THEN** deserialization **SHALL** fail rather than infer, default, wrap, or normalize the stored value.
  - **T-703-E3 — WHEN** serialized lifecycle sequence, transition source, completion phase, final phase, or final status disagrees with validated replay, **THEN** deserialization **SHALL** retain its existing rejection behavior while also requiring revision agreement.
- **Notes:** Add required `revision: IntentUnitRevision` to the private deserialization representation with no Serde default or legacy inference. Reconstruct from revision `0`, replay history through the normal unconditioned lifecycle methods, and compare replayed revision with the stored revision before returning the aggregate. Preserve existing phase/status/history checks and owned workflow validation. JSON remains only the test vehicle for provisional format-neutral Serde behavior; this task does not promise legacy snapshot compatibility or a stable numeric transport encoding. Commit as `sprint-7: T-703 validate serialized revisions`.

### T-704: Document the revision contract, guarded usage, and explicit nonclaims

- **Intent:** [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md), preserving [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) and [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Touches:** `crates/cubikan-core/src/lib.rs`, `README.md`
- **Depends on:** T-703
- **Acceptance criterion:** Public documentation shows how callers observe and condition on revisions, defines stale rejection as optimistic conflict detection, and explicitly avoids claims about storage isolation, locking, cross-unit transactions, idempotent delivery, or a revised CLI protocol.
- **Success criterion (EARS):**
  - **T-704-E1 — WHEN** `cargo test --doc --workspace` runs, **THEN** the crate-level example **SHALL** compile and pass while demonstrating initial revision observation, a successful revision-conditioned lifecycle command, and its returned revision.
  - **T-704-E2 — WHEN** a consumer reads the project documentation, **THEN** it **SHALL** understand revision `0`, exactly-once advancement per accepted lifecycle mutation, current-versus-stale command behavior, typed reject-and-refresh conflicts, and the distinction between revision and one-based lifecycle-record sequence.
  - **T-704-E3 — WHEN** the completed Sprint 7 diff and workspace gates are inspected, **THEN** they **SHALL** show no database, lock, retry, lease, clock, actor, cross-unit atomicity, delivery-idempotency, dependency, CLI protocol, request/response DTO, or error-code change.
  - **T-704-E4 — WHEN** Sprint 7 Book state and navigation are validated, **THEN** the installed validator **SHALL** report a valid Book v2 with INT-0009 linked in legal lifecycle state and every new local Markdown link **SHALL** resolve.
- **Notes:** Keep the guarded example caller-configured and library-only. Describe revision as an aggregate-local optimistic command token, not a timestamp, ETag wire format, global sequence, durable compare-and-swap implementation, or cryptographic proof. Retain the existing one-shot CLI documentation and serialization-provisional warning. Hosted exact-head CI is a later Test-phase strengthening gate, not a Build EARS prerequisite. Commit as `sprint-7: T-704 document optimistic lifecycle revisions`.
