Finalized - DO NOT EDIT

# Sprint 11 Build Plan

## Intents

- [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md) — state: planned; acceptance criteria covered: mandatory immutable origin, exact external-reference identity, whole-unit/revision-scoped recorded associations, append-only revocation, bidirectional verified queries, Git demonstration, and evidence/privacy nonclaims.
- [INT-0014](../../../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md) — state: planned; acceptance criteria covered: pinned Polkadot SDK runtime, bounded canonical lifecycle/relationship/provenance calls, finalized-only Subxt projection, exact SQLite v3/envelope v2, adapter-owned protocol v2, local 2+2 Zombienet proof, defensive storage, and explicit public-deployment exclusions.
- [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) — state: superseded by planned INT-0014; acceptance criteria carried forward: immutable versioned definitions, exact directed edge identity, endpoint/species/self/cycle/duplicate/delete precedence, bounded direct queries, and ephemeral projection-v1 semantics.
- [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) — state: realized and retained; acceptance criteria carried forward: caller-observed revisions, stale-first domain rejection, one-success concurrency, and atomic successor state.
- [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) — state: superseded by planned INT-0014; the exact Sprint 8 implementation remains historical evidence, while its SQLite-canonical mutation, migration, schema-v1, and local protocol-v1 authority are replaced rather than extended.
- [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) — state: superseded by planned INT-0014; its Sprint 10 appendix correction remains historical evidence, while T-1116 owns the new current-state reconciliation.

## Sprint Goal

Build CubiKan's first local blockchain-canonical current generation. One pinned
Polkadot SDK parachain runtime accepts required-origin lifecycle, relationship,
and provenance mutations. A finalized-event projector derives an exact hardened
SQLite v3 read model. Protocol v2 submits and reports finalized outcomes. A
native Zombienet journey proves one pinned relay runtime across the relay
validators and collator relay-side services plus one distinct pinned CubiKan
parachain runtime across both collators, collator failover, and deletion/rebuild
equality without touching a public network.

This is a local development realization of the selected public/shared-security
architecture, not a public deployment. The sprint must not create or fund a
public account, reserve a ParaId, obtain coretime, publish a runtime, or claim
live Polkadot shared security.

## Locked Design Decisions

- **One domain authority:** accepted `pallet-cubikan` events are canonical.
  SQLite has no independent lifecycle, relationship, or provenance write API.
- **Complete current generation:** required-origin lifecycle, bounded INT-0012
  relationships, and INT-0008 provenance all move together; no authoritative
  SQLite island remains.
- **Greenfield versions:** schema v3, envelope v2, protocol v2, pallet storage
  v1, and event schema v1 are distinct identities. Schema v1/v2, envelope v1,
  and protocol v1 reject unchanged; no migration or synthetic origin exists.
- **Adapter-owned protocol v2:** `cubikan` v2 remains a one-shot,
  chain-neutral lifecycle validator/simulator and can never report canonical
  acceptance or retain runtime state. `cubikan-local` v2 is the only JSON
  boundary that submits to the pinned local chain and reads its verified
  projection. The two adapters share hardening outcomes, not one wire schema.
- **Pinned release family:** Polkadot SDK `polkadot-stable2606-1`, exact commit
  `8ae9775dc43c0d8cdd0f6d87700596e14278b1e1`, Rust 1.93.0 with exact
  `rustfmt`/`clippy` components and `wasm32v1-none` target identity, Subxt 0.50.2,
  same-commit SDK template basis, and Zombienet source commit
  `a7c434271f094320d17cf94f7a2f95fdef417379`. T-1101 records every
  compatible asset version/checksum before dependent implementation.
  Root `rust-toolchain.toml` pins channel `1.93.0` with `rustfmt` and `clippy`;
  chain `rust-toolchain.toml` pins the same channel/components plus
  `wasm32v1-none`. No gate or workflow may use the floating `stable` alias.
- **Workspace isolation:** `chain/` is a nested Rust-2021/resolver-2 workspace
  with its own lockfile/toolchain and is excluded from the root Rust-2024
  workspace. The final root direct-dependency delta is closed: workspace pins
  `subxt=0.50.2` (`jsonrpsee`,`native` only), `subxt-signer=0.50.2`
  (`sr25519`,`subxt` only), `tokio=1.47.1` (`rt`,`macros`,`time`,`sync` only),
  `futures-util=0.3.31` (`std` only), `url=2.5.8` (`std` only),
  `parity-scale-codec=3.7.5` (`derive`,`std` only), `scale-info=2.11.6`
  (`derive`,`std` only), `rustix=1.1.4` (`fs`,`process`,`std` only), and
  `sha2=0.10.9` (`std` only), plus the existing exact rusqlite feature expansion.
  Every new or changed declaration uses exact `=VERSION`,
  `default-features=false`, and exactly its listed requested feature array. The
  T-1101 pin artifact records and SHA-256-hashes the complete resolved
  `cargo tree -e features --locked` closure; verification rejects closure drift
  and any effective feature on those nine root packages beyond the closure
  implied by their listed arrays. Necessary upstream default/implied transitive
  features are recorded facts, not falsely treated as absent.
  The root lockfile must also keep every package entry named `subxt` or
  `subxt-*` exactly `0.50.2`, including unselected optional entries such as
  `subxt-lightclient` as well as signer, macro, codegen, metadata, RPCs,
  accountid32, and fetchmetadata utilities. Any mixed 0.50.2/0.50.3 lock
  resolution is rejected before compilation.
  One reviewed local source override is required for rusqlite: T-1101 verifies
  the registry `rusqlite 0.40.2` crate-archive checksum
  `23f2a97da3e3873c73cb2a2e71b35c40ff95e0b1eefa8d72d8499a6928c3b5b3`,
  applies repository-owned
  `patches/rusqlite-0.40.2-commit-authorizer.patch`, and commits the result at
  `vendor/rusqlite-0.40.2-cubikan/`. The patch does only two semantic things:
  adds `TransactionOperation::Commit`, and maps exact SQLite authorizer argument
  `"COMMIT"` to it; `Unknown` remains fail-closed. `chain/pins.toml` records the
  pristine extracted-tree SHA-256, patch SHA-256, patched-tree SHA-256, and
  normalized upstream-to-vendor diff SHA-256. Each tree digest is SHA-256 over
  the UTF-8 manifest formed by byte-sorting every regular relative POSIX path
  and emitting lowercase file SHA-256, two ASCII spaces, that path, and LF;
  symlinks, absolute/parent paths, duplicate paths, and nonregular entries reject.
  The verifier reconstructs the
  pristine tree from the checked registry archive, applies the exact patch, and
  requires byte equality with the checked vendor tree before any build.
  Root Cargo uses an exact local `[patch.crates-io]` path override; no public fork,
  floating source, unrelated vendor edit, or unsafe wrapper is permitted.
  `cubikan-chain-client` may depend directly on that list; `cubikan-backend`
  may add `cubikan-chain-client`, rustix, sha2, and rusqlite features;
  `cubikan-local` may add `cubikan-chain-client`, tokio, and url; `cubikan-git`
  uses only std process/fs plus existing core/serde dependencies. No other new
  root direct dependency or feature is permitted without a new reviewed plan,
  and Polkadot SDK, FRAME, Cumulus, node, and runtime packages/sources remain
  chain-only. Root and chain gates are explicit and independently reproducible.
  Root dependency direction is `cubikan-backend -> cubikan-chain-client`: the
  client exposes strict finalized RPC/submission primitives, while backend owns
  high-level sync and attestation, every raw SQLite write, and all
  `VerifiedReadSnapshot` construction. No public seam accepts caller-made
  blocks/events/rows/checkpoints/capabilities and chain-client never depends on
  backend.
- **Loopback-only offline execution:** after the one exact fetch phase, every
  compiler, build script, test, node, and verifier runs through the pinned
  repository-owned `chain/tools/loopback-netns.sh`. On the declared Linux
  runner it capability-checks and invokes `unshare --user --map-root-user --net`,
  raises only `lo`, and requires an empty non-loopback interface/route inventory.
  It records a denied external-connect probe and a successful loopback probe,
  then executes the requested argv without a shell. Failure to create that
  namespace aborts the gate; Cargo `--offline` alone is never treated as an
  egress control. Exact util-linux/iproute2 identities and wrapper SHA-256 are
  pinned with the other T-1101 tools.
- **Deterministic bounds:** namespace is 1–64 bytes with grammar
  `[a-z][a-z0-9._-]{0,63}`; exact scope/value and domain text are nonblank,
  NUL-free UTF-8 from 1–256 bytes; there are at most 32 phases, 128 workflow
  edges, 32 completion phases, 256 lifecycle records per unit, 128 edges per exact relationship definition
  version, 128 active associations per unit, 16 authorized technical submitters,
  and projected query limits 1–100. Runtime collections use bounded SCALE values
  and generated worst-case weights. Core vocabulary/workflow types share the
  common byte and collection bounds; chain-only authorization/storage-capacity
  errors stay distinct. Every accepted event variant has a mechanical
  `MaxEncodedLen <= 1,048,576` proof and a maximal encoded fixture.
- **No on-chain randomness or wall clock:** UUIDs are caller-generated and
  submitted as 16 bytes. Lifecycle remains revision/sequence based.
- **Exact counters without sentinels:** lifecycle revision and lifecycle-record
  sequence are unsigned `u64`; a new unit has revision `0`, its first lifecycle
  record has sequence/revision `1`, and both remain within `0..=256` under the
  complete-history capacity. Global CubiKan
  event sequence is nonzero unsigned `u64`, starts with valid value `1`, may
  reach `u64::MAX`, and is represented as `Option<u64>` wherever no accepted
  event exists yet; numeric zero is never a sentinel. Rejection precedence is
  command schema version, directly signed/allowlisted origin, target selection
  and stale revision where applicable, lifecycle-history capacity where
  applicable, remaining command-domain validity, then global-sequence capacity
  immediately before mutation/event. Exhaustion is typed and produces no
  CubiKan mutation or accepted event; history capacity precedes remaining
  lifecycle validity and global-sequence capacity.
- **Technical authorization, not ownership:** a Root-managed bounded allowlist
  authorizes signed `AccountId32` calls. Either local development submitter may
  operate any unit. Signers pay fees but are not owners, authors, humans, causal
  agents, or provenance subjects. The pallet exposes one max-16 root-only
  allowlist replacement call; signed callers cannot invoke it. The fixed local
  runtime includes no sudo, proxy, utility, multisig, governance, or other
  origin-transforming route to Root, so the journey's genesis allowlist cannot
  change and every domain call is directly signed and fee-paid.
- **Local fixed runtime:** ParaId 1000, two relay validators, two archive-capable
  collators, four fixed distinct test-facing loopback RPC endpoints, fixed
  unique primary P2P/metrics ports, plus fixed unique relay-side RPC/P2P/metrics
  ports inside each collator process. A repository-owned argv-normalizing
  launcher, whose bytes/SHA-256 are pinned, accepts only the exact generated
  Zombienet argv grammar/hash, removes all RPC/WS/Prometheus external-bind flags,
  supplies the locked loopback listen addresses/ports and loopback bootnodes,
  and fails before launch on any missing/duplicate/unknown flag. There is one
  local deployment ID, one relay chain spec/runtime identity, and one distinct
  parachain chain spec/CubiKan runtime Wasm/code hash. The two collators execute
  byte-identical CubiKan Wasm; the relay validators and both collator relay-side
  services use the byte-identical pinned relay runtime. Relay and parachain Wasm
  bytes are never compared as though they were one artifact.
  Both collators use exact
  `--blocks-pruning=archive --state-pruning=archive`. No runtime upgrade occurs.
- **Noncircular deployment anchor:** the CubiKan deployment-anchor and
  `pallet-cubikan` genesis fields contain only non-self-referential ParaId,
  deployment ID, pallet storage version, and event schema version; standard
  System storage supplies `:code`, and the pinned chain specification contains
  the exact additional System/parachain/session/collator/Aura/balances genesis
  state required to run the local network. A checked post-genesis manifest composes
  relay genesis hash, parachain block-0 hash obtained through the strict RPC,
  those on-chain fields, and the hash of `:code`. SQLite copies this verified
  composite. The canonical JSON bytes live at
  `chain/artifacts/local-deployment-anchor-v1.json`, their SHA-256 is pinned by
  `chain/pins.toml`, the harness re-verifies both RPC provenances, and root
  clients accept only that fixed pin-verified artifact. No chain spec or runtime
  storage predicts its own block hash.
- **Finalized-only, node-trusted projection:** no provisional SQLite view.
  Stable identity is deployment/chain anchor plus finalized block number/hash,
  extrinsic index/hash, system event index, and checked global CubiKan event sequence.
  Runtime spec, code hash, metadata, and signer are verified supplemental data.
  A read capability is issued only after replaying the full
  finalized event stream through the requested checkpoint from the configured
  pinned local archive RPC and comparing all stored blocks, events, derived
  state, envelopes, and checkpoint. This trusts that configured node's finalized
  RPC assertions; it is not an independent GRANDPA proof or shared-security
  verifier. SQLite event rows store block number and obtain block hash only by
  restrictive join to the immutable projected-block row; every public coordinate
  and attestation validates the joined hash. A fresh v3 file is schema-only. The
  first backend-owned sync transaction atomically inserts the verified anchor,
  block-zero zero-event row, and block-zero checkpoint before reads can exist.
  Mutation preflight/signing uses only explicit request data and canonical RPC
  state—never SQLite, even when attested.
- **Synchronous finalized submission:** default wait 120 seconds; cooperating
  `cubikan-local` processes coordinate each signer through the versioned,
  owner-only cross-process journal and exclusive lock, use a mortal 64-block
  extrinsic and zero tip, require exactly one matching accepted event for
  success, distinguish every known rejection from indeterminate delivery, and
  never retry automatically. External software using the same dev signer is not
  coordinated; detected nonce disagreement or an unresolved lane fails closed
  pending explicit reconciliation, or finalized-head-past-death plus a complete
  finalized birth-through-death exact-extrinsic-hash absence scan. Era expiry by
  itself never clears the lane.
  Nonce is decoded from `System::Account` storage at the exact chosen finalized
  signing block hash and bound to that same block's mortality checkpoint; the
  client never uses `system_accountNextIndex`, best-head state, or a transaction-
  pool-adjusted suggestion. Best/finalized divergence or an external pending
  nonce therefore cannot silently change the signed nonce.
- **Deterministic signer lane:** journal/lock names are derived as direct
  children of the canonical projection directory from deployment ID and signer
  `AccountId32`; the CLI cannot choose an alternate journal path. Let `P` be the
  canonical projection-directory path encoded as raw Unix bytes. The lane digest
  is SHA-256 over `b"CubiKan signer lane v1\0" || u32_be(len(P)) || P ||
  deployment_id[32] || signer[32]`; its lowercase 64-hex encoding `H` yields
  exactly `cubikan-submission-H.lock`, `cubikan-submission-H.journal`, and
  `cubikan-submission-H.tmp`. Length overflow, NUL, noncanonical path bytes, or
  any derived basename mismatch rejects. Independent asymmetric vectors lock
  every delimiter/length/order byte and traversal-like inputs.
  Coordination
  covers cooperating CubiKan processes using that projection. A different
  projection or external signer user is outside the boundary and any resulting
  nonce disagreement fails closed. The persistent mode-`0600` no-follow lock
  inode is never replaced or unlinked. `submission-journal-v1` is exactly 256
  bytes with no trailing bytes. Bytes `0..8` are ASCII `CUBKJNL1`; `8..10` are
  big-endian format version `1`; byte `10` is the state tag (`0` prepared, `1`
  finalized accepted, `2` finalized dispatch rejected, `3` finalized invariant
  failed, `4` expired not included); byte `11` is zero
  flags; `12..14` are the
  big-endian total length `256`; and `14..16` are zero reserved bytes. The
  ordered body is deployment ID `16..48`, signer `AccountId32` `48..80`,
  big-endian `u64` nonce `80..88`, extrinsic hash `88..120`, big-endian signing
  finalized block number `120..128`, signing block hash `128..160`, inclusive
  big-endian birth/death heights `160..168`/`168..176`, big-endian resolution
  block number `176..184`, and resolution block hash `184..216`. Byte `216` is
  the original `MutationOperation` tag (`0` create unit, `1` transition unit,
  `2` complete unit, `3` create definition, `4` create relationship, `5` delete
  relationship, `6` record association, `7` revoke association); bytes
  `217..224` are zero reserved bytes. Prepared records require the resolution coordinate
  to be all zero; finalized states use the exact finalized inclusion coordinate;
  finalized-invariant failure uses the known finalized inclusion coordinate;
  expiry uses the first observed finalized head strictly after death. Bytes
  `224..256` are SHA-256 over the domain-separated prefix
  `b"CubiKan submission-journal-v1\0" || record[0..224]`. Any unknown state or
  operation tag, nonzero reserved/flags byte, inconsistent state/coordinate, checksum mismatch,
  short/long record, or trailing byte rejects. The only persisted transitions
  are absent→prepared; prepared→one of finalized accepted, finalized dispatch
  rejected, finalized invariant failed, or expired not
  included; and a resolved state→absent through fsynced removal. Every restart,
  scan, and reconciliation response uses the persisted original operation tag,
  never the operation on a later incoming request. Resolved→resolved,
  resolved→prepared, and prepared→absent without proven resolution reject.
  Publication uses an owner-only
  same-directory `O_EXCL|O_NOFOLLOW` temporary direct child, complete write,
  checksum, file `fsync`, atomic rename, and parent-directory `fsync` before
  send; resolution uses the same sequence. Absence is clean first-use state.
  Same-user deletion of an unresolved record is undetectable and out of scope.
  A later process clears an unresolved lane only after exact-hash finalization or
  finalized head past death plus a complete birth-through-death finalized scan
  proving absence (`expired_not_included`); era expiry alone proves nothing about
  earlier inclusion. For terminal states 1–3, recovery re-fetches the exact
  stored inclusion block, locates exactly one matching extrinsic hash/index,
  decodes dispatch outcome, signer/call identity and accepted events, and
  reconstructs the identical persisted operation/outcome plus chain-derived
  effect/coordinate/error. For state 4, recovery validates the stored first
  post-death finalized head, re-scans every finalized block in inclusive
  birth..death, requires the exact hash absent throughout, and reconstructs
  `expired_not_included`. Projection is freshly attested and may
  legitimately advance, so complete stdout byte equality is not claimed. Missing,
  unavailable, duplicate, or mismatched recovery evidence keeps the terminal
  record and fails closed; the record is removed only after the reconstructed
  response's durability boundary completes. A crash after stdout but before
  removal may duplicate that semantic response, never a submission.
- **Strict local RPC:** before and after URL parsing, the raw endpoint must match
  lowercase `ws://` followed by either four canonical decimal IPv4 octets (no
  leading-zero, short, integer, octal, or hexadecimal spelling) within
  `127.0.0.0/8`, or exact bracketed `[::1]`; then an explicit canonical decimal
  port in `1..=65535` except the normalized-away default port `80`; then the
  literal path `/`. Parsed host/port/path must
  round-trip to that same raw canonical spelling. An omitted/empty raw path,
  hostnames, scheme/host case
  variants, percent encoding, userinfo, query, fragment, redirects, alternate
  schemes, alternate IP spellings, and public addresses reject before any
  connection, signing, database capability, or log.
- **Hardened projection:** bound all SQL values; use constant/private-enum SQL
  structure; separate defensive projector writer from query-only readers; reject
  symlink/nonregular paths; disable dangerous SQLite capabilities; enforce exact
  schema/integrity/replay/checkpoint validation and resource limits. The proof
  supports Linux only on a test-owned local filesystem/VFS that honors advisory
  locks, same-directory atomic rename, file/directory fsync, and DELETE-journal
  locking. Before any file access, one locked classifier contract canonicalizes the
  directory, parses `/proc/self/mountinfo` using the longest containing mount
  point, and requires filesystem type `ext2`, `ext3`, `ext4`, `xfs`, or `btrfs`
  plus the matching `statfs` magic (`0xEF53`, `0x58465342`, or `0x9123683E`).
  Volatile `tmpfs` is intentionally not an accepted durable store. SQLite is
  opened only through the registered
  built-in `unix` VFS selected by name; callers cannot select a VFS. Missing,
  malformed, mismatched, overlay, network, 9p/DrvFS, FUSE, or unknown mount/VFS
  identity rejects before any projection/journal create or open. Backend storage
  and chain-client journal code each implement that same contract independently
  to preserve the one-way crate dependency; both must pass the independently
  authored `tests/fixtures/filesystem-boundary-v1.json` corpus and the same real
  supported-local-directory suite. Neither implementation may broaden the
  accepted set or infer safety from the other.
- **Stable local path boundary:** creation/open receives a stable, test-owned,
  owner-only projection directory and one direct-child filename. The directory,
  target, and `-journal`/`-wal`/`-shm` siblings reject when symlinked or
  nonregular. Journal mode is `DELETE` and synchronous mode is `EXTRA`; an
  unexpected pre-open journal/WAL state fails for rebuild without allowing
  SQLite recovery before trust. Hard links, hostile parent replacement, custom
  VFS behavior, same-user unresolved-journal deletion, and continuous protection
  after a successful open are explicit nonclaims requiring sandboxing/least
  privilege. Although bundled SQLite's `SQLITE_USE_URI` is accepted, the path
  passed to SQLite is the canonical absolute stable-directory path joined to
  one validated direct-child basename, so it cannot begin with `file:`; no
  `SQLITE_OPEN_URI` flag or `ATTACH` is permitted. A `file:`-shaped basename is
  tested as a literal child beneath that canonical directory.
- **Pinned SQLite surface:** root storage uses the exact checked local
  `rusqlite 0.40.2` source override above with only the
  planned `bundled`, `limits`, `modern_sqlite`, `hooks`, and `load_extension`
  features (`hooks` only for the production deny authorizer; `load_extension`
  only for safe `load_extension_disable()`),
  `libsqlite3-sys 0.38.2`, bundled SQLite 3.53.2, and a checked per-target sorted
  compile-option manifest.
  Native `ENABLE_LOAD_EXTENSION` and `USE_URI` compile options are accepted and
  recorded, but every connection disables extension loading, rejects SQL
  `load_extension`, omits URI open flags, and sets/reads
  `SQLITE_DBCONFIG_ENABLE_COMMENTS=false` before schema SQL. All production SQL
  is private, compile-time inventory text with no comments, and no caller
  supplies SQL. A version,
  feature, compile-option, or runtime-library mismatch rejects before schema SQL.
- **Public synthetic data only:** no source bodies, prompts, transcripts,
  credentials, seed phrases, private repository locators, provider secrets, or
  production identifiers enter runtime, events, SQLite, JSON, logs, or fixtures.

## Exact schema-v3 and envelope-v2 inventory

Schema v3 uses UTF-8 `encoding`, `user_version=3`, and exactly eight rowid `STRICT` application
tables. Actual declared types are only SQLite `INTEGER`, `TEXT`, and `BLOB`;
unsigned `u64` values are exact eight-byte big-endian BLOBs, never custom STRICT
type names. Text identities use SQLite's effective built-in `BINARY` default and
every ordering/equality query spells `COLLATE BINARY`; exact DDL validation tests
that chosen form rather than pretending the column clauses are explicit.
Columns occur in the listed order and every abbreviated predicate below expands
literally in checked DDL:

- `namespace(x)` means `typeof(x)='text' AND length(CAST(x AS BLOB)) BETWEEN 1
  AND 64 AND instr(x,char(0))=0 AND x GLOB '[a-z]*' AND x NOT GLOB
  '*[^a-z0-9._-]*'`.
- `bounded_text(x)` means `typeof(x)='text' AND length(CAST(x AS BLOB)) BETWEEN
  1 AND 256 AND instr(x,char(0))=0`; checked Rust replay additionally rejects
  Unicode-blank text.
- `blob8(x)` means `typeof(x)='blob' AND length(x)=8`; `nonzero_blob8(x)` adds
  `x<>X'0000000000000000'`; `hash32(x)` means `typeof(x)='blob' AND length(x)=32`.

The exact tables are:

- `projection_anchor(singleton INTEGER NOT NULL PRIMARY KEY CHECK(typeof(singleton)='integer'
  AND singleton=1), namespace TEXT NOT NULL CHECK(typeof(namespace)='text' AND
  namespace='polkadot-sdk-parachain'), relay_genesis_hash BLOB NOT NULL
  CHECK(hash32(relay_genesis_hash)), parachain_genesis_hash BLOB NOT NULL
  CHECK(hash32(parachain_genesis_hash)), para_id INTEGER NOT NULL
  CHECK(typeof(para_id)='integer' AND para_id=1000), deployment_id BLOB NOT NULL
  CHECK(hash32(deployment_id)), pallet_storage_version INTEGER NOT NULL
  CHECK(typeof(pallet_storage_version)='integer' AND pallet_storage_version=1),
  event_schema_version INTEGER NOT NULL CHECK(typeof(event_schema_version)='integer'
  AND event_schema_version=1), initial_runtime_spec_version INTEGER NOT NULL
  CHECK(typeof(initial_runtime_spec_version)='integer' AND
  initial_runtime_spec_version BETWEEN 0 AND 4294967295),
  initial_runtime_code_hash BLOB NOT NULL CHECK(hash32(initial_runtime_code_hash)))`.
  This row copies the checked post-genesis manifest: hashes originate from RPC,
  non-self-referential fields from decoded state, and code hash from `:code`.
- `projected_blocks(anchor_singleton INTEGER NOT NULL CHECK(typeof(anchor_singleton)='integer'
  AND anchor_singleton=1), block_number BLOB NOT NULL PRIMARY KEY
  CHECK(blob8(block_number)), block_hash BLOB NOT NULL CHECK(hash32(block_hash)),
  parent_hash BLOB NOT NULL CHECK(hash32(parent_hash)), runtime_spec_version
  INTEGER NOT NULL CHECK(typeof(runtime_spec_version)='integer' AND
  runtime_spec_version BETWEEN 0 AND 4294967295), runtime_code_hash BLOB NOT NULL
  CHECK(hash32(runtime_code_hash)), cubikan_event_count INTEGER NOT NULL
  CHECK(typeof(cubikan_event_count)='integer' AND cubikan_event_count BETWEEN 0
  AND 4294967295), first_global_sequence BLOB, last_global_sequence BLOB,
  FOREIGN KEY(anchor_singleton) REFERENCES projection_anchor(singleton) ON
  UPDATE RESTRICT ON DELETE RESTRICT, CHECK((cubikan_event_count=0 AND
  first_global_sequence IS NULL AND last_global_sequence IS NULL) OR
  (cubikan_event_count>0 AND nonzero_blob8(first_global_sequence) AND
  nonzero_blob8(last_global_sequence) AND first_global_sequence<=last_global_sequence)))`.
  The count covers CubiKan domain events only; Rust verification also proves that
  the inclusive first/last range contains exactly that many consecutive events.
- `projected_events(block_number BLOB NOT NULL CHECK(blob8(block_number)),
  extrinsic_index INTEGER NOT NULL CHECK(typeof(extrinsic_index)='integer' AND
  extrinsic_index BETWEEN 0 AND 4294967295), system_event_index INTEGER NOT NULL
  CHECK(typeof(system_event_index)='integer' AND system_event_index BETWEEN 0
  AND 4294967295), global_sequence BLOB NOT NULL CHECK(nonzero_blob8(global_sequence)),
  deployment_id BLOB NOT NULL CHECK(hash32(deployment_id)), event_schema_version
  INTEGER NOT NULL CHECK(typeof(event_schema_version)='integer' AND
  event_schema_version=1), event_kind TEXT NOT NULL CHECK(event_kind IN
  ('unit_created','unit_transitioned','unit_completed',
  'relationship_definition_created','relationship_created','relationship_deleted',
  'association_recorded','association_revoked')), scale_payload BLOB NOT NULL
  CHECK(typeof(scale_payload)='blob' AND length(scale_payload) BETWEEN 1 AND
  1048576), signer BLOB NOT NULL CHECK(hash32(signer)), extrinsic_hash BLOB NOT
  NULL CHECK(hash32(extrinsic_hash)), PRIMARY KEY(block_number,extrinsic_index,
  system_event_index), FOREIGN KEY(block_number) REFERENCES
  projected_blocks(block_number) ON UPDATE RESTRICT ON DELETE RESTRICT)`.
- `projection_checkpoint(singleton INTEGER NOT NULL PRIMARY KEY
  CHECK(typeof(singleton)='integer' AND singleton=1), block_number BLOB NOT NULL
  CHECK(blob8(block_number)), block_hash BLOB NOT NULL CHECK(hash32(block_hash)),
  last_global_sequence BLOB CHECK(last_global_sequence IS NULL OR
  nonzero_blob8(last_global_sequence)), runtime_spec_version INTEGER NOT NULL
  CHECK(typeof(runtime_spec_version)='integer' AND runtime_spec_version BETWEEN 0
  AND 4294967295), runtime_code_hash BLOB NOT NULL CHECK(hash32(runtime_code_hash)),
  FOREIGN KEY(block_number,block_hash) REFERENCES projected_blocks(block_number,
  block_hash) ON UPDATE RESTRICT ON DELETE RESTRICT, FOREIGN
  KEY(last_global_sequence) REFERENCES projected_events(global_sequence) ON
  UPDATE RESTRICT ON DELETE RESTRICT)`.
- `intent_units(id TEXT NOT NULL PRIMARY KEY CHECK(typeof(id)='text' AND
  length(CAST(id AS BLOB))=36 AND instr(id,char(0))=0), envelope_version INTEGER
  NOT NULL CHECK(typeof(envelope_version)='integer' AND envelope_version=2),
  envelope TEXT NOT NULL CHECK(typeof(envelope)='text' AND length(CAST(envelope
  AS BLOB)) BETWEEN 1 AND 2097152), origin_namespace TEXT NOT NULL
  CHECK(namespace(origin_namespace)), origin_scope TEXT NOT NULL
  CHECK(bounded_text(origin_scope)), origin_value TEXT NOT NULL
  CHECK(bounded_text(origin_value)), workflow_id TEXT NOT NULL
  CHECK(bounded_text(workflow_id)), species TEXT NOT NULL CHECK(bounded_text(species)),
  phase TEXT NOT NULL CHECK(bounded_text(phase)), status TEXT NOT NULL
  CHECK(typeof(status)='text' AND status IN ('active','completed')), revision BLOB
  NOT NULL CHECK(blob8(revision)), last_global_sequence BLOB NOT NULL
  CHECK(nonzero_blob8(last_global_sequence)) REFERENCES
  projected_events(global_sequence) ON UPDATE RESTRICT ON DELETE RESTRICT)`.
- `relationship_definitions(definition_id TEXT NOT NULL CHECK(namespace(definition_id)),
  definition_version BLOB NOT NULL CHECK(nonzero_blob8(definition_version)),
  directed INTEGER NOT NULL CHECK(typeof(directed)='integer' AND directed=1),
  source_species TEXT CHECK(source_species IS NULL OR bounded_text(source_species)),
  target_species TEXT CHECK(target_species IS NULL OR bounded_text(target_species)),
  self_policy TEXT NOT NULL CHECK(self_policy IN ('allow','reject')), cycle_policy
  TEXT NOT NULL CHECK(cycle_policy IN ('allow','reject')), created_global_sequence
  BLOB NOT NULL CHECK(nonzero_blob8(created_global_sequence)) REFERENCES
  projected_events(global_sequence) ON UPDATE RESTRICT ON DELETE RESTRICT,
  PRIMARY KEY(definition_id,definition_version))`.
- `intent_unit_relationships(definition_id TEXT NOT NULL, definition_version BLOB
  NOT NULL CHECK(nonzero_blob8(definition_version)), source_id TEXT NOT NULL,
  target_id TEXT NOT NULL, created_global_sequence BLOB NOT NULL
  CHECK(nonzero_blob8(created_global_sequence)) REFERENCES
  projected_events(global_sequence) ON UPDATE RESTRICT ON DELETE RESTRICT,
  PRIMARY KEY(definition_id,definition_version,source_id,target_id), FOREIGN KEY
  (definition_id,definition_version) REFERENCES relationship_definitions
  (definition_id,definition_version) MATCH NONE ON UPDATE RESTRICT ON DELETE
  RESTRICT, FOREIGN KEY(source_id) REFERENCES intent_units(id) MATCH NONE ON
  UPDATE RESTRICT ON DELETE RESTRICT, FOREIGN KEY(target_id) REFERENCES
  intent_units(id) MATCH NONE ON UPDATE RESTRICT ON DELETE RESTRICT)`.
- `recorded_associations(unit_id TEXT NOT NULL, subject_kind TEXT NOT NULL
  CHECK(subject_kind IN ('whole_unit','revision')), subject_revision_key BLOB NOT
  NULL CHECK((subject_kind='whole_unit' AND typeof(subject_revision_key)='blob'
  AND length(subject_revision_key)=0) OR (subject_kind='revision' AND
  blob8(subject_revision_key))), namespace TEXT NOT NULL CHECK(namespace(namespace)),
  scope TEXT NOT NULL CHECK(bounded_text(scope)), value TEXT NOT NULL
  CHECK(bounded_text(value)), created_global_sequence BLOB NOT NULL
  CHECK(nonzero_blob8(created_global_sequence)) REFERENCES
  projected_events(global_sequence) ON UPDATE RESTRICT ON DELETE RESTRICT,
  PRIMARY KEY(unit_id,subject_kind,subject_revision_key,namespace,scope,value),
  FOREIGN KEY(unit_id) REFERENCES intent_units(id) MATCH NONE ON UPDATE RESTRICT
  ON DELETE RESTRICT)`. Empty revision
  key is whole-unit; an exact revision, including zero, is its eight-byte value.
  Revocation removes only this active row; `projected_events` retains history.

The DDL generator expands `namespace`, `bounded_text`, `blob8`,
`nonzero_blob8`, and `hash32` into the literal expressions above before issuing
SQL; they are not SQLite types or runtime functions. The only named indexes are
`projected_blocks_by_hash(block_hash)` UNIQUE,
`projected_blocks_by_number_hash(block_number,block_hash)` UNIQUE,
`projected_events_by_sequence(global_sequence)` UNIQUE,
`intent_units_by_workflow(workflow_id,id)`,
`intent_units_by_species(species,id)`, `intent_units_by_phase(phase,id)`,
`intent_units_by_status(status,id)`,
`relationship_edges_by_source(definition_id,definition_version,source_id,target_id)`,
`relationship_edges_by_target(definition_id,definition_version,target_id,source_id)`,
`recorded_associations_by_unit(unit_id,subject_kind,subject_revision_key,namespace,scope,value)`,
and `recorded_associations_by_reference(namespace,scope,value,unit_id,subject_kind,subject_revision_key)`.
Validation covers column type/nullability/order, PK order, literal `CHECK` SQL,
every immediate `MATCH NONE` `ON UPDATE/DELETE RESTRICT` foreign key, index
uniqueness/origin/partial flags/order, SQLite-created autoindexes, `wr=0`,
`strict=1`, and absence of any extra table/index/trigger/view.

Every connection fixes `SQLITE_LIMIT_LENGTH=4_194_304` (row/value safety ceiling;
the envelope column remains independently capped at `2_097_152`),
`SQLITE_LIMIT_SQL_LENGTH=65_536`, `SQLITE_LIMIT_COLUMN=64`,
`SQLITE_LIMIT_EXPR_DEPTH=64`, `SQLITE_LIMIT_COMPOUND_SELECT=8`,
`SQLITE_LIMIT_VDBE_OP=100_000`, `SQLITE_LIMIT_FUNCTION_ARG=32`,
`SQLITE_LIMIT_ATTACHED=0`, `SQLITE_LIMIT_LIKE_PATTERN_LENGTH=256`,
`SQLITE_LIMIT_VARIABLE_NUMBER=128`, `SQLITE_LIMIT_TRIGGER_DEPTH=0`, and
`SQLITE_LIMIT_WORKER_THREADS=0`. Memory mapping is zero. Defensive mode,
foreign keys, and cell-size checking are on; trusted/writable schema, DQS
DML/DDL, trigger/view execution, FTS tokenizer, ATTACH create/write, SQL
comments, and extension loading are off and read back where SQLite exposes a
runtime value.
The checked production-SQL inventory is private, static, comment-free, and has
no caller SQL seam. `temp_store=MEMORY`,
`busy_timeout=5000`, and no application-level SQLite retry are exact; SQLite
sleeps until at least 5,000 accumulated milliseconds before typed Busy and the
test uses a separate 7,500-ms outer timeout to allow scheduling overhead. New
files set UTF-8 before the first schema object and use page size `4096`. Read-only
preflight rejects a file larger than `1_073_741_824` bytes, with another
encoding/page size, or with `page_count>262_144`. Because `max_page_count` is
connection-local rather than persistent, every writer sets and reads back
`max_page_count=262_144` under its write lock before any write; readers do not.

The connection-role matrix is exact. Every allowed action requires
`accessor=None`; variants for which SQLite 3.53.2 supplies a schema require
`database_name=Some("main")`, while `Select`, `Function`, `Transaction`,
`Pragma`, and other non-schema variants require `None`. `Unknown` and
`Recursive` always deny. The checked local rusqlite patch is required because
raw SQLite authorizer tuple `(22,"COMMIT",NULL,NULL,NULL)` otherwise maps to
`Transaction(Unknown)` in 0.40.2; after the patch the exact transaction tuples
are `Transaction(Begin|Commit|Rollback)`, while `Release`, savepoint operations,
unknown spellings, and every other transaction tuple deny.

The independently authored `tests/fixtures/sqlite-authorizer-v1.json` locks the
complete `(raw_code,variant,arg1,arg2,database,accessor)` trace for every static
statement and role against bundled SQLite 3.53.2 before production statements
are implemented. It spells every tuple literally—no wildcard, prefix,
open-ended `pragma_*`, generated-oracle refresh, or statement-identity guess is
allowed. The closed common scalar `Function` names are only `typeof`, `length`,
`instr`, `char`, `hex`, `coalesce`, and `glob`. Production SQL contains no
aggregate or `count(*)`; bounded fetched rows are counted in Rust. Therefore
`Function("count")` and SQLite's special empty-column/NULL-database `Read`
callback for `count(*)` always deny.

SQLite schema introspection is locked to SQLite's actual authorizer names, not
SQL aliases. Reading `sqlite_schema` authorizes `Read("sqlite_master",column)`
for exactly `type|name|tbl_name|rootpage|sql`. The table-valued PRAGMA queries
authorize `Select`, the corresponding `Read(table,column)` with database
`main`, and the matching base-name `Pragma(name,arg-or-None)` with no database:

- `pragma_table_list`: `schema|name|type|ncol|wr|strict`, plus
  `Pragma("table_list",None)`;
- `pragma_table_info`: `cid|name|type|notnull|dflt_value|pk`, plus
  `Pragma("table_info",<one exact application table>)`;
- `pragma_index_list`: `seq|name|unique|origin|partial`, plus
  `Pragma("index_list",<one exact application table>)`;
- `pragma_index_info`: `seqno|cid|name`, plus
  `Pragma("index_info",<one exact named or SQLite autoindex>)`;
- `pragma_index_xinfo`: `seqno|cid|name|desc|coll|key`, plus
  `Pragma("index_xinfo",<one exact named or SQLite autoindex>)`;
- `pragma_foreign_key_list`: `id|seq|table|from|to|on_update|on_delete|match`,
  plus `Pragma("foreign_key_list",<one exact application table>)`;
- `pragma_integrity_check`: `integrity_check`, plus
  `Pragma("integrity_check",None)`; and
- `pragma_foreign_key_check`: `table|rowid|parent|fkid`, plus
  `Pragma("foreign_key_check",None)`.

Preflight and public-reader connections use `READ_ONLY|NOFOLLOW`,
`query_only=ON`. Their static-query traces permit only the exact application
table/column reads used by the named queries, the `sqlite_master` and PRAGMA
virtual-table tuples above, the closed scalar functions, and exact read/config
`Pragma` pairs for `user_version|encoding|page_size|page_count|integrity_check|
foreign_key_check|journal_mode|synchronous|foreign_keys|trusted_schema|
query_only|cell_size_check|mmap_size|temp_store|busy_timeout|max_page_count`
with the literal None/value forms in the fixture. The public reader additionally
permits only `Pragma("data_version",None)` after common defensive configuration
and before the pinned read transaction is consumed; no other role, value, or
write form may use it. Non-SQL database-config, limit, busy-handler, and
extension-disable calls occur before authorizer installation; the authorizer is
installed before the first SQL/PRAGMA statement and remains installed.
The exclusive creator first opens the exact direct child with safe rustix
`O_CREAT|O_EXCL|O_NOFOLLOW|O_CLOEXEC`, mode `0600`, validates the returned
regular-file identity, and fails on any preexisting target before SQLite access.
SQLite then opens that already-created empty file through built-in `unix` using
only `READ_WRITE|NOFOLLOW`—never SQLite `CREATE` or `EXCLUSIVE`, which are not an
OS exclusivity primitive—sets `query_only=OFF`, and installs its authorizer
before schema SQL. The creator admits `CreateTable` only for the exact eight
application tables. It admits `CreateIndex` only for the eleven named indexes
above and the six SQLite primary-key autoindexes
`sqlite_autoindex_projected_blocks_1`, `sqlite_autoindex_projected_events_1`,
`sqlite_autoindex_intent_units_1`,
`sqlite_autoindex_relationship_definitions_1`,
`sqlite_autoindex_intent_unit_relationships_1`, and
`sqlite_autoindex_recorded_associations_1`, each paired with its exact table and
key-column read order. Named-index creation additionally admits `Reindex` only
for those eleven literal named indexes; every autoindex/unknown Reindex denies.
The creator's internal schema tuples are only `Insert("sqlite_master")`,
`Update("sqlite_master",type|name|tbl_name|rootpage|sql)`, and
`Read("sqlite_master","ROWID")`, all on `main`, plus the exact application
key/index column reads emitted by those 19 CREATE statements. It otherwise
admits only the patched transaction operations, `Select`, the closed scalar
functions, and the exact schema-v3 tuples in the independent trace;
its `Pragma` allowlist is `user_version|encoding|page_size|journal_mode|
synchronous|foreign_keys|trusted_schema|query_only|cell_size_check|mmap_size|
temp_store|busy_timeout|max_page_count`. The
projector writer uses `READ_WRITE|NOFOLLOW`, `query_only=OFF`, and an authorizer
limited to patched `Transaction`, `Select`, `Read`, `Insert`, `Update`, and
`Delete` on the exact eight application tables/columns and statement-specific
column orders, the same finite functions, and
read/configuration PRAGMAs from the creator list; it cannot execute any create,
drop, alter, reindex, analyze, virtual-table, savepoint, trigger, view, or temp
action. During its mandatory lock-time schema revalidation only, its independent
statement entries additionally admit the identical literal `sqlite_master` and
eight PRAGMA virtual-table `Select`/`Read`/base-`Pragma` tuples enumerated above;
no projector DML statement admits those tuples, and this is not a general
schema/virtual-table allowance. Every role denies ATTACH/DETACH,
extension functions, writable/trusted schema, triggers/views, caller SQL, and
all unlisted authorizer tuple/action/function/PRAGMA codes while retaining the common
extension/comment/DQS/URI/mmap/limit/temp-store/defensive protections.

Before any SQLite open of an existing/unknown file, the already validated
`O_RDONLY|O_NOFOLLOW` descriptor must yield at least one aligned 4096-byte page
and an exact fixed header: `SQLite format 3\0`, page-size bytes `0x10 0x00`,
write/read format bytes 18/19 both `1` (rollback journal, never WAL), reserved
space zero, and big-endian text-encoding field at bytes 56..60 equal `1`
(UTF-8). Any short/misaligned/unknown/WAL header or existing `-journal`/`-wal`/
`-shm` child rejects before `sqlite3_open_v2`, and syscall tracing must show no
sidecar creation or recovery attempt. Only then does SQLite open
`READ_ONLY|NOFOLLOW` with the canonical absolute direct-child path and no
`SQLITE_OPEN_URI` flag.
Before any schema SQL it applies and reads back all available connection-local
database configuration, limits, cell-size checking, zero mmap, memory temp
store, busy timeout, deny authorizer, extension disable, and query-only mode;
it separately verifies the compile-time comment-free static-SQL inventory, then
requires full
`PRAGMA integrity_check` to
return exactly one `ok` row, `PRAGMA foreign_key_check` to return zero rows, and
validates exact schema without any write PRAGMA or recovery. Only a file that
passes may be reopened by the crate-private projector as
`READ_WRITE|NOFOLLOW`; the writer applies its role-specific `query_only=OFF`
authorizer plus all common defenses, acquires its transaction, and revalidates
under the write lock before any projection. Public readers always remain OS
read-only plus SQLite query-only. New creation is a separate exclusive owner-only operation,
never adoption of an existing empty or unknown file.

Fresh creation commits schema only: no anchor, block, checkpoint, unit, edge, or
association row exists and no `VerifiedReadSnapshot` can be minted. T-1110's
first backend-owned sync transaction verifies the fixed manifest and finalized
block zero, then atomically inserts `projection_anchor`, the block-zero
`projected_blocks` row (`cubikan_event_count=0`, null per-block sequences), and
the block-zero checkpoint (`last_global_sequence=NULL`). A later zero-event block
keeps its own first/last null but retains the checkpoint's prior nonzero global
sequence. Event coordinates obtain block hash only by joining the event's
`block_number` to its immutable `projected_blocks` row.

Envelope v2 is one closed canonical JSON object whose members serialize in this
exact order: `{representation_version:2,id,origin,species,workflow,phase,status,
revision,history}`. It uses the protocol contract's exact `Uuid`,
`ExternalReference`, `Text256`, `Workflow`, `U64Text`, member ordering, UTF-8,
escaping, duplicate/unknown rejection, and no-whitespace rules; unlike protocol
stdout it has no trailing LF. `status` is exactly `active|completed`. `history`
contains 0..=256 ordered closed objects, exactly
`{type:"transition",sequence,from,to}` or
`{type:"completion",sequence,phase}`, with every sequence and the top-level
revision encoded as `U64Text`. Every member is required and explicit null is
forbidden. No signer, timestamp, relationship, association, provider payload,
checkpoint, coordinate, or extra member exists. Loading replays lifecycle
history through `cubikan-core`, then compares every unit projection and its
referenced accepted-event coordinate. Relationship and association state are
separately replayed from accepted events; an envelope is never chain proof.
Before serializer implementation, independently authored raw cases and
`manifest-v1.json` under `tests/fixtures/envelope-v2/` lock exact bytes/SHA-256,
maxima, escaping, malformed/duplicate/unknown/null cases, and the checked
worst-case formula across all 256 records and maximum text/graph bounds.
Production serialization cannot generate or refresh that oracle; both formula
and adversarial maximum must remain at or below the locked 2,097,152-byte
envelope/SQLite length ceiling. A 257th lifecycle record rejects on chain before
state/event mutation.

`VerifiedReadSnapshot` is a non-serializable, single-read/page capability that
owns one exact OS-read-only SQLite connection and pinned read transaction at
checkpoint `C`. Issuance first observes candidate C, fetches every finalized
parachain block from the deployment anchor through C outside a SQLite
transaction, validates the complete ordered CubiKan event stream, and derives
the expected typed state. It then begins the read transaction, pins it by
reading the checkpoint, requires it still equals C, and compares the complete
schema-v3 snapshot before executing exactly one read/page. It binds file
identity, deployment anchor, block/hash, nullable last global sequence,
and `PRAGMA data_version`. Only the finalized-
stream attestor in T-1110 can create it. Under DELETE journaling, a writer's C+1
commit waits only in SQLite's 5,000-ms busy handler or returns typed Busy while
this snapshot exists; the query
therefore returns entirely C. The snapshot and capability are then discarded,
after which the writer may commit C+1 and a newly attested page may expose live
membership. A candidate/checkpoint mismatch before pinning yields
`RefreshRequired`; a successfully pinned C page is never retroactively
discarded. T-1109 tests construct the otherwise-private capability only through
a `cfg(test)` backend-module harness; production construction remains
T-1110-only. No network-duration read lock or cross-page snapshot is claimed.
Structural SQLite
checks alone can report coherence but can never mint this capability.
The capability attests equality with one configured node's finalized RPC stream;
it does not independently verify GRANDPA proofs, validator honesty, or economic
shared security.

## Locked protocol-v2 schema and inventory

Both adapters accept exactly one RFC-8259 UTF-8 JSON value and at most
`1_048_576` raw request bytes; the streaming ingress buffer/read counter retains
at most `1_048_577` raw bytes to detect overflow. This is not a total heap/RSS
bound; external resource containment/measurement remains T-1117's concern.
Every object at every depth rejects duplicate and unknown members. Required
members must be present with their exact JSON type; optional input members are
omission-only and explicit `null` rejects. Tags are lowercase `snake_case` and
text is never trimmed, case-folded, Unicode-normalized, or URL-normalized.
Output object members serialize in the exact order written in this contract,
with no insignificant whitespace. Strings emit Unicode scalar values as UTF-8,
escape quote/backslash and U+0008/U+0009/U+000A/U+000C/U+000D as
`\"|\\|\b|\t|\n|\f|\r`, escape every other U+0000..U+001F byte as lowercase
`\u00xx`, and never escape `/` or non-ASCII solely for transport. This ordering
and escaping, plus one final LF, makes independently hashed stdout canonical.

The common scalar and object encodings are locked as follows:

- `Uuid` is lowercase hyphenated RFC-4122 text of exactly 36 ASCII bytes. A
  create request may omit `id`, causing the adapter to generate it before
  construction or signing; explicit null rejects.
- Every domain `u64` (revision, lifecycle sequence, definition version, global
  sequence, and block number) is a canonical unsigned-decimal JSON string:
  `"0"` or `[1-9][0-9]*` within `u64`; definition versions and global sequences
  are additionally nonzero. These values never accept JSON numbers.
- `protocol_version`, `query_version`, page limit, runtime spec version,
  extrinsic index, and System event index are JSON integers in range;
  `protocol_version=2`, `query_version=1`, and `limit=1..=100`.
- `Hash32` is `0x` plus exactly 64 lowercase hexadecimal digits. Git OIDs are
  exactly 40 lowercase hex for `git.commit.sha1` or 64 lowercase hex for
  `git.commit.sha256`, with no `0x`.
- `Text256` is nonblank NUL-free UTF-8 of 1..=256 bytes. `Namespace` is exact
  ASCII `[a-z][a-z0-9._-]{0,63}`. `ExternalReference` is exactly
  `{namespace:Namespace,scope:Text256,value:Text256}`.
- `Workflow` is exactly `{id:Text256,phases:[Text256;1..=32],initial_phase:
  Text256,edges:[{from:Text256,to:Text256};0..=128],completion_phases:
  [Text256;0..=32]}` and obeys the shared core topology rules.
- `DefinitionKey` is exactly `{id:Namespace,version:NonzeroU64Text}`;
  `RelationshipKey` is exactly `{definition:DefinitionKey,source_id:Uuid,
  target_id:Uuid}`.
- `AssociationSubject` is exactly `{type:"whole_unit"}` or
  `{type:"revision",revision:U64Text}`. Revision zero is not whole-unit.
  `AssociationKey` is exactly `{unit_id:Uuid,subject:AssociationSubject,
  reference:ExternalReference}`.
- `LedgerCoordinate` is exactly `{parachain_genesis_hash:Hash32,
  deployment_id:Hash32,block_number:U64Text,block_hash:Hash32,
  extrinsic_index:u32,extrinsic_hash:Hash32,system_event_index:u32,
  global_sequence:NonzeroU64Text}`. Its block hash must equal the restrictive
  block-number join. `FinalizedExtrinsicCoordinate` contains the first six
  fields through `extrinsic_hash`. `ProjectionCheckpoint` is exactly
  `{block_number:U64Text,block_hash:Hash32,last_global_sequence:
  NonzeroU64Text|null,runtime_spec_version:u32,runtime_code_hash:Hash32}`.
  Output `null` is used only for an absent next cursor, an absent lagging
  checkpoint, or checkpoint `last_global_sequence` before any CubiKan event.
- `JsonPointer256` is NUL-free UTF-8 from 0..=256 bytes following RFC 6901;
  empty string means the document root, and `~`/`/` tokens use exact `~0`/`~1`
  escaping. It is distinct from nonblank `Text256`.
- `MutationOperation` is one of the exact mutation tags
  `create_intent_unit|transition_intent_unit|complete_intent_unit|
  create_relationship_definition|create_relationship|delete_relationship|
  record_association|revoke_association`. `MortalEra` is exactly
  `{birth:U64Text,death:U64Text}` with `death=birth+63`. An error `field`, when
  present, is `JsonPointer256`; an
  `operation_number`, when present on stateless simulation failure, is a JSON
  integer `0..=255` indexing the zero-based `operations` array.

### Stateless `cubikan` v2

The request has exactly `protocol_version`, `workflow`, `intent_unit`, and
`operations`. `intent_unit` is exactly `{id?,origin,species}`; `id` is its only
optional member. `operations` contains 0..=256 entries and only
`{type:"transition",target:Text256}` or `{type:"complete"}`. This adapter has
no RPC, database, signer, command-schema, coordinate, relationship, provenance,
or durable-state operation.

Success is exactly `{protocol_version:2,authority:"simulation_only",outcome:
"success",result:{type:"simulation",intent_unit:UnitView}}`. Failure is exactly
`{protocol_version:2,authority:"simulation_only",outcome:"error",error:
ErrorDetail,intent_unit?}`; the unit appears only after an operation-level
lifecycle rejection. `UnitView` is exactly `{id,origin,species,workflow,phase,
status,revision,history}` where status is `active|completed` and ordered history
variants are `{type:"transition",sequence,from,to}` or
`{type:"completion",sequence,phase}`. Revision and sequence use `U64Text`.
Nothing on this surface reports canonical, accepted, committed, finalized,
ledger, or verified authority.

### Chain-backed `cubikan-local` v2

The request is exactly `{protocol_version:2,operation:Operation}`. The adapter,
not the caller, supplies SCALE `command_schema_version=1`. Its complete
operation inventory and fields are:

1. `create_intent_unit {intent_unit:{id?,origin,species},workflow}`;
2. `get_intent_unit {id}`;
3. `list_intent_units {filters:{workflow_id?,species?,phase?,status?},limit,
   after?}`, where status is `active|completed` and `after=Uuid`;
4. `transition_intent_unit {id,target,expected_revision}`;
5. `complete_intent_unit {id,expected_revision}`;
6. `create_relationship_definition {definition,source_species?,target_species?,
   self_policy,cycle_policy}`, where policy is `allow|reject`, omission means an
   unconstrained species, and direction is fixed to directed;
7. `get_relationship_definition {definition}`;
8. `create_relationship {relationship:RelationshipKey}`;
9. `delete_relationship {relationship:RelationshipKey}`;
10. `list_relationships {definition,source_id?,target_id?,limit,after?}`, where
    `after` is a complete same-definition `RelationshipKey`;
11. `project_intent_units_v1 {query_version:1,filters:{workflow_id?,species?,
    phase?,status?},predicate?,limit,after?}`, where predicate is exactly
    `{type:"outgoing",definition,anchor_id}` or
    `{type:"incoming",definition,anchor_id}` and `after=Uuid`;
12. `record_association {association:AssociationKey}`;
13. `revoke_association {association:AssociationKey}`;
14. `list_associations_by_unit {unit_id,subject?,limit,after?}`; and
15. `list_associations_by_reference {reference,limit,after?}`.

Each listed object additionally has the exact `type` tag shown. Association
cursors are complete `AssociationKey` values; all cursors are exclusive and
`next_cursor` uses the identical encoding. No opaque token is accepted.

Read success is exactly `{protocol_version:2,outcome:"success",result:Result}`.
The complete tagged `Result` union consists of the following literal closed
objects: `{type:"intent_unit",intent_unit:ProjectedUnit,checkpoint}`;
`{type:"intent_unit_page",items:[UnitSummary],next_cursor:Uuid|null,checkpoint}`;
`{type:"relationship_definition",definition:ProjectedDefinition,checkpoint}`;
`{type:"relationship_page",items:[ProjectedRelationship],next_cursor:
RelationshipKey|null,checkpoint}`; `{type:"projection_v1_page",query_version:1,
items:[UnitSummary],next_cursor:Uuid|null,checkpoint}`; and
`{type:"association_page",direction:"by_unit"|"by_reference",items:
[ProjectedAssociation],next_cursor:AssociationKey|null,checkpoint}`.
`ProjectedUnit` is `UnitView` plus exactly `last_coordinate:LedgerCoordinate`;
`UnitSummary` is exactly `{id,origin,species,workflow_id,phase,status,revision,
last_coordinate}`. `ProjectedDefinition` is exactly `{key:DefinitionKey,
directed:true,source_species?,target_species?,self_policy:"allow"|"reject",
cycle_policy:"allow"|"reject",created_coordinate:LedgerCoordinate}`, with
absent constraints omitted. `ProjectedRelationship` is exactly
`{key:RelationshipKey,created_coordinate:LedgerCoordinate}` and
`ProjectedAssociation` exactly `{key:AssociationKey,created_coordinate:
LedgerCoordinate}`.
Every row and checkpoint consumes one `VerifiedReadSnapshot`; no capability or
attestation token serializes.

Every local failure that is not one of the seven mutation-delivery outcomes is
exactly `{protocol_version:2,outcome:"error",error:ErrorDetail}`. This includes
request/shape/version rejection, read misses (including `intent_unit_not_found`
and `relationship_definition_not_found` rather than nullable success), cursor
or coordinate rejection, platform/path/storage failure, RPC/archive/identity/
runtime failure, attestation mismatch, `refresh_required`, and projection Busy.
It contains no `operation`, era, coordinate, result, or partial page. Codes in
the parse/value/storage/RPC/read subsets of the finite registry map exhaustively
to this envelope and their locked exit class; the seven mutation outcomes below
are the only other `cubikan-local` top-level response variants.

A mutation returns exactly one top-level outcome, never generic success before
finality:

- `{protocol_version:2,outcome:"submission_rejected",operation,error}` only for
  typed preparation/validation rejection proven before `submit_and_watch` is
  invoked and with send counter zero; it creates no durable prepared journal;
- `{protocol_version:2,outcome:"submission_lane_unresolved",operation,
  expected_extrinsic_hash,era,error}`
  with zero new sends;
- `{protocol_version:2,outcome:"expired_not_included",operation,
  expected_extrinsic_hash,era,error}` only after finalized head is past death and
  the complete inclusive birth-through-death scan proves the exact hash absent;
  it sends nothing, durably publishes the resolved journal state, emits this
  reproducible response, and only then fsync-removes the resolved file;
- `{protocol_version:2,outcome:"finalized_dispatch_rejected",operation,
  finalized_extrinsic,error}` for an
  included fee/nonce-consuming dispatch with no accepted CubiKan event;
- `{protocol_version:2,outcome:"finalized_invariant_failed",operation,
  finalized_extrinsic,error}` for a successfully included exact extrinsic whose
  CubiKan event count/identity violates the locked invariant; it resolves the
  journal durably, clears only after fsynced resolution, and is never retried;
- `{protocol_version:2,outcome:"delivery_indeterminate",operation,
  expected_extrinsic_hash,era,error}` for a
  possibly sent extrinsic, never retried or described as rollback; or
- `{protocol_version:2,outcome:"finalized_accepted",operation,coordinate,
  effect,projection}` only for exactly
  one matching event inside the exact extrinsic. `effect` is exactly one closed
  object: `{type:"unit_created",unit_id,committed_revision}`,
  `{type:"unit_transitioned",unit_id,committed_revision}`,
  `{type:"unit_completed",unit_id,committed_revision}`,
  `{type:"relationship_definition_created",definition:DefinitionKey}`,
  `{type:"relationship_created",relationship:RelationshipKey}`,
  `{type:"relationship_deleted",relationship:RelationshipKey}`,
  `{type:"association_recorded",association:AssociationKey}`, or
  `{type:"association_revoked",association:AssociationKey}`. `projection` is
  exactly `{status:"caught_up",checkpoint:ProjectionCheckpoint}` only when a
  fresh attested read contains that effect, otherwise exactly
  `{status:"lagging",checkpoint:ProjectionCheckpoint|null}`.

`ErrorCode` is the following closed string enum. `ErrorDetail` is exactly
`{code:ErrorCode,message:Text256,field?,operation_number?,expected_revision?,
actual_revision?}`. `message` is diagnostic and not machine-interpreted, but its
exact bytes are stable within fixture schema v1 because stdout is hashed.
`field` is required exactly for `invalid_request`,
`unsupported_protocol_version`, `invalid_intent_unit_id`,
`invalid_external_reference`, `invalid_species`, `invalid_workflow_id`,
`invalid_phase_id`, `invalid_workflow`, `invalid_revision`,
`invalid_definition_id`, `invalid_definition_version`,
`invalid_relationship_policy`, `invalid_association_subject`, `invalid_query`,
`invalid_cursor`, `invalid_coordinate`, and `invalid_rpc_endpoint`; it is
forbidden for all other codes. `operation_number` is required exactly for a
stateless `transition_already_completed`, `transition_unknown_target`,
`transition_not_allowed`, `completion_already_completed`, or
`completion_phase_not_eligible` operation failure and forbidden otherwise.
`expected_revision` and `actual_revision` occur together and are required only
for `revision_conflict`; both are forbidden otherwise. The finite code registry is:

`malformed_json`, `request_too_large`, `invalid_request`,
`unsupported_protocol_version`, `invalid_intent_unit_id`,
`invalid_external_reference`, `invalid_species`, `invalid_workflow_id`,
`invalid_phase_id`, `invalid_workflow`, `invalid_revision`,
`invalid_definition_id`, `invalid_definition_version`,
`invalid_relationship_policy`, `invalid_association_subject`, `invalid_query`,
`invalid_cursor`, `invalid_coordinate`, `invalid_rpc_endpoint`,
`unsupported_platform`, `insecure_projection_path`, `projection_busy`,
`unsupported_schema_version`, `unsupported_envelope_version`, `corrupt_schema`,
`corrupt_envelope`, `projection_mismatch`, `refresh_required`,
`archive_rpc_unavailable`, `archive_history_unavailable`, `deployment_mismatch`,
`runtime_mismatch`, `unsupported_event_schema_version`,
`conflicting_finalized_block`, `projection_error`, `dev_signer_unavailable`,
`submission_lane_corrupt`, `submission_lane_unresolved`, `nonce_conflict`,
`insufficient_balance`, `transaction_invalid`, `rpc_submission_rejected`,
`submission_watch_lost`, `submission_timeout`, `finalized_invariant_failed`,
`expired_not_included`,
`unsupported_command_schema_version`, `unsigned_call`,
`unauthorized_submitter`, `duplicate_intent_unit`, `intent_unit_not_found`,
`revision_conflict`, `lifecycle_history_capacity_exceeded`,
`transition_already_completed`, `transition_unknown_target`,
`transition_not_allowed`, `completion_already_completed`,
`completion_phase_not_eligible`, `global_sequence_exhausted`,
`relationship_definition_already_exists`, `relationship_definition_not_found`,
`relationship_source_not_found`, `relationship_target_not_found`,
`relationship_source_species_mismatch`, `relationship_target_species_mismatch`,
`self_relationship_rejected`, `duplicate_relationship`, `cycle_rejected`,
`relationship_capacity_exceeded`, `relationship_not_found`,
`association_revision_out_of_range`, `duplicate_association`,
`association_capacity_exceeded`, and `association_not_found`.

The response-code matrix is closed; a code is illegal in every adapter/envelope
not listed below.

- `cubikan` generic error, exit `2`: `malformed_json`, `request_too_large`,
  `invalid_request`, `unsupported_protocol_version`, `invalid_intent_unit_id`,
  `invalid_external_reference`, `invalid_species`, `invalid_workflow_id`,
  `invalid_phase_id`, and `invalid_workflow`.
- `cubikan` operation error, exit `3`: `transition_already_completed`,
  `transition_unknown_target`, `transition_not_allowed`,
  `completion_already_completed`, or `completion_phase_not_eligible`.
- `cubikan-local` generic error, exit `2`: `malformed_json`, `request_too_large`,
  `invalid_request`, `unsupported_protocol_version`, `invalid_intent_unit_id`,
  `invalid_external_reference`, `invalid_species`, `invalid_workflow_id`,
  `invalid_phase_id`, `invalid_workflow`, `invalid_revision`,
  `invalid_definition_id`, `invalid_definition_version`,
  `invalid_relationship_policy`, `invalid_association_subject`, `invalid_query`,
  `invalid_cursor`, `invalid_coordinate`, or `invalid_rpc_endpoint`.
- `cubikan-local` generic error, exit `4`: `unsupported_platform`,
  `insecure_projection_path`, `projection_busy`, `unsupported_schema_version`,
  `unsupported_envelope_version`, `corrupt_schema`, `corrupt_envelope`,
  `projection_mismatch`, `refresh_required`, `archive_rpc_unavailable`,
  `archive_history_unavailable`, `deployment_mismatch`, `runtime_mismatch`,
  `unsupported_event_schema_version`, `conflicting_finalized_block`, or
  `projection_error`.
- `cubikan-local` generic error, exit `1`: `dev_signer_unavailable` or
  `submission_lane_corrupt`; generic read-miss error, exit `3`:
  `intent_unit_not_found` or `relationship_definition_not_found`.
- `submission_rejected`, exit `3`: exactly `nonce_conflict`,
  `insufficient_balance`, or `transaction_invalid`, and only while the send
  counter is zero before the first `submit_and_watch` invocation.
- `submission_lane_unresolved`, exit `1`: exactly `submission_lane_unresolved`;
  `expired_not_included`, exit `1`: exactly `expired_not_included`.
- `finalized_dispatch_rejected`, exit `3`: exactly `runtime_mismatch` or one of
  `unsupported_command_schema_version`, `unsigned_call`,
  `unauthorized_submitter`, `duplicate_intent_unit`, `intent_unit_not_found`,
  `revision_conflict`, `lifecycle_history_capacity_exceeded`,
  `transition_already_completed`, `transition_unknown_target`,
  `transition_not_allowed`, `completion_already_completed`,
  `completion_phase_not_eligible`, `global_sequence_exhausted`,
  `relationship_definition_already_exists`, `relationship_definition_not_found`,
  `relationship_source_not_found`, `relationship_target_not_found`,
  `relationship_source_species_mismatch`, `relationship_target_species_mismatch`,
  `self_relationship_rejected`, `duplicate_relationship`, `cycle_rejected`,
  `relationship_capacity_exceeded`, `relationship_not_found`,
  `association_revision_out_of_range`, `duplicate_association`,
  `association_capacity_exceeded`, or `association_not_found`.
- `finalized_invariant_failed`, exit `1`: exactly `finalized_invariant_failed`.
  `delivery_indeterminate`, exit `1`: exactly `nonce_conflict`,
  `transaction_invalid`, `rpc_submission_rejected`, `submission_watch_lost`, or
  `submission_timeout`, and only after the first `submit_and_watch` invocation.
- `finalized_accepted`, read success, and stateless simulation success use exit
  `0` and contain no `ErrorDetail`.

The deliberate overlaps are phase/operation-disjoint. The two not-found codes
use the generic envelope only for reads and finalized-dispatch rejection only
for included mutations. `runtime_mismatch` is generic before a known finalized
dispatch coordinate and finalized-dispatch rejection only for an unknown pinned
dispatch variant inside the exact included extrinsic. `nonce_conflict` and
`transaction_invalid` use `submission_rejected` only before the send boundary
and `delivery_indeterminate` only after it. `rpc_submission_rejected` is never
zero-send proof: once `submit_and_watch` is invoked it is indeterminate.

`ErrorDetail` serializes in member order `code,message,field,operation_number,
expected_revision,actual_revision`, omitting absent optional members. `field`
uses exactly the 17-code allowlist above. `operation_number` is required exactly
for the five `cubikan` operation errors, forbidden for those codes in
`cubikan-local`, and forbidden otherwise. The revision pair is required exactly
for `revision_conflict`. These optional-member families never coexist. The
stateless failure envelope includes `intent_unit` exactly for its five operation
errors and forbids it for setup errors. Pallet mappings are compile-time
exhaustive; unknown metadata/dispatch variants become `runtime_mismatch`, never
a guessed domain code. Any stdout body, final-LF, or flush failure returns exit
`1` rather than the modeled exit.
One compact body, one LF, and one explicit flush must all complete before a
modeled exit is returned.

The send boundary is the first invocation of Subxt `submit_and_watch`. Only a
typed preparation/validation error before that invocation may become
`submission_rejected`, with send counter zero and no durable prepared state.
After invocation begins, transport loss, stream end, watcher `Invalid`,
`Dropped`, `Error`, timeout, or any unknown status leaves the prepared journal
unresolved and returns `delivery_indeterminate`; no Subxt watcher status alone
proves non-inclusion or permits journal removal. Resolution thereafter requires
the exact finalized inclusion outcome or the finalized-head-past-death complete
exact-hash absence scan.

Before either decoder/result implementation, independently author Draft
2020-12 structural schemas at `protocol/v2/cubikan.schema.json` and
`protocol/v2/cubikan-local.schema.json` plus separate raw fixture manifests at
`tests/fixtures/protocol-v2/cubikan/manifest-v1.json` and
`tests/fixtures/protocol-v2/cubikan-local/manifest-v1.json`. Each manifest is
exactly `{fixture_schema_version:1,hash_algorithm:"sha256",schema:{path,bytes,
sha256},cases:[...]}` and each case is exactly `{id,request:{path,bytes,sha256},
context?,stdout:{path,bytes,sha256},exit_code}`; the manifest does not hash
itself. When present, `context` is exactly `{generated_uuid?,rpc?,signer?,
projection?}`: `generated_uuid` is the fixed canonical UUID returned by the
private fixture-only ID source, while each other member is a
`{path,bytes,sha256}` independently authored raw transcript/snapshot. Omitted-ID
cases require `generated_uuid`; local mutation/read cases pin every nonce,
block, signature, RPC response, and projection input needed for deterministic
stdout. Production retains UUID-v4 CSPRNG generation and live RPC/signing; the
fixture injection seam is private/test-only and cannot be selected by JSON or
process arguments.
Hashes are lowercase SHA-256 over exact raw bytes, including stdout's terminal
LF. Fixtures cover every operation/result/outcome/code, omission versus null,
raw duplicate/unknown keys, scalar boundaries, codecs, and request sizes
`1_048_575`, `1_048_576`, and `1_048_577`. JSON Schema is not the duplicate-key
oracle; raw parser fixtures are. A small independent verifier may recompute
hashes/completeness, but adapter serializers, generated Rust/SCALE schemas, and
decoders may not generate expected bytes or hashes. Any contract change bumps
`fixture_schema_version` and receives review; implementation output can never be
used to refresh an expectation.

## Schema Tree

- Local blockchain-canonical CubiKan
  - Reproducible chain foundation
    - T-1101: isolated pinned SDK/tooling workspace
    - T-1102: bounded SCALE model and cross-runtime conformance corpus
  - Canonical runtime
    - T-1103: event envelope, authorization, and lifecycle pallet
    - T-1104: bounded relationship pallet semantics
    - T-1105: required-origin provenance pallet semantics
    - T-1106: local parachain runtime, chain spec, metadata, and weights
  - Verified read and process boundary
    - T-1107: required-origin core rebaseline and root-consumer bridge
    - T-1108: exact SQLite v3/envelope v2 store and connection hardening
    - T-1109: capability-gated v3 unit, relationship, projection, and provenance queries
    - T-1110: finalized projector and full archive-RPC stream attestation
    - T-1111: Subxt submission and cross-process signer journal
    - T-1112: stateless `cubikan` protocol v2
    - T-1113: chain-backed `cubikan-local` protocol v2
    - T-1114: provider-neutral Git reference adapter
  - Composition and operational evidence
    - T-1115: four-node Zombienet failover, resynchronization, and dual-node rebuild
    - T-1116: security/authority documentation and current-state reconciliation
    - T-1117: portable gates, separate chain CI, and measured resource budgets

## Execution Sequence

### T-1101: Pin and isolate the Polkadot SDK development toolchain
- **Intent:** [INT-0014](../../../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Touches:** `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `patches/rusqlite-0.40.2-commit-authorizer.patch`, `vendor/rusqlite-0.40.2-cubikan/**`, `chain/Cargo.toml`, `chain/Cargo.lock`, `chain/rust-toolchain.toml`, `chain/pins.toml`, `chain/README.md`, `chain/tools/**`, `chain/runtime/**`, `chain/pallets/cubikan/**`, `.gitignore`
- **Depends on:** (none)
- **Acceptance criterion:** One coherent exact SDK/runtime/tooling family is reproducible while SDK/FRAME/Cumulus stay isolated from the root workspace.
- **Success criterion (EARS):**
  - **T-1101-E1 — WHEN** the checked pin verifier runs after the one explicit dependency-fetch phase, **THEN** it **SHALL** resolve SDK commit `8ae9775dc43c0d8cdd0f6d87700596e14278b1e1`, Rust 1.93.0 plus exact rustfmt/clippy/`wasm32v1-none` identities, Subxt 0.50.2, Zombienet commit `a7c434271f094320d17cf94f7a2f95fdef417379`, the repository-owned argv normalizer and loopback-netns wrapper bytes/SHA-256 plus exact accepted generated-command grammar hash, exact relay/collator/chain-spec and same-commit benchmark-capable node/omni-node assets/commands/checksums, util-linux/iproute2, Node/npm toolchain, one same-family scaffold without a floating ref or 2512 dependency, and the exact rusqlite registry archive/pristine tree/patch/patched tree/normalized-diff hashes while reconstructing the checked vendor tree byte-for-byte, after which every gate runs inside the fail-closed loopback-only namespace and offline.
  - **T-1101-E2 — WHEN** Cargo inspects the root and `chain/` manifests at foundation and final candidate, **THEN** it **SHALL** report two isolated workspaces/lockfiles, require exact-version/default-features-false root declarations, every `subxt`/`subxt-*` lockfile package entry at exactly 0.50.2, exact requested features on the nine planned root packages, and SHA-256 equality of the complete resolved `cargo tree -e features --locked` closure to the T-1101 pin artifact, keep every Polkadot SDK/FRAME/Cumulus/node/runtime package and source chain-only, and build minimal chain native and Wasm targets with the pinned toolchain; at T-1101 itself the root delta is exactly the explicit `chain/` exclusion plus the checked local rusqlite patch override/vendor source and its corresponding lock-source identity, with no other root dependency or source change.
  - **T-1101-E3 — WHEN** an asset checksum, source revision, tool version, vendored source byte, patch/diff hash, lock-source identity, or generated artifact identity differs from `chain/pins.toml`, **THEN** verification **SHALL** fail before executing or building dependent network work.
- **Notes:** Derive the scaffold from the stable2606-1 SDK at the same commit. The external parachain-template HEAD is not an allowed basis while it declares SDK 2512.1.0. Do not download metadata during compilation.

### T-1102: Define bounded SCALE values and independent conformance fixtures
- **Intents:** [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md), [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md), [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md), [INT-0014](../../../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Touches:** `crates/cubikan-core/src/lib.rs`, `crates/cubikan-core/src/external_reference.rs`, `crates/cubikan-core/src/vocabulary.rs`, `crates/cubikan-core/src/workflow.rs`, `crates/cubikan-core/tests/**`, `chain/pallets/cubikan/src/types.rs`, `chain/pallets/cubikan/src/conformance.rs`, `chain/pallets/cubikan/src/tests/model.rs`, `tests/fixtures/chain-conformance-v1.json`
- **Depends on:** T-1101
- **Acceptance criterion:** Provider-neutral required-origin/provenance values and bounded chain equivalents have one exact validation contract without making `cubikan-core` depend on FRAME.
- **Success criterion (EARS):**
  - **T-1102-E1 — WHEN** namespace, scope, value, vocabulary, workflow, definition, edge, association, or collection inputs hit empty, exact-minimum, exact-maximum, maximum-plus-one, invalid UTF-8/NUL, or invalid grammar cases, **THEN** core and chain conversion **SHALL** return the exact locked value or typed error with byte-index/length precision and no normalization.
  - **T-1102-E2 — WHEN** the independent fixture corpus is evaluated by `cubikan-core` and bounded chain types, **THEN** both **SHALL** agree on valid workflow/lifecycle/relationship/provenance meaning while chain-only capacity and authorization errors remain explicitly distinct.
  - **T-1102-E3 — WHEN** chain values compile for Wasm, **THEN** every stored/call/event type **SHALL** implement bounded SCALE encoding, type metadata, and finite maximum encoded length without `std`, unbounded `String`/`Vec`, UUID generation, filesystem, clock, RPC, account-key, or provider dependencies, and every accepted event variant's mechanical `MaxEncodedLen` must be at most `1_048_576` bytes with one maximal fixture.
  - **T-1102-E4 — WHEN** SCALE bytes are truncated, malformed, missing a nonoptional origin, encode an invalid variant, or exceed a bounded collection before dispatch, **THEN** codec/preflight **SHALL** reject them without entering the pallet, reading domain storage, returning an in-domain pallet error, mutating state, or emitting an event.
- **Notes:** Add external-reference values additively here; the final origin-required aggregate rebaseline occurs in T-1107 so every intermediate root commit remains green.

### T-1103: Implement canonical lifecycle events and pallet mutations
- **Intents:** [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md), [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md), [INT-0014](../../../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Touches:** `chain/pallets/cubikan/src/lib.rs`, `chain/pallets/cubikan/src/event.rs`, `chain/pallets/cubikan/src/error.rs`, `chain/pallets/cubikan/src/benchmarking.rs`, `chain/pallets/cubikan/src/weights.rs`, `chain/pallets/cubikan/src/mock.rs`, `chain/pallets/cubikan/src/tests/lifecycle.rs`
- **Depends on:** T-1102
- **Acceptance criterion:** Authorized required-origin create and exact-revision lifecycle calls are deterministic, transactional, stale-first, and replay-complete.
- **Success criterion (EARS):**
  - **T-1103-E1 — WHEN** an authorized submitter creates a bounded valid unit, **THEN** the pallet **SHALL** store ID/origin/species/workflow/current state at revision zero, increment the global sequence once, and emit exactly one versioned replay-complete accepted event.
  - **T-1103-E2 — WHEN** a successfully decoded bounded lifecycle call is evaluated, **THEN** the pallet **SHALL** first return typed `UnsupportedCommandSchemaVersion` for a version other than 1 before authorization/domain reads and, for version 1, return the exact typed authorization, duplicate, or semantic-domain error with byte-equal `pallet-cubikan` domain storage/global sequence and no accepted domain event. Runtime nonce, fee, and System failure effects for an included dispatch are governed separately by the runtime-accounting clause in T-1106.
  - **T-1103-E3 — WHEN** transition or completion carries the current revision, **THEN** the pallet **SHALL** apply existing lifecycle policy, advance revision and global sequence exactly once, preserve immutable origin/workflow/species/ID, and emit one replay-complete event.
  - **T-1103-E4 — WHEN** expected revision is stale alongside any lifecycle-domain error, **THEN** stale conflict **SHALL** win after supported-version/authorization/target selection and before domain validity, with no storage or accepted-event mutation.
  - **T-1103-E5 — WHEN** two authorized extrinsics use the same expected revision in either canonical order, **THEN** sequential block execution **SHALL** accept exactly one and reject the other stale without assigning either signer ownership or authorship.
  - **T-1103-E6 — WHEN** a unit moves from lifecycle record 255 to the exact 256-record/revision capacity, **THEN** that otherwise-valid command **SHALL** succeed once; the next current-revision command must return `LifecycleHistoryCapacityExceeded` before remaining lifecycle validity. Separately, global sequence `u64::MAX-1` to `u64::MAX` shall succeed once and the next otherwise-domain-valid mutation shall return `GlobalSequenceExhausted`, always without overflow, sentinel, CubiKan mutation, or accepted domain event.
  - **T-1103-E7 — WHEN** mock Root replaces the complete allowlist with at most 16 unique accounts, **THEN** the pallet **SHALL** replace it and emit one non-domain administrative event; a signed origin, duplicate account, or maximum-plus-one list must reject without changing authorization or the global domain-event sequence.
- **Notes:** Every lifecycle, relationship, and provenance dispatch carries explicit `command_schema_version: u16 = 1`; event schema version is runtime-produced rather than caller-selected. The local runtime has no reachable Root path, so the root-only administrative call is exercised in mock tests only.

### T-1104: Port bounded relationship definitions and edges to the pallet
- **Intents:** [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md), [INT-0014](../../../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Touches:** `chain/pallets/cubikan/src/relationship.rs`, `chain/pallets/cubikan/src/lib.rs`, `chain/pallets/cubikan/src/event.rs`, `chain/pallets/cubikan/src/error.rs`, `chain/pallets/cubikan/src/benchmarking.rs`, `chain/pallets/cubikan/src/weights.rs`, `chain/pallets/cubikan/src/tests/relationships.rs`, `tests/fixtures/chain-conformance-v1.json`
- **Depends on:** T-1103
- **Acceptance criterion:** INT-0012 relationship acceptance semantics are canonical on chain within explicit finite graph bounds.
- **Success criterion (EARS):**
  - **T-1104-E1 — WHEN** an authorized submitter creates a new exact definition key or valid directed edge, **THEN** the pallet **SHALL** preserve immutable definition fields and endpoint units, increment global sequence once, and emit one complete definition/edge event without changing endpoint lifecycle revisions.
  - **T-1104-E2 — WHEN** a successfully decoded bounded definition-create call is evaluated, **THEN** the pallet **SHALL** check command schema version, direct signed/allowlisted origin, duplicate exact definition identity, and global-sequence capacity in that order, returning the first exact typed failure with no definition/global-sequence change or accepted event; malformed or over-bound definition fields reject structurally before dispatch under the bounded-codec clause in T-1102.
  - **T-1104-E3 — WHEN** a decoded bounded edge-create call is evaluated, **THEN** the pallet **SHALL** check command schema version, direct signed/allowlisted origin, selected definition, source, target, source species, target species, self policy, exact duplicate, 128-edge capacity, same-definition cycle policy, and global-sequence capacity in that order, returning the first exact typed failure without mutation/event and deciding every in-bound traversal with finite measured work.
  - **T-1104-E4 — WHEN** opposite authorized edge creations could jointly close a forbidden cycle, **THEN** canonical sequential execution **SHALL** accept at most one and leave no cycle regardless of extrinsic order.
  - **T-1104-E5 — WHEN** a decoded bounded exact-edge deletion is evaluated, **THEN** the pallet **SHALL** check command schema version, direct signed/allowlisted origin, selected definition, source, target, source species, target species, exact active edge, and global-sequence capacity in that order before one replay-complete delete event; any failure shall be eventless, and success shall preserve definitions/endpoints/near-neighbor edges and require a separate create for correction.
- **Notes:** Cycle traversal is exact-version scoped and bounded at 128 edges. No on-chain pagination or stored board exists; queries derive from finalized projection.

### T-1105: Implement canonical provenance record and revocation
- **Intents:** [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md), [INT-0014](../../../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Touches:** `chain/pallets/cubikan/src/provenance.rs`, `chain/pallets/cubikan/src/lib.rs`, `chain/pallets/cubikan/src/event.rs`, `chain/pallets/cubikan/src/error.rs`, `chain/pallets/cubikan/src/benchmarking.rs`, `chain/pallets/cubikan/src/weights.rs`, `chain/pallets/cubikan/src/tests/provenance.rs`, `tests/fixtures/chain-conformance-v1.json`
- **Depends on:** T-1104
- **Acceptance criterion:** Exact many-to-many whole-unit/revision associations and append-only revocations are canonical without altering lifecycle revision or implying attribution.
- **Success criterion (EARS):**
  - **T-1105-E1 — WHEN** an authorized submitter records a valid whole-unit association or any exact revision from `0..=current`, including an interior historical revision, **THEN** the pallet **SHALL** preserve their distinct complete identities, support many-to-many active links, increment global sequence once, emit one replay-complete event, and leave lifecycle revision/history unchanged.
  - **T-1105-E2 — WHEN** a decoded bounded association-record call is evaluated, **THEN** the pallet **SHALL** check command schema version, direct signed/allowlisted origin, selected unit and whole/exact-revision subject, reference validity, exact-active duplicate, 128-active-association capacity, and global-sequence capacity in that order, returning the first exact typed failure with no lifecycle/provenance/global-sequence change or accepted event.
  - **T-1105-E3 — WHEN** a decoded bounded association-revoke call is evaluated, **THEN** the pallet **SHALL** check command schema version, direct signed/allowlisted origin, selected unit and whole/exact-revision subject, reference validity, exact active association, and global-sequence capacity in that order before removing only active membership and emitting one revocation; missing/repeated revocation rejects and correction requires a later independent record.
  - **T-1105-E4 — WHEN** runtime/event/metadata surfaces are inventoried, **THEN** they **SHALL** contain only bounded reference/lifecycle/relationship data and technical signer metadata, with no ownership, authorship, credentials, prompts, transcripts, source bodies, or provider secrets.

### T-1106: Integrate the fixed local parachain runtime and artifact contract
- **Intent:** [INT-0014](../../../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Touches:** `chain/runtime/**`, `chain/config/**`, `chain/metadata/**`, `chain/artifacts/**`, `chain/pins.toml`, `chain/tools/**`, `chain/tests/runtime.rs`, `chain/tests/weights.rs`
- **Depends on:** T-1104, T-1105
- **Acceptance criterion:** One fixed Wasm runtime, chain spec, deployment anchor, authorization/fee policy, metadata, and finite weights are internally consistent and ready for two collators.
- **Success criterion (EARS):**
  - **T-1106-E1 — WHEN** the runtime and local chain specification build from the locked sources, **THEN** CubiKan deployment-anchor/pallet genesis fields **SHALL** encode only non-self-referential ParaId 1000, one 32-byte deployment ID, and pallet/event/storage versions, while the exact pinned standard System/parachain/session/collator/Aura/balances genesis includes two funded technical submitters distinct from node roles, transaction payment, fixed runtime spec/code identity, and no exercised upgrade or signed allowlist mutation path.
  - **T-1106-E2 — WHEN** runtime Wasm, metadata, chain spec, artifact manifest, and native runtime are verified, **THEN** Wasm bytes **SHALL** match their manifest checksum while heterogeneous artifacts must agree through their canonical hashes/decoded metadata, genesis/deployment semantics, runtime version, and embedded code identity; no plan shall compare unlike artifact bytes, and any semantic/hash mismatch shall fail before node launch.
  - **T-1106-E3 — WHEN** pallet benchmarks/weight generation run at every declared maximum, **THEN** each dispatch **SHALL** have finite generated weight covering worst-case reads/writes/traversal and no zero placeholder, unbounded collection, or unmeasured graph path.
  - **T-1106-E4 — WHEN** the local runtime is inspected for data and authority, **THEN** it **SHALL** expose ordinary dev fees and bounded technical authorization only, contain no production key/material or private fixture, and remain fixed for the complete Sprint 11 journey.
  - **T-1106-E5 — WHEN** the runtime call/origin inventory is inspected, **THEN** it **SHALL** contain direct signed CubiKan domain calls and the unreachable root-only allowlist call, with no sudo, proxy, utility, multisig, governance, batch, derivative-account, or other wrapper capable of transforming a signed origin or reaching Root.
  - **T-1106-E6 — WHEN** nodes produce genesis, **THEN** committed `chain/artifacts/local-deployment-anchor-v1.json` **SHALL** trace relay genesis to relay RPC, parachain block-0 hash to strict parachain RPC, ParaId/deployment/pallet/event versions to decoded on-chain state, and runtime code hash to `blake2_256(:code)`, match its `chain/pins.toml` SHA-256, reject every provenance/value mismatch in every consumer, and never store a self-hash in genesis.
  - **T-1106-E7 — WHEN** Runtime Executive classifies a rejected CubiKan transaction, **THEN** it **SHALL** distinguish malformed SCALE or transaction-validity rejection before inclusion, which consumes no nonce or fee, from a well-formed included extrinsic returning a typed CubiKan dispatch error, which incurs ordinary nonce/fee/System failure effects while `pallet-cubikan` domain storage/global sequence remain byte-equal and no accepted domain event is emitted.
- **Notes:** Generated metadata is committed from the exact runtime and consumed offline by Subxt codegen. The genesis-fixed local allowlist is not mutated during the journey.

### T-1107: Rebaseline the chain-neutral core around required origin
- **Intents:** [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md), [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md), [INT-0014](../../../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Touches:** `crates/cubikan-core/src/lib.rs`, `crates/cubikan-core/src/intent_unit.rs`, `crates/cubikan-core/src/vocabulary.rs`, `crates/cubikan-core/src/workflow.rs`, `crates/cubikan-core/tests/**`, `crates/cubikan-backend/src/model.rs`, `crates/cubikan-backend/src/stored.rs`, `crates/cubikan-backend/src/sqlite.rs`, `crates/cubikan-backend/tests/**`, `crates/cubikan-cli/src/**`, `crates/cubikan-cli/tests/**`, `crates/cubikan-local/src/**`, `crates/cubikan-local/tests/**`
- **Depends on:** T-1102, T-1106
- **Acceptance criterion:** Every current root-domain aggregate requires and preserves one origin while every root consumer remains green and legacy JSON paths become unsupported-only bridges.
- **Success criterion (EARS):**
  - **T-1107-E1 — WHEN** core construction, serialization, or validated restoration occurs, **THEN** exactly one valid immutable origin **SHALL** be required and preserved through every lifecycle operation with no originless/null/legacy constructor, placeholder, or in-place correction.
  - **T-1107-E2 — WHEN** every root consumer and test compiles after the core rebaseline, **THEN** temporary backend/CLI/local legacy generation entry points **SHALL** return typed unsupported before removed canonical writes, RPC, database mutation, or synthetic attribution, and the complete root workspace must remain green without a transitional current write authority.
  - **T-1107-E3 — WHEN** lifecycle history reaches the exact 256-record current-generation capacity, **THEN** core and chain conformance fixtures **SHALL** preserve revision/history equality and reject record 257 with the same typed capacity result and no CubiKan history/state change.
- **Notes:** This is the only task that changes the aggregate constructor. It does not create schema v3 or a transitional current write authority.

### T-1108: Build the exact hardened SQLite v3/envelope v2 projection store
- **Intents:** [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md), [INT-0014](../../../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Touches:** `Cargo.toml`, `Cargo.lock`, `crates/cubikan-backend/Cargo.toml`, `crates/cubikan-backend/src/lib.rs`, `crates/cubikan-backend/src/error.rs`, `crates/cubikan-backend/src/schema.rs`, `crates/cubikan-backend/src/schema/tests.rs`, `crates/cubikan-backend/src/sqlite.rs`, `crates/cubikan-backend/src/stored.rs`, `crates/cubikan-backend/src/projection_store.rs`, `crates/cubikan-backend/src/projection_store/tests.rs`, `crates/cubikan-backend/tests/security.rs`, `tests/fixtures/filesystem-boundary-v1.json`, `tests/fixtures/sqlite-authorizer-v1.json`, `tests/fixtures/envelope-v2/**`
- **Depends on:** T-1107
- **Acceptance criterion:** Schema v3/envelope v2 form an exact, crate-private-write projection store with verified SQLite/file defenses and no legacy migration.
- **Success criterion (EARS):**
  - **T-1108-E1 — WHEN** a fresh Linux projection is created on a classifier-approved test-owned local filesystem, **THEN** safe OS `O_CREAT|O_EXCL|O_NOFOLLOW|O_CLOEXEC` **SHALL** create/validate exactly one mode-`0600` canonical-absolute regular direct child before SQLite opens that existing inode as built-in-`unix` `READ_WRITE|NOFOLLOW` without SQLite CREATE/EXCLUSIVE; it shall commit only the exact UTF-8 schema-v3 objects/settings with no anchor, block, checkpoint, unit, edge, association, or read capability, while any preexisting target/race, non-Linux, or missing/malformed/mismatched/unapproved mount, `statfs`, or VFS identity rejects without adoption, and the backend classifier shall pass every independently authored shared-corpus case.
  - **T-1108-E2 — WHEN** an existing file opens, **THEN** its already-validated read-only descriptor **SHALL** prove exact rollback-mode/4096-page/UTF-8/aligned SQLite header bytes and absent sidecars before SQLite access; OS/SQLite `READ_ONLY|NOFOLLOW` preflight shall then apply the numeric limits, safe load-extension/comment disable, exact reader-role authorizer, query-only/cell-size/mmap/temp-store/busy settings and read them back where exposed before schema SQL, require file-size/page-count/full-integrity/foreign-key/exact-schema equality without write PRAGMA/recovery, and permit a writer reopen only with the distinct query-only-off projector tuple allowlist plus common defenses, max-page setting, lock, and revalidation.
  - **T-1108-E3 — WHEN** owner/mode/type/sidecar/stable-parent checks fail, the absolute direct-child invariant fails, a short/misaligned/WAL/unknown header or unexpected journal/WAL/SHM exists, or SQLite version/features/compile options/runtime library/role settings differ, **THEN** open/create **SHALL** fail before unsafe SQLite access without following, sidecar creation, repairing, adopting, recovering, or creating outside the test-owned root; a `file:`-shaped basename shall remain one literal child despite accepted `SQLITE_USE_URI` because neither `SQLITE_OPEN_URI` nor `ATTACH` is available.
  - **T-1108-E4 — WHEN** envelope v2 or any selected derived row/coordinate/checkpoint is decoded, **THEN** exact replay and all projections **SHALL** agree, and the computed plus adversarial maximum 256-record escaped encoding must fit the 2,097,152-byte ceiling; schema v1/v2, envelope v1, mixed versions, edits, replay disagreement, and any over-ceiling encoding shall fail without migration, synthetic origin, or repair.
  - **T-1108-E5 — WHEN** quote/comment/semicolon/PRAGMA/ATTACH-shaped free-text scope/value is passed to crate-private fixture writers, **THEN** it **SHALL** round-trip byte-exactly only through binds without SQL/schema effect; invalid constrained namespace/species text shall reject before SQL, all production SQL text shall be private/static/comment-free, `load_extension` shall be denied, and no caller SQL or raw row/block/event/checkpoint/capability input shall exist.
  - **T-1108-E6 — WHEN** a writer connection is closed/reopened or growth crosses the exact page budget, **THEN** it **SHALL** reapply/read back `max_page_count=262144`, accept the final in-budget transaction, return typed `SQLITE_FULL` at growth beyond the limit with atomic rollback, and use only SQLite's `busy_timeout=5000` handler with no application retry.
- **Notes:** All writer tests are private module tests because production raw writes remain crate-private. File checks do not claim protection against hard links, hostile parent replacement, custom VFS, continuous TOCTOU, lying hardware/power-loss beyond acknowledged Linux `fsync`, or a fully coherent forgery.

### T-1109: Implement capability-gated v3 queries
- **Intents:** [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md), [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md), [INT-0014](../../../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Touches:** `crates/cubikan-backend/src/lib.rs`, `crates/cubikan-backend/src/model.rs`, `crates/cubikan-backend/src/relationship.rs`, `crates/cubikan-backend/src/query.rs`, `crates/cubikan-backend/src/provenance.rs`, `crates/cubikan-backend/src/verified_read.rs`, `crates/cubikan-backend/tests/read_boundary.rs`
- **Depends on:** T-1108
- **Acceptance criterion:** Unit, direct-relationship, projection-v1, and bidirectional provenance reads retain exact bounded semantics and are uncallable without one attested read snapshot.
- **Success criterion (EARS):**
  - **T-1109-E1 — WHEN** no `VerifiedReadSnapshot` is supplied, **THEN** public APIs **SHALL** expose no unit, relationship, projection, or provenance row; a structurally coherent database alone shall never authenticate state.
  - **T-1109-E2 — WHEN** the private `cfg(test)` issuer supplies one pinned snapshot for a unit/direct-relationship/projection-v1/provenance read, **THEN** the query **SHALL** return canonical complete-key order, limits 1–100, exclusive structurally validated cursors, validated lookahead, joined block hash and checkpoint `C`, and whole-read failure on selected corruption without exposing that issuer in production.
  - **T-1109-E3 — WHEN** checkpoint advances before pinning or a writer attempts C+1 during a pinned DELETE-mode read, **THEN** pre-pin mismatch **SHALL** return `refresh_required`, while a successfully pinned C page shall finish entirely at C and invoke only the configured `busy_timeout=5000` handler before typed Busy; a separate 7,500-ms outer test timeout bounds scheduling overshoot, and after snapshot drop a newly issued capability shall expose C+1 membership.
  - **T-1109-E4 — WHEN** one unit participates in multiple exact relationship versions and many-to-many whole/revision provenance links, **THEN** queries **SHALL** preserve INT-0012 filters/projection-v1 semantics and INT-0008 forward/reverse identity without copied lifecycle state.
- **Notes:** Positive E2–E4 query tests live in internal `#[cfg(test)]` modules in
  `query.rs`, `relationship.rs`, and `provenance.rs`, where the private issuer is
  reachable. `tests/read_boundary.rs` contains only public-signature/no-raw-open
  negatives because Cargo integration tests compile the library without
  `cfg(test)`. Construction failures live as built-in rustdoc `compile_fail`
  snippets beside `VerifiedReadSnapshot` and compile in the locked workspace
  doctest gate—no third-party harness or manifest change. T-1110 alone owns
  production capability minting.

### T-1110: Project and attest the complete finalized archive-RPC event stream
- **Intents:** [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md), [INT-0014](../../../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Touches:** `Cargo.toml`, `Cargo.lock`, `crates/cubikan-chain-client/Cargo.toml`, `crates/cubikan-chain-client/src/lib.rs`, `crates/cubikan-chain-client/src/identity.rs`, `crates/cubikan-chain-client/src/rpc.rs`, `crates/cubikan-backend/Cargo.toml`, `crates/cubikan-backend/src/lib.rs`, `crates/cubikan-backend/src/projector.rs`, `crates/cubikan-backend/src/attestation.rs`, `crates/cubikan-backend/src/projector/tests.rs`, `crates/cubikan-backend/src/attestation/tests.rs`, `tests/fixtures/finalized-events-v1/**`
- **Depends on:** T-1106, T-1108, T-1109
- **Acceptance criterion:** Strict chain-client RPC primitives feed backend-owned atomic projection and full-stream attestation, with no public raw-write or capability-minting seam.
- **Success criterion (EARS):**
  - **T-1110-E1 — WHEN** a client connects, **THEN** literal loopback-`ws` URL parsing, fixed manifest SHA/provenance, metadata/runtime/code identity, exact node flags `--blocks-pruning=archive --state-pruning=archive`, genesis/early/mid/current historical block-body and event probes, and parent continuity **SHALL** pass before decode/apply/attestation; failure shall claim neither perpetual archive retention nor independent finality.
  - **T-1110-E2 — WHEN** backend sync first observes verified finalized block zero, **THEN** one transaction **SHALL** atomically insert the manifest-derived anchor, the zero-event block-zero row with null per-block sequences, and the checkpoint with `last_global_sequence=null`; a later zero-event block shall keep its row sequences null while carrying the checkpoint's prior nonzero sequence.
  - **T-1110-E3 — WHEN** later contiguous finalized blocks arrive, **THEN** backend-owned code **SHALL** decode every CubiKan event in block/extrinsic/system-event order, enforce each payload at most `1_048_576` bytes, obtain event block hash only through the restrictive block-number join, and atomically commit one complete block, derived rows, and checkpoint exactly once; statement/commit/space/limit failure shall roll back the block while chain state remains unaffected.
  - **T-1110-E4 — WHEN** input is best-only, displaced, duplicate-conflicting, skipped, out-of-order, malformed, wrong-parent/anchor/runtime/version/count/sequence, overbound, or replay-invalid, **THEN** backend sync **SHALL** expose no partial state/capability/checkpoint and accept identical replay as a no-op only after complete row/envelope equality.
  - **T-1110-E5 — WHEN** attestation for checkpoint `C` is requested, **THEN** backend code **SHALL** fetch and replay the full finalized block/event stream from the deployment anchor through C outside SQLite, start and pin one exact read transaction at C, compare every schema-v3 block/event/derived row/envelope/checkpoint including joined block hashes, and only then mint one nonserializable single-read `VerifiedReadSnapshot` bound to that file identity and transaction.
  - **T-1110-E6 — WHEN** an archive probe fails, checkpoint advances before pinning, projection restarts, or two projectors contend, **THEN** work **SHALL** return bounded source-preserving archive/refresh/Busy errors or serialize, never expose caller-made blocks/events/rows/checkpoints/capabilities, and never claim independent GRANDPA proof, double application, heuristic rollback, or perpetual history.
- **Notes:** This trusts the configured pinned local archive node's finalized-RPC assertions; it does not verify GRANDPA proofs or economic shared security.

### T-1111: Submit finalized Subxt mutations through a crash-recoverable signer lane
- **Intent:** [INT-0014](../../../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Touches:** `Cargo.toml`, `Cargo.lock`, `crates/cubikan-chain-client/Cargo.toml`, `crates/cubikan-chain-client/src/lib.rs`, `crates/cubikan-chain-client/src/submission.rs`, `crates/cubikan-chain-client/src/submission_journal.rs`, `crates/cubikan-chain-client/tests/submission.rs`, `crates/cubikan-chain-client/tests/submission_journal.rs`, `tests/fixtures/submission-journal-v1/**`
- **Depends on:** T-1110
- **Acceptance criterion:** Mutation submission has one owner-only cross-process lane per signer, honest finalized/indeterminate outcomes, and no blind retry.
- **Success criterion (EARS):**
  - **T-1111-E1 — WHEN** a mutation process opens its lane, **THEN** it **SHALL** on supported Linux derive journal, persistent lock, and one fixed `.tmp` direct-child name from the canonical projection path, deployment ID, and signer, independently satisfy every shared filesystem-classifier corpus case before validating the stable local-filesystem/owner/mode/type boundary, acquire the mode-`0600` `O_NOFOLLOW` lock inode that is never replaced/unlinked, treat absent journal as clean first use, remove only an owner-mode-`0600` regular derived temp under lock followed by parent `fsync`, and fail closed on symbolic/nonregular/wrong-owner/wrong-mode/oversized temp or any caller-selected path, unsupported platform/filesystem, or corrupt state before signing or sending.
  - **T-1111-E2 — WHEN** no unresolved record exists and canonical RPC supplies deployment plus a chosen finalized signing block number/hash, **THEN** the lane **SHALL** decode the signer nonce from `System::Account` at that exact block hash (never next-index/best/pool state), use `ClientAtBlock::transactions().create_signable_offline` with explicit nonce and Subxt's exact equivalent of `mortal_from_unchecked(64, signing_finalized_number, signing_finalized_hash)`, compare `SignableTransaction::signer_payload()` byte-for-byte with an independently constructed expected payload from pinned metadata/call/params/finalized hash, sign that payload, verify its signature, decode the signed extrinsic to prove encoded signer/call/nonce/period/phase/zero-tip and journal birth=`n`/death=`n+63` plus original mutation-operation tag, then publish the exact 256-byte `submission-journal-v1` record with locked magic/version/length/order/width/endian/state/reserved/coordinate rules and domain-separated SHA-256 before send via the derived same-directory `O_EXCL|O_NOFOLLOW` temp, complete write, temp `fsync`, atomic rename, and parent-directory `fsync`, and hold the lock through a known final outcome. The additional signed block hash is proven by expected-payload equality and signature verification, never claimed to be encoded in the extrinsic.
  - **T-1111-E3 — WHEN** kill/fault injection strikes after temp creation or before/after temp write, file `fsync`, rename, directory `fsync`, send, or durable resolution, **THEN** restart **SHALL** observe exactly the old or new complete checksummed journal, safely discard at most the one derived torn/complete temp under lock, make zero unsafe new sends, prevent orphan accumulation, and preserve process-crash durability under acknowledged Linux local-filesystem semantics without claiming lying-hardware power-loss durability.
  - **T-1111-E4 — WHEN** submission finalizes within 120 seconds, **THEN** `finalized_accepted` **SHALL** require the exact prepared extrinsic hash/index to dispatch successfully and contain exactly one matching accepted event inside that extrinsic by deployment/version/signer/call identity; included dispatch failure shall be `finalized_dispatch_rejected`, successful inclusion with zero/multiple/wrong accepted events shall durably resolve as `finalized_invariant_failed`, and safely resolved publication/removal shall `fsync` its parent without retry.
  - **T-1111-E5 — WHEN** `submit_and_watch` has begun and timeout, crash, transport/watcher/RPC/response loss, watcher `Invalid`/`Dropped`/`Error`, stream end, unknown status, or ambiguous nonce state leaves a prepared or resolved record, **THEN** the call **SHALL** use its persisted original operation and submit nothing until the exact hash is found/reconstructed from finalized chain evidence or finalized head is past death and every finalized block from inclusive birth through death has been scanned for that hash before publishing `expired_not_included`; unresolved evidence shall return `submission_lane_unresolved`, possible delivery shall return `delivery_indeterminate`, a recovered terminal record shall reproduce the identical persisted operation/outcome and chain-derived coordinate/effect/error while freshly attesting a projection that may legitimately advance, and expiry, watcher terminal status, incoming-request operation, or SQLite state shall never justify clearing, signing, nonce selection, or automatic retry. A crash after response but before removal may duplicate that semantic response, never a submission.
  - **T-1111-E6 — WHEN** two cooperating processes target one signer or separate signers, **THEN** same-signer work **SHALL** serialize while separate lanes may proceed, nonce disagreement from external software shall fail closed, and same-user deletion of an unresolved record shall remain explicitly undetectable/out-of-bound rather than an exactly-once claim.

### T-1112: Replace `cubikan` with strict stateless protocol v2
- **Intents:** [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md), [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md)
- **Touches:** `crates/cubikan-cli/**`, `protocol/v2/cubikan.schema.json`, `protocol/v2/verify-fixtures.sh`, `tests/fixtures/protocol-v2/cubikan/**`
- **Depends on:** T-1107
- **Acceptance criterion:** `cubikan` v2 is a strict one-shot in-memory validator/simulator, never a canonical runtime or durable state authority.
- **Success criterion (EARS):**
  - **T-1112-E1 — WHEN** its one locked v2 lifecycle-simulation request decodes, **THEN** every nesting level **SHALL** enforce the exact schema/scalar/operation inventory, require origin, generate an omitted ID client-side, reject explicit null, and return only `authority:"simulation_only"` UnitView results with no canonical vocabulary.
  - **T-1112-E2 — WHEN** v1/old shape, malformed input, below/at/over 1 MiB input, or body/newline/flush I/O failure occurs, **THEN** the adapter **SHALL** preserve bounded ingestion, one-response, checked flush, source-retaining I/O, stderr, and exit semantics without RPC/database/session/canonical-success behavior.
  - **T-1112-E3 — WHEN** the independently authored `cubikan` schema and raw fixture manifest are verified before decoder implementation and replayed afterward, **THEN** exact bytes/hashes **SHALL** cover every stateless operation/result/code/boundary/duplicate-key case and implementation output shall never generate or update its oracle.

### T-1113: Replace `cubikan-local` with strict chain-backed protocol v2
- **Intents:** [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md), [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md), [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md), [INT-0014](../../../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Touches:** `crates/cubikan-local/**`, `Cargo.toml`, `Cargo.lock`, `protocol/v2/cubikan-local.schema.json`, `protocol/v2/verify-fixtures.sh`, `tests/fixtures/protocol-v2/cubikan-local/**`
- **Depends on:** T-1109, T-1110, T-1111, T-1112
- **Acceptance criterion:** `cubikan-local` v2 owns the full local chain/projection operation inventory and reports submission, finality, attestation, and lag without bypasses.
- **Success criterion (EARS):**
  - **T-1113-E1 — WHEN** a v2 request decodes, **THEN** the exact locked top-level/operation/scalar/cursor schema **SHALL** reject duplicates, unknowns, wrong shapes, null optionals, bad origin/revision/coordinate, and v1 before signing, RPC dial, SQLite open, or capability issuance.
  - **T-1113-E2 — WHEN** operation inventory is inspected, **THEN** it **SHALL** expose exactly the fifteen named lifecycle/definition/edge/projection-v1/provenance operations and fields in the locked contract, supply non-caller-selectable command schema version 1, expose no mutation path except T-1111 submission, and expose no raw SQLite/RPC/capability seam.
  - **T-1113-E3 — WHEN** mutation outcomes occur, **THEN** exact JSON unions **SHALL** distinguish all seven outcomes—`submission_rejected`, `submission_lane_unresolved`, `expired_not_included`, `finalized_dispatch_rejected`, `finalized_invariant_failed`, `delivery_indeterminate`, and `finalized_accepted`—using the persisted original operation for reconciliation, one joined-hash ledger coordinate where finalized acceptance requires it, and caught-up/lagging projection, without false success, rollback, SQLite-derived preflight, or retry.
  - **T-1113-E4 — WHEN** process arguments are parsed, **THEN** mutations **SHALL** require `--database`, strict `--rpc`, and named `--dev-signer`, derive rather than accept a journal path, while reads omit signer/journal use and serialized surfaces contain no secret/owner/author/source payload.
  - **T-1113-E5 — WHEN** below/at/over 1 MiB input or response/newline/flush failure occurs, **THEN** the realized bounded one-response/process-exit contract **SHALL** remain exact while the raw ingress buffer/read counter retains at most `1_048_577` bytes, without claiming that total process memory fits that buffer ceiling.
  - **T-1113-E6 — WHEN** the independently authored `cubikan-local` schema and raw fixture manifest are verified before decoder implementation and replayed afterward, **THEN** exact bytes/hashes **SHALL** cover all fifteen operations, result/outcome/error registries, optional/cursor/coordinate codecs, raw duplicate keys, and size boundaries without using implementation output as the oracle.

### T-1114: Add the local Git reference adapter demonstration
- **Intent:** [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md)
- **Touches:** `Cargo.toml`, `Cargo.lock`, `crates/cubikan-git/**`, `tests/fixtures/git/**`
- **Depends on:** T-1113
- **Acceptance criterion:** A full Git commit identity can be submitted without importing provider authority or attribution.
- **Success criterion (EARS):**
  - **T-1114-E1 — WHEN** installed Git `>=2.45.0` resolves a caller repository/revision, **THEN** it **SHALL** use argv-only execution with one canonical validated `-C` repository; clear Git directory/worktree/object/alternate/config/replace/proxy/credential environment; reject a nonempty common-dir or worktree `objects/info/alternates`; set literal `GIT_TERMINAL_PROMPT=0`, `GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_NO_REPLACE_OBJECTS=1`, `GIT_NO_LAZY_FETCH=1`, and `GIT_OPTIONAL_LOCKS=0`; capability-check the global `--no-lazy-fetch` option and `rev-parse --show-object-format`; and run `git --no-lazy-fetch -C <repository> rev-parse --verify --end-of-options <revision>^{commit}` with bounded stdout/stderr before building exact `(git.commit.sha1|git.commit.sha256, scope, full OID)` without lazy fetch, promisor/alternate objects, credentials, shell, or normalization; an older or incompatible Git shall fail typed before repository mutation or submission.
  - **T-1114-E2 — WHEN** local Git lacks SHA-256 capability, **THEN** the live SHA-256 demo **SHALL** skip explicitly while an independent checked SHA-256 repository fixture proves format/length/namespace validation; SHA-1 remains a real temporary-repository journey.
  - **T-1114-E3 — WHEN** source moves and blame changes after recording, **THEN** finalized association and rebuilt query **SHALL** remain byte-identical and never promote blame/committer/signer into authorship, causality, verification, or satisfaction.
  - **T-1114-E4 — WHEN** input is abbreviated, NUL/leading-option-shaped, noncommit, wrong format, outside the repository, unsupported, or evaluated under hostile inherited Git config/directory/in-process-or-on-disk-alternate/replace/proxy/credential/promisor state or fake remote/credential helpers, **THEN** it **SHALL** return typed noncanonical failure without helper/network invocation, submission, or projection mutation.

### T-1115: Prove the four-node local failover, resynchronization, and rebuild journey
- **Intents:** [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md), [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md), [INT-0014](../../../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Touches:** `chain/config/zombienet.toml`, `chain/tools/**`, `tests/chain-e2e/**`, `crates/cubikan-local/tests/chain_e2e.rs`
- **Depends on:** T-1113, T-1114
- **Acceptance criterion:** One pinned relay runtime across relay validators/collator relay sides and one distinct byte-identical CubiKan runtime across both collators survive one-collator loss, resynchronization, and equal projection rebuilds without public action.
- **Success criterion (EARS):**
  - **T-1115-E1 — WHEN** the bounded harness launches, **THEN** exactly two relay-validator and two collator node processes **SHALL** use test-facing RPC `127.0.0.1:9944`, `:9945`, `:9988`, and `:9989`, primary P2P `:30333` through `:30336`, primary metrics `:9615` through `:9618`, collator relay-side RPC `:9990` and `:9991`, relay-side P2P `:30337` and `:30338`, and relay-side metrics `:9619` and `:9620`; the pin-verified argv-normalizing launcher shall reject unexpected generated grammar/flags, strip every external-bind flag, force all listeners and bootnodes to loopback, preserve both collators' archive flags, verify byte-identical CubiKan Wasm across collators and byte-identical relay runtime/spec across relay validators and collator relay sides while keeping those two runtime identities distinct, and `/proc/<pid>/cmdline` plus `ss -lntup` shall show exactly those PID-owned node listeners with no wildcard/public bind, while the separate Zombienet orchestrator, local data, manifests, timeout, and cleanup remain test-owned.
  - **T-1115-E2 — WHEN** both dev submitters execute required-origin lifecycle, relationship, provenance, and Git work, **THEN** each mutation **SHALL** finalize once and both endpoints plus freshly attested SQLite reads shall converge.
  - **T-1115-E3 — WHEN** one collator stops after checkpoint `C`, **THEN** the survivor **SHALL** finalize remaining work; the stopped collator shall restart with its original archive data/config, sync to the named final checkpoint, pass historical range probes and full identity checks, and only then become a rebuild source.
  - **T-1115-E4 — WHEN** disposable projection files rebuild independently through the synchronized collators, **THEN** full-stream-attested units/history/origin/definitions/edges/projections/provenance/pages/checkpoint **SHALL** equal the uninterrupted projection.
  - **T-1115-E5 — WHEN** sockets/config/actions/logs/journals are audited, **THEN** they **SHALL** show loopback/synthetic/dev-only work, no allowlist mutation, and no public RPC/account/key import/faucet/transfer/ParaId/coretime/upload/deploy/release/governance/secret action.
- **Notes:** Test must record one actual exact-candidate run; a skipped hosted job is not evidence and archive configuration is not a perpetual-availability promise.

### T-1116: Reconcile security, authority, and current-state documentation
- **Intents:** [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md), [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md), [INT-0014](../../../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Touches:** `README.md`, `crates/*/README.md`, `chain/README.md`, `docs/appendix/potential-derivative-projects.md`, `docs/SUMMARY.md`
- **Depends on:** T-1115
- **Acceptance criterion:** Users see one precise authority/security model, local proof, historical boundary, and no public-security overclaim while terminal intent documents stay locked.
- **Success criterion (EARS):**
  - **T-1116-E1 — WHEN** docs are read across all surfaces, **THEN** they **SHALL** agree on Book/pallet/provider authority, node-trusted full-stream attestation, SQLite projection, signer non-attribution, journal noncanonicity, and local-only limits.
  - **T-1116-E2 — WHEN** security guidance is checked, **THEN** it **SHALL** distinguish SQL binds from hostile path/resource/TOCTOU/VFS risks, Linux process-crash durability from lying-hardware power loss, signer-journal deletion/noncanonicity, finality/lag/indeterminacy/fees/disclosure/rebuild/least privilege, and reject independent-finality, tamper-proof, audit, erasure, perpetual-history, generic-Unix-filesystem, or production-readiness claims.
  - **T-1116-E3 — WHEN** the candidate is compared with the approved Plan snapshot, **THEN** current-generation documentation **SHALL** introduce no supported migration, synthetic-origin, dual-write, current-v1, public action/secret handling, or live public-security claim; explicitly historical and negative occurrences remain allowed, while Build continuously requires SHA-256 `521bc0e01bcbc393a1b6c9fabb5b0d5c13cfd1f2e0f41166ce96bbc0860a4fa4` for terminal INT-0010, `639f374ebf7d62ed6fcf9e50224239aaa3e91bbe90eb0590b6d450dcc152e6ab` for terminal INT-0012, `c3841eb71b3f0c363369cd26ae21457f313769988a3563ac66dc69b901d07ca8` for terminal INT-0013, and `365116856e79d1bded60b12c68a5d8f2b4965d650af2c6feeebdc918148c15e0` for root `.github/workflows/ci.yml`.

### T-1117: Add portable offline gates, separate chain CI, and resource evidence
- **Intents:** [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md), [INT-0014](../../../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Touches:** `.github/workflows/chain.yml`, `scripts/check-book.sh`, `chain/tools/measure-resources.sh`, `docs/sprints/s11/sprint-tests/verify-links-and-scope.sh`
- **Depends on:** T-1116
- **Acceptance criterion:** Portable local and opt-in CI gates reproduce the exact candidate within locked time/disk limits without weakening root CI or contacting a public chain.
- **Success criterion (EARS):**
  - **T-1117-E1 — WHEN** the repo-local Book/link/scope validator runs in a minimal POSIX environment, **THEN** it **SHALL** use portable repository-owned tooling rather than assuming `rg`, enforce intent/task/link/nonclaim/terminal-byte guards, and report exact actionable failures.
  - **T-1117-E2 — WHEN** `.github/workflows/chain.yml` is inspected or manually dispatched, **THEN** it **SHALL** be a separate `workflow_dispatch` job, leave root `.github/workflows/ci.yml` byte-identical, verify pins before execution, perform one explicit dependency/artifact fetch, and then run root plus chain fmt/Clippy-with-warnings/tests/doctests/Wasm/benchmark-generated weights/protocol-fixture/offline Zombienet gates through the pinned loopback-only namespace with no public blockchain endpoint.
  - **T-1117-E3 — WHEN** exact cache configuration is audited, **THEN** it **SHALL** key separate immutable root Cargo, chain Cargo, Rust-toolchain/Wasm, Zombienet, and target caches by OS, lockfile/tool/version/pin hashes, restore no unverified binary or node database, and rerun checksum verification on every hit.
  - **T-1117-E4 — WHEN** one cold and one warm exact-candidate run are measured on the declared Linux runner, **THEN** cold completion **SHALL** be at most 90 minutes, warm completion at most 30 minutes, peak workspace/cache/node disk at most 60 GiB, and the T-1115 Zombienet journey at most 30 minutes, with timeout/cleanup and measurements retained as synthetic evidence rather than a skipped hosted-job claim.

## Dependency and Commit Discipline

- Complete tasks strictly in the dependency order above; T-1105 follows T-1104
  so the full relationship conformance corpus is stable before provenance work.
- Every task must leave all affected root/chain checks green and receive an
  independent read-only acceptance review before its ledger/helper commit.
- Downloaded binaries, node data, build caches, keys, and network logs remain
  ignored/test-owned. Only manifests, checksums, configs, synthetic fixtures,
  generated metadata/weights, and evidence explicitly named above are tracked.
- Build may request network access only during one explicit exact pinned
  dependency/artifact retrieval phase; all compilation, fixtures, tests, and
  journeys after that run offline and may not contact or mutate a public
  blockchain endpoint.
- The terminal intent documents and root `.github/workflows/ci.yml` are immutable
  Build inputs under the four exact terminal-byte SHA-256 values above. Every task preflight
  and the repository-local validator verifies them. T-1117 adds a separate opt-in
  chain workflow rather than editing or weakening the root workflow.
- A public deployment, runtime upgrade, allowlist-governance design, production
  signer custody, coretime/funding decision, or private-data policy is a new
  intent and cannot be inferred from successful local tests.
