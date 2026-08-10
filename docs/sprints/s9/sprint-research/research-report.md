# Sprint 9 Research Report

## Intents Reviewed

- [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) — selected and revised; relevance: INT-0010 now supplies the canonical durable unit collection needed for typed cross-unit relationships and multi-board projections; current state: `planned`.
- [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) — reviewed; relevance: its realized strict SQLite backend, replay validation, guarded mutations, and live keyset pagination are the foundation and compatibility boundary Sprint 9 must extend deliberately; current state: `realized`.
- [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md) — reviewed but not selected; relevance: provenance is strategically important, but full artifact association still needs external identity, correction, verification, privacy, and attribution policy beyond the relationship vertical; current state: `proposed`.
- [INT-0011](../../../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md) — reviewed but not selected; relevance: metric evidence still needs observation identity, trusted time, window, correction, numeric, retention, and governance semantics that relationships do not require; current state: `proposed`.

## 1. Sprint Goal

Realize INT-0012 as the first durable cross-unit extension above the lifecycle
aggregate. Add immutable, versioned relationship definitions; validated directed
edges among existing Intent Units; explicit edge deletion; bounded direct
relationship queries; and ephemeral versioned board projections. Introduce exact
SQLite schema version 2 and an explicit atomic v1-to-v2 migration while leaving
`cubikan-core`, stored Intent Unit envelopes, lifecycle revisions, and local JSON
protocol v1 unchanged.

The first contract deliberately selects the previously unresolved relationship
policy: definitions may constrain endpoint species and choose `allow` or `reject`
for self-edges and cycles; exact duplicates reject; cycle scope is one definition
version; correction is delete-and-recreate; deletion is non-cascading; and board
membership is computed rather than stored. This is the sprint's material product
assumption and must remain visible in the intent, Plan, tests, and human-approved
merge checkpoint.

## 2. Existing Code Survey

| File | Relevance | Notes |
|------|-----------|-------|
| `docs/intents/INT-0012-intent-unit-relationships-and-board-projections.md` | high | Selected authority; revised in Research to lock the first definition, edge, deletion, projection, and schema-evolution contract. |
| `docs/intents/INT-0010-durable-intent-unit-backend.md` | high | Realized prerequisite; owns exact schema/envelope/protocol v1, replay-validated units, transactional writes, and live keyset pagination while explicitly deferring automatic migration. |
| `docs/intents/INT-0008-traceable-intent-instantiation.md` | medium | Full provenance has greater external-identity, verification, correction, privacy, and attribution policy and remains downstream. |
| `docs/intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md` | medium | Metrics require clocks, windows, numeric definitions, evidence correction, retention, and governance and remain downstream. |
| `crates/cubikan-backend/src/lib.rs` | high | Exposes the concrete synchronous `SqliteBackend`; relationship functionality belongs here rather than in the pure lifecycle core. |
| `crates/cubikan-backend/src/model.rs` | high | Existing adapter-owned commands, views, summaries, pages, cursors, and typed revisions provide the pattern for definition, relationship, and projection values. |
| `crates/cubikan-backend/src/schema.rs` | high | Schema v1 accepts one exact table/four-index object set and rejects every extra object, so relationship tables require exact schema v2 and explicit evolution. |
| `crates/cubikan-backend/src/sqlite.rs` | high | Existing `BEGIN IMMEDIATE`, replay-before-write, bound SQL, commit-before-return, busy classification, and projection checking are the mutation baseline. |
| `crates/cubikan-backend/src/query.rs` | high | Existing constant-fragment exact filters, canonical ID ordering, `limit + 1`, exclusive cursor, and live-page semantics should be reused for projections. |
| `crates/cubikan-backend/src/error.rs` | high | Logical, corruption, schema, busy, and storage errors remain typed; migration and relationship policy need distinct additions without taxonomy collapse. |
| `crates/cubikan-backend/README.md` | high | Documents exact schema v1, no automatic migration, local-filesystem assumptions, serialized writers, live pages, and the current downstream-domain exclusions Sprint 9 must update precisely. |
| `crates/cubikan-local/src/protocol.rs` | medium | Local JSON protocol v1 is strict and has exactly five operations; Sprint 9 must not add relationship variants under the same version. |

The three candidate audits reached the same ordering. INT-0012 has the best
readiness-to-value ratio because it stays within canonical CubiKan identities
and directly unlocks board/portfolio consumers. A narrow origin-reference intent
would be useful but would not realize INT-0008. INT-0011's observation ledger is
implementable only as a prerequisite slice and would not realize its required
count, conversion, attainment, and time projections.

## 3. External Sources

- [SQLite foreign-key documentation](https://www.sqlite.org/foreignkeys.html) — defines database-enforced endpoint existence, per-connection foreign-key enablement, immediate/deferred checks, and the importance of indexes on child keys. Sprint 9 keeps `foreign_keys=ON` on every connection and uses explicit indexes for both endpoint directions.
- [SQLite transaction documentation](https://www.sqlite.org/lang_transaction.html) — confirms that SQLite permits concurrent readers but only one write transaction and that `BEGIN IMMEDIATE` acquires the writer up front or returns busy. Migration and relationship mutations therefore validate and write inside one immediate transaction.
- [SQLite PRAGMA documentation](https://www.sqlite.org/pragma.html#pragma_user_version) — defines `user_version` as an application-owned integer SQLite does not interpret. CubiKan continues to treat it as an exact adapter schema contract and changes it to 2 only inside the explicit successful migration.
- [SQLite ALTER TABLE documentation](https://www.sqlite.org/lang_altertable.html) — documents transactional schema-change procedures and foreign-key validation. Because v2 adds tables rather than rewriting `intent_units`, migration can preserve the existing table/envelopes while creating the exact new objects and validating them before commit.
- [SQLite recursive common-table-expression documentation](https://www.sqlite.org/lang_with.html) — documents bounded graph traversal and cycle-safe recursive queries. The backend can test whether a proposed edge would close a path within one definition version before inserting it.

## 4. Risks, Unknowns, Dependencies

- **Decision — relationship semantics:** Arbitrary opaque edges would defer the very policy INT-0012 requires. The selected immutable definition records direction (v1 is directed), optional source/target species constraints, and explicit self/cycle rules. Self policy exclusively governs length-one edges; cycle policy governs non-self closure. Duplicate rejection, non-atomic delete-and-recreate correction, and non-cascading deletion are fixed v1 behavior.
- **Risk — accidental schema-v1 mutation:** Current schema inspection rejects every extra object. Fresh databases must initialize exact v2, ordinary open must validate and retain exact v1 for existing unit operations, relationship operations must require v2, and only an explicit migration entry point may change v1 after a locked reinspection.
- **Risk — migration partial state:** Migration must acquire `BEGIN IMMEDIATE`, revalidate exact v1, create only the exact v2 tables/indexes/foreign keys, run schema/integrity/foreign-key validation, set `user_version=2` last, validate exact v2, and commit. Any error or process interruption before commit leaves exact v1. It does not create a backup, downgrade path, or indefinite compatibility promise.
- **Risk — dual lifecycle authority:** Definitions and edges reference canonical Intent Unit IDs but never edit unit envelopes or SQL lifecycle projections. Relationship changes do not advance an Intent Unit revision or append lifecycle history.
- **Risk — policy race:** Endpoint, species, duplicate, self-edge, and cycle validation must occur after acquiring the writer and before inserting the edge. A recursive path check outside that transaction could allow two concurrent writers to form a forbidden cycle.
- **Risk — deletion overclaim:** Physical edge deletion is semantic removal from current relationship state, not forensic erasure from SQLite pages, retained audit history, cascade, or Intent Unit deletion. Definition deletion remains unsupported.
- **Risk — projection drift:** Stored board membership would become a second state authority. Projection v1 is an ephemeral typed query over replay-validated unit rows and validated direct relationships; the result identifies query version 1 but is not persisted.
- **Unknown — authorization and topology privacy:** The realized backend is local, single-tenant, unauthenticated, and unencrypted. This sprint does not decide who may define, relate, delete, or inspect organizational topology and makes no network/service security claim.
- **Unknown — advanced graph language:** Transitive traversal, arbitrary Boolean expressions, stored definitions, historical snapshots, WIP limits, scheduling, delegation, fan-out/join, retries, and executor policy remain outside the first relationship/projection contract.
- **Unknown — process wire:** Rust callers can use the reusable backend API. The strict `cubikan-local` protocol v1 remains unchanged; a later intent must select a new protocol version before exposing relationship operations to non-Rust process consumers.
- **Dependency:** Realized INT-0010 supplies canonical durable endpoints, exact schema ownership, replay validation, transaction behavior, and pagination precedent. `cubikan-core` remains unchanged and relationship state stays above its aggregate-local INT-0009 revision.

## 5. Recommended Approach

Primary: extend `cubikan-backend` with adapter-owned
`RelationshipDefinitionId`, positive definition version, endpoint-species
constraints, `Allow | Reject` self/cycle policies, immutable definition views,
directed relationship values, typed create/delete/query commands, and projection
query version 1. Use one exact relationship identity tuple—definition ID/version,
source ID, and target ID—so duplicates, deletion, ordering, and cursors are
unambiguous. Keep identifier grammar bounded and canonical in Plan rather than
reusing provisional core serialization.

Schema v2 retains the existing `intent_units` table and stored envelope bytes
unchanged, then adds exact STRICT definition and edge tables plus indexes for
definition/version, source, and target access. Both endpoints use foreign keys
to `intent_units(id)`, and application validation additionally replays endpoint
rows and enforces optional species constraints. Fresh databases initialize v2.
An exact v1 database remains usable for the existing create/get/list/transition/
complete surface, while relationship operations return a typed migration-required
error. `SqliteBackend::migrate_v1_to_v2(path)` is the only upgrade path and is
atomic, explicit, and non-retrying; callers reopen backend handles after success
before using relationship operations.

Definition creation and edge create/delete each use `BEGIN IMMEDIATE`. Edge
creation validates the immutable definition, both replayed endpoints, species,
self-edge policy, duplicate identity, and—when selected—the absence of a path
from target to source within the same definition version before one insert.
Deletion names the complete edge identity and removes no other state. Direct
relationship queries use bounded composite keyset pagination and validate every
selected row before returning a page.

Projection query v1 reuses the four existing lifecycle filters and accepts at
most one direct relation predicate containing an exact definition version,
anchor unit, and endpoint direction. Predicates are ANDed. Results are unit
summaries ordered by canonical ID with the existing `1..=100`, exclusive cursor,
and live-page rules. A unit can therefore appear in multiple projections without
copied membership. The returned projection version and complete typed query make
evaluation reproducible over the same committed state.

Alternatives considered: an automatic migration on open was rejected because it
would silently reverse INT-0010's fail-closed contract. Rejecting ordinary v1
unit operations under the v2-capable binary was rejected because it would strand
accepted data before an operator chooses migration. A companion relation database was rejected because it
weakens endpoint and mutation atomicity across two authorities. Putting edges in
core or the unit envelope was rejected because cross-unit topology has different
identity and revision semantics. Expanding local protocol v1 was rejected because
its strict operation set is already realized.

Rationale: this is the smallest complete INT-0012 vertical. It delivers durable,
typed relationships and useful multi-board queries while resolving the schema
evolution forced by that capability, but it does not import execution-graph,
analytics, provenance, UI, service, authorization, or blockchain policy. The
normal human-approved Sprint 9 `dev -> main` checkpoint ratifies the selected
relationship policy and explicit migration boundary.

## Artifacts

- [Revised INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) — stable authority for the selected relationship, projection, deletion, and schema-evolution contract.
- [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) — realized backend prerequisite and compatibility boundary preserved by the recommendation.
