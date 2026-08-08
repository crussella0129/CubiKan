# `cubikan` JSON CLI

`cubikan` is CubiKan's experimental runnable adapter. It reads one complete
lifecycle scenario from standard input, delegates validation and mutation to
`cubikan-core`, and writes exactly one compact JSON response followed by a
newline.

One process owns one in-memory scenario. There is no durable session, repository,
or cross-invocation state.

## Run the lifecycle example

From the repository root:

```sh
cargo run -p cubikan-cli --bin cubikan < crates/cubikan-cli/tests/fixtures/lifecycle-success-v1.json
```

## Protocol version 1 request

Version 1 is strict: unknown fields, missing fields, and fields with the wrong
JSON type are rejected as `invalid_request`.

```json
{
  "protocol_version": 1,
  "workflow": {
    "id": "delivery",
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
    "species": "feature"
  },
  "operations": [
    { "type": "transition", "target": "doing" },
    { "type": "transition", "target": "done" },
    { "type": "complete" }
  ]
}
```

`intent_unit.id` is optional. When omitted, the core generates a non-nil UUID
v4; when supplied, any UUID accepted by `cubikan-core` retains the same UUID
value.
Workflow IDs, phases, species, and operation targets are caller-defined nonblank
text and are not trimmed. Empty operation lists, empty completion sets, and
explicit reverse or self edges are valid when the core accepts the topology.

## Success response

```json
{
  "outcome": "success",
  "protocol_version": 1,
  "intent_unit": {
    "id": "67e55044-10b1-426f-9247-bb680e5fe0c8",
    "species": "feature",
    "workflow_id": "delivery",
    "phase": "done",
    "status": "completed",
    "history": [
      {
        "type": "transition",
        "sequence": 1,
        "from": "queued",
        "to": "doing"
      },
      {
        "type": "transition",
        "sequence": 2,
        "from": "doing",
        "to": "done"
      },
      { "type": "completion", "sequence": 3, "phase": "done" }
    ]
  }
}
```

The response is an adapter-owned view assembled from core accessors. It is not
the provisional serialized representation of `Workflow` or `IntentUnit`.

## Typed failure response

Request and setup failures contain no `intent_unit` or `operation_number`.
Field-specific failures may contain `field`. Lifecycle failures always contain
the rejected operation's one-based `operation_number` and the Intent Unit state
after earlier successful operations but before the rejected operation.

```json
{
  "outcome": "error",
  "protocol_version": 1,
  "error": {
    "code": "transition_not_allowed",
    "message": "transition `doing -> queued` is not declared",
    "operation_number": 2
  },
  "intent_unit": {
    "id": "67e55044-10b1-426f-9247-bb680e5fe0c8",
    "species": "feature",
    "workflow_id": "delivery",
    "phase": "doing",
    "status": "active",
    "history": [
      {
        "type": "transition",
        "sequence": 1,
        "from": "queued",
        "to": "doing"
      }
    ]
  }
}
```

Execution is fail-fast, not transactional. Earlier successful operations remain
visible; the core guarantees that the rejected operation itself does not mutate
state; later operations are not attempted. Error `message` text is for humans
and must not be parsed as a machine contract.

## Error codes and exits

| Exit | Meaning | Codes |
|------|---------|-------|
| `0` | Success | none |
| `1` | Operational input/output failure; a complete JSON response cannot be guaranteed | none; best-effort stderr diagnostic |
| `2` | Request or setup rejection | codes below; no Intent Unit snapshot |
| `3` | Lifecycle rejection | codes below; operation number and partial snapshot included |

Version 1 request/setup codes:

- `invalid_json` for malformed JSON or unexpected EOF;
- `invalid_request` for an invalid JSON shape or unknown field;
- `unsupported_protocol_version`;
- `blank_value`, with the failing JSON field path;
- `invalid_intent_unit_id`, with `intent_unit.id`;
- `workflow_empty_phases`;
- `workflow_duplicate_phase`;
- `workflow_unknown_initial_phase`;
- `workflow_unknown_edge_source`;
- `workflow_unknown_edge_target`;
- `workflow_duplicate_edge`;
- `workflow_unknown_completion_phase`;
- `workflow_duplicate_completion_phase`.

Version 1 lifecycle codes:

- `transition_already_completed`;
- `transition_unknown_target`;
- `transition_not_allowed`;
- `completion_already_completed`;
- `completion_phase_not_eligible`.

## Boundary and hardening status

The protocol is explicitly experimental. Protocol version 1 defines the current
adapter contract, but no cross-version compatibility guarantee exists yet.

The local CLI currently reads standard input without a size limit. Resource
limiting is deferred hardening and is required before any production network
exposure. This adapter does not select or provide persistence, networking,
deployment, authorization, concurrency, KPI enforcement, completed-unit naming,
blockchain behavior, or UI policy. Each requires separate product intent rather
than silent expansion of this execution envelope.
