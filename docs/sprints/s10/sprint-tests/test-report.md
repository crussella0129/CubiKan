# Sprint 10 Test Report

## Intent Verification

| Intent | Acceptance criterion | EARS / tests | Result | Intent evidence update |
|--------|----------------------|---------------|--------|------------------------|
| [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) | Distinguish the four current surfaces, SQLite schema v1/v2, relationship/projection v1, and the lifecycle-only local protocol from Rust-only relationship/projection operations. | T-1001 / boundary **SHALL** distinguish four surfaces and named versions; `test_current_boundary_and_version_matrix_are_current`; `test_appendix_matches_current_project_and_backend_guides` | pass | Link this report from Test evidence; keep the intent active until required completion and realization evidence exists. |
| [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) | Report INT-0009, INT-0010, and INT-0012 as realized and available; retain INT-0008 and INT-0011 as proposed; preserve one canonical authority per datum. | T-1002 / maps **SHALL** distinguish realized and proposed capabilities and retain documented authorities; `test_capability_map_statuses_match_book` | pass | Link this report from Test evidence; keep the intent active until required completion and realization evidence exists. |
| [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) | Preserve all seven theme families, the backend/adapter/derivative classifications, complete catalog fields, and the lifecycle-edge distinction. | T-1004 / inventory and catalog **SHALL** retain all seven families, all required fields, and edge meaning; `test_catalog_remains_complete_and_preserves_edge_meaning`; `test_catalog_prerequisites_use_realized_capabilities` | pass | Link this report from Test evidence; keep the intent active until required completion and realization evidence exists. |
| [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) | Keep all six recommended entries complete and conditional, without representing a derivative repository as existing, scheduled, or authorized. | T-1004 / every entry **SHALL** retain outcome, owned data/policy, integration boundary, prerequisites, conditional trigger, and non-goals; T-1005 / entries **SHALL** retain authorization and creation gates; `test_catalog_remains_complete_and_preserves_edge_meaning`; `test_derivative_creation_remains_unauthorized` | pass | Link this report from Test evidence; keep the intent active until required completion and realization evidence exists. |
| [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) | Preserve supported backend/pinned-core consumption, separate adapter governance, storage and Serde guardrails, scoped Book/backend authority, no dual-write, and separately authorized authority transfer. | T-1003 / consumption paths **SHALL** be explicit; authority transfer **SHALL** prohibit dual-write and require a separate authorized intent; guardrails **SHALL** preserve prohibitions and nonclaims; `test_supported_consumption_paths_and_authority_transfer_are_explicit`; `test_advisory_and_storage_protocol_boundaries_remain_intact`; `test_appendix_matches_current_project_and_backend_guides` | pass | Link this report from Test evidence; keep the intent active until required completion and realization evidence exists. |
| [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) | Close questions about realized foundations while retaining still-open provenance, evidence, compatibility, security, authorization, UI, deployment, and derivative-policy questions. | T-1006 / sequencing **SHALL** leave only unresolved work open; `test_sequence_and_open_questions_exclude_completed_foundations` | pass | Link this report from Test evidence; keep the intent active until required completion and realization evidence exists. |
| [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) | Validate links, fragments, Book navigation, documentation-only scope, unchanged protected product surfaces, singular backlog closure, and no derivative create/push/publish/release/deploy operation. | T-1007 / backlog **SHALL** move once to completion; T-1001–T-1007 composed; T-1005 / action audit **SHALL** contain no derivative mutation; `test_backlog_moves_once_to_completion_ledger`; `test_appendix_links_and_book_navigation_resolve`; `test_documentation_maintenance_scope_is_non_product`; `verify_workspace_regression_gates`; `verify_repository_hygiene`; `verify_no_derivative_repository_operations` | pass | Link this report from Test evidence; keep the intent active until required completion and realization evidence exists. |

## Summary

- Unit/repository tests: 9 passed / 0 failed / 9 total
- Integration tests: 3 passed / 0 failed / 3 total
- E2E tests: 3 passed / 0 failed / 3 total
- Local Rust regression: 191 passed / 0 failed / 191 total; doctests 1 passed / 0 failed / 1 total
- Finalized Sprint 10 selection: 15 passed / 0 failed / 15 total
- CI status: green
- Formal Test critique: [clean](critique.md)

The exact tested candidate was
`0a7bc3a023364cca9197e735c5acfeab019ce8a1`. Detailed arrangements,
assertions, and observations are preserved in [unit-tests.md](unit-tests.md),
[integration-tests.md](integration-tests.md), and [e2e-tests.md](e2e-tests.md).

## CI Confirmation

- **Head SHA:** `0a7bc3a023364cca9197e735c5acfeab019ce8a1`
- **CI run:** [31533101690 — Rust CI](https://github.com/crussella0129/CubiKan/actions/runs/31533101690)
- **CI job:** [93917639820 — Rust quality gate](https://github.com/crussella0129/CubiKan/actions/runs/31533101690/job/93917639820)
- **Conclusion:** success on attempt 1
- **Confirmations:** The run API, job API, checkout fetch, checkout, and final
  `git rev-parse` identified the same exact SHA. Formatting, Clippy with
  warnings denied, the warnings-denied check, all-target tests, and doctests
  completed successfully. The hosted workspace result was 191/191 tests and
  1/1 doctest; the stricter locked/offline local gates reported the same test
  totals.

## Failures

None. Every finalized structural, integration, E2E, local Rust, and hosted CI
check passed.

## Technical Debt Identified

- No new in-scope technical debt was identified by a failed or caveated test.
- [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md) and
  [INT-0011](../../../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md)
  remain proposed capabilities. Their provenance/evidence work and any
  derivative-specific adapters remain separately governed future work, not
  debt introduced or authorized by Sprint 10.
- Complete private-account inventory was outside the available provider
  evidence. Expanding that scope would require separately authorized,
  authenticated evidence collection; this report does not create a follow-up
  intent or treat the bounded observation as an implementation defect.

## Coverage Observations

### Formal Test critique response

The initial formal Test critique blocked on a malformed 41-character first-push
SHA that the offline audit accepted. The value was corrected to the resolvable
40-character commit `4040493ce4cb3ff060d10721211e3ec1135de6d5`, and candidate
`0a7bc3a023364cca9197e735c5acfeab019ce8a1` hardened the validator to require
full lowercase commit identities, object resolution, ordered candidate
ancestry, final-push equality, scoped repository findings, and timestamps
inside the declared observation window. Negative self-tests now reject both
malformed and unresolved heads. The full local selection and exact-head hosted
CI passed again, and the saved final critique is clean.

### Plan-critique catalog response

The plan critic's C-001 vacuity risk is closed by
`test_catalog_remains_complete_and_preserves_edge_meaning`. The executable
oracle requires all seven retained theme families, all six exact recommended
slugs, exactly one of every mandatory entry field, the backend/adapter/
derivative classification, the lifecycle phase-edge non-conflation rule, and
all nine responsibility mappings. `test_catalog_prerequisites_use_realized_capabilities`
then inspects every catalog reference to INT-0009, INT-0010, and INT-0012 so a
complete but stale catalog also fails. Both checks passed.

### Evidence scope and provider limitations

The connected repository searches covered the installation visible to this
session, not a complete GitHub account inventory. Each of the six exact
recommended slugs returned zero connected-search results in that scope and an
HTTP 404 from the public repository endpoint. Public 404 proves only that the
named repository was not publicly visible through that endpoint. The release
observation covers published releases, not drafts; the paginated deployment
observation reported zero deployments.

The durable Sprint 10 action ledger covers the declared observation window
`2026-08-11T19:25:11Z`–`2026-08-11T20:28:26Z`. It contains 23 actions and four
mutations, all four being pushes to `crussella0129/CubiKan` on `dev`; it records
no derivative creation, push, publication, release, or deployment. The
checked-in audit validator proves fixture shape, declared completeness and
internal consistency, candidate binding, commit resolution/ancestry, and
allowed mutation targets. It runs offline and does not itself call or
authenticate GitHub.

Accordingly, the E2E result is a bounded claim about the six named slugs, the
two observed provider scopes, current durable Git state, and the recorded
Sprint 10 actions. It is not proof of account-wide nonexistence, draft-release
absence, or erased, rewritten, or otherwise unavailable external history.

### Product and lifecycle nonclaims

Accepted-base comparison found the protected Rust sources, manifests,
lockfile, CI, remote profile, and other pre-existing intent semantics
unchanged; the 191/191 Rust tests and 1/1 doctest corroborate runtime
regression safety. This report does not authorize a derivative repository,
promise compatibility or network/security/deployment behavior, merge `dev`
to `main`, or itself realize INT-0013. At Test completion the intent remained
active pending Loop reconciliation of completion, test, and documentation
evidence.
