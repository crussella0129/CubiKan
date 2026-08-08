# Architectural Decisions

## 2026-08-08 — Start with a chain-agnostic Rust core (sprint 0)
- **Context:** CubiKan has product language but no implementation, and the repository does not identify a blockchain, persistence layer, service boundary, or UI platform.
- **Decision:** Implement the first executable behavior as a `cubikan-core` Rust library containing only domain vocabulary and lifecycle invariants.
- **Alternatives considered:** Building an Ethereum contract or Electron application first was rejected because either would commit unresolved platform and product semantics before the lifecycle is testable.
- **Consequences:** The core remains reusable by later adapters; blockchain, persistence, API/CLI, and Electron integration require separate researched decisions.

## 2026-08-08 — Store caller-declared workflow snapshots on Intent Units (sprint 0)
- **Context:** Phase topology, reverse movement, completion eligibility, and custom/KPI-related phase names are product-specific, while workflow IDs alone cannot disambiguate two definitions with different topology.
- **Decision:** Callers declare directed phases, edges, and completion-eligible phases. Each Intent Unit owns an immutable validated snapshot of the workflow used to create it.
- **Alternatives considered:** Exported default Kanban phases would invent policy, and looking up mutable definitions in a registry would make lifecycle validation depend on infrastructure and workflow versioning that do not yet exist.
- **Consequences:** Lifecycle operations are deterministic and self-contained. Snapshot migration, workflow versioning, KPI evaluation, and registry-backed sharing remain future concerns.

## 2026-08-08 — Use opaque UUID v4 Intent Unit identifiers (sprint 0)
- **Context:** The README requires unique IDs but does not require chronological ordering, content addressing, or a blockchain-native representation.
- **Decision:** Represent `IntentUnitId` as an immutable domain newtype backed by UUID v4, with generation, parsing, formatting, and fixed-value construction.
- **Alternatives considered:** UUID v7 would introduce an unneeded ordering/time signal; chain addresses and content hashes would prematurely select persistence or network semantics.
- **Consequences:** IDs can be generated without a central allocator and their representation is isolated behind a domain API. No ordering guarantee is part of the contract, and future external IDs require an explicit migration or mapping decision.

## 2026-08-08 — Validate every deserialization boundary (sprint 0)
- **Context:** Derived deserialization can bypass private constructors and create workflows or lifecycle histories that violate domain invariants.
- **Decision:** Route deserialization through the same validation used by ordinary construction, including topology, history ordering, transition continuity, and completion consistency checks.
- **Alternatives considered:** Unchecked derives were rejected as unsafe for domain state; omitting serialization entirely would delay a small, format-neutral boundary useful to every future adapter.
- **Consequences:** Invalid serialized state is rejected. JSON is only a test vehicle, and field names/layout are provisional rather than a stable wire-format promise.

## 2026-08-08 — Defer unresolved product policy (sprint 0)
- **Context:** The repository does not define default phases, KPI evaluation, completed-unit naming syntax, parent/child lineage, ownership, authorization, concurrency, privacy, or audit-proof requirements.
- **Decision:** Sprint 0 preserves immutable species provenance and caller-declared structural workflow policy without implementing or implying the unresolved semantics.
- **Alternatives considered:** Guessing conventional Kanban, naming, lineage, or authorization behavior was rejected because multiple valid interpretations exist and would become externally visible contracts.
- **Consequences:** Later research must define these policies before implementation. Sprint 0 history is an in-memory domain record, not durable storage or cryptographic proof.
