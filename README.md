# CubiKan

CubiKan is an exploration of blockchain-backed workflow coordination. Its core
concept is an **Intent Unit**: a uniquely identified unit of work that occupies a
caller-defined process phase until it reaches terminal completion.

## Current scope

`cubikan-core` is a chain-agnostic Rust library for defining and validating
Intent Unit lifecycles. The `cubikan` executable is an experimental stateless
JSON adapter that executes one complete caller-defined lifecycle scenario per
process. `cubikan-backend` adds a synchronous, embedded SQLite boundary for
multiple durable Intent Units, and the separate `cubikan-local` executable runs
one versioned durable operation per process against an explicit caller-selected
local database path.

The durable layers version their contracts independently. The stored Intent
Unit envelope remains version 1; SQLite schema versions 1 and 2 are supported;
the relationship contract and ephemeral projection query are each version 1;
and the `cubikan-local` JSON protocol remains version 1. Schema v2 adds durable
relationship definitions and edges without changing existing envelope bytes or
the local protocol. None of these contracts turns the provisional core Serde
layout or the stateless `cubikan` protocol into storage authority. No current
layer selects a blockchain, network service, deployment model, or user
interface.

## Core model

- An `IntentUnitId` is an opaque UUID-backed identity. Locally generated IDs use
  UUID v4 and carry no chronological ordering contract.
- An `IntentSpecies` is caller-defined text preserved on the Intent Unit for
  future completion and naming policy.
- A `Workflow` contains caller-declared phases, an initial phase, exact directed
  transition edges, and zero or more completion-eligible phases.
- Each Intent Unit owns an immutable validated workflow snapshot, so its
  lifecycle does not depend on a mutable external registry.
- A successful transition changes the current phase and appends a deterministic,
  one-based record. Undeclared edges and unknown phases leave the unit unchanged.
- Completion is allowed only at configured phases, appends one final record, and
  makes the unit terminal.
- Every new Intent Unit starts at aggregate-local revision
  `IntentUnitRevision::INITIAL` (numeric `0`). Each accepted conditioned or
  unconditioned transition or completion advances that revision exactly once;
  every rejected operation leaves it unchanged.
- Serialized workflows and Intent Units are restored through the same validation
  rules as live operations, including replay validation of the serialized
  revision. JSON is currently a test format, not a stable wire contract; a
  snapshot from an older schema that omits the required revision fails to
  restore.

The library does not export default Kanban phase names. A caller can declare a
simple forward workflow, explicit rework/backward edges, self edges, or arbitrary
custom phase labels.

## Revision-conditioned lifecycle mutations

`transition_to_if_revision` and `complete_if_revision` provide aggregate-local
optimistic conflict detection using the revision that the caller previously
observed. They compare that value before terminal-state, phase, edge, or
completion-eligibility validation. A stale value therefore returns the
`Conflict(RevisionConflict)` variant of `RevisionedTransitionError` or
`RevisionedCompletionError` even when the requested lifecycle operation would
also be invalid. Matching that variant exposes both `expected()` (the caller's
observation) and `actual()` (the aggregate's current revision), allowing the
caller to reject the command, refresh its view, and decide whether to retry. If
the supplied revision is current, normal domain validation runs and retains its
typed transition or completion error; a successful operation returns the newly
committed revision.

A revision is an opaque version of one Intent Unit, not a clock or a global
sequence. Lifecycle record sequence numbers are one-based positions in that
unit's history; revision values start at zero. Although both currently advance
with each accepted mutation, they are distinct contracts, and callers must not
derive one from the other.

The unconditioned `transition_to` and `complete` methods remain convenient for a
single owner that already has exclusive access to the in-memory aggregate. A
durable or multi-client adapter must instead receive the caller's previously
observed revision and pass it to the conditioned operation. Reading the revision
inside the adapter immediately before mutation would erase the stale-observation
check. The core comparison alone does not make storage writes atomic.
`cubikan-backend` supplies that boundary with one SQLite `BEGIN IMMEDIATE`
transaction, a revision-qualified compare-and-set update, and commit before
success returns; any future adapter must provide an equivalent guarantee.

## Stateless runnable JSON adapter

Run the checked-in configure → create → transition → complete example
from the repository root:

```sh
cargo run -p cubikan-cli --bin cubikan < crates/cubikan-cli/tests/fixtures/lifecycle-success-v1.json
```

The process reads one strict version 1 request from standard input and writes one
newline-terminated JSON success or typed error response to standard output. The
runner returns a modeled outcome only after the supplied output writer accepts
the JSON, the newline, and one explicit `flush()`; a failure at any of those
steps is an operational error (`1`) with a best-effort stderr diagnostic. This
checks only the supplied writer's flush contract, not durable or acknowledged
delivery. Other exit codes distinguish request or setup rejection (`2`) and
lifecycle rejection (`3`). See the
[`cubikan` protocol reference](crates/cubikan-cli/README.md) for complete request,
response, error-code, and partial-state semantics.

Raw request ingestion is capped at 1 MiB (`1_048_576` bytes), including JSON
whitespace. An input whose required next byte would exceed that ceiling receives
one `request_too_large` response and exit `2` before JSON classification. The
ceiling is a compile-time source constant, not a runtime setting. It bounds the
retained raw request payload only; it is not a total-memory guarantee or a claim
that the adapter is production-ready.

This is intentionally one-shot and in-memory. It does not preserve state between
invocations, and its experimental adapter-owned protocol is not a cross-version
compatibility promise.

## Durable local backend and process adapter

The durable boundary uses this version matrix:

| Contract | Supported version | Scope |
|----------|-------------------|-------|
| Stored Intent Unit envelope | 1 | Complete workflow snapshot and replayable lifecycle history |
| SQLite schema | 1 and 2 | Exact v1 lifecycle store; exact v2 adds relationship definitions and edges |
| Relationship contract | 1 | Immutable directed definition versions and exact current-state edges |
| Projection query | 1 | Ephemeral lifecycle/relationship views |
| `cubikan-local` JSON protocol | 1 | Create, get, list, transition, and complete only |

Every unit load reconstructs and validates the aggregate through
`cubikan-core`; unsupported envelopes, corrupt representations, and SQL
projection mismatches fail closed. `cubikan-local` continues to execute one
strict protocol-v1 lifecycle operation per process. The only invocation form is
an explicit database path:

```sh
cargo run -p cubikan-local --bin cubikan-local -- --database ./cubikan.sqlite3 < request.json
```

A fresh or truly empty path initializes exact schema v2. An exact schema-v1
file still opens without implicit schema migration, preserves its logical
schema and rows, and retains create, get, list, transition, and complete;
relationship-definition, edge, relationship-query, and projection methods
instead return typed migration-required. Migration never runs during
`open` or through `cubikan-local`. A Rust caller must explicitly invoke
`SqliteBackend::migrate_v1_to_v2(path)`, then drop and reopen backend handles.
An already-open v1 handle keeps its cached v1 capability after another
connection migrates the file: its existing lifecycle operations remain usable,
but its relationship/projection operations continue to report migration
required until reopen.

Migration opens an existing file read/write without creating a missing path,
acquires one immediate writer transaction, revalidates exact v1 and every
stored unit, adds only the exact v2 relationship objects, advances
`user_version` last, validates v2, and commits once. It preserves every
`intent_units` column value byte-for-byte; that is not a whole-file byte-layout
promise. There is no automatic retry. A busy or interrupted attempt against
accepted exact v1 leaves exact v1; in a race, one migrator may commit exact v2
and the loser then reports that its source is no longer version 1. A source
that was not acceptable exact v1—such as unowned, unsupported, malformed, or
non-SQLite input—is rejected in its unchanged prior logical state, which is not
claimed to be exact v1 or v2. Before migration, operators should quiesce writers
and preserve any desired recovery copy outside CubiKan. On rejection, preserve
and diagnose or restore the file, then reopen only an exact supported schema.
See the
[`cubikan-backend` migration and recovery contract](crates/cubikan-backend/README.md#explicit-v1-to-v2-migration)
for the complete procedure and exclusions.

The local adapter validates the complete request before opening the path, caps
raw stdin at 1 MiB, and writes one compact JSON response plus newline with one
explicit stdout flush. Its exits are `0` for success, `1` for operational
input/output delivery failure, `2` for usage/request rejection, `3` for
command/domain rejection, and `4` for storage rejection.

SQLite uses rollback-journal `DELETE`, `synchronous=EXTRA`, isolated/default
connections, and a 5,000-millisecond busy timeout. Create and guarded mutations
acquire a writer with `BEGIN IMMEDIATE`; transition and completion replay the
stored unit, preserve the core's stale-before-domain check, update through a
revision-qualified compare-and-set, and commit before success is written.
Writer contention can therefore produce `storage_busy` before a stale revision
is evaluated.

List filters are exact, case-sensitive matches on workflow ID, species, phase,
and status. Workflow-ID equality means the ID only, not equal topology. Limits
range from 1 through 100; pages use ascending lexical canonical-ID order and an
exclusive last-returned-ID cursor. Each request sees a live committed page, not
a snapshot across requests, so mutations can change later membership.

Relationship contract v1 stores immutable, directed definitions identified by
caller-owned `(definition ID, definition version)` and current-state edges
identified by `(definition ID, definition version, source Intent Unit ID,
target Intent Unit ID)`. Definition versions are positive `u64` labels, not a
latest-version sequence. A definition can constrain source and target species
and independently allow or reject self-edges and non-self cycles. Accepted
edges do not change either endpoint's workflow, phase, status, revision, or
history. Creation validates the definition, source, target, species, self-edge,
duplicate, and cycle conditions in the documented transaction order. Busy
writer acquisition can occur before those semantic checks.

Deletion requires and validates the complete edge identity, definition, both
replay-valid endpoints, and endpoint species before removing exactly one edge.
It is physical, non-cascading semantic removal. Correction is a committed
delete followed by a separate committed create, so it is neither an atomic
replacement nor idempotent and can expose an intermediate absence.

Direct relationship queries name one exact definition version, optionally AND
source and target filters, and return direct edges only in canonical
`(source,target)` order. Limits are 1 through 100 and the exclusive cursor
retains the complete edge identity; it is ordering state and need not name a
currently stored edge. Each request is a live committed page, not a snapshot.

Projection query v1 ANDs existing lifecycle filters with at most one direct
predicate: outgoing returns an anchor's direct targets, and incoming returns an
anchor's direct sources. Results are validated unit summaries in canonical ID
order with the existing bounded exclusive cursor. Queries and results are not
stored board membership. The same unit can appear in multiple projections
without copied state or ownership transfer. Unchanged canonical state produces
the same result, while later committed lifecycle or edge changes can change a
later live page. Projection is a read model, not a scheduler or execution graph.

A commit can succeed before stdout body, newline, or flush delivery fails. In
that case the client does not know the committed outcome from process delivery
alone and must retrieve the unit and refresh its revision before deciding what
to do; rollback, retry safety, and idempotency are not implied. See the
[`cubikan-backend` contract](crates/cubikan-backend/README.md) and
[`cubikan-local` protocol](crates/cubikan-local/README.md) for the exact
envelope, schema, recovery, pagination, response, error-code, and exit
contracts.

## Species provenance and future naming

Sprint 0 preserves an Intent Unit's immutable species through every transition
and after completion. It does **not** define a completed-unit naming grammar or
parent/child lineage model. Those are product decisions that must be specified
before the core exposes them as stable behavior.

## Development

The workspace requires current-stable Rust with Rust 2024 edition support. The
[Rust CI](.github/workflows/ci.yml) workflow runs on GitHub-hosted Ubuntu for
pull requests targeting `dev` or `main` and pushes to `dev` or `main`.
Reproduce its five quality gates from the repository root in this order:

```sh
cargo +stable fmt --all -- --check
cargo +stable clippy --workspace --all-targets --all-features -- -D warnings
RUSTFLAGS="-D warnings" cargo +stable check --workspace --all-targets
cargo +stable test --workspace --all-targets
cargo +stable test --doc --workspace
```

The workflow produces a GitHub status for review. It does not configure
required branch protection or authorize a merge; retained human merge approval
remains required. This CI boundary intentionally adds no caches, artifacts,
coverage/security scanners, secrets, releases/deployment, automatic merge
(auto-merge), MSRV, or OS/toolchain matrices. These CI nonclaims do not change
the product boundaries below.

## Explicit current exclusions

The current durable boundary is a single-tenant, unencrypted, embedded SQLite
file on a caller-controlled local filesystem. It does not support network
filesystems or a network service. Multiple CubiKan backend connections and
`cubikan-local` processes can use the same supported local database under
SQLite's writer serialization; unrelated consumers are not allowed shared
direct write access or row editing.

The current project does not provide:

- authentication or authorization, tenancy, encryption, or privacy policy;
- automatic migration, backup, replication, repair, import, downgrade, reverse
  migration, migration progress/resume/cancellation, or a fixed migration
  duration;
- direct persistence of the provisional core Serde representation;
- automatic retries, idempotency keys, exactly-once execution, or retry-safety
  guarantees;
- definition listing/deletion, latest-version inference, definition or
  relationship history, relationship revisions, actors, timestamps, idempotent
  correction, atomic edge replacement, cascade deletion, Intent Unit deletion,
  or forensic-erasure guarantees;
- stored boards, stored query results, cross-request snapshots, transitive
  traversal, or an arbitrary Boolean graph-query language;
- delegation, readiness, scheduling, retries, fan-out/join, WIP limits, skill
  loading, artifact routing, or executor policy;
- old-binary readability or indefinite stable schema, envelope, relationship,
  projection, protocol, core-serialization, or CLI compatibility;
- a blockchain, network, smart contract, cryptographic audit/tamper proof, or
  blockchain policy;
- KPI/metrics evaluation or automatic transition authorization;
- agent, actor, commit, blame, or other provenance tracking;
- a service/API, UI, durable interactive application session, or deployment
  model;
- clocks, timestamps, or global chronological ordering across Intent Units; or
- default phase topology or completed-unit naming syntax.

The existing `cubikan` executable remains stateless and its version 1 protocol
still has no revision fields or revision-conditioned commands. The durable
relationship and projection APIs are currently a public Rust
`cubikan-backend` boundary only; `cubikan-local` protocol v1 adds no relationship
operations, fields, results, or error codes. The stored envelope-v1 codec,
`cubikan-core`, stateless `cubikan`, workspace manifests/lockfile, and CI
workflow remain outside the schema-v2 relationship change. The local E2E
unsupported-schema fixture alone now uses version 3 because version 2 is
supported. The local raw byte ceiling, busy timeout, rollback journal, and
explicit flush do not claim a total request deadline, network controls, crash
immunity, acknowledged response delivery, or production readiness.

These boundaries keep the domain foundation testable and allow later product
policy to be selected through explicit intent rather than embedded assumptions.

## License

CubiKan is available under the [MIT License](LICENSE).
