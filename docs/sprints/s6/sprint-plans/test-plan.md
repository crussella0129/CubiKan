Finalized - DO NOT EDIT

# Sprint 6 Test Plan

## Intent Traceability

| Intent | Acceptance criterion | Build task / EARS clause | Verification |
|--------|----------------------|--------------------------|--------------|
| [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md) | Exact advisory appendix title, authority, and accurate current-state warning. | T-601-E1–E2 | `test_appendix_authority_and_current_boundary` |
| [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md) | Retained themes are classified without conflating lifecycle, provenance, or execution graphs. | T-601-E2–E4, T-602-E2–E4, T-603-E2–E5 | `test_graph_and_authority_boundaries_are_distinct`, `test_theme_to_intent_to_repository_traceability` |
| [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md) | The catalog covers all six requested derivative families. | T-602-E1–E4, T-603-E1–E4 | `test_derivative_catalog_has_six_complete_unique_entries` |
| [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md) | Every repository entry has outcome, ownership, CubiKan boundary, prerequisites, trigger, and non-goals. | T-602-E1, T-603-E1 | `test_agent_accounting_entries_have_required_contract_fields`, `test_process_application_entries_have_required_contract_fields` |
| [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md) | High-value backend gaps are distinct proposed intents with a dependency map. | T-601-E4 | `test_proposed_backend_intent_map_is_complete_and_unscheduled` |
| [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md) | Derivatives use a pinned current core or future versioned boundary, never shared storage/provisional Serde; each datum has one authority. | T-601-E3, T-601-E6, T-602-E2–E4, T-603-E2–E4 | `test_derivative_dependency_direction_is_safe`, `test_book_backend_authority_avoids_split_brain` |
| [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md) | Navigation and links validate with prose-only local scope and no derivative-repository authorization; local and hosted quality gates remain green. | T-601-E5, T-603-E5–E6 | `test_book_navigation_and_local_links_resolve`, `test_prose_only_scope_has_no_runtime_or_existing_intent_drift`, `test_t603_candidate_passes_five_local_rust_gates`, `test_remote_scope_matches_recorded_baseline_and_operations`, `test_hosted_sprint_six_quality_run_succeeds` |

## Unit and Repository Contract Checks

### T-601 appendix foundation checks

- **Intent:** [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md)
- `test_appendix_authority_and_current_boundary` [T-601-E1–E2]: inspect the exact heading, advisory banner, semantic-authority statement, explicit chain-agnostic current core/CLI description, and absence of present-tense durable backend or repository-existence claims.
- `test_derivative_dependency_direction_is_safe` [T-601-E3]: inspect the integration baseline and assert CubiKan-owned versus derivative-owned data/policy, permitted explicitly pinned-current-core/future-versioned paths, no cross-version Rust API promise, explicit rejection of direct database access, provisional core Serde, and one-shot CLI-as-session coupling, and separate `WorkflowEdge`, provenance, delegation, cross-unit, and execution-edge meanings.
- `test_proposed_backend_intent_map_is_complete_and_unscheduled` [T-601-E4]: require one link each to INT-0008–INT-0012, exact `proposed` state and `none` Work/Completion evidence in those chapters, the locked partial order and hard dependencies, INT-0009 stale-before-domain-error precedence, and explicit storage/transport/auth/concurrency/privacy/metric/relation policy tripwires.
- `test_book_navigation_and_local_links_resolve` [T-601-E5]: assert one Summary link to `appendix/README.md`, one to `appendix/potential-derivative-projects.md`, one link to each new intent, and full local path/fragment resolution. This check—not `check-book.sh`—is the navigation/link oracle.
- `test_book_backend_authority_avoids_split_brain` [T-601-E6]: inspect the authority table and require one canonical owner per datum, the Book as current semantic/historical authority, no Book/backend dual-write, and a separate selected projection/migration intent before operational truth changes authority.
- `test_book_v2_intent_schema_and_state` [T-601-E4]: run the installed `check-book.sh` and require 12 valid intent chapters; attribute only intent schema/state validation to this check.
- Stubs/mocks: none. These are one-off source and repository inspections recorded in Test artifacts, not implementation-mirroring Rust tests.

### T-602 agent and accounting catalog checks

- **Intent:** [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md)
- `test_agent_accounting_entries_have_required_contract_fields` [T-602-E1]: parse the three entries present after T-602 and assert each separately names problem/outcome, owned data, owned policy, inputs, outputs, CubiKan interaction, prerequisites, creation trigger, separation rationale, and non-goals with no placeholder values.
- `test_agent_operations_boundary_is_explicit` [T-602-E2]: assert Agent Ops represents assigned work as Intent Units while retaining every named orchestration, permission, retry, approval, and cost concern outside CubiKan.
- `test_observatory_separates_recorded_provenance_from_inference` [T-602-E3]: assert distinct ID namespaces, full revision evidence, Book/Git/PR/CI/test/trace inputs, derived blame/scores, no causality or certification claim, and a creation gate requiring data minimization, retention/redaction, access control, and human approval before scoring or adaptation.
- `test_animus_ledger_requires_an_accounting_model` [T-602-E4]: assert separate accounting ownership, provenance prerequisite, explicit unit/correction/trust/anti-gaming questions, no claim that current lifecycle history is a journal, and nested/recursive loop composition mapped to Agent Ops, Skill Graph, and INT-0012 or explicitly left open without core lineage.

### T-603 process, graph, organization, and scope checks

- **Intent:** [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md)
- `test_process_application_entries_have_required_contract_fields` [T-603-E1]: parse the three entries added by T-603 and require the same separately named problem/outcome, owned-data, owned-policy, input, output, interaction, prerequisite, creation-trigger, separation, non-goal, and intent-link fields with no placeholders.
- `test_process_studio_keeps_kpi_policy_outside_core` [T-603-E2]: assert Studio authors/governs business definitions and authorization, the backend stores raw observations and deterministically evaluates only caller-supplied versioned definitions, Observatory consumes results, immutable workflows enter core validation, local pinned-core design can precede persistence, shared operational/KPI use waits for INT-0010/INT-0011, and Electron-first is a recommendation rather than an implemented UI.
- `test_skill_graph_keeps_execution_edges_outside_workflow` [T-603-E3]: assert separate graph identities and every named skill/execution policy outside core.
- `test_org_apps_keep_domain_data_and_policy_bounded` [T-603-E4]: assert one primary app-kit entry, the unnamed/independently authorized vertical-repo pattern, INT-0010 collection queries for basic projections, INT-0012 for advanced multi-board relations, and explicit domain/PII/RBAC/integration/UX ownership.
- `test_appendix_sequence_and_noncommitment_scope` [T-603-E5]: assert exactly six primary entries, sequencing, creation triggers, rejected/merged alternatives, open questions, no conflicting system-of-record/policy authority, and language that neither claims nor authorizes external repository mutation, publication, package/release/deployment, chain/database/transport selection, or roadmap commitment.
- `test_prose_only_scope_has_no_runtime_or_existing_intent_drift` [T-601-E2–E4, T-603-E5–E6]: compare accepted base to candidate/committed Build head with an exhaustive changed-path allowlist limited to `docs/SUMMARY.md`, exactly `docs/appendix/README.md` and `docs/appendix/potential-derivative-projects.md`, the exact INT-0007–INT-0012 chapters, `docs/sprints/s6/**`, `docs/work/tasks.md`, and `docs/work/completed-tasks.md`; separately require byte-identical INT-0001–INT-0006 and no manifest, lockfile, Rust, CI, submodule, workspace-member, dependency, package, release, root-README, license, history, or remote-profile change.
- `test_t603_candidate_passes_five_local_rust_gates` [T-603-E6]: after the complete T-603 prose candidate and scope check exist but before `commit-task.sh` creates T-603’s semantic and ledger-evidence commits, execute and record the exact formatting, Clippy, warnings-denied check, all-target test, and doctest commands from Final Quality Gates; require all five to pass. Test later repeats them at the committed Build head but does not substitute that later run for this timing-aware Build check.
- `test_remote_scope_matches_recorded_baseline_and_operations` [T-603-E5–E6]: record `git remote -v` and local remote configuration before Build, compare them at Test, and record that Sprint 6 issued only the authorized push to the existing CubiKan `dev` remote and no create/push/release/deploy call targeting any derivative slug. This operational record is distinct from the Git diff and does not claim visibility into unrelated external activity.

## Integration Tests

### Cross-document ecosystem consistency

- **Intents:** [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md), informed by proposed [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md), [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md), [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md), [INT-0011](../../../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md), and [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- `test_theme_to_intent_to_repository_traceability` [T-601-E4, T-602-E1–E4, T-603-E1–E5]: use the enumerated sanitized inventory `DV-01` through `DV-09` as the oracle; trace every retained/merged/deferred item through its backend/adapter/derivative classification to a catalog entry, intent, alternative, or open question without claiming completeness against the omitted Discord conversation.
- `test_graph_and_authority_boundaries_are_distinct` [T-601-E2–E3, T-601-E6, T-602-E2–E4, T-603-E2–E5]: compare realized core vocabulary, proposed intents, and all six repo entries; assert that phase topology, provenance, cross-unit relations, execution DAGs, metrics, accounting, and Book semantics have separate authorities.
- `test_derivative_catalog_has_six_complete_unique_entries` [T-602-E1–E4, T-603-E1–E4]: after T-603, assert exactly six primary recommended repo slugs, no conflicting system-of-record or policy ownership, explicit shared inputs, and complete integration/prerequisite fields across the final document; organizational verticals are an unnamed future pattern rather than extra entries.
- `test_each_derivative_maps_to_declared_cubikan_capabilities` [T-601-E3–E4, T-602-E2–E4, T-603-E2–E4]: assert each repo names an explicitly pinned current public core version or the linked future intents it needs, and that its creation sequence does not precede an unmet hard prerequisite.
- `test_hosted_sprint_six_quality_run_succeeds` [Test-phase remote checkpoint; INT-0005 preservation]: after the clean committed Build head exists and is pushed to the existing CubiKan `dev` branch, query GitHub and assert event `push`, branch `dev`, exact head SHA, one `Rust quality gate` job, and success for formatting, Clippy, warnings-denied check, all-target tests, and doctests. This hosted oracle cannot prove external non-mutation and is not a Build-task precondition.
- No mocks/stubs. The integration boundary is the composed Project Book: realized intents → proposed intents → advisory appendix → navigation.

## End-to-End Tests

- **Status:** possible for the documentation product; runtime integration is intentionally out of scope.
- `test_reader_can_navigate_summary_to_appendix_and_intents` [T-601-E1–E6, T-602-E1, T-603-E1, T-603-E5]: start at `docs/SUMMARY.md`, follow the Appendix index to Potential Derivative Projects, traverse every local intent/capability prerequisite link, and assert that a reader can reach the authority banner, current-state boundary, all six complete primary entries, partial-order sequence, alternatives, open questions, and non-goals without a broken reference or contradictory current/future claim. Recommended repo slugs remain code labels or local anchors, not links to nonexistent repositories.
- `test_hosted_sprint_six_quality_run_succeeds` [Test-phase remote checkpoint; INT-0005 preservation]: observe the existing CubiKan GitHub Actions boundary at the exact Build SHA and require its sole job and five gates to complete successfully; this is hosted regression evidence, not proof that a derivative backend or repository exists.
- Product/runtime E2E is not evidence for this prose-only intent. It becomes possible only when a later sprint selects and implements one proposed backend intent and a derivative repository is separately authorized; Sprint 6 must not claim such integration from document checks.

## Test Artifact Locations

- Intent schema/state, appendix structure, catalog fields, scope, Rust regression, and Book/link confirmations: `docs/sprints/s6/sprint-tests/unit-tests.md`.
- Cross-document theme/intent/repository/authority composition: `docs/sprints/s6/sprint-tests/integration-tests.md`.
- Reader navigation journey and hosted exact-head quality evidence: `docs/sprints/s6/sprint-tests/e2e-tests.md`.
- Final reviewed result: `docs/sprints/s6/sprint-tests/test-report.md`.

## Final Quality Gates

- Installed `research-budget.sh` passes its source/file budget; installed `check-book.sh` reports 12 valid intent chapters and proves their schema/state only.
- The separate `test_book_navigation_and_local_links_resolve` check proves Summary reachability and every local Markdown path/fragment introduced by Sprint 6.
- Exact required headings, six unique repository slugs, catalog fields, theme traceability, authority boundaries, dependency ordering, open questions, and nonclaims pass the named one-off checks.
- `git diff --check` passes; the accepted-base-to-Build-head changed-path set exactly fits the exhaustive prose allowlist; INT-0001–INT-0006 remain byte-identical; and recorded remote configuration matches its pre-Build baseline.
- `cargo +stable fmt --all -- --check` passes.
- `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings` passes.
- `RUSTFLAGS="-D warnings" cargo +stable check --workspace --all-targets` passes.
- `cargo +stable test --workspace --all-targets` preserves the accepted 100-test runtime baseline.
- `cargo +stable test --doc --workspace` preserves the accepted one-doctest baseline.
- After the committed Build head exists, the existing GitHub `Rust CI` push run and sole quality job succeed at that exact SHA before INT-0007 realization; the later draft PR run is a remote-checkpoint confirmation.
