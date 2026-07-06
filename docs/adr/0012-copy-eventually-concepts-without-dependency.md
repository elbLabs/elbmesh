# ADR 0012: Copy Eventually Concepts Without Taking a Dependency

Status: Accepted

Date: 2026-07-04

## Context

The `eventually-rs` project is a clean Rust event-sourcing library. It provides useful patterns:

```text
Aggregate trait
Aggregate Root that tracks version and uncommitted Events
EventStore split into streaming and appending
expected version checks
message envelope with metadata
given/when/then scenario tests
in-memory and Postgres stores
```

Elbmesh has a different product thesis. It should not become another Rust event-sourcing library. It should become an architecture-as-code execution substrate for humans and coding agents.

Elbmesh needs concepts that `eventually-rs` does not model directly:

```text
Resource/Action/Event/Reaction/View/Policy vocabulary
architecture manifest and JSON Schema contracts
generated Rust bindings and capability docs
NATS JetStream/KV/Object Store as the v1 substrate
execution journals for Actions, Operations, and Reactions
Restate-backed durable external operation execution
Event-to-Action Reaction graphs
materialized multi-Resource Views and declared Queries
agent-oriented architecture checks
```

## Decision

Do not depend on `eventually-rs` in v1.

Copy and adapt the concepts that fit Elbmesh:

```text
Root-like Resource execution state with version and pending Events
EventStore = Streamer + Appender
expected version checks
message envelope with metadata
given/when/then scenario testing
```

Build Elbmesh's own core around the project vocabulary and runtime contracts:

```text
Resource
Action
Event
Receipt
ActionExecutor
ExternalOperation
Reaction
View
Query
Policy
```

## Consequences

Elbmesh avoids fighting another library's abstractions while the architecture manifest, generated bindings, NATS, Restate, journals, Reactions, and Views are still being shaped.

The implementation can still learn from `eventually-rs` where its patterns are proven and minimal.

The first core should remain small enough that it can be compared against `eventually-rs` concepts during review.

If Elbmesh later converges with `eventually-rs`, this decision can be revisited. For v1, the dependency would constrain more than it helps.

## Rejected Approach

Do not wrap `eventually-rs` as the core execution engine in v1.

Reasons:

```text
It is Rust-code-first, while Elbmesh needs manifest/schema-generated contracts.
It uses a classic aggregate repository flow, while Elbmesh needs ActionExecutor, Receipts, journals, Reactions, and External Operations.
It supports in-memory/Postgres storage, while Elbmesh is NATS-first.
Its command handler returns Result<(), Error>, while Elbmesh needs caller Receipts and execution audit.
Its macro layer is not aimed at Elbmesh's model/codegen direction.
```
