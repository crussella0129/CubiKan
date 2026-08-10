Finalized - DO NOT EDIT

# Sprint 9 Test Plan

## Intent Traceability

| Intent | Acceptance criterion | Build task / EARS clause | Verification |
|--------|----------------------|--------------------------|--------------|
| [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) | Immutable definitions and validated directed edges preserve endpoint lifecycle state | T-901-E1–E4; T-903-E1–E4; T-904-E1 | `test_public_relationship_model_exposes_complete_contract`; `test_definition_create_commits_exact_view_before_return`; `test_definition_versions_round_trip_independently_across_reopen`; `test_edge_create_commits_without_mutating_endpoints` |
| [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) | Species, self-edge, cycle, and duplicate policy is exact-version scoped | T-903-E1–E4; T-904-E2–E4 | `test_edge_policy_rejections_are_atomic`; `test_self_and_cycle_policy_matrix_is_version_scoped`; `test_concurrent_cycle_creators_commit_once_then_reject_cycle` |
| [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) | Unknown, corrupt, policy-invalid, and concurrent work rejects atomically | T-902-E4–E6; T-903-E3–E4; T-904-E2–E5 | `test_migration_rejects_unowned_corrupt_and_wrong_version_sources`; `test_open_rejects_corrupt_v2_without_repair`; `test_selected_definition_value_corruption_fails_closed_without_repair`; `test_edge_policy_rejections_are_atomic`; `test_edge_delete_is_exact_non_cascading_and_atomic_on_failure` |
| [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) | Exact deletion is fail-closed and non-cascading; correction is delete then create | T-904-E5 | `test_edge_delete_is_exact_non_cascading_and_atomic_on_failure` |
| [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) | Exact-version direct relationship queries are bounded, canonical, exclusive, and live | T-905-E1–E4 | `test_relationship_query_ands_exact_filters_and_orders_direct_edges`; `test_relationship_query_enforces_bounds_complete_cursor_and_live_pages`; `test_relationship_query_missing_definition_and_absent_filter_are_distinct`; `test_relationship_query_rejects_selected_corruption_without_partial_results` |
| [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) | One unit can appear in multiple ephemeral projections without copied state | T-906-E1–E2; T-908-E1 | `test_projection_v1_ands_lifecycle_filters_with_direct_predicate`; `test_unit_appears_in_multiple_live_projections_without_copied_state`; `test_public_backend_relationship_projection_journey_across_reopen` |
| [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) | Projection v1 retains its query and uses canonical, bounded, exclusive, live pages | T-901-E2; T-906-E3–E4 | `test_public_relationship_model_exposes_complete_contract`; `test_projection_v1_reports_query_and_uses_exclusive_live_pages`; `test_projection_v1_missing_and_corrupt_inputs_fail_whole_page` |
| [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) | Exact schema v2 and explicit atomic migration preserve all Intent Unit values | T-902-E1–E6; T-908-E2 | `test_new_and_existing_v2_databases_have_exact_schema_and_pragmas`; `test_exact_v1_retains_unit_operations_and_caches_relationship_migration_guard`; `test_explicit_migration_orders_version_last_and_preserves_all_unit_columns`; `test_migration_rejects_unowned_corrupt_and_wrong_version_sources`; `test_busy_interrupted_and_racing_migrations_leave_one_exact_state`; `test_open_rejects_corrupt_v2_without_repair`; `test_public_backend_migrates_v1_then_relates_projects_and_preserves_units` |
| [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) | Documentation separates projections from execution graphs and preserves protocol v1 | T-907-E1–E3; T-908-E3 | `test_backend_docs_define_schema_v2_relationship_migration_and_projection_contract`; `test_docs_separate_projection_from_execution_graph_and_list_nonclaims`; `test_sprint_nine_scope_preserves_core_envelope_and_local_protocol_v1`; `verify_existing_process_and_workspace_regressions` |

## Primary EARS Verification

Each EARS clause has exactly one primary named verification. Auxiliary tests may
strengthen a clause but never replace this mapping.

| EARS clause | Primary named verification |
|-------------|----------------------------|
| T-901-E1 | `test_relationship_model_validates_ids_versions_policies_limits_and_cursors` |
| T-901-E2 | `test_public_relationship_model_exposes_complete_contract` |
| T-901-E3 | `test_relationship_error_taxonomy_is_typed_and_source_preserving` |
| T-901-E4 | `test_relationship_model_does_not_expose_storage_or_execution_authority` |
| T-902-E1 | `test_new_and_existing_v2_databases_have_exact_schema_and_pragmas` |
| T-902-E2 | `test_exact_v1_retains_unit_operations_and_caches_relationship_migration_guard` |
| T-902-E3 | `test_explicit_migration_orders_version_last_and_preserves_all_unit_columns` |
| T-902-E4 | `test_migration_rejects_unowned_corrupt_and_wrong_version_sources` |
| T-902-E5 | `test_busy_interrupted_and_racing_migrations_leave_one_exact_state` |
| T-902-E6 | `test_open_rejects_corrupt_v2_without_repair` |
| T-903-E1 | `test_definition_create_commits_exact_view_before_return` |
| T-903-E2 | `test_definition_versions_round_trip_independently_across_reopen` |
| T-903-E3 | `test_definition_duplicate_and_missing_are_typed_and_nonmutating` |
| T-903-E4 | `test_selected_definition_value_corruption_fails_closed_without_repair` |
| T-904-E1 | `test_edge_create_commits_without_mutating_endpoints` |
| T-904-E2 | `test_edge_policy_rejections_are_atomic` |
| T-904-E3 | `test_self_and_cycle_policy_matrix_is_version_scoped` |
| T-904-E4 | `test_concurrent_cycle_creators_commit_once_then_reject_cycle` |
| T-904-E5 | `test_edge_delete_is_exact_non_cascading_and_atomic_on_failure` |
| T-905-E1 | `test_relationship_query_ands_exact_filters_and_orders_direct_edges` |
| T-905-E2 | `test_relationship_query_enforces_bounds_complete_cursor_and_live_pages` |
| T-905-E3 | `test_relationship_query_missing_definition_and_absent_filter_are_distinct` |
| T-905-E4 | `test_relationship_query_rejects_selected_corruption_without_partial_results` |
| T-906-E1 | `test_projection_v1_ands_lifecycle_filters_with_direct_predicate` |
| T-906-E2 | `test_unit_appears_in_multiple_live_projections_without_copied_state` |
| T-906-E3 | `test_projection_v1_reports_query_and_uses_exclusive_live_pages` |
| T-906-E4 | `test_projection_v1_missing_and_corrupt_inputs_fail_whole_page` |
| T-907-E1 | `test_backend_docs_define_schema_v2_relationship_migration_and_projection_contract` |
| T-907-E2 | `test_docs_separate_projection_from_execution_graph_and_list_nonclaims` |
| T-907-E3 | `test_sprint_nine_scope_preserves_core_envelope_and_local_protocol_v1` |
| T-908-E1 | `test_public_backend_relationship_projection_journey_across_reopen` |
| T-908-E2 | `test_public_backend_migrates_v1_then_relates_projects_and_preserves_units` |
| T-908-E3 | `verify_existing_process_and_workspace_regressions` |

## Test Fixtures and Oracles

- Use only real test-owned SQLite files under RAII temporary directories; never
  use `:memory:`, a mocked backend, or a shared fixed path. Separate backend
  handles/threads represent competing writers.
- Reuse fixed replay-valid workflows and canonical IDs inserted out of lexical
  order. Snapshot every `intent_units` column as typed/text/blob values before
  migration and relationship mutations; do not require whole-file byte identity
  after valid SQLite commits.
- Raw `rusqlite` fixture access is allowed only to create exact legacy-v1 stores,
  unsupported/corrupt rows, lock holders, and deterministic abort/interruption
  conditions. Product journeys use public `SqliteBackend` after setup.
- Relationship fixtures include all four `Allow | Reject` self/cycle policy
  combinations, two versions of one definition ID, optional endpoint-species
  constraints, and enough fixed IDs for 101-edge pagination.
- Validation precedence is asserted after an immediate writer is acquired:
  definition; source replay; target replay; source species; target species; self;
  duplicate; non-self cycle. Busy may precede semantic classification.
- Corruption intended for exact get/delete retains the valid typed lookup
  identity. CHECK-invalid definition IDs/version blobs and FK-invalid edge keys
  belong to exact-v2 open validation. Malformed but FK-satisfied edge endpoint
  text belongs to operation-selected reachability/list/projection tests; open
  makes no global semantic replay claim.
- Rejected migration/open fixtures compare schema version, owned objects, typed
  row values, and readable non-SQLite source bytes. SQLite sidecar creation or
  removal and page layout are not compatibility oracles.
- Human-readable error text, SQLite page layout, wall-clock timing beyond the
  configured busy bound, and filtered-out hostile corruption are not compatibility
  oracles.

### Semantic selection and corruption boundaries

“Selected” means a row decoded as an input or candidate of the current operation,
including the one-row pagination lookahead. It does not mean every table row, a
row excluded by SQL filters/cursors, or rows after lookahead.

- Open validates exact schema metadata, SQL constraints through integrity
  checking, and foreign keys; it does not globally replay definitions, edges, or
  endpoint envelopes.
- Migration selects and replay-validates every v1 `intent_units` row because its
  acceptance contract explicitly requires the complete source scan.
- Definition create/get selects the exact key. A corrupt exact collision returns
  corrupt-definition; a valid collision returns duplicate-definition.
- Relationship create selects its exact definition, proposed source/target,
  duplicate key, and edge identities visited by same-definition reachability.
  The recursive query validates visited edge identities but does not replay every
  endpoint already in the graph.
- Relationship delete selects definition, source, target, and exact edge in the
  locked delete precedence; no unrelated edge or endpoint is selected.
- Relationship list selects its exact definition and at most `limit + 1` edges
  surviving filters/cursor, plus both endpoints of those candidates.
- Projection without a direct predicate selects at most `limit + 1` unit rows
  surviving lifecycle filters/cursor. A direct predicate additionally selects
  its definition, replay-valid anchor, and at most `limit + 1` matching
  edge/candidate-unit pairs; it performs no transitive or whole-graph validation.

Any corrupt selected candidate fails the complete operation/page without partial
results or repair. Constraint-bypassing edits and corruption outside these
boundaries are not promised to be discovered by that operation.

## Unit Tests

### T-901 public model and error tests

- **Intent:** [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- `test_relationship_model_validates_ids_versions_policies_limits_and_cursors` [T-901-E1]: accept boundary-length canonical IDs, versions 1/`i64::MAX + 1`/`u64::MAX`, every policy, page limits 1/100, nil/ordinary endpoint IDs, and matching complete cursors; reject empty/65-byte/non-ASCII/uppercase/bad-character IDs in empty→length→first-byte→remaining-byte precedence with exact byte index, version 0, limits 0/101, and a cursor whose definition differs from the query before storage.
- `test_public_relationship_model_exposes_complete_contract` [T-901-E2]: construct every definition/edge create/get/delete/list value, both projection directions, projection-without-relation, views/pages, complete retained query, and next cursors; inspect every locked field/getter, show cursor retains the complete edge identity, and make a second direct predicate or partial deletion identity unrepresentable by construction.
- `test_relationship_error_taxonomy_is_typed_and_source_preserving` [T-901-E3]: exhaustively construct/derive definition-ID/version/query model errors, migration-required, both migration-error variants, definition duplicate/missing/corrupt, endpoint role/not-found/corrupt/species, self, cycle, duplicate/missing/corrupt relationship, and wrapped busy/storage/schema/unit errors; assert exact fields, distinct variants, and `Error::source` without message parsing.
- `test_relationship_model_does_not_expose_storage_or_execution_authority` [T-901-E4]: public compile/API inventory proves no SQL row DTO, direct core-Serde wrapper, textual/Serde relationship cursor contract, timestamp, actor, stored board, scheduler, executor, or local-protocol type enters the model.

### T-902 schema and migration unit tests

- `test_exact_v1_retains_unit_operations_and_caches_relationship_migration_guard` [T-902-E2]: an in-module real-file test opens exact v1; uses public create/get/list/transition/complete; asserts cached schema version 1; invokes the private relationship-capability guard and receives migration-required before relationship SQL; and preserves v1 schema, rows, envelopes, and projections while retaining the realized connection PRAGMA contract.
- `test_explicit_migration_orders_version_last_and_preserves_all_unit_columns` [T-902-E3]: an in-module real-file test exercises the production migration body, observes version 1 until the final version step, commits exact v2, compares every pre/post `intent_units` value including envelope bytes and revision blobs, verifies that a pre-migration handle retains cached v1 capability and can still perform an existing lifecycle get after external migration, and verifies that only a reopened v2 handle passes the private relationship-capability guard. Public operation proof belongs to T-908-E2.
- `test_busy_interrupted_and_racing_migrations_leave_one_exact_state` [T-902-E5]: an in-module real-file table uses the private migration-stage seam only for deliberate pre-commit interruption, plus real independent lock holders and racing migration callers; it requires one return/no retry, exact rollback for interruption/busy, one exact v2 race winner, and loser `SourceVersionNotOne { found: 2 }`.

## Integration Tests

### T-902 exact schema v2 and migration

- `test_new_and_existing_v2_databases_have_exact_schema_and_pragmas` [T-902-E1]: inspect a real fresh file and reopen it; assert exact objects/DDL/columns/constraints/indexes/autoindexes, user version 2, integrity/foreign-key checks, and existing DELETE/EXTRA/foreign-key/trusted/read-uncommitted/locking/busy settings.
- `test_migration_rejects_unowned_corrupt_and_wrong_version_sources` [T-902-E4]: table a nonexistent path, empty-v0, unowned-v0, corrupt unit under exact v1, malformed/extra-object v1, already-v2, unsupported version, and non-SQLite inputs; assert exact typed source error, no creation of the missing path, and unchanged logical/readable snapshots.
- `test_open_rejects_corrupt_v2_without_repair` [T-902-E6]: table missing/wrong/extra objects, rootpage/integrity damage, CHECK-invalid definition IDs/version blobs, FK-invalid edge keys, other invalid foreign-key rows, and invalid schema text; assert fail-closed classification and unchanged logical snapshots without treating sidecars as an oracle.

### T-903 immutable definitions

- `test_definition_create_commits_exact_view_before_return` [T-903-E1]: create definitions with/without species constraints and every policy, then use a fresh connection immediately after each return to compare the exact committed view.
- `test_definition_versions_round_trip_independently_across_reopen` [T-903-E2]: create gapped/out-of-order versions under one ID with different constraints/policies; reopen and compare exact independent views without inferred latest or supersession.
- `test_definition_duplicate_and_missing_are_typed_and_nonmutating` [T-903-E3]: duplicate a valid stored identity with different content and get a missing version; assert exact errors and complete row/unit snapshots unchanged. Corrupt exact-key collision precedence belongs only to T-903-E4.
- `test_selected_definition_value_corruption_fails_closed_without_repair` [T-903-E4]: retain the valid ID/version key while independently tampering direction, species, self-policy, and cycle-policy values; exact create collision and get return corrupt relationship state rather than duplicate/success and preserve the selected row. Edge/list/projection dispatch is verified only in its later owning task.

### T-904 relationship mutation policy

- `test_edge_create_commits_without_mutating_endpoints` [T-904-E1]: create a valid edge, reopen immediately, compare its full view, and assert byte-identical endpoint envelopes/projections plus unchanged revisions/histories.
- `test_edge_policy_rejections_are_atomic` [T-904-E2]: table missing/corrupt definition, missing/corrupt source, missing/corrupt target, source/target species mismatch, rejected self, duplicate, corrupt edge identity visited by cycle reachability, and rejected cycle combined with earlier errors; assert locked precedence and exact unit/edge snapshots.
- `test_self_and_cycle_policy_matrix_is_version_scoped` [T-904-E3]: exercise all four policies; allowed self bypasses cycle rejection; non-self paths close or reject only according to cycle policy; other definition versions do not affect reachability.
- `test_concurrent_cycle_creators_commit_once_then_reject_cycle` [T-904-E4]: separate connections/threads barrier-start opposite non-self edges under one reject-cycle definition; require exactly one success and one cycle rejection (busy is not accepted), then prove no forbidden committed cycle or partial row.
- `test_edge_delete_is_exact_non_cascading_and_atomic_on_failure` [T-904-E5]: delete one of several edges by full identity, prove no cascade or endpoint/definition change, reject a different missing identity, recreate the corrected edge, and table missing/corrupt definition, missing/corrupt source, missing/corrupt target, source/target species mismatch, a real writer lock, and a SQLite abort in the locked precedence; every failure returns once without retry and preserves relationship, definition, and endpoint snapshots.

### T-905 direct relationship query

- `test_relationship_query_ands_exact_filters_and_orders_direct_edges` [T-905-E1]: query an exact definition with no/source/target/both filters, near-miss IDs, and graph paths; assert direct matches only and canonical `(source,target)` ordering independent of insertion order.
- `test_relationship_query_enforces_bounds_complete_cursor_and_live_pages` [T-905-E2]: seed 101 edges; prove limits 1/100, invalid 0/101, validated lookahead, exclusive full-identity cursor including a same-definition cursor naming no stored edge, cross-definition cursor rejection, no repeats, terminal null cursor, and insertion/deletion membership between pages.
- `test_relationship_query_missing_definition_and_absent_filter_are_distinct` [T-905-E3]: missing definition returns not-found; valid definition plus source/target filters whose units do not exist returns an empty page.
- `test_relationship_query_rejects_selected_corruption_without_partial_results` [T-905-E4]: retain selectable definition keys while corrupting selected/lookahead non-key definition values, relationship endpoint text, or endpoint unit state; require whole-page failure/no repair and demonstrate that filtered-out corruption is not claimed to be scanned.

### T-906 projection query v1

- `test_projection_v1_ands_lifecycle_filters_with_direct_predicate` [T-906-E1]: prove no-relation equivalence to lifecycle filters, outgoing target and incoming source semantics, ANDed workflow/species/phase/status filters, and no transitive membership.
- `test_unit_appears_in_multiple_live_projections_without_copied_state` [T-906-E2]: select one unit through multiple definitions/anchors, compare the one canonical summary, then mutate an edge/lifecycle phase and observe only later query membership change with no stored board row or relationship-induced revision.
- `test_projection_v1_reports_query_and_uses_exclusive_live_pages` [T-906-E3]: unchanged state returns identical page/query/version; 101 units prove canonical ordering, 1/100 bounds, validated lookahead, exclusive cursors, and live membership changes.
- `test_projection_v1_missing_and_corrupt_inputs_fail_whole_page` [T-906-E4]: missing definition/anchor and selectable non-key definition, relationship endpoint, anchor/unit, selected-unit, or lookahead corruption each yields the precise error without partial summaries or repair.

## Repository and Documentation Checks

### T-907 contract and scope

- `test_backend_docs_define_schema_v2_relationship_migration_and_projection_contract` [T-907-E1]: checklist exact schema compatibility/migration/reopen, definition/edge identity/policies/precedence, delete/recreate, direct query, cursor, projection version/query/reproducibility, transactions, busy, recovery, and live pages across root/backend/local guides.
- `test_docs_separate_projection_from_execution_graph_and_list_nonclaims` [T-907-E2]: require every locked nonclaim and reject wording that promises automatic migration, atomic correction, cascade, forensic erasure, snapshots, transitive graph execution, protocol-v1 relationships, security, or stable compatibility.
- `test_sprint_nine_scope_preserves_core_envelope_and_local_protocol_v1` [T-907-E3]: accepted-base diff/tree and semantic fixtures prove byte-identical core and stored envelope codec, unchanged local production operation/field/result/error-code sets, unchanged stateless CLI, manifests/lock/workflow, and only the declared local E2E unsupported-schema fixture update from v2 to v3 while retaining exact code, shape, exit 4, empty stderr, and nonmutation.

## End-to-End Tests

- **Status:** possible at Sprint 9's declared public Rust backend boundary.
- `test_public_backend_relationship_projection_journey_across_reopen` [T-908-E1]: public API only after fresh-file setup; create units/definitions/edges, reject invalid work, list, project into multiple views, change lifecycle membership, delete/recreate, repeatedly reopen, and compare exact final units/definitions/edges/projections.
- `test_public_backend_migrates_v1_then_relates_projects_and_preserves_units` [T-908-E2]: raw fixture setup creates exact v1 only; create/get definition, create/delete/list relationship, and projection first report migration-required without mutation; after another connection migrates, the old handle remains cached v1 yet successfully exercises create/get/list/transition/complete before continuing to reject every new API; then reopen plus public APIs preserve all old unit columns and persist new v2 relation/projection state.
- `verify_existing_process_and_workspace_regressions` [T-908-E3]: existing two `cubikan-local` and six `cubikan` actual-process E2Es, all backend/core suites, and doctests remain green. This is regression evidence, not relationship wire E2E.
- Process-level relationship E2E is outside the declared boundary because local protocol v1 intentionally remains unchanged. A future separately authorized local-protocol-version intent unlocks it; Sprint 9 makes no protocol-v2 claim.

## Final Quality Gates

Run against one clean tested candidate in this order:

1. `cargo +stable metadata --no-deps --format-version 1` and bounded dependency trees.
2. `cargo +stable fmt --all -- --check`.
3. `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings`.
4. `RUSTFLAGS="-D warnings" cargo +stable check --workspace --all-targets`.
5. `cargo +stable test --workspace --all-targets`.
6. `cargo +stable test --doc --workspace`.
7. Run the installed Book-v2 validator for intent schema/state and a separate
   pinned-object Markdown resolver for Summary reachability and local
   path/fragment resolution; these are procedural gates, not additional named
   EARS tests.
8. Accepted-base path/semantic scope checks and `git diff --check`.
9. Push the clean Build/Test candidate to `dev` and require the exact-SHA hosted `Rust CI` quality job to complete successfully. Hosted CI is regression evidence, not relationship-process E2E or merge authorization.

## Negative Boundaries

- No mock/in-memory SQLite substitute, source-string SQL oracle, deliberate real
  disk exhaustion, network filesystem, kill/power-loss test, or unstable wall-
  clock race is required.
- No definition list/delete, relationship timestamps/actors/history, atomic
  replacement, cascade, Intent Unit deletion, transitive traversal, OR/NOT DSL,
  stored boards, snapshot pagination, WIP/scheduling/execution, protocol v2,
  authentication/tenancy/network/UI/provenance/metric/blockchain behavior, or
  dependency/workflow change may enter the sprint.
- Passing tests prove semantic deletion from current rows, not secure/forensic
  erasure; prove SQLite transaction rollback under the tested VFS, not backup,
  downgrade, crash-kill, device-loss, or indefinite compatibility.
