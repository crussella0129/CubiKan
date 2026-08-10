# `cubikan-local` durable JSON adapter

`cubikan-local` executes exactly one durable backend operation against one
explicit local SQLite path. It is separate from the existing
[`cubikan`](../cubikan-cli/README.md) executable: `cubikan` still runs one
stateless in-memory lifecycle scenario, while `cubikan-local` preserves many
Intent Units across independent process invocations.

## Invocation and request ingestion

The only accepted invocation is:

```sh
cubikan-local --database PATH
```

From the repository, a request file can be run with:

```sh
cargo run -p cubikan-local --bin cubikan-local -- --database ./cubikan.sqlite3 < request.json
```

`PATH` is mandatory, literal, and used only for this invocation. Missing,
empty, repeated, unknown, positional, and `:memory:` arguments produce the usage
line on stderr and exit 2 before request execution. No environment variable,
working-directory convention, or default database is selected.

The process reads one request from stdin. The complete raw input, including JSON
whitespace, is limited to 1 MiB (`1_048_576` bytes). The runner retains at most
one lookahead byte. If byte 1,048,577 is required, `request_too_large` takes
precedence over JSON classification and the database is not opened. The bound
limits retained raw request bytes, not total process memory or execution time.

For an input within the bound, the adapter completes JSON syntax, strict
structure, protocol-version, and semantic field validation before opening the
database. A malformed or invalid request therefore cannot initialize or inspect
the selected path.

## Local JSON protocol version 1

Every request is one strict object:

```text
{protocol_version: 1, operation: OPERATION}
```

`OPERATION` is exactly one of:

```text
{
  type: "create",
  intent_unit: {id?: string, species: string},
  workflow: {
    id: string,
    phases: [string...],
    initial_phase: string,
    edges: [{from: string, to: string}...],
    completion_phases: [string...]
  }
}

{type: "get", id: string}

{
  type: "list",
  filters: {
    workflow_id?: string,
    species?: string,
    phase?: string,
    status?: "active" | "completed"
  },
  limit: integer,
  after?: string
}

{type: "transition", id: string, target: string, expected_revision: string}

{type: "complete", id: string, expected_revision: string}
```

Unknown fields, missing required fields, wrong JSON types, and explicit `null`
for optional strings are rejected; only omission selects absence. Operation IDs
and list cursors use canonical lowercase hyphenated UUID text. Revision values
are strings containing canonical unsigned decimal text: `"0"` or a nonzero
value without a leading zero, within the full `u64` range. List limits are
integers from 1 through 100. Vocabulary and workflow topology must satisfy the
same core validation used by live aggregates.

This local protocol stays version 1 while the independently versioned backend
supports stored envelope v1, SQLite schemas v1 and v2, relationship contract
v1, and projection query v1. It is not `cubikan` protocol version 2, does not
expose `cubikan-core` Serde, and does not acquire relationship or projection
operations merely because schema v2 exists. See the
[`cubikan-backend` version matrix](../cubikan-backend/README.md#version-matrix).

## Success responses

Success is one strict object:

```text
{protocol_version: 1, outcome: "success", result: RESULT}
```

`RESULT` is exactly one of:

```text
{type: "unit", intent_unit: FULL_UNIT}

{type: "page", items: [SUMMARY...], next_cursor: string | null}

{
  type: "mutation",
  committed_revision: string,
  intent_unit: FULL_UNIT
}
```

`FULL_UNIT` contains exactly:

```text
{
  id: string,
  species: string,
  workflow: {
    id: string,
    phases: [string...],
    initial_phase: string,
    edges: [{from: string, to: string}...],
    completion_phases: [string...]
  },
  phase: string,
  status: "active" | "completed",
  revision: string,
  history: [
    {type: "transition", sequence: unsigned integer, from: string, to: string}
    | {type: "completion", sequence: unsigned integer, phase: string}
  ...]
}
```

`SUMMARY` contains exactly `id`, `species`, `workflow_id`, `phase`, `status`,
and `revision`. Every response revision, including `committed_revision`, is a
canonical unsigned-decimal string.

## Failure responses and exact codes

Failure is:

```text
{
  protocol_version: 1,
  outcome: "failure",
  error: {
    code: string,
    message: string,
    field?: string,
    expected_revision?: string,
    actual_revision?: string
  }
}
```

`field` appears only for semantic field validation. `expected_revision` and
`actual_revision` appear only for `revision_conflict`. Messages are for humans
and are not stable machine-readable contract text.

Request codes are:

- `malformed_json`;
- `request_too_large`;
- `invalid_request`;
- `unsupported_protocol_version`;
- `invalid_intent_unit_id`;
- `invalid_species`;
- `invalid_workflow_id`;
- `invalid_phase_id`;
- `invalid_workflow`;
- `invalid_query`; and
- `invalid_revision`.

Command/domain codes are:

- `duplicate_intent_unit`;
- `intent_unit_not_found`;
- `revision_conflict`;
- `transition_already_completed`;
- `transition_unknown_target`;
- `transition_not_allowed`;
- `completion_already_completed`; and
- `completion_phase_not_eligible`.

Storage codes are:

- `storage_busy`;
- `unowned_database`;
- `unsupported_schema_version`;
- `corrupt_schema`;
- `unsupported_envelope_version`;
- `corrupt_envelope`;
- `projection_mismatch`;
- `concurrent_storage_change`; and
- `storage_error`.

## Process output and exits

For a modeled outcome, stdout receives exactly one compact JSON body, one
newline, and one explicit flush, in that order. The modeled exit is returned
only after all three output stages succeed, and modeled outcomes leave stderr
empty.

| Exit | Meaning |
|------|---------|
| `0` | backend operation succeeded |
| `1` | operational stdin/stdout delivery failure; no modeled status is available |
| `2` | usage or request rejection |
| `3` | command/domain rejection |
| `4` | modeled storage rejection |

Usage errors write the usage line to stderr rather than a JSON response. An
operational read, response-body, newline, or flush failure returns exit 1 with a
best-effort stderr diagnostic. An unusable database path is normally a modeled
`storage_error` and exit 4 when stdout remains usable.

The explicit flush checks only the supplied stdout writer's contract. It does
not prove operating-system, filesystem, pipe peer, or external-reader receipt.
For transition and completion, the backend commits before response writing.
Consequently, a body, newline, or flush failure can leave the client uncertain
even though the successor is durable. After any such delivery failure, retrieve
the Intent Unit from the explicit database path and use its observed revision;
do not assume rollback or blindly replay the mutation.

## Schema compatibility and Rust-only relationship boundary

Opening a fresh or truly empty path through `cubikan-local` initializes exact
SQLite schema v2. Exact schema v1 also remains openable without implicit schema
migration and preserves its logical schema and rows. Both versions support all
five protocol-v1 operations—create, get, list, transition, and complete—with
the same request, result, failure-code, stdout, stderr, and exit contracts
documented above. An unsupported nonzero version, such as version 3, remains
`unsupported_schema_version` and exit 4; version 0 with user objects remains
`unowned_database`, malformed v1/v2 remains `corrupt_schema`, and a truly empty
version-0 file initializes v2. None changes the production protocol shape.

`cubikan-local` never migrates a database and has no relationship-definition,
edge, relationship-list, or projection request. Those capabilities are exposed
only by the public Rust `cubikan-backend` API on schema v2. To upgrade exact v1,
an operator must stop or quiesce local writers, preserve any desired external
recovery copy, call `SqliteBackend::migrate_v1_to_v2(path)` from a Rust consumer,
and reopen handles/processes after success. A pre-migration backend handle keeps
its cached v1 relationship capability until reopen even if its lifecycle
operations continue after another connection migrates the file.

Migration is explicit, read/write-without-create, non-retrying, and atomic. It
replay-validates every unit, preserves every `intent_units` column value
byte-for-byte, installs only exact v2 relationship objects, sets the schema
version last, validates v2, and commits once. Busy or interruption against
accepted exact v1 leaves exact v1; a race can leave committed exact v2. An
unowned, unsupported, malformed, non-SQLite, or otherwise unacceptable source
is rejected in its unchanged prior logical state, not converted into an exact
schema. The backend does not provide the adapter with automatic backup, repair,
downgrade/reverse migration, progress/resume/cancellation, fixed duration, or
old-binary compatibility. Follow the
[`cubikan-backend` migration and recovery procedure](../cubikan-backend/README.md#explicit-v1-to-v2-migration)
rather than editing the file directly.

## Storage and pagination semantics

The process delegates storage ownership, envelope replay, SQL projection
checking, exact schema validation, transaction/CAS behavior, `DELETE` journal
mode, `synchronous=EXTRA`, and the 5,000-millisecond busy timeout to
[`cubikan-backend`](../cubikan-backend/README.md). A busy writer may be rejected
before stale-revision evaluation. Once the writer is acquired, a stale revision
is reported before other lifecycle rejection, and pre-commit failures preserve
the previous durable row. Opening the backend performs its own immediate
ownership recheck, and reads can also report local SQLite busy/locked storage;
the 5,000-millisecond setting is a lock wait bound rather than a total request
deadline.

List filters use exact case-sensitive equality and are combined. A
`workflow_id` filter means equal workflow ID only, not equal workflow topology.
Pages are ordered by canonical ID in lexical order, use an exclusive
last-returned-ID keyset cursor, and are live committed views. They are not
chronological and do not form a snapshot across process invocations; mutations
between requests can change filter membership. Returned rows and the one-row
lookahead candidate are replay- and projection-validated, while filtered-out
rows are not inspected by that query.

## Explicit boundary and nonclaims

`cubikan-local` and its backend support a caller-controlled local filesystem.
They do not support network filesystems, a network service, or shared direct
write access by unrelated consumers. Competing writers from multiple supported
local invocations are serialized through SQLite; direct row editing is outside
the contract.

Protocol v1 does not add relationship definitions, edges, direct queries, or
projections to its operation, field, result, or error-code sets. Relationship
contract v1 and projection query v1 are Rust-backend APIs only; neither is a
local protocol-v2 promise.

The local boundary does not provide automatic migration, backup, replication,
repair, downgrade/reverse migration, migration progress/resume/cancellation, or
a fixed migration duration. It does not promise old-binary readability or
indefinite stable schema, envelope, relationship, projection, protocol,
core-serialization, or CLI compatibility.

It also does not provide definition listing/deletion/latest resolution,
relationship history/revisions/actors/timestamps, idempotent correction, atomic
edge replacement, cascade deletion, Intent Unit deletion, forensic erasure,
stored boards/results, snapshot pages, transitive or arbitrary Boolean graph
queries, delegation, scheduling, retries, WIP/executor policy, authentication or
authorization, tenancy, encryption, network transport, a UI, deployment,
provenance, metrics/KPI behavior, or blockchain policy. The 1 MiB input bound
and finite SQLite busy timeout do not make this a production network service or
supply a total request deadline. No crash-kill, power-loss,
acknowledged-delivery, or stable-compatibility guarantee is added at the process
boundary.

See the [root overview](../../README.md) and
[INT-0010](../../docs/intents/INT-0010-durable-intent-unit-backend.md) for the
original durable boundary, and
[INT-0012](../../docs/intents/INT-0012-intent-unit-relationships-and-board-projections.md)
for the Rust-only relationship/projection boundary.
