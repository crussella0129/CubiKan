# INT-0013 — Maintain derivative ecosystem current-state accuracy

<!-- sprint-loop-intent-v2 -->
- **Intent ID:** INT-0013
- **State:** superseded
- **Work evidence:** [Sprint 10 build plan](../sprints/s10/sprint-plans/build-plan.md)
- **Completion evidence:** [T-1001–T-1007 completion ledger](../work/completed-tasks.md#t-1001-sprint-10)
- **Code evidence:** none
- **Test evidence:** [Sprint 10 test report](../sprints/s10/sprint-tests/test-report.md)
- **Documentation evidence:** [maintained derivative-ecosystem appendix](../appendix/potential-derivative-projects.md)

## Intent

Maintain the superseded INT-0007 advisory derivative-ecosystem appendix as an
accurate current-state map when CubiKan capabilities advance. Synchronize its
present-tense surface, version, capability-state, prerequisite, sequencing, and
open-question language with the authoritative Book and current operator guides
without rewriting terminal INT-0007 history.

This is a documentation-maintenance outcome. It does not change runtime
behavior, re-realize an existing capability, create or authorize a derivative
repository, select an adapter or deployment, or turn the appendix into a
roadmap. Historical retained-theme provenance remains historical; current
claims use current Book states.

## Acceptance criteria

- The appendix accurately distinguishes `cubikan-core`, stateless `cubikan`,
  durable `cubikan-backend`, and explicit-path `cubikan-local`; names SQLite
  schema v1/v2 and relationship/projection contract v1; and states that local
  protocol v1 remains lifecycle-only while relationship/projection operations
  are Rust-only.
- INT-0009, INT-0010, and INT-0012 are described as realized and available,
  while INT-0008 and INT-0011 remain proposed. Capability, dependency, and
  authority maps retain one canonical owner for Book meaning, backend state,
  external-provider facts, and derivative policy.
- Every retained theme family remains classified as a backend capability,
  adapter concern, or derivative responsibility, and lifecycle phase edges are
  not presented as delegation, provenance, relationship, or pipeline edges.
  The catalog continues to cover manager/doer operations, Git and intent
  provenance, agent analytics, process/KPI design, multi-board skill graphs,
  organizational applications, and Sprint Loops/Animus accounting.
- Every recommended repository entry retains its outcome, owned data and
  policy, CubiKan integration boundary, prerequisites, conditional creation
  trigger, and explicit non-goals. Names remain recommendations; no entry is
  represented as existing, scheduled, or authorized.
- Safe integration names the available versioned local Rust backend or an
  explicitly pinned core as supported consumption paths, keeps provider and
  network adapters separately governed, prohibits shared writable storage and
  provisional core-Serde contracts, and makes no cross-version promise. The
  Book remains semantic task/completion authority, while the backend is
  authoritative only for durable Intent Unit and accepted relationship state
  through its versioned boundaries. Dual-write is prohibited, and operational
  task/completion truth moves only through a separately authorized projection
  or migration intent.
- Sequencing and open questions no longer ask whether realized revision,
  durability, relationship, or projection foundations will exist. Still-open
  provenance, evidence, compatibility, security, authorization, UI,
  deployment, and derivative-policy questions remain explicit.
- Local links, fragments, Book navigation, and all 12 pre-existing intent
  chapters validate. The accepted-base product diff is documentation-only;
  Rust, manifests, lockfile, CI, remote profile, and other intent semantics stay
  unchanged. The exact maintenance backlog item moves once to completion, and
  sprint execution performs no derivative-repository create, push, publish,
  release, or deployment operation.

## Rationale

INT-0007 was correctly realized against the Sprint 6 tree, then became
historically stale as INT-0009, INT-0010, and INT-0012 were realized. Rewriting
that terminal chapter would violate Book lifecycle rules, while leaving the
appendix stale would make it unsafe planning input. A follow-on maintenance
intent preserves terminal history and defines the observable current-state
contract separately.

## Alternatives

Rewriting realized INT-0007 was rejected because terminal intent chapters are
immutable except for supersession. Pinning only its documentation link while
leaving its live “current boundary” semantics authoritative was rejected
because it would leave competing current-state contracts. Legal supersession
preserves its original catalog outcome while assigning later maintenance here.
Beginning INT-0008 or INT-0011 realization was rejected as materially different
work with unresolved product policy.

## Consequences

The appendix becomes useful current planning context again, but remains
advisory and may need another follow-on maintenance intent after future
capability changes. This work adds Book provenance and documentation diff only;
it creates no compatibility, network, security, deployment, or derivative
maintenance promise.

## Transition history

- 2026-08-10: created as `proposed` after the post-Sprint 9 audit found INT-0007's appendix still described realized INT-0009, INT-0010, and INT-0012 capabilities as future.
- 2026-08-10: moved to `planned` when Sprint 10 mapped the bounded current-state correction and non-creation safeguards to T-1001 through T-1007.
- 2026-08-10: moved to `active` when Build queued T-1001 through T-1007 from the finalized clean Sprint 10 plans.
- 2026-08-11: moved to `realized` after T-1001–T-1007 completed, all 15 finalized Sprint 10 checks, 191 workspace tests, and one doctest passed, GitHub Actions run 31533101690 succeeded at exact tested commit `0a7bc3a023364cca9197e735c5acfeab019ce8a1`, and the final Test Critic returned `clean`.
- 2026-08-11: moved from `realized` to `superseded` when planned [INT-0014](INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md) became the current-state authority for the blockchain-canonical generation and its required appendix/documentation reconciliation; the bounded Sprint 10 maintenance outcome and evidence remain historical.
