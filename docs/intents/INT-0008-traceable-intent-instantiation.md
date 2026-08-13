# INT-0008 — Traceable intent instantiation and artifact provenance

<!-- sprint-loop-intent-v2 -->
- **Intent ID:** INT-0008
- **State:** active
- **Work evidence:** [Sprint 11 build plan](../sprints/s11/sprint-plans/build-plan.md)
- **Completion evidence:** none
- **Code evidence:** none
- **Test evidence:** none
- **Documentation evidence:** none

## Intent

Enable CubiKan to preserve the exact external intent that caused every supported
Intent Unit to be instantiated and to associate whole-unit or exact-revision
lifecycle work with externally owned artifacts such as commits, pull requests,
tests, documents, and canonical-ledger records.

Every supported Intent Unit is created with exactly one validated immutable
origin. An external reference has complete identity `(namespace, scope, value)`.
The namespace is 1 through 64 ASCII bytes with exact grammar
`[a-z][a-z0-9._-]{0,63}`; validation does not trim, fold case, or normalize.
Scope and value are nonblank, NUL-free UTF-8 text from 1 through 256 bytes,
preserved and compared byte-for-byte. CubiKan performs no URL normalization,
alias resolution, provider lookup, or inferred `latest` selection.

The blockchain lifecycle authority selected by
[INT-0014](INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
is canonical for unit creation, lifecycle revisions, relationship state,
provenance recording, and provenance revocation. Its first realization is a
pinned Polkadot SDK parachain runtime exercised on an ephemeral local
two-validator/two-collator Zombienet topology. `cubikan-core` remains
provider-neutral and chain-agnostic: it validates typed references and lifecycle
rules but contains no Book, Git, hosting, filesystem, wallet, account, signing,
RPC, FRAME, Subxt, or rendered-link client. SQLite is a verified rebuildable
projection of finalized canonical events, never a second write authority.

A recorded association means only that the canonical ledger accepted that exact
link. It does not prove authorship, causality, quality, intent satisfaction, or
provider verification. Transaction senders, block producers, signatures, blame,
and other provider metadata are not silently promoted into human or agent
attribution.

## Acceptance criteria

- Every supported creation boundary requires exactly one valid external origin.
  Missing, null, malformed, or out-of-bound origin input rejects before unit
  construction or canonical submission. There is no originless constructor,
  `None` state, unknown placeholder, synthetic legacy attribution, or in-place
  origin correction.
- Origin identity remains distinct from `IntentUnitId`, survives transitions and
  completion unchanged, round-trips through core serialization and validated
  restoration, is included in the canonical create event, and is reproduced
  exactly by a rebuilt verified SQLite projection.
- A provider-neutral `RecordedAssociation` can target either a whole Intent Unit
  or one exact aggregate revision from zero through the canonical current
  revision. Whole-unit scope and exact revision zero are distinct. Future or
  nonexistent revisions reject without canonical or projected mutation.
- The canonical ledger, not SQLite, accepts association records and revocations.
  Association work never advances the targeted unit's lifecycle revision.
  Projected state identifies the exact canonical event coordinate and verified
  checkpoint from which it was derived.
- The complete logical association identity is the Intent Unit ID, subject
  scope, and external reference. One unit may have multiple artifacts and one
  artifact may serve multiple units. Recording an already-active exact
  association rejects deterministically.
- Correction is append-only: revoke one exact active association and then record
  the intended association. These are independently canonical operations, may
  expose an intermediate absence, and provide no atomic replacement or
  idempotency-token promise. Revoking a missing or already-revoked association
  rejects. Canonical history is retained even when SQLite removes the active
  projected row.
- Bounded forward and reverse queries return only state derived through the
  verified SQLite read model, use canonical complete-key ordering, limits from
  1 through 100, exclusive cursors, validated lookahead, and identify the
  projection checkpoint. Rebuilding from the canonical deployment anchor
  produces the same query-visible state.
- At least one Git-facing adapter demonstration receives caller-owned repository
  scope, discovers the repository object format, resolves a full commit object
  ID, records it with algorithm-specific namespace `git.commit.sha1` or
  `git.commit.sha256`, and proves that later source moves or `git blame` output
  cannot rewrite the association.
- Documentation distinguishes canonical ledger acceptance, read-model
  verification, external-provider verification, human attribution, and causal or
  quality claims. No layer silently promotes one category into another.
- Canonical payloads contain references and lifecycle data only, not
  credentials, raw prompts, transcripts, source bodies, or provider secrets.
  Documentation states the selected chain's disclosure, retention, finality,
  and access consequences explicitly.
- The first canonical-ledger journey runs only on the pinned local Polkadot SDK
  runtime with public synthetic references. It proves bounded on-chain
  validation, finalized event identity, projection, and rebuild behavior but
  performs no public network, account, funding, registration, deployment, or
  governance action and makes no live shared-security claim.

## Rationale

Required origin closes the identity gap between project intent and instantiated
work. Canonical blockchain events make accepted lifecycle and provenance history
independent of one local database, while a verified SQLite projection preserves
bounded local queries without creating a competing source of truth.

A deliberately weak recorded-association relation keeps provider facts,
attribution, analytical inference, and intent satisfaction under their proper
authorities. Exact references and revision targets support reproducible
correlation without embedding provider clients in the domain core.

## Alternatives

Optional or synthetic origin was rejected because every current-generation unit
must be attributable at creation. Making SQLite authoritative was rejected
because the approved product direction assigns canonical lifecycle authority to
the blockchain. Dual-writing blockchain and SQLite was rejected because it
creates split-brain acceptance.

Physical deletion as canonical correction was rejected after selecting an
append-only blockchain authority. SQLite may remove a current-state projection
row when it applies a canonical revocation, but that is not erasure of canonical
history. Git blame, transaction sender identity, and signatures were rejected as
automatic authorship or causality claims.

## Consequences

INT-0008 depends on realized [INT-0009](INT-0009-revisioned-lifecycle-commands.md)
for exact revision semantics and on INT-0014 for canonical event ordering,
finality, verified projection, and current schema/protocol authority. It no
longer depends on SQLite as a canonical durable store.

An erroneous origin requires a distinct Intent Unit; the original canonical
record remains. Evidence correction retains canonical revocation history.
References written to a blockchain inherit that deployment's disclosure,
retention, fee, finality, and reorganization properties, so private identifiers
must not be submitted until a public deployment policy permits them. The local
Sprint 11 proof uses deterministic development signers only; their accounts
authorize calls but are not treated as authors, owners, or causal agents.

Book parsing, Git/hosting access, source rendering, analytics, attribution, and
human approval remain adapter or derivative responsibilities. The Project Book
remains canonical for project intent, sprint work, and realization history; the
blockchain is canonical for CubiKan runtime lifecycle and accepted relationship
and provenance events.

## Transition history

- 2026-08-08: created as `proposed` after Sprint 6 research identified original-intent-to-code traceability as the strongest reusable capability in the user-provided design discussion.
- 2026-08-08: revised while `proposed` to make revision-scoped evidence depend on INT-0009 and bidirectional durable indexing depend on INT-0010.
- 2026-08-11: revised while `proposed` after product approval to require one exact immutable origin on every supported Intent Unit, select `(namespace, scope, value)` reference identity, reject synthetic legacy attribution, and make earlier schema, envelope, and protocol generations unsupported rather than migrated.
- 2026-08-11: revised while `proposed` after the blockchain-authority checkpoint selected canonical ledger acceptance with SQLite as a verified rebuildable read model; provenance correction changed from physical canonical deletion to append-only revocation followed by recording the intended association.
- 2026-08-11: revised while `proposed` after the public/shared-security direction selected a Polkadot SDK parachain and a local multi-node proof as the first realization; public deployment, production governance, economic policy, key custody, and private identifiers remain follow-on outcomes.
- 2026-08-11: moved to `planned` when Sprint 11 mapped required-origin lifecycle, canonical provenance, Git identity, finalized projection, strict protocol v2, the local four-node proof, documentation reconciliation, and portable resource-measured gates to T-1101–T-1117.
- 2026-08-11: amended while `planned` to lock the exact reference-namespace grammar `[a-z][a-z0-9._-]{0,63}` before the Sprint 11 plans were finalized.
- 2026-08-12: amended while `planned` before Sprint 11 plan finalization to clarify that the multi-node proof uses distinct pinned relay and CubiKan parachain runtime identities rather than one shared runtime binary.
- 2026-08-13: moved to `active` immediately before T-1102 began the bounded provider-neutral reference, workflow, SCALE, and independent conformance-fixture implementation.
