# Sprint 8 Integration Test Results

- **Primary intent:** [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md)
- **Preserved dependency:** [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md)
- **Accepted base:** `91d3260d50af8f6c5ec3a852fad50e4e32df3b59`
- **Build task/ledger head:** `581281cb8e4ab38c0f47f4e12f085ea825b92096`
- **Tested critic-response candidate:** `065b71fa1b63ba6abce6effb23c9d20674171835`
- **Backend result:** 30 passed, 0 failed, 0 ignored, 0 measured, 0 filtered out
- **Durable local result:** 19 passed, 0 failed, 0 ignored, 0 measured, 0 filtered out
- **Conclusion:** pass; every locked T-803 through T-808 integration clause and
  every planned cross-component seam passed against production code

All backend storage tests use a real on-disk SQLite file in a test-owned
temporary directory. Reopen assertions use fresh `SqliteBackend` or raw SQLite
connections. Competing-writer tests use independent connections and real
threads. Raw SQLite appears only to inspect the owned schema or prepare a
specific corrupt, unsupported, locked, ignored-update, or aborted-update
fixture; it never substitutes for the production backend operation under test.

The runner tests use the real backend plus injectable readers/writers to make
otherwise nondeterministic I/O failure stages observable. Those readers and
writers are boundary fixtures, not repository or domain mocks. Actual process
creation is reserved for the E2E artifact.

## T-803 owned schema and open behavior

| EARS | Named executed evidence | Arrangement | Exact SHALL observation | Result |
|------|-------------------------|-------------|-------------------------|--------|
| T-803-E1 | external and backend-library tests both named `test_new_empty_database_initializes_exact_schema_v1_and_pragmas` | Open a nonexistent path and a precreated zero-byte file. Inspect `user_version`, `sqlite_schema`, table metadata, constraints, index columns/SQL, integrity, and connection PRAGMAs after the production open path performs its immediate ownership recheck. | One transaction creates only the locked `STRICT intent_units` table, its autoindex and four named indexes, then sets `user_version=1`. Returned connections verify `journal_mode=DELETE`, `synchronous=EXTRA`, `foreign_keys=ON`, `trusted_schema=OFF`, `read_uncommitted=OFF`, `locking_mode=NORMAL`, and `busy_timeout=5000`. | pass |
| T-803-E2 | `test_exact_schema_v1_reopens_without_migration` | Initialize exact v1, insert a sentinel owned row, snapshot version/schema/journal/content, close, and reopen through `SqliteBackend`. | Reopen accepts and preserves the exact owned schema and sentinel without reinitialization, migration, or content change. | pass |
| T-803-E3 | `test_open_rejects_unversioned_unknown_incomplete_extra_and_corrupt_databases` | Independently exercise empty/`:memory:` paths, nonempty version 0, version 2, missing/wrong/extra v1 objects, reserved-name injection, aliased physical rootpages, and non-SQLite bytes; snapshot logical version/schema/journal/content around rejection. | Empty/special paths are typed storage failure; nonempty v0 is unowned; version 2 is unsupported; malformed/physically corrupt/non-SQLite v1 is corrupt schema. Inspection precedes persistent PRAGMA assignment, and every rejected fixture retains its logical version, schema, journal mode, and sentinel data without adoption, repair, deletion, or migration. | pass |
| T-803-E4 | `test_sqlite_dependency_is_bundled_and_adapter_only`; accepted-base scope check | Inspect offline Cargo metadata and exact core/CLI manifests at the candidate. | `rusqlite = 0.40.2` is exact-pinned with `default-features = false` and `bundled`, and only `cubikan-backend` depends on it. The core and stateless CLI manifests/dependency graphs are byte-identical to the accepted base. | pass |

SQLite open can touch file headers; the rejection oracle therefore compares
logical state except where the deliberately non-SQLite byte fixture is safely
compared byte-for-byte. This is the locked recovery claim, not a general
filesystem non-mutation promise.

## T-804 durable create and replay-validated get

| EARS | Named executed evidence | Arrangement | Exact SHALL observation | Result |
|------|-------------------------|-------------|-------------------------|--------|
| T-804-E1 | `test_create_commits_complete_revision_zero_unit` | Create units with supplied ordinary ID, supplied canonical nil ID, and omitted ID; after each return open a fresh backend and inspect stored rows. | Each immediate transaction commits one complete version-1 envelope and matching projections before return. Supplied IDs are exact; omission alone generates a non-nil UUID v4. Every unit is active at its workflow initial phase, revision 0, empty history, and an exact eight-zero-byte revision projection. | pass |
| T-804-E2 | `test_create_get_round_trip_multiple_units_across_reopen`; `test_backend_codec_schema_crud_query_and_mutation_compose` | Create multiple IDs with distinct Unicode/whitespace-preserving species and workflow snapshots, drop connections, reopen, and get each independently; the composition journey later reopens between lifecycle operations. | Stable ID retrieval reconstructs and replay-validates each exact immutable workflow, identity, species, phase, status, revision, and ordered history without cross-unit leakage. | pass |
| T-804-E3 | `test_duplicate_create_and_missing_get_are_typed_and_nonmutating` | Seed an accepted row, snapshot all rows, create the same ID with a different payload, and get an absent ID. | Duplicate create returns `DuplicateIntentUnit`; absent get returns `IntentUnitNotFound`. Row count and every accepted logical value remain unchanged. | pass |
| T-804-E4 | `test_get_rejects_envelope_and_each_projection_mismatch_without_repair`; `test_corruption_never_reaches_mutation_or_protocol_success` | Mutate envelope version/content and each ID/workflow/species/phase/status/revision projection independently using raw fixture access, then call production `get`. | Unsupported envelope version, corrupt envelope, and projection mismatch remain distinct typed failures. No tampered row is repaired, deleted, or returned; the cross-component fixture remains byte-for-byte unchanged through backend and executor rejection. | pass |

## T-805 bounded exact-filter live keyset pagination

| EARS | Named executed evidence | Arrangement | Exact SHALL observation | Result |
|------|-------------------------|-------------|-------------------------|--------|
| T-805-E1 | `test_list_filters_exact_fields_and_orders_ids_lexically`; composition test | Seed workflow IDs with different topology, species/phase/status values with case, Unicode normalization, whitespace, and SQL-looking near-misses, and IDs out of insertion order. Query every filter alone and together. | Bound parameters and `BINARY` equality return only exact replay-validated matches. A shared workflow ID matches both topologies without asserting topology equality; no derivative data is interpreted. Results use canonical-ID lexical order. | pass |
| T-805-E2 | `test_list_enforces_bounds_and_exclusive_keyset_cursor` | Seed 101 IDs in reverse order; page with limits 100 and 1; use canonical nil, existing, and absent cursors. | The backend fetches `limit + 1`, returns no more than the requested 1–100 items in ascending canonical-ID lexical order, excludes IDs at or before the cursor, emits the last returned ID only with another match, and returns no cursor at terminal/empty pages. A cursor need not name a row. | pass |
| T-805-E3 | `test_list_pages_are_live_committed_views_with_mutable_membership` | Read page one, then insert matches below/above its cursor and move existing rows into/out of the phase filter before requesting page two. | Page two reflects its own committed after-cursor membership: below-cursor additions stay excluded, above-cursor additions/inclusions appear, and removals disappear. No cross-request snapshot is claimed. | pass |
| T-805-E4 | `test_list_fails_whole_page_for_any_selected_invalid_row`; corruption cross-component test | Corrupt a returned row, a projection, and the `limit + 1` lookahead candidate; separately corrupt a row excluded by an exact filter. | Any selected or lookahead corruption rejects the whole page with no partial summaries. A filtered-out corrupt row is not inspected and does not create a false claim of whole-database corruption detection. | pass |

The cursor is ordering state only. These tests make no offset, authorization,
chronological, snapshot, or stable-membership claim.

## T-806 guarded mutation, concurrency, busy, and abort behavior

| EARS | Named executed evidence | Arrangement | Exact SHALL observation | Result |
|------|-------------------------|-------------|-------------------------|--------|
| T-806-E1 | `test_guarded_transition_and_completion_commit_one_successor_before_return`; composition test | Transition at revisions 0 and 1, complete at 2, and after each method return open a fresh backend; inspect full view, row projections, prior-history prefix, and appended record. | Each `BEGIN IMMEDIATE` path loads/replays, calls the matching guarded core command, writes the full successor under ID/revision/envelope-version qualification, requires one row, commits, and returns exact revisions 1/2/3. Fresh connections see exactly one additional record and the complete successor before the caller receives success. | pass |
| T-806-E2 | `test_stale_mutations_win_precedence_and_preserve_durable_state`; `test_revision_conflict_propagates_core_to_local_protocol` | Combine stale observations with unknown target, undeclared edge, ineligible completion, and terminal transition/completion after writer acquisition; snapshot every row. | Exact expected/actual conflict wins before each domain error, including expected 0/actual 1 and terminal expected 2/actual 3. Rows remain unchanged, and the 0/1 values reach local JSON as strings without false success. | pass |
| T-806-E3 | `test_current_revision_missing_corrupt_and_domain_rejections_preserve_state`; corruption cross-component test | Submit missing IDs, a corrupt selected row, unknown target, undeclared edge, ineligible completion, and current-revision terminal transition/completion. | The backend returns the exact missing, corruption, `UnknownTarget`, `NotAllowed`, `PhaseNotEligible`, or `AlreadyCompleted` variant before update. No row is inserted, repaired, or mutated. | pass |
| T-806-E4 | `test_two_isolated_writers_commit_exactly_once`; `test_revision_qualified_zero_row_update_rolls_back`; conflict propagation | Let two independent connections/threads observe revision 0 and release them through a barrier; separately install a trigger that makes the qualified update affect zero rows. | Exactly one writer commits revision 1 and one later writer conflicts with actual 1; one durable record exists. A zero-row update returns `ConcurrentStorageChange` and rolls back rather than reporting success. | pass |
| T-806-E5 | `test_busy_writer_times_out_once_without_retry_or_mutation`; `test_sqlite_update_abort_rolls_back_complete_row` | Hold an independent `BEGIN IMMEDIATE` beyond the configured wait; separately use a fixture trigger that aborts after the production load. Compare elapsed time, typed error, and complete stored rows. | Contention makes one bounded attempt, returns `StorageBusy` after at least 4.5 seconds and before 9 seconds, performs no retry, and leaves revision 0. SQLite abort returns typed storage failure and rolls back the complete envelope/projections/history. | pass |

Busy occurs while acquiring `BEGIN IMMEDIATE`, so it can precede stale
evaluation. Stale-first semantics apply after the writer is acquired. No test
or implementation claims retry, lease, merge, idempotency, or cross-unit
transaction behavior.

## T-808 bounded runner and process shell

| EARS | Named executed evidence | Arrangement | Exact SHALL observation | Result |
|------|-------------------------|-------------|-------------------------|--------|
| T-808-E1 | `test_process_shell_requires_exactly_one_explicit_database_path` | Supply missing program/flag/path, empty and `:memory:` paths, wrong flag, positional path, trailing arg, and repeated flag while stdin is a panic-on-read sentinel; also exercise an opaque non-UTF-8 path on Unix. | Only exact `cubikan-local --database PATH` proceeds. Invalid forms return usage exit 2 on stderr, emit no JSON, never read stdin, and create neither candidate nor default database. Usage diagnostic failure remains best effort. | pass |
| T-808-E2 | `test_runner_dispatches_one_command_and_flushes_one_response` | Against real SQLite, dispatch all five success operations plus request, command, and storage failure classes through a recording writer. | Each within-bound validated request executes exactly one operation, writes one compact body and one newline, calls one final flush, and yields its modeled class only after `[Body, Newline, Flush]` succeeds. | pass |
| T-808-E3 | `test_runner_enforces_one_mib_before_json_or_database` | Build an exactly 1,048,576-byte request whose final byte closes valid JSON, then append bytes and use a reader that would fail after the one-byte lookahead. | The exact-bound request succeeds. The over-bound request consumes/retains exactly 1,048,577 bytes, returns `request_too_large` before JSON/storage classification, and leaves the sentinel database absent. | pass |
| T-808-E4 | `test_runner_preserves_first_io_error_precedence`; exit-mapping test | Inject read failure before the boundary and distinct body/newline/flush failures; record writer stages and downcast error sources. | The first failing stage stops every later output stage, preserves exact I/O kind/message/source, yields no modeled status, and maps through the shell to operational exit 1 with best-effort stderr. | pass |
| T-808-E5 | `test_committed_mutation_survives_response_delivery_failures` | For independent databases, commit a guarded transition and then fail body, newline, or flush; reopen each database. | Every runner error states that the committed outcome is unknown, while a fresh backend sees revision 1, target phase, and one record. No rollback, idempotency, acknowledged delivery, or safe retry is inferred. | pass |
| T-808-E6 | `test_local_process_exit_and_stderr_mapping` | Exercise success, malformed JSON, missing unit, unavailable storage path, and read failure through the injectable process shell; also fail the operational diagnostic sink. | Success maps to 0, request to 2, command/domain to 3, storage to 4, and operational I/O to 1. Modeled responses leave stderr empty; the operational diagnostic is best effort and cannot change exit 1. | pass |

## Exact cross-component verification

The three Test-plan cross-component checks all exist at the tested candidate and
passed. Test response commit `2e5e2b9` adds them without production changes.
After critique identified a direct mapper-coverage gap, `065b71f` adds only the
`#[cfg(test)]` oracle below to `crates/cubikan-local/src/execution.rs`; it does
not change the production mapper.

### `execution::tests::test_backend_errors_map_exhaustively_to_protocol_codes`

- **Coverage:** T-807-E4 critic response.
- **Arrangement:** Construct all 17 expanded mapping cases: every top-level
  `BackendError`, every `TransitionError` and `CompletionError` inner variant,
  a genuine expected-9/actual-0 stale conflict produced by the real backend,
  and opaque storage payloads obtained from a real failed open and cloned for
  busy/storage variants. A separate exhaustive match assigns unique ordinals
  0–16 and expected code/class/conflict fields to every case.
- **SHALL observation:** Every ordinal is visited exactly once. The production
  mapper preserves the original display message and emits the exact code/class;
  `field` is absent for every backend error, and decimal expected `"9"`/actual
  `"0"` appear only for `revision_conflict`. The exhaustive helper and the
  production match both require compiler-visible handling of every expanded
  variant.
- **Result:** pass.

### `test_backend_codec_schema_crud_query_and_mutation_compose`

- **Coverage:** T-802-E1, T-803-E1, T-804-E2, T-805-E1, T-806-E1.
- **Arrangement:** Through public `SqliteBackend` only, create a fixed-ID unit
  with a custom `draft -> shipped` workflow, close/reopen, get and exact-filter
  list it, guarded-transition at revision 0, reopen, guarded-complete at revision
  1, reopen, get/list the terminal unit, and compare it to an independently
  constructed core aggregate.
- **SHALL observation:** Envelope codec, schema initialization, CRUD, query, and
  guarded mutation compose across four real connection lifetimes. The final
  view is completed at revision 2 with exact workflow and transition/completion
  history, and the completed exact-filter page returns its summary only.
- **Result:** pass.

### `test_corruption_never_reaches_mutation_or_protocol_success`

- **Coverage:** T-802-E2, T-804-E4, T-805-E4, T-806-E3, T-807-E4/E5.
- **Arrangement:** Make an equal-length byte replacement that gives the stored
  envelope an undeclared initial phase while leaving owned schema/projections
  intact. Exercise backend get, list, and transition, then the local executor.
- **SHALL observation:** Every backend operation returns `CorruptEnvelope`; the
  protocol returns storage-class `corrupt_envelope` with no result or optional
  validation/conflict fields. The complete database bytes remain identical to
  the deliberately corrupt snapshot throughout.
- **Result:** pass.

### `test_revision_conflict_propagates_core_to_local_protocol`

- **Coverage:** T-806-E2/E4 and T-807-E2/E4.
- **Arrangement:** Create and transition a real unit from revision 0 to 1, then
  send stale revision `"0"` with an undeclared target through the local
  executor; compare fresh backend state before/after.
- **SHALL observation:** The core's stale-before-domain decision reaches the
  protocol as command-class `revision_conflict` with string expected `"0"` and
  actual `"1"`, no field/result, and unchanged revision-1 aggregate/history.
- **Result:** pass.

## INT-0010 acceptance-criterion traceability

All seven intent acceptance criteria have an executed end-to-end evidence chain:

| INT-0010 acceptance criterion | Named executed arrangements | Exact observation | Result |
|--------------------------------|-----------------------------|-------------------|--------|
| Multiple units survive restart with immutable workflows and complete history | Envelope round trips; create/get reopen; composition journey; `test_cubikan_local_persists_paginates_and_completes_across_processes` | Distinct stable IDs survive fresh connections/processes with exact owned workflows, species, phase/status, decimal revision, and full ordered history through completion. | pass |
| Bounded exact-filter pagination with documented order/cursor/live consistency | Limit/cursor model test; four T-805 query tests; local process lifecycle; T-810 docs check | Limits 1–100, bound exact filters, workflow-ID-only meaning, lexical canonical-ID order, exclusive last-returned cursor, lookahead validation, live pages, and mutable membership are implemented, process-observed, and documented. | pass |
| Adapter-owned versioned five-operation boundary | T-801 public model; T-807 strict operations/results/taxonomy; T-808 dispatch/exits | Backend Rust commands and local protocol v1 expose create/get/list/transition/complete without stored/core DTO leakage; all request/domain/storage classes and exits are exact. | pass |
| Every load validates/replays and corrupt/unsupported state fails closed | Four T-802 codec tests; T-803 reopen/rejection; T-804 projection corruption; T-805 selected-row corruption; T-806 corrupt mutation; cross-component corruption; process schema rejection | Valid rows reconstruct through core; malformed envelope/schema/projection state never yields an aggregate, summary, mutation, or protocol success and is not repaired. | pass |
| One complete guarded update commits before success; every rejected path preserves prior state | T-804 duplicate/missing; all seven mutation tests; T-807 executor atomicity; T-808 I/O/delivery tests | Current commands commit one full successor under transaction/CAS before return; stale/domain/busy/zero-row/abort/delivery failures retain typed outcomes and prior durable state, except documented post-commit delivery uncertainty where refresh sees the successor. | pass |
| Actual processes prove restart, stale conflict, pagination, completion, and final retrieval | The two exact T-809 executable tests | Independent Cargo-built processes share a real explicit local file, complete the full multi-unit journey, and reject unsupported/malformed storage without logical mutation. | pass |
| Schema/storage/concurrency/recovery guarantees and nonclaims are precise | T-803 exact schema/PRAGMAs/dependency; T-806 writer/busy/abort; T-809 rejection; both T-810 documentation checks | Exact v1 ownership, local rollback-journal settings, writer serialization, finite busy behavior, fail-closed recovery, delivery uncertainty, and every locked exclusion are tested and stated without network/crash/idempotency overclaim. | pass |

INT-0009's stale-first conflict and one-step revision advance are additionally
covered by the unchanged 65-test core suite, mutation/concurrency tests,
cross-component propagation, and process E2E. The backend consumes that
realized contract without modifying core source or tests.

## Integration boundary and nonclaims

No repository mock, in-memory SQLite database, fake aggregate, service stub,
network, or clock participates in the storage/concurrency results. Raw fixture
mutations are deliberately outside the system under test and always followed by
production reads/commands. Recording writers prove only Rust `Write` stage
behavior; they do not prove operating-system or consumer acknowledgement.

These passing integrations do not establish crash-kill or power-loss recovery,
network-filesystem safety, performance/load limits, cross-unit atomicity,
automatic retry, idempotency, encryption, backup/replication, migration,
cryptographic audit, or deployment fitness. The selected boundary remains one
embedded local SQLite file with explicitly documented recovery responsibility.
