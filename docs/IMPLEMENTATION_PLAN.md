# Implementation Plan

This document owns technical scope. [DELIVERY_ROADMAP.md](DELIVERY_ROADMAP.md) owns capability dependencies; GitHub Issues own executable work.

## Target

Build a Rust architecture substrate that executes typed Resource Actions, stores Resource Events, runs durable external operations, dispatches Reactions, projects Views, and generates capability contracts for humans and agents.

Canonical terms and invariants are defined in [GLOSSARY.md](GLOSSARY.md) and [GOAL.md](GOAL.md).

## Runtime Layers

### Typed Core

- `Resource`, `Action`, `Event`, `Handle<Action>`, and `Apply<Event>`.
- `ActionExecutor` and `ActionContext`.
- Typed domain/runtime failures and Action receipts.
- Given/when/then Resource scenarios.

### Persistence And Visibility

- `EventStore` for Resource Events.
- Separate `ActionJournal`, `OperationJournal`, and `ReactionJournal` ports.
- Shared message metadata, schema identity, correlation, causation, and stream identity.
- In-memory reference implementations and reusable adapter contracts.

### Coordination And Reads

- Typed Event-to-Action Reactions through `ActionExecutor`.
- Deterministic Reaction and downstream Action identity.
- `ViewStore`, projections, declared indexes, and simple `get_by_id`/`list_by_index_prefix` Queries.

### External Execution

- Declared `ExternalOperation` contracts with typed request/response/failure types.
- Deterministic operation identity and idempotency keys.
- OperationJournal reuse before provider retry.
- Restate hidden behind framework adapters.

### Infrastructure Adapters

- NATS-backed EventStore, journals, and ViewStore behind feature flags.
- Explicit key/subject encoding and shared contract tests.
- Default builds and tests require no live NATS or Restate runtime.

### Manifest And Generation

- Architecture manifest definitions and named validation findings.
- Generated capability Markdown/JSON and Rust binding stubs.
- Shared manifest hash and generator version.
- Drift checks between manifest-derived artifacts.

Generated code declares contracts and boilerplate. Developers write Action handling, Event apply logic, projections, and external response mapping.

### Tooling

Planned tooling should expose architecture checks, flow explanations, and guided manifest changes without becoming a second source of truth.

## Reference Capability

```text
Offer -> Sales Order -> Order Confirmation -> Invoice
```

The proof includes typed Resources/Actions/Events, cross-Resource Reactions, one View, a mocked LexOffice-like provider, generated capability artifacts, and retry after provider success plus Resource Event append failure.

## Remaining Decisions

- ActionJournal replay and duplicate Action semantics.
- Partial commit recovery when terminal journaling fails.
- Durable projection subscription and acknowledgement adapters beyond the checkpoint port.
- Manifest authoring and generated-file ownership in consuming repositories.
- Schema upcasting beyond direct v1 handlers.
- Policy representation for DMN/FGA integrations.
- Provider registration and generated binding boundaries.

No hard delete, cyclic Reaction graph, arbitrary query engine, background sync, live read enrichment, generic compensation engine, handwritten Restate calls in handlers, or generated business behavior belongs in v1.
