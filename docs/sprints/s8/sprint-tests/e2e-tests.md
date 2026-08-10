# Sprint 8 End-to-End Test Results

- **Primary intent:** [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md)
- **Preserved intents:** [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) and [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Tested critic-response head:** `065b71fa1b63ba6abce6effb23c9d20674171835`
- **Durable `cubikan-local` actual-process result:** 2/2 passed
- **Stateless `cubikan` actual-process regression:** 6/6 passed
- **Exact-head hosted quality result:** pass; run `31344560356`, job `93323978596`, attempt 1
- **Conclusion:** pass; real independent processes prove the locked local durable journey and fail-closed storage behavior, while the unchanged old process and hosted quality boundaries remain green

The two product E2E tests launch the Cargo-built `cubikan-local` executable with
`Command`, pipe real stdin/stdout/stderr, and share only an explicit test-owned
local path. Each invocation is a new operating-system process with a new
backend connection. There is no repository mock, in-memory SQLite substitute,
network service, or direct call to the runner under test.

Unit/repository evidence is recorded in [unit-tests.md](unit-tests.md), and the
real-SQLite transaction/concurrency/cross-component matrix is recorded in
[integration-tests.md](integration-tests.md).

## T-809-E1 durable multi-process lifecycle

- **Exact named test:** `test_cubikan_local_persists_paginates_and_completes_across_processes`
- **Command boundary:** `env!("CARGO_BIN_EXE_cubikan-local") --database <one test-owned path>` for every invocation
- **Result:** pass

The test starts each row below as a separate child process and parses its one
newline-terminated compact JSON response. Every modeled response has empty
stderr.

| Process step | Request arrangement | Exact SHALL observation | Exit | Result |
|--------------|---------------------|-------------------------|-----:|--------|
| 1 | create fixed ID `...0002`, workflow `delivery`, species `feature` | exact full active unit in `queued`, revision string `"0"`, empty history | 0 | pass |
| 2 | create fixed ID `...0001` against the same database | second independent exact active revision-0 unit; first remains durable | 0 | pass |
| 3–4 | fresh-process get for IDs 01 and 02 | each stable ID returns its own immutable three-phase workflow, species, phase/status/revision, and empty history | 0 | pass |
| 5 | list with all four filters, limit 1, no cursor | exact summary for lexical ID 01 and `next_cursor` equal to ID 01 | 0 | pass |
| 6 | same list with exclusive cursor ID 01 | exact summary for ID 02 and `next_cursor:null`; no boundary repeat | 0 | pass |
| 7 | transition ID 01 `queued -> doing` with expected `"0"` | mutation returns committed/unit revision `"1"` and one exact sequence-1 record | 0 | pass |
| 8 | stale completion of ID 01 with expected `"0"` while `doing` is also ineligible | `revision_conflict`, expected `"0"`, actual `"1"`, no `field`; stale wins before completion eligibility and no state changes | 3 | pass |
| 9 | fresh-process get after rejection | exact revision-1 `doing` unit with the one prior record, proving durable atomic rejection | 0 | pass |
| 10 | transition ID 01 `doing -> done` with refreshed expected `"1"` | committed revision `"2"`, active `done`, exact two-record history | 0 | pass |
| 11 | complete ID 01 with expected `"2"` | committed revision `"3"`, completed `done`, exact completion record at sequence 3 | 0 | pass |
| 12 | final fresh-process get after all prior processes exit | exact immutable workflow/species/identity, completed status, revision `"3"`, and ordered two-transition-plus-completion history | 0 | pass |

This single journey covers every T-809-E1 SHALL: two persistent units,
independent retrieval, exact filtered pagination in lexical order, guarded
mutation from revision 0, stale competing rejection, explicit refresh,
continued transition, completion, process exit, and exact final retrieval. It
also realizes the process-level acceptance criterion that Sprint 7 could not
exercise before INT-0010 existed.

## T-809-E2 actual-process fail-closed storage

- **Exact named test:** `test_cubikan_local_rejects_unknown_and_malformed_schema_without_mutation`
- **Result:** pass

The test first creates a real owned schema-v1 database and sentinel unit, then
builds two independent on-disk fixtures. One changes the SQLite header's
`user_version` from 1 to 2. The other makes an equal-length change to one locked
index definition, producing a malformed schema-v1 database without changing
file length.

| Fixture | Actual-process request | Exact SHALL observation | Preservation oracle | Exit | Result |
|---------|------------------------|-------------------------|---------------------|-----:|--------|
| unsupported version 2 | get the seeded ID through `cubikan-local --database PATH` | protocol v1 failure with `unsupported_schema_version`, human message, and no validation/conflict optional members | complete file bytes after process exit equal the pre-invocation fixture | 4 | pass |
| malformed schema v1 | same actual-process get | protocol v1 failure with `corrupt_schema`, human message, and no validation/conflict optional members | complete file bytes after process exit equal the pre-invocation fixture | 4 | pass |

Byte equality is stronger than the locked logical version/content requirement
for these controlled fixtures. It does not enlarge the product guarantee:
SQLite open may touch other rejected files, and the documented contract remains
fail-closed logical non-adoption/non-repair rather than universal byte identity.

## Existing stateless `cubikan` actual-process regression

- **Named gate:** `verify_existing_stateless_cli_e2e_6_of_6`
- **Executed target:** `cargo +stable test -p cubikan-cli --test cli_e2e`
- **Result:** pass — 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
- **Mocks/stubs:** none; every test launches the Cargo-built `cubikan` binary

| Existing actual-process test | Preserved observation | Exit | Result |
|------------------------------|-----------------------|-----:|--------|
| `test_cli_configure_create_transition_complete` | one in-memory configure/create/two-transition/complete scenario returns its exact completed adapter snapshot | 0 | pass |
| `test_cli_generates_id_when_member_is_omitted` | true omission generates a parseable non-nil UUID v4 and retains the old response shape | 0 | pass |
| `test_cli_reports_explicit_null_id_with_exit_2` | explicit null remains structural `invalid_request` with no unit state | 2 | pass |
| `test_cli_reports_malformed_request_with_exit_2` | malformed JSON retains the old `invalid_json` envelope | 2 | pass |
| `test_cli_reports_lifecycle_rejection_with_exit_3` | undeclared edge returns exact operation-2 failure plus the prior successful in-memory state | 3 | pass |
| `test_cli_reports_oversized_request_with_exit_2` | byte 1,048,577 returns the old `request_too_large` response | 2 | pass |

The accepted-base-to-tested-candidate diff contains no path under
`crates/cubikan-cli`. These six processes prove regression only: the old
`cubikan` binary is still stateless, has no expected-revision commands, and does
not read or write the new database. The durable outcome belongs only to the
separate `cubikan-local` binary and protocol.

## Exact-head hosted Rust quality run

- **Named gate:** `test_hosted_sprint_eight_quality_run_succeeds`
- **Run:** [31344560356 — Rust CI](https://github.com/crussella0129/CubiKan/actions/runs/31344560356)
- **Job:** [93323978596 — Rust quality gate](https://github.com/crussella0129/CubiKan/actions/runs/31344560356/job/93323978596)
- **Result:** pass

GitHub received the exact critic-response candidate through the existing `dev`
push boundary. The first Test response `2e5e2b9` added the three planned
cross-component tests; after critique, `065b71f` added only the `#[cfg(test)]`
exhaustive backend-error mapper oracle. Local/remote and GitHub observations
identify that final object:

| Provenance observation | Observed value | Result |
|------------------------|----------------|--------|
| Local committed candidate | `065b71fa1b63ba6abce6effb23c9d20674171835` | match |
| Local `origin/dev` | `065b71fa1b63ba6abce6effb23c9d20674171835` | match |
| GitHub run API head SHA | `065b71fa1b63ba6abce6effb23c9d20674171835` | match |
| GitHub job/checkout `rev-parse` and `git log` | `065b71fa1b63ba6abce6effb23c9d20674171835` | match |

| Hosted field | Observed value |
|--------------|----------------|
| Workflow / job | `Rust CI` / `Rust quality gate` |
| Event / branch | `push` / `dev` |
| Run / job IDs | `31344560356` / `93323978596` |
| Attempt | `1` |
| Run status / conclusion | `completed` / `success` |
| Run created / started | `2026-08-10T00:28:49Z` / `2026-08-10T00:28:49Z` |
| Run updated | `2026-08-10T00:29:57Z` |
| Job interval | `2026-08-10T00:28:52Z`–`2026-08-10T00:29:56Z` |
| Runner | GitHub runner `2.336.0` |
| Image | Ubuntu `24.04.4`; `ubuntu-24.04` image `20260720.247.2` |
| Installed current stable | `rustc 1.97.1 (8bab26f4f 2026-07-14)` |
| Workflow blob | `96420136d282ef93bb60b0607dffac1d28427a8d` |
| Token permissions | explicit `Contents: read`; implicit `Metadata: read` |
| Checkout credential behavior | `persist-credentials: false` |

The checked-in workflow pins
`actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1`
and configures a 15-minute job timeout. Setup, checkout, current-stable install,
all five quality steps, checkout post-step, and job completion each reported
`completed` / `success`:

| Hosted quality step | Exact command or boundary | Result |
|---------------------|---------------------------|--------|
| Set up job | GitHub-hosted runner initialization | pass |
| Check out repository | pinned checkout action; credentials not persisted | pass |
| Install stable Rust | `rustup toolchain install stable --profile minimal --component rustfmt,clippy` | pass |
| Check formatting | `cargo +stable fmt --all -- --check` | pass |
| Run Clippy | `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings` | pass; zero warnings |
| Check workspace | `RUSTFLAGS="-D warnings" cargo +stable check --workspace --all-targets` | pass; zero warnings |
| Run workspace tests | `cargo +stable test --workspace --all-targets` | pass; 165 tests |
| Run workspace doctests | `cargo +stable test --doc --workspace` | pass; 1 core doctest |
| Post checkout / complete job | action cleanup and job finalization | pass |

Hosted all-target logs reported the same exact subtotals as local execution:

| Crate boundary | Passed | Failed | Ignored / measured / filtered |
|----------------|-------:|-------:|-------------------------------:|
| `cubikan-backend` | 30 | 0 | 0 |
| stateless `cubikan-cli` | 51 | 0 | 0 |
| `cubikan-core` | 65 | 0 | 0 |
| durable `cubikan-local` | 19 | 0 | 0 |
| **Total** | **165** | **0** | **0** |

The hosted doctest step adds one passing core doctest. The run used attempt 1;
no workflow/job retry or rerun supplies this evidence.

## External, flake, and claim boundary

The product E2E boundary uses real local child processes, a test-owned local
filesystem path, and the bundled SQLite engine. Fixed UUIDs and checked-in
requests make ordering deterministic. No network, service, repository mock,
`:memory:` database, or external direct writer participates. The process tests
do not inject crash-kill, power loss, device failure, or network-filesystem
behavior, and they do not prove peer/OS acknowledgement of stdout after the
Rust writer flush.

The local contention integration is the only deliberately time-sensitive
product test: it observes the real 5,000-ms busy wait and allows a bounded
4.5–9-second interval. Product E2E itself contains no concurrency race or
wall-clock assertion. Temporary-directory allocation and local process/SQLite
availability remain ordinary host dependencies.

The hosted gate crosses the real GitHub Actions service, a floating
`ubuntu-latest` image, Rustup's moving `stable` channel, and crates.io
index/download availability. The checkout action is immutable-pinned, but the
runner image, current stable compiler, hosted availability, and registry state
can change on later runs. One successful attempt does not establish ongoing CI
availability, deterministic future versions, an MSRV, Windows/macOS support,
performance/load capacity, branch protection, merge authorization, security or
supply-chain certification, backup/recovery, release/deployment behavior, or a
successful pull-request event.

Together, the local actual-process tests are the durability oracle and the
hosted run is the exact-revision regression oracle. Neither is evidence for
network service safety, shared direct storage, retries/idempotency, cross-unit
transactions, cryptographic audit, acknowledged delivery, or indefinite
schema/protocol compatibility.
