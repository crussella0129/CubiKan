# Sprint 0 Research Report

## Intents Reviewed

- [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) — created retrospectively during Book v2 migration from this Sprint 0 evidence; relevance: defines the chain-agnostic lifecycle outcome researched and delivered by the sprint; current state: `realized`.

## Legacy Decision Context

`decisions.md` contains no ADR entries yet, so there are no prior architectural
decisions that constrain this sprint. No prior decision is being violated.

## 1. Sprint Goal

Establish CubiKan's first executable, chain-agnostic foundation in Rust: define
the core Intent Unit vocabulary and invariants, implement validated workflow
transitions and completion/species provenance in a small domain crate, and prove
the behavior with focused automated tests. Sprint 0 should leave blockchain,
network, persistence, and UI choices behind explicit interfaces or documented
deferrals so later work does not have to unwind an unverified platform choice.

## 2. Existing Code Survey

| File | Relevance | Notes |
|------|-----------|-------|
| `README.md` | high | The only product specification: an Intent Unit has a unique ID, occupies a predefined or custom/KPI-driven phase, and preserves species/lineage information when completed. |
| `LICENSE` | medium | Establishes MIT licensing for future source files and package metadata. |
| `.gitignore` | medium | Newly created Sprint Loop policy keeps ephemeral `sprints/` state untracked while preserving the long-term task and decision ledgers. |
| `agent-tasks/agent-tasks.md` | medium | Newly initialized persistent backlog; it has no implementation tasks yet and must be populated from the finalized Sprint 0 plan. |
| `decisions.md` | high | Newly initialized ADR log; it has no entries, so Sprint 0 should record the first domain and architecture decisions before implementation commits. |

The tracked history contains only two commits: the initial README/LICENSE and a
rename from BloKan to CubiKan. There is no source tree, manifest, test suite,
CI configuration, established blockchain, persistence model, or UI stack.
Local Rust tooling is available (`rustc 1.95.0`, `cargo 1.95.0`).

## 3. External Sources

- [Workspaces — The Cargo Book](https://doc.rust-lang.org/cargo/reference/workspaces.html) — a virtual workspace can keep the core domain separate now while allowing later blockchain, service, and desktop adapters to become independent members.
- [How to Write Tests — The Rust Programming Language](https://doc.rust-lang.org/stable/book/ch11-01-writing-tests.html) — Rust's built-in test harness supports direct setup/transition/assertion tests for lifecycle invariants without adding a test framework.
- [`uuid` crate documentation](https://docs.rs/uuid/latest/uuid/) — UUIDs provide decentralized unique identifiers; version 7 is time-sortable, while a domain newtype can prevent the storage representation from leaking through the API.
- [Using derive — Serde](https://serde.rs/derive.html) — deriving format-neutral serialization on domain values allows future persistence, API, and chain adapters to share one stable representation.
- [Introduction to smart contracts — ethereum.org](https://ethereum.org/developers/docs/smart-contracts/) — on-chain interactions are costly or irreversible and contracts cannot directly retrieve off-chain state, supporting the decision to validate domain semantics before selecting or deploying a blockchain implementation.

## 4. Risks, Unknowns, Dependencies

- **Risk:** "Blockchain" could mean a public smart-contract network, a private/permissioned ledger, or simply an append-only verifiable event history. Choosing one in Sprint 0 would harden an unsupported assumption into the domain model.
- **Risk:** Identifier format is an externally visible contract. A typed `IntentUnitId` should isolate an initial UUID representation so a later chain-specific address or content hash does not infect domain APIs.
- **Risk:** Directly mutable structs could bypass phase and completion invariants. Construction and transition operations should validate state changes and return explicit errors.
- **Unknown:** The allowed phase graph, whether transitions can move backward, and whether KPI values authorize transitions are unspecified.
- **Unknown:** "Species," derivation, and completed-unit naming have no formal grammar. Sprint 0 can preserve immutable species provenance, but should not invent parent/child lineage or a permanent naming convention without product input.
- **Unknown:** Ownership, authorization, multi-user conflict resolution, audit visibility, and privacy requirements are absent from the repository.
- **Dependency:** The first build will need a Cargo workspace and a core library crate, plus narrowly scoped `uuid` and `serde` dependencies; fetching crates may require network access if they are not cached.
- **Dependency:** Blockchain, persistence, service/API, and Electron UI adapters depend on the core lifecycle contract and are intentionally deferred.

## 5. Recommended Approach

Primary: create a virtual Cargo workspace with one `cubikan-core` library crate.
Model opaque IDs, intent species, workflow/phase identifiers, allowed transition
edges, Intent Unit state, completion metadata, and an append-only transition
record. Keep fields private where mutation could violate invariants. Expose
constructors and transition/completion methods that return typed errors, derive
format-neutral serialization, and add unit/integration tests for valid paths,
invalid edges, terminal-state behavior, ID stability, species preservation, and
serialization round trips. Document the vocabulary and Sprint 0 exclusions in
the README, and record the foundational choices in `decisions.md`.

Alternative considered: implement the first workflow directly as an Ethereum
smart contract and build an Electron board against it. This would demonstrate
the blockchain/UI vision earlier, but it would also commit to public-chain cost,
irreversibility, off-chain integration rules, and visual/product semantics that
the repository does not define. A chain-agnostic Rust kernel is smaller,
AI-verifiable, and reusable from a later contract adapter, service, or desktop
application.

Rationale: the recommendation delivers a testable vertical slice of the only
stable requirements in the repository while keeping every unresolved platform
choice reversible. A workspace adds little initial complexity and provides a
clean boundary for the adapters the product will eventually require.

## Artifacts

- No additional evidence artifacts were saved; all repository evidence is cited
  in the survey and every external source is linked above.
