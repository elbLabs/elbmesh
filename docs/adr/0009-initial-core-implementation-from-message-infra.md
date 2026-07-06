# ADR 0009: Start With a Typed Core Based on the Existing Message Infrastructure

Status: Accepted

Date: 2026-07-03

## Context

The existing implementation in `/Users/richardsto/code/lococo/suite-website-redesign/message-infra` already proves several useful patterns:

```text
event store abstraction
aggregate replay
optimistic version tracking
typed handlers
NATS JetStream as event store
NATS KV projections
macro-assisted registration
in-memory store for tests
```

The new framework has different modelling goals:

```text
JSON Schema contracts instead of protobuf as the long-term contract source
Resource/Action/Event vocabulary
execution journals separated from Resource Events
Restate-backed external operation execution
agent/human capability metadata
```

## Decision

The first implementation slice reuses the proven shape but changes the developer-facing core:

```text
Resource replaces AggregateRoot as the public model term.
Action replaces Command as the public model term.
Event remains explicit as the stored domain fact.
Handle<Action> and Apply<Event> are explicit trait impls.
ActionExecutor owns load -> replay -> handle -> append.
Events are recorded through ActionContext.
The first EventStore is in-memory for tests.
```

The old implementation stores uncommitted changes inside aggregates. The new core starts with handlers recording Events through `ActionContext` so generated code can own dispatch and execution metadata more directly.

To keep the useful old `apply_change` behavior, handlers should prefer:

```rust
ctx.record_applied(self, SomeEventV1 { ... })?;
```

This serializes the Event, applies it to the in-memory Resource, and only then adds it to the pending Events for append. That lets one Action emit multiple Events while later decisions can see earlier emitted changes.

## Consequences

The first slice is small and testable without NATS or Restate.

NATS and Restate adapters can be added behind the same core traits.

Generated code can later implement `Resource::apply_recorded` by matching generated Event enums and delegating to one `Apply<EventVersion>` impl per stored Event version.

The execution layer still owns persistence, metadata, journaling, idempotency, and future external operation handling.
