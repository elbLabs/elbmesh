# ADR 0005: Use Separate NATS Streams for Domain and Execution Records

Status: Accepted

Date: 2026-07-03

## Context

NATS is the intended base for event storage, metadata indexing, and object storage. Domain Events and execution records have different purposes and should not be mixed.

## Decision

Use separate logical NATS streams/categories:

```text
resources   = replayable Resource Events
actions     = Action attempts, policy decisions, receipts, errors
operations  = external operation reservations, attempts, results
reactions   = Reaction execution records
workflows   = optional workflow/subgraph execution records later
```

Only `resources` participates in Resource state reconstruction.

Messages should carry routing/indexing metadata in headers or envelope metadata.

Minimum metadata:

```text
message_id
message_type
message_version
resource_type
resource_id
stream_type
correlation_id
causation_id
action_id
actor_id
occurred_at
schema_id
schema_version
```

Additional operation metadata:

```text
operation_id
external_system
external_operation
idempotency_key
```

## Consequences

Resource replay stays simple.

Auditing and recovery can inspect execution streams.

Indexes can be built from message metadata without requiring payload inspection for common routing needs.
