# `cubikan-backend`

`cubikan-backend` is CubiKan's synchronous, embedded SQLite boundary for
multiple durable Intent Units. A caller supplies one explicit local filesystem
path to `SqliteBackend::open`; the backend owns that file's schema and stored
representation. It does not search for a database, select a default path, or
accept SQLite's special `:memory:` path.

The lifecycle operations are `open(path)`, `create(CreateIntentUnit)`,
`get(IntentUnitId)`, `list(ListIntentUnits)`,
`transition(TransitionIntentUnit)`, and `complete(CompleteIntentUnit)`. Schema
v2 additionally enables `create_relationship_definition`,
`get_relationship_definition`, `create_relationship`, `delete_relationship`,
`list_relationships`, and `project`. The associated
`SqliteBackend::migrate_v1_to_v2(path)` function is the only migration
entrypoint. This is a concrete synchronous API, not a repository trait, async
API, or `Send + Sync` contract. See the
[local process adapter](../cubikan-local/README.md) for the separate, unchanged
JSON boundary.

## Version matrix

The adapter-owned contracts evolve independently:

| Contract | Supported version | Authority |
|----------|-------------------|-----------|
| Stored Intent Unit envelope | 1 | Complete replayable aggregate representation |
| SQLite schema | 1 and 2 | Exact v1 lifecycle store; exact v2 relationship extension |
| Relationship contract | 1 | Immutable directed definition versions and exact edges |
| Projection query | 1 | Ephemeral lifecycle/direct-relationship query |
| [`cubikan-local` JSON protocol](../cubikan-local/README.md) | 1 | Five lifecycle operations only |

None is the provisional `cubikan-core` Serde layout. A version identifies one
exact contract; equal version numbers in different rows of the matrix do not
couple their evolution. There is no promise of automatic migration, old-binary
readability, or indefinite compatibility.

## Stored envelope version 1

Each row contains one strict JSON envelope with exactly these fields and nested
shapes:

```text
{
  representation_version: 1,
  id: string,
  species: string,
  phase: string,
  revision: canonical unsigned-decimal string,
  status: "active" | "completed",
  workflow: {
    id: string,
    phases: [string...],
    initial_phase: string,
    edges: [{from: string, to: string}...],
    completion_phases: [string...]
  },
  history: [
    {type: "transition", sequence: unsigned integer, from: string, to: string}
    | {type: "completion", sequence: unsigned integer, phase: string}
  ...]
}
```

Unknown fields are rejected at every nesting level. Workflow collections retain
caller order; history is ordered and its sequence values are one-based. Revision
text is `"0"` or a nonzero decimal value without a leading zero and must fit
`u64`.

Every load reconstructs the typed vocabulary and `Workflow`, creates the
aggregate at revision 0, and replays every history record through ordinary
`cubikan-core` lifecycle methods. The backend then compares the declared
identity, workflow, phase, status, revision, and exact history with the replayed
aggregate. Malformed, unsupported, or semantically impossible envelopes fail
closed. The codec never serializes or deserializes `IntentUnit` directly.

The SQL columns used for queries are checked projections, not a second source of
truth. On `get`, `list`, `transition`, and `complete`, the row's ID, envelope
version, workflow ID, species, phase, status, and eight-byte revision projection
must agree with the replayed envelope. A mismatch rejects the operation rather
than repairing either representation.

## SQLite schema versions 1 and 2

Exact schema v1 has `PRAGMA user_version = 1` and exactly one owned `STRICT`
table:

```sql
CREATE TABLE intent_units (
    id TEXT NOT NULL PRIMARY KEY COLLATE BINARY,
    envelope_version INTEGER NOT NULL CHECK(envelope_version = 1),
    envelope TEXT NOT NULL,
    workflow_id TEXT NOT NULL COLLATE BINARY,
    species TEXT NOT NULL COLLATE BINARY,
    phase TEXT NOT NULL COLLATE BINARY,
    status TEXT NOT NULL COLLATE BINARY CHECK(status IN ('active','completed')),
    revision BLOB NOT NULL CHECK(length(revision) = 8)
) STRICT
```

The revision projection is exactly eight big-endian bytes. The four owned
indexes are:

```sql
CREATE INDEX intent_units_by_workflow ON intent_units(workflow_id,id)
CREATE INDEX intent_units_by_species ON intent_units(species,id)
CREATE INDEX intent_units_by_phase ON intent_units(phase,id)
CREATE INDEX intent_units_by_status ON intent_units(status,id)
```

Together with `sqlite_autoindex_intent_units_1`, exact v1 owns six
`sqlite_schema` objects. Exact schema v2 preserves all six objects and every
`intent_units` declaration, then adds these two tables:

```sql
CREATE TABLE relationship_definitions (
    definition_id TEXT NOT NULL COLLATE BINARY
        CHECK(
            length(CAST(definition_id AS BLOB)) BETWEEN 1 AND 64
            AND instr(definition_id, char(0)) = 0
            AND definition_id GLOB '[a-z]*'
            AND definition_id NOT GLOB '*[^a-z0-9._-]*'
        ),
    definition_version BLOB NOT NULL
        CHECK(
            length(definition_version) = 8
            AND definition_version <> X'0000000000000000'
        ),
    directed INTEGER NOT NULL CHECK(directed = 1),
    source_species TEXT COLLATE BINARY,
    target_species TEXT COLLATE BINARY,
    self_policy TEXT NOT NULL COLLATE BINARY
        CHECK(self_policy IN ('allow','reject')),
    cycle_policy TEXT NOT NULL COLLATE BINARY
        CHECK(cycle_policy IN ('allow','reject')),
    PRIMARY KEY(definition_id,definition_version)
) STRICT
```

```sql
CREATE TABLE intent_unit_relationships (
    definition_id TEXT NOT NULL COLLATE BINARY,
    definition_version BLOB NOT NULL
        CHECK(
            length(definition_version) = 8
            AND definition_version <> X'0000000000000000'
        ),
    source_id TEXT NOT NULL COLLATE BINARY,
    target_id TEXT NOT NULL COLLATE BINARY,
    PRIMARY KEY(definition_id,definition_version,source_id,target_id),
    FOREIGN KEY(definition_id,definition_version)
        REFERENCES relationship_definitions(definition_id,definition_version)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY(source_id) REFERENCES intent_units(id)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY(target_id) REFERENCES intent_units(id)
        ON UPDATE RESTRICT ON DELETE RESTRICT
) STRICT
```

The only new explicit indexes are:

```sql
CREATE INDEX relationship_edges_by_source
    ON intent_unit_relationships(definition_id,definition_version,source_id,target_id)
CREATE INDEX relationship_edges_by_target
    ON intent_unit_relationships(definition_id,definition_version,target_id,source_id)
```

Exact v2 has `PRAGMA user_version = 2` and exactly twelve owned objects: the six
v1 objects; both new tables; their two implicit primary-key autoindexes; and the
two explicit edge indexes. It has no other table, index, trigger, view,
generated column, or partial index. Both added tables are `STRICT` rowid tables
(`strict=1`, `wr=0`). Composite primary-key positions follow declaration order.
All three foreign keys are immediate, `MATCH NONE`, and `RESTRICT` on update
and delete. `PRAGMA integrity_check` must return only `ok`, and
`PRAGMA foreign_key_check` must return no row.

Opening a fresh or truly empty database initializes exact v2 in one
transaction. Opening an existing exact v1 or exact v2 database accepts only its
complete owned object, SQL, column, constraint, index, autoindex, integrity, and
foreign-key contract; no additional user object is accepted. Version 0 with any
objects is unowned. Another version, malformed schema, failed integrity check,
invalid foreign key, or non-SQLite content fails closed. The backend does not
adopt, migrate, or repair such a file during open. Ownership and version
inspection precede the persistent journal and synchronous settings. SQLite
itself may touch a file while opening it, so rejection promises no logical
adoption or mutation by CubiKan, not byte-for-byte preservation of every
filesystem representation.

Open's boundary is structural. It validates exact schema metadata, SQL
constraints through integrity checking, and foreign keys; it does not globally
replay every definition, edge, or endpoint envelope. Definition, relationship,
list, cycle, and projection operations decode the exact input, candidate,
visited-edge, endpoint, and one-row lookahead state they select. Selected
corruption fails the whole operation without repair or partial results;
unrelated, filtered-out, or post-lookahead corruption is outside that
operation's detection claim. Cycle reachability validates each visited edge
identity but does not replay every endpoint already in that path.

After its first inspection, `open` uses its own `BEGIN IMMEDIATE` transaction to
recheck ownership and either initialize or accept the schema before returning.
Open can therefore report busy storage as well as schema/storage rejection.

`SqliteBackend` caches the accepted `BackendSchemaVersion`. Exact v1 reports
`V1` and keeps every lifecycle operation available, but all definition,
relationship, relationship-list, and projection methods return
`MigrationRequired { found: V1, required: V2 }` before relationship SQL. Exact
v2 reports `V2` and enables those methods. A handle does not refresh this
capability if another connection changes the file; see the migration procedure
below.

Each accepted connection selects and verifies:

- `journal_mode=DELETE` (rollback journal);
- `synchronous=EXTRA`;
- `foreign_keys=ON`;
- `trusted_schema=OFF`;
- `read_uncommitted=OFF`;
- `locking_mode=NORMAL`; and
- a 5,000-millisecond busy timeout.

Connections are isolated/default SQLite connections. URI filenames and shared
cache are not enabled. The busy timeout bounds a lock-acquisition wait; it is
not a five-second request deadline, a retry loop, or a guarantee that an
operation lasts exactly five seconds. Open, write, and read operations can
surface a SQLite busy/locked condition under local contention.

## Explicit v1-to-v2 migration

`open` never migrates. The only upgrade entrypoint is the public Rust function
`SqliteBackend::migrate_v1_to_v2(path)`. It opens an existing path read/write
without create, so a nonexistent path is rejected and remains nonexistent.
Operators should first stop or quiesce writers and preserve any desired backup
or recovery copy outside CubiKan. The backend creates no backup itself.

A successful migration performs one bounded sequence:

1. acquire a SQLite writer with `BEGIN IMMEDIATE` and revalidate exact schema
   v1 under that lock;
2. reconstruct and replay-validate every stored Intent Unit;
3. add only the two v2 tables, two primary-key autoindexes, and two explicit
   edge indexes declared above;
4. set `user_version=2` last;
5. validate exact schema v2, integrity, and foreign keys; and
6. commit once.

Every preexisting `intent_units` column value, including each envelope and
revision BLOB, remains byte-for-byte identical. This is a logical row-value
guarantee, not a promise that SQLite's whole file bytes, pages, or sidecars stay
unchanged.

Migration returns once and never retries. Busy acquisition or a deliberate
pre-commit interruption against accepted exact v1 leaves exact v1. Two racing
migrators serialize: one can commit exact v2, while the loser then reports
`SourceVersionNotOne { found: 2 }`. An empty version-0 file reports source
version 0; version 0 with user objects is unowned; exact v2 reports source
version 2; other versions are unsupported; and malformed, non-SQLite, or
replay-invalid sources preserve their typed backend cause. Each rejected source
retains its unchanged prior logical state, which is not claimed to be exact v1
or v2 when the input was not acceptable exact v1. No case is adopted, repaired,
partially upgraded, or retried.

After success, drop and reopen backend handles:

```rust
SqliteBackend::migrate_v1_to_v2(database_path)?;
let backend = SqliteBackend::open(database_path)?;
assert_eq!(backend.schema_version(), BackendSchemaVersion::V2);
```

An already-open v1 handle retains its cached `V1` capability even after another
connection migrates the file. Its existing create/get/list/transition/complete
operations remain available, but every relationship or projection method still
returns migration-required until that handle is dropped and reopened. There is
no stale-handle auto-upgrade or downgrade detection guarantee.

Migration has no automatic backup, downgrade or reverse path, progress report,
resume facility, cancellation contract, fixed-duration or large-store
availability guarantee, old-schema-binary readability guarantee, or indefinite
compatibility promise. If migration or later open rejects a file, preserve it,
diagnose or restore it outside the running backend, and reopen only an exact
supported schema. Do not edit rows or schema objects as an in-place repair.

## Writes, conflicts, and commit

Create and lifecycle mutations acquire a writer with `BEGIN IMMEDIATE`. Create
inserts one complete revision-zero row and commits before returning. When the
caller omits an ID, the core generates a non-nil UUID v4; a supplied
syntactically valid ID, including the nil UUID, remains valid at this Rust API
boundary.

A transition or completion performs these steps in one transaction:

1. acquire the SQLite writer;
2. load, reconstruct, replay, and projection-check the current unit;
3. pass the caller's `expected_revision` to the core's guarded command;
4. encode the complete successor envelope and projections;
5. update only the matching ID, stored revision, and envelope version, requiring
   exactly one changed row; and
6. commit before returning the successor and committed revision.

A zero-row compare-and-set update is a `ConcurrentStorageChange` invariant
failure. Any failure before commit leaves the prior complete row in place.
Competing local writers are serialized, but a writer can receive `StorageBusy`
before its stored revision is examined. Once the writer lock is held, the
core's stale-revision check precedes terminal-state, target, edge, and
completion-eligibility checks. There is no backend retry.

Commit precedes the local process adapter's response write. If stdout body,
newline, or flush delivery later fails, the mutation can already be durable;
the client must retrieve the unit again before deciding what to do. The backend
does not claim response acknowledgement, rollback after commit, idempotency, or
safe blind retry.

## Retrieval and live keyset pages

`get` returns a full replay-validated view. `list` accepts a required limit from
1 through 100 and optional workflow-ID, species, phase, and status filters. All
present filters are combined using bound parameters and exact, case-sensitive
`BINARY` equality. A workflow-ID filter compares only the workflow ID; it does
not assert equal workflow topology among matching units.

Pages use canonical Intent Unit ID text in ascending `BINARY` lexical order.
That order is stable identity order, not creation time, lifecycle time, or UUID
chronology. The optional cursor is an exclusive `id > cursor` boundary, need
not identify an existing row, and may be the canonical nil UUID. The backend
fetches `limit + 1`; `next_cursor` is the last returned ID only when another
matching candidate exists, otherwise it is absent.

Every list request reads a live committed view. A later page is not part of a
cross-request snapshot: concurrent mutations can add, remove, or move units in
or out of the filter set. Every selected candidate, including the one-row
pagination lookahead, is replay- and projection-validated; one corrupt candidate
fails the entire page. No claim is made that a filtered query detects corruption
in rows outside its candidate set.

## Relationship contract version 1

A relationship definition has immutable identity `(definition ID, definition
version)`. Its ID is 1 through 64 ASCII bytes: the first byte is `a` through
`z`, and each remaining byte is a lowercase letter, digit, `.`, `_`, or `-`.
Validation reports empty, byte length, first byte, then the first invalid
remaining byte in that precedence, using a zero-based byte offset. Its version
is any positive `u64`, encoded in SQLite as eight big-endian bytes. Versions are
caller-owned labels and may be gapped or created out of numeric order; the
backend does not infer a latest, compatible, or superseding version.

Every relationship contract-v1 definition is directed and contains optional
exact source and target species constraints plus independent `Allow` or
`Reject` self-edge and cycle policies. Self policy alone governs
`source == target`; an allowed self-edge does not also encounter cycle policy.
Cycle policy governs only non-self path closure among edges of that exact
definition version. Definitions are immutable. Changing policy requires a new
version, and there is no definition update, list, deletion, or latest-version
operation.

Definition creation acquires `BEGIN IMMEDIATE`, inserts one complete immutable
row, and commits before returning its typed view. An exact valid collision is a
typed already-exists error; an exact collision whose selected stored values do
not decode is corrupt-definition instead of duplicate. Exact retrieval either
returns the selected version or a typed missing/corrupt error and never repairs
it.

An edge has immutable identity `(definition ID, definition version, source
Intent Unit ID, target Intent Unit ID)`. Edge creation acquires
`BEGIN IMMEDIATE`, then validates in this order: definition existence and row
validity; replay-valid source; replay-valid target; source species; target
species; self policy; exact duplicate; and non-self cycle policy. Cycle
reachability is directed and exact-definition-version scoped. Storage busy can
precede semantic classification because writer acquisition comes first. Any
failure before commit preserves all definitions, accepted edges, and endpoint
aggregates; an accepted edge changes no endpoint envelope, workflow, phase,
status, revision, or history.

Deletion names the same complete edge identity and, after writer acquisition,
validates the definition, replay-valid source, replay-valid target, source
species, target species, and exact edge before deleting exactly one row and
committing. A missing or corrupt selected definition, endpoint, or edge rejects
without removing anything; the complete key selects a candidate but does not
authorize deletion by itself. Deletion never cascades into definitions, other
edges, or Intent Units. Correction is an explicit committed delete followed by
a separate committed create. It is not one atomic replacement, has no
idempotency/retry token, and can expose an intermediate absence or competing
work. Physical deletion means removal from current relationship state, not
retained history or secure/forensic erasure.

Relationship failures have a separate typed `RelationshipError`: migration
required, definition duplicate/missing/corrupt, endpoint missing/corrupt/species
mismatch, self/cycle rejection, edge duplicate/missing/corrupt, and wrapped
backend storage/schema errors remain distinguishable without parsing SQLite
messages. Endpoint replay failures and wrapped backend failures retain their
Rust source error.

## Direct relationship queries

`list_relationships` requires one exact definition identity and a limit from 1
through 100. Optional exact source and target filters are ANDed. A missing
definition rejects, while a filter naming no stored endpoint yields an empty
page. Only direct edges are returned; there is no transitive expansion.

The exact definition is decoded before candidate selection. For each selected
or lookahead candidate, validation then decodes the complete edge identity,
replays the source endpoint, replays the target endpoint, checks the
definition's source-species constraint, and checks its target-species constraint
in that order. Species constraints therefore remain read-time invariants, not
create-time-only checks.

Pages use canonical `(source ID, target ID)` ascending `BINARY` order. The
exclusive `RelationshipCursor` carries the complete edge identity, must belong
to the query's exact definition, and is ordering state rather than a row
existence assertion. A cursor from another definition rejects before storage
access. The backend reads at most `limit + 1`; a next cursor is exposed only
after the lookahead edge and its endpoints validate, and then names the last
returned edge. Each request is a live committed view, not a cross-request
snapshot, so committed insertion or deletion can change later membership.
Corruption in a selected or lookahead definition, edge, or endpoint fails the
whole page without repair or partial results; SQL-filtered-out state is not
globally scanned.

`RelationshipPage` retains the complete input query, its validated direct-edge
items, and the optional next cursor.

## Ephemeral projection query version 1

`ProjectionQueryV1` combines existing exact lifecycle `ListFilters`, a required
limit from 1 through 100, an optional exclusive `ListCursor`, and at most one
typed direct relationship predicate. `Outgoing { definition, anchor }` returns
the anchor's direct targets; `Incoming { definition, anchor }` returns its
direct sources. The lifecycle filters and predicate are ANDed, and the exact
definition plus replay-valid anchor must exist. Omitting the relationship
predicate gives a versioned lifecycle-filter projection. No form performs
transitive traversal.

Relationship predicates preserve endpoint roles and definition species
constraints. Outgoing evaluation decodes the definition, replays the source
anchor and checks its source species, then validates each selected/lookahead
edge and replayed target candidate plus its target species. Incoming evaluation
decodes the definition, replays the target anchor and checks its target species,
then validates each selected/lookahead edge and replayed source candidate plus
its source species. The complete `limit + 1` selection validates before any
partial page or cursor can escape.

`ProjectionPage` retains the complete query, including version 1, and returns
validated `IntentUnitSummary` values in canonical Intent Unit ID order. Its
cursor and `limit + 1` lookahead have the same bounded, exclusive, live-page
semantics as lifecycle listing. The same query over unchanged canonical
committed units and edges is reproducible; later lifecycle or edge mutations
can change later pages, and no historical snapshot is implied. A selected or
lookahead corrupt definition, edge, anchor, or unit fails the whole page.

Projection queries and membership are never stored. One canonical unit can
appear in several projections without copied state, revision change, ownership
transfer, or a second lifecycle authority. A projection is a read model, not a
board-layout contract, dependency executor, scheduler, or agent graph.

## Failures and recovery boundary

`BackendError` distinguishes duplicate and missing IDs, exact core revision
conflicts, exact transition/completion rejections, unowned databases,
unsupported schema versions, corrupt schemas, unsupported envelope versions,
corrupt envelopes, projection mismatches, busy storage, compare-and-set
invariant failure, and other SQLite/local-filesystem failures. SQLite diagnostic
text is retained in the Rust error source chain where applicable; it is not
stable API or protocol text.

`MigrationError` separately distinguishes a source that is not exact version 1
from a source-retaining `BackendError`. `RelationshipError` is not mapped into
the local protocol-v1 code set. Callers should match typed variants, not error
display text.

Recovery is deliberately operator-driven: preserve the rejected file, diagnose
or restore it outside the running backend, and reopen only a database that
matches an exact owned contract. The only in-product evolution action is the
explicit v1-to-v2 migration described above; there is no automatic adoption,
repair, backup, replication, downgrade, reverse migration, Intent Unit
deletion, or definition deletion facility. `DELETE` journal mode and
`synchronous=EXTRA` are the selected local durability settings, but this
project does not claim immunity to process termination/crash-kill, filesystem,
device, operating-system, power-loss, or host failures.

## Explicit boundary and nonclaims

The supported versions target a caller-controlled local filesystem and embedded
SQLite. They do not support network filesystems or a network service. Multiple
CubiKan connections/process invocations can use the same supported local
database, and competing writers are serialized by SQLite; unrelated
applications or consumers must not share the file as directly writable storage
or edit rows themselves.

The backend does not provide:

- automatic migration, backup, replication, repair, downgrade/reverse
  migration, progress/resume/cancellation, or fixed migration duration;
- old-binary readability or indefinite schema, envelope, relationship,
  projection, protocol, core-serialization, or CLI compatibility;
- definition list/delete/latest/supersession/history, relationship
  revisions/history, actors, timestamps, idempotent correction, atomic
  replacement, cascade deletion, Intent Unit deletion, or forensic-erasure
  guarantees;
- stored boards, persisted queries/results, cross-request snapshots, transitive
  traversal, or arbitrary Boolean/OR/NOT graph queries;
- delegation, readiness, scheduling, automatic retries, fan-out/join, WIP
  limits, skill loading, artifact routing, executor policy, notifications, or
  board-layout policy;
- a relationship expansion to `cubikan-local` protocol v1, a network API,
  authentication or authorization, tenancy, encryption, privacy policy, UI,
  deployment, or shared writable service;
- direct core Serde persistence, exactly-once execution, cryptographic audit or
  tamper proof, provenance/agent/commit/blame tracking, metrics/KPI behavior, or
  blockchain/smart-contract policy; or
- latency, graph-scale, crash-kill, power-loss, device-loss, acknowledged
  delivery, or production-readiness guarantees.

The public Rust backend is the relationship/projection product boundary. A
manager, skill graph, or other consumer may use these primitives, but it owns
execution semantics and must not treat a projection as authorization or
scheduling state. Each excluded outcome requires separate intent.

See the [root overview](../../README.md) and
[INT-0010](../../docs/intents/INT-0010-durable-intent-unit-backend.md) for the
original durable boundary, and
[INT-0012](../../docs/intents/INT-0012-intent-unit-relationships-and-board-projections.md)
for relationship/projection authority and rationale.
