# `cubikan-backend`

`cubikan-backend` is CubiKan's synchronous, embedded SQLite boundary for
multiple durable Intent Units. A caller supplies one explicit local filesystem
path to `SqliteBackend::open`; the backend owns that file's schema and stored
representation. It does not search for a database, select a default path, or
accept SQLite's special `:memory:` path.

The public operations are `open(path)`, `create(CreateIntentUnit)`,
`get(IntentUnitId)`, `list(ListIntentUnits)`,
`transition(TransitionIntentUnit)`, and `complete(CompleteIntentUnit)`. This is
a concrete synchronous API, not a repository trait, async API, or `Send + Sync`
contract. See the [local process adapter](../cubikan-local/README.md) for the
separate JSON boundary.

## The three version 1 contracts

The durable boundary has three adapter-owned contracts:

1. the stored Intent Unit envelope version 1 described below;
2. the SQLite schema version 1 described below; and
3. the [local JSON protocol version 1](../cubikan-local/README.md).

None is the provisional `cubikan-core` Serde layout. A version number identifies
the current exact contract; it is not a promise of indefinite compatibility or
automatic migration to a future version.

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

## SQLite schema version 1

The database has `PRAGMA user_version = 1` and exactly one owned `STRICT` table:

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

Opening a new or truly empty database initializes that exact schema in one
transaction. Opening an existing version 1 database requires the exact owned
table and index definitions and no additional user objects. Version 0 with any
objects is unowned; another version, malformed schema, failed integrity check,
or non-SQLite content fails closed. The backend does not adopt, migrate, or
repair such a file. Ownership and version inspection precede the persistent
journal and synchronous settings. SQLite itself may touch a file while opening
it, so rejection promises no logical adoption or mutation by CubiKan, not
byte-for-byte preservation of every filesystem representation.

After its first inspection, `open` uses its own `BEGIN IMMEDIATE` transaction to
recheck ownership and either initialize or accept the schema before returning.
Open can therefore report busy storage as well as schema/storage rejection.

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

## Failures and recovery boundary

`BackendError` distinguishes duplicate and missing IDs, exact core revision
conflicts, exact transition/completion rejections, unowned databases,
unsupported schema versions, corrupt schemas, unsupported envelope versions,
corrupt envelopes, projection mismatches, busy storage, compare-and-set
invariant failure, and other SQLite/local-filesystem failures. SQLite diagnostic
text is retained in the Rust error source chain where applicable; it is not
stable API or protocol text.

Recovery is deliberately operator-driven: preserve the rejected file, diagnose
or restore it outside the running backend, and reopen only a database that
matches the exact owned contract. There is no automatic adoption, repair,
migration, backup, replication, or deletion facility. `DELETE` journal mode and
`synchronous=EXTRA` are the selected local durability settings, but this project
does not claim immunity to process termination/crash-kill, filesystem, device,
operating-system, power-loss, or host failures.

## Explicit boundary and nonclaims

Version 1 is for a caller-controlled local filesystem and embedded SQLite. It
does not support network filesystems or a network service. Multiple CubiKan
connections/process invocations can use the same supported local database, and
their competing writers are serialized by SQLite; unrelated applications or
consumers must not share the file as directly writable storage or edit rows
themselves.

This backend does not provide authentication or authorization, tenancy,
encryption, backup, replication, automatic migration, deletion, direct core
Serde persistence, retries, idempotency or exactly-once execution, cross-unit
transactions, indefinite schema/protocol compatibility, cryptographic audit or
tamper proof, metrics/KPI evaluation, agent/actor/commit provenance, cross-unit
relationships, a UI, deployment, or blockchain/network policy. Each requires a
separate product intent.

See the [root overview](../../README.md) and
[INT-0010](../../docs/intents/INT-0010-durable-intent-unit-backend.md) for the
project boundary and rationale.
