Finalized - DO NOT EDIT

# Sprint 2 Build Plan

## Intents

- [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) — state: planned; acceptance criteria covered: one public 1 MiB raw-byte ceiling, bounded ceiling-plus-one ingestion, typed oversize rejection, preserved operational/result classifications, automated boundary/process verification, and explicit local-only documentation.
- [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) — state: realized; acceptance criteria preserved: strict version 1 one-shot scenarios, core-delegated lifecycle behavior, typed result responses, and actual-process E2E.

## Schema Tree

- Bound the existing one-shot CLI request without expanding platform scope
  - Public adapter contract
    - T-201: Define the 1 MiB request ceiling and oversize error vocabulary
  - Bounded ingestion
    - T-202: Read at most ceiling plus one byte before strict decoding
  - Boundary verification
    - T-203: Prove exact-limit, overflow, regression, and process behavior
  - Consumer contract
    - T-204: Document raw-byte accounting and retained exclusions

## Execution Sequence

### T-201: Define the request ceiling and typed oversize response contract

- **Intent:** [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md)
- **Touches:** `crates/cubikan-cli/src/lib.rs`, `crates/cubikan-cli/src/protocol.rs`
- **Depends on:** (none)
- **Acceptance criterion:** One documented public implementation constant defines the 1 MiB raw request ceiling; oversized input has one version 1 `request_too_large` response with no state.
- **Success criterion (EARS):**
  - **T-201-E1 — WHEN** a consumer inspects the `cubikan-cli` public API, **THEN** the adapter **SHALL** expose `MAX_REQUEST_BYTES` with the exact value `1_048_576` and the runner **SHALL** use that constant as its request ceiling.
  - **T-201-E2 — WHEN** the adapter serializes an oversized-request rejection, **THEN** the protocol **SHALL** emit a complete version 1 error envelope with code `request_too_large`, message `request exceeds maximum size of 1048576 bytes`, no field or operation number, and no Intent Unit snapshot.
  - **T-201-E3 — WHEN** the workspace dependency and source boundary is inspected, **THEN** Sprint 2 **SHALL** add no runtime dependency and **SHALL** leave `cubikan-core` source unchanged.
- **Notes:** Extend the experimental version 1 error-code set without changing its request shape. Do not add dependencies or core types.

### T-202: Implement ceiling-plus-one request ingestion before JSON decoding

- **Intent:** [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md)
- **Touches:** `crates/cubikan-cli/src/runner.rs`
- **Depends on:** T-201
- **Acceptance criterion:** The runner retains at most the ceiling plus one payload byte, preserves complete requests at or below the ceiling, rejects larger input before JSON classification, and keeps genuine I/O failures operational.
- **Success criterion (EARS):**
  - **T-202-E1 — WHEN** a reader reaches EOF at or below `MAX_REQUEST_BYTES`, **THEN** the runner **SHALL** pass the exact buffered bytes to the existing strict request decoder and preserve every current success, setup, and lifecycle result classification.
  - **T-202-E2 — WHEN** valid JSON places its required final root `}` at byte `MAX_REQUEST_BYTES`, **THEN** the runner **SHALL** retain, decode, and execute that complete exact-limit request without truncation.
  - **T-202-E3 — WHEN** the reader yields byte `MAX_REQUEST_BYTES + 1`, including after a malformed prefix, **THEN** the runner **SHALL** consume no additional byte, skip JSON classification and later reader errors, and write one `request_too_large` request rejection without state.
  - **T-202-E4 — WHEN** bounded ingestion encounters a read error before retaining the overflow byte or any modeled response output is incomplete, **THEN** the runner **SHALL** return `RunError::Read(io::Error)`, `WriteResponse`, or `WriteNewline` as applicable rather than a modeled result.
- **Notes:** Compose `Read::take` with bounded payload buffering, then use `serde_json::from_slice`. The public `RunError::Read` payload deliberately changes from `serde_json::Error` to `io::Error`; preserve response writing and `RunStatus` meanings.

### T-203: Add public-seam and actual-process boundary coverage

- **Intent:** [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md)
- **Touches:** `crates/cubikan-cli/tests/runner.rs`, `crates/cubikan-cli/tests/cli_e2e.rs`
- **Depends on:** T-202
- **Acceptance criterion:** Automated tests cover below/exact/one-byte-over/malformed-overflow and I/O behavior; an actual process proves the oversized JSON response and exit status.
- **Success criterion (EARS):**
  - **T-203-E1 — WHEN** the public `run` seam receives valid requests below and exactly at the ceiling, **THEN** integration tests **SHALL** observe the same completed version 1 result and exactly one response line.
  - **T-203-E2 — WHEN** the Cargo-built `cubikan` process receives a request one byte over the ceiling, **THEN** process E2E **SHALL** observe exit `2`, empty stderr, exactly one newline-terminated `request_too_large` response, and no Intent Unit snapshot.
  - **T-203-E3 — WHEN** the full existing CLI and core suites run after bounded ingestion is added, **THEN** every prior success, malformed/setup rejection, lifecycle rejection, operational failure, core invariant, and doctest **SHALL** remain green.
- **Notes:** Construct exact-size fixtures by removing the compact request's final root `}`, inserting JSON whitespace, and restoring `}` as the exact boundary byte; use the same shape with `}` at byte `MAX + 1` for overflow. Continue using `std::process::Command`; do not add a process-test dependency.

### T-204: Document the bounded local ingestion contract

- **Intent:** [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md)
- **Touches:** `README.md`, `crates/cubikan-cli/README.md`, `docs/intents/INT-0002-runnable-lifecycle-adapter.md`
- **Depends on:** T-203
- **Acceptance criterion:** Consumer documentation explains raw-byte counting, the exact ceiling/error code, and all retained platform and policy exclusions.
- **Success criterion (EARS):**
  - **T-204-E1 — WHEN** a consumer reads the root and CLI guides, **THEN** the documentation **SHALL** state the 1 MiB (`1_048_576` byte) ceiling, that whitespace counts, overflow code/exit behavior, and that the compile-time source value is local hardening rather than a runtime setting, total-memory bound, or production readiness.
  - **T-204-E2 — WHEN** the bounded adapter scope is reviewed, **THEN** the documentation **SHALL** continue to exclude persistence, sessions, networking, deployment, authorization, concurrency, UI, blockchain, timeouts, rate limits, concurrent-client quotas, and stable protocol compatibility.
  - **T-204-E3 — WHEN** INT-0002's realized consequences are reviewed after the follow-on is planned, **THEN** the intent **SHALL** describe unbounded input as historical Sprint 1 state and link INT-0003 without changing INT-0002's realized acceptance boundary.
  - **T-204-E4 — WHEN** the Project Book validator runs, **THEN** INT-0003, its research and plans, the INT-0002 follow-on link, and navigation **SHALL** satisfy Book v2 structure.
- **Notes:** Replace the obsolete unbounded-input disclosure in consumer docs while preserving historical truth in INT-0002; do not imply stable protocol compatibility.
