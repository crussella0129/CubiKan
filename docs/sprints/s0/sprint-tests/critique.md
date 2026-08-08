# Test Critique — Sprint 0

## Concerns

### C-001: Exact workflow policy is only spot-checked
- **Where:** `unit-tests.md` T-005 / `crates/cubikan-core/src/workflow.rs:258`
- **Quote:** “Assertions compare the exact accepted topology”
- **Failure mode:** weak-assertion
- **Why it matters:** `test_workflow_accepts_explicit_topology` checked one declared edge, one undeclared reverse edge, and two completion decisions, but did not compare the complete phase, edge, and completion collections or exhaust all phase-pair decisions. An implementation that silently permitted an extra edge or completion point could still pass despite the EARS requirement to permit exactly the declarations.
- **Suggested response:** tighten-assertion
- **Response:** **Addressed.** The test now compares the complete ID, phase, edge, initial-phase, and completion collections with independent expected values, then exhaustively checks every phase-pair transition decision and every phase completion decision. `cargo test --workspace --lib` passes all 43 tests after the change.

### C-002: Stability test compares each accessor with itself
- **Where:** `unit-tests.md` T-006 / `crates/cubikan-core/src/intent_unit.rs:493`
- **Quote:** “repeated immutable identity/species/workflow reads”
- **Failure mode:** weak-assertion
- **Why it matters:** Assertions such as `assert_eq!(unit.id(), unit.id())` were tautological and did not prove that construction preserved the supplied values.
- **Suggested response:** tighten-assertion
- **Response:** **Addressed.** The test now captures independent expected ID, species, complete workflow, and workflow ID values before construction, then compares repeated accessor reads with those values. `cargo test --workspace --lib` passes all 43 tests after the change.

### C-003: Integration artifact overstates lifecycle-record verification
- **Where:** `integration-tests.md` Domain Configuration and Lifecycle / `crates/cubikan-core/tests/lifecycle.rs:34`
- **Quote:** “verifies immutable identity/species/workflow, exact ordered history”
- **Failure mode:** weak-assertion
- **Why it matters:** The lifecycle integration test verified only sequence numbers `[1, 2, 3]`; it did not assert that the entries were the two expected transitions followed by the expected completion, including their from/to/final-phase fields.
- **Suggested response:** tighten-assertion
- **Response:** **Addressed.** The test now pattern-matches the exact two-transition/one-completion record sequence and asserts each sequence, source, target, and final phase. `cargo test --test lifecycle` passes all 4 integration cases after the change.

### C-004: UUID uniqueness smoke test is probabilistic
- **Where:** `unit-tests.md` T-003 / `crates/cubikan-core/src/id.rs:112`
- **Quote:** “two generated IDs differ as a smoke check”
- **Failure mode:** flake-risk
- **Why it matters:** `test_generated_intent_unit_ids_differ` depends directly on two random UUID v4 values, so it has a nonzero collision-based failure probability. The risk is negligible and matches the expressly limited smoke-check claim, but it should be acknowledged rather than treated as deterministic coverage.
- **Suggested response:** defer-with-rationale
- **Response:** **Deferred with rationale.** The locked EARS clause explicitly requires two independently generated IDs as a uniqueness smoke check. UUID v4 collision probability for one pair is negligible, and replacing generation with a deterministic fixture would stop testing the production generator. The final Test Report records this bounded probabilistic risk.

## Follow-up outcome

The follow-up critic found no remaining concerns: every EARS clause has a tight test that exercises the SHALL.

## Confidence

clean
