Finalized - DO NOT EDIT

# Sprint 3 Build Plan

## Intents

- [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) — state: planned; acceptance criteria covered: explicit supplied-writer flush before modeled status, typed flush error preservation, first-error precedence, five-branch response coverage, process exit mapping, and precise consumer documentation.
- [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) — state: realized and preserved; affected criterion: genuine output failures remain operational exit `1`, including the existing oversized response path.
- [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) — state: realized and preserved; affected criterion: one-response process behavior and existing actual-process lifecycle outcomes remain unchanged.
- [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) — state: realized and preserved; no core or domain behavior changes.

## Schema Tree

- Return modeled CLI outcomes only after the supplied writer accepts flush
  - Runner contract
    - T-301: Implement typed flush failure and body → newline → flush ordering
  - Public and process verification
    - T-302: Prove buffered drain failure and exit-1 process mapping
  - Consumer boundary
    - T-303: Document writer-flush checking and retained nonclaims

## Execution Sequence

### T-301: Implement the typed supplied-writer flush contract

- **Intent:** [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md)
- **Touches:** `crates/cubikan-cli/src/runner.rs`
- **Depends on:** (none)
- **Acceptance criterion:** The centralized response path performs JSON serialization, newline writing, and exactly one supplied-writer flush in order; it exposes a typed flush error and deterministic first-error precedence before returning any modeled status.
- **Success criterion (EARS):**
  - **T-301-E1 — WHEN** body serialization and newline writing succeed for success, malformed-JSON request rejection, setup rejection, lifecycle rejection, or oversized rejection, **THEN** the centralized response path **SHALL** call the supplied writer's `flush` exactly once after the newline and return the existing `RunStatus` only after that call returns `Ok`.
  - **T-301-E2 — WHEN** the supplied writer accepts the JSON body and newline but its flush returns `io::Error`, **THEN** public `RunError` **SHALL** return `FlushResponse(io::Error)` with exact diagnostic `failed to flush response: {source}` and preserve the source kind and message without returning a modeled status.
  - **T-301-E3 — WHEN** response body serialization/write fails or the later newline write fails, **THEN** the runner **SHALL** return `WriteResponse` or `WriteNewline` respectively and **SHALL NOT** attempt any subsequent newline or flush step.
  - **T-301-E4 — WHEN** the supplied writer's flush succeeds, **THEN** the response JSON, terminating newline, request-size precedence, existing `RunStatus`, and core-delegated behavior **SHALL** remain unchanged.
- **Notes:** Add `RunError::FlushResponse(io::Error)` plus exhaustive `Display` and `Error::source` arms. In `write_response`, keep `serde_json::to_writer` and `write_all(b"\n")`, then call `writer.flush().map_err(RunError::FlushResponse)?` immediately before `Ok(status)`. Adding a public enum variant intentionally changes exhaustive matching for this unpublished experimental Rust API; do not add `#[non_exhaustive]`, retries, response buffering, protocol codes, or exit changes. Pair implementation with colocated trace/error-precedence tests. Commit as `sprint-3: T-301 enforce supplied-writer response flush`.

### T-302: Prove the public buffered-writer and process-shell boundaries

- **Intent:** [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md)
- **Touches:** `crates/cubikan-cli/tests/runner.rs`, `crates/cubikan-cli/tests/cli_e2e.rs`, `crates/cubikan-cli/src/lib.rs`
- **Depends on:** T-301
- **Acceptance criterion:** A realistic buffered drain failure is observable through the public error API, maps to process exit `1`, and ordinary actual-process exits and response streams remain unchanged.
- **Success criterion (EARS):**
  - **T-302-E1 — WHEN** public `run` writes a small response through `std::io::BufWriter` whose underlying sink rejects the first drain write, **THEN** buffering the body/newline **SHALL** succeed while explicit flushing **SHALL** return public `FlushResponse(io::Error)` with the fixed kind, message, display, and source rather than a modeled status.
  - **T-302-E2 — WHEN** `run_process` receives a stdout writer that accepts body/newline and fails only on its one flush, **THEN** it **SHALL** return exit `1`, emit exactly `cubikan: failed to flush response: fixture flush failure\n` to a working stderr writer, retain exit `1` if that best-effort diagnostic writer fails, and never map the failure to exit `0`, `2`, or `3`.
  - **T-302-E3 — WHEN** the existing Cargo-built executable cases run with ordinary standard streams, **THEN** success exit `0`, malformed/oversized exits `2`, lifecycle exit `3`, their exact JSON/newline output, and empty modeled-result stderr **SHALL** remain unchanged.
- **Notes:** The integration fixture must use a real `BufWriter` with capacity larger than the small response over a sink that fails while draining, then disassemble it without relying on drop-time behavior. Do not assert that flush failure rolls back already accepted bytes. Strengthen the success, malformed, and lifecycle process tests to compare complete response JSON values; the oversize test already does so. A portable actual child-process flush-only failure cannot be isolated without test hooks or platform-specific sinks; negative proof belongs at the public `Write` and injected process-shell seams, while the four actual-process tests remain exact E2E regression evidence. Commit as `sprint-3: T-302 prove buffered flush failure handling`.

### T-303: Document the writer-flush-checked response boundary

- **Intent:** [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md)
- **Touches:** `README.md`, `crates/cubikan-cli/README.md`
- **Depends on:** T-302
- **Acceptance criterion:** Consumer documentation describes the exact supplied-writer flush and operational failure behavior without implying stronger delivery, durability, or compatibility guarantees.
- **Success criterion (EARS):**
  - **T-303-E1 — WHEN** a consumer reads the CLI output and exit contract, **THEN** the guides **SHALL** state that the runner serializes JSON, writes one newline, calls the supplied writer's flush once, and returns a modeled status only after those steps succeed; a flush error remains operational exit `1` with a best-effort stderr diagnostic.
  - **T-303-E2 — WHEN** the documented guarantee boundary is reviewed, **THEN** the guides **SHALL** explicitly deny stream atomicity/rollback, durable `fsync`, OS delivery or close success, persistence, retries, and external-reader/network acknowledgement while retaining the experimental no-cross-version-compatibility boundary.
  - **T-303-E3 — WHEN** the Sprint 3 repository diff and dependency graph are reviewed, **THEN** the sprint **SHALL** add no dependency and change no `cubikan-core` source, protocol request/response DTO, JSON error code/shape, process exit meaning, or realized intent semantics.
  - **T-303-E4 — WHEN** the Sprint 3 Project Book is validated after INT-0004 and its sprint evidence are linked, **THEN** the installed validator **SHALL** report a valid Book v2 with four reachable intent chapters.
- **Notes:** Keep the root guide concise and put detailed operational semantics in the CLI guide. Do not claim guaranteed stderr delivery, output transactionality, single-write serialization, signal/broken-pipe policy, timeouts, output-size limits, stable `RunError` compatibility, persistence, service, UI, or blockchain behavior. Commit as `sprint-3: T-303 document explicit response flushing`.
