# Submission Journal v1 Oracle Reasoning

This corpus is synthetic and implementation-independent. It was authored from
the locked Sprint 11 build/test contracts, the committed runtime metadata, and
the committed deployment anchor. Neither `submission.rs` nor
`submission_journal.rs` is an oracle input, and production code must never
regenerate these files.

## Journal encoding

Every raw journal vector decodes to exactly 256 bytes. The first 224 bytes are
the fixed-width body described by `journal-vectors-v1.json`; the last 32 bytes
are:

```text
SHA-256("CubiKan submission-journal-v1\0" || record[0..224])
```

All integers in the journal are big-endian. This deliberately differs from the
little-endian and compact integers in the SCALE transaction. The primary vector
uses nonce `66051` (`0x00010203`), signing/birth block `131` (`0x83`), inclusive
death `194` (`0xc2`), and operation tag `2` (`complete_unit`). Prepared has an
all-zero resolution coordinate. States 1–3 have a nonzero inclusion coordinate
within `131..=194`; state 4 stores the first observed finalized head after death,
block `195`.

## Signer lane derivation

For raw canonical Unix projection-directory bytes `P`, deployment `D`, and
signer `S`, the lane digest is:

```text
SHA-256("CubiKan signer lane v1\0" || u32_be(len(P)) || P || D || S)
```

Its lowercase hexadecimal spelling is substituted into exactly three direct
child basenames: `cubikan-submission-H.lock`,
`cubikan-submission-H.journal`, and `cubikan-submission-H.tmp`. The vectors vary
the last path byte, include a valid raw non-UTF-8 path, and swap the two 32-byte
identity fields to expose delimiter, length, and ordering mistakes.

## Signed transaction

The fixed signed transaction is an offline cryptographic/codec oracle. It uses
Alice's well-known synthetic sr25519 development key only because it provides a
reproducible public verification identity. Alice is not an authorized CubiKan
runtime submitter; production end-to-end submission must use the configured
Charlie or Dave development signer. No secret bytes are stored here.

Pinned metadata names pallet `Cubikan` at index `50` and call `complete_unit` at
index `2`. The call bytes are:

```text
32 02                       pallet and call
01 00                       command schema u16 little-endian
00 11 22 ... ee ff          16-byte IntentUnitId
08 07 06 05 04 03 02 01     expected revision u64 little-endian
```

Subxt v4's signer payload is the exact concatenation of call, mortal era,
compact nonce, compact zero tip, little-endian spec version, little-endian
transaction version, genesis hash, and the chosen finalized signing-block hash.
It is 107 bytes, so the v4 over-256-byte prehash rule does not apply. Period 64
and block 131 give phase 3 and SCALE era `35 00`; compact nonce 66051 is
`0e 08 04 00`; zero tip is `00`.

The signed extrinsic's compact body length is 134 and its signed v4 byte is
`0x84`. It contains `MultiAddress::Id` variant 0, Alice's AccountId32,
`MultiSignature::Sr25519` variant 1, the fixed valid signature, era, nonce, tip,
and call. Its BLAKE2b-256 hash is the journal's expected extrinsic hash. The
additional finalized signing-block hash is intentionally absent from the
extrinsic and is proven only by exact signer-payload equality plus sr25519
signature verification.

The Python verifier independently decodes SCALE compact integers and era,
reconstructs the payload, implements Merlin/STROBE-128, Ristretto255 group
decoding/arithmetic, and verifies the sr25519 equation under context
`substrate`. It also proves that a one-bit payload or signature mutation fails.

## Crash and reconciliation boundary

The crash registry admits only absence, the old complete journal, or the new
complete checksummed journal. A crash may leave at most the one fixed derived
owner-mode-0600 regular temp, which restart removes under the persistent lock
and follows with parent-directory `fsync`. A prepared record is durable before
send. Resolution uses the same publication sequence. There is no resend while
delivery may have occurred.

`expired_not_included` requires a finalized head strictly after block 194 and a
complete inclusive 131-through-194 scan proving the exact hash absent. Watcher
status, era expiry, nonce movement, incoming operation, and SQLite state never
clear the lane. Same-user deletion, external use of the signer, alternate
projection paths, and lying-hardware power loss are explicit nonclaims.
