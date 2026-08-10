# Plan Critique — Sprint 10

## Concerns

### C-001: Mandatory catalog coverage could pass vacuously
- **Where:** `build-plan.md` T-1002–T-1005; `test-plan.md` Unit Tests
- **Quote:** “catalog completeness”
- **Failure mode:** plan-test-mismatch
- **Why it matters:** The initial plan did not require all retained theme families, the phase-edge distinction, or every mandatory catalog field, so an entry could disappear without failing a check.
- **Suggested response:** fix-in-plan
- **Response:** Fixed. T-1004 now has a complete-inventory EARS clause, and `test_catalog_remains_complete_and_preserves_edge_meaning` verifies all seven theme families, all six entries, lifecycle-edge non-conflation, and every required entry field.

### C-002: Positive integration and authority-transfer requirements lacked an oracle
- **Where:** `build-plan.md` T-1003; `test-plan.md` T-1003 documentation checks
- **Quote:** “retain Book authority; prohibit direct database editing”
- **Failure mode:** intent-drift
- **Why it matters:** Negative prohibitions did not prove supported local-backend/pinned-core paths or the explicit migration boundary for operational truth.
- **Suggested response:** fix-in-plan
- **Response:** Fixed. T-1003 now separates supported consumption paths, scoped authority transfer, and negative guardrails; `test_supported_consumption_paths_and_authority_transfer_are_explicit` verifies the positive boundaries.

### C-003: Retrospective task identities were not bound to exact legacy commits
- **Where:** `build-plan.md` Provenance Boundary; `test-plan.md` T-1007
- **Quote:** “assigns those commits their durable Book task identities”
- **Failure mode:** hidden-dep
- **Why it matters:** Legacy implementation labels collide with historical Sprint 0 IDs, so attribution needs immutable object identities rather than message inference.
- **Suggested response:** fix-in-plan
- **Response:** Fixed. The plan locks a full-SHA mapping from T-1001–T-1007 to all seven integrated commits, identifies the removed legacy ledger path, separates helper-generated Book `Commit` values from `Integrated implementation commit` values, and records bootstrap/restoration commits outside product-task attribution.

### C-004: Terminal INT-0007 could not be rewritten for maintenance
- **Where:** research `## Intents Reviewed`; initial build-plan `## Intents`
- **Quote:** “amended while realized”
- **Failure mode:** intent-drift
- **Why it matters:** Book-v2 permits `realized` only to transition to `superseded`; terminal intent semantics cannot be rewritten in place.
- **Suggested response:** fix-in-plan
- **Response:** Fixed. INT-0007 preserves its Sprint 6 contract and immutable snapshot, legally transitions `realized → superseded`, and names follow-on planned INT-0013 as the current maintenance authority.

### C-005: Sprint 10 ledger completion was not constructible
- **Where:** initial T-1007 notes and `test_backlog_moves_once_to_completion_ledger`
- **Quote:** “keeping the existing `MAINT-001` umbrella entry singular”
- **Failure mode:** hidden-dep
- **Why it matters:** The initial plan silently expected normative task `Commit` fields to contain legacy hashes instead of helper-generated reconciliation commits.
- **Suggested response:** fix-in-plan
- **Response:** Fixed. T-1007 requires each granular Sprint 10 task exactly once, with a helper-generated normative `Commit`, separate integrated legacy SHA, one corrected MAINT entry, and a named exact ledger oracle.

### C-006: Accepted-base scope omitted INT-0007 supersession
- **Where:** `build-plan.md` Provenance Boundary; `test-plan.md` scope oracle
- **Quote:** “limited to new INT-0013, the maintained appendix, and the two Book work ledgers”
- **Failure mode:** plan-test-mismatch
- **Why it matters:** Legal INT-0007 supersession is an intentional changed path and must be permitted consistently in both Build and Test scope.
- **Suggested response:** fix-in-plan
- **Response:** Fixed. Both plans now permit the exact supersession while preserving every other pre-existing intent chapter.

## Confidence
clean

Independent formal re-review found no remaining concern after C-001 through
C-006 were addressed.
