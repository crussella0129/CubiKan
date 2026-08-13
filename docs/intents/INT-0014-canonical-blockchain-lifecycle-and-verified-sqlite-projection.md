# INT-0014 — Local Polkadot SDK canonical runtime and verified SQLite projection

<!-- sprint-loop-intent-v2 -->
- **Intent ID:** INT-0014
- **State:** active
- **Work evidence:** [Sprint 11 build plan](../sprints/s11/sprint-plans/build-plan.md)
- **Completion evidence:** none
- **Code evidence:** none
- **Test evidence:** none
- **Documentation evidence:** none

## Intent

Make one pinned Polkadot SDK parachain runtime the sole canonical acceptance
authority for current-generation CubiKan Intent Unit creation, immutable origin,
workflow snapshot, species, revisioned lifecycle commands, relationship
definitions and edges, and provenance association records and revocations.

The first realization is an ephemeral local development network, not a public
deployment. Two local relay validators and two local collators run on loopback
under Zombienet. Both collators execute one byte-identical checked CubiKan
parachain runtime and chain specification; relay validators and the collators'
relay-side services execute a separate byte-identical checked relay runtime and
chain specification. This reproduces the selected public/shared-
security execution model locally, but it does not provide independent validator
security, economic security, or production readiness.

A custom `pallet-cubikan` validates bounded SCALE commands and emits one
versioned, globally sequenced accepted event for each successful canonical
mutation. `cubikan-core` remains provider- and chain-neutral. Chain-compatible
types and conformance tests share its semantics without adding Polkadot, FRAME,
Subxt, RPC, account, or signing dependencies to the core crate.

SQLite schema version 3 is a local verified, rebuildable read model. Envelope
version 2, lifecycle histories, relationship state, provenance state, indexes,
and cursors derive only from finalized pallet events. Applications cannot write
canonical runtime truth directly to SQLite. Loss, replacement, or corruption of
the database is recovered by replaying finalized events from the immutable local
deployment anchor. `cubikan-chain-client` supplies strict finalized RPC and
submission primitives to `cubikan-backend`; the backend owns high-level
sync/attestation orchestration, every raw projection write, and construction of
non-serializable verified read capabilities. No public API accepts caller-made
blocks, events, rows, checkpoints, or capabilities.

Both JSON adapters move to their own protocol version 2 and require origin.
The stateless `cubikan` protocol remains a one-shot chain-neutral lifecycle
simulation and cannot report canonical acceptance or retain runtime state. The
`cubikan-local` protocol alone submits to the pinned local chain; its seven
mutation responses distinguish pre-send submission rejection, an unresolved
lane, proven expiry without inclusion, finalized dispatch rejection, finalized
invariant failure, delivery indeterminacy, and finalized acceptance. Projection
lag is nested only under finalized acceptance. No response reports
canonical success before the selected finality rule is satisfied. Reads expose
only rows held by a read capability scoped to one database connection, anchor,
checkpoint, and configured archive RPC. That capability is issued only after
the complete covered CubiKan event stream has been fetched from finalized
blocks through that RPC, compared byte-for-byte and coordinate-for-coordinate
with stored projected events, and independently replayed to the same typed
derived state. It is an attestation against a pinned, node-trusted RPC—not an
independent finality proof or a light client.

Schema versions 1 and 2, envelope version 1, and protocol version 1 remain
historical identities only. The current generation rejects them unchanged and
does not migrate, adopt, or assign synthetic origins to them.

The Project Book remains canonical for project intent, plans, task completion,
and sprint history. The parachain is canonical only for CubiKan runtime state.
SQLite, the Book, Git, provider APIs, and analytical derivatives cannot
dual-write or override finalized runtime events.

## Acceptance criteria

- The repository pins one compatible Polkadot SDK `polkadot-stable2606-1`
  release at commit `8ae9775dc43c0d8cdd0f6d87700596e14278b1e1`, Rust 1.93,
  `wasm32v1-none` target, Subxt 0.50.2, same-commit in-tree parachain-template basis,
  relay and collator binaries, chain-spec tooling, and Zombienet source commit
  `a7c434271f094320d17cf94f7a2f95fdef417379`. Lockfiles, source revisions,
  and downloaded-binary checksums make the tested toolchain reproducible.
- The chain implementation lives in an isolated nested Rust workspace so the
  existing root workspace, dependency graph, lockfile, and fast quality gate do
  not silently acquire the Polkadot SDK graph. Native and Wasm builds use the
  same checked sources, metadata, runtime specification version, and code hash.
  Root client crates may use the exact pinned Subxt/RPC/SCALE graph, and
  `cubikan-backend` depends on `cubikan-chain-client`; no reverse dependency or
  public raw projection-write/capability-construction seam exists.
- Chain values are bounded SCALE types with deterministic validation and
  `MaxEncodedLen`. The runtime fixes explicit maxima for every string and
  collection, including workflow phases and edges, relationship traversal, and
  active provenance associations. Shared core vocabulary and workflow values
  enforce the common byte/collection bounds; chain-only authorization and
  storage-capacity errors remain distinct. Oversize or graph-bound input rejects
  before mutation or accepted event; dispatch weights cover worst-case bounded
  work. Every accepted event variant mechanically proves
  `MaxEncodedLen <= 1,048,576`, the exact SQLite raw-event ceiling.
- Every lifecycle, relationship, and provenance dispatch carries unsigned
  16-bit command-schema version `1`. Another decodable version returns typed
  `UnsupportedCommandSchemaVersion` before domain reads, mutation, or accepted
  event.
  Missing, null, malformed, or SCALE-over-bound requests fail structurally in
  the adapter/codec before dispatch; event-schema version is runtime-produced
  and is validated by the projector rather than supplied by a caller.
- Atomic domain rejection means `pallet-cubikan` storage and the global domain
  event sequence remain unchanged and no accepted CubiKan event is emitted.
  Codec or transaction-validity rejection before inclusion consumes no nonce or
  fee. A well-formed signed extrinsic that is included and returns a typed pallet
  dispatch error still consumes its nonce, pays ordinary transaction fees, and
  produces the runtime's normal system failure evidence; those effects are not
  misdescribed as a full-chain rollback.
- Canonical create requires a valid caller-supplied UUID, exactly one valid
  immutable origin `(namespace, scope, value)`, species, and complete immutable
  workflow snapshot. Missing, null, malformed, duplicate, unsupported, or
  out-of-bound input rejects transactionally without storage mutation or an
  accepted event. Reference namespace uses exact ASCII grammar
  `[a-z][a-z0-9._-]{0,63}` with no trimming, case folding, or normalization.
  The runtime never generates randomness.
- Canonical transition and completion carry the caller-observed revision.
  Accepted commands advance revision exactly once and emit exactly one ordered
  event. Stale revision rejects before lifecycle-domain validity, preserving the
  realized INT-0009 behavior.
- Lifecycle revision is an unsigned 64-bit value starting at zero and no value
  is reserved as a sentinel. Current-generation units retain at most 256
  lifecycle records, so revision remains within `0..=256`; with a current
  expectation at capacity, `LifecycleHistoryCapacityExceeded` returns before
  lifecycle-domain validity and changes no CubiKan storage or accepted event.
  The cap is part of the deterministic runtime/storage/serialization contract,
  not a claim that the numeric type can be operationally driven to `u64::MAX`.
- Canonical relationship definition and edge commands carry forward INT-0012's
  exact definition/version identities, endpoint and species validation,
  self-edge and cycle policy, duplicate precedence, exact deletion/recreation,
  and operation-selected fail-closed behavior within the declared finite graph
  bounds. Definition creation checks version/origin, then duplicate, then global
  sequence. Edge creation checks version/origin, definition, source, target,
  source species, target species, self policy, duplicate, edge capacity, cycle,
  then global sequence. Edge deletion checks version/origin, definition, source,
  target, source species, target species, exact active edge, then global
  sequence; it has no capacity check. Accepted events are sufficient to
  reconstruct the same live direct queries and projection-v1 results in SQLite.
- Canonical provenance commands carry forward INT-0008's required origin,
  whole-unit and exact-revision subjects, exact external-reference identity,
  duplicate rejection, append-only revocation, bidirectional query, and
  non-attribution semantics. Record checks version/origin, unit, exact revision
  and reference, duplicate, active-association capacity, then global sequence.
  Revoke checks version/origin, unit and reference, exact active target, then
  global sequence and never checks active capacity. Recording or revoking
  evidence never advances the lifecycle revision.
- Every lifecycle, relationship, and provenance call requires a directly signed
  `AccountId32` on a maximum-16 technical submitter allowlist. Any authorized
  submitter may invoke domain commands; signer identity is authorization and
  fee-payment metadata, not unit ownership, human authorship, responsibility, or
  causality. The pallet exposes a bounded Root-only allowlist replacement, but
  the fixed local runtime has no sudo, proxy, utility, multisig, dispatch-as, or
  other origin-transforming wrapper and exercises no reachable Root route; its
  genesis allowlist therefore remains fixed throughout the sprint journey.
  Local genesis funds two development submitters distinct from validator and
  collator roles.
- The local runtime retains ordinary balances and transaction-payment behavior;
  calls use normal weight/length fees and zero default tip. Development balances
  have no economic meaning. No fee waiver, faucet, sponsor, reimbursement, or
  production token policy is implied.
- One immutable local deployment anchor records namespace
  `polkadot-sdk-parachain`, relay genesis hash, parachain genesis hash, local
  ParaId `1000`, a 32-byte CubiKan deployment ID, pallet storage version, event
  schema version, initial runtime specification version, and initial runtime
  code hash. The runtime is not upgraded during the sprint journey.
- The anchor is a checked post-genesis manifest rather than self-referential
  pallet genesis state: relay genesis and parachain block-zero hashes come from
  the pinned RPCs, ParaId/deployment ID/pallet and event versions come from
  on-chain state/metadata, and runtime code identity is the hash of block-zero
  `:code`. CubiKan deployment-anchor/pallet genesis stores only those
  non-self-referential CubiKan fields; the overall pinned chain specification
  also contains the standard System/parachain/session/collator/Aura/balances
  genesis needed to operate the local network. The exact
  canonical JSON manifest is committed at
  `chain/artifacts/local-deployment-anchor-v1.json`, its SHA-256 is pinned in
  `chain/pins.toml`, the local harness re-verifies both RPC provenances, and
  every root client accepts only those fixed bytes before SQLite copies the
  verified composite.
- Each accepted event carries deployment ID, event-schema version 1, a checked
  global CubiKan sequence, and a complete bounded replay payload. Its stable
  projected coordinate is parachain genesis hash, deployment ID, finalized
  parachain block number and hash, extrinsic index, canonical extrinsic hash,
  system event index, and global sequence; signer is supplemental authorization
  metadata rather than event identity. SQLite stores the event's block number and obtains its
  block hash only through the restrictive foreign-key join to the immutable
  projected-block row; public coordinates and attestation always include and
  validate the joined hash.
- Global CubiKan sequence is unsigned 64-bit, begins at one for the first
  accepted event, and reserves no in-band sentinel. A chain with no accepted
  CubiKan event represents `last_global_sequence` as absent in its projection
  checkpoint. At `u64::MAX`, any otherwise acceptable canonical mutation returns
  typed sequence exhaustion before changing domain storage or emitting an
  accepted event. A checkpoint through blocks before the first CubiKan event has
  absent last sequence. A later zero-CubiKan-event block retains the prior
  nonzero checkpoint sequence while that block's own first/last fields are null.
- The projector backfills and subscribes through Subxt using finalized
  parachain blocks only. It verifies the deployment anchor, parent-hash
  continuity, event sequence, runtime specification/code identity, event count,
  and replay result; applies each coordinate once; and atomically commits all
  block events with its checkpoint. Best or merely included blocks never enter
  verified SQLite state.
- A fresh schema-v3 database is schema-only and exposes no readable checkpoint.
  The backend's first finalized sync transaction verifies the fixed deployment
  manifest and atomically inserts the anchor row, parachain block-zero projected
  row with zero CubiKan events, and block-zero checkpoint with absent last
  sequence before any capability can exist.
- Duplicate, skipped, out-of-order, malformed, wrong-genesis, wrong-deployment,
  wrong-runtime, unsupported-version, nonfinalized, and conflicting-finalized
  inputs cannot advance the checkpoint or expose a partial page. A conflicting
  finalized hash is a fatal trust failure requiring operator recovery, never a
  heuristic rollback.
- Public projection reads require an RPC-attested capability scoped to the exact
  database connection, immutable anchor, stored checkpoint, configured archive
  RPC, and the same SQLite read transaction used by exactly one page/read.
  Before issuing it, the client fetches every finalized CubiKan event from the
  anchor through that transaction's checkpoint, compares the exact raw event
  bytes and complete coordinates with `projected_events`, replays that stream
  through an independent in-memory model, and requires semantic equality with
  every derived lifecycle, relationship, provenance, and checkpoint row. The
  capability expires with the transaction; every later live page obtains a new
  snapshot and attestation. A cache with genuine anchor/checkpoint metadata but
  substituted events or coherently substituted derived rows rejects. This
  protects against a database-only forgery relative to that RPC; it does not
  authenticate a compromised RPC, independently prove consensus finality, or
  provide a light client.
- No mutation preflight, deployment selection, revision inference, nonce choice,
  or signing decision may derive from SQLite, attested or otherwise. Mutation
  inputs are explicit protocol values and canonical RPC state; a coherently
  forged cache therefore causes zero signer calls and zero sends.

Both protocol-v2 identities are fixed by independently authored, separately
SHA-256-hashed schemas and positive/negative fixture corpora. Their exact
operation names, field names, encodings, response variants, and error-code
inventories are selected during Plan rather than snapshotted from the eventual
implementation.
- Fresh SQLite files initialize exact schema version 3 and accept only envelope
  version 2 through crate-private projection authority. Existing public backend
  write methods cannot create or mutate v3 canonical state. Schema v1/v2,
  envelope v1, extra objects, direct edits, corrupt projections, and wrong
  checkpoints fail closed without adoption or repair.
- Envelope v2 retains the complete ordered lifecycle history. The 256-record
  runtime cap and all other declared text/graph bounds must mechanically prove
  that the maximally escaped canonical JSON representation remains at or below
  2,097,152 bytes; maximum-plus-one lifecycle mutation rejects canonically
  before a unit can become unprojectable.
- SQLite opens with bound-value SQL and verified defense-in-depth settings,
  including no-follow path handling, defensive and query-only separation,
  trusted/writable schema disabled, double-quoted strings disabled, extension
  loading and ATTACH capability denied per connection, and a private static
  comment-free production-SQL inventory with no caller-supplied SQL,
  bounded SQLite limits, UTF-8 encoding, memory-only temporary storage, zero
  memory mapping, a 5,000-ms SQLite busy handler with no application retry,
  full integrity and foreign-key checks, exact schema validation, and
  least-authority file guidance. Bundled compile options
  `ENABLE_LOAD_EXTENSION` and `USE_URI` are accepted and recorded; every path
  passed to SQLite is canonical absolute and therefore cannot start with
  `file:`, `SQLITE_OPEN_URI` and `ATTACH` are absent, and `load_extension()` is
  denied. Every connection also sets and reads back
  `SQLITE_DBCONFIG_ENABLE_COMMENTS=false` before schema SQL; the static
  production-SQL inventory remains comment-free as a separate source oracle.
  Tests distinguish SQL injection strings from hostile-file/path/resource risks.
  Structural, integrity, and replay checks do not by themselves authenticate a
  coherently forged cache; public reads require the full RPC-stream comparison
  and independent typed replay capability described above.
- The proof supports Linux only, on a test-owned local filesystem whose default
  VFS honors advisory locks, same-directory atomic rename, regular-file and
  directory `fsync`, and DELETE-journal locking. Projection and signer-journal
  creation uses mode `0600`; every open/reopen requires regular non-symlink
  owner-matching files with no group/other bits. Non-Linux, network filesystems,
  DrvFS, FUSE, custom VFSes, and filesystems that cannot establish those
  semantics reject before SQLite/journal access; no Windows ACL equivalence is
  claimed. The directory is canonicalized once and SQLite receives only its
  absolute path joined to one validated direct-child basename; a `file:`-shaped
  basename is therefore a tested literal child, not a URI-leading path.
- Existing/unknown files are first opened OS read-only with no-follow, defensive
  configuration, numeric limits, and cell-size checking before schema-dependent
  SQL. Preflight requires database encoding UTF-8, page size 4096, page count at
  most 262,144, full `PRAGMA integrity_check` returning exactly one `ok`, and
  `PRAGMA foreign_key_check` returning no rows. Only a fully accepted
  current-generation file may be reopened for the crate-private writer and
  revalidated under its lock. Each writer sets and reads back
  `max_page_count=262144` before writes; this connection setting is never treated
  as persistent file state. The exact main file and
  journal/WAL/SHM sidecar paths reject when symbolic or nonregular; an unexpected
  hot journal fails for disposable rebuild rather than being recovered before
  trust. The current generation pins DELETE journaling and synchronous EXTRA.
- Deleting SQLite and replaying finalized events from the local genesis anchor
  through either archive-capable collator reconstructs semantically equal units,
  lifecycle histories, definitions, edges, provenance history and active
  associations, direct relationship queries, projection-v1 pages, provenance
  queries, and checkpoint metadata.
- Both adapter-owned protocol version 2 boundaries preserve the one-MiB request
  bound, omitted-versus-null ID distinction, checked body/newline/flush
  delivery, typed rejection, and process exit mapping while adding required
  origin. `cubikan` returns simulation results only. `cubikan-local` adds chain
  submission outcomes, stable ledger coordinates, and projection status.
  Protocol v1 rejects before any applicable RPC or SQLite access.
- The two protocol-v2 contracts have separate checked schema/fixture files with
  independent committed SHA-256 hashes. They close every JSON object to unknown
  or duplicate members and lock the top-level envelopes, complete operation and
  field inventories, UUID/hash/unsigned-integer/cursor/coordinate encodings,
  success and failure response unions, and exact adapter-owned error codes.
  Implementation-generated snapshots are not accepted as the oracle.
  `cubikan-local` request, read-miss, RPC/archive, storage, attestation,
  refresh, and Busy failures use exactly
  `{protocol_version:2,outcome:"error",error:ErrorDetail}` with no nullable
  success, partial page, operation, era, coordinate, or result; only mutation
  delivery uses the seven separately tagged outcome variants named above.
- `cubikan-local` accepts only raw canonical lowercase `ws://` loopback RPC URLs:
  four decimal IPv4 octets with no alternate spelling inside `127.0.0.0/8`, or
  exact bracketed `[::1]`, followed by an explicit canonical decimal port in
  `1..=65535` except default port `80`, and the literal `/` path. Raw and parsed
  spelling must round-trip. An omitted raw path, hostnames, scheme or
  host case variants, leading-zero/short/integer/octal/hex IPs, percent encoding,
  user information, query/fragment components, redirects, and non-loopback
  addresses reject before any dial. The local adapter accepts named development
  signers only and never raw seed or private-key material.
- Mutation submission is synchronous only through a default 120-second finalized
  wait. `finalized_accepted` requires a successful finalized extrinsic and
  exactly one matching accepted event inside that exact extrinsic, matched by
  finalized extrinsic index/hash, deployment/version, signer, and call identity.
  A successful inclusion with zero, multiple, or wrong CubiKan accepted events
  is a durable finalized-invariant failure rather than acceptance or dispatch
  rejection. Known signer/fee/pre-send failures, unresolved lanes, proven
  expiry, finalized dispatch rejection, watcher timeout or loss, and projection
  lag are distinct. Indeterminate delivery is never retried automatically or
  described as rollback; finalized acceptance remains canonical even if SQLite
  is lagging.
- Cross-process nonce safety uses a versioned, owner-only, noncanonical
  per-signer submission journal outside the projection database. Its prepared
  record is an exact 256-byte canonical encoding with format version, state,
  deployment, signer, nonce, expected extrinsic hash, 64-block era, signing
  finalized block number/hash, absolute inclusive birth/death heights, the
  original mutation-operation tag, resolution coordinate, and a SHA-256
  checksum. Reconciliation always uses that persisted original operation rather
  than a later request's operation. A mutation process acquires a persistent mode-`0600`
  no-follow lock inode, reconciles prior state, and publishes the prepared record
  through one fixed derived same-directory `.tmp` direct child using
  `O_EXCL|O_NOFOLLOW`, complete
  write/checksum, file `fsync`, atomic rename, and parent-directory `fsync`
  before submission. The lock remains held through a known final outcome.
  On restart under that lock, an existing temporary child is never adopted: a
  symlink, nonregular file, wrong owner/mode, or oversized object fails closed;
  the sole owner-mode-`0600` regular temporary child (complete or torn) is
  removed and the parent directory fsynced before a new publication. Thus one
  crash cannot strand the lane or accumulate unbounded orphan temp files.
  Timeout, response loss, or process interruption leaves the lane unresolved; a
  later process submits nothing until it finds that exact hash finalized or the
  finalized head passes death and a complete birth-through-death finalized-block
  scan proves the exact hash absent. Only then may it publish
  `expired_not_included`, clear safely, and refresh nonce; expiry alone never
  proves non-inclusion. Resolution uses the same durable publication sequence.
  The journal contains no secret, is never canonical domain state, and cannot
  justify automatic retry. Absence means no recorded unresolved state and may be
  initialized on first use; same-user deletion of an unresolved record is
  explicitly undetectable and outside the coordination claim. Corrupt,
  non-owner, permissive, symbolic, conflicting, or checksum-invalid state fails
  closed. Its direct-child filename
  is derived—not caller-selected—from the canonical projection path, deployment
  ID, and signer `AccountId32`, so cooperating CubiKan processes using that
  projection converge on one lane. An alternate projection path or external
  program using the same signer is outside that coordination boundary; observed
  nonce disagreement fails reconciliation and creates no retry or safety claim.
- A local Zombienet test starts two relay validators and two archive-capable
  collators with four fixed distinct public-to-the-test loopback RPC endpoints,
  fixed unique primary P2P/metrics ports, and fixed unique collator relay-side
  RPC/P2P/metrics ports. A repository-owned, SHA-pinned argv-normalizing
  launcher accepts only the exact pinned Zombienet-generated grammar, removes
  every external-bind flag, supplies loopback-only listen addresses and
  bootnodes, and rejects missing/duplicate/unknown arguments before launch.
  PID command-line and socket inspection must find no wildcard/public listener
  or listener outside that complete four-node-process inventory; the Zombienet
  orchestrator is tracked separately and is not a fifth blockchain node.
  Both collators execute byte-identical CubiKan parachain Wasm. Both relay
  validators and both collator relay-side services use one byte-identical relay
  runtime and chain specification; relay and parachain runtime bytes are
  deliberately distinct and never compared as one artifact. Both collators pin
  `--blocks-pruning=archive --state-pruning=archive`; probes cover
  historical hash, header, body, events, and `:code` across the required range.
  Both collator
  endpoints converge on the same finalized state, metadata, specification
  version, and code hash; after one collator stops, the remaining collator
  finalizes the rest of the required-origin lifecycle, relationship, and
  provenance journey. The stopped collator is then restarted and must catch up
  to the survivor's named finalized hash, code hash, metadata, and checkpoint
  before either collator is used for the two independent rebuilds.
- The Git demonstration uses a real temporary repository, detects SHA-1 or
  SHA-256 object format, records a repository-qualified full commit reference
  under `git.commit.sha1` or `git.commit.sha256`,
  renames source, and observes blame without mutating the canonical association
  or claiming authorship, causality, verification, or intent satisfaction.
- The sprint uses only loopback RPC, ephemeral chain state, deterministic
  development accounts, and public synthetic fixtures. It performs no public
  RPC mutation, account creation, key import, token transfer, ParaId reservation,
  registration, coretime purchase, runtime upload, deployment, release, or
  governance action. Tests and documentation call the result a local execution
  and projection proof, not live Polkadot shared security or production safety.
- Existing root-workspace lifecycle, relationship, query, bounded-ingestion,
  flush, corruption, and atomic-rejection tests remain green. Separate pinned
  chain checks cover pallet, runtime Wasm, projector, protocol v2, and the local
  network without weakening the existing warnings-denied quality gate.
- The existing root CI workflow and terminal INT-0010/INT-0012/INT-0013 chapters
  remain byte-identical. A separate manually dispatched chain workflow fetches
  exact dependencies first and then runs locked/offline, keys caches by exact
  toolchain/SDK/locks/artifact hashes, and records budgets of cold at most 90
  minutes, warm at most 30 minutes, peak workspace-plus-cache disk at most 60
  GiB, and Zombienet at most 30 minutes. A repository-local Book validator makes
  the gate portable outside the planning author's machine.

## Rationale

A canonical blockchain runtime gives CubiKan one independently replayable
acceptance history while retaining a chain-neutral domain model. A Polkadot SDK
parachain is the closest public/shared-security target to CubiKan's Rust-first
design. FRAME provides bounded deterministic state transitions and Subxt exposes
finalized events to a Rust projector.

The first proof deliberately runs one pinned relay-runtime identity and one
distinct pinned CubiKan parachain-runtime identity across the appropriate sides
of a local relay/parachain topology. Separate processes and ports validate consensus-facing composition,
finality consumption, failover, and rebuild behavior without prematurely
creating public accounts, moving funds, or claiming economic security.

A new generation is cleaner than inventing origins or migrations for software
that has never been live. Preserving historical version identities keeps the
Book and checked-in evidence truthful while allowing the current generation to
start with mandatory provenance.

## Alternatives

Keeping SQLite canonical, treating blockchain as an audit mirror, and dual-write
conflict resolution were rejected because each leaves two competing acceptance
authorities. Trusting an off-chain adapter to emit canonical events without
on-chain enforcement was rejected because adapter compromise could create
canonical invalid state.

Arbitrum Stylus, Solana, and Hyperledger Fabric were considered. They can host
Rust-adjacent or governed programs, but their settlement, account, compute,
sequencer, privacy, language, or consortium policies fit the selected public
shared-security target less directly than a custom Polkadot SDK parachain.

A single-node development chain was rejected as the only integration oracle.
The local proof uses multiple relay validators and collators so each of the two
runtime identities and the CubiKan event model cross their actual process and
consensus boundaries. It still does not
substitute for a public network.

Synthetic origin, legacy migration, protocol-number reuse, and physical deletion
of canonical evidence were rejected because no deployed state requires those
compatibility compromises.

## Consequences

This realization adds a large, pinned, separately built Polkadot SDK workspace,
Wasm runtime, local network harness, Subxt client, and projector. Build time,
disk use, CI timeout, binary provenance, port/process cleanup, runtime weights,
and dependency security become explicit engineering concerns.

Canonical history cannot promise erasure. Public deployment would make submitted
identifiers, signatures, state, and events broadly inspectable and persistent.
The local sprint therefore permits synthetic public fixtures only and stores no
source bodies, credentials, prompts, transcripts, seed material, or private
repository locators.

Finality can increase latency and chain unavailability can prevent writes.
Projection unavailability can delay reads without changing chain acceptance.
Timeout, RPC loss, and response loss create indeterminate delivery requiring
reconciliation rather than blind retry.

The ordinary Subxt RPC path trusts the configured archive node's finalized
view. Full event-stream comparison detects database-only substitution relative
to that node, including coherent local forgeries that preserve anchor and tip
metadata, but does not replace consensus proof verification or defend against a
colluding node-and-database compromise. A public deployment must separately
select an authenticated RPC, multi-provider, or light-client trust model.

No-follow flags and metadata checks do not prove race-free path identity on
every VFS and do not defend against a same-user hard-link, parent-directory
replacement, or deletion of an unresolved submission journal followed by
first-use initialization. The Linux local-filesystem proof therefore uses a
stable, test-owned, owner-only directory without elevated or confused-deputy
execution and promises no continuous protection after a checked descriptor is
opened. Journal durability covers ordinary process kill and the acknowledged
local-filesystem file/rename/directory-fsync contract; lying storage hardware,
power loss beyond that contract, same-user journal deletion, and unsupported
filesystems remain explicit nonclaims. The journal is never canonical recovery
evidence.

The local proof authorizes deterministic dev signers, fixed genesis, a fixed
runtime, normal but economically meaningless dev fees, and loopback archive
nodes only. A public Paseo or Polkadot deployment requires a follow-on intent and
human approval for public ParaId and coretime, runtime governance and upgrade
origins, submitter roles and key custody, fee/deposit/token policy, benchmarked
capacity, data disclosure/privacy, archive and RPC operations, monitoring and
incident response, external security review, funding, and release coordination.

This intent succeeds the exact SQLite-canonical and protocol-version-specific
authorities it replaces. Historical realization evidence remains preserved in
their terminal chapters.

## Transition history

- 2026-08-11: created as `proposed` after product approval selected blockchain as CubiKan's canonical runtime lifecycle authority and SQLite as a verified rebuildable read model rather than an independent write authority.
- 2026-08-11: revised while `proposed` after the public/shared-security direction selected a Polkadot SDK parachain and the first realization was bounded to a two-validator/two-collator local Zombienet proof with every public deployment action excluded.
- 2026-08-11: moved to `planned` when Sprint 11 mapped the pinned bounded runtime, complete lifecycle/relationship/provenance authority, finalized Subxt projection, exact hardened SQLite v3, adapter-owned protocol v2, and the local four-node proof to T-1101–T-1117.
- 2026-08-11: amended while `planned`, before plan finalization, to lock full archive-RPC stream attestation and single-page snapshot scope, the 256-record lifecycle/envelope bound, exact counter and rejection accounting, post-genesis anchor provenance, Root-only allowlist administration without origin-transforming wrappers, Unix hostile-path limits, strict loopback parsing, crash-recoverable derived signer journals, and collator resynchronization.
- 2026-08-11: amended while `planned`, before plan finalization, after the Plan Critic's blocking review to lock the Linux local-filesystem boundary, backend-owned projection seam, exact SQLite runtime settings, crash-safe signer-lane publication and birth/death reconciliation, independently hashed protocol-v2 schemas, operation-specific rejection precedence, archive/resource budgets, portable offline gates, and the split T-1116 documentation/T-1117 CI sequence.
- 2026-08-12: amended while `planned`, before plan finalization, to distinguish the relay and CubiKan parachain runtime identities, close all seven mutation outcomes, preserve original operation identity through crash recovery, and require canonical raw loopback RPC spelling.
- 2026-08-12: moved to `active` immediately before T-1101 began the pinned isolated toolchain and runtime foundation.
