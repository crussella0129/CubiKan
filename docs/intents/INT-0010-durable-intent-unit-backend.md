# INT-0010 — Durable multi-unit CubiKan backend

<!-- sprint-loop-intent-v2 -->
- **Intent ID:** INT-0010
- **State:** superseded
- **Work evidence:** [Sprint 8 build plan](../sprints/s8/sprint-plans/build-plan.md)
- **Completion evidence:** [T-801–T-810 completion ledger](../work/completed-tasks.md#t-801-sprint-8)
- **Code evidence:** [`cubikan-backend` public boundary](../../crates/cubikan-backend/src/lib.rs) and [`cubikan-local` process adapter](../../crates/cubikan-local/src/lib.rs)
- **Test evidence:** [Sprint 8 test report](../sprints/s8/sprint-tests/test-report.md)
- **Documentation evidence:** [CubiKan README](../../README.md), [`cubikan-backend` guide](../../crates/cubikan-backend/README.md), and [`cubikan-local` guide](../../crates/cubikan-local/README.md)

## Intent

Provide a durable, multi-unit application boundary above `cubikan-core` so
independent invocations and derivative applications can create, retrieve,
transition, and complete Intent Units by stable identity. Stored state and the
external command/query contract will be adapter-owned and versioned, and every
load will reconstruct state through validated domain behavior.

The first realization is a local, embedded backend: a reusable
`cubikan-backend` Rust crate backed by bundled SQLite and a separate
`cubikan-local` one-request process adapter. Callers select the database path
explicitly. The SQLite file, its strict schema version 1, and the local JSON
protocol version 1 are adapter-owned contracts; neither is the provisional core
Serde representation or the existing stateless `cubikan` protocol.

This first boundary is single-tenant, unencrypted, and local-filesystem only. It
does not select a network service, deployment model, authentication,
authorization, multi-tenancy, backup, replication, deletion, cross-unit
transactions, or indefinite schema/protocol compatibility. Those outcomes need
separate intent rather than silent expansion of the embedded backend.

## Acceptance criteria

- Multiple Intent Units survive process restart and can be retrieved by stable
  ID with their immutable workflow snapshots and complete ordered lifecycle
  history intact.
- A bounded, paginated collection query can discover units by documented stable
  lifecycle fields such as workflow, species, phase, and status without storing
  or interpreting derivative-domain records. Version 1 orders canonical Intent
  Unit IDs lexically, accepts limits from 1 through 100, and uses an exclusive
  last-returned-ID keyset cursor. Each page is a live committed view rather than
  a cross-request snapshot, and mutable filter membership is documented.
- Create, get, transition, and complete operations use an adapter-owned,
  explicitly versioned boundary rather than exposing provisional core Serde or
  permitting consumers to edit shared storage directly. Mutation requests carry
  a decimal-string expected revision and return the committed revision.
- Every load validates the stored workflow and replays or otherwise proves the
  aggregate through `cubikan-core`; corrupt, inconsistent, or unsupported
  representations fail closed without mutation. The stored version 1 envelope
  contains the complete workflow snapshot, lifecycle history, status, phase,
  revision, species, and identity, while checked SQL projection columns support
  bounded queries without becoming a second authority.
- Successful mutation commits one complete unit update before reporting success;
  duplicate IDs, missing IDs, stale revisions, and rejected lifecycle commands
  preserve the prior durable state. SQLite mutations use one `BEGIN IMMEDIATE`
  transaction, validate before mutation, invoke the core's revision-conditioned
  command, compare the stored revision again during update, and commit before a
  success response is written.
- A process-level test creates a unit, exits, performs lifecycle work across
  later invocations, exercises a stale competing command and bounded collection
  query, completes the unit, and retrieves the same validated final state.
- The selected storage, schema evolution, concurrency, failure-atomicity, and
  recovery guarantees are documented precisely: SQLite rollback-journal DELETE
  mode with synchronous EXTRA, isolated connections, serialized writers, a
  finite busy timeout, fail-closed schema/envelope versions, and no automatic
  migration or retry. Documentation does not claim cryptographic audit,
  network-filesystem safety, acknowledged response delivery, or indefinite
  compatibility.

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

The realized [INT-0009](INT-0009-revisioned-lifecycle-commands.md) conflict
contract supplies the aggregate token used inside each durable transaction.
Choosing embedded SQLite adds a bundled C build dependency and a concrete local
file format, but avoids inventing a network/security deployment surface before
one is required. `rusqlite` is isolated above the chain-agnostic core, and the
existing stateless CLI remains byte-for-byte behaviorally separate.

Only a newly empty database is initialized at schema version 1. Unknown schema
or envelope versions and inconsistent projection/envelope pairs fail closed;
automatic migrations and import of core/CLI JSON are deferred. A committed
mutation can still be applied even if later stdout delivery fails, so clients
must refresh rather than assume rollback; delivery idempotency is not claimed.
Advanced relationship/portfolio queries remain owned by
[INT-0012](INT-0012-intent-unit-relationships-and-board-projections.md).

## Transition history

- 2026-08-08: created as `proposed` after Sprint 6 research identified durable multi-unit state as the common enabling boundary for most derivative applications.
- 2026-08-08: revised while `proposed` to make INT-0009 a hard dependency and add the minimal bounded collection-query surface needed by queues and dashboards.
- 2026-08-09: revised while `proposed` after INT-0009 realization to select an explicit-path local SQLite backend, a separate versioned process adapter, strict replay-validated storage, transactional stale-writer rejection, and live keyset pagination as the bounded first implementation.
- 2026-08-09: moved to `planned` when Sprint 8 mapped the selected embedded backend, storage contract, query semantics, guarded mutations, process boundary, process proof, and documentation to T-801–T-810.
- 2026-08-09: moved to `active` immediately before T-801 began the adapter-owned backend value contract.
- 2026-08-10: moved to `realized` after T-801–T-810 completed, the final Test Critic returned `clean`, 165 workspace tests and one doctest passed, and GitHub Actions run 31344560356 succeeded at exact tested commit `065b71fa1b63ba6abce6effb23c9d20674171835`.
- 2026-08-11: moved from `realized` to `superseded` when planned [INT-0014](INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md) replaced the governing SQLite-canonical mutation and local protocol-v1 contract, selecting a canonical pallet and verified SQLite-v3 projection for Build; all Sprint 8 implementation and realization evidence remains historical until the successor is realized.
