Finalized - DO NOT EDIT

# Sprint 10 Test Plan

## Intent Traceability

| Intent | Acceptance criterion | Build task / EARS clause | Verification |
|--------|----------------------|--------------------------|--------------|
| [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) | Accurate current boundary and local-protocol/Rust-API distinction | T-1001 / boundary SHALL distinguish four surfaces and named versions | `test_current_boundary_and_version_matrix_are_current` |
| [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) | Current intent states, dependencies, and canonical authorities | T-1002 / maps SHALL distinguish realized and proposed capabilities | `test_capability_map_statuses_match_book` |
| [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) | Available backend or pinned-core consumption path; provider/network adapters governed separately | T-1003 / consumption paths SHALL be explicit | `test_supported_consumption_paths_and_authority_transfer_are_explicit` |
| [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) | Book/backend authority separation and explicit operational-truth migration | T-1003 / authority transfer SHALL prohibit dual-write and require separate intent | `test_supported_consumption_paths_and_authority_transfer_are_explicit` |
| [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) | No direct editing, shared writable storage, provisional Serde, or inferred network/security/deployment/blockchain surface | T-1003 / guardrails SHALL preserve prohibitions and nonclaims | `test_advisory_and_storage_protocol_boundaries_remain_intact` |
| [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) | Seven theme families, lifecycle-edge distinction, and every required catalog field remain | T-1004 / inventory and catalog SHALL remain complete | `test_catalog_remains_complete_and_preserves_edge_meaning` |
| [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) | Catalog prerequisites use current capability state | T-1004 / entries SHALL treat realized primitives as available | `test_catalog_prerequisites_use_realized_capabilities` |
| [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) | Repository names remain conditional recommendations | T-1005 / entries SHALL retain authorization and creation gates | `test_derivative_creation_remains_unauthorized` |
| [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) | No derivative remote or publication mutation | T-1005 / action audit SHALL contain no create/push/publish/release/deploy operation | `verify_no_derivative_repository_operations` |
| [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) | Appendix remains advisory and reflects current dependency state | T-1006 / sequencing SHALL leave only unresolved work open | `test_sequence_and_open_questions_exclude_completed_foundations` |
| [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) | Maintenance remains bounded, traceable, and non-realizing | T-1007 / backlog SHALL move once to completion | `test_backlog_moves_once_to_completion_ledger` |
| [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) | Navigation and accepted-base scope remain valid and non-product | T-1001–T-1007 composed | `test_appendix_links_and_book_navigation_resolve`; `test_documentation_maintenance_scope_is_non_product` |

## Unit Tests

### T-1001 documentation/repository check

- **Intent:** [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- `test_current_boundary_and_version_matrix_are_current`: inspect the appendix's current-boundary and layer sections; require all four current surfaces, schema v1/v2, relationship/projection v1, and local protocol v1 with relationships/projections explicitly Rust-only.
- Stubs/mocks: none; inspect tracked Markdown and Book metadata directly.

### T-1002 documentation/repository check

- **Intent:** [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- `test_capability_map_statuses_match_book`: compare appendix status, dependency, and authority statements with the Book; require INT-0009/0010/0012 realized and INT-0008/0011 proposed, with no blanket “INT-0008–INT-0012 are proposed” statement and no canonical-authority drift.
- Stubs/mocks: none; inspect tracked Markdown and Book metadata directly.

### T-1003 documentation/repository check

- **Intent:** [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- `test_supported_consumption_paths_and_authority_transfer_are_explicit`: require the available versioned local Rust backend and explicitly pinned core paths, separate governance for provider/network adapters, Book/backend authority separation, no dual-write, and a separate projection/migration intent before operational truth moves.
- `test_advisory_and_storage_protocol_boundaries_remain_intact`: require the advisory/no-repository-exists language, no direct database editing/shared writable storage, no provisional core Serde contract, no cross-version promise, and no network/auth/deployment/blockchain claim.
- Stubs/mocks: none; inspect tracked Markdown directly.

### T-1004 documentation/repository check

- **Intent:** [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- `test_catalog_remains_complete_and_preserves_edge_meaning`: require all seven theme families, backend/adapter/derivative classification, the phase-edge non-conflation rule, all six catalog entries, and each entry's outcome, owned data/policy, integration boundary, prerequisites, conditional creation trigger, and explicit non-goals.
- `test_catalog_prerequisites_use_realized_capabilities`: inspect every catalog occurrence of INT-0009/0010/0012 and require realized capabilities to be described as available while still-proposed evidence/provenance or derivative adapters remain future.
- Stubs/mocks: none; use exact-text searches plus semantic review of each matching paragraph.

### T-1005 documentation/repository check

- **Intent:** [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- `test_derivative_creation_remains_unauthorized`: require every creation trigger and the appendix-level noncommitment language to remain conditional, with no statement that any derivative repository exists, is scheduled, or has been authorized.
- Stubs/mocks: none; inspect the advisory banner and every catalog entry.

### T-1006 documentation/repository check

- **Intent:** [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- `test_sequence_and_open_questions_exclude_completed_foundations`: require sequencing and open questions to stop asking whether revision, durable backend, or relationship/projection foundations will be realized; retain unresolved compatibility, security, authorization, evidence, and derivative-policy questions.
- Stubs/mocks: none; use exact-text searches plus semantic review of each matching paragraph.

### T-1007 documentation/repository check

- **Intent:** [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- `test_backlog_moves_once_to_completion_ledger`: require the exact INT-0007 maintenance item to be absent from `docs/work/tasks.md`; `MAINT-001` and T-1001 through T-1007 to occur exactly once in `docs/work/completed-tasks.md`; every normative `Commit` to resolve to a new helper-created Book reconciliation task commit; every `Integrated implementation commit` to equal its locked legacy SHA; `MAINT-001` to use integrated T-1007 hash `a7ed48992897c8463ba6cc729e944398c8ae8779` and distinguish originating INT-0007 work from superseding INT-0013 reconciliation; and authority-restoration commit `b170e107d08ac1855d6b1be82fbf1ebe25a22f3a` to remain outside product-task attribution.
- Stubs/mocks: none; use Book ledger and Git-object inspection.

## Integration Tests

### Current-contract consistency

- **Intent:** [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- `test_appendix_matches_current_project_and_backend_guides`: T-1001–T-1004 composed; cross-check the appendix against `README.md`, `crates/cubikan-backend/README.md`, and `crates/cubikan-local/README.md`; require consistent surfaces, versions, migration, relationship/projection, and protocol-boundary descriptions.
- `test_appendix_links_and_book_navigation_resolve`: T-1001–T-1007 composed; validate every local Markdown path and fragment plus Book intent navigation; require zero broken targets, all 12 pre-existing intents, and new INT-0013 reachable.
- `test_documentation_maintenance_scope_is_non_product`: T-1001–T-1007 composed; compare accepted base `bb257db8c62083ae8be4e8d77ec63762ba2e8fa8` with the candidate. Permit product/documentation changes only to INT-0007's legal supersession, new INT-0013, the appendix, and the two Book ledgers, plus Sprint 10 provenance and `docs/SUMMARY.md` navigation; require Rust sources, Cargo manifests/lockfile, `.github`, remote profile, every other pre-existing intent chapter, and runtime behavior unchanged, with no legacy root Sprint Loop authority.

## End-to-End Tests

- **Status:** possible
- **Intent:** [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- `verify_workspace_regression_gates`: T-1001–T-1007 composed; run workspace formatting, Clippy with warnings denied, warnings-denied all-target check, all-target tests, and doctests; require all existing Rust behavior to remain green.
- `verify_repository_hygiene`: T-1001–T-1007 composed; run `git diff --check`, the authoritative Book-v2 validator, pinned/current Markdown path-and-fragment resolution, and layout routing; require clean output, 13 intent chapters reachable, and no duplicate root writable authority.
- `verify_no_derivative_repository_operations`: T-1005 action audit; save the exact tested-head Git remotes/commits, connected GitHub account repository inventory for all six recommended slugs, CubiKan release/deployment inventory, and the sprint tool/action ledger in `e2e-tests.md`. Require every recorded remote mutation to target only `crussella0129/CubiKan` on declared work branch `dev`, no recommended derivative slug to exist, and no derivative create/push/publish/release/deploy action. This is a bounded audit of durable provider and sprint evidence, not a claim about erased external history.
- Mock-real data: checked-in Book, appendix, README guides, Rust workspace, and Git history; no mocks, external service mutation, derivative repository, or deployment action.
