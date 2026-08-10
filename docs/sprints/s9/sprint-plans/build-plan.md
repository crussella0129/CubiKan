Finalized - DO NOT EDIT

# Sprint 9 Build Plan

## Intents

- [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) — state: planned; acceptance criteria covered: immutable versioned definitions, validated directed relationships, explicit policy and deletion semantics, atomic rejection, multi-projection membership, reproducible versioned live queries, exact schema-v2 evolution, and separation from execution graphs/local protocol v1.

## Schema Tree

- Durable relationships and projections above the lifecycle aggregate
  - T-901: public relationship/projection value and error contract
  - T-902: exact schema v2 and explicit atomic v1-to-v2 migration
  - T-903: immutable relationship-definition creation and retrieval
  - T-904: validated relationship creation and deletion
  - T-905: bounded direct relationship queries
  - T-906: ephemeral board-projection query v1
  - T-907: operator/consumer documentation and scope preservation
  - T-908: public-backend real-file composition proof

## Locked Design Decisions

- `RelationshipDefinitionId` is an adapter-owned canonical ASCII identifier of
  1 through 64 bytes: the first byte is `a`–`z`; remaining bytes are lowercase
  letters, digits, `.`, `_`, or `-`. Definition version is a positive `u64`,
  stored as an eight-byte big-endian BLOB. The complete immutable definition
  identity is `(id, version)`.
- All definitions in relationship contract v1 are directed and contain optional exact source/target
  `IntentSpecies` constraints plus independent `Allow | Reject` self-edge and
  cycle policies. Self policy exclusively governs `source == target`; when a
  self-edge is allowed, cycle policy is not applied to that length-one edge.
  Cycle policy governs only non-self path closure inside one definition version.
- The complete immutable edge identity is `(definition id, definition version,
  source IntentUnitId, target IntentUnitId)`. Exact duplicates reject. Correction
  is delete-and-recreate. Delete names the complete identity, validates the
  selected definition, both replay-valid endpoints, and applicable endpoint
  species constraints, removes only that edge, and is non-cascading. Missing or corrupt selected definitions/endpoints fail
  closed and cannot be deleted as an implicit repair path: the full key selects
  the candidate but does not alone authorize removal. A partial deletion identity
  is unrepresentable by the public command type.
- Edge-create validation precedence after the writer is acquired is: definition
  existence/row validity; source replay; target replay; source then target species;
  self policy; exact duplicate; then non-self cycle policy. Storage busy can
  precede semantic checks. Relationship-only failures use a separate public
  `RelationshipError` that can wrap existing `BackendError`; local protocol v1
  does not acquire unreachable relationship codes.
- Exact schema v1 remains openable for existing create/get/list/transition/
  complete operations and reports `BackendSchemaVersion::V1`; exact v2 reports
  `V2`. Relationship operations on v1 return typed
  `MigrationRequired { found: 1, required: 2 }`. Fresh/empty stores initialize
  exact v2. Migration is only `SqliteBackend::migrate_v1_to_v2(path)`; it is
  explicit, non-retrying, and never runs during `open`. Callers must reopen
  backend handles after a successful migration; existing handles are not
  auto-upgraded. A second racing migrator that observes committed v2 returns
  typed `SourceVersionNotOne { found: 2 }`.
- `SqliteBackend` caches its post-lock `BackendSchemaVersion`.
  `schema_version()` returns that cached capability without storage access, and
  every new definition/relationship/list/projection method checks it before a
  transaction or relationship SQL. If another connection migrates the file, an
  already-open v1 handle continues to report v1: existing lifecycle operations
  remain usable, but all new operations still return migration-required until
  the caller drops and reopens it. No stale-handle auto-upgrade, downgrade
  detection, or hostile external schema-replacement guarantee is selected.
- Schema v2 preserves the existing `intent_units` table, its four explicit
  indexes and implicit primary-key autoindex, and every stored envelope-v1 byte.
  The two added tables are locked to these literal declarations (including
  column order, nullability, collation, checks, primary-key order, and FK action):

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

  The only new explicit indexes are literal
  `CREATE INDEX relationship_edges_by_source ON intent_unit_relationships(definition_id,definition_version,source_id,target_id)`
  and
  `CREATE INDEX relationship_edges_by_target ON intent_unit_relationships(definition_id,definition_version,target_id,source_id)`.
  Exact v2 therefore has twelve owned `sqlite_schema` objects: the six exact v1
  objects; both new tables; implicit autoindexes
  `sqlite_autoindex_relationship_definitions_1` and
  `sqlite_autoindex_intent_unit_relationships_1`; and those two indexes. It has
  no trigger, view, generated column, partial index, or other object and uses
  `user_version=2`. Both added tables report `(wr=0, strict=1)`; composite PK
  positions follow declaration order; all three FKs are immediate, `MATCH NONE`,
  and `RESTRICT` on update/delete; `integrity_check` returns only `ok`; and
  `foreign_key_check` returns no row. Open validates that structural/constraint
  boundary but does not globally replay relationship state. Each operation owns
  semantic decoding of the exact definition, edge, endpoint, candidate, and
  lookahead rows it selects. Definition/edge rows have no timestamp, actor,
  mutable board membership, or lifecycle revision.
- The public contract is concrete and synchronous. It adds
  `BackendSchemaVersion::{V1,V2}`, `RelationshipDefinitionId`,
  `RelationshipDefinitionVersion`, `RelationshipDefinitionKey`,
  `RelationshipDirection::Directed`, `RelationshipPolicy::{Allow,Reject}`,
  `RelationshipEndpoint::{Source,Target}`, typed definition/edge commands and
  views, `RelationshipIdentity`, `RelationshipCursor`, `ListRelationships`,
  `RelationshipPage`, `DirectRelationshipPredicate`, `ProjectionQueryV1`, and
  `ProjectionPage`. `SqliteBackend` exposes `schema_version`,
  `create_relationship_definition`, `get_relationship_definition`,
  `create_relationship`, `delete_relationship`, `list_relationships`, and
  `project`; associated `migrate_v1_to_v2(path)` is the only migration entrypoint.

  ```rust
  pub const fn schema_version(&self) -> BackendSchemaVersion;
  pub fn migrate_v1_to_v2(path: impl AsRef<Path>) -> Result<(), MigrationError>;
  pub fn create_relationship_definition(&mut self, command: CreateRelationshipDefinition)
      -> Result<RelationshipDefinitionView, RelationshipError>;
  pub fn get_relationship_definition(&self, key: RelationshipDefinitionKey)
      -> Result<RelationshipDefinitionView, RelationshipError>;
  pub fn create_relationship(&mut self, command: CreateRelationship)
      -> Result<RelationshipView, RelationshipError>;
  pub fn delete_relationship(&mut self, command: DeleteRelationship)
      -> Result<RelationshipView, RelationshipError>;
  pub fn list_relationships(&self, query: ListRelationships)
      -> Result<RelationshipPage, RelationshipError>;
  pub fn project(&self, query: ProjectionQueryV1)
      -> Result<ProjectionPage, RelationshipError>;
  ```

  `RelationshipDefinitionKey` contains definition ID/version.
  `CreateRelationshipDefinition` and `RelationshipDefinitionView` contain that
  key, directed-only direction, optional source/target species, and self/cycle
  policies. `RelationshipIdentity` contains definition key/source/target;
  create/delete/view each contain that complete identity. `ListRelationships`
  contains definition, optional source/target, limit, and optional cursor;
  `RelationshipPage` retains the query, items, and optional next cursor.
  `ProjectionQueryV1` contains lifecycle filters, optional direct predicate,
  limit, and optional `ListCursor`; `ProjectionPage` retains the query, unit
  summaries, and optional next cursor. `ProjectionQueryV1::VERSION` and its
  `version()` accessor return 1. `RelationshipCursor` is a typed newtype
  over complete `RelationshipIdentity`; Sprint 9 selects no textual cursor or
  Serde contract. Definition-ID validation precedence is empty, byte length,
  first byte, then remaining byte; the invalid-character index is a zero-based
  byte offset.
- Definition-ID construction returns
  `RelationshipDefinitionIdError::{Empty,TooLong { bytes },InvalidStart,
  InvalidCharacter { index }}`; zero definition version returns
  `RelationshipDefinitionVersionError::Zero`; cross-definition query cursors
  return `RelationshipQueryError::CursorDefinitionMismatch { expected, actual }`.
  Runtime and migration errors are exhaustively locked to:

  ```rust
  enum RelationshipError {
      MigrationRequired {
          found: BackendSchemaVersion,
          required: BackendSchemaVersion,
      },
      DefinitionAlreadyExists { definition: RelationshipDefinitionKey },
      DefinitionNotFound { definition: RelationshipDefinitionKey },
      CorruptDefinition { definition: RelationshipDefinitionKey },
      EndpointNotFound { endpoint: RelationshipEndpoint, id: IntentUnitId },
      EndpointCorrupt {
          endpoint: RelationshipEndpoint,
          id: IntentUnitId,
          source: BackendError,
      },
      EndpointSpeciesMismatch {
          endpoint: RelationshipEndpoint,
          id: IntentUnitId,
          expected: IntentSpecies,
          actual: IntentSpecies,
      },
      SelfEdgeRejected { relationship: RelationshipIdentity },
      CycleRejected { relationship: RelationshipIdentity },
      DuplicateRelationship { relationship: RelationshipIdentity },
      RelationshipNotFound { relationship: RelationshipIdentity },
      CorruptRelationship { definition: RelationshipDefinitionKey },
      Backend(BackendError),
  }

  enum MigrationError {
      SourceVersionNotOne { found: i64 },
      Backend(BackendError),
  }
  ```

  `EndpointCorrupt`, `RelationshipError::Backend`, and
  `MigrationError::Backend` preserve their nested source through `Error::source`.
  `EndpointCorrupt.source` is limited to endpoint replay failures
  (`UnsupportedEnvelopeVersion`, `CorruptEnvelope`, or `ProjectionMismatch`);
  busy and other storage/schema failures use `RelationshipError::Backend`.
  Migration opens read/write without create. Empty version-0 input maps to
  `SourceVersionNotOne { found: 0 }`; v0 with user objects to wrapped
  `UnownedDatabase`; exact v2 to `SourceVersionNotOne { found: 2 }`; other
  versions to wrapped `UnsupportedSchemaVersion`; malformed SQLite/schema to
  wrapped `CorruptSchema`; invalid v1 unit state to its matching wrapped backend
  error; and busy/other open or storage failures to their source-retaining
  wrapped backend error. A nonexistent path is not created.
  Existing `BackendError` and local JSON error-code enums do not gain variants.
- `RelationshipCursor` retains the complete edge identity. `ListRelationships`
  requires one exact definition version, rejects a cursor from another
  definition before storage access, accepts optional exact source and target
  filters (ANDed), orders by canonical `(source, target)`, uses `PageLimit`
  `1..=100`, and carries an exclusive cursor that is ordering state and need not
  name a currently stored edge.
  Missing definitions reject; a filter naming no stored endpoint simply yields
  no matches. Pages are live committed views and validate every selected plus
  lookahead row; corruption excluded by SQL filters is not globally discovered.
- `ProjectionQueryV1` contains existing lifecycle `ListFilters`, `PageLimit`, an
  optional `ListCursor`, and at most one typed direct predicate:
  `Outgoing { definition, anchor }` returns targets, while
  `Incoming { definition, anchor }` returns sources. All predicates are ANDed;
  the definition and anchor must exist and validate. `ProjectionPage` returns the
  complete query (including version 1), validated `IntentUnitSummary` items, and
  an exclusive next cursor. It stores no board or membership state.
- Rust public backend API plus a real SQLite file is the Sprint 9 product boundary.
  `cubikan-local` JSON protocol v1, stored Intent Unit envelope v1,
  `cubikan-core`, dependencies, CI, network/auth/tenant/UI/execution/provenance/
  metric/blockchain policy, transitive traversal, arbitrary Boolean queries,
  definition deletion, relationship history, and Intent Unit deletion remain
  unchanged or excluded.

## Execution Sequence

### T-901: Add the public relationship and projection value contract
- **Intent:** [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- **Touches:** `crates/cubikan-backend/src/lib.rs`, `crates/cubikan-backend/src/relationship.rs`, `crates/cubikan-backend/src/projection.rs`, `crates/cubikan-backend/tests/relationship_model.rs`, `docs/SUMMARY.md`, `docs/intents/INT-0012-intent-unit-relationships-and-board-projections.md`, `docs/sprints/s9/**`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Depends on:** (none)
- **Acceptance criterion:** Relationship definitions state direction, endpoint constraints, duplicate, self-edge, cycle, correction, and deletion behavior; queries identify exact versions and remain adapter-owned.
- **Success criterion (EARS):**
  - **T-901-E1 — WHEN** callers construct definition identifiers, definition versions, policies, edge identities, limits, or cursors, **THEN** the public model **SHALL** enforce the locked canonical grammar, positive/full-`u64` version range, complete identity, and `1..=100` page bounds without narrowing core IDs.
  - **T-901-E2 — WHEN** callers construct definition, edge, relationship-query, or projection-query commands and results, **THEN** the public model **SHALL** expose every locked field, version, direction, filter, predicate, item, and cursor while making more than one direct projection predicate unrepresentable.
  - **T-901-E3 — WHEN** relationship validation, lookup, policy, duplicate, missing-edge, corruption, busy, or storage failures occur, **THEN** `RelationshipError` **SHALL** preserve distinct typed classifications and sources without requiring SQLite-message parsing or adding variants to the existing local JSON error taxonomy.
  - **T-901-E4 — WHEN** the new model is compared with public core and stored DTO boundaries, **THEN** it **SHALL** reuse typed CubiKan identities but **SHALL NOT** expose private SQL rows, provisional core Serde, timestamps, actors, stored boards, or an execution-graph API.
- **Notes:** Public modules remain private and are re-exported from `lib.rs`. T-901 defines and tests `BackendSchemaVersion` and `MigrationError` in `relationship.rs` as public values; T-902 alone creates `migration.rs` and adds schema/migration behavior. Keep migration errors separate from relationship errors and preserve existing `BackendError` exhaustiveness. The first semantic task commit also includes the initialized Sprint 9 Book/research, finalized plans, INT-0012 state/work evidence, metadata, and queued ledgers.

### T-902: Introduce exact schema v2 and explicit atomic v1-to-v2 migration
- **Intent:** [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- **Touches:** `crates/cubikan-backend/src/lib.rs`, `crates/cubikan-backend/src/error.rs`, `crates/cubikan-backend/src/schema.rs`, `crates/cubikan-backend/src/sqlite.rs`, `crates/cubikan-backend/src/migration.rs`, `crates/cubikan-backend/tests/common/mod.rs`, `crates/cubikan-backend/tests/schema.rs`, `crates/cubikan-backend/tests/migration.rs`, `crates/cubikan-local/tests/cli_e2e.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Depends on:** T-901
- **Acceptance criterion:** Fresh stores use exact schema v2; exact v1 remains usable for existing operations and changes only through explicit, atomic, content-preserving migration.
- **Success criterion (EARS):**
  - **T-902-E1 — WHEN** a fresh/empty store initializes or exact v2 reopens, **THEN** the backend **SHALL** validate the locked literal v2 DDL, complete twelve-object inventory, columns, constraints, immediate `MATCH NONE` restrictive foreign keys, indexes, implicit autoindexes, rowid/`STRICT` flags, `user_version=2`, integrity and foreign-key checks, and the preserved DELETE/EXTRA/foreign-keys-on/trusted-schema-off/read-uncommitted-off/locking-normal/5000-ms-busy connection contract without repair or reinitialization.
  - **T-902-E2 — WHEN** ordinary open receives exact v1, **THEN** it **SHALL** return a backend reporting cached schema version 1, retain all existing unit operations and bytes, make the private relationship-capability guard return migration-required before relationship SQL, and leave schema, rows, envelopes, and projections unchanged.
  - **T-902-E3 — WHEN** explicit migration receives replay-valid exact v1, **THEN** it **SHALL** acquire `BEGIN IMMEDIATE`, revalidate schema and every unit, add only exact v2 objects, set `user_version=2` last, run integrity/foreign-key/exact-v2 validation, commit once, preserve every existing `intent_units` column value byte-for-byte, and require callers to reopen while any pre-migration handle retains its cached v1 capability.
  - **T-902-E4 — WHEN** migration receives a nonexistent path, empty or unowned v0, malformed/corrupt v1, already-v2, unsupported-version, or non-SQLite input, **THEN** it **SHALL** return the locked typed source error without creating a missing file, adoption, repair, partial objects, or row mutation.
  - **T-902-E5 — WHEN** migration is busy, is deliberately interrupted before commit, or races another migrator, **THEN** it **SHALL** return once without retry, preserve one exact prior-or-successor schema, and make the racing loser after a successful commit report source version 2.
  - **T-902-E6 — WHEN** v2 has missing, wrong, extra, physically corrupt, constraint-invalid, or foreign-key-invalid state, **THEN** open **SHALL** fail closed before returning a backend and **SHALL NOT** repair, delete, or reinitialize it.
- **Notes:** Keep `foreign_keys=ON` per connection. Existing local E2E's unsupported-schema fixture moves from version 2 to version 3 while its protocol-v1 response shape/code remains unchanged. Migration rollback may use an in-module test-only interruption seam around production transaction steps; do not add a public fault hook.

### T-903: Persist immutable relationship definitions
- **Intent:** [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- **Touches:** `crates/cubikan-backend/src/lib.rs`, `crates/cubikan-backend/src/sqlite.rs`, `crates/cubikan-backend/src/relationship_store.rs`, `crates/cubikan-backend/tests/common/mod.rs`, `crates/cubikan-backend/tests/relationship_definitions.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Depends on:** T-902
- **Acceptance criterion:** A caller can create and query immutable versioned definitions carrying explicit direction, endpoint, self-edge, and cycle policy.
- **Success criterion (EARS):**
  - **T-903-E1 — WHEN** a valid definition is created on v2, **THEN** one immediate transaction **SHALL** commit its exact identity, directed marker, optional species constraints, and policies before returning a typed view that survives reopen.
  - **T-903-E2 — WHEN** the same definition ID is used with a different positive version, **THEN** the backend **SHALL** retain both immutable versions independently and exact get **SHALL** return the requested version.
  - **T-903-E3 — WHEN** create repeats an exact identity whose stored definition decodes validly or get names a missing identity, **THEN** the backend **SHALL** return duplicate or not-found respectively and leave every accepted definition and unit row unchanged.
  - **T-903-E4 — WHEN** a selected definition retains a valid exact identity but its directed marker, endpoint constraint, or policy representation is corrupt, **THEN** exact create collision and get **SHALL** fail as corrupt relationship state without reporting duplicate, repairing, or deleting it.
- **Notes:** There is no update/list/delete definition surface in Sprint 9. Definition rows are typed adapter state, not JSON envelopes.

### T-904: Create and delete validated directed relationships atomically
- **Intent:** [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- **Touches:** `crates/cubikan-backend/src/sqlite.rs`, `crates/cubikan-backend/src/relationship_store.rs`, `crates/cubikan-backend/tests/common/mod.rs`, `crates/cubikan-backend/tests/relationship_mutations.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Depends on:** T-903
- **Acceptance criterion:** Valid edges connect existing replay-valid units; every endpoint/policy rejection is atomic; explicit deletion/correction never mutates lifecycle state.
- **Success criterion (EARS):**
  - **T-904-E1 — WHEN** a valid edge is created, **THEN** one immediate transaction **SHALL** apply the locked validation precedence, insert the complete directed identity, commit before return, survive reopen, and leave both endpoint envelopes, projections, revisions, and histories unchanged.
  - **T-904-E2 — WHEN** creation encounters a missing/corrupt definition, missing/corrupt endpoint, source/target species mismatch, rejected self-edge, exact duplicate, corrupt edge identity visited by cycle reachability, or rejected non-self cycle, **THEN** it **SHALL** return the precise typed error in locked precedence and preserve all units and accepted relationship rows.
  - **T-904-E3 — WHEN** all four self/cycle policy combinations and multiple definition versions are exercised, **THEN** self policy **SHALL** exclusively govern length-one edges and cycle traversal **SHALL** govern only non-self paths inside the exact definition version.
  - **T-904-E4 — WHEN** independent writers concurrently propose opposite edges that together violate one reject-cycle definition, **THEN** SQLite writer serialization plus in-transaction path validation **SHALL** commit exactly one edge and reject the other as a cycle, with no partial or forbidden edge.
  - **T-904-E5 — WHEN** deletion names an existing complete edge identity, **THEN** it **SHALL** remove only that edge and permit an explicit replacement; **WHEN** a different complete identity is absent or selected definition/endpoint validation, writer acquisition, or SQLite deletion fails, **THEN** deletion **SHALL** preserve every relationship, definition, and endpoint without cascade or retry.
- **Notes:** Use a cycle-safe recursive CTE with bound parameters and `UNION`, scoped by definition ID/version. After acquiring the writer, delete validates in this order: definition load/decode; source replay; target replay; source species; target species; exact-edge existence; one-row `DELETE`; commit. Deletion is semantic removal, not forensic erasure or retained history.

### T-905: Add bounded direct relationship queries
- **Intent:** [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- **Touches:** `crates/cubikan-backend/src/sqlite.rs`, `crates/cubikan-backend/src/relationship_store.rs`, `crates/cubikan-backend/tests/common/mod.rs`, `crates/cubikan-backend/tests/relationship_query.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Depends on:** T-904
- **Acceptance criterion:** Callers can query validated typed relationships with bounded, deterministic, documented live pagination.
- **Success criterion (EARS):**
  - **T-905-E1 — WHEN** callers query an existing exact definition version with optional source and target filters, **THEN** the backend **SHALL** AND the filters and return only direct validated edges in canonical `(source, target)` order without transitive expansion.
  - **T-905-E2 — WHEN** callers paginate with limits 1 or 100 and an exclusive composite cursor, **THEN** pages **SHALL** contain at most the limit, never repeat the cursor edge, expose a next cursor only after validated lookahead, and reflect current committed membership on each request; limits 0/101 reject in the public model.
  - **T-905-E3 — WHEN** the required definition is missing, **THEN** list **SHALL** return definition-not-found; optional filters that name no stored endpoint **SHALL** yield an empty page rather than an endpoint error.
  - **T-905-E4 — WHEN** any selected or lookahead definition/edge/endpoint state is corrupt, **THEN** the whole page **SHALL** fail without partial results or repair, while corruption excluded by the SQL filters is outside that query's detection claim.
- **Notes:** All dynamic values are bound parameters and all ordering/equality is explicit `BINARY` over canonical UUID text and definition fields.

### T-906: Add ephemeral board-projection query version 1
- **Intent:** [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- **Touches:** `crates/cubikan-backend/src/sqlite.rs`, `crates/cubikan-backend/src/query.rs`, `crates/cubikan-backend/src/relationship_store.rs`, `crates/cubikan-backend/tests/common/mod.rs`, `crates/cubikan-backend/tests/projection.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Depends on:** T-905
- **Acceptance criterion:** Units can appear in multiple reproducible versioned board/portfolio projections without copied lifecycle state or execution-graph policy.
- **Success criterion (EARS):**
  - **T-906-E1 — WHEN** projection v1 contains lifecycle filters and one outgoing or incoming direct predicate, **THEN** the backend **SHALL** validate the exact definition/anchor, AND every filter, return only the direct target/source units respectively, and perform no transitive traversal.
  - **T-906-E2 — WHEN** different typed queries select the same unit, **THEN** the unit **SHALL** appear in multiple projections as one validated summary without stored membership, copied aggregate state, revision change, or ownership transfer; edge/lifecycle changes **SHALL** affect later membership only through canonical committed state.
  - **T-906-E3 — WHEN** projection v1 evaluates unchanged state or paginates later committed state, **THEN** the page **SHALL** return its complete versioned query, canonical-ID ordered summaries, exclusive cursor, reproducible unchanged results, and documented live-page behavior.
  - **T-906-E4 — WHEN** the definition or anchor is missing, **THEN** projection **SHALL** return the precise typed error; when any selected/lookahead unit, definition, or relationship is corrupt, the whole page **SHALL** fail without partial summaries or repair.
- **Notes:** Projection without a relationship predicate remains a versioned lifecycle-filter query. Do not persist the query or its result.

### T-907: Document the relationship, migration, and projection boundary
- **Intent:** [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- **Touches:** `README.md`, `crates/cubikan-backend/README.md`, `crates/cubikan-local/README.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Depends on:** T-906
- **Acceptance criterion:** Documentation states the selected durable/query guarantees and keeps board projections separate from execution graphs and protocol/UI policy.
- **Success criterion (EARS):**
  - **T-907-E1 — WHEN** a backend consumer reads the guides, **THEN** they **SHALL** find exact schema-v1/v2 compatibility and migration/reopen behavior, definition/edge identity and validation, delete/recreate semantics, transaction/error precedence, direct-query pagination, projection version/reproducibility/live-page behavior, and recovery limits.
  - **T-907-E2 — WHEN** the guides describe nonclaims, **THEN** they **SHALL** explicitly exclude automatic migration, backup, downgrade/reverse migration, progress/resume or fixed-duration guarantees, old-binary or indefinite compatibility, definition listing/deletion/latest/history, relationship revisions/idempotent correction, forensic erasure, stored boards/snapshots, transitive/Boolean graph queries, delegation/scheduling/retries/WIP/executor policy, protocol-v1 expansion, auth/tenant/network/UI/provenance/metrics/blockchain behavior, and stable compatibility.
  - **T-907-E3 — WHEN** accepted-base scope and dependency direction are inspected, **THEN** Sprint 9 **SHALL** preserve byte-identical `cubikan-core`, stored envelope-v1 codec, `cubikan-local` protocol-v1 production shapes/operations, `cubikan-cli`, manifests/lockfile, and CI workflow, except for the declared local E2E unsupported-version fixture update.
- **Notes:** Do not refresh the historical derivative appendix under INT-0007 in this sprint; retain that explicit backlog item rather than mixing authority.

### T-908: Prove the public-backend relationship/projection vertical across reopen
- **Intent:** [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- **Touches:** `crates/cubikan-backend/tests/relationship_e2e.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Depends on:** T-907
- **Acceptance criterion:** The complete schema, definition, relationship, lifecycle, deletion, and multi-projection outcomes compose through the supported public Rust boundary.
- **Success criterion (EARS):**
  - **T-908-E1 — WHEN** a Rust caller uses only public `SqliteBackend` APIs against a real fresh file across repeated reopen, **THEN** it **SHALL** create units/definitions/edges, reject a policy-invalid edge, query relationships and multiple projections, mutate lifecycle membership, delete/recreate an edge, and retrieve the exact final durable units and relationships without mocks or private storage access.
  - **T-908-E2 — WHEN** a caller starts from a real exact-v1 file, **THEN** create/get definition, create/delete/list relationship, and project **SHALL** report migration-required before mutation; a pre-migration handle **SHALL** retain that cached capability after another connection migrates; and after explicit reopen every preexisting unit column/envelope **SHALL** remain exact while v2 definitions, edges, and projections persist correctly.
  - **T-908-E3 — WHEN** the sprint-wide regression surface runs, **THEN** all prior core/backend/local/stateless CLI tests and doctests **SHALL** remain green, while process-level relationship E2E **SHALL** remain explicitly deferred until a future intent selects a new local protocol version.
- **Notes:** T-908 is composition evidence only; it must not finish missing behavior from earlier tasks or add production seams.
