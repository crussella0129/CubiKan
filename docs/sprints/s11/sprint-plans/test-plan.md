Finalized - DO NOT EDIT

# Sprint 11 Test Plan

## Intent Traceability

Build EARS clauses are the normative implementation promises. The Primary EARS
Verification table maps all 80 clauses exactly once to 80 unique named
verifications; the intent tables below additionally show that every acceptance
criterion affected by Sprint 11 has an observable oracle. Build and Test are
finalized atomically after this mapping is accepted.

### [INT-0008 — traceable intent instantiation and artifact provenance](../../../intents/INT-0008-traceable-intent-instantiation.md)

| Acceptance criterion | Build coverage (joins to named Primary verification) | Verification oracle |
|---|---|---|
| Every current-generation creation boundary requires one exact immutable origin; missing, null, malformed, over-bound, originless, placeholder, legacy-attribution, and in-place correction paths are absent | T-1102-E1, T-1102-E4, T-1103-E1, T-1107-E1, T-1112-E1, T-1113-E1 | Bound/value tables, codec-entry instrumentation, API compile checks, and both strict protocol corpora |
| Origin is distinct from ID and survives lifecycle, core serialization/restoration, accepted create event, envelope, attested projection, and dual-node rebuild byte-for-byte | T-1103-E1, T-1103-E3, T-1107-E1, T-1108-E4, T-1110-E3, T-1115-E4 | Pallet state/event snapshots, envelope replay, projector coordinates, and semantic rebuild digests |
| Whole-unit and exact revisions `0..=current`, including revision zero and interior history, remain distinct; future/nonexistent revisions reject | T-1105-E1, T-1105-E2 | Independent provenance fixtures and byte-equal rejected-state snapshots |
| Canonical record/revoke does not advance lifecycle revision; projected results identify the accepted event coordinate and verified checkpoint | T-1105-E1, T-1105-E3, T-1109-E2, T-1110-E3 | Pallet aggregate snapshots plus capability-gated query/result fixtures |
| Complete association identity is unit, subject, and reference; many-to-many links work and active duplicates reject | T-1105-E1, T-1105-E2, T-1109-E4 | Many-to-many state/query corpus with complete-key ordering |
| Correction is independently canonical revoke then record, may expose intermediate absence, retains event history, and rejects missing/repeated revoke | T-1105-E3 | Ordered accepted-event replay and active-row membership assertions |
| Forward/reverse pages use complete-key order, limits 1–100, exclusive cursors, validated lookahead, exact checkpoint, and rebuild equality | T-1109-E2, T-1109-E3, T-1109-E4, T-1115-E4 | Private test-issued pinned snapshots, DELETE-lock schedule, and dual archive rebuild comparison |
| Git resolves repository-qualified full commit identity with the algorithm-specific namespace; later source movement or blame cannot rewrite it | T-1114-E1, T-1114-E2, T-1114-E3, T-1114-E4 | Real SHA-1 repository, capability-gated SHA-256 journey/fixture, mutation audit, and rejection table |
| Documentation separates ledger acceptance, read-model attestation, provider verification, attribution, causality, quality, and satisfaction | T-1105-E4, T-1116-E1, T-1116-E2 | Metadata/data inventory plus repository-owned documentation/nonclaim checks |
| Canonical payloads exclude credentials, prompts, transcripts, source bodies, provider secrets, private locators, and production identifiers | T-1105-E4, T-1106-E4, T-1113-E4, T-1115-E5 | SCALE/JSON/log/config byte scans and runtime inventory |
| The first journey is pinned, loopback-only, synthetic, local, and performs no public account/funding/registration/coretime/deployment/governance action or live shared-security claim | T-1110-E1, T-1115-E1, T-1115-E5, T-1116-E1, T-1116-E2 | RPC parser/probes, exact socket/process audit, and documentation guards |

### [INT-0009 — revisioned lifecycle commands](../../../intents/INT-0009-revisioned-lifecycle-commands.md)

| Acceptance criterion | Build coverage (joins to named Primary verification) | Verification oracle |
|---|---|---|
| New units begin at documented revision zero without wall-clock derivation | T-1102-E3, T-1103-E1 | Wasm dependency inspection and exact create snapshot |
| Each accepted transition/completion advances revision and one lifecycle record exactly once | T-1103-E3 | Core/pallet successor-state and event comparison |
| A current expectation preserves domain behavior; stale returns typed conflict with every aggregate field unchanged | T-1103-E3, T-1103-E4 | Paired current/stale tables with byte digests |
| Revision comparison precedes remaining lifecycle validity after version, authorization, and target selection | T-1103-E2, T-1103-E4 | Pairwise precedence matrix and domain-read instrumentation |
| Stale-plus-domain-invalid and current-plus-domain-invalid paths are atomic | T-1103-E2, T-1103-E4 | Pre/post storage/event digests |
| Validated restoration rejects revision/history/phase/status disagreement and preserves exact revision | T-1102-E2, T-1107-E3, T-1108-E4 | Independent conformance corpus and envelope replay |
| Documentation limits revision to optimistic conflict detection, not locking, isolation, cross-unit atomicity, or delivery idempotency | T-1116-E1, T-1116-E2 | Portable documentation/nonclaim validator |

### [INT-0012 — relationship and projection semantics carried forward](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md)

| Acceptance criterion | Build coverage (joins to named Primary verification) | Verification oracle |
|---|---|---|
| Immutable versioned definitions and directed edges preserve endpoint identity, workflow, phase, status, history, and revision | T-1104-E1 | Pallet state/event byte snapshots |
| Source/target species, self, cycle, duplicate, version, and exact 128-edge capacity policies are definition-version scoped | T-1104-E2, T-1104-E3 | Ordered definition/edge failure matrices and bounded traversal weights |
| Unknown/policy-invalid and competing cycle-closing work is atomic under canonical sequential execution | T-1104-E3, T-1104-E4 | Both extrinsic orders plus independent cycle traversal |
| Exact deletion validates definition, endpoints, species, and active edge; is non-cascading; preserves neighbors; and correction requires a later create | T-1104-E5 | Ordered deletion matrix and post-state graph comparison |
| Exact-definition lookup and direct-edge pages are bounded, binary complete-key ordered, exclusive-cursor, live, and selected-corruption fail closed | T-1109-E2, T-1109-E3, T-1109-E4 | Internal test-only snapshot issuer, lookahead corruption fixtures, and C/C+1 schedule |
| A unit can appear in multiple ephemeral projections without copied lifecycle authority | T-1109-E4 | Multi-version/multi-predicate query corpus |
| Projection v1 combines lifecycle filters with at most one direct predicate, uses ID order/1–100/exclusive cursor/live pages, and rebuilds from canonical events | T-1109-E2, T-1109-E3, T-1115-E4 | Query matrix, live-page lock oracle, and dual rebuild digests |
| Historical SQLite-v1/v2 migration authority is superseded: v3 is schema-only fresh creation and old generations reject unchanged | T-1108-E1, T-1108-E2, T-1108-E4, T-1116-E3 | Exact-schema/open tests and immutable-snapshot scope guard |
| Documentation separates board views from execution/scheduling and disclaims relationship ownership/history/actor/timestamp/authorization meaning | T-1116-E1, T-1116-E2 | Cross-surface terminology and forbidden-claim checks |

### [INT-0014 — blockchain authority and verified SQLite projection](../../../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)

| Acceptance criterion | Build coverage (joins to named Primary verification) | Verification oracle |
|---|---|---|
| Exact SDK/Rust/Wasm/Subxt/template/relay/collator/Zombienet revisions, lockfiles, assets, and checksums are reproducible and run offline after one fetch | T-1101-E1, T-1101-E3, T-1117-E2 | Pin parser/mutation tests and fetch-network denial trace |
| Nested `chain/` isolates SDK/FRAME/Cumulus while native/Wasm/metadata/runtime identities agree; root gets only the planned exact client graph, with `backend -> chain-client`, no reverse edge, and no public raw projection/capability seam | T-1101-E2, T-1106-E2, T-1109-E1, T-1110-E6 | Baseline/final dependency graphs, artifact semantic checks, and API privacy tests |
| Call/storage/event values and collections are bounded SCALE/`MaxEncodedLen`; every accepted event is at most 1 MiB and generated weights cover maxima | T-1102-E1, T-1102-E3, T-1104-E3, T-1106-E3 | Boundary corpus, metadata/encoded maxima, maximal fixtures, and benchmark output |
| Command schema version 1 is explicit; another decodable version precedes domain reads; malformed/null/over-bound bytes reject structurally | T-1102-E4, T-1103-E2, T-1104-E2, T-1104-E3, T-1105-E2, T-1105-E3 | Codec instrumentation and operation-specific precedence matrices |
| Pre-inclusion codec/validity rejection consumes no nonce/fee; included typed dispatch failure consumes normal nonce/fee/System failure while CubiKan state/events remain equal | T-1106-E7 | Runtime Executive accounting matrix separated from pallet-domain snapshots |
| Create requires caller UUID/origin/species/workflow, uses no runtime randomness, starts revision zero, and emits one replay-complete event | T-1103-E1, T-1103-E2 | Exact success snapshot and decoded failure table |
| Transition/completion uses observed revision, advances once, and preserves stale-before-domain semantics under competing extrinsics | T-1103-E3, T-1103-E4, T-1103-E5 | Core/pallet comparison and both-order concurrency matrix |
| Exactly 256 lifecycle records are representable; record 257 rejects before remaining lifecycle validity; no counter uses zero as sentinel or wraps | T-1103-E6, T-1107-E3 | Boundary state/event digests and shared core/chain fixtures |
| Relationship definition/create/delete precedence and bounded query/projection semantics are carried onto the chain and back into the projection | T-1104-E1–E5, T-1109-E2, T-1109-E4 | Pallet matrices and verified query corpus |
| Provenance required-origin, whole/exact subject, duplicate/capacity/revoke/query/non-attribution semantics are carried forward without lifecycle revision change | T-1105-E1–E4, T-1109-E4 | Pallet state/events, metadata inventory, and bidirectional query corpus |
| Direct signed maximum-16 technical allowlist authorizes without ownership; mock Root replacement is bounded, and the fixed runtime exposes no reachable Root/origin-transforming route | T-1103-E7, T-1106-E1, T-1106-E5 | Mock-runtime call tests and fixed-runtime call/origin inventory |
| Ordinary weight/length fees and zero tip apply; dev balances have no economic meaning | T-1106-E4, T-1111-E2, T-1111-E4 | Runtime payment assertions, signed payload decode, and documentation guard |
| One immutable post-genesis deployment anchor traces both genesis hashes, ParaId 1000, deployment/pallet/event/spec/code identities, fixed bytes, and no runtime upgrade | T-1106-E1, T-1106-E2, T-1106-E4, T-1106-E6, T-1110-E1 | Genesis/artifact checks, RPC provenance mutation table, and code-hash timeline |
| CubiKan/pallet genesis anchor fields contain only non-self-referential values, standard runtime genesis stays exact, and relay/parachain block-zero hashes are sourced post-genesis rather than predicted by a chain spec | T-1106-E1, T-1106-E6 | Genesis storage inventory, standard-runtime fixture, and post-genesis RPC provenance mutation oracle |
| Accepted events contain deployment/schema/global sequence/replay payload and stable finalized block/extrinsic/System/global coordinate; signer is supplemental | T-1103-E1, T-1103-E3, T-1104-E1, T-1104-E5, T-1105-E1, T-1105-E3, T-1110-E3 | Event-only independent replay and restrictive joined-hash projection assertions |
| Global sequence begins at one, an empty stream is absent, a later zero-event block retains prior checkpoint sequence, `u64::MAX` is valid, and the next mutation rejects | T-1103-E6, T-1110-E2 | Counter boundary tests plus block-zero/later-zero-event checkpoint fixtures |
| Projector consumes finalized blocks only, verifies anchor/parent/runtime/code/count/sequence/replay, and commits each whole block/checkpoint once | T-1110-E1, T-1110-E2, T-1110-E3, T-1110-E4 | RPC/archive probes, bootstrap transaction, block fault matrix, and no-partial-state assertions |
| Fresh schema v3 is schema-only; first sync atomically inserts anchor, zero-event block zero, and nullable checkpoint before any read capability exists | T-1108-E1, T-1110-E2 | Row-count inspection, transaction fault injection, and capability privacy checks |
| Duplicate/conflicting/gapped/out-of-order/malformed/wrong-anchor/runtime/version/finality/replay inputs expose no partial rows/checkpoint/capability and conflicting finalized hash is fatal | T-1110-E4, T-1110-E6 | Complete equality/no-op oracle, failure source chains, and restart/contention tests |
| Public reads require a nonserializable single-read capability minted only after full archive-RPC comparison and independent replay against the same pinned transaction; coherent DB-only forgery rejects before mint | T-1109-E1, T-1109-E2, T-1109-E3, T-1110-E5 | Production API privacy tests, internal query harness, corruption corpus, and RPC-vs-DB semantic digests |
| No SQLite value influences mutation preflight, deployment selection, revision inference, nonce, signing, or send | T-1111-E5, T-1113-E3 | Call-order spy proves zero SQLite reads and zero signer/send activity for forged-cache cases |
| The two protocol-v2 identities have separately authored, independently hashed schemas and raw fixture manifests with exact inventories and no implementation-generated oracle | T-1112-E3, T-1113-E6 | Independent manifest verifier, fixed hashes, completeness registry, and mutation rejection |
| Schema v3/envelope v2 are fresh-only and backend-private; schema v1/v2, envelope v1, extra/edited/corrupt/wrong-checkpoint files reject without migration/adoption/repair | T-1108-E1, T-1108-E2, T-1108-E4, T-1108-E5 | Exact DDL/open/replay tests and public API compile-fail checks |
| Maximal 256-record envelope remains at most 2,097,152 bytes and canonical record 257 rejects before an unprojectable unit exists | T-1103-E6, T-1107-E3, T-1108-E4 | Checked worst-case formula, adversarial bytes, and chain boundary result |
| SQL values are bound and SQL structure is private/static/comment-free; exact SQLite feature/open/runtime/resource defenses distinguish injection text from hostile paths/files/resources | T-1108-E2, T-1108-E3, T-1108-E5, T-1108-E6 | Authorizer/trace/open syscall probes, injection values, limit/page/Busy boundaries, and unchanged schema bytes |
| Linux-only local-filesystem creation/open/journal semantics require owner-only regular non-symlink direct children; unsupported/non-Linux/network/DrvFS/FUSE/custom semantics reject before access | T-1108-E1, T-1108-E3, T-1111-E1 | Platform/filesystem/path/mode/type matrix with no-created/no-opened evidence |
| Existing files are preflighted OS/SQLite read-only with defenses and exact UTF-8/page/integrity/FK/schema checks before writer reopen; unexpected sidecars fail without recovery | T-1108-E2, T-1108-E3 | Ordered authorizer/syscall trace and byte-identical hostile fixtures |
| Deleting SQLite and replaying through either synchronized archive collator reconstructs all semantic state, pages, event coordinates, and checkpoint | T-1115-E3, T-1115-E4 | Named catch-up checkpoint and uninterrupted/A/B semantic digests |
| Both adapters preserve 1 MiB ingestion, strict v2 shapes, newline/flush/error/exit behavior; stateless is simulation-only, local owns the fifteen chain operations, and v1 rejects before access | T-1107-E2, T-1112-E1–E3, T-1113-E1–E6 | Root regression tests and separately hashed raw protocol corpora |
| Local RPC accepts literal loopback `ws` IP/explicit port only and local mutations accept named dev signers only | T-1110-E1, T-1113-E1, T-1113-E4 | URL/argument rejection table with dial/log/sign/open spies |
| Finalized submission waits at most 120 seconds, exactly matches accepted event to prepared extrinsic, distinguishes known rejection/lag/indeterminacy, and never retries | T-1111-E2–E5, T-1113-E3 | RPC/watch fault matrix, exact coordinate/event match, send counter, and response fixtures |
| Per-signer journal/lock is owner-only, fixed/versioned/checksummed, durably published, crash-recoverable, same-signer serialized, and clears only after exact finalization or complete birth-through-death absence scan | T-1111-E1–E6 | Real-process lock test and every publication/scan fault point |
| Exactly two validators and two archive collators use the normalized fixed loopback socket inventory, one relay runtime identity, and one distinct byte-identical CubiKan runtime across collators; stopped collator catches up and passes archive probes before rebuild use | T-1115-E1, T-1115-E2, T-1115-E3, T-1115-E4 | Generated-argv hash/normalizer mutations, two-runtime identity inventory, PID/socket checks, failover checkpoint, probe transcript, and dual rebuild |
| Git demonstration detects SHA-1/SHA-256 and never promotes blame, committer, or signer into attribution | T-1114-E1–E4 | Real repository/fixture tests and finalized/rebuilt association comparison |
| Journey remains synthetic/dev/loopback and performs no public blockchain, account, key, funding, ParaId, coretime, upload, deployment, release, or governance action | T-1115-E5, T-1116-E1, T-1116-E2 | Socket/action/log/fixture audit and forbidden-claim checks |
| Existing root regressions remain green; chain checks are separate, pinned, warnings-denied, and do not weaken root CI | T-1101-E2, T-1116-E3, T-1117-E2 | Byte guards, root/chain command exits, and separate workflow inspection |
| Root CI and terminal INT-0010/INT-0012/INT-0013 stay byte-identical; portable manual chain CI fetches once then runs offline under exact cache and cold/warm/disk/Zombienet budgets | T-1116-E3, T-1117-E1–E4 | Approved-snapshot SHA-256 guards, network trace, cache audit, and retained resource measurements |

## Primary EARS Verification

This table is exhaustive and one-to-one. Repository checks mechanically compare
its clause set to the Build plan and require 80 unique clause IDs and 80 unique
primary test names.

| EARS clause | Primary named verification |
|---|---|
| T-1101-E1 | `test_chain_dependency_toolchain_metadata_and_artifact_pins_are_exact` |
| T-1101-E2 | `test_nested_chain_workspace_has_only_allowlisted_root_delta` |
| T-1101-E3 | `test_pin_verifier_rejects_every_identity_mismatch` |
| T-1102-E1 | `test_reference_and_origin_bounds_are_exact` |
| T-1102-E2 | `test_core_chain_conformance_corpus_matches` |
| T-1102-E3 | `test_chain_types_are_no_std_bounded_and_dependency_clean` |
| T-1102-E4 | `test_scale_structural_rejections_never_enter_dispatch` |
| T-1103-E1 | `test_create_stores_revision_zero_and_one_complete_event` |
| T-1103-E2 | `test_decoded_lifecycle_rejections_are_typed_and_domain_atomic` |
| T-1103-E3 | `test_transition_and_completion_advance_once_and_preserve_identity` |
| T-1103-E4 | `test_stale_revision_precedes_lifecycle_domain_errors` |
| T-1103-E5 | `test_same_revision_extrinsics_accept_exactly_one` |
| T-1103-E6 | `test_lifecycle_and_global_sequence_boundaries_never_wrap` |
| T-1103-E7 | `test_root_allowlist_replacement_is_bounded_and_nondomain` |
| T-1104-E1 | `test_definition_and_edge_creation_preserve_endpoint_lifecycle` |
| T-1104-E2 | `test_definition_creation_precedence_is_typed_and_atomic` |
| T-1104-E3 | `test_edge_creation_precedence_bounds_and_cycles_are_exact` |
| T-1104-E4 | `test_opposite_cycle_closures_accept_at_most_one` |
| T-1104-E5 | `test_relationship_delete_is_exact_noncascading_and_ordered` |
| T-1105-E1 | `test_provenance_subjects_many_to_many_and_revision_exact` |
| T-1105-E2 | `test_provenance_record_precedence_is_typed_and_atomic` |
| T-1105-E3 | `test_provenance_revoke_is_ordered_append_only_and_nonreplacement` |
| T-1105-E4 | `test_runtime_event_surfaces_exclude_attribution_and_secrets` |
| T-1106-E1 | `test_runtime_genesis_and_authority_contract_are_exact` |
| T-1106-E2 | `test_runtime_artifacts_are_semantically_consistent` |
| T-1106-E3 | `test_generated_weights_cover_every_declared_maximum` |
| T-1106-E4 | `test_runtime_data_fee_and_fixed_code_policy` |
| T-1106-E5 | `test_runtime_has_no_origin_transform_or_root_route` |
| T-1106-E6 | `test_post_genesis_manifest_traces_every_field_source` |
| T-1106-E7 | `test_runtime_executive_separates_preinclusion_and_dispatch_failure_effects` |
| T-1107-E1 | `test_core_requires_and_preserves_immutable_origin` |
| T-1107-E2 | `test_root_consumers_reject_v1_before_removed_authority` |
| T-1107-E3 | `test_core_and_chain_share_256_record_capacity` |
| T-1108-E1 | `test_fresh_linux_schema_v3_is_exact_and_empty` |
| T-1108-E2 | `test_existing_projection_preflight_order_is_read_only_and_fail_closed` |
| T-1108-E3 | `test_projection_paths_sidecars_features_and_uri_surface_fail_closed` |
| T-1108-E4 | `test_envelope_replay_bounds_and_generation_rejection_are_exact` |
| T-1108-E5 | `test_sql_injection_shapes_are_bound_and_private_writers_stay_private` |
| T-1108-E6 | `test_projection_page_budget_and_busy_timeout_are_exact` |
| T-1109-E1 | `test_public_reads_are_uncallable_without_verified_snapshot` |
| T-1109-E2 | `test_private_snapshot_queries_are_ordered_bounded_and_fail_closed` |
| T-1109-E3 | `test_delete_snapshot_pins_c_blocks_c_plus_one_then_refreshes` |
| T-1109-E4 | `test_query_semantics_preserve_versions_and_many_to_many_identity` |
| T-1110-E1 | `test_rpc_archive_anchor_and_runtime_preflight_precedes_projection` |
| T-1110-E2 | `test_first_sync_bootstraps_anchor_block_zero_and_nullable_checkpoint` |
| T-1110-E3 | `test_finalized_block_projection_is_atomic_joined_and_ordered` |
| T-1110-E4 | `test_invalid_or_nonfinalized_stream_inputs_expose_no_progress` |
| T-1110-E5 | `test_full_rpc_stream_attestation_mints_one_pinned_read_or_nothing` |
| T-1110-E6 | `test_archive_refresh_restart_and_projector_contention_fail_honestly` |
| T-1111-E1 | `test_submission_lane_path_lock_and_first_use_are_hardened` |
| T-1111-E2 | `test_submission_journal_is_durable_before_send` |
| T-1111-E3 | `test_submission_crash_matrix_never_resends_unsafely` |
| T-1111-E4 | `test_finalized_submission_outcomes_match_exact_extrinsic_and_event` |
| T-1111-E5 | `test_unresolved_lane_scans_birth_through_death_without_sqlite_or_retry` |
| T-1111-E6 | `test_cross_process_signer_lanes_serialize_with_explicit_nonclaims` |
| T-1112-E1 | `test_stateless_v2_is_strict_origin_required_simulation` |
| T-1112-E2 | `test_stateless_protocol_preserves_ingestion_delivery_and_no_state` |
| T-1112-E3 | `test_stateless_schema_and_fixture_hashes_are_independent` |
| T-1113-E1 | `test_local_v2_rejects_invalid_shape_before_any_state_access` |
| T-1113-E2 | `test_local_v2_has_exactly_fifteen_operations_and_one_submission_path` |
| T-1113-E3 | `test_local_v2_outcomes_are_exact_and_signing_never_reads_sqlite` |
| T-1113-E4 | `test_local_process_arguments_and_serialized_surface_are_safe` |
| T-1113-E5 | `test_local_protocol_preserves_one_mib_and_delivery_contract` |
| T-1113-E6 | `test_local_schema_and_fixture_hashes_are_independent` |
| T-1114-E1 | `test_git_resolves_full_algorithm_specific_commit_without_network` |
| T-1114-E2 | `test_git_sha256_capability_gate_and_fixture_are_exact` |
| T-1114-E3 | `test_git_reference_survives_move_and_blame_change` |
| T-1114-E4 | `test_git_rejects_noncanonical_inputs_without_submission` |
| T-1115-E1 | `test_four_node_topology_ports_archive_flags_and_cleanup_are_exact` |
| T-1115-E2 | `test_both_submitters_and_collator_endpoints_converge` |
| T-1115-E3 | `test_restarted_collator_catches_up_and_passes_archive_probes_before_use` |
| T-1115-E4 | `test_dual_archive_rebuilds_equal_uninterrupted_projection` |
| T-1115-E5 | `test_local_journey_has_no_public_or_secret_action` |
| T-1116-E1 | `test_docs_state_one_authority_attestation_and_local_proof_model` |
| T-1116-E2 | `test_security_docs_preserve_exact_risks_and_nonclaims` |
| T-1116-E3 | `test_docs_have_no_legacy_authority_and_locked_inputs_are_byte_identical` |
| T-1117-E1 | `test_repo_local_validator_is_posix_portable_and_actionable` |
| T-1117-E2 | `test_chain_workflow_fetches_once_then_runs_locked_offline` |
| T-1117-E3 | `test_chain_cache_keys_and_hits_are_pin_verified` |
| T-1117-E4 | `test_exact_candidate_resource_budgets_are_met` |

## Test Fixtures and Oracles

- Pallet component tests use FRAME `TestExternalities` and byte-compare every
  CubiKan storage item plus the global domain sequence before and after rejected
  dispatch. Runtime Executive tests separately inspect nonce, balance/fee, and
  System failure effects; full-chain accounting is never folded into the pallet
  atomicity digest.
- The independently authored core/chain conformance corpus fixes inputs and
  expected values/errors for all text/collection minima and maxima, 256
  lifecycle records, definition/edge/provenance bounds, operation precedence,
  and replay. Neither implementation generates the other's oracle.
- Structural SCALE cases pass truncated, malformed, invalid-variant,
  missing-origin, and over-bound bytes through codec/transaction preflight and
  instrument pallet entry/domain reads. They must consume no nonce/fee and
  cannot return an in-domain pallet error.
- Every accepted event variant has a maximally encoded SCALE fixture and a
  mechanical `MaxEncodedLen <= 1_048_576` assertion. Event-only replay through
  an independent typed model must reproduce the accepted successor state.
- Storage tests run only on supported Linux in a unique owner-only local
  filesystem directory. Crate-internal `cfg(test)` helpers may write hostile or
  selected-corruption fixtures; production APIs expose neither raw
  row/block/event/checkpoint writers nor snapshot constructors. Production
  attestation must reject every such corruption before capability minting.
- Exact schema inspection covers eight rowid `STRICT` tables, literal column
  order/type/nullability/checks/PKs, restrictive FKs, eleven named indexes plus
  SQLite autoindexes, `wr=0`, `strict=1`, UTF-8, `user_version=3`, BINARY text
  identity/order, and absence of any extra table/index/view/trigger. Unsigned
  `u64` fields are eight-byte big-endian BLOBs.
- SQLite runtime checks pin the locally patched rusqlite 0.40.2 source reconstructed
  from registry archive checksum
  `23f2a97da3e3873c73cb2a2e71b35c40ff95e0b1eefa8d72d8499a6928c3b5b3`,
  with pristine-tree, patch, patched-tree, and normalized-diff hashes, and features
  `bundled`, `limits`, `modern_sqlite`, `hooks`, and `load_extension`;
  libsqlite3-sys 0.38.2, bundled
  SQLite 3.53.2, and the per-target compile-option manifest. Native
  `ENABLE_LOAD_EXTENSION` and `USE_URI` are accepted facts: each connection
  safely disables loading, installs the deny authorizer, never enables it,
  omits `SQLITE_OPEN_URI`, denies ATTACH/load_extension, sets and reads back
  `SQLITE_DBCONFIG_ENABLE_COMMENTS=false`, and treats a `file:`-shaped basename
  as a literal direct child. The private SQL inventory is independently
  comment-free.
- Existing-file preflight records an ordered syscall/authorizer trace: validated
  no-follow descriptor and rollback/4096/UTF-8/aligned header plus absent
  sidecars before SQLite open; `READ_ONLY|NOFOLLOW`; numeric limits; safe
  extension/comment disable; exact reader-role action/argument tuple authorizer;
  defensive/foreign-key/cell-size/query-only; mmap zero; `temp_store=MEMORY`;
  `busy_timeout=5000`; UTF-8/page-size/file-size/page-count; full
  `integrity_check` returning exactly one `ok`; empty `foreign_key_check`; then
  exact schema SQL. Only afterward may the private writer reopen with
  `query_only=OFF`, the exact projector tuple allowlist, all common defenses,
  its lock, set/read `max_page_count=262144`, and revalidate. Creator, projector,
  preflight, and public-reader traces are compared to their literal finite
  action/table/column/function/PRAGMA inventories; every other tuple denies.
  The public-reader trace alone admits `Pragma("data_version",None)` after common
  configuration; every other role/value/write form denies it.
- `tests/fixtures/sqlite-authorizer-v1.json` is independently authored before
  production statements and enumerates the complete raw-code plus patched
  rusqlite AuthAction tuple trace for every role/static statement. It covers
  patched BEGIN/COMMIT/ROLLBACK mapping; exact `sqlite_master` internal schema
  writes/reads; the eight table, eleven named-index, six autoindex, and eleven
  named-Reindex inventories; and exact `pragma_table_*`, `pragma_index_*`,
  `pragma_foreign_key_*`, and integrity virtual-table columns plus base Pragma
  tuples. It forbids wildcard/open-ended actions and implementation
  regeneration. `tests/fixtures/filesystem-boundary-v1.json` drives both
  independent backend and chain-client classifier implementations.
- `tests/fixtures/envelope-v2/manifest-v1.json` independently pins the literal
  ordered representation-version-2 envelope bytes/hashes, U64Text lifecycle
  history, escaping, maxima and malformed/duplicate/unknown/null rejection before
  serializer implementation; production output cannot refresh it.
- The SQLite limit oracle asserts length 4,194,304; SQL length 65,536; columns
  64; expression depth 64; compound selects 8; VDBE ops 100,000; function args
  32; attached 0; LIKE pattern 256; variables 128; trigger depth 0; workers 0;
  mmap 0; page size 4096; file size at most 1,073,741,824; page count at most
  262,144. The envelope independently remains at most 2,097,152 bytes.
- Projector streams have exact finalized block/extrinsic/System/global
  coordinates and include block zero, zero-event blocks before and after the
  first event, identical and conflicting duplicates, gaps/order/parent/anchor/
  runtime/code/version/count/sequence faults, displaced best-only blocks,
  malformed/overbound payloads, replay failure, source interruption, and two
  contending projectors.
- Attestation has two observable phases. It fetches and independently replays
  anchor through candidate C outside SQLite. It then opens one OS-read-only,
  query-only connection, starts and pins one transaction at C, verifies the
  complete blocks/events/joined hashes/derived rows/envelopes/checkpoint and
  executes exactly one read. The capability binds file identity, anchor, C,
  nullable sequence, and `PRAGMA data_version`; no process-generation field is
  invented.
- DELETE-mode scheduling is deterministic: a pre-pin candidate mismatch yields
  `RefreshRequired`; a pinned C page remains wholly C; a C+1 writer invokes only
  SQLite's configured `busy_timeout=5000` handler or returns typed Busy within a
  separate 7,500-ms outer test timeout; after drop, a new attestation may expose
  C+1. No network-duration lock or cross-page snapshot is
  claimed.
- Submission tests use real OS processes, one persistent no-follow mode-0600
  lock inode per derived signer lane, and the exact 256-byte
  `submission-journal-v1` layout locked in Build: `CUBKJNL1`, big-endian version
  and length, finite state tag, zero flags/reserved bytes, the ordered fixed-width
  deployment/signer/nonce/extrinsic/signing/era/resolution fields, the persisted
  original mutation-operation tag at byte 216, seven zero reserved bytes, and the
  domain-separated SHA-256 over bytes `0..224`. No trailing byte is accepted.
- Journal fault injection occurs before/after exclusive temp creation, partial
  and complete write, checksum, file `fsync`, atomic rename, directory `fsync`,
  send, watcher loss, response loss, resolution publication, and removal. On
  restart only the complete old or new checksummed record is admissible and the
  send counter must remain zero until safe reconciliation.
- Independently authored submission-journal-v1 fixtures pin exact canonical
  bytes and SHA-256 for prepared, `finalized_accepted`,
  `finalized_dispatch_rejected`, `finalized_invariant_failed`, and
  `expired_not_included` state tags, plus every allowed/forbidden state
  transition, unknown state/operation tag, nonzero-reserved,
  inconsistent-coordinate, truncated, bit-flipped, trailing/overlong, and
  wrong-version rejection. An unresolved operation A followed by an incoming
  operation B must retain A in every reconciliation response. Production serialization never generates or
  refreshes this oracle.
- Birth/death reconciliation enumerates every finalized block and exact
  extrinsic hash in the inclusive range. `expired_not_included` is permitted
  only after finalized head passes death and the complete scan proves absence;
  mere era expiry, nonce movement, or any SQLite value proves nothing.
- The two raw protocol corpora are independently authored before decoder/result
  code. Each Draft-2020-12 schema and manifest has separately pinned SHA-256
  bytes; every request/stdout case includes exact byte length/hash/exit and LF.
  A read-only independent verifier covers every operation/result/outcome/code,
  omission-vs-null, unknown/duplicate raw keys, scalar/cursor/coordinate bounds,
  and request sizes 1,048,575/1,048,576/1,048,577. Implementation output cannot
  update schemas, fixtures, manifests, or hashes.
- The `cubikan-local` operation completeness oracle lists exactly:
  `create_intent_unit`, `get_intent_unit`, `list_intent_units`,
  `transition_intent_unit`, `complete_intent_unit`,
  `create_relationship_definition`, `get_relationship_definition`,
  `create_relationship`, `delete_relationship`, `list_relationships`,
  `project_intent_units_v1`, `record_association`, `revoke_association`,
  `list_associations_by_unit`, and `list_associations_by_reference`.
- The protocol registry locks canonical member order/escaping, Uuid/hash/u64/
  cursor/coordinate encodings, every literal nested projected row/effect object,
  exact stateless success/error unions, the exact local error/read-result and seven
  mutation-outcome unions, the full finite error-code registry, and exit codes
  0/1/2/3/4.
  `MutationOperation` is one exact mutation tag, `MortalEra` is exactly
  `{birth:U64Text,death:U64Text}` with `death=birth+63`, `ErrorDetail.field` is
  a distinct NUL-free 0..=256-byte RFC-6901 `JsonPointer256` only for the exact
  finite code allowlist; empty string means document root and `~`/`/` tokens use
  `~0`/`~1` escaping. Raw fixtures require non-object top-level input to produce
  `invalid_request` with `field:""` and cover pointer escapes. `operation_number` is a zero-based JSON integer from 0 through
  255 only for the five named stateless operation failures. Raw parser fixtures—not JSON
  Schema—are the duplicate-key oracle. Omitted-ID cases inject one
  manifest-pinned UUID through a private test-only ID source; local cases also
  hash independent RPC/signer/projection transcripts so nonce, signature,
  coordinate, and stdout bytes remain deterministic without weakening
  production UUID-v4 randomness or live chain inputs.
- A mutation call-order spy instruments database open/read, manifest read, RPC,
  signer, and send. Signing inputs may come only from the decoded request, fixed
  pin-verified manifest, and canonical RPC. Any forged SQLite file causes zero
  SQLite-derived preflight choices and zero signer/send calls.
- Git tests require Git at least 2.45 and capability-check global
  `--no-lazy-fetch`. SHA-1 always uses a real temporary
  repository. SHA-256 uses a real repository when capability probing succeeds;
  otherwise the live case is explicitly skipped while an independently checked
  fixture still proves namespace/format/length.
- Zombienet evidence records exactly four node processes plus the separate
  orchestrator; test-facing RPC 9944/9945/9988/9989; primary P2P 30333–30336;
  primary metrics 9615–9618; collator relay-side RPC 9990/9991, P2P 30337/30338,
  and metrics 9619/9620. The SHA-pinned argv normalizer fails closed on any
  generated-command grammar/hash drift, strips external-bind flags, and forces
  every listener/bootnode to loopback. `/proc/<pid>/cmdline` and `ss -lntup`
  must match the exact PID-owned inventory with no wildcard/public bind. Both
  collator archive flags, one pinned relay runtime/spec identity plus one
  distinct pinned CubiKan parachain runtime/manifest byte-equal across
  collators, two submitters distinct from node roles, a bounded timeout, and
  deterministic cleanup are retained.
  Hash/header/body/events/`:code` probes cover genesis, early, middle, and final
  heights.
- Rebuild equality is semantic: units, immutable origins, complete histories,
  definition/edge/query pages, provenance event history/active pages, joined
  ledger coordinates, runtime identity, and checkpoint. SQLite rowids, pages,
  freelists, file sizes, and timing are not equality oracles.

### Rejection precedence

- Structural codec/preflight rejects malformed/missing/invalid-variant/
  over-bound bytes before dispatch or domain reads.
- Lifecycle dispatch order is command version; direct signed allowlisted
  origin; target selection; stale revision; history capacity; remaining domain
  validity; global sequence immediately before mutation/event.
- Definition create order is command version; direct signed allowlisted origin;
  exact duplicate; global sequence. Malformed or over-bound definition fields
  reject structurally before dispatch under T-1102-E4.
- Edge create order is command version; origin; definition; source; target;
  source species; target species; self; duplicate; 128-edge capacity; cycle;
  global sequence. Edge delete has the same prefix through target species, then
  exact active edge and global sequence, with no capacity check.
- Association record order is command version; origin; unit and whole/exact
  revision; reference; duplicate; 128-active capacity; global sequence. Revoke
  checks version; origin; unit/subject/reference; exact active association;
  global sequence, with no capacity check.
- Projector order is RPC/anchor/archive/runtime preflight; block continuity;
  coordinate/count/version/sequence/payload verification; independent replay;
  then one atomic block/derived-state/checkpoint write.

## Unit Tests

### T-1101 — pinned isolated foundation

- `test_chain_dependency_toolchain_metadata_and_artifact_pins_are_exact`
  [T-1101-E1]: parse pins, tools, locks, checksums, and fetched assets; require
  SDK `8ae9775…`, Rust 1.93.0 plus rustfmt/clippy/`wasm32v1-none`, Subxt 0.50.2,
  Zombienet `a7c4342…`, exact argv-normalizer and loopback-netns bytes, accepted
  generated-command grammar hash, benchmark-capable node/omni-node command,
  relay/collator/chain-spec/util-linux/iproute2/Node/npm/scaffold identities,
  the exact rusqlite registry archive/pristine-tree/patch/patched-tree/normalized-
  diff hashes and byte-identical reconstructed vendor tree, no floating or 2512
  dependency, and only loopback after the explicit fetch.
- `test_nested_chain_workspace_has_only_allowlisted_root_delta` [T-1101-E2]:
  compare foundation and final root graphs/locks; T-1101 changes only the root
  manifest `chain/` exclusion plus the exact local rusqlite patch override/vendor
  source and corresponding lock-source identity, while final root
  declarations use exact `=VERSION`, `default-features=false`, and only the
  listed requested arrays; effective features on the nine planned root packages
  must equal the resolved closure implied by those requested arrays (and no
  more), while the complete resolved `cargo tree -e features --locked` closure
  (including necessary upstream defaults/implied features)
  must match its independently recorded T-1101 SHA-256 pin. Every lockfile
  package named `subxt` or `subxt-*`, including the
  unselected optional `subxt-lightclient` plus signer, macro, codegen, metadata,
  RPCs, accountid32 and fetchmetadata utilities, must resolve to exactly 0.50.2;
  mutating any entry to 0.50.3 rejects before build. Require
  isolated locks/native/Wasm builds and reject any other direct dependency or
  SDK/FRAME/Cumulus/node/runtime source in root.
- `test_pin_verifier_rejects_every_identity_mismatch` [T-1101-E3]: mutate each
  asset checksum/source/tool/generated-artifact identity, registry archive,
  pristine/vendor byte, patch/diff hash, and lock-source identity independently
  and prove failure precedes dependent execution/build.

### T-1102 — bounded values and independent conformance

- `test_reference_and_origin_bounds_are_exact` [T-1102-E1]: table empty,
  exact-min/max/max+1, NUL, Unicode blank, invalid UTF-8/namespace grammar,
  byte-index/length, and no-normalization cases across every value/collection.
- `test_core_chain_conformance_corpus_matches` [T-1102-E2]: run one independent
  fixture corpus through core and chain values; require identical shared
  lifecycle/relationship/provenance meaning while authorization/capacity errors
  remain chain-specific.
- `test_chain_types_are_no_std_bounded_and_dependency_clean` [T-1102-E3]: Wasm
  build and metadata/type inspection prove bounded SCALE, finite encoded length,
  maximal event fixtures, and absence of unbounded String/Vec, std, UUID
  generation, filesystem, clock, RPC, account-key, or provider dependencies.
- `test_scale_structural_rejections_never_enter_dispatch` [T-1102-E4]: use
  pallet-entry/domain-read counters for truncated/malformed/missing-origin/
  invalid-variant/over-bound SCALE; require zero dispatch/read/mutation/event.

### T-1103 — lifecycle pallet

- `test_create_stores_revision_zero_and_one_complete_event` [T-1103-E1]: valid
  maximum create stores exact ID/origin/species/workflow/state/revision zero,
  increments global sequence once, emits one replay-complete event, and uses no
  randomness or clock.
- `test_decoded_lifecycle_rejections_are_typed_and_domain_atomic` [T-1103-E2]:
  table unsupported command version, origin authorization, duplicate, and every
  decoded semantic failure; assert precedence, byte-equal domain storage/global
  sequence, and no accepted event.
- `test_transition_and_completion_advance_once_and_preserve_identity`
  [T-1103-E3]: compare core/pallet successors for every declared transition and
  eligible completion; require one record/revision/global/event and immutable
  ID/origin/species/workflow.
- `test_stale_revision_precedes_lifecycle_domain_errors` [T-1103-E4]: pair
  stale and current expectations with terminal/unknown-target/undeclared-edge/
  completion-ineligible cases and prove exact order plus atomicity.
- `test_same_revision_extrinsics_accept_exactly_one` [T-1103-E5]: execute two
  authorized same-revision extrinsics in both orders; one succeeds, one is
  stale, successor state is singular, and neither signer becomes owner/author.
- `test_lifecycle_and_global_sequence_boundaries_never_wrap` [T-1103-E6]:
  record 256 succeeds, 257 rejects capacity before domain; sequence MAX-1→MAX
  succeeds, next otherwise-valid rejects exhaustion; combined exhaustion returns
  history capacity first and never mutates/wraps/emits.
- `test_root_allowlist_replacement_is_bounded_and_nondomain` [T-1103-E7]: mock
  Root replaces 0..16 unique accounts with one admin event; signed, duplicate,
  and 17-entry calls preserve allowlist/global domain sequence.

### T-1104 — relationship pallet

- `test_definition_and_edge_creation_preserve_endpoint_lifecycle` [T-1104-E1]:
  create nonsequential definition versions and valid directed edges; assert
  immutable fields/endpoints, one global/event increment, and byte-equal endpoint
  aggregates.
- `test_definition_creation_precedence_is_typed_and_atomic` [T-1104-E2]: pair
  version, origin, duplicate, and global-sequence failures; prove exact order/
  type and unchanged definition/sequence/events, while malformed/over-bound
  definition fields remain structural T-1102-E4 cases.
- `test_edge_creation_precedence_bounds_and_cycles_are_exact` [T-1104-E3]:
  pair every ordered version/origin/definition/source/target/species/self/
  duplicate/capacity/cycle/global failure, accept boundary 128, reject 129 before
  cycle, and require finite measured traversal with no mutation/event on reject.
- `test_opposite_cycle_closures_accept_at_most_one` [T-1104-E4]: execute both
  canonical orders and independently traverse the result; at most one accepted
  edge and no forbidden cycle.
- `test_relationship_delete_is_exact_noncascading_and_ordered` [T-1104-E5]:
  exercise ordered version/origin/definition/endpoints/species/active-edge/global
  failures and success; success emits one delete, preserves endpoints/definition/
  neighbors, and correction requires a distinct later create.

### T-1105 — provenance pallet

- `test_provenance_subjects_many_to_many_and_revision_exact` [T-1105-E1]:
  record whole, revision zero, interior, and current subjects across many-to-many
  references; require distinct complete identities, one event/global increment,
  and unchanged lifecycle revision/history.
- `test_provenance_record_precedence_is_typed_and_atomic` [T-1105-E2]: pair
  version/origin/unit/subject/reference/duplicate/capacity/global failures and
  assert first exact error with unchanged lifecycle/provenance/sequence/events.
- `test_provenance_revoke_is_ordered_append_only_and_nonreplacement`
  [T-1105-E3]: pair version/origin/unit/subject/reference/active/global failures;
  on success remove only active membership, retain record+revoke events, expose
  intermediate absence, reject repeated revoke, and require a later record.
- `test_runtime_event_surfaces_exclude_attribution_and_secrets` [T-1105-E4]:
  inventory storage/calls/events/metadata/fixtures; permit bounded domain data and
  technical signer only and reject attribution, secret, source, provider, or
  production fields.

### T-1106 — fixed runtime and artifact contract

- `test_runtime_genesis_and_authority_contract_are_exact` [T-1106-E1]: inspect
  ParaId 1000, one deployment, versions/code, two funded submitters distinct
  from node roles, balances/payment, and no self-hash/upgrade/signed allowlist.
- `test_runtime_artifacts_are_semantically_consistent` [T-1106-E2]: checksum
  Wasm and compare canonical hashes/decoded metadata, chain spec, native runtime,
  genesis/deployment/version/code semantics; reject mismatches before launch
  without comparing heterogeneous raw bytes.
- `test_generated_weights_cover_every_declared_maximum` [T-1106-E3]: validate
  generated benchmark weights at all maxima and reject zero placeholders,
  unbounded paths, or unmeasured worst-case reads/writes/traversal.
- `test_runtime_data_fee_and_fixed_code_policy` [T-1106-E4]: execute dev calls
  with normal weight/length fee and zero tip, audit synthetic-only data, and
  prove one unchanged runtime code hash throughout.
- `test_runtime_has_no_origin_transform_or_root_route` [T-1106-E5]: enumerate
  calls/origins; allow necessary direct Root-only System/Balances/CubiKan
  administration calls only while proving no runtime origin can produce Root;
  reject every sudo/proxy/utility/multisig/governance/dispatch-as/batch/
  derivative or reachable-Root/origin-transform route.
- `test_post_genesis_manifest_traces_every_field_source` [T-1106-E6]: prove
  relay/parachain genesis from strict RPC, fields/versions from decoded state,
  code hash from block-zero `:code`, exact canonical bytes/SHA pin, and consumer
  rejection for every field/provenance mismatch.
- `test_runtime_executive_separates_preinclusion_and_dispatch_failure_effects`
  [T-1106-E7]: compare malformed/invalid pre-inclusion nonce+fee invariance with
  included typed dispatch failures that consume nonce/fee and emit System
  failure while CubiKan storage/global/events stay equal.

### T-1107 — required-origin core rebaseline

- `test_core_requires_and_preserves_immutable_origin` [T-1107-E1]: compile-time
  API inventory plus construction/serialization/restoration/lifecycle round trip
  proves exactly one origin and no originless/null/legacy/placeholder/correction.
- `test_root_consumers_reject_v1_before_removed_authority` [T-1107-E2]: compile
  every root consumer, exercise legacy entry points, and require typed
  unsupported before removed write/RPC/database/synthetic attribution while the
  complete root regression suite stays green.
- `test_core_and_chain_share_256_record_capacity` [T-1107-E3]: run revisions
  0..256 through shared fixtures and require exact history/revision equality and
  record-257 typed capacity rejection with unchanged state.

## Integration Tests

### T-1108 — exact hardened SQLite v3/envelope v2

- `test_fresh_linux_schema_v3_is_exact_and_empty` [T-1108-E1]: exclusively
  classify the canonical directory by longest `/proc/self/mountinfo` entry and
  matching `statfs` magic, select only the registered built-in `unix` VFS, then
  use rustix `O_CREAT|O_EXCL|O_NOFOLLOW|O_CLOEXEC` mode 0600, validate the
  returned regular-file identity, and reject a preexisting target/race before
  SQLite; then open that existing inode only through built-in-unix
  `READ_WRITE|NOFOLLOW` without SQLite CREATE/EXCLUSIVE on a declared allowed
  Linux local filesystem, set UTF-8 before schema, pin DELETE journaling and
  synchronous EXTRA, commit only exact v3 objects/settings, and assert zero
  domain/anchor/block/checkpoint rows and no capability. Table-driven injected
  observations and the independently authored shared classifier corpus cover
  accepted ext2/ext3/ext4, XFS, and Btrfs identities/magics
  and reject malformed/mismatched/missing, overlay, network, 9p/DrvFS, FUSE,
  tmpfs, unknown, or custom-VFS identities before access; one real supported local
  directory exercises the actual classifier. Unsupported cases leave no file.
  With the patched authorizer installed before schema SQL, execute all eight
  CREATE TABLE and eleven CREATE INDEX statements and compare every raw-code and
  mapped tuple to the independent fixture: exact `sqlite_master` Insert/Update/
  ROWID Read, exact application key-column Reads, six autoindexes, and Reindex
  only for the eleven named indexes. Raw BEGIN/COMMIT/ROLLBACK must map to the
  patched three variants; RELEASE, unknown transaction spellings, unknown
  schema objects, and one-byte vendor/patch mutations deny before commit.
- `test_existing_projection_preflight_order_is_read_only_and_fail_closed`
  [T-1108-E2]: assert no-follow descriptor header validation and absent sidecars
  precede SQLite open, then the exact pre-schema reader-role configuration/
  readback order and full integrity=`ok`/empty-FK/exact-schema checks; fixtures
  include short/misaligned/WAL/unknown header, wrong UTF-8/page size/count/file
  size, and corrupt/extra/legacy objects. Bytes and directory stay unchanged;
  private writer reopen occurs only after success with query-only off, exact
  projector tuple allowlist, common defenses, and lock-time revalidation. Run
  that complete revalidation with the writer authorizer installed and require
  its statement-scoped `sqlite_master` plus eight PRAGMA virtual-table traces to
  equal the independent oracle; mutating any table, column, database, accessor,
  PRAGMA name/value, or using those tuples from projector DML must deny.
- `test_projection_paths_sidecars_features_and_uri_surface_fail_closed`
  [T-1108-E3]: table directory/target/sidecar owner/mode/type/symlink/nonregular,
  stable-parent/direct-child, pre-open rollback/WAL header and no-sidecar trace,
  version/feature/compile/runtime/role settings, DQS/
  trusted/writable schema/ATTACH/extensions/URI cases. Require no follow/recovery/
  adoption/external creation; `file:` basename remains a literal child.
- `test_envelope_replay_bounds_and_generation_rejection_are_exact`
  [T-1108-E4]: round-trip maximal envelope with immutable origin and all 256
  records through the independent literal closed-object fixture/manifest; verify
  exact representation version, member order, scalar/history encodings, UTF-8/
  escaping/no-whitespace bytes, and duplicate/unknown/null rejection; replay
  core/relationship/provenance/event coordinates, prove formula plus adversarial
  bytes <=2,097,152, and reject v1/v2 schema, v1/mixed envelope, edits/
  disagreement/over-ceiling without migration/synthesis/repair.
- `test_sql_injection_shapes_are_bound_and_private_writers_stay_private`
  [T-1108-E5]: pass quote/comment/semicolon/PRAGMA/ATTACH/load_extension-shaped
  valid scope/value only through crate-private fixture writers and require exact
  round trip with unchanged schema/settings; constrained invalid text rejects
  before SQL, all production SQL is private/static/comment-free, every
  connection reads back `SQLITE_DBCONFIG_ENABLE_COMMENTS=false`, independently
  verify the exact no-wildcard `sqlite-authorizer-v1` raw/mapped role/statement
  tuple manifest. Schema inspection must use actual SQLite-3.53.2 callbacks:
  `sqlite_master` columns; literal `pragma_table_list|table_info|index_list|
  index_info|index_xinfo|foreign_key_list|integrity_check|foreign_key_check`
  virtual-table columns; and their matching base `Pragma` tuples—not Function
  aliases. Prove `data_version` is public-reader-only, all configuration SQL
  occurs after authorizer installation, `count(*)`/`Function("count")`/the
  empty-column NULL-database Read deny while bounded fetched rows are counted in
  Rust, all other tuples deny, and production
  cannot refresh the oracle; public API compile tests find no raw write/
  capability seam.
- `test_projection_page_budget_and_busy_timeout_are_exact` [T-1108-E6]: after
  close/reopen prove writer reapplies and reads max_page_count 262,144, accepts
  final in-budget commit, returns typed SQLite-full with full rollback beyond it,
  and uses exactly the 5,000-ms SQLite handler with no application retry.

### T-1109 — capability-gated queries

Positive E2–E4 query tests live in backend-module `cfg(test)` modules so they
can exercise the private fixture issuer. The external `read_boundary` test is
negative-only and inspects public signatures/no-raw-open behavior. Built-in
rustdoc `compile_fail` snippets supply the construction/bypass compile oracle;
the plan adds no third-party compile-test harness.

- `test_public_reads_are_uncallable_without_verified_snapshot` [T-1109-E1]:
  built-in rustdoc `compile_fail` snippets prove callers cannot construct or
  bypass the capability; external `read_boundary` tests inspect public
  signatures and no-raw-open behavior. No public unit/relationship/projection/
  provenance read accepts a path, connection, coherent DB, raw rows, or caller
  capability; structural coherence alone exposes nothing.
- `test_private_snapshot_queries_are_ordered_bounded_and_fail_closed`
  [T-1109-E2]: a backend-module `cfg(test)` issuer creates the otherwise-private
  snapshot solely for query semantics. Table all read kinds, limits 0/1/100/101,
  binary complete-key order, exclusive cursor/lookahead, joined block hash/C,
  and selected corruption; the issuer is absent from production artifacts.
- `test_delete_snapshot_pins_c_blocks_c_plus_one_then_refreshes` [T-1109-E3]:
  pre-pin mismatch returns RefreshRequired; pinned C page finishes entirely C;
  C+1 writer invokes only configured `busy_timeout=5000` and returns Busy within
  the separate 7,500-ms outer test timeout; after drop a newly issued snapshot
  sees C+1. Assert no cross-page/network-duration lock.
- `test_query_semantics_preserve_versions_and_many_to_many_identity`
  [T-1109-E4]: one unit spans exact relationship versions, projection predicates,
  and whole/revision forward+reverse associations; require exact INT-0012/
  INT-0008 filters/identity without copied lifecycle state.

### T-1110 — finalized projection and full-stream attestation

- `test_rpc_archive_anchor_and_runtime_preflight_precedes_projection`
  [T-1110-E1]: strict literal-loopback-`ws` table plus fixed manifest provenance,
  metadata/spec/code, exact archive flags, genesis/early/mid/current hash/header/
  body/events/`:code` probes, and parent continuity all precede decode/apply;
  failures retain source and claim neither perpetual archive nor independent
  finality.
- `test_first_sync_bootstraps_anchor_block_zero_and_nullable_checkpoint`
  [T-1110-E2]: inject failure at every first-sync statement/commit; only all or
  none of manifest anchor, zero-event block-zero/null row sequences, and null
  checkpoint appears. After a nonzero event, a later zero-event row stays null
  while checkpoint retains prior sequence.
- `test_finalized_block_projection_is_atomic_joined_and_ordered` [T-1110-E3]:
  apply contiguous finalized blocks in block/extrinsic/System order, payload <=1
  MiB, enforce joined block hash, derived rows/checkpoint and once-only equality;
  statement/commit/space/limit faults roll back the entire block while chain
  state is unaffected.
- `test_invalid_or_nonfinalized_stream_inputs_expose_no_progress` [T-1110-E4]:
  table best-only/displaced, identical/conflicting duplicate, gaps/order, wrong
  parent/anchor/runtime/version/count/sequence, malformed/overbound/replay faults;
  only completely equal replay is no-op and every other case exposes no row,
  checkpoint, or capability.
- `test_full_rpc_stream_attestation_mints_one_pinned_read_or_nothing`
  [T-1110-E5]: fetch/replay full anchor..C outside SQLite, pin transaction C,
  compare every block/raw event/coordinate/join/derived row/envelope/checkpoint,
  then mint one single-read capability. Independently forge raw events, derived
  rows, and coherent event+derived state with genuine anchor/checkpoint; all
  reject pre-mint.
- `test_archive_refresh_restart_and_projector_contention_fail_honestly`
  [T-1110-E6]: fail archive probes, move checkpoint before pin, restart at named
  points, and barrier-start two projectors; require bounded source-preserving
  archive/RefreshRequired/Busy or serialization, no public raw seam, double
  apply, heuristic rollback, GRANDPA/light-client, or perpetual-history claim.

### T-1111 — crash-recoverable finalized submission

- `test_submission_lane_path_lock_and_first_use_are_hardened` [T-1111-E1]: on
  supported Linux derive the three literal basenames from the exact domain-
  separated, length-prefixed canonical-path/deployment/signer digest and test
  asymmetric vectors/traversal/overflow cases; validate filesystem/owner/mode/type, acquire persistent
  mode-0600 no-follow lock inode whose identity is never replaced or unlinked,
  accept absent journal as first use, discard only one owner-mode-0600 regular
  derived temp under lock followed by directory fsync, and reject symbolic/
  nonregular/wrong-owner/wrong-mode/oversized temp, caller path, non-Linux,
  unsupported/corrupt/insecure state before sign/send.
- `test_submission_journal_is_durable_before_send` [T-1111-E2]: RPC supplies
  deployment and the chosen finalized signing block; decode nonce only from
  `System::Account` at that same hash, fixture divergent best-head and pending-
  pool next-index suggestions, and assert neither is used. Assert exact 64-block zero-tip
  Subxt offline signable params: compare `signer_payload()` bytes against an
  independent expected payload containing the pinned metadata/call/nonce/mortal
  params/finalized hash, sign and verify it; then decode a non-64-aligned signed
  extrinsic to require encoded signer/call/nonce/period/phase/zero tip and journal
  birth=`n`/death=`n+63` agree. Do not claim the additional block hash is encoded
  in the extrinsic; prove it through payload equality and signature verification.
  Assert byte-equality with the independent exact 256-byte v1 checksummed
  prepared-record fixture, including every byte offset, big-endian integer,
  zero reserved/coordinate byte, persisted original-operation tag, and
  checksum-domain byte, then O_EXCL|NOFOLLOW
  temp→complete write/checksum→file fsync→rename→directory fsync before the
  first send while lock remains held.
- `test_submission_crash_matrix_never_resends_unsafely` [T-1111-E3]: kill/fault
  after temp creation and at every write/fsync/rename/directory-fsync/send/
  resolution boundary; restart accepts only old/new complete journal, safely
  removes at most the one derived torn/complete temp, proves no orphan
  accumulation or unsafe send, and retains the declared Linux process-crash
  contract with lying-hardware power loss explicitly excluded.
- `test_finalized_submission_outcomes_match_exact_extrinsic_and_event`
  [T-1111-E4]: within 120 seconds distinguish send-counter-zero preparation/
  validation rejection, finalized dispatch failure, finalized acceptance, and
  finalized invariant failure; acceptance requires exact prepared hash+index,
  successful dispatch, and exactly one matching event by deployment/version/
  signer/call. Zero, duplicate, or wrong CubiKan events after successful exact
  inclusion durably resolve as invariant failure with no retry. Every durable
  resolution/removal directory-fsyncs.
- `test_unresolved_lane_scans_birth_through_death_without_sqlite_or_retry`
  [T-1111-E5]: after `submit_and_watch`, timeout/crash/transport/RPC/response/
  nonce ambiguity plus watcher Invalid/Dropped/Error/stream-end/unknown leaves prepared
  and returns indeterminate; no watcher terminal status clears the lane;
  restart sends/signs nothing until exact hash finalizes or head passes death and
  every finalized birth..death block proves absence. Before then return
  unresolved; expiry and SQLite never clear/select nonce/sign/retry. Crash after
  terminal-record fsync but before stdout must re-fetch the stored finalized
  block, uniquely reconstruct the identical persisted operation/outcome and
  chain-derived dispatch/event/effect/coordinates without resend, and retain the
  record on unavailable, duplicate, or mismatched evidence. For expired state,
  validate the stored first post-death head and re-scan inclusive birth..death
  for exact-hash absence instead of trying to find an inclusion in that head.
  Projection is
  freshly attested and may validly advance, so only stable response members—not
  complete stdout bytes—must match; a crash after stdout but before removal may
  duplicate a semantic response, never a submission.
- `test_cross_process_signer_lanes_serialize_with_explicit_nonclaims`
  [T-1111-E6]: real cooperating processes serialize the same signer while
  separate lanes overlap; external nonce disagreement fails closed. Document and
  test boundary markers for alternate projection/external user and undetectable
  same-user unresolved-record deletion—never exactly-once delivery.

### T-1112 and T-1113 — adapter-owned protocol v2

- `test_stateless_v2_is_strict_origin_required_simulation` [T-1112-E1]: enforce
  exact stateless top-level/nested/scalar/operation inventory, required origin,
  omitted-ID generation and null rejection; outputs carry only
  `authority:"simulation_only"` UnitView and no canonical vocabulary.
- `test_stateless_protocol_preserves_ingestion_delivery_and_no_state`
  [T-1112-E2]: table old/v1/malformed and below/at/over 1 MiB plus body/LF/flush
  failure; instrument retained raw request-buffer bytes at <=1,048,577, require
  one response, source-retaining I/O, exact stderr/exit, and no claim that total
  process memory fits that buffer ceiling, RPC/database/session, or canonical success.
- `test_stateless_schema_and_fixture_hashes_are_independent` [T-1112-E3]: run
  independent verifier before and after implementation against exact schema/
  manifest bytes; require all stateless operations/results/codes/bounds/raw
  duplicate cases, use the manifest-pinned UUID for omitted-ID output, and
  reject adapter-generated oracle drift or production-selectable test injection.
- `test_local_v2_rejects_invalid_shape_before_any_state_access` [T-1113-E1]:
  table exact top/operation/scalars/cursors, duplicate/unknown/wrong/null/bad
  origin/revision/coordinate/v1; spies require zero signer, dial, SQLite open, or
  capability issuance.
- `test_local_v2_has_exactly_fifteen_operations_and_one_submission_path`
  [T-1113-E2]: compare public schema/API/fixtures to the fifteen-name registry,
  exact fields and non-caller command version 1; every mutation routes through
  T-1111 and no raw SQLite/RPC/capability seam exists.
- `test_local_v2_outcomes_are_exact_and_signing_never_reads_sqlite`
  [T-1113-E3]: fixture the exact local `outcome:"error"` envelope for all
  request/read/attestation failures and misses with no nullable/partial success,
  plus all seven mutation outcomes including durable finalized invariant failure
  and zero-send `expired_not_included`,
  exact nested projected/effect members, ErrorDetail legality, canonical member
  order/escaping, joined-hash coordinate and caught-up/lagging projection.
  Call-order instrumentation proves decoded
  request+fixed manifest+RPC are the only signing inputs and no cache read,
  false success, rollback wording, or retry occurs.
- `test_local_process_arguments_and_serialized_surface_are_safe` [T-1113-E4]:
  mutations require database, strict RPC, named dev signer and derive journal;
  reads need no signer/journal. Reject raw seed/key, user journal path, secret,
  owner/author, and source/provider content on serialized surfaces. Table raw
  canonical lowercase `ws://` loopback forms against scheme/host case,
  leading-zero/short/integer/octal/hex IPv4, percent encoding, hostname/userinfo/
  query/fragment/redirect, omitted path, missing/leading-zero/default-80/out-of-
  range port, and public-address spellings; raw
  and parsed host/port/path must round-trip before dial.
- `test_local_protocol_preserves_one_mib_and_delivery_contract` [T-1113-E5]:
  table 1,048,575/1,048,576/1,048,577 request bytes and body/LF/flush failure for
  representative read/mutation responses; require exact exit and at most
  1,048,577 retained raw request-buffer bytes, make no total-process-memory claim,
  and rely on the external resource gate for process RSS/disk.
- `test_local_schema_and_fixture_hashes_are_independent` [T-1113-E6]: verify
  separate local schema/manifest hashes before/after implementation and exact
  coverage of all fifteen operations, result/outcome/error registries, optional/
  cursor/coordinate encodings, raw duplicates and size boundaries. Omitted-ID
  and stateful cases consume only independently hashed UUID/RPC/signer/
  projection context from the private fixture harness; implementation output
  cannot refresh the oracle and production input cannot select fixture context.

### T-1114 — Git adapter

- `test_git_resolves_full_algorithm_specific_commit_without_network`
  [T-1114-E1]: require Git >=2.45, capability-check global `--no-lazy-fetch`,
  canonicalize the repo, clear every Git
  directory/worktree/object/alternate/config/replace/proxy/credential variable,
  reject nonempty common-dir/worktree `objects/info/alternates`, set literal
  `GIT_TERMINAL_PROMPT=0`, `GIT_CONFIG_NOSYSTEM=1`,
  `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_NO_REPLACE_OBJECTS=1`,
  `GIT_NO_LAZY_FETCH=1`, and `GIT_OPTIONAL_LOCKS=0`, use argv-only
  `git --no-lazy-fetch -C <repo> rev-parse --verify --end-of-options
  <revision>^{commit}`, bound stdout/stderr,
  and resolve full lowercase OID and exact namespace/scope/value without shell/
  fetch/credential/normalization; incompatible Git fails before mutation/send.
- `test_git_sha256_capability_gate_and_fixture_are_exact` [T-1114-E2]: always
  run real SHA-1; run real SHA-256 when supported, otherwise record an explicit
  live skip and still validate the independent SHA-256 repository fixture.
- `test_git_reference_survives_move_and_blame_change` [T-1114-E3]: finalize an
  association, rename/edit source and change blame, rebuild, and require byte-
  identical association with no blame/committer/signer attribution promotion.
- `test_git_rejects_noncanonical_inputs_without_submission` [T-1114-E4]: table
  abbreviated/NUL/leading-dash/noncommit/wrong-format/outside-repository/
  unsupported inputs plus hostile inherited Git config/directory/alternate/
  replace/proxy/credential/promisor state, on-disk alternates, and fake remote/
  credential helpers; require typed noncanonical
  failure with zero helper/network/submission/projection mutation.

## End-to-End Tests

- **Status:** possible; T-1115 is required and must run unskipped on the exact
  candidate within the T-1117 resource gate.

### T-1115 — four-node failover, resynchronization, and rebuild

- `test_four_node_topology_ports_archive_flags_and_cleanup_are_exact`
  [T-1115-E1]: launch exactly 2+2 node processes plus the separately inventoried
  orchestrator with RPC 9944/9945/9988/9989, primary P2P 30333–30336, primary
  metrics 9615–9618, and relay-side RPC 9990/9991, P2P 30337/30338, metrics
  9619/9620. Mutate the expected generated argv grammar/hash and every external/
  duplicate/missing/unknown flag; require the pinned normalizer to abort before
  launch. On the valid case inspect PID cmdlines and bound addresses, require
  loopback-only exact listeners/bootnodes, both archive flags, byte-identical
  CubiKan Wasm across collators, byte-identical relay runtime/spec across relay
  validators and collator relay sides, deliberate inequality between relay and
  parachain runtime identities, local owner-only data, external timeout, and cleanup.
- `test_both_submitters_and_collator_endpoints_converge` [T-1115-E2]: Charlie
  and Dave execute required-origin lifecycle/relationship/provenance/Git work;
  each finalizes once and both collators plus fresh attested reads converge.
- `test_restarted_collator_catches_up_and_passes_archive_probes_before_use`
  [T-1115-E3]: name C, stop B, finish through A, name F, restart B with original
  data/config, and prohibit B as source until F hash/code/metadata/checkpoint and
  complete historical hash/header/body/events/`:code` probes match A.
- `test_dual_archive_rebuilds_equal_uninterrupted_projection` [T-1115-E4]:
  after catch-up rebuild fresh projections independently through A and B, full-
  stream attest them, and compare all semantic objects/pages/history/origin/
  coordinates/checkpoint with uninterrupted projection.
- `test_local_journey_has_no_public_or_secret_action` [T-1115-E5]: audit every
  socket/config/action/log/journal/fixture for loopback/synthetic/dev-only use,
  no allowlist mutation, and no public RPC/account/key/faucet/transfer/ParaId/
  coretime/upload/deploy/release/governance/secret action.

## Repository and Documentation Checks

- `test_docs_state_one_authority_attestation_and_local_proof_model`
  [T-1116-E1]: all current-generation Book/root/crate/chain/appendix surfaces
  agree that Project Book, pallet, and providers own different facts; SQLite is
  a projection; attestation trusts one archive RPC; signer/journal do not imply
  attribution; and the proof is local only. Terminal INT-0010/0012/0013 remain
  byte-identical and explicitly historical/superseded rather than rewritten.
- `test_security_docs_preserve_exact_risks_and_nonclaims` [T-1116-E2]: require
  SQL-bind versus path/file/resource/TOCTOU/VFS distinction; Linux process-crash
  versus lying-hardware power loss; journal deletion/noncanonicity; finality,
  fees, lag, indeterminacy, disclosure, rebuild and least privilege. Reject
  independent-finality/tamper-proof/audit/erasure/perpetual-history/generic-Unix/
  production-readiness claims.
- `test_docs_have_no_legacy_authority_and_locked_inputs_are_byte_identical`
  [T-1116-E3]: scan new/current-generation documentation for positive supported-
  migration/synthetic-origin/dual-write/current-v1/public-action/secret/live-
  security authority claims, using an explicit allowlist for approved
  historical or negative occurrences; throughout Build require exact SHA-256
  `521bc0e01bcbc393a1b6c9fabb5b0d5c13cfd1f2e0f41166ce96bbc0860a4fa4`
  for INT-0010, `639f374ebf7d62ed6fcf9e50224239aaa3e91bbe90eb0590b6d450dcc152e6ab`
  for INT-0012, `c3841eb71b3f0c363369cd26ae21457f313769988a3563ac66dc69b901d07ca8`
  for INT-0013, and `365116856e79d1bded60b12c68a5d8f2b4965d650af2c6feeebdc918148c15e0`
  for root `.github/workflows/ci.yml`.
- `test_repo_local_validator_is_posix_portable_and_actionable` [T-1117-E1]: run
  repository-owned Book/link/scope validator in a minimal POSIX environment
  without `rg`; inject broken intent/task/link/nonclaim/byte guards and require
  exact actionable path/reason failures.
- `test_chain_workflow_fetches_once_then_runs_locked_offline` [T-1117-E2]:
  inspect/manual-dispatch separate `workflow_dispatch` chain job, assert root CI
  bytes equal, verify pins, trace one explicit dependency/artifact fetch, then
  run each child in the pinned user+network namespace; require only-loopback
  interface/route inventory, denied external and successful loopback probes,
  and root + chain exact-1.93.0 fmt/Clippy `-D warnings`/tests/doctests/Wasm/
  benchmark weights/protocol fixtures/Zombienet with no public endpoint.
- `test_chain_cache_keys_and_hits_are_pin_verified` [T-1117-E3]: inspect/cache-
  exercise separate immutable root Cargo, chain Cargo, toolchain/Wasm, Zombienet,
  and target keys using OS plus exact lock/tool/pin/artifact hashes; reject
  unverified binaries/node DB and require checksum verification on every hit.
- `test_exact_candidate_resource_budgets_are_met` [T-1117-E4]: retain one cold
  and one warm measurement from the declared Linux runner and exact candidate.
  Cold uses fresh test-owned workspace/cache/node roots; warm reuses only the
  verified dependency/build caches from that cold run and never a node database.
  Require cold <=90 min, warm <=30 min, peak workspace+cache+node disk <=60 GiB,
  Zombienet <=30 min, bounded timeout/cleanup, and no skipped-job substitution.

## Canonical Commands and Evidence

Run one clean exact candidate in this order. Only step 1 may access the network,
and only for exact development dependencies/artifacts. Every command in steps
2–22 is executed as the argv after `bash chain/tools/loopback-netns.sh --`; that
pinned wrapper must establish a user+network namespace with only `lo`, no
non-loopback interfaces/routes, a denied external-connect probe, and a
successful loopback probe before executing any child. Failure to establish
isolation fails the gate; no command may contact a public blockchain.

1. `bash chain/tools/verify-pins.sh --fetch-exact`
2. `bash chain/tools/verify-pins.sh --locked --offline`
3. `bash scripts/check-book.sh`
4. `bash protocol/v2/verify-fixtures.sh --locked`
5. `cargo +1.93.0 metadata --locked --offline --format-version 1`
6. `cargo +1.93.0 fmt --all -- --check`
7. `cargo +1.93.0 clippy --workspace --all-targets --all-features --locked --offline -- -D warnings`
8. `env 'RUSTFLAGS=-D warnings' cargo +1.93.0 check --workspace --all-targets --all-features --locked --offline`
9. `cargo +1.93.0 test --workspace --all-targets --all-features --locked --offline`
10. `cargo +1.93.0 test --doc --workspace --locked --offline`
11. `cargo +1.93.0 fmt --manifest-path chain/Cargo.toml --all -- --check`
12. `cargo +1.93.0 clippy --manifest-path chain/Cargo.toml --workspace --all-targets --all-features --locked --offline -- -D warnings`
13. `env 'RUSTFLAGS=-D warnings' cargo +1.93.0 check --manifest-path chain/Cargo.toml --workspace --all-targets --all-features --locked --offline`
14. `cargo +1.93.0 test --manifest-path chain/Cargo.toml --workspace --all-targets --all-features --locked --offline`
15. `cargo +1.93.0 test --doc --manifest-path chain/Cargo.toml --workspace --locked --offline`
16. `cargo +1.93.0 build --manifest-path chain/Cargo.toml --package cubikan-runtime --release --locked --offline`; require `wasm32v1-none` `wbuild` output, then `bash chain/tools/verify-runtime-artifacts.sh --locked`
17. `bash chain/tools/verify-weights.sh --locked --offline`
18. `bash chain/tools/run-zombienet-e2e.sh --config chain/config/zombienet.toml --relay-validators 2 --collators 2 --loopback-only`; enforce <=30-minute timeout and cleanup trap, and require an actual unskipped run
19. `bash chain/tools/measure-resources.sh --exact-candidate --cold --offline` and `bash chain/tools/measure-resources.sh --exact-candidate --warm --offline`
20. `bash docs/sprints/s11/sprint-tests/verify-links-and-scope.sh`
21. Mechanically extract `T-11xx-Ex` identifiers from Build success clauses and Primary EARS rows; require exact set equality, 80 unique IDs, 80 unique primary names, and no missing/extra/duplicate row.
22. `git diff --check`

Evidence records the exact commit, OS/filesystem declaration, all tool/pin/
artifact/schema/fixture hashes, command lines/exits, named test counts, before/
after atomicity digests, ordered SQLite open trace, schema/config inventory,
attested coordinates, every journal crash/scan point, four processes and eighteen
fixed ports, failover C/final F, restart/probe proof, uninterrupted/A/B semantic
digests, cleanup/socket/action audit, cold/warm/disk/Zombienet measurements, and
zero skipped required steps. A manually dispatched hosted run counts only when
it actually runs the identical candidate; it authorizes neither merge nor
public deployment.

## Negative Boundaries

- No Paseo/Polkadot public endpoint, public account, key import, funding,
  transfer, faucet, ParaId reservation/registration, coretime, runtime upload,
  deployment, release, governance, production secret, or external mutation.
- Local 2+2 Zombienet proves pinned-runtime composition, finality consumption,
  failover, archive resynchronization, and rebuild—not public shared/economic
  security, Byzantine resilience, validator honesty, governance, or production
  readiness.
- Full-stream attestation proves equality with one configured archive RPC
  assertion. It is not GRANDPA/light-client verification, compromised-node
  defense, cryptographic database authenticity, or perpetual history.
- SQLite never supplies mutation preflight, deployment, revision, nonce, signing,
  or retry inputs. There is no schema/envelope/protocol legacy migration,
  synthetic origin, dual write, public projection writer/capability constructor,
  provisional state, runtime upgrade, or reachable allowlist-governance journey.
- Bound parameters prevent caller text becoming SQL structure; they do not by
  themselves defend hostile files, path replacement, resource exhaustion, or a
  compromised RPC. The proof excludes hard links, hostile parent replacement,
  custom VFS, same-user unresolved-journal deletion, continuous post-open TOCTOU,
  unsupported filesystem semantics, lying hardware/power-loss, arbitrary hostile
  database service, SQL firewall, and OS-level CPU/memory quota claims.
- The signer journal coordinates cooperating processes on one canonical
  projection/signer lane and survives ordinary process kill after acknowledged
  fsync steps. It is not canonical state, a secret store, an automatic-retry
  basis, an exactly-once guarantee, or protection from external signer use.
- No owner/author/causal/quality/satisfaction meaning attaches to technical
  signer, block producer, Git committer/blame, provider output, association, peer
  approval, or attestation.
- Pagination is one live attested page, not a cross-page snapshot. There is no
  transitive execution graph, scheduler, delegation, stored board, relationship
  history API, atomic association replacement, delivery idempotency token,
  provider fetch/verification, public service, or erasure guarantee.
