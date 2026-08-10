Finalized - DO NOT EDIT

# Sprint 10 Build Plan

## Intents

- [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) — state: planned; acceptance criteria covered: accurate advisory/current boundary, complete retained-theme classification and catalog, current intent-state/dependency map, safe consumption paths and canonical authority, conditional derivative creation, valid navigation, operational non-creation, and documentation-only scope. Superseded [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md) remains immutable historical authority for the original catalog outcome.

## Provenance Boundary

The explicit user invocation initially routed through an installed legacy
Sprint Loop runtime that is incompatible with this repository's authoritative
Book-v2 layout. Its seven bounded implementation commits are preserved in Git,
but the duplicate root authority was removed and its working evidence archived
before Sprint 10 was initialized. This plan assigns those commits their durable
Book task identities without rewriting history. The accepted pre-maintenance
base is Sprint 9's merge commit
`bb257db8c62083ae8be4e8d77ec63762ba2e8fa8`; Book sprint evidence and
navigation are provenance, while the product/documentation delta is limited to
INT-0007's legal supersession, new INT-0013, the maintained appendix, and the
two Book work ledgers.

Exact reconciliation is immutable and testable:

| Book task | Legacy label | Integrated commit |
|-----------|--------------|-------------------|
| T-1001 | `sprint-0: T-001` | `d725411e0bf4c97437544e28c604e48f0c1badbf` |
| T-1002 | `sprint-0: T-002` | `a4c14cfcaccc23afeebafe28490b63b0683d17e8` |
| T-1003 | `sprint-0: T-003` | `a3e6aec3afe739091d03103744a82d89ad1c467b` |
| T-1004 | `sprint-0: T-004` | `336b4e48e791f9a7d0a25e5de84c9404c3e266d2` |
| T-1005 | `sprint-0: T-005` | `99864da63fc9a51b24ead1d5792c4d6b7f706207` |
| T-1006 | `sprint-0: T-006` | `9517dc17797f25e7a2d8f924abf1b5d51fb62e5a` |
| T-1007 | `sprint-0: T-007` | `a7ed48992897c8463ba6cc729e944398c8ae8779` |

The incompatible bootstrap was `ef69c8f7df8eae8e445189c0c5b7ba7f3d747608`;
authority restoration is `b170e107d08ac1855d6b1be82fbf1ebe25a22f3a`.
Neither is attributed to a product Build task.

Every integrated legacy task commit also touched the now-removed
`agent-tasks/completed-tasks.md` legacy runtime ledger. That path is historical
process metadata, not part of the current task `Touches` contract. During Book
Build, `commit-task.sh` creates a new reconciliation task commit recorded in
each entry's normative `Commit` field; the table's legacy SHA is preserved in a
separate `Integrated implementation commit` field.

## Schema Tree

- Refresh the derivative ecosystem appendix without changing product authority
  - Current CubiKan description
    - T-1001: Correct current surfaces and version boundaries
    - T-1002: Correct capability status and authority maps
    - T-1003: Correct safe integration boundaries while preserving exclusions
  - Derivative catalog readiness
    - T-1004: Replace stale waits on realized primitives
    - T-1005: Preserve conditional repository-creation governance
    - T-1006: Correct sequencing, open questions, and status non-goals
  - Maintenance bookkeeping
    - T-1007: Close the exact backlog item once

## Execution Sequence

### T-1001: Correct the appendix's current CubiKan surfaces and version boundary

- **Intent:** [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- **Touches:** `docs/appendix/potential-derivative-projects.md`
- **Depends on:** (none)
- **Acceptance criterion:** The appendix accurately describes the current CubiKan boundary and distinguishes the Rust-only relationship/projection APIs from lifecycle-only local protocol v1.
- **Success criterion (EARS):**
  - **WHEN** a reader reviews the current CubiKan boundary and version matrix, **THEN** the appendix **SHALL** distinguish `cubikan-core`, stateless `cubikan`, durable `cubikan-backend`, and explicit-path `cubikan-local`; identify SQLite schemas v1/v2 and relationship/projection v1; and state that `cubikan-local` protocol v1 remains lifecycle-only while relationships/projections are Rust-only.
- **Notes:** Keep the stateless CLI distinct from the durable local process adapter; do not imply a network service.

### T-1002: Correct the appendix's capability status, dependency, and authority maps

- **Intent:** [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- **Touches:** `docs/appendix/potential-derivative-projects.md`
- **Depends on:** T-1001
- **Acceptance criterion:** Reusable gaps remain distinct intent chapters with current lifecycle states and one canonical authority per datum.
- **Success criterion (EARS):**
  - **WHEN** a reader reviews the layer, authority, capability, and dependency maps, **THEN** the appendix **SHALL** identify INT-0009, INT-0010, and INT-0012 as realized while keeping INT-0008 and INT-0011 proposed, with the Book, backend, external providers, and derivatives retaining their documented canonical authorities.
- **Notes:** Separate mixed rows where necessary; do not imply realized provenance or measurement evidence.

### T-1003: Correct the safe integration boundary while preserving current exclusions

- **Intent:** [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- **Touches:** `docs/appendix/potential-derivative-projects.md`
- **Depends on:** T-1002
- **Acceptance criterion:** Derivatives use a versioned local Rust boundary or pinned core without shared storage, provisional core serialization, or competing authority.
- **Success criterion (EARS):**
  - **WHEN** safe consumption paths are reviewed, **THEN** the appendix **SHALL** name the available versioned local Rust backend or explicitly pinned core as supported boundaries while keeping provider and network adapters separately governed and making no cross-version promise.
  - **WHEN** operational-authority transfer is reviewed, **THEN** the appendix **SHALL** keep the Book as semantic task/completion authority, scope backend authority to durable Intent Unit and accepted relationship state through versioned boundaries, prohibit dual-write, and require a separately authorized projection or migration intent before operational task/completion truth moves.
  - **WHEN** the advisory guardrails are reviewed, **THEN** the appendix **SHALL** prohibit direct database editing, shared writable backend storage, and provisional core-Serde contracts and avoid network, authentication, deployment, blockchain, or derivative-repository existence claims.
- **Notes:** Preserve Sprint 6 retained-theme provenance and distinguish the available Rust backend from future provider/network adapters.

### T-1004: Preserve the complete catalog while replacing stale waits on realized primitives

- **Intent:** [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- **Touches:** `docs/appendix/potential-derivative-projects.md`
- **Depends on:** T-1003
- **Acceptance criterion:** Every retained theme family and required entry field remains present, phase edges retain their lifecycle-only meaning, and prerequisites use current intent states.
- **Success criterion (EARS):**
  - **WHEN** the retained-theme inventory and catalog are reviewed, **THEN** the appendix **SHALL** retain all seven required theme families, classify each theme as backend, adapter, or derivative responsibility, prevent lifecycle phase edges from becoming delegation/provenance/relationship/pipeline edges, and give every repository entry an outcome, owned data and policy, CubiKan integration boundary, prerequisites, conditional creation trigger, and explicit non-goals.
  - **WHEN** a derivative catalog entry references revisions, durable lifecycle state, bounded lifecycle queries, relationships, or projections, **THEN** the entry **SHALL** describe INT-0009, INT-0010, and INT-0012 as available realized primitives rather than future prerequisites while leaving INT-0008, INT-0011, and derivative-specific adapters future where applicable.
- **Notes:** Review every catalog occurrence; do not mechanically replace capability-neutral uses of “future.”

### T-1005: Preserve conditional creation governance for every recommended repository

- **Intent:** [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- **Touches:** `docs/appendix/potential-derivative-projects.md`
- **Depends on:** T-1004
- **Acceptance criterion:** Repository slugs remain recommendations, and every entry retains ownership, prerequisite, security, compatibility, and creation-trigger boundaries.
- **Success criterion (EARS):**
  - **WHEN** any recommended repository entry is reviewed after the status refresh, **THEN** the appendix **SHALL** still require its own authorization, ownership, security, compatibility, and creation trigger without implying that a derivative repository exists or is scheduled.
  - **WHEN** Sprint 10 remote and publication actions are audited, **THEN** execution **SHALL** contain no derivative-repository create, push, publish, release, or deployment operation.
- **Notes:** This is a governance pass over advisory language, not a repository-creation action.

### T-1006: Correct global sequencing, open questions, and status non-goals

- **Intent:** [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- **Touches:** `docs/appendix/potential-derivative-projects.md`
- **Depends on:** T-1005
- **Acceptance criterion:** The appendix remains an advisory map whose unresolved questions and sequence reflect current intent states.
- **Success criterion (EARS):**
  - **WHEN** sequencing, roadmap, open-question, and document-non-goal sections are reviewed, **THEN** the appendix **SHALL** leave only still-unselected capabilities and derivative-owned policy unresolved instead of asking whether INT-0009, INT-0010, or INT-0012 exists or classifying those realized intents as proposed.
- **Notes:** Preserve open questions about security, compatibility, provenance, evidence, UI, deployment, and derivative policy.

### T-1007: Close the derivative-appendix refresh backlog item

- **Intent:** [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- **Touches:** `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Depends on:** T-1006
- **Acceptance criterion:** The bounded follow-on maintenance work is traceable without rewriting or re-realizing INT-0007.
- **Success criterion (EARS):**
  - **WHEN** T-1001 through T-1006 are complete, **THEN** the persistent Book backlog **SHALL** remove the exact INT-0007 maintenance item and the completed ledger **SHALL** record it once with its bounded documentation scope.
- **Notes:** Record T-1001 through T-1007 exactly once using `commit-task.sh`; each normative `Commit` is the new Book reconciliation commit and each `Integrated implementation commit` is the mapped legacy hash. Keep `MAINT-001` singular, replace its message resolver with exact integrated T-1007 hash `a7ed48992897c8463ba6cc729e944398c8ae8779`, and clarify INT-0007 as the originating realized backlog authority versus INT-0013 as the legal superseding reconciliation authority.
