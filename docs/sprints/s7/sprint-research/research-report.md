# Sprint 7 Research Report

## Intents Reviewed

- [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) — selected; relevance: defines the smallest backend-neutral stale-writer primitive and is the only proposed implementation intent with no unrealized prerequisite; current state: `proposed`.
- [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md) — reviewed but not selected; relevance: immutable origin references could begin independently, but full revision-scoped and bidirectional provenance depends on revisioned commands and a durable index; current state: `proposed`.
- [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) — reviewed but not selected; relevance: it is the next platform boundary, but depends on INT-0009 and unresolved storage, transport, migration, pagination, and consistency policy; current state: `proposed`.
- [INT-0011](../../../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md) — reviewed but not selected; relevance: measurement evidence depends on revisioned durable lifecycle state and still requires metric, time, correction, and governance semantics; current state: `proposed`.
- [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) — reviewed but not selected; relevance: relationships and multi-board projections depend on a durable collection boundary and unresolved relation policy; current state: `proposed`.
- [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) — reviewed; relevance: its realized aggregate, atomic mutation, history, and validated restoration contracts are the implementation baseline INT-0009 must preserve; current state: `realized`.
- [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) — reviewed; relevance: its one-shot in-memory CLI cannot exhibit competing writers and therefore must not be enlarged merely to demonstrate revision conflicts; current state: `realized`.

## 1. Sprint Goal

Realize INT-0009 in `cubikan-core` by giving every Intent Unit an explicit,
zero-based lifecycle revision and additive revision-conditioned transition and
completion operations. A caller using the current revision will retain the
existing lifecycle result; a caller using a stale revision will receive a typed
conflict before domain-command evaluation, with the aggregate unchanged. Every
accepted conditioned or unconditioned mutation will append one lifecycle record
and advance the revision exactly once. Validated restoration will reject a
stored revision that disagrees with replayed history, phase, or status. Sprint 7
will not select a database, transport, lock, retry policy, clock, actor model, or
new CLI protocol.

## 2. Existing Code Survey

| File | Relevance | Finding |
|------|-----------|---------|
| `Cargo.toml` | medium | The virtual workspace contains only `cubikan-core` and `cubikan-cli`; INT-0009 needs no new crate or dependency. |
| `crates/cubikan-core/Cargo.toml` | medium | The unpublished `0.1.0` core already depends on Serde and UUID; a fixed-width revision can use the standard library. |
| `crates/cubikan-core/src/lib.rs` | high | The public facade and crate doctest expose the current unconditioned lifecycle API and will need additive revision exports and an ergonomic guarded example. |
| `crates/cubikan-core/src/intent_unit.rs` | high | `IntentUnit` owns identity, species, workflow, phase, status, and ordered history. Each successful transition/completion appends exactly one one-based record, while validation completes before mutation. |
| `crates/cubikan-core/src/id.rs` | low | Opaque UUID-backed identity remains separate from the new monotonic aggregate-local revision; no ID semantics need to change. |
| `crates/cubikan-core/src/workflow.rs` | medium | Caller-defined topology and completion eligibility remain the domain validators after a current revision check; workflow policy is unchanged. |
| `crates/cubikan-core/tests/common/mod.rs` | medium | Fixed IDs and reusable workflow/unit fixtures support deterministic public revision tests without clocks or mocks. |
| `crates/cubikan-core/tests/lifecycle.rs` | high | Public-API journeys already prove lifecycle atomicity, rework, self-edges, terminal behavior, and ordered history; they are the external-consumer seam for conflict precedence. |
| `crates/cubikan-core/tests/serialization.rs` | high | Semantic round trips and tamper tests already exercise validated aggregate replay; revision mismatch cases belong at this seam. |
| `crates/cubikan-cli/src/execution.rs` | high | The CLI maps existing transition/completion errors exhaustively and invokes only unconditioned operations; an additive core API avoids changing its v1 behavior. |
| `crates/cubikan-cli/src/protocol.rs` | medium | The experimental one-shot v1 DTO has no revision and no resumable state; changing it would not demonstrate real stale-writer protection. |
| `README.md` | medium | The documented project boundary is a chain-agnostic core plus one-shot CLI and explicitly excludes persistence, services, authorization, clocks, and stable compatibility promises. |

The present aggregate already supplies the key invariant: accepted mutations and
lifecycle records have a one-to-one relationship. Revision should remain an
explicit aggregate field and public command token, however, rather than making
the length of an internal `Vec` a caller contract. Today its value can equal the
count of accepted records: initial revision `0`, then revision `n` after the
`n`th accepted mutation.

## 3. External Sources

- [RFC 9110 — HTTP Semantics, If-Match](https://www.rfc-editor.org/rfc/rfc9110.html#name-if-match) — a mature example of checking a caller-observed state token before evaluating a state-changing request to avoid lost updates; CubiKan adopts only the backend-neutral precondition idea, not HTTP semantics.
- [Rust `u64::checked_add`](https://doc.rust-lang.org/stable/std/primitive.u64.html#method.checked_add) — supplies explicit overflow detection so revision advancement cannot silently wrap.
- [Serde custom serialization](https://serde.rs/custom-serialization.html) — supports the existing design of decoding an untrusted representation and reconstructing through validated domain behavior instead of restoring private aggregate fields directly.

## 4. Risks, Unknowns, Dependencies

- **Risk — compatibility surface:** Adding a conflict variant to the existing
  public `TransitionError` or `CompletionError` would break exhaustive matches
  such as the CLI adapter’s mappings. Preserve those errors and mutator
  signatures; expose separate conditioned-command errors that wrap a shared
  typed revision conflict or the unchanged domain error.
- **Risk — precedence drift:** A stale expectation must be rejected before
  terminal, target, edge, or completion-eligibility checks. With the current
  revision, existing error precedence—including terminal transition rejection
  before target validation—must remain unchanged.
- **Risk — partial mutation:** Revision overflow must be checked before phase,
  status, or history changes. All success paths should share one private apply
  path so a record and revision cannot diverge.
- **Risk — redundant-looking state:** Revision initially equals history length,
  but it is an explicit optimistic-command contract. It must not expose vector
  layout, acquire a setter, or be confused with `LifecycleRecord` sequence.
- **Risk — serialization evolution:** Adding a required stored revision makes
  older provisional core snapshots fail restoration. The repository promises no
  stable serialized schema, so rejecting a missing revision is less ambiguous
  than silently defaulting or partially inferring legacy state.
- **Risk — client numeric representation:** `u64` is appropriate for a
  persistence-facing counter, but future JavaScript transports may need a string
  representation above the safe-integer range. Sprint 7 does not stabilize a
  JSON wire encoding or expand Electron/CLI contracts.
- **Unknown:** Whether a future adapter will expose revision as a decimal string,
  number, ETag-like value, or another versioned DTO representation. That belongs
  to INT-0010’s selected transport boundary.
- **Unknown:** Command idempotency remains separate. Refreshing and retrying a
  self-transition can correctly create another record and revision.
- **Dependency:** INT-0009 has no unrealized prerequisite and extends the
  realized INT-0001 aggregate. INT-0010 depends on this conflict contract;
  INT-0011 depends on INT-0009 and INT-0010; INT-0012 depends on INT-0010; full
  revision-scoped INT-0008 evidence also needs INT-0009 and a durable index.

## 5. Recommended Approach

Primary: add an opaque `IntentUnitRevision(u64)` with documented initial value
`0`, an `IntentUnit::revision()` accessor, and explicit checked advancement in
the core aggregate. Preserve `transition_to` and `complete` as single-owner
convenience operations, but route them through the same private mutation paths
so they also advance revision. Add `transition_to_if_revision` and
`complete_if_revision` for competing or durable callers. Their separate public
error types will distinguish a shared `RevisionConflict { expected, actual }`
from the unchanged `TransitionError` or `CompletionError`, avoiding new variants
in existing exhaustive-match surfaces. Successful conditioned operations should
return the new revision.

The conditioned methods will compare the expected revision first. If it is
current, normal domain validation runs with its existing precedence. If it is
stale, the method returns the conflict without evaluating whether the command is
otherwise valid. Both conditioned and unconditioned successes will calculate the
next revision before mutation, append exactly one existing record, mutate phase
or status, and commit that revision once. Rejections will preserve full aggregate
equality.

Serialization will include the explicit revision in the provisional aggregate
representation. Restoration will replay existing records through the normal
mutation paths, derive the validated revision, and then compare it with the
stored value alongside existing phase/status checks. Missing, lower, or higher
revisions will be rejected rather than inferred. Tests will cover initial,
transition, self-edge, completion, competing-observer, stale-plus-invalid,
current-plus-invalid, terminal, overflow-safe, active/completed round-trip, and
tampered-restoration paths through unit and public integration seams.

Alternative considered: replace existing mutators with mandatory revision
arguments. That would force single-owner callers and the one-shot CLI to invent
an observation step, break the realized API, and still provide no durable
compare-and-swap. Adding conflict variants directly to existing errors was also
rejected because it needlessly breaks exhaustive downstream matches. Deriving
revision from history length alone was rejected because it would expose an
internal collection property instead of a stable domain token. Implementing
INT-0010 first was rejected because a backend cannot honestly specify atomic
stale-writer behavior until this primitive exists and still requires human
choices about storage and transport.

## Artifacts

- [INT-0009 — Revisioned lifecycle commands and atomic conflict rejection](../../../intents/INT-0009-revisioned-lifecycle-commands.md)
- Planned implementation: `crates/cubikan-core/src/intent_unit.rs` and `crates/cubikan-core/src/lib.rs`.
- Planned public verification: `crates/cubikan-core/tests/lifecycle.rs` and `crates/cubikan-core/tests/serialization.rs`.
