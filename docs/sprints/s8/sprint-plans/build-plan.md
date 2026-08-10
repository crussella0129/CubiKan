Finalized - DO NOT EDIT

# Sprint 8 Build Plan

## Intents

- [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) — state: `planned`; acceptance criteria covered: restart-safe multi-unit storage, bounded filtered pagination, a separate versioned boundary, replay-validated load, transactionally guarded mutations, actual-process lifecycle continuity, and precise storage/recovery documentation.
- [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) — state: `realized`; preserved dependency: stale-first aggregate conflict semantics are consumed unchanged by T-806 and exposed by T-807/T-808.

## Schema Tree

- Realize a durable multi-unit CubiKan boundary
  - Backend-owned values
    - T-801: Scaffold `cubikan-backend` and define its public value contract
    - T-802: Implement the strict replay-validated envelope
  - Embedded SQLite persistence
    - T-803: Own and validate SQLite schema v1
    - T-804: Add transactional create/get
    - T-805: Add bounded filtered keyset queries
    - T-806: Add revision-guarded transactional mutations
  - Versioned local process boundary
    - T-807: Define local protocol v1 and execute every command
    - T-808: Add the bounded runner and `cubikan-local` executable
  - Consumer proof
    - T-809: Prove cross-process continuity and fail-closed process behavior
    - T-810: Document the first backend boundary and nonclaims

## Locked Version 1 Contracts

### Backend boundary

`cubikan-backend` exposes a concrete synchronous `SqliteBackend`; this sprint
does not introduce a repository trait, async runtime, or `Send + Sync`
promise. Its public operations are `open(path)`, `create(CreateIntentUnit)`,
`get(IntentUnitId)`, `list(ListIntentUnits)`,
`transition(TransitionIntentUnit)`, and `complete(CompleteIntentUnit)`.
Commands carry core-typed IDs, workflow values, phases, species, status, and
`IntentUnitRevision`; adapter-owned full views and summaries expose immutable
copies. Create accepts an optional ID and generates the existing non-nil UUID v4
only when it is absent. Supplied syntactically valid IDs, including nil UUID,
remain valid.

The public typed failure taxonomy distinguishes duplicate ID, missing ID,
exact core revision conflict, exact core transition/completion rejection,
unowned database, unsupported schema version, corrupt schema, unsupported
envelope version, corrupt envelope, projection mismatch, busy storage,
concurrent-storage/CAS invariant failure, and other storage failure. SQLite
messages remain error sources, not stable protocol text.

Backend commands and views retain typed `IntentUnitRevision` values across the
full `u64` range. At JSON boundaries, revision text is canonical unsigned
decimal: `"0"` or a nonzero value without a leading zero, fitting `u64`. The
SQL revision key is exactly eight big-endian bytes. List limits are required
and range from 1 through 100. A
cursor is a canonical lowercase hyphenated `IntentUnitId` string equal to the
core value's `Display`; a canonical nil UUID is valid.

### Stored envelope v1

The stored JSON object denies unknown fields at every nesting level and contains
exactly:

- `representation_version: 1`;
- `id`, `species`, `phase`, and `revision` (canonical decimal string);
- `status` tagged as `active` or `completed`;
- `workflow` with `id`, caller-ordered `phases`, `initial_phase`,
  caller-ordered `edges[{from,to}]`, and caller-ordered
  `completion_phases`;
- `history`, in order, whose records are
  `{type:"transition",sequence,from,to}` or
  `{type:"completion",sequence,phase}`, with one-based unsigned integer
  sequences.

Decode reconstructs every validated vocabulary value and `Workflow`, creates
the unit at revision 0, replays each record through ordinary core lifecycle
methods, and compares the declared identity, workflow, phase, status, revision,
and exact history to the replayed aggregate. The codec never calls Serde on
`IntentUnit` or treats its provisional layout as storage authority.

### SQLite schema v1

The one owned table is a SQLite `STRICT` table named `intent_units` with
these columns and constraints:

- `id TEXT NOT NULL PRIMARY KEY COLLATE BINARY`;
- `envelope_version INTEGER NOT NULL CHECK(envelope_version = 1)`;
- `envelope TEXT NOT NULL`;
- `workflow_id TEXT NOT NULL COLLATE BINARY`;
- `species TEXT NOT NULL COLLATE BINARY`;
- `phase TEXT NOT NULL COLLATE BINARY`;
- `status TEXT NOT NULL COLLATE BINARY CHECK(status IN ('active','completed'))`;
- `revision BLOB NOT NULL CHECK(length(revision) = 8)`.

Four indexes are owned: `intent_units_by_workflow(workflow_id,id)`,
`intent_units_by_species(species,id)`,
`intent_units_by_phase(phase,id)`, and
`intent_units_by_status(status,id)`, all using binary comparison. Schema
`user_version` is 1. Initialization is allowed only for a new or truly empty
database and is one transaction. Existing version 1 must have exactly the
owned table/index definitions and no unexpected user objects; version 0 with
objects, any other version, malformed schema, or non-SQLite content fails
closed without migration.

Every connection selects and verifies rollback journal `DELETE`,
`synchronous=EXTRA`, `foreign_keys=ON`, `trusted_schema=OFF`,
`read_uncommitted=OFF`, isolated/default connection behavior, and a
5,000-millisecond busy timeout before work. Shared cache and network filesystems
are not enabled or supported.

Open rejects an empty path and SQLite's special `:memory:` path. It inspects an
existing database's version and objects before assigning persistent PRAGMAs, so
rejecting an unowned database does not silently adopt or reconfigure it.

### Local JSON protocol v1

Every request is one strict object
`{protocol_version:1,operation:{...}}`. Operation shapes are:

- create:
  `{type:"create",intent_unit:{id?:string,species:string},workflow:{id,phases,initial_phase,edges:[{from,to}],completion_phases}}`;
- get: `{type:"get",id:string}`;
- list:
  `{type:"list",filters:{workflow_id?:string,species?:string,phase?:string,status?:"active"|"completed"},limit:integer,after?:string}`;
- transition:
  `{type:"transition",id:string,target:string,expected_revision:string}`;
- complete: `{type:"complete",id:string,expected_revision:string}`.

Unknown fields, missing fields, wrong types, and explicit null for optional
string fields are invalid; omission alone selects absence. Responses are one
strict object. Success is
`{protocol_version:1,outcome:"success",result:{...}}`, where result is exactly
one of:

- `{type:"unit",intent_unit:FULL_UNIT}`;
- `{type:"page",items:[SUMMARY...],next_cursor:string|null}`;
- `{type:"mutation",committed_revision:string,intent_unit:FULL_UNIT}`.

`FULL_UNIT` contains exactly `id`, `species`, `workflow`, `phase`, `status`,
`revision`, and `history`; its workflow/history shapes match the locked
envelope fields without `representation_version`. `SUMMARY` contains exactly
`id`, `species`, `workflow_id`, `phase`, `status`, and `revision`. All revisions
are decimal strings. Failure is
`{protocol_version:1,outcome:"failure",error:{code,message,field?,expected_revision?,actual_revision?}}`.
`message` is human-readable and not a compatibility oracle; `field` appears
only for field validation, and expected/actual appear only for conflicts.

Protocol v1 error codes are exactly:

- request: `malformed_json`, `request_too_large`, `invalid_request`,
  `unsupported_protocol_version`, `invalid_intent_unit_id`,
  `invalid_species`, `invalid_workflow_id`, `invalid_phase_id`,
  `invalid_workflow`, `invalid_query`, `invalid_revision`;
- command/domain: `duplicate_intent_unit`, `intent_unit_not_found`,
  `revision_conflict`, `transition_already_completed`,
  `transition_unknown_target`, `transition_not_allowed`,
  `completion_already_completed`,
  `completion_phase_not_eligible`;
- storage: `storage_busy`, `unowned_database`,
  `unsupported_schema_version`, `corrupt_schema`,
  `unsupported_envelope_version`, `corrupt_envelope`, `projection_mismatch`,
  `concurrent_storage_change`, `storage_error`.

JSON syntax maps to `malformed_json`; structurally missing/unknown/wrong-typed
or null members map to `invalid_request`; a non-1 version maps to
`unsupported_protocol_version`. Invalid operation IDs, species, workflow IDs,
phase values, topology, and list bounds/cursor/status map respectively to the
named `invalid_*` codes above with a field path. A semantically noncanonical or
overflowing `expected_revision` string maps to `invalid_revision` with its
field path; a non-string revision remains structural `invalid_request`.
Duplicate/missing/conflict and core lifecycle variants map one-to-one to their
command/domain codes. Backend
variants map one-to-one to the storage codes; `storage_error` is reserved for
unclassified SQLite/I/O failure. Exact expected/actual strings appear only on
`revision_conflict`.

`cubikan-local --database PATH` is the only accepted invocation. It reads one
request of at most 1,048,576 raw bytes from stdin, writes one compact response
plus newline to stdout, and flushes once. Exit classes are 0 success, 2 usage or
request rejection, 3 command/domain rejection, 4 modeled storage rejection,
and 1 operational stdin/stdout/stderr failure. Modeled responses leave stderr
empty. This is a new experimental contract, not `cubikan` protocol v2.

## Execution Sequence

### T-801: Scaffold `cubikan-backend` and define its public value contract

- **Intent:** [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md)
- **Touches:** `Cargo.toml`, `Cargo.lock`, `crates/cubikan-backend/Cargo.toml`, `crates/cubikan-backend/src/lib.rs`, `crates/cubikan-backend/src/model.rs`, `crates/cubikan-backend/src/error.rs`, `crates/cubikan-backend/tests/model.rs`, `docs/SUMMARY.md`, `docs/intents/INT-0010-durable-intent-unit-backend.md`, `docs/sprints/s8/**`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Depends on:** (none)
- **Acceptance criterion:** The backend has a reusable adapter-owned command/query/value boundary whose revisions, limits, and cursors preserve the full core domain without selecting storage or transport behavior.
- **Success criterion (EARS):**
  - **T-801-E1 — WHEN** workspace metadata is resolved, **THEN** it **SHALL** contain a separate Rust 2024 `cubikan-backend` crate depending only on `cubikan-core`, Serde, and Serde JSON at this task boundary, without changing dependencies of `cubikan-core` or `cubikan-cli`.
  - **T-801-E2 — WHEN** a caller constructs backend commands and reads backend results, **THEN** the public model **SHALL** represent create/get/list/transition/complete, external expected revisions, complete unit views, summaries, pages, and typed failures without exposing stored DTOs.
  - **T-801-E3 — WHEN** backend commands, views, summaries, pages, conflicts, or mutation results carry a revision, **THEN** the public model **SHALL** preserve the exact typed `IntentUnitRevision` through `u64::MAX` without signed narrowing or text conversion.
  - **T-801-E4 — WHEN** a list limit or cursor is constructed, **THEN** it **SHALL** accept limits 1–100 and every canonical lowercase hyphenated core ID (including nil), while rejecting out-of-range limits and noncanonical/malformed cursor text before storage access.
- **Notes:** The first semantic task commit also includes the initialized Sprint 8 Book/research, finalized plans, INT-0010 state/work evidence, metadata, and queued ledgers.

### T-802: Implement the strict replay-validated storage envelope

- **Intent:** [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md)
- **Touches:** `crates/cubikan-backend/src/lib.rs`, `crates/cubikan-backend/src/model.rs`, `crates/cubikan-backend/src/error.rs`, `crates/cubikan-backend/src/stored.rs`, `crates/cubikan-backend/tests/common/mod.rs`, `crates/cubikan-backend/tests/envelope.rs`
- **Depends on:** T-801
- **Acceptance criterion:** Every load validates a complete adapter-owned representation through `cubikan-core`; unsupported or inconsistent representations fail closed, and core Serde is not promoted to storage authority.
- **Success criterion (EARS):**
  - **T-802-E1 — WHEN** an active or completed unit is encoded to the locked envelope v1 and decoded, **THEN** the codec **SHALL** reconstruct vocabulary/workflow, replay every record through core behavior, and return an equivalent complete view.
  - **T-802-E2 — WHEN** envelope lifecycle sequence/source/edge/completion/final state/revision is malformed or inconsistent, **THEN** decode **SHALL** return typed corruption without yielding an aggregate.
  - **T-802-E3 — WHEN** envelope v1 has a missing or unknown field, invalid vocabulary/workflow, or a representation version other than 1, **THEN** decode **SHALL** distinguish malformed/corrupt state from unsupported version and SHALL NOT normalize it.
  - **T-802-E4 — WHEN** revisions `0`, `i64::MAX + 1`, or `u64::MAX` cross the adapter codecs, **THEN** JSON text and the exact eight-byte big-endian SQL projection **SHALL** preserve the value without signed narrowing; invalid text or blob length SHALL fail.
- **Notes:** Stored record sequences are checked as one-based and contiguous. No SQLite access exists in this task.

### T-803: Own, initialize, and validate SQLite schema v1

- **Intent:** [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md)
- **Touches:** `Cargo.toml`, `Cargo.lock`, `crates/cubikan-backend/Cargo.toml`, `crates/cubikan-backend/src/lib.rs`, `crates/cubikan-backend/src/error.rs`, `crates/cubikan-backend/src/schema.rs`, `crates/cubikan-backend/src/sqlite.rs`, `crates/cubikan-backend/tests/common/mod.rs`, `crates/cubikan-backend/tests/schema.rs`
- **Depends on:** T-802
- **Acceptance criterion:** Storage/schema ownership, initialization, connection configuration, and fail-closed version behavior are explicit and independently verified before CRUD.
- **Success criterion (EARS):**
  - **T-803-E1 — WHEN** `SqliteBackend` opens a caller-selected new or truly empty local path, **THEN** it **SHALL** configure persistent/connection PRAGMAs outside transactions where SQLite requires it, recheck ownership after acquiring `BEGIN IMMEDIATE`, transactionally create exactly the locked strict schema and `user_version=1`, then verify every locked PRAGMA with a 5,000-millisecond busy timeout.
  - **T-803-E2 — WHEN** an exact owned schema-v1 database reopens, **THEN** the backend **SHALL** validate and preserve it without migration or reinitialization.
  - **T-803-E3 — WHEN** open sees version 0 with user objects, an unsupported version, incomplete/extra/wrong version-1 objects, or non-SQLite content, **THEN** it **SHALL** classify ownership/version before persistent PRAGMA assignment and fail closed without logical schema repair, deletion, migration, or adoption.
  - **T-803-E4 — WHEN** dependency scope is inspected, **THEN** `rusqlite` 0.40.2 with `default-features = false` and `bundled` **SHALL** exist only in `cubikan-backend`; the core and existing CLI dependency graphs SHALL remain unchanged.
- **Notes:** SQLite open may touch file headers, so failure tests compare logical schema/version/content rather than promising byte-identical rejected database files.

### T-804: Add transactional durable create and replay-validated get

- **Intent:** [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md)
- **Touches:** `crates/cubikan-backend/src/lib.rs`, `crates/cubikan-backend/src/model.rs`, `crates/cubikan-backend/src/error.rs`, `crates/cubikan-backend/src/sqlite.rs`, `crates/cubikan-backend/tests/common/mod.rs`, `crates/cubikan-backend/tests/persistence.rs`, `crates/cubikan-backend/tests/corruption.rs`
- **Depends on:** T-803
- **Acceptance criterion:** Multiple units survive restart; valid loads replay through core; duplicate, missing, corrupt, and inconsistent rows preserve durable state.
- **Success criterion (EARS):**
  - **T-804-E1 — WHEN** create receives a valid workflow/species and optional supplied ID, **THEN** one immediate transaction **SHALL** insert one complete revision-0 envelope plus matching projections, commit before return, preserve a supplied ID, or generate a non-nil UUID v4 only when absent.
  - **T-804-E2 — WHEN** multiple units are created, connections close, and the database reopens, **THEN** get by stable ID **SHALL** replay and return each exact immutable workflow, identity, species, phase, status, revision, and ordered history independently.
  - **T-804-E3 — WHEN** create receives a duplicate ID or get receives an unknown ID, **THEN** the backend **SHALL** return distinct typed outcomes and leave every accepted row logically unchanged.
  - **T-804-E4 — WHEN** an envelope, envelope version, or any duplicated ID/workflow/species/phase/status/revision projection disagrees with replay, **THEN** get **SHALL** return the corresponding typed corruption/unsupported/projection error without repairing, deleting, or returning the row.
- **Notes:** Full views are derived from the replayed aggregate, never from unchecked projection columns.

### T-805: Add bounded exact-filter live keyset pagination

- **Intent:** [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md)
- **Touches:** `crates/cubikan-backend/src/lib.rs`, `crates/cubikan-backend/src/model.rs`, `crates/cubikan-backend/src/query.rs`, `crates/cubikan-backend/src/sqlite.rs`, `crates/cubikan-backend/tests/common/mod.rs`, `crates/cubikan-backend/tests/query.rs`
- **Depends on:** T-804
- **Acceptance criterion:** A bounded paginated query discovers units by the locked lifecycle projections with deterministic order, exclusive cursor semantics, validation, and explicit live-page consistency.
- **Success criterion (EARS):**
  - **T-805-E1 — WHEN** list receives workflow/species/phase/status filters singly or combined, **THEN** it **SHALL** use bound parameters and case-sensitive exact equality, return only matching replay-validated summaries, and SHALL NOT interpret derivative-domain data or equate a shared workflow ID with identical topology.
  - **T-805-E2 — WHEN** list receives a limit 1–100 and optional canonical cursor, **THEN** it **SHALL** fetch at most `limit + 1`, return at most `limit` in canonical-ID lexical order, exclude IDs at or before the cursor, and expose the last returned ID only when another match exists.
  - **T-805-E3 — WHEN** rows are inserted or mutable filter membership changes between page requests, **THEN** each page **SHALL** reflect its own committed view after the exclusive cursor without claiming snapshot consistency.
  - **T-805-E4 — WHEN** any selected candidate row is corrupt or projection-inconsistent, **THEN** list **SHALL** fail the whole page without returning partial or unchecked summaries; it SHALL NOT claim to detect corrupt rows excluded by SQL filters.
- **Notes:** A valid cursor need not identify an existing row. It is ordering state, not offset, authorization, or snapshot state.

### T-806: Add revision-guarded transition and completion transactions

- **Intent:** [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md), preserving [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md)
- **Touches:** `crates/cubikan-backend/src/lib.rs`, `crates/cubikan-backend/src/model.rs`, `crates/cubikan-backend/src/error.rs`, `crates/cubikan-backend/src/sqlite.rs`, `crates/cubikan-backend/tests/common/mod.rs`, `crates/cubikan-backend/tests/mutations.rs`, `crates/cubikan-backend/tests/corruption.rs`
- **Depends on:** T-804
- **Acceptance criterion:** Successful mutation commits one complete update before return; missing, corrupt, stale, busy, concurrent, and domain-rejected commands preserve prior durable state.
- **Success criterion (EARS):**
  - **T-806-E1 — WHEN** transition or completion receives the current revision and a valid command, **THEN** one `BEGIN IMMEDIATE` transaction **SHALL** replay the row, invoke the matching core guarded command, update the full envelope/projections with `WHERE id = ? AND revision = ? AND envelope_version = 1`, require one changed row, commit, and return the exact successor visible to a fresh connection.
  - **T-806-E2 — WHEN** expected revision is stale, including when the command is also domain-invalid, **THEN** after acquiring the transaction the backend **SHALL** return exact expected/actual conflict before domain validation and preserve the row.
  - **T-806-E3 — WHEN** revision is current but the target is missing/corrupt or core rejects transition/completion, **THEN** the backend **SHALL** return the matching typed error and preserve the row.
  - **T-806-E4 — WHEN** two isolated connections act from the same observed revision, **THEN** exactly one valid write **SHALL** commit and the later writer **SHALL** observe conflict without overwrite; zero rows from the revision-qualified update SHALL fail closed and roll back.
  - **T-806-E5 — WHEN** another writer holds the database beyond 5,000 milliseconds or SQLite aborts the update, **THEN** mutation **SHALL** return one typed operational failure without retry and leave the prior row intact.
- **Notes:** Busy can occur before stale evaluation because `BEGIN IMMEDIATE` must acquire the writer; stale-first applies after acquisition. No cross-unit transaction, retry, idempotency key, lease, or merge is added.

### T-807: Define local protocol v1 and execute every backend command

- **Intent:** [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md)
- **Touches:** `Cargo.toml`, `Cargo.lock`, `crates/cubikan-local/Cargo.toml`, `crates/cubikan-local/src/lib.rs`, `crates/cubikan-local/src/protocol.rs`, `crates/cubikan-local/src/execution.rs`, `crates/cubikan-local/tests/protocol.rs`
- **Depends on:** T-805, T-806
- **Acceptance criterion:** Create/get/list/transition/complete use the locked adapter-owned protocol v1 rather than core Serde or direct storage editing.
- **Success criterion (EARS):**
  - **T-807-E1 — WHEN** each locked protocol-v1 operation is decoded, **THEN** it **SHALL** preserve the exact operation/field/tag shape, reject unknown/missing/wrong-typed fields and unsupported versions, and construct validated backend/core values without opening storage for a structurally invalid request.
  - **T-807-E2 — WHEN** request or response carries a revision, **THEN** every unit, summary, mutation, expected, and actual revision **SHALL** use canonical decimal-string form and never a JSON number.
  - **T-807-E3 — WHEN** backend execution succeeds, **THEN** the adapter **SHALL** emit exactly the locked adapter-owned unit/page/mutation result shape and SHALL NOT serialize a core or stored DTO directly.
  - **T-807-E4 — WHEN** validation, command/domain, or storage failure occurs, **THEN** execution **SHALL** map exhaustively to exactly one locked error code (including `invalid_revision` for semantic revision text), include `field` only for field validation, and include expected/actual only for revision conflict.
  - **T-807-E5 — WHEN** a modeled command fails, **THEN** the executor **SHALL** emit no false success and the real backend state **SHALL** retain T-804/T-806 atomicity.
- **Notes:** Human-readable messages may improve without defining protocol compatibility; codes and structural fields are the v1 oracle.

### T-808: Add the bounded runner and `cubikan-local` executable

- **Intent:** [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md)
- **Touches:** `crates/cubikan-local/Cargo.toml`, `crates/cubikan-local/src/lib.rs`, `crates/cubikan-local/src/runner.rs`, `crates/cubikan-local/src/main.rs`, `crates/cubikan-local/tests/runner.rs`
- **Depends on:** T-807
- **Acceptance criterion:** Independent process invocations use one explicit local database path and emit one bounded, writer-flush-checked protocol response with precise commit/delivery semantics.
- **Success criterion (EARS):**
  - **T-808-E1 — WHEN** invocation contains exactly `--database PATH`, **THEN** the executable **SHALL** use only that path; missing/empty/`:memory:`/repeated/unknown/positional arguments SHALL produce usage exit 2 on stderr before database open and no default path SHALL be selected.
  - **T-808-E2 — WHEN** stdin contains at most 1,048,576 bytes and a structurally valid request, **THEN** the runner **SHALL** open the database only after request validation, execute exactly one operation, write one compact response plus newline, flush once, and return the modeled status only after flush succeeds.
  - **T-808-E3 — WHEN** raw input requires byte 1,048,577, **THEN** the runner **SHALL** retain at most 1,048,577 raw bytes and classify `request_too_large` before JSON or database behavior.
  - **T-808-E4 — WHEN** stdin, response body, newline, or flush fails, **THEN** the shell **SHALL** return exit 1 with a best-effort stderr diagnostic and no modeled status; earlier errors SHALL prevent later output stages.
  - **T-808-E5 — WHEN** a mutation commits but response delivery then fails, **THEN** the runner **SHALL** report operational delivery uncertainty while a fresh read still observes the commit, without claiming rollback or retry safety.
  - **T-808-E6 — WHEN** modeled requests finish, **THEN** locked request codes SHALL map to exit 2, command/domain codes to 3, storage codes to 4, success to 0, and modeled stderr SHALL remain empty.
- **Notes:** A database path that cannot be opened is a modeled `storage_error` response/exit 4 if stdout remains usable; argument errors are not JSON protocol responses.

### T-809: Prove cross-process continuity and fail-closed process behavior

- **Intent:** [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md), preserving [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md)
- **Touches:** `crates/cubikan-local/tests/cli_e2e.rs`, `crates/cubikan-local/tests/fixtures/`
- **Depends on:** T-808
- **Acceptance criterion:** Actual processes demonstrate restart-safe multi-unit lifecycle, pagination, stale rejection, completion, final retrieval, and fail-closed unsupported storage.
- **Success criterion (EARS):**
  - **T-809-E1 — WHEN** actual `cubikan-local` processes share one fresh explicit database, **THEN** separate invocations **SHALL** create two units, retrieve/filter/page them in lexical ID order, transition from revision 0, reject a stale competing command unchanged, continue from the refreshed revision, complete, exit, and retrieve the exact final workflow/history/status/revision.
  - **T-809-E2 — WHEN** actual processes open unsupported or malformed schema-v1 fixtures, **THEN** they **SHALL** fail closed with the locked storage code/exit and preserve logical version/content.
- **Notes:** Use the Cargo-built executable and a real test-owned local file; no repository mock or in-memory SQLite substitutes for E2E.

### T-810: Document the first backend boundary and nonclaims

- **Intent:** [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md)
- **Touches:** `crates/cubikan-backend/README.md`, `crates/cubikan-local/README.md`, `README.md`
- **Depends on:** T-809
- **Acceptance criterion:** Consumers can operate the selected local backend with precise schema, storage, concurrency, recovery, pagination, and delivery guarantees without inferring stronger product policy.
- **Success criterion (EARS):**
  - **T-810-E1 — WHEN** consumers read backend/local/root documentation, **THEN** it **SHALL** define all three v1 contracts, explicit path, replay/projection validation, transaction/commit/busy behavior, workflow-ID filter meaning, keyset/live-page semantics, exits, local-filesystem assumptions, and post-commit delivery uncertainty.
  - **T-810-E2 — WHEN** documentation describes non-goals, **THEN** it **SHALL** exclude network filesystems/services, auth/tenancy/encryption, backup/replication/migrations, deletion, shared writable storage, direct core Serde, retries/idempotency, cross-unit transactions, stable compatibility, cryptographic audit, metrics, provenance, relationships, UI, deployment, and blockchain policy.
- **Notes:** Scope, Book/link validation, workspace quality gates, existing CLI regression, and exact-head hosted CI are Test-phase final gates rather than a mixed Build commit. No derivative repository is created.
