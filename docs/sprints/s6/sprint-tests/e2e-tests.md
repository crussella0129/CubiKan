# Sprint 6 End-to-End Test Results

- **Primary intent:** [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md)
- **Tested Build head:** `b6daf73cf4c12e496466ebdcb393b3204e7ffeb7`
- **Status:** documentation E2E and exact-head hosted regression executed and passed
- **Runtime derivative status:** not yet possible; no derivative runtime or repository exists

## Reader navigation journey

- **Named check:** `test_reader_can_navigate_summary_to_appendix_and_intents`
- **Coverage:** T-601-E1–E6, T-602-E1, T-603-E1, and T-603-E5
- **Result:** pass

This is an end-to-end check of the Sprint 6 documentation product, not a
runtime integration test. Starting from `docs/SUMMARY.md`, the journey followed
the `Appendix` link to `docs/appendix/README.md`, followed `Potential Derivative
Projects` to `docs/appendix/potential-derivative-projects.md`, and then traversed
the page's local owning-intent and capability-prerequisite links.

| Journey assertion | Observed result | Result |
|-------------------|-----------------|--------|
| Summary entry point | `Appendix` and its nested `Potential Derivative Projects` page are each reachable once from `docs/SUMMARY.md`; the appendix index also links the page. | pass |
| Authority and current boundary | The destination opens with the advisory authority banner, leaves semantic authority with Project Book intents, and describes only the realized chain-agnostic core and one-shot in-memory CLI as current. | pass |
| Capability traversal | The local links reach INT-0008 through INT-0012, whose chapters remain `proposed` with no Work or Completion evidence and whose prerequisites agree with the capability map. | pass |
| Complete primary catalog | Exactly six primary entries are present: `cubikan-agent-ops`, `cubikan-observatory`, `animus-ledger`, `cubikan-process-studio`, `cubikan-skill-graph`, and `cubikan-org-app-kit`. Each separately records its outcome, owned data and policy, inputs, outputs, CubiKan interaction, prerequisites, creation trigger, separation rationale, non-goals, and related intents. | pass |
| Sequence and unresolved choices | The reader can reach the dependency partial order, sequencing and creation gates, merged/deferred/rejected alternatives, open questions, and appendix-wide non-goals. The order preserves INT-0009 before INT-0010; full INT-0008 provenance and INT-0011 also require INT-0009 and INT-0010, while INT-0012 requires INT-0010. | pass |
| Repository-name safety | Recommended repository slugs are plain code labels under local headings, not links to repositories that do not exist. Unnamed organizational verticals remain a future pattern rather than a seventh catalog entry. | pass |
| Current/future consistency | No traversal path turns a proposed backend, adapter, repository, UI, metric, relationship graph, deployment, or blockchain behavior into a present capability or authorization. | pass |

At the tested Build head, the complete local-link resolver enumerated 91
Markdown documents, 521 local links, and 7 fragment references. Every path and
fragment resolved. The traversal therefore reached the requested content
without a broken reference or a contradictory current-versus-future claim.

## Exact-head hosted quality run

- **Named check:** `test_hosted_sprint_six_quality_run_succeeds`
- **Coverage:** Test-phase remote checkpoint and preservation of INT-0005
- **Run:** [31293927701](https://github.com/crussella0129/CubiKan/actions/runs/31293927701)
- **Job:** [93195790436](https://github.com/crussella0129/CubiKan/actions/runs/31293927701/job/93195790436)
- **Result:** pass

The final committed Sprint 6 Build head was pushed to the existing `dev`
branch. Three independent observations identify the same revision:

| Provenance observation | SHA | Result |
|------------------------|-----|--------|
| Local committed Build `HEAD` | `b6daf73cf4c12e496466ebdcb393b3204e7ffeb7` | match |
| Local remote-tracking `origin/dev` | `b6daf73cf4c12e496466ebdcb393b3204e7ffeb7` | match |
| GitHub run API `head_sha` | `b6daf73cf4c12e496466ebdcb393b3204e7ffeb7` | match |

| Hosted assertion | Observed value | Result |
|------------------|----------------|--------|
| Workflow | `Rust CI` | pass |
| Event | `push` | pass |
| Head branch | `dev` | pass |
| Run attempt | `1` | pass |
| Run status / conclusion | `completed` / `success` | pass |
| Run interval | `2026-08-09T04:08:25Z`–`2026-08-09T04:09:09Z` | pass |
| Sole job | `Rust quality gate` (ID `93195790436`) | pass |
| Job status / conclusion | `completed` / `success` | pass |
| Job interval | `2026-08-09T04:08:29Z`–`2026-08-09T04:09:08Z` | pass |

The job's setup, checkout, stable-Rust installation, `Check formatting`, `Run
Clippy`, `Check workspace`, `Run workspace tests`, `Run workspace doctests`,
post-checkout, and completion steps all reported `completed` / `success`. This
is one real push run at the exact Build SHA, not a local emulation. It used
attempt 1 with no workflow or job retry, rerun, or dependency-cache step. The
workflow configures a 15-minute job timeout; the observed 39-second job and
44-second run both completed in under one minute.

## External, flake, and claim boundary

No mocks or stubs support either E2E result. The reader journey uses the actual
checked-in Book, while the hosted result crosses the real GitHub Actions,
floating `ubuntu-latest`, Rustup current-`stable`, and crates.io
index/download boundaries. Those floating services and toolchain/image choices
are the remaining availability and reproducibility risks for future runs.

This single successful attempt proves only that the existing CubiKan quality
workflow passed at the stated revision. It does not establish ongoing CI
availability, deterministic future runner or Rust versions, an MSRV,
Windows/macOS support, coverage or security certification, release or
deployment behavior, branch-protection behavior, external non-mutation, or the
existence of any derivative repository or backend. A later draft pull-request
run is a separate handoff checkpoint and is not this exact-Build-head oracle.

## Product/runtime E2E boundary

A derivative product/runtime E2E is not currently possible because Sprint 6
delivers an advisory prose artifact and explicitly creates no derivative
repository, durable backend, or versioned runtime integration boundary.
Document navigation and hosted regression evidence must not be relabeled as
that missing product test.

The runtime E2E unlock is explicit: a later sprint must select and realize the
relevant capability or capabilities among INT-0008–INT-0012 for a chosen
derivative, and that derivative repository must be separately authorized and
implemented. Only then can a real derivative process exercise its selected,
versioned CubiKan boundary end to end. No mock derivative or speculative
service is introduced merely to manufacture evidence before those conditions
exist.
