# Sprint 9 Real-file Integration Test Results

- **Intent:** [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- **Accepted base:** `e4c7aee275b9a95e08fd7f3235addbb41df5855a`
- **Build head:** `90ac02d75f7d756fad3a527487727ea4a27b9f27`
- **First critic-response head:** `0892886f83b40c3230dfe5d492d70dce1f0ecf5d`
- **Final tested head:** `153aa648847f6b3d48eef2264801807ea5316952`
- **Primary integration result:** 20 passed / 0 failed / 20 total
- **Exact command result:** 21 passed / 0 failed / 21 total (20 primary +
  1 auxiliary)
- **Boundary:** public `SqliteBackend` behavior over real, test-owned SQLite
  files; no backend, database, aggregate, transaction, or concurrency mock
- **Conclusion:** pass at the final tested head

The Build head remains the completed Sprint 9 implementation/evidence point.
The two later critic-response heads change only five backend test sources
relative to that head; this artifact attributes no additional production
behavior to them. The final `relationship_model` strengthening is owned by the
unit artifact under T-901-E4 and is not counted as integration evidence here.

## Fixture and observation boundary

Every primary test allocates a unique RAII temporary directory and a real
`cubikan.sqlite3` file. Product journeys initialize, reopen, and operate on the
file through `SqliteBackend`; no test substitutes `:memory:`, a mocked backend,
or a shared fixed path. Separate backend handles, SQLite connections, and OS
threads are used where independent readers or writers are part of the contract.

Raw `rusqlite` access is deliberately restricted to fixture authority that the
public product must not expose: constructing exact schema-v1 input, taking
complete typed row snapshots, inserting or retaining hostile/corrupt state,
holding a real writer lock, and installing a deterministic abort trigger. The
operation under observation always returns through production code. Rejected
operations compare complete unit, definition, and relationship snapshots; no
file-byte identity is claimed across valid SQLite commits.

The corruption oracle follows the locked selection boundary. Open detects exact
schema, integrity, CHECK, and foreign-key failures without globally replaying
all graph values. Definition, mutation, list, and projection operations decode
the rows they select, including the `limit + 1` lookahead. A hostile row excluded
by SQL filters is therefore not expected to fail an unrelated query, but the
same row must fail the whole operation once selected. No partial page, repair,
adoption, or cascade is accepted. Busy and abort paths are each invoked once and
leave the complete observed snapshot unchanged; internal attempt counts are not
instrumented by these integration tests.

## Exact local command and result

The following command was executed at the final tested head:

```sh
cargo +stable test -p cubikan-backend --test schema --test migration --test relationship_definitions --test relationship_mutations --test relationship_query --test projection
```

| Test binary | Observed result |
|-------------|-----------------|
| `migration` | 1 passed / 0 failed; finished in 0.04s |
| `projection` | 4 passed / 0 failed; finished in 0.08s |
| `relationship_definitions` | 4 passed / 0 failed; finished in 0.04s |
| `relationship_mutations` | 5 passed / 0 failed; finished in 5.09s |
| `relationship_query` | 4 passed / 0 failed; finished in 0.11s |
| `schema` | 3 passed / 0 failed; finished in 0.47s |

The command executed 21 tests. Twenty are the primary real-file integration
tests mapped below. The additional passing schema test,
`test_sqlite_dependency_is_bundled_and_adapter_only`, is an existing
dependency/scope regression and is not relabeled as a Sprint 9 primary.

## T-902 exact schema v2 and rejected migration/open inputs

| EARS / named test | Arrangement | Exact SHALL observation | Result |
|-------------------|-------------|-------------------------|--------|
| T-902-E1 / `test_new_and_existing_v2_databases_have_exact_schema_and_pragmas` | Run the production open path against both an absent path and a pre-created empty file. Inspect each initialized real file, insert a sentinel unit row, close, and reopen through `SqliteBackend` before comparing the complete logical snapshot. | Fresh and empty inputs initialize `user_version=2`; reopen preserves the sentinel and never reinitializes it. The file has exactly the locked 12 owned objects: the unchanged v1 table, four indexes and autoindex; both `STRICT` relationship tables and their autoindexes; and the two exact source/target indexes. Literal DDL, column order/type/nullability/PK positions, `BINARY` index keys, `(wr=0, strict=1)`, composite definition FK sequence, all `RESTRICT`/`MATCH NONE` FKs, `integrity_check=ok`, an empty `foreign_key_check`, rollback-journal `DELETE`, and `locking_mode=NORMAL` all match. Open remains structural rather than a global envelope replay. | pass |
| T-902-E4 / `test_migration_rejects_unowned_corrupt_and_wrong_version_sources` | Call only `SqliteBackend::migrate_v1_to_v2` on a nonexistent path, an empty v0 file, unowned v0 data containing `foreign_data('keep')`, v1 with an extra object, exact v2, v3, readable non-SQLite bytes, and three independent exact-v1 matrices. Each matrix contains canonical unit IDs 1, 2, and 3, with the corrupt envelope respectively first, middle, or last and both other units created and replayed through the public v1 backend. Snapshot schema version, complete object inventory, foreign values, and every column of all three ordered unit rows. | The missing path returns wrapped storage failure and is not created; empty v0 returns `SourceVersionNotOne { found: 0 }`; unowned v0 returns wrapped `UnownedDatabase`; every corrupt-first/middle/last v1 matrix returns wrapped `CorruptEnvelope`; malformed v1 and non-SQLite input return wrapped `CorruptSchema`; exact v2 returns `SourceVersionNotOne { found: 2 }`; and v3 returns wrapped `UnsupportedSchemaVersion { found: 3 }`. Every existing logical snapshot—including `foreign_data('keep')` and all three rows in each corruption position—and the non-SQLite source bytes remains exact, with no v2 object, adoption, repair, or partial migration. | pass |
| T-902-E6 / `test_open_rejects_corrupt_v2_without_repair` | Table production opens over invalid special paths; unowned v0 and unsupported v3 sentinels; v2 missing, wrong, or extra owned objects; CHECK-invalid definition ID/version rows; definition/endpoint FK-invalid edges; invalid `sqlite_schema` SQL; an injected reserved `sqlite_*` object; aliased physical rootpages; and non-SQLite bytes. Preserve sentinel rows and use WAL on rejection fixtures so mutation is observable. | Empty/`:memory:` paths retain opaque source-preserving storage classification. Unowned and unsupported inputs remain typed. Every malformed v2/physical fixture fails closed as `CorruptSchema`; selected schema text, object inventory, rootpage alias, `user_version`, WAL mode, logical rows, and readable non-SQLite bytes remain unchanged. Open neither repairs nor adopts state, and CHECK/FK-invalid relationship rows are rejected at the structural boundary rather than deferred to a semantic operation. | pass |

The primary T-902 integration oracle directly observes durable schema metadata,
`DELETE`, and raw-connection `NORMAL`. Every successful open also executes the
production configuration verifier before returning. The auxiliary in-module
real-file test `test_new_empty_database_initializes_exact_schema_v2_and_pragmas`
retains the returned backend connection and directly asserts `DELETE`, `NORMAL`,
`synchronous=EXTRA`, `foreign_keys=ON`, `trusted_schema=OFF`,
`read_uncommitted=OFF`, and the 5,000-ms busy setting, then reopens and verifies
the same contract. That supporting test is not counted again among these 20
primaries.

## T-903 immutable relationship definitions

| EARS / named test | Arrangement | Exact SHALL observation | Result |
|-------------------|-------------|-------------------------|--------|
| T-903-E1 / `test_definition_create_commits_exact_view_before_return` | On one v2 file, create four definitions spanning every self/cycle `Allow`/`Reject` pair, absent and role-specific species constraints, versions `1`, `2`, `i64::MAX + 1`, and `u64::MAX`, and directed-only direction. Open a fresh backend immediately after every return. | Each call returns the exact typed view only after a fresh connection can retrieve it. Four rows persist with `directed=1`, exact optional species/policies, and full-width big-endian versions, including `u64::MAX`; no inferred version or mutable value is introduced. | pass |
| T-903-E2 / `test_definition_versions_round_trip_independently_across_reopen` | Insert versions `u64::MAX`, `1`, `i64::MAX + 1`, and `42` out of order under the single ID `depends-on`, giving each version different constraints and policies; close and reopen. | Exact get returns each independently created view. Raw ordered storage contains big-endian versions `1`, `42`, `i64::MAX + 1`, and `u64::MAX`; creation order does not create latest, contiguous, supersession, or signed-integer semantics. | pass |
| T-903-E3 / `test_definition_duplicate_and_missing_are_typed_and_nonmutating` | Persist one unit and definition `implements@7`, snapshot all unit and definition columns, then create the same identity with different content and get missing `implements@99`. | A valid exact-key collision returns `DefinitionAlreadyExists { definition: implements@7 }`; the absent version returns `DefinitionNotFound { definition: implements@99 }`. The accepted view remains exact and both snapshots are unchanged. | pass |
| T-903-E4 / `test_selected_definition_value_corruption_fails_closed_without_repair` | For independent real files, retain valid key `tracks@u64::MAX` while raw fixture access corrupts, one at a time, `directed`, source species, target species, self policy, or cycle policy. Invoke both create-on-collision and exact get, snapshotting after each call. | Every selected malformed value returns `CorruptDefinition` for the retained key. Corruption outranks duplicate classification; neither operation reports success, repairs, deletes, or mutates the single hostile row. | pass |

## T-904 atomic relationship mutation policy

| EARS / named test | Arrangement | Exact SHALL observation | Result |
|-------------------|-------------|-------------------------|--------|
| T-904-E1 / `test_edge_create_commits_without_mutating_endpoints` | Create three real units, transition source `A` to active `done` revision 1, and transition then complete target `B` at `done` revision 2. Create a species-constrained reject-self/reject-cycle definition and edge `A -> B`; snapshot every unit and definition column before creation, then close/reopen, delete the edge, and recreate it. | Relationship eligibility is independent of the endpoints' differing phase, status, and lifecycle revisions: create returns the exact complete identity only after one edge row is durable. The active-revision-1 source row, completed-revision-2 target row, third unit, and definition remain exact across create, reopen, delete, and recreate. None of those relationship operations changes endpoint envelopes, projections, revisions, histories, phases, or statuses. | pass |
| T-904-E2 / `test_edge_policy_rejections_are_atomic` | Arrange conflicting failures across the locked adjacent precedence boundaries: missing definition plus two missing endpoints; corrupt definition plus two corrupt endpoints; two missing or two corrupt endpoints; target replay failure plus species mismatches; both species mismatches; target-species mismatch plus rejected self; rejected self on an existing self edge; duplicate plus corrupt reachability; and corrupt source on a proposal that would otherwise close a cycle. Also submit an otherwise-valid cycle-closing proposal. Snapshot units, definitions, and edges around every command. | After writer acquisition, observed classification follows definition, source replay, target replay, source species, target species, self, duplicate, then non-self reachability. The conflicting fixtures return the precise earlier `DefinitionNotFound`, `CorruptDefinition`, role-specific `EndpointNotFound`/`EndpointCorrupt`/`EndpointSpeciesMismatch`, `SelfEdgeRejected`, or `DuplicateRelationship`; selected malformed reachability returns `CorruptRelationship`. The clean closing proposal returns `CycleRejected`. Every rejection preserves the complete pre-call snapshot. | pass |
| T-904-E3 / `test_self_and_cycle_policy_matrix_is_version_scoped` | Exercise all four independent self/cycle policy pairs. For each, propose one self-edge and then opposite non-self edges. Also create opposite edges under versions `1` and `u64::MAX` of one definition ID. | Self policy alone accepts or rejects length-one edges even when cycle policy differs. Cycle policy alone accepts or rejects the reverse non-self edge. Reachability is scoped to the complete definition key, so opposite edges split across versions both commit. | pass |
| T-904-E4 / `test_concurrent_cycle_creators_commit_once_then_reject_cycle` | Open two independent backend handles on one file, barrier-start two OS threads proposing `A -> B` and `B -> A` under one reject-cycle definition, and join both results. | SQLite writer serialization plus in-transaction reachability commits exactly one proposal and returns exactly one typed `CycleRejected`; `StorageBusy` is not accepted. Exactly one complete edge remains, in either winning direction, and no forbidden cycle or partial row is durable. | pass |
| T-904-E5 / `test_edge_delete_is_exact_non_cascading_and_atomic_on_failure` | Start with three edges, delete one complete identity, reject a different absent identity, and explicitly recreate corrected/original edges. In separate files table missing/corrupt definitions, missing/corrupt endpoints, both species mismatches, a real independent `BEGIN IMMEDIATE` lock holder, and a trigger-raised SQLite delete abort. Compare complete durable snapshots. | Exact delete removes only the named row and returns its full view; retained edges, definitions, and endpoints do not cascade or revise. Missing identity returns `RelationshipNotFound`, and each selected definition/endpoint fixture returns its precise typed role. One locked call returns wrapped `StorageBusy`; the trigger abort returns wrapped storage failure. Every failure preserves the complete pre-call snapshot, and correction remains two independently committed operations. | pass |

## T-905 bounded direct relationship queries

| EARS / named test | Arrangement | Exact SHALL observation | Result |
|-------------------|-------------|-------------------------|--------|
| T-905-E1 / `test_relationship_query_ands_exact_filters_and_orders_direct_edges` | Create five units, two versions of `depends-on`, and insert five version-`u64::MAX` edges deliberately out of order, including paths longer than one hop; add a near edge under version `1`. Query with no filter, source only, target only, both, and an absent near-miss source. | Results contain only the named exact version's direct edges in canonical `(source,target)` order. Source and target filters are ANDed, no transitive edge is synthesized, the other version is excluded, and an absent optional filter yields an empty successful page. | pass |
| T-905-E2 / `test_relationship_query_enforces_bounds_complete_cursor_and_live_pages` | Reject model limits `0`/`101`; create one source and 101 ordered targets under definition version `u64::MAX`; query limits `100` and `1`; continue after the returned full-identity cursor and after a same-definition cursor naming no stored edge; construct a cross-definition cursor; corrupt the `limit + 1` endpoint; and mutate membership between two pages. In a separate source-boundary file, insert two edges for source 10 at targets 900/901 and two edges for later source 20 at lower targets 2/3, then paginate at limit 2. | Limits `1` and `100` succeed. Page one returns 100 items and a cursor containing definition/version/source/last target; page two returns only item 101 with no repeat or terminal cursor. A nonexistent same-definition cursor is valid ordering state; a cross-definition cursor fails before storage. The source-boundary cursor `(source 10,target 901)` continues to `(source 20,target 2)` and `(source 20,target 3)` despite their lower target IDs, with no repeats and a null terminal cursor, proving composite `(source,target)` continuation. Corrupt lookahead fails the whole page. After cursor target `20`, deleting `30` and inserting `25`/`15` makes the live next page exactly `25,40`: later committed membership is observed and before-cursor insertion is excluded. | pass |
| T-905-E3 / `test_relationship_query_missing_definition_and_absent_filter_are_distinct` | Create only `existing@1`, then query `missing@1`; separately query the existing definition with nonexistent source, target, and combined optional filters. | The required missing definition returns typed `DefinitionNotFound`. Each absent optional filter returns its complete retained query, zero items, and no cursor rather than an endpoint error. | pass |
| T-905-E4 / `test_relationship_query_rejects_selected_corruption_without_partial_results` | Independently retain a valid key while corrupting a selected definition value; place malformed endpoint text in lookahead; corrupt selected source and target envelopes in separate files; change selected source- and target-species constraints in separate files; and place an edge whose target envelope is corrupt behind a source filter before selecting it in a later query. Snapshot every hostile file. | Selected definition, edge, lookahead, and endpoint failures return exact `CorruptDefinition`, `CorruptRelationship`, role-specific source or target `EndpointCorrupt`, or exact source or target `EndpointSpeciesMismatch` and expose no partial page/cursor. The filtered query returns only its valid edge without scanning the hostile row; selecting that row later fails. No operation repairs or mutates the snapshot. | pass |

## T-906 ephemeral projection query v1

| EARS / named test | Arrangement | Exact SHALL observation | Result |
|-------------------|-------------|-------------------------|--------|
| T-906-E1 / `test_projection_v1_ands_lifecycle_filters_with_direct_predicate` | Create an anchor plus matching, wrong-species, wrong-workflow, wrong-phase, completed, transitive, and inbound units. Create direct outgoing, one second-hop, and incoming edges. Compare a predicate-free projection with the existing lifecycle list, then apply all workflow/species/phase/status filters to outgoing and an unfiltered incoming predicate. | Predicate-free projection exactly equals lifecycle list items/cursor. The outgoing result is only the direct matching target after all lifecycle predicates are ANDed; the second-hop unit is absent. Incoming returns the two direct sources in canonical unit-ID order and does not traverse onward. | pass |
| T-906-E2 / `test_unit_appears_in_multiple_live_projections_without_copied_state` | Relate one target from two anchors under two definitions, issue both outgoing queries, inspect schema and complete durable snapshots, delete only one edge, and then transition the target from `queued` to `done`. | Both projections return the same canonical summary while retaining distinct queries; reads create no mutation and schema contains no board/projection object. Removing one edge changes only its later view and leaves endpoint revision `0`; lifecycle transition advances the canonical unit to revision `1`, removes it from the queued view, adds it to the done view, and leaves the other relationship row intact. | pass |
| T-906-E3 / `test_projection_v1_reports_query_and_uses_exclusive_live_pages` | Reject limits `0`/`101`; project 101 related units at limits `100` and `1`; repeat an unchanged query; continue from the returned unit-ID cursor; then delete/insert members between pages and continue from both a returned and an absent cursor. | Unchanged committed state produces equal pages retaining the complete query with `version() == ProjectionQueryV1::VERSION == 1`. Items are canonical-ID ordered; page one is IDs `1..100`, its exclusive cursor is `100`, and the terminal page is only `101`. Live continuation after `20` reflects deletion of `30` and insertion of `25` as `25,40`, excludes before-cursor `15`, and accepts absent cursor `22` as ordering state. | pass |
| T-906-E4 / `test_projection_v1_missing_and_corrupt_inputs_fail_whole_page` | Exercise predicate-free projection on exact v1; missing definition; missing outgoing/incoming anchors; selected non-key definition corruption; malformed edge text in the one-row lookahead; SQL-filtered versus selected corrupt outgoing candidates; predicate-free lifecycle corruption; corrupt outgoing source anchor/target candidate; corrupt incoming target anchor/source candidate; and incoming target-anchor/source-candidate species mismatches. Preserve snapshots around hostile operations. | Exact v1 returns `MigrationRequired { V1 -> V2 }` before query work. Missing definition precedes anchor replay. Outgoing failures retain `Source` anchor and `Target` candidate roles; incoming failures retain `Target` anchor and `Source` candidate roles. Selected definition/edge state returns `CorruptDefinition`/`CorruptRelationship`; selected or lookahead endpoint corruption returns the exact role-specific `EndpointCorrupt`; both incoming species constraints return role-specific `EndpointSpeciesMismatch`; predicate-free replay failure remains wrapped `Backend(CorruptEnvelope)`. SQL-excluded corruption is not scanned, but selecting hostile state fails the whole page without partial summaries, cursor, repair, or mutation. | pass |

## Transaction, pagination, and corruption conclusions

- Successful definition and edge mutations are observable from fresh
  connections before their public calls return. Rejections compare all three
  durable collections, so endpoint lifecycle state and accepted graph state are
  not inferred from return values alone.
- The independent-writer cycle test permits no busy substitute: one transaction
  commits and the serialized second transaction sees the committed path and
  rejects the cycle. Deletion separately proves real lock contention and a
  SQLite abort leave prior state exact.
- Relationship pagination uses the complete edge identity and projection
  pagination uses the last unit ID. Both validate lookahead before emitting a
  cursor, both are exclusive, and both intentionally provide live committed
  pages rather than snapshots across requests.
- Structural corruption is rejected on open; semantically hostile definition,
  edge, and endpoint values are rejected only when selected by an operation.
  This is a fail-closed operation boundary, not a whole-file semantic scanner.

## External hosted-workflow corroboration

This section records the existing GitHub Actions boundary only. It is not a
21st primary integration test, a mock substitute for the real-file tests, a
relationship process-level E2E, or merge authorization.

GitHub reports workflow ID `330204114`, name `Rust CI`, path
`.github/workflows/ci.yml`, and state `active`. The registered `dev` workflow
blob is `96420136d282ef93bb60b0607dffac1d28427a8d`, identical to the local blob at
the final tested head.

| Field | Observed value |
|-------|----------------|
| Run | [31362124061 — Rust CI #27](https://github.com/crussella0129/CubiKan/actions/runs/31362124061) |
| Event / branch / run number / attempt | `push` / `dev` / `27` / `1` |
| Head SHA | `153aa648847f6b3d48eef2264801807ea5316952` |
| Run status / conclusion | `completed` / `success` |
| Run created / started | `2026-08-10T06:28:38Z` / `2026-08-10T06:28:38Z` |
| Run updated | `2026-08-10T06:29:49Z` |
| Sole job | [93372894839 — Rust quality gate](https://github.com/crussella0129/CubiKan/actions/runs/31362124061/job/93372894839) |
| Job started / completed | `2026-08-10T06:28:42Z` / `2026-08-10T06:29:49Z` |
| Job status / conclusion | `completed` / `success` |

Every hosted step completed successfully:

| Step | Started | Completed | Conclusion |
|------|---------|-----------|------------|
| Set up job | `2026-08-10T06:28:42Z` | `2026-08-10T06:28:43Z` | success |
| Checkout | `2026-08-10T06:28:43Z` | `2026-08-10T06:28:44Z` | success |
| Install stable toolchain | `2026-08-10T06:28:44Z` | `2026-08-10T06:28:44Z` | success |
| Formatting | `2026-08-10T06:28:44Z` | `2026-08-10T06:28:45Z` | success |
| Clippy | `2026-08-10T06:28:45Z` | `2026-08-10T06:28:59Z` | success |
| Warnings-denied workspace check | `2026-08-10T06:28:59Z` | `2026-08-10T06:29:11Z` | success |
| All-target workspace tests | `2026-08-10T06:29:11Z` | `2026-08-10T06:29:46Z` | success |
| Workspace doctests | `2026-08-10T06:29:46Z` | `2026-08-10T06:29:47Z` | success |
| Post-checkout cleanup | `2026-08-10T06:29:47Z` | `2026-08-10T06:29:48Z` | success |
| Complete job | `2026-08-10T06:29:48Z` | `2026-08-10T06:29:48Z` | success |

The all-target command retained the 191-test workspace total and the doctest
command retained the one passing workspace doctest. The five hosted quality
commands were exactly:

```sh
cargo +stable fmt --all -- --check
cargo +stable clippy --workspace --all-targets --all-features -- -D warnings
RUSTFLAGS="-D warnings" cargo +stable check --workspace --all-targets
cargo +stable test --workspace --all-targets
cargo +stable test --doc --workspace
```

The job log records runner name `GitHub Actions 1000004629`, runner version
`2.336.0`, Ubuntu `24.04.4`, image
`ubuntu-24.04` version `20260720.247.2`, provisioner
`20260707.563`, and stable `rustc 1.97.1 (8bab26f4f 2026-07-14)` with current
`rustfmt` and `clippy`. These are run provenance, not fixed support, MSRV, or
cross-platform promises.

GitHub downloaded immutable
`actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1`, with
`persist-credentials: false`. Checkout fetched
`+153aa648847f6b3d48eef2264801807ea5316952:refs/remotes/origin/dev`;
both `git rev-parse refs/remotes/origin/dev` and hosted
`git log -1 --format=%H` returned
`153aa648847f6b3d48eef2264801807ea5316952`. The workflow explicitly grants
only `contents: read`; the hosted permissions display additionally reports
implicit `Metadata: read` for the masked built-in `GITHUB_TOKEN`. That implicit
metadata permission is not a second explicit workflow grant, and checkout
removed its temporary credential configuration because persistence was
disabled. No custom secret or write permission participates in this evidence.

## Integration nonclaims

These results do not prove snapshot pagination, transitive graph execution,
definition deletion/listing/latest-version policy, retained relationship
history, idempotent correction, cascade behavior, forensic erasure, automatic
migration, backup/downgrade/progress/resume, crash-kill or device-loss recovery,
network-filesystem safety, authorization/tenancy, protocol-v2 relationships,
UI/board layout, scheduling, retries, WIP policy, provenance, metrics,
blockchain behavior, branch protection, or automatic merge authority.
