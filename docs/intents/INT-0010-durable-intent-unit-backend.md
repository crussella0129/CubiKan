# INT-0010 — Durable multi-unit CubiKan backend

<!-- sprint-loop-intent-v2 -->
- **Intent ID:** INT-0010
- **State:** proposed
- **Work evidence:** none
- **Completion evidence:** none
- **Code evidence:** none
- **Test evidence:** none
- **Documentation evidence:** none

## Intent

Provide a durable, multi-unit application boundary above `cubikan-core` so
independent invocations and derivative applications can create, retrieve,
transition, and complete Intent Units by stable identity. Stored state and the
external command/query contract will be adapter-owned and versioned, and every
load will reconstruct state through validated domain behavior.

This intent does not yet select a database, local versus network transport,
deployment model, multi-tenancy, authentication, authorization, encryption,
backup, replication, or long-term compatibility policy. Those choices must be
made explicitly before the intent moves into executable work.

## Acceptance criteria

- Multiple Intent Units survive process restart and can be retrieved by stable
  ID with their immutable workflow snapshots and complete ordered lifecycle
  history intact.
- A bounded, paginated collection query can discover units by documented stable
  lifecycle fields such as workflow, species, phase, and status without storing
  or interpreting derivative-domain records; ordering and cursor consistency are
  explicit.
- Create, get, transition, and complete operations use an adapter-owned,
  explicitly versioned boundary rather than exposing provisional core Serde or
  permitting consumers to edit shared storage directly.
- Every load validates the stored workflow and replays or otherwise proves the
  aggregate through `cubikan-core`; corrupt, inconsistent, or unsupported
  representations fail closed without mutation.
- Successful mutation commits one complete unit update before reporting success;
  duplicate IDs, missing IDs, stale revisions, and rejected lifecycle commands
  preserve the prior durable state.
- A process-level test creates a unit, exits, performs lifecycle work across
  later invocations, completes it, and retrieves the same validated final state.
- The selected storage, schema evolution, concurrency, failure-atomicity, and
  recovery guarantees are documented precisely without claiming cryptographic
  audit or indefinite compatibility.

## Rationale

The current CLI intentionally loses state when its process exits. Every proposed
manager, studio, graph, organizational, or accounting application needs a
reusable source of current state before it can honestly use CubiKan as a backend.
Keeping storage and transport above the pure core preserves its existing domain
boundary.

## Alternatives

Passing CLI response snapshots into later requests was rejected because those
snapshots omit workflow topology and are not a durable contract. Persisting
`cubikan-core` JSON directly would turn a provisional representation into an
accidental schema. Letting every derivative project invent its own store would
duplicate validation and produce incompatible state authorities.

## Consequences

This intent has a hard dependency on
[INT-0009](INT-0009-revisioned-lifecycle-commands.md) for its accepted stale
revision behavior and also requires a deliberate storage decision. A networked
service would require additional rate, timeout, authentication, tenancy, and
deployment policy; an embedded local backend would not satisfy every derivative
runtime. Advanced relationship/portfolio queries remain owned by
[INT-0012](INT-0012-intent-unit-relationships-and-board-projections.md).

## Transition history

- 2026-08-08: created as `proposed` after Sprint 6 research identified durable multi-unit state as the common enabling boundary for most derivative applications.
- 2026-08-08: revised while `proposed` to make INT-0009 a hard dependency and add the minimal bounded collection-query surface needed by queues and dashboards.
