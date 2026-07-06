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
