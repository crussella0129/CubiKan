# Sprint 3 Research Report

## Intents Reviewed

- [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) — created; relevance: owns the selected supplied-writer flush outcome without rewriting a terminal intent; current state: `proposed`.
- [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) — selected; relevance: supplies the realized operational I/O and exit-status behavior that flush failures must preserve; current state: `realized`.
- [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) — selected; relevance: supplies the generic one-response writer boundary and the rule that incomplete output is operational rather than a protocol rejection; current state: `realized`.
- [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) — selected; relevance: its domain semantics and dependency boundary must remain unchanged; current state: `realized`.

## 1. Sprint Goal

Close a concrete false-success gap in the existing local JSON adapter. Sprint 3
will make the generic runner call the supplied writer's flush after writing the
complete response line and before returning any modeled status, expose a
flush-only failure as a typed operational error,
map it to process exit `1`, and prove that every realized response class keeps
its existing JSON and exit behavior. The sprint does not choose persistence,
networking, UI, blockchain, or new protocol policy.

## 2. Existing Code Survey

| File | Relevance | Finding |
|------|-----------|---------|
| `crates/cubikan-cli/src/runner.rs` | high | `write_response` serializes JSON and calls `write_all(b"\n")`, then returns `RunStatus` without calling `flush`; `RunError` has body and newline variants but no flush variant. |
| `crates/cubikan-cli/src/lib.rs` | high | `run_process` already maps every `RunError` to exit `1` and a best-effort stderr diagnostic, so a new operational variant composes without changing exit meanings. |
| `crates/cubikan-cli/src/main.rs` | medium | The executable supplies locked standard streams to the generic process shell; normal actual-process behavior must remain unchanged. |
| `crates/cubikan-cli/tests/runner.rs` | high | The public seam covers body-write failure but has no buffered-writer or flush-only failure assertion. |
| `crates/cubikan-cli/tests/cli_e2e.rs` | high | Existing actual-process tests cover exits `0`, `2`, and `3` plus exact JSON/stream output and remain the regression boundary. |
| `crates/cubikan-cli/README.md` | high | Promises exactly one complete newline-terminated response and classifies incomplete output as operational, but does not state explicit flushing or its non-atomic limits. |
| `README.md` | medium | Defines the current one-shot local boundary and the canonical quality gates. |
| `docs/sprints/s1/sprint-plans/build-plan.md` | high | Locked T-105-E5 requires an underlying operational error whenever a response cannot be written completely, rather than a false protocol outcome. |
| `docs/sprints/s1/sprint-plans/test-plan.md` | high | Its output-failure coverage exercises failed body/newline writes but did not identify buffered flush failure. |
| `docs/sprints/s1/sprint-tests/test-report.md` | medium | Realization evidence records body and trailing-newline failures only. |
| `docs/intents/INT-0002-runnable-lifecycle-adapter.md` | high | Keeps incomplete response handling in the experimental adapter boundary and excludes persistence/platform choices. |
| `docs/intents/INT-0003-bounded-cli-request-ingestion.md` | high | Requires genuine output failures to remain exit `1`; the terminal intent stays historical and INT-0004 owns the newly discovered follow-on. |
| `docs/sprints/s2/sprint-tests/test-report.md` | medium | Confirms 85 all-target tests and one doctest passed while CI remained unconfigured; no test asserted flush completion. |
| `Cargo.toml` and `crates/cubikan-cli/Cargo.toml` | medium | The correction needs only `std::io`; no crate, dependency, or core change is justified. |

## 3. External Sources

- [Rust `Write::flush`](https://doc.rust-lang.org/std/io/trait.Write.html#tymethod.flush) — defines flush as ensuring intermediately buffered contents reach their destination and reports an error when all bytes cannot be written.
- [Rust `BufWriter`](https://doc.rust-lang.org/std/io/struct.BufWriter.html) — states that callers must flush explicitly because errors from drop-time flushing are ignored.
- [`serde_json::to_writer` source](https://docs.rs/serde_json/latest/src/serde_json/ser.rs.html#2177-2184) — constructs a serializer and invokes `Serialize`; it does not flush the writer after serialization.

## 4. Risks, Unknowns, Dependencies

- **Risk:** Calling flush before the newline, or returning the modeled status
  before flush succeeds, would retain the false-success gap.
- **Risk:** A body or newline error must remain the first returned error and must
  not trigger a later flush that obscures it.
- **Risk:** A late flush error can occur after some bytes reached the true sink.
  The adapter cannot roll back a stream and must not claim atomic output.
- **Risk:** Merely using a stub whose `write` fails repeats existing coverage.
  A `BufWriter` over a sink that fails only when drained is required to prove the
  missing behavior realistically.
- **Risk:** Adding `RunError::FlushResponse` changes exhaustive matching for an
  experimental public Rust enum. The protocol version, error-code set, and
  process exit meanings must remain unchanged.
- **Unknown:** A portable actual child-process fixture cannot reliably force an
  OS-level stdout flush failure. The injectable public `Write` and process-shell
  seams provide deterministic negative evidence; actual-process E2E remains the
  success-path regression boundary.
- **Dependency:** Only `std::io::Write::flush` is needed. No dependency, core,
  request, response DTO, or domain change is required.

## 5. Recommended Approach

Add `RunError::FlushResponse(io::Error)` with deterministic `Display` and
`Error::source` behavior. In the one centralized response writer, preserve the
existing JSON serialization and newline steps, then call `flush` and return the
modeled status only after it succeeds. Leave `run_process`'s general operational
mapping intact so the new error exits `1` with its existing best-effort
diagnostic.

Use a recording writer to prove separately that success, malformed-JSON request
rejection, setup rejection, lifecycle rejection, and oversized rejection each
write the newline and call the supplied writer's flush exactly once. Use
flush-only and call-order writers to prove exact error/source preservation and
body → newline → flush precedence. Add a public integration test with
`BufWriter` over a write-failing sink so current code would falsely succeed and
corrected code surfaces `FlushResponse`. Retain every actual-process regression
and full workspace gate. Document the writer-flush-checked boundary while
preserving nonclaims for OS delivery, stream atomicity, durable synchronization,
and external acknowledgement.

Alternatives considered: automated hosted CI would be useful and is repeatedly
recorded as absent, but its first real run occurs only after the sprint's remote
checkpoint and therefore cannot be the sole pre-close realization oracle.
Persistent CLI commands are a larger capability but require storage format,
transaction, migration, concurrency, and recovery choices. Service, Electron,
blockchain, performance quotas, and output caps introduce still broader product
or platform policy without accepted evidence.

Rationale: explicit flush completion repairs an already-promised writer
boundary, is directly reproducible through the public API, and is fully
reversible without expanding the product surface.

## Artifacts

- No separate artifacts were saved; repository evidence is listed in the code
  survey and external evidence is linked above.

## Budget Override

The 20-file survey cap was exceeded only to inspect three locked Sprint 1
plan/report artifacts needed to verify that the newly observed flush gap
contradicts accepted cross-sprint response-completion evidence. No unrelated
project surface was opened.
