# Sprint 11 Research Report

## Intents Reviewed

- [INT-0008 — Traceable intent instantiation and artifact provenance](../../../intents/INT-0008-traceable-intent-instantiation.md) — reviewed and selected as the recommended Sprint 11 provenance intent; required origin, greenfield versioning, and local-reference policy were approved during Research.
- [INT-0011 — Lifecycle checkpoints and metric evidence](../../../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md) — reviewed and not selected; its clock, observation, correction, denominator, window, and privacy semantics remain materially broader than INT-0008's reference contract.
- [INT-0009 — Revisioned lifecycle commands and atomic conflict rejection](../../../intents/INT-0009-revisioned-lifecycle-commands.md) — reviewed as the realized exact-revision prerequisite.
- [INT-0010 — Durable multi-unit CubiKan backend](../../../intents/INT-0010-durable-intent-unit-backend.md) — reviewed as the historical realized durability, transaction, migration, and bounded-query precedent; it is now superseded by INT-0014's current-generation chain authority and rebuildable projection.
- [INT-0012 — Intent Unit relationships and board projections](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) — reviewed as historical realized precedent for immutable typed identities, explicit correction, schema evolution, transactional validation, and live keyset queries; INT-0014 now supersedes its storage authority while carrying its relationship semantics forward, and its unit-to-unit edges are not reused as artifact relations.
- [INT-0013 — Maintain derivative ecosystem current-state accuracy](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) — reviewed as the realized Sprint 10 documentation-maintenance authority; INT-0014 now supersedes its current-state boundary while preserving the advisory appendix and assigning its blockchain/projection reconciliation to Sprint 11 documentation work.
- [INT-0014 — Local Polkadot SDK canonical runtime and verified SQLite projection](../../../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md) — created as the new authority contract after Research selected a canonical chain state machine and a rebuildable SQLite projection, then moved to `planned` once this report resolved the local chain, runtime, signer, finality, privacy, and deployment-exclusion policies.

## 1. Sprint Goal

Make INT-0008 and INT-0014 planning-ready and realize the first complete local
"blockchain Kanban" vertical. A pinned Polkadot SDK parachain runtime becomes
the sole acceptance authority for required-origin Intent Units, revisioned
lifecycle commands, bounded relationship definitions and edges, and provenance
association records and revocations. Two relay validators and two collators run
one pinned relay runtime across relay-side services and a distinct byte-identical
CubiKan parachain runtime across the collators on fixed loopback ports. SQLite becomes a
hardened, verified, rebuildable projection of finalized pallet events rather
than a second writer.

The vertical is a deliberate greenfield current-generation reset. It adds
bounded chain types, a FRAME pallet and runtime, a finalized Subxt submission
and projection boundary, stored envelope version 2, exact SQLite schema version
3, and protocol version 2. Schema versions 1 and 2, envelope version 1, and
protocol version 1 remain historical identities but are unsupported current
creation or migration paths. They fail closed without adoption.

Every supported Intent Unit requires exactly one immutable external origin.
Whole-unit and exact-revision associations are canonical, revocable through
append-only events, and queryable in both directions. One Git demonstration
preserves repository-qualified full commit identity without treating moves,
blame, verification, attribution, or causality as CubiKan state. The sprint
uses only development accounts and synthetic public fixtures, performs no
public-chain action, and makes no live shared-security or production claim.

## 2. Existing Code Survey

| File | Relevance | Finding |
|------|-----------|---------|
| `docs/intents/INT-0008-traceable-intent-instantiation.md` | high | The complete acceptance boundary is already stated, but namespace, correction, repository identity, private-source retention, and verification policy are explicitly reserved before planning. |
| `docs/intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md` | medium | Shares the realized revision/storage prerequisites but also requires trusted time, numerical aggregation, observation identity, late-arrival, correction, and privacy rules; it is the less bounded next sprint. |
| `docs/intents/INT-0009-revisioned-lifecycle-commands.md` | high | Supplies clock-free exact aggregate revisions and stale-first mutation semantics. |
| `docs/intents/INT-0010-durable-intent-unit-backend.md` | high | Supplies replay-validated envelopes, transactions, schema ownership, live pagination, and the explicit migration precedent. |
| `docs/intents/INT-0012-intent-unit-relationships-and-board-projections.md` | high | Supplies reusable policy precedents for exact composite identity, duplicate rejection, physical delete/create correction, operation-selected fail-closed validation, and canonical live queries. |
| `crates/cubikan-core/src/intent_unit.rs` | high | `IntentUnit` has immutable identity/species/workflow plus lifecycle state and revision, but no external origin. The new current-generation constructor must require origin; originless construction is not retained as a supported path. |
| `crates/cubikan-core/src/vocabulary.rs` | high | Existing vocabulary preserves caller text but only rejects blank values; provenance namespaces need a stricter bounded ASCII grammar while scope/value fields remain exact bounded text. |
| `crates/cubikan-backend/src/model.rs` | high | `CreateIntentUnit` and `IntentUnitView` currently have no origin. Current-generation commands and views must require/expose it; missing or null origin rejects before storage. |
| `crates/cubikan-backend/src/stored.rs` | high | Envelope v1 is exact, complete, and replay-validated. Required origin belongs in a distinct exact envelope v2; mixed envelope generations are rejected. |
| `crates/cubikan-backend/src/schema.rs` | high | Schema v2 owns an exact twelve-object set and constrains envelope version to 1. Provenance requires a distinct exact schema v3 carrying forward relationship behavior and adding association storage. |
| `crates/cubikan-backend/src/migration.rs` | high | The existing migration proves why version identities matter, but no deployed data exists. Current-generation code can remove migration success paths and retain only fail-closed rejection of old schema versions. |
| `crates/cubikan-backend/src/sqlite.rs` | high | Public lifecycle and relationship operations already separate cached schema capability, replay validation, immediate write transactions, and commit-before-return behavior. |
| `crates/cubikan-backend/src/relationship.rs` | medium | Typed exact identities, bounded values, errors, pages, and cursors are useful design patterns, but artifact references need their own public model. |
| `crates/cubikan-backend/src/relationship_store.rs` | medium | Provides transaction and selected-corruption patterns; artifact associations cannot reuse relationship edges because those require two Intent Unit endpoints and species validation. |
| `crates/cubikan-local/src/protocol.rs` | high | Protocol v1 cannot provide a required origin. Current-generation create/get/list/mutation shapes need protocol v2; v1 must reject as unsupported rather than be silently extended. |
| `crates/cubikan-cli/src/protocol.rs` | high | The stateless adapter also creates units and therefore needs its own required-origin protocol v2, but it remains an in-memory validator/simulator and must never report chain acceptance or retain canonical state. |
| `crates/cubikan-local/tests/protocol.rs` | medium | Exact request/response and unknown-field tests define the regression behaviors that protocol v2 must preserve while adding required origin and rejecting v1. |
| `crates/cubikan-backend/README.md` | medium | Documents exact schemas, migration, fail-closed loading, local security limits, and live pagination that schema v3 documentation must extend without stronger guarantees. |
| `docs/appendix/potential-derivative-projects.md` | high | Requires repository-qualified full Git identities, recorded-versus-verified separation, caller-owned privacy/governance, and a normal checkpoint before INT-0008 advances. |
| root `Cargo.toml` / `Cargo.lock` | high | The fast Rust-2024 workspace is unsuitable for silently absorbing the large Rust-2021/resolver-2 Polkadot SDK graph. The chain runtime belongs in an isolated nested workspace with its own toolchain and lockfile. |
| `crates/cubikan-core/src/workflow.rs` and `intent_unit.rs` | high | Current `std`, `HashSet`, unbounded `String`/`Vec`, Serde, and UUID-v4 generation cannot execute in a deterministic FRAME Wasm runtime. Bounded SCALE types need conformance tests against core; UUIDs are generated off-chain and submitted as 16 bytes. |

## 3. External Sources

- [Git hash-function transition documentation](https://git-scm.com/docs/hash-function-transition/2.52.0.html) — Git object names may be full 40-hex SHA-1 or 64-hex SHA-256 identities and the object format is repository configuration. The adapter must therefore record the object format explicitly rather than hard-code one hash width. Git does not supply CubiKan's cross-provider repository namespace policy.
- [W3C PROV Data Model](https://www.w3.org/TR/prov-dm/) — the standard distinguishes entities, activities, agents, derivation, and responsibility. That supports a deliberately weak recorded-association relation instead of silently promoting a reference into authorship, causality, verification, or intent satisfaction.
- [SQLite: Defense Against The Dark Arts](https://www.sqlite.org/security.html) — SQLite distinguishes ordinary bound-value safety from the stronger precautions needed for hostile database files: untrusted schemas, native parser exposure, memory/CPU pressure, defensive connection configuration, integrity checking, cell-size checking, and disabled memory mapping.
- [SQLite binding API](https://www.sqlite.org/c3ref/bind_blob.html) — host parameters keep caller values separate from SQL statement text. Current CubiKan queries already follow this pattern; new provenance queries must do the same.
- [SQLite database-connection configuration](https://www.sqlite.org/c3ref/c_dbconfig_defensive.html) — SQLite 3.49+ exposes `SQLITE_DBCONFIG_ENABLE_COMMENTS` alongside defensive, trusted-schema, DQS, ATTACH, trigger/view, and related per-connection controls. The pinned 3.53.2 build can therefore disable and read back SQL comments before schema work instead of relying only on static SQL review.
- [Polkadot SDK runtime customization](https://docs.polkadot.com/parachains/customize-runtime/) — FRAME supplies Rust-native storage, dispatchable calls, errors, and events for a custom deterministic state-transition runtime. This is the best default fit for CubiKan's Rust-first domain model.
- [Polkadot Subxt client](https://docs.polkadot.com/reference/tools/subxt/) — the Rust client supports finalized-block subscriptions and event access needed by a SQLite projector; complete rebuilding also requires an explicit archive-data boundary.
- [Polkadot network progression](https://docs.polkadot.com/reference/parachains/networks/) — the documented path is local development and multi-node Zombienet testing, then the public Paseo TestNet, then Polkadot MainNet. Sprint 11 can verify the runtime locally without performing an externally visible deployment.
- [Polkadot Zombienet local network](https://docs.polkadot.com/tutorials/polkadot-sdk/testing/spawn-basic-chain/) — a native test network can run multiple relay validators and parachain collators on distinct ports; the approved model pins one relay runtime/specification across relay-side services and a separate CubiKan parachain runtime/specification across collators.
- [Zombienet network-definition specification](https://paritytech.github.io/zombienet/network-definition-spec.html) — collator arguments apply to the parachain side while its relay-chain side is generated separately. The fixed local proof therefore audits and normalizes the complete generated argv/socket inventory rather than assuming one listener set per collator.
- [Polkadot SDK stable2606-1 release](https://github.com/paritytech/polkadot-sdk/releases/tag/polkadot-stable2606-1) — node release 1.24.1 at commit `8ae9775dc43c0d8cdd0f6d87700596e14278b1e1` is the selected immutable SDK family; its documented build toolchain is Rust 1.93.
- [Polkadot SDK Wasm builder](https://paritytech.github.io/polkadot-sdk/master/substrate_wasm_builder/index.html) — for Rust 1.84 and later the supported runtime target is `wasm32v1-none`; a full runtime's `build.rs` drives the target build and emits the runtime blob under `wbuild`.
- [Polkadot SDK parachain template](https://github.com/paritytech/polkadot-sdk-parachain-template) — supplies the Cumulus runtime/node layout, ParaId 1000 development convention, chain-spec generation, and Zombienet/Omni Node starting point. The local template basis and all executable artifacts still require exact pins and checksums in Build.
- [Arbitrum Stylus introduction](https://docs.arbitrum.io/stylus/gentle-introduction) — an Ethereum-L2 alternative can run Rust/Wasm contracts with EVM interoperability, but its sequencer, L1 settlement, data-availability, upgrade, and optimistic-finality policies would need separate selection.
- [Solana program model](https://solana.com/docs/core/programs) — native Rust programs are supported, while mutable state lives in program-owned accounts under explicit compute, account, upgrade-authority, and archival-RPC constraints.
- [Hyperledger Fabric introduction](https://hyperledger-fabric.readthedocs.io/en/latest/whatis.html) — a permissioned alternative provides governed identities, endorsement, and confidentiality-oriented architecture, but its official chaincode/application stack is not Rust-first and its consortium governance is materially different from public shared security.

## 4. Risks, Unknowns, Dependencies

- **Resolved policy — reference identity:** Use exact
  `(namespace, scope, value)` identity. Namespace is canonical lowercase ASCII
  from 1 through 64 bytes; scope is the caller-owned naming authority, project,
  or repository identity; value is the exact external identifier. Scope and
  value are bounded nonblank text, compared byte-for-byte, with no URL
  normalization, alias resolution, provider lookup, or inferred `latest`.
- **Resolved policy — Git object identity:** The Git adapter discovers the
  repository object format and uses algorithm-specific namespace
  `git.commit.sha1` or `git.commit.sha256`; scope is caller-owned repository
  identity and value is the full lowercase commit object ID.
- **Resolved policy — required origin:** Every current-generation unit is
  created with exactly one validated immutable origin. Missing, null, or
  malformed origin rejects. There is no `None`, unknown placeholder, legacy
  attribution state, originless constructor, or in-place correction; an
  erroneous origin requires a distinct Intent Unit.
- **Resolved policy — correction:** Use exact duplicate rejection and
  append-only evidence correction by revoking one exact active association and
  then recording the intended association. These are two independently
  canonical operations with a visible intermediate absence, no idempotency
  token, and no atomic replacement. Canonical revocation history remains even
  when SQLite removes the active projected row.
- **Resolved policy — storage/security:** Use schema v3 and local
  unencrypted storage of references only. CubiKan stores no provider content,
  credentials, raw prompts, transcripts, or verification result; it provides no
  internal authentication, authorization, automatic retention, redaction, or
  secure-erasure guarantee.
- **Risk — current-generation reset:** Adding origin to envelope v1 or either
  protocol v1 would silently redefine exact realized contracts. Use envelope v2,
  schema v3, and protocol v2; reject earlier generations unchanged. Because no
  deployment exists, do not build migration or synthetic attribution machinery.
  Stable Book chapters must still be superseded legally rather than rewritten.
- **Resolved protocol boundary:** Both JSON adapters use adapter-owned version
  2 and require origin. Stateless `cubikan` runs one in-memory lifecycle
  simulation and never reports canonical acceptance. `cubikan-local` alone
  submits lifecycle, relationship, and provenance mutations to the pinned local
  chain and reads an SQLite snapshot attested by complete event-stream comparison
  and typed replay against the configured pinned local archive RPC. That RPC is
  node-trusted rather than an independent finality proof. Sharing version number
  2 does not make their request/response schemas interchangeable. Plan locks two
  independently authored schema/fixture corpora with separate SHA-256 hashes,
  exact envelopes, operations, fields, encodings, result unions, and error-code
  inventories; implementation output cannot become its own snapshot oracle.
- **Risk — SQL injection versus hostile-file attacks:** Current production SQL
  binds caller values, and its dynamic query text concatenates only private
  constant fragments. Classic quote/semicolon injection is therefore not a
  confirmed user-reachable path. A caller-selected or locally replaced SQLite
  file remains a native-parser, schema-poisoning, symlink, confused-deputy, and
  resource-exhaustion boundary that parameter binding alone does not address.
- **Risk — defense in depth:** New work should enable and verify SQLite defensive
  mode; disable trusted schema, writable schema, double-quoted string literals,
  triggers, views, extension loading, ATTACH, and memory mapping;
  enable cell-size checking; use `SQLITE_OPEN_NOFOLLOW`; bound SQLite lengths and
  work; and retain exact schema, full integrity, foreign-key, envelope, and
  projection validation. Bundled SQLite does compile extension loading and URI
  support, so production connections need a deny authorizer, safe
  `load_extension_disable`, no URI open flag, canonical absolute direct-child
  paths that cannot begin with `file:`, and exact readback rather than a false
  compile-time-absence claim. Existing dynamic PRAGMA identifiers should
  become private enums or bound table-valued PRAGMA queries.
- **Resolved SQLite runtime surface:** Pin registry rusqlite 0.40.2 and one
  checked local vendor override whose only semantic diff adds
  `TransactionOperation::Commit` and maps SQLite authorizer argument `COMMIT`;
  reconstruct it from the exact registry archive and pin pristine-tree, patch,
  patched-tree, and normalized-diff hashes. Enable only `bundled`, `limits`,
  `modern_sqlite`, `hooks`, and `load_extension`; `hooks` implements a
  production deny authorizer and `load_extension` is present solely so safe
  `load_extension_disable()` can be called. Pin UTF-8, `temp_store=MEMORY`, mmap
  zero, `busy_timeout=5000` with no application retry, full
  `integrity_check`=`ok`, empty `foreign_key_check`, and the declared numeric
  limits before schema-dependent work. Read-only preflight checks page size,
  file bytes, and `page_count<=262144`; each writer sets/reads back
  `max_page_count=262144` because it is connection-local rather than persistent.
  Every connection sets and reads back
  `SQLITE_DBCONFIG_ENABLE_COMMENTS=false` before schema SQL. Production SQL is
  additionally a checked private static comment-free inventory, and no caller
  supplies SQL text. SQLite 3.53.2 authorizer traces name the schema table
  `sqlite_master`, emit `Reindex` for named index creation, and expose
  table-valued PRAGMAs as exact virtual-table `Read` plus base `Pragma` tuples;
  the independently authored finite oracle must use those actual callbacks,
  not `sqlite_schema` or Function aliases.
- **Resolved proof filesystem:** Support Linux only on a test-owned local
  filesystem whose default VFS honors advisory locks, same-directory atomic
  rename, file/directory fsync, and SQLite DELETE-journal locking. Reject
  non-Linux, network filesystems, DrvFS, FUSE, custom VFSes, symbolic/nonregular
  files, wrong ownership/mode, and unprovable semantics before SQLite/journal
  access. Hard links, parent replacement, same-user journal deletion, continuous
  post-open TOCTOU, and lying storage remain nonclaims. Sandboxing and external
  CPU/memory/time quotas remain required for hostile ingestion.
- **Risk — false provenance:** The only CubiKan relation is
  `RecordedAssociation`. It means that a caller recorded the link. Provider
  verification remains an ephemeral adapter observation; human attribution,
  causality, certification, and intent satisfaction remain outside the model.
- **Risk — revision confusion:** Whole-unit scope and exact revision `0` are
  distinct. Exact-revision association accepts only a revision from zero through
  the replay-validated current revision; association mutation never advances
  lifecycle revision.
- **Risk — private identifiers:** Even a reference can reveal private project or
  repository information. The local database inherits the caller's filesystem
  protections, and documentation must tell callers not to store secrets or
  sensitive locators without their own access and retention controls.
- **Dependency:** INT-0009 remains realized. The still-valid lifecycle,
  relationship, query, transaction, corruption, ingestion, and flush behavior
  from earlier generations must be carried forward and re-proved under the new
  generation; the terminal chapters were legally superseded in Plan and remain
  immutable during Build.
- **Dependency:** The Git demonstration needs a real temporary repository and
  installed Git, but Git remains outside `cubikan-core` and no hosting API or
  network access is required.
- **Resolved direction — blockchain authority:** “Blockchain Kanban” means the
  eventual chain is the canonical lifecycle state machine. SQLite is a
  deterministic, locally verified, rebuildable read model rather than a second
  writer or competing authority. Consensus does not secure the local database:
  a node must verify finalized chain events or a state checkpoint before using
  cached state. Mutation preflight and signing never derive from SQLite at all,
  including from an attested cache; they use explicit request values and
  canonical RPC state.
- **Resolved direction — public shared security:** Use a Polkadot SDK parachain.
  FRAME is the closest fit for the existing Rust domain validator, and a Rust
  Subxt projector can consume finalized events. A native Zombienet topology with
  multiple relay validators and collators on distinct ports supplies the first
  reversible two-runtime integration environment. Paseo is the next public
  test deployment; reserving an external identifier, obtaining coretime, or
  deploying publicly remains a separate human-gated action.
- **Resolved pin — SDK family:** Use `polkadot-stable2606-1` / node 1.24.1 at
  exact SDK commit `8ae9775dc43c0d8cdd0f6d87700596e14278b1e1` and its
  documented Rust 1.93 toolchain with the required `wasm32v1-none` target.
  Derive the runtime from the SDK's same-commit
  in-tree template rather than the external template whose current HEAD still
  declares the 2512.1.0 SDK. Pin Subxt 0.50.2 and Zombienet source commit
  `a7c434271f094320d17cf94f7a2f95fdef417379`; the exact relay/collator
  assets, chain-spec builder, Wasm target, Node/npm toolchain, and downloaded-
  asset checksums must be recorded before dependent work begins. Mixing release
  families is forbidden.
- **Resolved workspace boundary:** Put the runtime/pallet/node in a nested
  `chain/` workspace with its own `Cargo.toml`, `Cargo.lock`, and
  `rust-toolchain.toml`, excluded from the root workspace. Root tests stay fast;
  chain-native, Wasm, and local-network gates are explicit separate commands.
  Root may use only the exact Subxt/RPC/SCALE client graph. Dependency direction
  is `cubikan-backend -> cubikan-chain-client`: the client provides strict RPC
  and submission primitives while backend owns high-level sync/attestation,
  every raw SQLite write, and verified-capability construction. Rust has no
  friend-crate seam, so no public API accepts caller-made events, rows,
  checkpoints, or capabilities.
- **Resolved runtime bounds:** Use UUID bytes supplied off-chain; namespace
  1–64 bytes; scope/value, workflow ID, species, and phase text each 1–256 bytes
  with their stricter domain grammars; at most 32 phases, 128 workflow edges,
  32 completion phases, 256 lifecycle records per unit, 128 relationship edges
  per exact definition version, and 128 active provenance associations per unit.
  The lifecycle cap plus maximum JSON escaping must keep every complete envelope
  v2 within 2,097,152 bytes. All collection and traversal limits are compile-time
  FRAME constants with worst-case weights; queries may still return at most 100
  projected rows. Core vocabulary/workflow values share the common byte and
  collection bounds, while chain authorization/storage-capacity errors remain
  distinct. Every accepted event variant mechanically proves
  `MaxEncodedLen <= 1,048,576` so finalized canonical input cannot exceed the
  raw SQLite event column.
- **Resolved authorization:** Require a directly signed `AccountId32` on a
  bounded Root-managed technical submitter allowlist containing at most 16
  accounts. The pallet exposes one Root-only bounded replacement call, but the
  fixed local runtime supplies no reachable Root path and no sudo, proxy,
  utility, multisig, dispatch-as, or other origin-transforming wrapper. Any
  authorized signer may invoke lifecycle, relationship, or provenance commands.
  The same signer pays fees and authorizes submission but is not a unit owner,
  author, responsible person, or causal agent. Local genesis funds Charlie and
  Dave as dev submitters distinct from relay-validator and collator roles; the
  allowlist does not change during the sprint journey.
- **Resolved local chain:** Use local ParaId 1000, two relay validators, two
  archive-capable collators, four fixed distinct test-facing loopback RPC
  endpoints plus fixed unique primary and collator relay-side P2P/RPC/metrics
  ports. The pinned Zombienet generator adds external-bind arguments and the
  collator relay side, so a repository-owned SHA-pinned argv normalizer accepts
  only its exact generated grammar/hash, removes every external-bind flag,
  supplies the locked loopback listeners/bootnodes, and fails before launch on
  any unknown/missing/duplicate argument. `/proc` and socket inspection verify
  the four node processes separately from the orchestrator. Use one checked
  runtime Wasm and a checked local chain spec. Both collators use exact
  `--blocks-pruning=archive --state-pruning=archive`; probes cover historical
  block hash/header/body/events and `:code` across the journey range. No runtime
  upgrade or sudo facility exists in the fixed sprint runtime.
- **Resolved deployment anchor and coordinate:** Persist relay and parachain
  genesis hashes, ParaId, 32-byte CubiKan deployment ID, pallet storage version,
  event schema version, runtime specification version, and runtime code hash.
  Stable event identity is parachain genesis hash, deployment ID, finalized
  block number/hash, extrinsic index, canonical extrinsic hash, system event
  index, and checked global CubiKan sequence; signer is supplemental. The canonical
  JSON manifest is committed at `chain/artifacts/local-deployment-anchor-v1.json`,
  its bytes are SHA-256 pinned, and the harness re-verifies relay/parachain RPC
  provenance. SQLite event rows join block number to the immutable block row for
  the required block hash; no public coordinate may omit that joined hash.
- **Resolved finality:** Project finalized parachain blocks only; there is no
  provisional SQLite view. Verify parent continuity, anchor, runtime identity,
  event count/version/sequence, and replay before atomically committing block
  events with the checkpoint. A conflicting finalized hash is fatal and demands
  operator recovery rather than heuristic rollback.
- **Resolved counters:** Lifecycle revision and global CubiKan sequence are
  checked `u64` with no in-band sentinels. Revision starts at zero and remains
  within `0..=256` because complete lifecycle history is capped at 256 records;
  capacity rejects before domain validity. The first accepted CubiKan event uses
  sequence one. A checkpoint before the first domain event stores no last
  sequence; a later zero-event block has null per-block first/last values but
  retains the checkpoint's prior nonzero sequence. Global-sequence maximum
  returns typed exhaustion before mutation or accepted event and never wraps.
- **Resolved rejection accounting:** Atomic rejection means CubiKan pallet
  storage/global domain sequence do not change and no accepted domain event is
  emitted. Malformed codec or transaction-validity rejection before inclusion
  consumes no nonce or fee; an included typed dispatch failure still consumes
  its nonce, pays the runtime's ordinary fee, and leaves System failure evidence.
  This is not described as whole-chain rollback.
- **Resolved operation precedence:** After command version and direct
  signed/allowlisted origin, relationship-definition creation checks duplicate;
  edge create checks definition, source, target, source species, target species,
  self, duplicate, capacity, cycle; edge delete checks the same selected state
  through target species and then exact active edge, with no capacity check.
  Provenance record checks unit, revision/reference, duplicate, active capacity;
  revoke checks unit/reference then exact active target, with no capacity check.
  Global-sequence capacity is last for every otherwise-valid mutation.
- **Resolved projection attestation:** Public pages use a single-page SQLite
  read transaction whose anchor, checkpoint, complete stored event bytes and
  coordinates, and independently replayed typed state were compared against the
  complete finalized range returned by the configured pinned archive RPC. The
  capability dies with that read snapshot. This detects coherent database-only
  substitution relative to the node, not a compromised RPC, and is not a light
  client or independent consensus-finality proof.
- **Resolved fresh projection and task seam:** Creation initializes only the
  exact empty schema. The first backend-owned finalized sync transaction verifies
  the fixed manifest and atomically inserts anchor, block-zero zero-event row,
  and block-zero checkpoint with absent sequence before any capability exists.
  Query semantics remain independently testable before production attestation
  through a private `cfg(test)` backend harness; external callers receive no raw
  writer or capability constructor.
- **Resolved submission semantics:** Mutation validation and signing use only
  explicit request values plus canonical RPC state; SQLite, even attested, never
  supplies mutation preflight, deployment selection, revision inference, nonce,
  or signing data. A persistent owner-only per-signer lock protects one fixed
  versioned, SHA-256-checksummed prepared record of at most 256 bytes containing
  deployment, signer, nonce, expected hash, mortal era, signing finalized
  block/hash, and absolute birth/death heights. Publish before send via one fixed
  derived no-follow exclusive same-directory temp, full write/checksum, file
  fsync, atomic rename, and directory fsync; publish resolution the same way.
  Under the persistent lane lock, restart removes and directory-fsyncs only that
  owner-mode-0600 regular temp (complete or torn) and fails closed on every
  other type/owner/mode/size, preventing one crash from stranding the lane or
  accumulating orphan temps. Wait at most 120 seconds
  for finalized status. Success requires exactly one matching accepted event
  inside the exact finalized extrinsic. An unresolved lane sends nothing until
  exact-hash finalization is found or finalized head exceeds death and a complete
  birth-through-death scan proves it absent, yielding `expired_not_included`;
  expiry alone is insufficient and there is no retry. Absence is clean first-use
  state, while same-user deletion is undetectable/out-of-bound. The journal is
  operational and noncanonical.
- **Resolved fees/upgrades/data:** Retain normal balances and transaction-payment
  behavior with zero tip and economically meaningless dev funds. Fix runtime
  code from genesis through the journey. Store bounded lifecycle, relationship,
  and structured reference data only; public synthetic identifiers are the only
  permitted fixtures.
- **Risk — relationship runtime cost:** Carrying INT-0012 removes a split
  authority but adds bounded cycle traversal, capacity rejection, weights,
  event replay, and conformance work. The exact 128-edge-per-definition bound is
  therefore a current-generation product limit, not an unbounded graph claim.
- **Risk — build and CI cost:** A cold Polkadot SDK native/Wasm build and local
  four-node journey may exceed the existing 15-minute root job. Preserve the
  root CI workflow byte-for-byte. A separate `workflow_dispatch` chain job gets
  one explicit dependency/toolchain fetch phase and then runs locked/offline,
  with caches keyed by exact toolchain/SDK/lock/artifact hashes. The candidate
  budgets are cold <=90 minutes, warm <=30 minutes, peak workspace-plus-cache
  disk <=60 GiB, and Zombienet <=30 minutes; exceeding any budget fails the gate.

## 5. Recommended Approach

Implement one ordered vertical rather than parallel SQL and chain authorities:

1. Pin the complete stable2606-1 family, compatible template/tooling, Subxt,
   Rust/Wasm toolchain, checksums, and local topology in an isolated `chain/`
   workspace. Keep the existing workspace and lockfile independent.
2. Add provider-neutral reference/origin values to `cubikan-core`, requiring one
   origin at every current-generation construction boundary and sharing exact
   vocabulary/workflow bounds. Add bounded SCALE chain equivalents and
   table-driven conformance fixtures without making core a FRAME dependency;
   prove every accepted event fits the one-MiB projected-event ceiling.
3. Implement `pallet-cubikan` storage and transactional calls for create,
   revision-guarded transition/completion, bounded relationship definitions and
   edges, and provenance record/revoke. Preserve stale-first and INT-0012
   precedence within fixed bounds. Emit exactly one accepted event with a
   checked global sequence per successful call.
4. Integrate the pallet into the pinned parachain runtime and local chain spec.
   Fix ParaId 1000, deployment ID, event/storage versions, runtime code hash,
   technical submitter allowlist, balances/fees, and two archive collators.
   Generate and commit metadata from the exact tested runtime; do not fetch
   metadata during compilation.
5. Replace SQLite canonical writes with an internal finalized-event projector.
   `cubikan-backend` depends on the strict chain client and alone owns sync,
   attestation, raw writes, and snapshot minting. Schema v3/envelope v2 store
   complete replayed state plus joined block-hash coordinates. Fresh creation is
   schema-only; the first block-zero transaction inserts anchor/checkpoint. A
   block transaction verifies anchor, continuity, runtime,
   sequence, event shape, and domain replay before committing rows and checkpoint
   together. Public reads obtain a one-page read snapshot only after comparing
   the complete covered event stream and an independent typed replay against the
   configured node-trusted archive RPC. Remove successful old-generation
   migration and reject v1/v2 stores.
6. Preserve bounded lifecycle, direct relationship, projection-v1, and forward/
   reverse provenance queries over the verified checkpoint. Fully validate
   selected candidates/lookahead. Harden connection configuration, open flags,
   paths, limits, and schema handling against hostile file and resource inputs;
   keep every caller value bound. Limit proof claims to the checked Linux local
   filesystem and exact UTF-8/authorizer/temp-store/busy/page/integrity settings.
7. Implement both adapter-owned protocol-v2 boundaries and a loopback-only
   development runner from independent hashed schema/fixture contracts.
   Stateless `cubikan` remains simulation-only. Mutation preflight and signing
   never use SQLite. A hardened per-signer cross-process journal atomically and
   durably publishes the exact prepared nonce/hash/era/birth/death record before
   send. Submissions wait for finalized success and an event inside the exact
   extrinsic, distinguish every rejected/indeterminate outcome, require a full
   finalized birth..death exact-hash scan before `expired_not_included`, block an
   unresolved lane, and never retry. Reads state checkpoint/lag. V1 fails before
   RPC, SQLite, or journal access.
8. Add the provider-neutral Git adapter demonstration using a real temporary
   repository and full repository-qualified object identity. Git remains outside
   core and the runtime.
9. Prove the complete vertical under native Zombienet with two relay validators
   and two collators, four fixed test-facing RPC endpoints, fixed primary and
   collator relay-side ports normalized to loopback by the checked launcher, archive
   block/state flags, and a 30-minute cap. Compare both endpoints, stop one
   collator, continue the
   journey through the survivor, restart and synchronize the stopped collator,
   probe its archive range, wipe SQLite, rebuild through each archive source,
   and compare exact semantic state and checkpoint. Keep port allocation,
   process cleanup, logs, timeouts, and fixture data deterministic.
10. Document the authority map, stable coordinates, finality and delivery,
    SQLite threat model, disclosure, correction, fees, dev authorization,
    recovery, and public-deployment exclusions. Keep terminal superseded intents
    and root CI byte-identical. Add a portable repository-local Book validator
    and a separate manual chain/resource workflow with exact caches, one fetch
    phase, offline execution, and the locked time/disk budgets.

This is the smallest architecture-complete realization of INT-0008 and the
local-development portion of INT-0014. An origin-only slice is insufficient.
Leaving current relationship writes authoritative in SQLite would recreate the
dual-authority problem, so the current generation carries bounded INT-0012
semantics onto the chain and re-projects them.

The local network proves deterministic runtime acceptance, finalized-event
consumption, failover, and rebuild using the selected public/shared-security
technology. It does not itself provide Polkadot shared economic security.
Paseo is the next public target, but any public ParaId, registration, coretime,
funding, account/key, runtime upload, disclosure, governance, release, or
deployment action requires a separate intent and explicit human approval.

## Budget Override

Sprint 11 intentionally crosses five independently authoritative boundaries:
the existing Rust domain and SQLite implementation, provenance identity, SQLite
hostile-file defense, the pinned Polkadot SDK/FRAME/Subxt runtime stack, and a
four-process Zombienet proof. The 21 inspected repository files and 16 primary
external sources are retained because removing any source family would leave a
locked dependency, finality, security, provenance, or alternative-chain choice
unsupported. Research remains bounded to this single local vertical and does
not survey public deployment operations.

## Artifacts

No separate research artifacts were created. This report records the approved
current-generation contract, selected Polkadot SDK family, local network and
finality model, SQLite threat model, and the explicit boundary between the
automated local proof and a later human-gated public deployment.
