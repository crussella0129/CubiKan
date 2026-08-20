# `cubikan` JSON CLI

`cubikan` is a strict, one-shot adapter for CubiKan's stateless protocol v2. It
reads exactly one RFC 8259 JSON value from standard input, simulates an Intent
Unit lifecycle with `cubikan-core`, and writes exactly one compact JSON response
plus one line feed. A modeled exit is returned only after the response body,
line feed, and an explicit stdout flush all succeed.

The adapter is simulation-only. It has no database, RPC client, signer,
repository, durable session, or cross-invocation state, and its output never
claims canonical or durable authority.

## Run a locked example

From the repository root:

```sh
cargo run -p cubikan-cli --bin cubikan \
  < tests/fixtures/protocol-v2/cubikan/requests/success_completion.json
```

The closed structural schema is
[`protocol/v2/cubikan.schema.json`](../../protocol/v2/cubikan.schema.json). The
independent raw request/response corpus and hashes are in
[`tests/fixtures/protocol-v2/cubikan/manifest-v1.json`](../../tests/fixtures/protocol-v2/cubikan/manifest-v1.json).

## Request contract

A request has exactly these top-level members:

```json
{
  "protocol_version": 2,
  "workflow": {
    "id": "delivery-v1",
    "phases": ["queued", "doing", "done"],
    "initial_phase": "queued",
    "edges": [
      { "from": "queued", "to": "doing" },
      { "from": "doing", "to": "done" }
    ],
    "completion_phases": ["done"]
  },
  "intent_unit": {
    "id": "67e55044-10b1-426f-9247-bb680e5fe0c8",
    "origin": {
      "namespace": "book.intent",
      "scope": "INT-0008",
      "value": "required-origin"
    },
    "species": "work-item"
  },
  "operations": [
    { "type": "transition", "target": "doing" },
    { "type": "transition", "target": "done" },
    { "type": "complete" }
  ]
}
```

Every object is closed at every depth: unknown and duplicate members reject.
Required members reject when omitted or `null`. `intent_unit.id` is the sole
optional member; omission generates a cryptographically random UUID v4 in the
client process, while explicit `null` rejects. A supplied ID must be exactly a
36-byte lowercase, hyphenated RFC 4122 UUID. Nil is syntactically valid.

`intent_unit.origin` is required and is preserved exactly. Its namespace uses
`[a-z][a-z0-9._-]{0,63}`; scope and value are nonblank, NUL-free UTF-8 of at
most 256 bytes. Workflow IDs, phase IDs, and species use the same 256-byte text
ceiling. Values are never trimmed, case-folded, or normalized.

One workflow permits at most 32 phases, 128 edges, and 32 completion phases.
One request permits at most 256 operations. Transition and completion behavior
is delegated to the bounded core workflow and Intent Unit types.

The complete raw request is limited to 1,048,576 bytes. Every byte counts,
including leading and trailing JSON whitespace. The reader retains at most one
byte beyond the ceiling to distinguish an exact-size request from an oversized
request; overflow rejects before JSON classification.

## Responses and exits

Success is always explicitly simulation-only:

```json
{"protocol_version":2,"authority":"simulation_only","outcome":"success","result":{"type":"simulation","intent_unit":{"id":"67e55044-10b1-426f-9247-bb680e5fe0c8","origin":{"namespace":"book.intent","scope":"INT-0008","value":"required-origin"},"species":"work-item","workflow":{"id":"delivery-v1","phases":["queued","doing","done"],"initial_phase":"queued","edges":[{"from":"queued","to":"doing"},{"from":"doing","to":"done"}],"completion_phases":["done"]},"phase":"done","status":"completed","revision":"3","history":[{"type":"transition","sequence":"1","from":"queued","to":"doing"},{"type":"transition","sequence":"2","from":"doing","to":"done"},{"type":"completion","sequence":"3","phase":"done"}]}}}
```

Request/setup failures contain `error` and no Intent Unit snapshot. Lifecycle
failures use zero-based `operation_number` and include the partial state after
earlier successful operations but before the rejected operation. Rejected
operations do not mutate that snapshot, and later operations are not attempted.
Revisions and history sequence numbers are canonical decimal JSON strings.

| Exit | Meaning |
|---:|---|
| `0` | Simulation succeeded |
| `1` | Input/output delivery failed; a complete response is not guaranteed |
| `2` | Request or setup rejected |
| `3` | Lifecycle operation rejected |

Exit `1` covers request-read, response-body, response-line-feed, and explicit
flush failures. The process writes one best-effort `cubikan: ...` diagnostic to
stderr and preserves the underlying I/O error as the Rust error source.

The protocol's error codes, exact messages, field pointers, response member
order, raw escaping, size boundaries, and I/O fault behavior are locked by the
independent fixture corpus. Validate it with:

```sh
bash protocol/v2/verify-fixtures.sh --locked
```
