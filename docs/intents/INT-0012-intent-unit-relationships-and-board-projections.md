# INT-0012 — Intent Unit relationships and board projections

<!-- sprint-loop-intent-v2 -->
- **Intent ID:** INT-0012
- **State:** active
- **Work evidence:** [Sprint 9 build plan](../sprints/s9/sprint-plans/build-plan.md)
- **Completion evidence:** none
- **Code evidence:** none
- **Test evidence:** none
- **Documentation evidence:** none

## Intent

Let a CubiKan backend preserve caller-defined, typed relationships among Intent
Units and project units into multiple board or portfolio views without changing
the lifecycle workflow owned by any unit. Cross-unit dependency, derivation, or
grouping edges remain distinct from `WorkflowEdge`, which continues to connect
phases inside one immutable workflow snapshot.

Relationship contract version 1 uses immutable, versioned definitions. Every
relationship is directed, belongs to one exact definition version, and connects
two existing Intent Units. A definition may constrain each endpoint by immutable
species and explicitly selects whether self-edges and cycles are allowed or
rejected. Cycle evaluation is scoped to edges belonging to that definition
version. For `source == target`, the self-edge policy is the sole policy; cycle
policy applies only to non-self path closure. Exact duplicate edges are rejected.

A relationship definition has complete identity `(definition ID, definition
version)`. Versions are caller-supplied positive `u64` labels; they need not be
contiguous or created in numeric order, and CubiKan does not infer a latest,
compatible, or superseding version. Reusing a complete definition identity is
an atomic duplicate error; changed policy requires a distinct version. An edge
has complete identity `(definition ID, definition version, source Intent Unit
ID, target Intent Unit ID)`. Edge eligibility is independent of endpoint phase,
status, and lifecycle revision.

Accepted definitions and edges are immutable. Correcting an edge means deleting
that exact edge and creating the intended replacement; deletion is explicit,
physical, and non-cascading. Before removal, the backend validates the selected
definition, replay-valid source and target aggregates, and applicable endpoint
species constraints. Missing or corrupt
selected state, including a missing exact edge, fails closed, so deletion is not
an implicit repair path: the complete key selects the candidate edge but does
not alone authorize its removal. The two caller-issued operations are independently
committed and are not one atomic replacement, so an intermediate absence or
competing work can be observed. There is no relationship revision or idempotent
retry token. Physical deletion is semantic removal, not forensic erasure.
Definition deletion, retained relationship history, and automatic correction
are not part of the first contract.

A direct relationship query names one exact definition version, optionally ANDs
exact source and target filters, and returns only direct edges in canonical
`(source, target)` order. It uses a required limit from 1 through 100, an
exclusive complete-edge cursor, and live committed pages. A missing definition
rejects; the cursor is ordering state and need not name a stored edge; an
optional filter naming no stored endpoint returns an empty page.
Selected or lookahead corruption fails the whole page, while filtered-out state
is not claimed to be scanned.

Schema open validates the exact structural, integrity, and foreign-key contract;
semantic replay is operation-selected rather than a global graph scan. Each
operation fails closed on the definition, edge, endpoint, candidate, and
lookahead rows it actually decodes. Cycle creation additionally validates the
edge identities visited by its same-definition reachability query, but does not
replay every endpoint already in that path. Unrelated, filtered-out, and
post-lookahead rows are outside that operation’s corruption-detection claim.

The first board contract is an ephemeral, versioned projection query rather
than a stored board or copied membership list. Projection query version 1 ANDs
the existing lifecycle filters with at most one direct relationship predicate.
That predicate is either outgoing from one exact anchor, returning direct
targets, or incoming to one exact anchor, returning direct sources. Its
definition and anchor must exist and validate. With no relationship predicate,
projection v1 remains the versioned lifecycle-filter query.
It uses canonical Intent Unit ID ordering, limits from 1 through 100, and an
exclusive last-returned-ID cursor. Each page is a live committed view. The same
query version over the same canonical committed unit and relationship state is
reproducible; historical snapshots across later mutations are not implied.

This intent supplies reusable relationship and query primitives only. It does
not define delegation, scheduling, fan-out/join execution, skill loading, WIP
limits, cascade behavior, board layout, notifications, or user interface policy.

## Acceptance criteria

- A caller can create and query an immutable, versioned relationship definition
  and a validated directed edge between existing Intent Units while preserving
  each unit’s independent identity, workflow, phase, status, revision, and
  history.
- A definition may constrain source and target species and explicitly selects
  `allow` or `reject` for self-edges and cycles. Exact duplicate edges reject;
  cycle evaluation considers only the directed edges of the same definition
  version.
- Rejected unknown-endpoint or policy-invalid relationships are atomic and do
  not mutate either Intent Unit or accepted relationship state. Concurrent
  writers serialize through the backend transaction boundary rather than
  bypassing validation.
- An accepted edge can be deleted only by naming its complete definition-version,
  source, and target identity and validating its selected definition, both
  replay-valid endpoints, and applicable species constraints. Missing or corrupt selected state rejects without
  removing the edge. Deletion is non-cascading and leaves both endpoint
  aggregates unchanged; correction is delete-and-recreate, with no claim of
  retained relationship history or definition deletion.
- A caller can retrieve one exact definition and list direct relationships through
  the bounded, canonical, exclusive-cursor, live-page contract. Missing
  definitions, absent filter endpoints, and selected corruption have the exact
  fail-closed behavior stated above.
- A unit can appear in multiple caller-defined board or portfolio projections
  based on a versioned ephemeral query that combines existing lifecycle filters
  with at most one direct relationship predicate, without copying or
  transferring ownership of its lifecycle state.
- Projection results are reproducible from canonical unit and relationship
  state and identify projection query version 1. Results use canonical ID order,
  a required limit from 1 through 100, an exclusive ID cursor, and documented
  live-page membership rather than a cross-request snapshot.
- Fresh stores use exact SQLite schema version 2. Existing exact schema-v1
  stores are never silently changed: ordinary open preserves their existing
  create/get/list/transition/complete operations, while every definition, edge,
  relationship-query, and projection operation reports migration required.
  Explicit migration accepts only exact replay-valid v1, acquires one immediate
  transaction, validates every stored Intent Unit, adds only the exact v2
  relationship objects, advances the schema version last, validates exact v2,
  and commits while preserving every existing `intent_units` column value
  byte-for-byte. Busy, interruption, invalid input, and racing migration leave
  one exact prior-or-successor schema without retry, adoption, repair, or partial
  objects. Callers reopen backend handles after success.
- Documentation keeps multi-board views separate from cross-unit execution
  graphs; a skill-graph or manager application owns readiness, scheduling,
  retries, artifact routing, and executor policy. The first realization is a
  Rust backend API only and does not silently extend local JSON protocol v1.
  Documentation also states that migration provides no automatic backup,
  downgrade, reverse migration, progress/resume facility, old-binary
  compatibility, or indefinite schema guarantee, and that relationship state
  carries no actor, timestamp, lifecycle revision, retained history, or
  authorization meaning.

## Rationale

Multi-board pipelines and organizational views need relationships beyond the
single-unit phase graph. A generic backend relation/query layer can support those
consumers without teaching the lifecycle core how to run an agent graph or draw
a board. Immutable definition versions make caller-selected policy explicit,
while ephemeral projections avoid creating a second lifecycle authority.
Caller-owned, non-sequential definition versions preserve policy meaning without
making CubiKan a “latest version” authority. Physical edge deletion was selected
for a bounded current-state relationship store rather than an audit ledger;
retained correction history would introduce a different provenance and privacy
contract. Exact v1 remains usable for lifecycle work so migration is deliberate,
while one-store atomic migration preserves endpoint and transaction authority
that a companion database would weaken.

## Alternatives

Treating relationships as workflow edges was rejected because phase topology
has a different identity and validation scope. Adding parent/child lineage to
the core would prematurely select one relation taxonomy. Copying units between
boards would create competing lifecycle authorities. Storing mutable board
memberships was rejected for the same reason. Silently adding tables to schema
v1 or automatically migrating on open was rejected because schema v1 is an
exact, fail-closed contract. A companion database was rejected because it would
weaken endpoint validation and transaction authority across two stores.

## Consequences

This intent has a hard dependency on the realized
[INT-0010](INT-0010-durable-intent-unit-backend.md) for canonical unit
collections and durable relationship state. The first contract deliberately
selects definition versioning, directed edges, endpoint species constraints,
explicit self/cycle policy, duplicate rejection, delete-and-recreate correction,
non-cascading deletion, bounded direct predicates, live pagination, and explicit
schema migration. These choices add a schema-v2 contract and a physical deletion
operation for relationship edges only; they do not add Intent Unit deletion.

Migration scans and replay-validates all units under one writer transaction and
therefore has no fixed duration, progress, cancellation, resume, or large-store
availability guarantee. It creates no backup and has no downgrade path;
operators must preserve any recovery copy outside CubiKan before migration.
Schema-v2 files are not promised readable by schema-v1-only binaries. A
cycle-reject definition may perform reachability work proportional to its
committed graph, with no Sprint 9 latency or scale guarantee.

Definition authorization, definition listing/deletion, latest-version resolution,
relationship revisions, idempotent correction/retry, historical relationship
queries, relationship history, transitive
queries, arbitrary Boolean query languages, stored boards, snapshots, WIP limits,
delegation, scheduling, fan-out/join, retries, skills, artifact routing, network
transport, local-protocol changes, authentication, tenancy, UI, provenance,
metrics, and blockchain policy remain separate outcomes.

## Transition history

- 2026-08-08: created as `proposed` after Sprint 6 research separated multi-board and pipeline relationships from the existing single-unit workflow graph.
- 2026-08-08: revised while `proposed` to make the durable multi-unit backend in INT-0010 an explicit prerequisite.
- 2026-08-10: revised while `proposed` after INT-0010 realization to select immutable versioned directed definitions, explicit endpoint/self/cycle/duplicate/correction/deletion policy, bounded ephemeral projection queries, exact schema v2, and explicit atomic v1-to-v2 migration as the first implementation boundary.
- 2026-08-10: moved to `planned` when Sprint 9 mapped the selected public relationship model, explicit migration, immutable definitions, atomic edge mutations, bounded queries, projections, documentation, and real-file composition proof to T-901–T-908.
- 2026-08-10: amended while `planned` to make definition and replay-valid endpoint checks part of authorized edge deletion, preventing deletion from becoming an implicit corruption-repair path.
- 2026-08-10: amended while `planned`, before implementation, to make caller-owned non-sequential definition versions, complete edge identity, bounded direct-query semantics, non-idempotent deletion, unrevisioned current-state relationships, projection-anchor behavior, and explicit migration/compatibility limits authoritative before T-901–T-908 began.
- 2026-08-10: amended while `planned` to make structural-open versus operation-selected semantic corruption detection explicit before the Sprint 9 Plan was relocked.
- 2026-08-10: moved to `active` when T-901 began implementing the finalized Sprint 9 relationship and projection value contract.
