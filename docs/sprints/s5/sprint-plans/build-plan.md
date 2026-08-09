Finalized - DO NOT EDIT

# Sprint 5 Build Plan

## Intents

- [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md) — state: planned; acceptance criteria covered: absent-versus-present ID decoding, rejection of explicit `null` and every other non-string JSON value, preserved omitted/fixed/malformed-string identity semantics, public runner and actual-process behavior, consumer documentation, and no dependency/core/output-contract drift.
- [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) — state: realized and preserved; affected criterion: the new structural rejection returns a modeled result only after one response newline and one successful supplied-writer flush, with output-error precedence unchanged.
- [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) — state: realized and preserved; affected criterion: overflow remains classified before JSON shape, and process/request error meanings remain unchanged.
- [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) — state: realized and preserved; affected criterion: strict version 1 request handling, adapter-owned responses, generated/fixed IDs, lifecycle execution, and actual-process exits remain satisfied.
- [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) — state: realized and preserved; affected criterion: supplied UUID parsing, local UUID v4 generation, and all domain behavior remain unchanged.
- [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) — state: realized and preserved; affected criterion: the existing five-gate local and hosted Rust quality boundary validates Sprint 5 without workflow changes.

## Schema Tree

- Make omission the only CLI request for generated identity
  - Wire decoding
    - T-501: Distinguish an absent ID member from every present JSON value and correct the internal omission fixture
  - Public adapter boundary
    - T-502: Prove generated, supplied, malformed, and explicit-null runner behavior
  - Actual process boundary
    - T-503: Prove omitted and explicit-null process exits
  - Consumer boundary
    - T-504: Document the ID-presence contract and explicit nonclaims

## Execution Sequence

### T-501: Distinguish absent and present ID values in the version 1 decoder

- **Intent:** [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md)
- **Touches:** `crates/cubikan-cli/src/protocol.rs`, `crates/cubikan-cli/src/lib.rs` (`#[cfg(test)]` fixture only)
- **Depends on:** (none)
- **Acceptance criterion:** The wire decoder preserves true omission as `None`, preserves a present string as `Some`, and rejects `null` and every other non-string value as a structural data error without weakening existing request strictness.
- **Success criterion (EARS):**
  - **T-501-E1 — WHEN** a valid version 1 request omits `intent_unit.id`, **THEN** the protocol decoder **SHALL** produce `None`; **WHEN** the member contains a JSON string, **THEN** it **SHALL** preserve that string exactly as `Some`.
  - **T-501-E2 — WHEN** `intent_unit.id` is present as `null`, a Boolean, a number, an array, or an object, **THEN** the protocol decoder **SHALL** reject the request as a Serde data/shape error rather than normalize the value to omission.
  - **T-501-E3 — WHEN** required `intent_unit.species` is absent or an unknown member is present at an existing strict root, workflow, edge, Intent Unit, transition, or completion DTO boundary, **THEN** the protocol decoder **SHALL** retain its existing rejection behavior while `intent_unit.id` alone may be absent.
  - **T-501-E4 — WHEN** in-crate process-shell tests use their otherwise valid request after the decoder correction, **THEN** the fixture **SHALL** express generation by omitting the ID member, decode with `id == None`, and retain the existing read, newline, flush, and diagnostic-write exit-`1` behavior.
- **Notes:** Add a field-specific deserializer with `#[serde(default, deserialize_with = ...)]`: the default handles a missing member, while the helper must deserialize a present `String` and wrap it in `Some`. Do not deserialize through `Option<String>` inside the helper, because that would retain the `null` alias. Remove the `id` member from the `#[cfg(test)]` process-shell fixture in the same task so T-501 leaves a coherent green intermediate commit. The first task commit also records the initialized Sprint 5 Book, finalized plans, and task ledger required by the task helper. Commit as `sprint-5: T-501 distinguish omitted and present CLI IDs`.

### T-502: Prove the public runner identity boundary

- **Intent:** [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md), preserving [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md), [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md), and [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md)
- **Touches:** `crates/cubikan-cli/tests/runner.rs`
- **Depends on:** T-501
- **Acceptance criterion:** Public `run` exposes omission as successful generated identity, explicit `null` as a flush-checked structural request rejection without state, and string values through the existing fixed/malformed UUID taxonomy while preserving overflow precedence.
- **Success criterion (EARS):**
  - **T-502-E1 — WHEN** public `run` receives a valid wire request with `intent_unit.id` absent, **THEN** it **SHALL** return `RunStatus::Success` and one success snapshot whose ID parses as a non-nil UUID v4.
  - **T-502-E2 — WHEN** public `run` receives `null`, a Boolean, a number, an array, or an object as a present `intent_unit.id`, **THEN** it **SHALL** return `RunStatus::RequestRejected` only after writing one newline-terminated response and flushing the supplied writer exactly once; every response **SHALL** contain protocol version `1`, code `invalid_request`, a nonempty message, and no `intent_unit`, `field`, or `operation_number`.
  - **T-502-E3 — WHEN** public `run` receives a valid fixed UUID string, **THEN** it **SHALL** preserve that value; **WHEN** it receives malformed UUID text, **THEN** it **SHALL** retain code `invalid_intent_unit_id`, field `intent_unit.id`, no state, and request-rejection status.
  - **T-502-E4 — WHEN** an input containing explicit-null ID syntax exceeds `MAX_REQUEST_BYTES`, **THEN** public `run` **SHALL** retain `request_too_large` classification before JSON shape classification and consume no more than the existing ceiling-plus-one boundary.
- **Notes:** Use a test-owned recording writer at the public crate seam to count the explicit-null response newline and supplied-writer flush. Do not pin Serde's human message text. Parse generated IDs through `cubikan_core::IntentUnitId`, which is already a direct CLI dependency; do not add a direct UUID dependency. Commit as `sprint-5: T-502 prove the public ID boundary`.

### T-503: Prove the actual-process identity boundary

- **Intent:** [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md), preserving [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md), [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md), and [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md)
- **Touches:** `crates/cubikan-cli/tests/cli_e2e.rs`
- **Depends on:** T-502
- **Acceptance criterion:** The Cargo-built binary makes omission and explicit `null` observably distinct while its operational shell, exact ordinary lifecycle output, and established exit meanings remain unchanged.
- **Success criterion (EARS):**
  - **T-503-E1 — WHEN** a valid request that omits `intent_unit.id` is written to the Cargo-built `cubikan` process, **THEN** the process **SHALL** exit `0`, write exactly one newline-terminated success JSON document, leave stderr empty, and expose a non-nil UUID v4.
  - **T-503-E2 — WHEN** a request with explicit JSON `null` for `intent_unit.id` is written to the Cargo-built process, **THEN** the process **SHALL** exit `2`, leave stderr empty, and write exactly one newline-terminated version 1 `invalid_request` response with a nonempty message and no `intent_unit`, `field`, or `operation_number`.
  - **T-503-E3 — WHEN** the existing fixed-ID lifecycle, malformed-JSON, lifecycle-rejection, and oversized-request actual-process cases run, **THEN** their exact JSON, stderr, and exit `0`, `2`, or `3` behavior **SHALL** remain unchanged.
- **Notes:** Remove the ID member from a parsed test request with object-member removal; assigning JSON `null` does not model omission. Do not mutate the checked-in fixed-ID lifecycle fixture; it remains the exact preservation oracle. Commit as `sprint-5: T-503 prove actual-process ID semantics`.

### T-504: Document the ID-presence contract and preserve scope

- **Intent:** [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md), preserving [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md), [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md), [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md), [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md), and [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md)
- **Touches:** `crates/cubikan-cli/README.md`
- **Depends on:** T-503
- **Acceptance criterion:** Consumers can unambiguously request generated identity by omission and distinguish structural non-string failures from malformed UUID strings, without any broader compatibility or product claim.
- **Success criterion (EARS):**
  - **T-504-E1 — WHEN** a consumer reads the version 1 request contract, **THEN** the guide **SHALL** identify `intent_unit.id` as the sole optional member, state that absence requests a generated non-nil UUID v4, and state that every present value must be a JSON string and that `null` is invalid.
  - **T-504-E2 — WHEN** a consumer compares ID failure classes, **THEN** the guide **SHALL** classify present non-string values as structural `invalid_request` and malformed UUID strings as `invalid_intent_unit_id` with field `intent_unit.id`, without promising exact human diagnostic prose.
  - **T-504-E3 — WHEN** the completed Sprint 5 diff and Project Book are validated, **THEN** they **SHALL** show no dependency, `cubikan-core`, workflow, protocol-version, response-shape, error-code, request-ceiling, output-precedence, or unrelated product-policy change, and the installed Book validator **SHALL** report six reachable intent chapters.
- **Notes:** Keep the protocol experimental and retain all one-shot, persistence, networking, UI, blockchain, and stable-compatibility nonclaims. The existing CI definition is unchanged; Test will require its local five gates and a successful hosted `dev` run for the exact committed Build head before realization. Commit as `sprint-5: T-504 document the CLI ID presence contract`.
