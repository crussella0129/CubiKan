# Sprint 5 Integration Test Results

- **Primary intent:** [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md)
- **Preserved intents:** [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md), [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md), [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md), [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md), and [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md)
- **Tested Build head:** `6979ca2217ac1b838c406bf21821e32b3a4f6227`
- **Local execution:** `cargo +stable test --workspace --all-targets` and `cargo +stable test --doc --workspace`
- **Result:** pass

## T-502 public runner identity contract

The all-target run executed the external `crates/cubikan-cli/tests/runner.rs`
target with 13 passed, 0 failed, 0 ignored, 0 measured, and 0 filtered out.
The four Sprint 5 public-boundary tests supplied the following EARS evidence:

| Named test | EARS | Arrangement | Assertions | Result |
|------------|------|-------------|------------|--------|
| `test_run_generates_id_when_member_is_omitted` | T-502-E1 | Remove the `id` object member from an otherwise valid version 1 request, serialize it, and pass the bytes through public `cubikan_cli::run`. | `RunStatus::Success`; exactly one newline-terminated success response; response ID parses through `cubikan_core::IntentUnitId`, is non-nil, and reports UUID version 4. | pass |
| `test_run_rejects_present_non_string_ids_without_creating_state` | T-502-E2 | Table-test present `null`, Boolean `true`, number `42`, array `["value"]`, and object `{"value": true}` values through public `run` with a recording `Write`. | Every case returns `RunStatus::RequestRejected`; writes exactly one newline; flushes exactly once at the final byte offset after that newline; returns outcome `error`, protocol version `1`, code `invalid_request`, and a nonempty unpinned message; omits `intent_unit`, `field`, and `operation_number`. | pass |
| `test_run_preserves_id_string_validation_taxonomy` | T-502-E3 | Run the canonical fixed UUID string, then replace it with the present string `not-a-uuid` and run again. | The valid string succeeds with the exact fixed ID. The malformed string returns request rejection, version `1`, code `invalid_intent_unit_id`, field `intent_unit.id`, a nonempty message, no operation number, and no state snapshot. | pass |
| `test_run_preserves_oversize_before_explicit_null_classification` | T-502-E4 | Set the ID to explicit `null`, serialize, remove the final root brace, pad through byte `MAX_REQUEST_BYTES`, and append the required brace as byte `MAX_REQUEST_BYTES + 1`. | Public `run` returns request rejection and the exact existing one-line `request_too_large` response before structural ID classification. | pass |

### Preserved runner and component regressions

All nine pre-existing tests in the same public-runner target also passed:

- `test_request_limit_is_one_mib` retained the public `1_048_576`-byte
  ceiling.
- `test_runner_accepts_exact_limit_request` and
  `test_runner_rejects_one_byte_over_limit` retained exact-limit acceptance
  and the one-byte-over response.
- `test_runner_executes_configure_create_transition_complete` retained the
  composed fixed-ID lifecycle result.
- `test_runner_returns_request_failure_without_unit_state` and
  `test_runner_preserves_prior_successes_on_lifecycle_failure` retained setup
  and lifecycle rejection state boundaries.
- `test_runner_exposes_io_read_error_payload` and
  `test_runner_propagates_output_io_failure` retained public operational I/O
  errors.
- `test_runner_surfaces_buffered_sink_failure_on_explicit_flush` retained the
  real `BufWriter` drain-failure proof.

The same all-target run also passed the named in-crate preservation oracles
from the finalized Test plan:

| Named regression | Preserved assertion | Result |
|------------------|---------------------|--------|
| `test_fixed_id_scenario_constructs_core_state` | A valid supplied UUID constructs state with that identity. | pass |
| `test_omitted_id_generates_non_nil_v4` | The unchanged execution branch generates a non-nil UUID v4 for decoded omission. | pass |
| `test_unsupported_version_and_scalar_failures_are_typed` | Unsupported-version and malformed-string setup taxonomy remains typed. | pass |
| `test_run_consumes_at_most_limit_plus_one` | The bounded reader consumes exactly `MAX_REQUEST_BYTES + 1` bytes from a longer input. | pass |
| `test_run_flushes_each_modeled_response_once_after_newline` | Success, malformed, setup, lifecycle, and oversize modeled responses each flush once after one newline. | pass |
| `test_run_preserves_response_output_error_precedence` | Body, newline, and flush failures retain first-error ordering and do not attempt later stages. | pass |

### Test-double boundary

There are no domain mocks, service stubs, or network mocks in the runner
evidence. Each identity case invokes the production public `run`, production
Serde decoder, production execution mapping, and real `cubikan-core` types.
Byte slices and vectors exercise the public `Read`/`Write` seams. The
test-owned `RecordingWriter` records accepted bytes and flush offsets only;
the deterministic `CountingReader` records the public ingestion bound only.
The buffered-output regression uses the standard library's real `BufWriter`
around a deterministic drain-rejecting `Write`. These I/O doubles do not
implement protocol, identity, lifecycle, classification, or status behavior.

## `test_hosted_sprint_five_quality_run_succeeds`

**Coverage:** T-504-E3 and the preserved INT-0005 hosted quality boundary.

GitHub received the exact committed Build head through a real `push` to
`dev` and completed attempt 1 with one job:

| Field | Observed value |
|-------|----------------|
| Run | [31285064082 — Rust CI](https://github.com/crussella0129/CubiKan/actions/runs/31285064082) |
| Event / branch | `push` / `dev` |
| Run attempt | `1` |
| Run status / conclusion | `completed` / `success` |
| Run created / started | `2026-08-08T23:55:40Z` / `2026-08-08T23:55:40Z` |
| Run updated | `2026-08-08T23:56:07Z` |
| Job | [93172288024 — Rust quality gate](https://github.com/crussella0129/CubiKan/actions/runs/31285064082/job/93172288024) |
| Job status / conclusion | `completed` / `success` |
| Job created / started / completed | `2026-08-08T23:55:41Z` / `2026-08-08T23:55:43Z` / `2026-08-08T23:56:06Z` |
| Run and job head SHA | `6979ca2217ac1b838c406bf21821e32b3a4f6227` |
| Remote `refs/heads/dev` | `6979ca2217ac1b838c406bf21821e32b3a4f6227` |
| Checked-out commit | `6979ca2217ac1b838c406bf21821e32b3a4f6227` |

The remote workflow blob at that commit was
`96420136d282ef93bb60b0607dffac1d28427a8d`, identical to the local committed
blob. The checkout log fetched the Build SHA for remote `dev`, checked out that
revision, and reported the same SHA from the job worktree. This ties the remote
ref, workflow run, sole job, and executed source to the Test oracle rather than
to later evidence-only commits.

### Hosted gate results

Setup and every configured Cargo quality gate completed successfully:

| Hosted step / command | Started | Completed | Status / conclusion |
|-----------------------|---------|-----------|---------------------|
| Check out repository | `2026-08-08T23:55:45Z` | `2026-08-08T23:55:45Z` | `completed` / `success` |
| Install stable Rust — `rustup toolchain install stable --profile minimal --component rustfmt,clippy` | `2026-08-08T23:55:45Z` | `2026-08-08T23:55:46Z` | `completed` / `success` |
| Check formatting — `cargo +stable fmt --all -- --check` | `2026-08-08T23:55:46Z` | `2026-08-08T23:55:47Z` | `completed` / `success` |
| Run Clippy — `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings` | `2026-08-08T23:55:47Z` | `2026-08-08T23:55:53Z` | `completed` / `success` |
| Check workspace — `RUSTFLAGS="-D warnings" cargo +stable check --workspace --all-targets` | `2026-08-08T23:55:53Z` | `2026-08-08T23:55:58Z` | `completed` / `success` |
| Run workspace tests — `cargo +stable test --workspace --all-targets` | `2026-08-08T23:55:58Z` | `2026-08-08T23:56:04Z` | `completed` / `success` |
| Run workspace doctests — `cargo +stable test --doc --workspace` | `2026-08-08T23:56:04Z` | `2026-08-08T23:56:04Z` | `completed` / `success` |

The hosted all-target log recorded the exact following test distribution:

| Cargo target | Passed | Failed |
|--------------|-------:|-------:|
| `cubikan_cli` library unit tests | 32 | 0 |
| `cubikan` binary unit tests | 0 | 0 |
| `cli_e2e` integration target | 6 | 0 |
| `runner` integration target | 13 | 0 |
| `cubikan_core` library unit tests | 43 | 0 |
| `lifecycle` integration target | 4 | 0 |
| `serialization` integration target | 2 | 0 |
| **All targets** | **100** | **0** |

Each target also reported zero ignored, measured, or filtered-out tests. The
hosted doctest log recorded zero `cubikan_cli` doctests and one passing
`cubikan_core` doctest, for **1 passed and 0 failed** workspace doctests.

### Hosted runtime, checkout, and permissions provenance

The job log observed GitHub Actions runner `2.336.0`, Ubuntu `24.04.4` on the
`ubuntu-24.04` image version `20260720.247.2`, and stable Rust resolving to
`rustc 1.97.1 (8bab26f4f 2026-07-14)` with the minimal `rustfmt` and `clippy`
components. These values are run provenance, not a fixed runner, OS matrix,
toolchain matrix, or MSRV promise.

GitHub downloaded the immutable
`actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1`
revision identified in the workflow as v7.0.1. The workflow sets
`persist-credentials: false`: checkout still receives GitHub's masked built-in
token to perform the authorized fetch, but does not leave that credential in
the worktree's Git configuration. The workflow explicitly grants only
`contents: read`; GitHub additionally reported implicit `Metadata: read` for
the built-in token. No custom secret is referenced.

## Evidence authority and reproducibility

The local commands ran the named Rust assertions against the Build source and
corroborated 100 all-target tests plus one doctest on local stable Rust
`1.95.0`. Static inspection and the matching workflow blob prove the exact
configuration. Neither local execution nor static YAML inspection can prove
that GitHub registered or executed the workflow.

The completed GitHub run and sole job are therefore the authoritative hosted
integration oracle: they independently executed the committed Build SHA on a
GitHub-hosted runner with hosted stable Rust and completed every configured
quality step successfully. This was one successful attempt; no retry or
synthetic failure run was used. The evidence does not claim branch protection,
merge authorization, fixed future runner/toolchain versions, or a successful
pull-request event. Human merge approval and the later `dev` to `main` remote
checkpoint remain separate.

Reproduction queries:

```sh
gh api repos/crussella0129/CubiKan/git/ref/heads/dev
gh api 'repos/crussella0129/CubiKan/contents/.github/workflows/ci.yml?ref=6979ca2217ac1b838c406bf21821e32b3a4f6227'
gh run view 31285064082 --json databaseId,url,event,headBranch,headSha,status,conclusion,createdAt,startedAt,updatedAt,jobs
gh run view 31285064082 --job 93172288024 --log
```
