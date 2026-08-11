# Sprint 10 End-to-End Test Results

- **Primary intent:** [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- **Accepted base:** `bb257db8c62083ae8be4e8d77ec63762ba2e8fa8`
- **Exact tested head:** `0a7bc3a023364cca9197e735c5acfeab019ce8a1`
- **Local toolchain:** `rustc 1.95.0 (59807616e 2026-04-14)`; `cargo 1.95.0 (f2d3ce0bd 2026-03-21)`
- **Finalized Sprint 10 checks:** 15/15 passed; 0 failed
- **Workspace all-target regression:** 191/191 passed
- **Workspace doctests:** 1/1 passed
- **Hosted exact-head quality result:** pass; run `31533101690`, job `93917639820`, attempt 1
- **Conclusion:** pass; all three planned end-to-end/procedural gates passed at the exact tested head, the bounded provider observations found none of the six recommended derivative repositories in either observed scope, and the recorded Sprint 10 mutations targeted only `crussella0129/CubiKan` on `dev`

Unit/repository evidence is recorded in [unit-tests.md](unit-tests.md), and the
composed documentation evidence is recorded in
[integration-tests.md](integration-tests.md). This artifact records the three
exact end-to-end checks from the finalized test plan and preserves the boundary
between direct observations and later offline consistency validation.

## Evidence and claim boundary

The local all-mode run exercised the checked-in structural harness, all five
Rust gates, repository hygiene, and a bounded action-audit fixture. The fixture
contains results captured by actual GitHub public REST requests and actual
connected-GitHub queries performed during this sprint. The checked-in
`audit_evidence.py` validator is different evidence: it runs offline and checks
the JSON's type, completeness, internal consistency, exact-head binding, action
kinds, and mutation targets. It does not call GitHub, authenticate a recorded
observation, or independently prove that a provider response occurred.

The repository/action conclusion is deliberately bounded. It covers the six
exact slugs, the connected installation visible to this session, the public
REST observations, the current local Git configuration and history, and the
recorded Sprint 10 action ledger during the observation window
`2026-08-11T19:25:11Z`–`2026-08-11T20:28:26Z`. It is not a claim about a
complete private account inventory or about externally erased, rewritten, or
otherwise unavailable history.

## `verify_workspace_regression_gates`

- **Result:** pass
- **All-mode invocation:**
  `bash docs/sprints/s10/sprint-tests/documentation-checks.sh all --audit-evidence docs/sprints/s10/sprint-tests/remote-audit-evidence.json`
- **All-mode summary:** `15 passed, 0 failed, 15 selected`

The complete typed provider observations and action ledger used by that
invocation are preserved in
[remote-audit-evidence.json](remote-audit-evidence.json). The fixture declares
the exact tested candidate above, and the validator requires that candidate to
resolve as an ancestor of any later evidence-only checkout. This avoids a
self-referential evidence-commit SHA while keeping clean-clone replay possible:
run the command above from the later evidence-bearing checkout that contains
the fixture. The validator resolves the declared tested candidate as its
ancestor and verifies the fixture's commit objects and final-push equality;
the filename itself does not establish candidate identity.

The all-mode runner executed these five canonical local gates:

```text
cargo +stable fmt --all -- --check
CARGO_NET_OFFLINE=true cargo +stable clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
CARGO_NET_OFFLINE=true RUSTFLAGS='-D warnings' cargo +stable check --workspace --all-targets --all-features --locked --offline
CARGO_NET_OFFLINE=true cargo +stable test --workspace --all-targets --all-features --locked --offline
CARGO_NET_OFFLINE=true cargo +stable test --workspace --doc --all-features --locked --offline
```

Formatting, Clippy with warnings denied, the warnings-denied all-target check,
all-target tests, and doctests all completed successfully. The 191 all-target
tests include library, binary, and integration targets; zero-test binaries do
not add to the total.

| Crate boundary | Passed | Failed | Ignored | Measured | Filtered |
|----------------|-------:|-------:|--------:|---------:|---------:|
| `cubikan-backend` | 56 | 0 | 0 | 0 | 0 |
| stateless `cubikan-cli` / `cubikan` | 51 | 0 | 0 | 0 | 0 |
| `cubikan-core` | 65 | 0 | 0 | 0 | 0 |
| durable `cubikan-local` | 19 | 0 | 0 | 0 | 0 |
| **Total** | **191** | **0** | **0** | **0** | **0** |

The separate doctest command executed the one example in
`crates/cubikan-core/src/lib.rs`; it passed 1/1. The other three crates reported
zero doctests.

## `verify_repository_hygiene`

- **Result:** pass
- **Candidate revision:** `0a7bc3a023364cca9197e735c5acfeab019ce8a1`

The exact-head all-mode run produced the following repository observations:

| Check | Exact result |
|-------|--------------|
| Working-tree and accepted-base whitespace | Both `git diff --check` invocations passed. |
| Authoritative Book validator | `check-book: valid v2 Book (13 intent chapters)` |
| Markdown paths, fragments, and navigation | `markdown_resolver: 136 Markdown files, 981 links, 898 local targets, 13 fragments, 13 Book intents; 0 errors` |
| Markdown parser self-test | Full, collapsed, and shortcut references resolved; invalid forms were rejected. |
| Offline audit-validator self-test | Five unsafe/unknown actions, malformed and unresolved push heads, and four invalid evidence shapes were rejected; the typed offline fixture was accepted. |
| Sprint layout router | `test` |
| Legacy writable authority | No tracked or on-disk root `sprints/`, `agent-tasks/`, or `decisions.md` authority existed. |

The resolver counts above describe the evidence-bearing working tree after the
three result artifacts and durable JSON link were populated. The clean tested
candidate's pinned `967`-link / `886`-local-target observation is recorded in
the unit evidence. A final resolver and diff check is run after
report and critique capture so later evidence prose cannot introduce a broken
local target or whitespace error.

## `verify_no_derivative_repository_operations`

- **Result:** pass within the bounded evidence scope
- **Local branch:** `dev`
- **Local `HEAD` / `origin/dev`:** both
  `0a7bc3a023364cca9197e735c5acfeab019ce8a1`
- **Only configured remote:** `origin`, fetch and push URL
  `https://github.com/crussella0129/CubiKan`
- **Accepted-base-to-head commits inspected:** 28

### Actual provider observations

At `2026-08-11T19:59:13Z`, each exact recommended slug returned both a zero-result
connected-app search in scope `connected-installation-only` and HTTP 404 from a
public GitHub REST `GET`:

| Exact repository name | Connected result count | Public REST status |
|-----------------------|-----------------------:|-------------------:|
| `crussella0129/animus-ledger` | 0 | 404 |
| `crussella0129/cubikan-agent-ops` | 0 | 404 |
| `crussella0129/cubikan-observatory` | 0 | 404 |
| `crussella0129/cubikan-org-app-kit` | 0 | 404 |
| `crussella0129/cubikan-process-studio` | 0 | 404 |
| `crussella0129/cubikan-skill-graph` | 0 | 404 |

Each connector query used the exact form
`<slug> user:crussella0129 in:name`. A separate connected-app repository-list
request at `2026-08-11T19:25:11Z` returned zero repositories, but that list was
limited to the app's connected installation and explicitly marked
`complete_account_inventory: false`. Public 404 likewise establishes only that
the named repository was not publicly visible through that endpoint. Together,
these observations found no named derivative in the two observed scopes; they
do not establish a complete private-account inventory.

The same public REST capture observed `crussella0129/CubiKan` with HTTP 200,
zero deployments from the complete paginated deployment response, and zero
published releases from the complete paginated public release response. The
release result covers published releases only: the unauthenticated public
endpoint does not expose drafts. It must not be restated as proof that no draft
release exists.

### Recorded Sprint 10 action ledger

The bounded ledger contains 23 actions: 19 read-only Git/provider observations
and four mutations. All four mutations were Git pushes to
`https://github.com/crussella0129/CubiKan.git`, branch `dev`:

| Push | Exact pushed head |
|-----:|-------------------|
| 1 | `4040493ce4cb3ff060d10721211e3ec1135de6d5` |
| 2 | `cdede78dae2d3838328b63abd6171e54d4a557f4` |
| 3 | `1c26ee010b8c201a8e8092255df4b949011ed21a` |
| 4 | `0a7bc3a023364cca9197e735c5acfeab019ce8a1` |

No ledger action creates, pushes, publishes, releases, or deploys a derivative
repository. No local remote targets a derivative slug. These are affirmative
claims about the complete recorded Sprint 10 ledger and the inspected durable
Git state, subject to the bounded/no-erased-history caveat above.

### Offline shape and consistency validation

The audit runner reported:

```text
audit_evidence: head=0a7bc3a023364cca9197e735c5acfeab019ce8a1 targeted_repository_checks=6 published_releases=0 deployments=0 actions=23 remote_mutations=4; captured observations report no derivative repository or mutation; offline shape/consistency validation only (no provider calls)
```

This pass proves that the supplied JSON conformed to the checked-in bounded
schema and agreed with the declared tested candidate, branch, remote, six exact
targets, and allowed ledger policy. It does not upgrade the provider
observations into an account-wide or historical nonexistence proof.

## Exact-head hosted Rust quality run

- **Run:** [31533101690 — Rust CI](https://github.com/crussella0129/CubiKan/actions/runs/31533101690)
- **Job:** [93917639820 — Rust quality gate](https://github.com/crussella0129/CubiKan/actions/runs/31533101690/job/93917639820)
- **Event / branch:** `push` / `dev`
- **Exact head:** `0a7bc3a023364cca9197e735c5acfeab019ce8a1`
- **Result:** `completed` / `success`

The run API, job API, checkout fetch, checkout, and final `git rev-parse`
independently identify the same exact SHA. Local `HEAD` and local
remote-tracking `origin/dev` also matched it when the evidence was captured.

| Hosted field | Exact observation |
|--------------|-------------------|
| Workflow / run number | `Rust CI` / `36` |
| Display title | `sprint-10: harden remote audit provenance` |
| Run / job IDs | `31533101690` / `93917639820` |
| Attempt / previous attempt | `1` / none |
| Run created / started / updated | `2026-08-11T20:28:20Z` / `2026-08-11T20:28:20Z` / `2026-08-11T20:29:44Z` |
| Job created / started / completed | `2026-08-11T20:28:21Z` / `2026-08-11T20:28:24Z` / `2026-08-11T20:29:44Z` |
| Runner | GitHub-hosted `ubuntu-latest`; `GitHub Actions 1000004667` (runner ID `1000004667`), runner version `2.336.0` |
| OS / image | Ubuntu `24.04.4 LTS`; `ubuntu-24.04` image `20260810.271.1` |
| Installed stable | `rustc 1.97.1 (8bab26f4f 2026-07-14)` via Rustup minimal profile with `rustfmt` and Clippy |
| Checkout | `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1`; `persist-credentials: false` |
| Token permissions | explicit `Contents: read`; implicit `Metadata: read` |
| Shell / timeout | `/usr/bin/bash -e` for setup and the five quality commands; 15-minute job timeout |

Every hosted job step completed successfully on attempt 1:

| Step | UTC interval | Exact command or boundary | Status / conclusion |
|-----:|--------------|---------------------------|---------------------|
| 1 | `20:28:24`–`20:28:25` | Set up the hosted job, runner, image, permissions, and shell. | `completed` / `success` |
| 2 | `20:28:25`–`20:28:26` | Check out exact head using the pinned action without persisted credentials. | `completed` / `success` |
| 3 | `20:28:26`–`20:28:28` | `rustup toolchain install stable --profile minimal --component rustfmt,clippy` | `completed` / `success` |
| 4 | `20:28:28`–`20:28:32` | `cargo +stable fmt --all -- --check` | `completed` / `success` |
| 5 | `20:28:32`–`20:28:57` | `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings` | `completed` / `success` |
| 6 | `20:28:57`–`20:29:08` | `RUSTFLAGS="-D warnings" cargo +stable check --workspace --all-targets` | `completed` / `success` |
| 7 | `20:29:08`–`20:29:41` | `cargo +stable test --workspace --all-targets`; 191/191 passed. | `completed` / `success` |
| 8 | `20:29:41`–`20:29:42` | `cargo +stable test --doc --workspace`; one core doctest passed. | `completed` / `success` |
| 16 | `20:29:42`–`20:29:42` | Post-checkout cleanup. | `completed` / `success` |
| 17 | `20:29:42`–`20:29:42` | Complete job. | `completed` / `success` |

The hosted gate is complementary to the stricter offline local invocation: its
workflow commands do not include the local harness's `--locked`, `--offline`,
or every `--all-features` argument. It crosses the real GitHub Actions service,
the floating `ubuntu-latest` label, Rustup's moving `stable` channel, and
registry availability. Attempt-1 success records no observed hosted flake; it
does not prove permanent availability or future reproducibility.
