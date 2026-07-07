# Implementation Plan

This plan captures the first implementation path for the event-sourcing framework. It is intentionally strict to keep v1 buildable.

Delivery is governed by [Phased Delivery Plan](PHASED_DELIVERY_PLAN.md). The Orchestrator should create GitHub Issues and PRs/MRs from that phased plan rather than from this document directly.

## V1 Goal

Build a Rust event-sourcing framework that can execute typed Resource Actions, append Resource Events to NATS, run durable external operations through Restate, project Views into NATS KV, and expose generated capability documentation for humans and agents.

The first acceptance example is:

```text
Offer -> Sales Order -> Order Confirmation -> Invoice
```

with a mocked external API for LexOffice-like document creation.

## Non-Goals

```text
No hard domain delete.
No arbitrary query engine.
No background sync.
No live external enrichment in v1.
No cyclic Reaction graphs.
No generic compensation engine.
No handwritten Restate usage in domain handlers.
No generated business behavior by default.
```

## Architecture Slices

### 1. Core Traits

Define the typed framework contracts:

```text
Resource
Handle<Action>
Apply<Event>
Project<Event>
ActionExecutor
EventStore
ActionJournal
OperationJournal
ReactionJournal
ExternalOperation
ViewStore
QueryEngine
```

V1 developer behavior should use explicit trait impls.

### 2. Message Envelope And Metadata

Define a shared message envelope for Resource Events and execution journals.

Required metadata:

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

Add operation-specific metadata where needed:

```text
operation_id
external_system
external_operation
idempotency_key
```

### 3. NATS Event Store

Implement Resource Event append and replay on NATS JetStream.

Requirements:

```text
append Events to one Resource stream
load Events for one Resource
support optimistic concurrency or expected version
preserve per-Resource ordering
store metadata in headers/envelope
```

### 4. Execution Journals

Implement NATS-backed execution journals:

```text
actions
operations
reactions
```

Only Resource Events are used for replay. Journals support audit, idempotency, and recovery.

### 5. Action Runtime

Implement `ActionExecutor`.

Flow:

```text
receive Action
record Action called
load Resource Events
replay Resource State
evaluate policy hooks
execute typed handler
perform declared External Operations through context when needed
append Resource Events
record Receipt or failure
return Receipt
```

Action rules:

```text
Action targets exactly one Resource.
Action appends Events to exactly one Resource stream.
Failed/denied Actions do not append Resource Events unless modelled explicitly.
```

### 6. Restate Execution Adapter

Use Restate as mandatory v1 durable execution runtime.

Hide Restate behind framework APIs.

External operation flow:

```text
reserve operation
call external API with idempotency key
journal operation success/failure
append Resource Event
retry append if it fails without repeating successful external call
```

### 7. External Operation Support

Implement declared External Operations for Actions.

Requirements:

```text
operation identity
idempotency key
request/response metadata
selected domain facts mapped into Events
raw provider diagnostics stored in Operation Journal or Object Store if needed
```

### 8. Reaction Runtime

Implement typed `Event -> Action` Reactions.

V1 rules:

```text
one Event may trigger many Reactions
cycles forbidden
conditions are pure
reactions subscribe only to successful Resource Events
reaction execution calls ActionExecutor
```

Restate should durably run Reaction execution.

### 9. Views And Queries

Implement materialized Views on NATS KV.

Storage:

```text
View documents: NATS KV
View indexes: NATS KV
Projection checkpoints: NATS KV
Large payloads/files: NATS Object Store
```

Query primitives:

```text
get_by_id
list_by_index_prefix
```

Projection rules:

```text
Views may subscribe to Events from many Resource types.
Projection code is handwritten Rust.
Projection updates are idempotent.
Checkpoint only after view/index writes succeed.
```

### 10. Macros And Generated Bindings

Provide Rust macros/codegen helpers for generated contracts.

Generated artifacts should include:

```text
Action DTOs
Event DTOs
Receipt DTOs
Resource IDs
Component structs
Action/Event enums
schema IDs and versions
NATS metadata bindings
External Operation declarations
policy hook bindings
dispatch boilerplate
capability docs
```

Developers write:

```text
Handle<Action> impls
Apply<Event> impls
Project<Event> impls
external response mapping
```

### 11. Capability Documentation

Generate both human and machine-readable capability docs from the same manifest.

Outputs:

```text
RESOURCE_CAPABILITIES.md
resource-capabilities.json
```

Both must include a manifest hash and generator version to keep them in sync.

### 12. Agent Workflow Task Routes And Architecture Checks

Make the repository agentically usable.

Canonical workflow route catalog:

```text
docs/AGENT_SKILLS.md
```

PPP task routes to support:

```text
task.elbmesh-plan-implementation-slice
task.elbmesh-coordinate-phase-work
task.elbmesh-write-failing-tests
task.elbmesh-implement-runtime-slice
task.elbmesh-review-change
task.elbmesh-review-mr-readiness
task.elbmesh-maintain-docs
task.elbmesh-check-architecture-boundaries
task.elbmesh-explain-action-event-flow
task.elbmesh-update-architecture-manifest
```

Concrete project PPP routing files:

```text
.opencode/skills/ppp/SKILL.md
.ppp/library/tasks/elbmesh-*.json
```

Workflow-specific `elbmesh-*` OpenCode skills were migrated to PPP task bundles and removed. Update PPP tasks/items first, then keep the central PPP route map and this catalog aligned.

The architecture checker should eventually become:

```text
elbmesh check-architecture
```

The flow explainer should eventually become:

```text
elbmesh explain-flow
```

### 13. Acceptance Example

Build an example app with:

```text
Resources: Offer, Sales Order, Order Confirmation, Invoice
Actions: Create Offer, Accept Offer, Create Sales Order, Create Order Confirmation, Create Invoice
Events: Offer Created, Offer Accepted, Sales Order Created, Order Confirmation Created, Invoice Created
Reactions: Offer Accepted -> Create Sales Order -> Sales Order Created -> Create Order Confirmation -> Order Confirmation Created -> Create Invoice
View: Receivable or Document Flow Status
Mock External API: LexOffice-like document endpoint
```

The hard acceptance test should prove:

```text
External API call succeeds.
Resource Event append initially fails.
Restate retries append.
External API is not called twice.
Invoice Created is recorded exactly once.
```

## Open Questions

```text
Exact manifest format and whether it is authored directly or generated by a modeller.
Exact optimistic concurrency mechanism with NATS JetStream.
Exact schema versioning and upcasting strategy beyond v1 direct Apply per Event version.
Whether generated Rust files are checked into consuming repositories or generated at build time.
How policy hooks are represented for DMN and FGA.
How Claims/Reservations for external references should be implemented in v1.
```

## Suggested Build Order

1. Define core traits and message envelope.
2. Implement in-memory EventStore and ActionExecutor for fast tests.
3. Implement NATS EventStore.
4. Implement Action Journal and Operation Journal.
5. Implement Restate-backed external operation execution.
6. Build the Offer-to-Invoice example without Reactions.
7. Add Reaction runtime and connect the example graph.
8. Add NATS KV ViewStore and one View.
9. Add macro/codegen helpers.
10. Add generated capability docs.
11. Add generated or checked agent skill packaging.

For MR sequencing, use `docs/PHASED_DELIVERY_PLAN.md` as the source of truth.
