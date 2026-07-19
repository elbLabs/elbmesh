# ADR 0008: Views Are Materialized Read Models With Declared Queries

Status: Accepted

Date: 2026-07-03

## Context

Resources are optimized for consistency and replay, not cross-Resource querying. Business questions such as `List Receivables` require data from multiple Resource types.

The first implementation should stay light and avoid requiring Postgres if possible.

## Decision

Use Views for read models and Queries for read capabilities.

```text
Resource = write/consistency boundary
View = query boundary
Query = declared read capability against a View
Projection = code that updates a View from Events
```

Views may subscribe to Events from multiple Resource types.

V1 supports materialized Views only. Live external enrichment is postponed but should remain a planned extension.

Use NATS-only storage in v1:

```text
Resource Events: NATS JetStream
View documents: NATS KV
View indexes: NATS KV
Large payloads/files: NATS Object Store
Projection checkpoints: NATS KV
```

Projection application is source-aware. Every handler receives a `ProjectionContext` containing
the source message ID, source Resource stream identity, aggregate sequence, and an opaque
transport cursor. The core stores per-View application metadata under this identity:

```text
projection type + source Resource stream + target View key
```

Duplicate positions and older positions from the same source Resource stream do not rewrite that
View target. Aggregate sequences from different Resource streams are never compared. The built-in
ViewStores persist the application position with the View document update, so retry after a View
write and before Event checkpoint persistence does not reapply a successful write.

One projection checkpoint represents completion of one source Event across every required
projection handler. It advances only after all required View writes succeed. The checkpoint stores
the same source identity and opaque cursor as the handler context. Cursor interpretation, message
acknowledgement, and transport redelivery remain adapter responsibilities; the projection core does
not acknowledge transport messages.

A rebuild first pauses normal delivery, then idempotently resets an explicit selection of Views,
their application metadata, and the Event checkpoint before replay. Replay runs through the same
source-aware handlers, and finishing the rebuild resumes normal delivery. This is a focused
projection lifecycle, not an arbitrary transaction abstraction.

V1 query primitives:

```text
get_by_id
list_by_index_prefix
```

Every list query must use a declared index. `List all` is implemented as a declared `all` index.

## Consequences

NATS-only views are feasible for explicit queries.

The framework does not promise arbitrary SQL-like querying in v1.

Adapters can later add Postgres, Elasticsearch, or another query engine behind framework ports without changing Resource behavior.

Projection code is handwritten Rust in v1, mirroring Action handlers and Event apply logic.

Partial handler or checkpoint failures remain retryable without regressing successful View writes.
Checkpoint durability is provided through the `ProjectionCheckpointStore` port. A future JetStream
consumer may use the opaque cursor to resume and acknowledge delivery, but no consumer or implicit
acknowledgement is part of this decision.
