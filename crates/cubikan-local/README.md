# `cubikan-local`

`cubikan-local` is the strict protocol-v2 process adapter for a local CubiKan
chain and its verified SQLite projection. Reads are served from an attested
projection. Mutations use the single chain-client submission lane and report a
modeled finality outcome; they never write canonical lifecycle state to SQLite.

## Invocation

The command line has one exact positional form:

```text
usage: cubikan-local --database PATH --rpc URL [--dev-signer charlie|dave]
```

`--database`, then `--rpc`, then the optional `--dev-signer` pair must appear in
that order. Equals forms, repeated or reordered flags, extra arguments,
`:memory:`, caller-selected journal paths, raw seeds, and private keys are
rejected with the usage line and exit 2 before stdin is read. Environment
variables cannot supply or override an argument.

The request is decoded before the RPC endpoint or signer value is interpreted.
After decoding:

- reads require no signer and reject a supplied signer with usage and exit 2;
- mutations require exactly the named `charlie` or `dave` development signer
  and reject an omitted or differently named signer with usage and exit 2;
- malformed RPC values become modeled protocol errors after decoding, and an
  unavailable recognized development signer is likewise modeled.

RPC endpoints are accepted only in the protocol's canonical lowercase `ws`
loopback form with an explicit non-default port and exactly `/` as the path,
such as `ws://127.0.0.1:9944/` or `ws://[::1]:9944/`. Hostnames, public or
wildcard addresses, credentials, queries, fragments, redirects, ambiguous IP
spellings, and normalized-but-not-byte-identical forms are rejected before a
dial.

## Protocol and operation inventory

stdin contains one strict JSON protocol-v2 request. The public operation union
contains exactly fifteen operations:

- lifecycle: `create_intent_unit`, `get_intent_unit`, `list_intent_units`,
  `transition_intent_unit`, and `complete_intent_unit`;
- definitions and relationships: `create_relationship_definition`,
  `get_relationship_definition`, `create_relationship`,
  `delete_relationship`, and `list_relationships`;
- projection: `project_intent_units_v1`;
- provenance associations: `record_association`, `revoke_association`,
  `list_associations_by_unit`, and `list_associations_by_reference`.

Every object is closed: duplicate or unknown members, wrong shapes, explicit
null optionals, invalid scalar/cursor values, and protocol v1 are rejected
before RPC, SQLite, signing, or submission authority is acquired. Command
schema version 1 is supplied internally and is not caller-selectable.

The normative public schema is
[`protocol/v2/cubikan-local.schema.json`](../../protocol/v2/cubikan-local.schema.json).
The independently authored raw oracle, manifest, inventory, process contract,
and I/O contract live under
[`tests/fixtures/protocol-v2/cubikan-local`](../../tests/fixtures/protocol-v2/cubikan-local).

## Bounded delivery and exits

The runner retains at most 1,048,577 raw request bytes: the 1 MiB limit plus one
lookahead byte. Inputs of 1,048,575 and 1,048,576 bytes continue to decoding;
1,048,577 bytes produce the modeled `request_too_large` response without RPC,
SQLite, signing, or submission. This is an ingress-buffer bound, not a claim
about total process memory.

For every modeled outcome stdout receives exactly one compact JSON body, one LF,
and one explicit flush, in that order. A resolved mutation journal is
acknowledged only after all three delivery stages succeed. A response-body, LF,
or flush failure returns exit 1 and leaves the journal available for restart
reconciliation. An acknowledgement failure also returns exit 1, leaves the
already-flushed response line untouched, emits no second stdout value, and
retains the journal.

| Exit | Meaning |
| ---: | --- |
| 0 | read success or finalized mutation acceptance |
| 1 | operational, unresolved, expired, invariant, indeterminate-delivery, process-I/O, or acknowledgement failure |
| 2 | usage, request-shape/value, or setup rejection |
| 3 | read miss, domain rejection, submission rejection, or finalized dispatch rejection |
| 4 | environment or infrastructure failure |

Structural usage failures write only the exact usage line to stderr. Modeled
outcomes write only their JSON line to stdout. Process I/O diagnostics have the
fixed `cubikan-local:` prefix; acknowledgement failure intentionally omits the
underlying journal error from stderr.

## Authority and security boundary

The crate's Rust library surface exposes only the bounded `run_process` entry
point and `MAX_REQUEST_BYTES`. Request execution, raw RPC, projection access,
signing, submission, and journal acknowledgement are private, so another crate
cannot bypass the process delivery contract.

The named accounts and loopback RPC are development-only conveniences, not
production key management or public deployment support. Signer identity is not
authorship, ownership, causality, or proof of external content. The SQLite
projection is derived and disposable; the chain remains canonical. The
per-signer journal coordinates cooperating local processes and is neither
canonical state nor tamper-proof against the same user. Finality, archive
availability, projection lag, fees, disclosure, filesystem durability, and
same-user deletion remain explicit operational risks.
