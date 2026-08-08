# Sprint 2 Research Report

## Intents Reviewed

- [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) — created; relevance: defines the bounded request-ingestion outcome selected for Sprint 2; current state: `proposed`.
- [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) — revised; relevance: supplies the realized one-shot protocol and now points its historical unbounded-input consequence to follow-on INT-0003; current state: `realized`.
- [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) — selected; relevance: supplies the already-realized domain invariants that Sprint 2 must leave unchanged; current state: `realized`.

## 1. Sprint Goal

Bound the existing `cubikan` process's known unbounded raw-input retention risk
without changing its domain or platform scope. Sprint 2 will cap the complete
raw standard-input request at 1 MiB, detect overflow after reading no more than
one additional byte, return a deterministic version 1 `request_too_large`
rejection, and prove that every realized success, failure, and process-exit
boundary remains intact. The value is a reversible source-level engineering
guardrail for the local experimental adapter, not a runtime setting, total
memory bound, domain policy, or production-readiness.

## 2. Existing Code Survey

| File | Relevance | Notes |
|------|-----------|-------|
| `README.md` | high | States that the CLI is one-shot/in-memory and that resource limiting is required before production network exposure. |
| `docs/intents/INT-0001-chain-agnostic-intent-lifecycle-core.md` | medium | Keeps lifecycle semantics, persistence, networking, and product policy outside this adapter-only hardening. |
| `docs/intents/INT-0002-runnable-lifecycle-adapter.md` | high | Realizes the version 1 process boundary and explicitly records unbounded stdin as deferred hardening. |
| `docs/sprints/s1/sprint-research/research-report.md` | high | Identifies unbounded local input as a risk while finding no evidence for persistence, service, Electron, or blockchain choices. |
| `docs/sprints/s1/sprint-tests/test-report.md` | high | Confirms the unbounded-input issue remained after all Sprint 1 acceptance criteria passed. |
| `crates/cubikan-cli/src/runner.rs` | high | Deserializes directly from an arbitrary `Read`, classifies JSON failures, and owns response writing. |
| `crates/cubikan-cli/src/lib.rs` | high | Maps runner classifications to process exits 0/2/3 and operational failures to exit 1. |
| `crates/cubikan-cli/src/protocol.rs` | high | Owns the exhaustive experimental error-code vocabulary and version 1 response shape. |
| `crates/cubikan-cli/src/execution.rs` | medium | Delegates setup/lifecycle rules to the core; no change is required for request-size rejection. |
| `crates/cubikan-cli/tests/runner.rs` | high | Exercises the public reader/writer seam and can prove exact-limit and overflow behavior without external services. |
| `crates/cubikan-cli/tests/cli_e2e.rs` | high | Spawns the actual Cargo-built executable and can verify oversized stdin, stdout, stderr, and exit status. |
| `crates/cubikan-cli/README.md` | high | Documents the complete protocol/error/exit contract and currently discloses unbounded input. |
| `Cargo.toml` | medium | The change requires no new crate or dependency. |

The accepted code has no persistence, service, desktop, chain, or concurrent
client boundary. The checked-in successful request is substantially smaller
than 1 MiB; the selected ceiling therefore bounds allocation without
constraining the only demonstrated workload.

## 3. External Sources

- [Rust `Read::take`](https://doc.rust-lang.org/std/io/trait.Read.html#method.take) — the standard reader adapter returns EOF after at most the supplied byte count, supporting a ceiling-plus-one overflow probe without reading an unbounded stream.
- [Rust `Read::read_to_end`](https://doc.rust-lang.org/std/io/trait.Read.html#method.read_to_end) — documents byte buffering and I/O error behavior; composing it with `take` bounds retained payload length and preserves read errors encountered before the probe completes.
- [`serde_json::from_slice`](https://docs.rs/serde_json/latest/serde_json/fn.from_slice.html) — decodes the already-bounded byte slice into the existing strict request DTO.
- [`serde_json::error::Category`](https://docs.rs/serde_json/latest/serde_json/error/enum.Category.html) — preserves the established syntax/EOF versus data-error classification after bounded ingestion.

## 4. Risks, Unknowns, Dependencies

- **Risk:** Reading the full stream and checking length afterward would retain the unbounded-input problem. The reader must expose at most `1_048_577` payload bytes to buffering.
- **Risk:** Parsing before overflow classification could label a malformed prefix `invalid_json` even though the raw request is oversized. Size rejection must take precedence once the ceiling-plus-one probe fills.
- **Risk:** Exact-limit tests can pass even if insignificant trailing padding is truncated. The final root `}` must be the exact boundary byte so success proves the entire request was retained.
- **Risk:** An I/O error before byte `1_048_577` proves neither EOF nor overflow and must remain operational. Once that byte is retained, overflow is conclusive and no later reader error is observed.
- **Risk:** Changing successful and lifecycle paths while refactoring ingestion would regress realized INT-0002 behavior; focused regression and full workspace gates remain required.
- **Unknown:** No accepted workload establishes an ideal ceiling. One MiB is a documented source-level guardrail chosen because it is far above the demonstrated request while placing a concrete retained-input bound.
- **Unknown:** Network timeouts, rates, concurrent quotas, and authentication remain undefined and must not be implied by this local byte limit.
- **Dependency:** The implementation uses only `std::io` and existing `serde_json`; no dependency or core-domain change is needed.

## 5. Recommended Approach

Primary: add a public `MAX_REQUEST_BYTES: usize = 1_048_576` at the CLI
boundary. Read through `Read::take(MAX_REQUEST_BYTES + 1)` into a byte vector,
map a read error observed before the overflow byte to `RunError::Read(io::Error)`, reject a vector
larger than the limit with `request_too_large`, and otherwise pass the exact
bytes to `serde_json::from_slice`. Count all raw bytes, including trailing
whitespace. Preserve the current response writer and `RunStatus::RequestRejected`
so oversized input emits one version 1 error line and exits 2.

Alternative considered: add an adjustable CLI flag or environment variable.
Rejected because there is no evidenced configuration consumer, and a fixed
public constant is the smaller observable contract.

Alternative considered: add local SQLite persistence and separate commands.
This would create a useful cross-process capability, but it also requires a new
adapter-owned stored representation plus explicit atomicity, locking,
concurrency, corruption, and migration semantics. Those decisions are not
authorized by the current intents and should be researched as a distinct
future outcome.

Alternative considered: build an HTTP service, Electron shell, blockchain
adapter, or new domain policy. Each introduces unresolved network, deployment,
interaction, chain, naming, KPI, lineage, or authorization choices. None is
required to close the accepted unbounded-input debt.

The `RunError::Read` payload changes from `serde_json::Error` to `io::Error`
because parsing now occurs after ingestion and parse failures remain modeled
responses; this is an acknowledged experimental Rust API adjustment rather
than an ad-hoc conversion.

Rationale: this is the only follow-on requirement explicitly carried by a
realized intent. It is locally and process-level verifiable, dependency-free,
and reversible, while every larger candidate would silently choose policy the
Book continues to defer.

## Artifacts

- No separate artifacts were saved; repository evidence is listed in the code survey and all external evidence is linked above.
