# ADR 0005: Use Separate NATS Streams for Domain and Execution Records

Status: Accepted

Date: 2026-07-03

Amended: 2026-07-19

## Context

NATS is the intended base for event storage, metadata indexing, and object storage. Domain Events and execution records have different purposes and should not be mixed. The original decision named the logical categories but did not define the native JetStream protocol needed by EventStore and journal adapters.

## Decision

### Native streams

Use four separate native JetStream streams:

| Stream | Subject filter | Contents | Atomic publish |
| --- | --- | --- | --- |
| `ELBMESH_RESOURCES` | `elbmesh.resources.>` | Replayable Resource Events | Enabled |
| `ELBMESH_ACTIONS` | `elbmesh.actions.>` | Action attempts, policy decisions, receipts, and errors | Disabled |
| `ELBMESH_OPERATIONS` | `elbmesh.operations.>` | External Operation reservations, attempts, and results | Disabled |
| `ELBMESH_REACTIONS` | `elbmesh.reactions.>` | Reaction execution records | Disabled |

Every stream uses `Limits` retention, `File` storage, one replica, and a duplicate window of 2 minutes. `allow_batch_publish` remains disabled: non-atomic fast ingest is not part of the Elbmesh protocol. Only `ELBMESH_RESOURCES` enables atomic publish. A later workflow journal requires a separate decision rather than sharing one of these streams.

Only `ELBMESH_RESOURCES` participates in Resource state reconstruction. Views, View indexes, and projection checkpoints remain in NATS KV; large payloads remain eligible for NATS Object Store.

### Subjects and token encoding

One Resource stream maps to one exact NATS subject. Journal identities likewise map to exact subjects:

```text
elbmesh.resources.<resource-type-length>.<encoded-resource-type>.<resource-id-length>.<encoded-resource-id>
elbmesh.actions.<action-id-length>.<encoded-action-id>
elbmesh.operations.<operation-id-length>.<encoded-operation-id>
elbmesh.reactions.<reaction-id-length>.<encoded-reaction-id>
```

Each length is the original token's UTF-8 byte length written in base 10. Encoding leaves ASCII letters, digits, `_`, and `-` unchanged; every other UTF-8 byte becomes uppercase `%XX`. An empty token uses `_` with length `0`, so it remains distinct from a literal `_` with length `1`. Length prefixes and encoding prevent `.`, `*`, and `>` in identifiers from changing subject structure.

### Headers and sequence domains

Every native message carries a stable `Nats-Msg-Id` for server deduplication and these application headers:

```text
Elbmesh-Message-Type
Elbmesh-Message-Version
Elbmesh-Resource-Type
Elbmesh-Resource-Id
Elbmesh-Stream-Type
Elbmesh-Correlation-Id
Elbmesh-Causation-Id
Elbmesh-Action-Id
Elbmesh-Actor-Id
Elbmesh-Occurred-At
Elbmesh-Schema-Id
Elbmesh-Schema-Version
Content-Type: application/json
```

Operation records additionally carry `Elbmesh-Operation-Id`, `Elbmesh-External-System`, `Elbmesh-External-Operation`, and `Elbmesh-Idempotency-Key` when those values apply. Payloads remain the canonical serialized Event or journal record; headers provide routing, concurrency, and common indexing metadata without requiring payload inspection.

Resource messages also carry `Elbmesh-Aggregate-Sequence`. This is the one-based, aggregate-local Event sequence used for replay and the public Resource version. It is not a JetStream sequence. JetStream's stream sequence is a global transport position used by publish acknowledgements, consumers, and checkpoints. On an append, only the first message carries `Nats-Expected-Last-Subject-Sequence`: it contains the JetStream stream sequence of the previous message on that exact aggregate subject and enforces optimistic concurrency. Later messages in the same atomic batch target that subject again, so the NATS batch contract forbids repeating this expected-subject check on them. The header must never contain the aggregate-local sequence. Replay validates that `Elbmesh-Aggregate-Sequence` values are contiguous.

### Atomic Resource append

A multi-Event Resource append uses the NATS 2.14 atomic batch protocol and one stable batch identity. It uses the Action ID when that value is a valid NATS batch ID; otherwise the adapter derives a deterministic value no longer than the 64-character server limit:

1. Reject an empty append locally and reject more than 1,000 messages before publishing.
2. Give every message a unique, stable `Nats-Msg-Id`, the same `Nats-Batch-Id`, a one-based consecutive `Nats-Batch-Sequence`, and `Nats-Required-Api-Level: 4`.
3. Publish the first message without `Nats-Batch-Commit`. Its successful handshake is a zero-byte acknowledgement; this means only that the server opened the batch, not that any Event committed.
4. Publish every message within the server's 10 seconds batch timeout. Put `Nats-Batch-Commit: 1` only on the final message.
5. Treat only the final JSON publish acknowledgement as commit. Its `batch` must equal `Nats-Batch-Id`, its `count` must equal the requested message count, and its `seq` is the final global JetStream stream sequence, not the Resource version.

Until the commit acknowledgement, the server does not expose any message in the batch. A missing final message or timeout makes the server abandon the batch. The adapter reports no success from intermediate zero-byte acknowledgements.

Duplicate message IDs within an atomic batch reject the whole batch with the stable server error `10201` (`batch publish contains duplicate message id`); no partial append is accepted. The adapter preserves message and batch identities across recovery.

If the server may have committed but the client lost acknowledgement, the adapter performs read-back reconciliation on the exact Resource subject before republishing. It compares the expected `Nats-Msg-Id`, `Elbmesh-Aggregate-Sequence`, and payload identity for every Event. A complete match is success, no match permits retry with the same identities after the abandoned batch is known to be gone, and a partial or mismatched result is a named storage/protocol error. Blind retry after a lost acknowledgement is forbidden because deduplication cannot by itself distinguish a committed batch from an abandoned one.

Single-Event Resource appends use the same identity, header, expected-last-subject-sequence, and reconciliation rules; they do not need the batch handshake.

### Durable delivery and cursor ownership

Reactions and projections consume Resource Events through explicit durable consumers. Durable names are deterministic:

```text
ELBMESH_REACTION_<encoded-reaction-type>
ELBMESH_PROJECTION_<encoded-projection-type>
```

The durable-name token uses the same uppercase percent encoding and includes its UTF-8 byte length, joined without NATS subject separators. The adapter creates and owns the consumer, filter subject, explicit acknowledgement policy, redelivery settings, and delivery cursor. Application Reaction and Projection code cannot acknowledge messages or move a cursor directly.

The JetStream consumer ack floor is the authoritative transport delivery cursor and advances only after the adapter has durably completed the Reaction or projection step. A projection checkpoint remains in KV and records the last applied global JetStream sequence for idempotency and read-back reconciliation; it is adapter-owned recovery state, not Resource truth and not a replacement for the consumer ack floor. Reaction execution records remain outside Resource streams.

### Client and server compatibility

The harness pins `nats:2.14.3-alpine` and starts it with JetStream enabled. `async-nats` disables default features and enables its cumulative `server_2_10`, `server_2_11`, `server_2_12`, and `server_2_14` contracts. In particular, the 2.12 gate exposes atomic publish configuration and the 2.14 gate exposes final batch acknowledgement fields.

## Consequences

Resource replay stays simple.

Auditing and recovery can inspect execution streams.

Indexes can be built from message metadata without requiring payload inspection for common routing needs.

Aggregate-local ordering remains explicit instead of being inferred from a global broker cursor. Atomic Resource batches preserve one-Action/one-stream all-or-nothing append semantics, while durable consumer and checkpoint state stay adapter-owned.

This ADR defines the protocol foundation. Replacing the existing KV-backed EventStore and journal adapters is follow-on work; this decision neither changes their public ports nor introduces domain-specific behavior.
