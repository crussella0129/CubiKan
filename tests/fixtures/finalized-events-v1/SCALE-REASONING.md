# Finalized events v1 SCALE oracle

This fixture is an independently authored test oracle. Its bytes were fixed
from the locked runtime/event contract before the root production RPC decoder
or projector existed. Production code must never generate, refresh, or rewrite
these files.

Every `*.scale.hex` file contains exactly one lowercase hexadecimal line and a
final LF. The decoded bytes, rather than the hexadecimal file representation,
are the canonical SCALE input. `inventory-v1.json` pins both identities.

## Primitive encodings

- Unsigned integers use fixed-width little endian unless SCALE compact encoding
  is explicitly required.
- A bounded byte vector has the same SCALE encoding as a vector: compact byte
  length followed by exact bytes. All selected text is ASCII and receives no
  normalization.
- `Option::None` is `00`; `Option::Some(value)` is `01 || value`.
- Enum discriminants are the explicit single-byte indices in the pallet types.
- UUID values are their exact 16 RFC-4122 bytes, without a string encoding.
- `AccountId32`, deployment IDs, and hashes are exact 32-byte values.

## Domain payload indices

| Index | Payload |
| --- | --- |
| `00` | `UnitCreated` |
| `01` | `UnitTransitioned` |
| `02` | `UnitCompleted` |
| `03` | `RelationshipDefinitionCreated` |
| `04` | `RelationshipCreated` |
| `05` | `RelationshipDeleted` |
| `06` | `AssociationRecorded` |
| `07` | `AssociationRevoked` |

The two create payloads encode, in order, command-schema version `1`, UUID,
origin, species, and the complete immutable workflow. The workflow is
`lifecycle-v1`, phases `[queued, doing]`, initial phase `queued`, edge
`queued -> doing`, and completion phases `[doing]`.

The relationship definition key is `(depends_on, 7)`. Direction is `Directed`
index `00`; both species options contain `task`; self and cycle policies are
`Reject` index `01`. Relationship keys then append source UUID A and target
UUID B.

Association subject index `01` carries revision zero for unit A. Subject index
`00` is the whole-unit subject for unit B. Both use the exact reference
`(git.commit.sha256, public-synthetic/repository,
0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef)`.

## System Events encoding

Each events file is a SCALE vector of complete `EventRecord` values. A selected
accepted record is:

1. `Phase::ApplyExtrinsic` index `00` plus a little-endian `u32` index;
2. runtime pallet index `50` (`32` hex);
3. CubiKan event index `0` (`Accepted`);
4. deployment ID, event-schema `u16`, global-sequence `u64`, signer
   `AccountId32`, and the exact domain-payload bytes;
5. an empty topics vector (`00`).

The ignored interleaving record is `System::Remarked`: runtime pallet index
`0`, event index `5`, followed by one 32-byte sender, one 32-byte hash, and an
empty topics vector. Block 5 contains only this System event, proving that a
zero-CubiKan-event block may still have nonempty `System::Events` bytes.

Body fixtures contain independently selected opaque extrinsic bytes. Their
canonical hashes are BLAKE2b-256 over the complete byte strings returned by
`chain_getBlock`, including each compact length prefix. The fixture does not
claim that its synthetic opaque extrinsics are executable signed transactions;
they exist to lock index/hash joining and malformed-stream defenses. Block
hashes after genesis are likewise synthetic identities with exact parent
continuity, not recomputed header commitments.
