Finalized - DO NOT EDIT

# Sprint 8 Test Plan

## Intent Traceability

| Intent | Acceptance criterion | Build task / EARS clause | Verification |
|--------|----------------------|--------------------------|--------------|
| [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) | Multiple units survive restart with immutable workflows and complete history | T-802-E1; T-804-E1–E4; T-809-E1 | envelope round trip; create/get/reopen/corruption tests; process lifecycle E2E |
| [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) | Bounded exact-filter pagination has documented order/cursor/live consistency | T-801-E4; T-805-E1–E4; T-809-E1; T-810-E1 | validated query values; filter/order/live/corruption integration; process pagination; docs check |
| [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) | Adapter-owned versioned create/get/list/transition/complete boundary | T-801-E2/E3; T-807-E1–E5; T-808-E1/E2/E6 | backend model/revision tests; exact protocol shape/error tests; runner/exit tests |
| [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) | Every load replays through core; corrupt/unsupported state fails closed | T-802-E1–E4; T-803-E2/E3; T-804-E4; T-805-E4; T-806-E3 | strict codec/schema/projection/query/mutation corruption tests |
| [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) | Complete guarded mutation commits before response; failures preserve prior state | T-804-E3; T-806-E1–E5; T-808-E4/E5 | duplicate/missing, guarded command, race, busy/abort, and delivery-uncertainty tests |
| [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) | Actual-process restart, stale conflict, pagination, completion, and final get | T-809-E1/E2 | two process-level journeys |
| [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) | Precise schema/storage/concurrency/recovery guarantees and nonclaims | T-803-E1–E4; T-806-E4/E5; T-809-E2; T-810-E1/E2 | schema/pragma/dependency, concurrency/failure, process rejection, and docs checks |
| [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) | Stale-first typed conflict and exact one-step revision advance remain intact | T-806-E1–E4; T-809-E1 | guarded success/conflict/race tests plus full core regression |

## Unit Tests

### T-801 backend model

- **Intent:** [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md)
- `test_workspace_adds_isolated_backend_crate` [T-801-E1]: Cargo metadata recognizes the Rust 2024 backend crate; its task-boundary direct dependencies are only core/Serde/Serde JSON; core and old CLI manifest blobs are unchanged.
- `test_public_backend_model_exposes_complete_commands_and_results` [T-801-E2]: construct all five commands and inspect full view/summary/page/error values through public exports; no stored DTO is public.
- `test_public_backend_model_preserves_typed_u64_revisions` [T-801-E3]: construct every revision-bearing command/result/conflict at 0, `i64::MAX + 1`, and `u64::MAX`; getters and equality preserve the exact typed value without text or signed conversion.
- `test_query_limit_and_cursor_validation` [T-801-E4]: accept limits 1/100 and canonical ordinary/nil/absent-row UUID cursors; reject 0/101, uppercase, compact, malformed, whitespace-padded, and non-hyphenated cursor text before any store exists.
- Fixtures: fixed core values only; no database, clock, or mock.

### T-802 envelope and revision codecs

- **Intent:** [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md)
- `test_envelope_v1_round_trips_active_and_completed_units` [T-802-E1]: encode/decode custom forward/reverse/self workflows and completed history; compare every identity/workflow/state/revision/record field.
- `test_envelope_v1_rejects_malformed_or_unreplayable_lifecycle` [T-802-E2]: table-drive sequence 0/gap/duplicate, wrong source, undeclared edge, ineligible/nonterminal completion, record after completion, and declared phase/status/revision disagreement; require typed corruption and no unit.
- `test_envelope_v1_rejects_unknown_missing_invalid_and_unsupported_state` [T-802-E3]: mutate each required/unknown nested field, vocabulary/workflow topology, and representation versions 0/2; assert corrupt versus unsupported without matching human text.
- `test_revision_codecs_preserve_full_u64_and_reject_aliases` [T-802-E4]: canonical JSON strings and exact eight-byte big-endian blobs round-trip 0, `i64::MAX + 1`, and `u64::MAX`; invalid grammar and blob lengths 0/7/9 reject.
- Fixtures: adapter DTO JSON values plus public core constructors; assert the envelope shape differs from direct core Serde.

### T-807 protocol and executor

- **Intent:** [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md)
- `test_protocol_v1_decodes_all_locked_operations_strictly` [T-807-E1]: table-drive the exact create/get/list/transition/complete shapes and every unknown/missing/wrong-type/null/unsupported-version class; a sentinel DB path remains absent on structural rejection.
- `test_protocol_v1_rejects_semantically_invalid_values_before_storage` [T-807-E1,T-807-E2,T-807-E4]: table-drive malformed/noncanonical operation IDs, blank species/workflow/phase values, every invalid workflow topology class, limits 0/101, unknown status, malformed/noncanonical cursor, and revision strings with empty/sign/whitespace/leading-zero/fraction/overflow plus numeric/null/bool JSON; assert `invalid_revision` for semantic strings, structural `invalid_request` for non-strings, each other exact locked `invalid_*` code/field, and that a sentinel DB path remains absent.
- `test_protocol_v1_uses_decimal_strings_for_every_revision` [T-807-E2]: requests, units, summaries, mutations, and conflict expected/actual accept/emit canonical strings through `u64::MAX`, never numbers.
- `test_protocol_v1_serializes_exact_unit_page_and_mutation_results` [T-807-E3]: compare complete semantic JSON values, including full unit workflow/history and always-present page `next_cursor`.
- `test_protocol_v1_maps_exact_error_code_taxonomy` [T-807-E4]: exhaustively construct every locked request/command/storage error code; assert field-only and conflict-only optional members.
- `test_executor_preserves_backend_atomicity_on_modeled_failure` [T-807-E5]: against real SQLite, execute success then duplicate/missing/stale/domain/store failure paths and compare logical state before/after each rejection.

## Integration Tests

All storage tests use a real on-disk SQLite file under a test-owned temporary
directory, reopen through fresh backend/SQLite connections, and use fixed
canonical UUIDs. No `:memory:` store or mocked repository substitutes for the
sprint outcome. Raw SQLite is used only to prepare unsupported/corrupt fixtures.

### T-803 schema/open integration

- `test_new_empty_database_initializes_exact_schema_v1_and_pragmas` [T-803-E1]: inspect `user_version`, exact owned table/columns/types/checks/STRICT status, four exact indexes, absence of extra user objects, and every locked PRAGMA/busy timeout.
- `test_exact_schema_v1_reopens_without_migration` [T-803-E2]: close/reopen a known schema; compare logical schema/version and sentinel owned content with no reinitialization.
- `test_open_rejects_unversioned_unknown_incomplete_extra_and_corrupt_databases` [T-803-E3]: separate fixtures cover nonempty v0, version 2, missing/wrong/extra v1 objects, and non-SQLite bytes; assert typed failure and unchanged logical schema/version/content.
- `test_sqlite_dependency_is_bundled_and_adapter_only` [T-803-E4]: Cargo metadata/tree proves exact rusqlite 0.40.2 flags in backend and byte-identical core/CLI dependency manifests.

### T-804 create/get integration

- `test_create_commits_complete_revision_zero_unit` [T-804-E1]: fixed and omitted IDs create exact revision-0 rows; supplied nil/ordinary IDs are preserved, omission generates non-nil v4, and a fresh connection sees the complete row before return.
- `test_create_get_round_trip_multiple_units_across_reopen` [T-804-E2]: create distinct workflows/IDs, drop/reopen, and compare complete replayed views independently.
- `test_duplicate_create_and_missing_get_are_typed_and_nonmutating` [T-804-E3]: duplicate with different payload and absent get return distinct variants; row count and every accepted logical row value remain unchanged.
- `test_get_rejects_envelope_and_each_projection_mismatch_without_repair` [T-804-E4]: raw-SQL mutate envelope version/content plus ID/workflow/species/phase/status/revision projections one at a time; assert exact unsupported/corrupt/projection class and unchanged tampered row.

### T-805 query integration

- `test_list_filters_exact_fields_and_orders_ids_lexically` [T-805-E1]: seed workflows/species/phases/status values with case/Unicode/whitespace near-misses and out-of-order IDs; assert each filter/intersection and bound-parameter exactness.
- `test_list_enforces_bounds_and_exclusive_keyset_cursor` [T-805-E2]: seed 101 IDs; prove limits 1/100, limit+1 detection, ascending canonical IDs, next cursor only with another match, no boundary repeat, terminal null, empty page, and a valid absent/nil cursor; invalid values were already rejected by T-801.
- `test_list_pages_are_live_committed_views_with_mutable_membership` [T-805-E3]: after page one, insert/change matches below and above cursor; assert only the current committed after-cursor set without snapshot language.
- `test_list_fails_whole_page_for_any_selected_invalid_row` [T-805-E4]: corrupt one selected candidate among valid rows; require one page error and no partial result, while explicitly not claiming detection of filtered-out corruption.

### T-806 mutation/concurrency integration

- **Intents:** [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md), [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md)
- `test_guarded_transition_and_completion_commit_one_successor_before_return` [T-806-E1]: transition at 0/1 then complete at 2; after each method, a fresh connection sees exactly one new record and revisions 1/2/3.
- `test_stale_mutations_win_precedence_and_preserve_durable_state` [T-806-E2]: stale transition/completion combined with unknown/not-allowed/ineligible/terminal conditions returns exact expected/actual conflict after lock acquisition; full row unchanged.
- `test_current_revision_missing_corrupt_and_domain_rejections_preserve_state` [T-806-E3]: absent ID, corrupt row, and every current-revision core rejection return the matching variant before update; no insertion/repair/mutation.
- `test_two_isolated_writers_commit_exactly_once` [T-806-E4]: two real connections/threads start at revision 0; one commits revision 1 and the other conflicts actual 1; one durable record remains.
- `test_revision_qualified_zero_row_update_rolls_back` [T-806-E4]: a fixture forces zero affected rows in the real update path; require `concurrent_storage_change` and unchanged row rather than silent success.
- `test_busy_writer_times_out_once_without_retry_or_mutation` [T-806-E5]: a real independent `BEGIN IMMEDIATE` exceeds the 5,000-ms timeout; assert one busy outcome, elapsed lower bound, and unchanged state.
- `test_sqlite_update_abort_rolls_back_complete_row` [T-806-E5]: a fixture trigger aborts update after load; assert storage failure and unchanged envelope/projections/history.

### T-808 runner/process-shell integration

- `test_process_shell_requires_exactly_one_explicit_database_path` [T-808-E1]: missing/empty/`:memory:`/repeated/unknown/positional args yield usage 2/stderr and create no candidate/default database.
- `test_runner_dispatches_one_command_and_flushes_one_response` [T-808-E2]: real SQLite plus a recording writer covers every operation/result class; assert one compact JSON line and one final flush before status.
- `test_runner_enforces_one_mib_before_json_or_database` [T-808-E3]: exact-bound fixture needs its final byte to close JSON; one-over returns exact `request_too_large`, consumes/retains at most limit+1, and leaves sentinel DB absent.
- `test_runner_preserves_first_io_error_precedence` [T-808-E4]: deterministic read/body/newline/flush failures return operational error, skip later stages/status, and preserve source kind/message.
- `test_committed_mutation_survives_response_delivery_failures` [T-808-E5]: body/newline/flush failing writers act after a real commit; each reports delivery uncertainty and a fresh backend read proves the successor remains.
- `test_local_process_exit_and_stderr_mapping` [T-808-E6]: injectable shell covers exact exits 0/2/3/4/1, empty modeled stderr, and best-effort operational diagnostic.

### Cross-component integration

- `test_backend_codec_schema_crud_query_and_mutation_compose` [T-802-E1,T-803-E1,T-804-E2,T-805-E1,T-806-E1]: public backend API only; create custom workflow, reopen/query, transition, reopen/complete, and compare final view.
- `test_corruption_never_reaches_mutation_or_protocol_success` [T-802-E2,T-804-E4,T-805-E4,T-806-E3,T-807-E4/E5]: corrupt storage then exercise get/list/mutation/executor; require typed failure and unchanged logical row.
- `test_revision_conflict_propagates_core_to_local_protocol` [T-806-E2/E4,T-807-E2/E4]: exact expected/actual values survive core→backend→protocol while durable state remains unchanged.

## End-to-End Tests

- **Status:** possible
- `test_cubikan_local_persists_paginates_and_completes_across_processes` [T-809-E1]: invoke `env!("CARGO_BIN_EXE_cubikan-local")` repeatedly with one explicit temp path; create fixed IDs 02 then 01, exit, get/list/filter page 01 then 02, transition at `"0"`, reject a stale also-invalid command with expected `"0"`/actual `"1"`, refresh/transition at `"1"`, complete at `"2"`, exit, and get exact completed revision `"3"` with full workflow/history.
- `test_cubikan_local_rejects_unknown_and_malformed_schema_without_mutation` [T-809-E2]: actual processes open prebuilt version-2 and incomplete-version-1 files; assert exact storage response/exit and unchanged logical schema/version/rows.
- `verify_existing_stateless_cli_e2e_6_of_6` [final regression gate]: rerun and compare the six existing `cubikan` actual-process tests; this is regression evidence, not durable-backend proof.
- Hosted regression E2E: after the clean Build head is pushed to `dev`, record exact run/job/SHA and all five successful quality steps. Hosted CI strengthens regression evidence but is not the durability oracle.
- External boundary: product E2E uses only actual local processes, a test-owned local path, and bundled SQLite. No mocks/network, crash-kill/power-loss injection, network filesystem, or future runner reliability claim.

## Documentation and Repository Checks

- `test_backend_docs_define_versions_storage_pagination_concurrency_and_delivery` [T-810-E1]: verify all three v1 contracts, explicit path, envelope replay/projection checking, exact PRAGMAs/timeout, workflow-ID semantics, keyset/live-page behavior, exits, local VFS assumptions, fail-closed recovery, and refresh-after-delivery-uncertainty guidance.
- `test_backend_docs_state_all_locked_nonclaims` [T-810-E2]: require every named exclusion and reject language claiming network safety, snapshot pages, idempotency, cryptographic audit, or indefinite compatibility.
- `test_sprint_eight_scope_preserves_core_cli_ci_and_realized_intents` [final scope gate]: accepted-base diff permits workspace/lock, new backend/local crates, root README, INT-0010, and Sprint 8 Book evidence; require byte-identical `crates/cubikan-core/**`, `crates/cubikan-cli/**`, `.github/workflows/**`, and INT-0001–INT-0009.
- `test_book_v2_and_markdown_navigation_are_valid` [final Book gate]: installed Book validator reports 12 reachable intent chapters; a separate path/fragment resolver checks all local Markdown links.
- `test_workspace_quality_gates_pass` [final quality gate]: run in workflow order:
  `cargo +stable fmt --all -- --check`;
  `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings`;
  `RUSTFLAGS="-D warnings" cargo +stable check --workspace --all-targets`;
  `cargo +stable test --workspace --all-targets`;
  `cargo +stable test --doc --workspace`;
  then Book/link validation and `git diff --check`.
