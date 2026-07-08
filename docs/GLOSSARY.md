# Glossary

This glossary captures the modelling language for the event-sourcing framework. The goal is to keep domain modelling, generated code, and agent-facing capability descriptions aligned.

## Core Terms

### Resource

A top-level business object with its own event stream.

Technical meaning: event-sourced aggregate root.

Rules:

```text
A Resource owns its Components.
A Resource exposes Actions.
A Resource records Events.
A Resource is reconstructed from its Events.
```

Examples:

```text
Offer
Sales Order
Order Confirmation
Invoice
Payment
```

### Component

Owned state inside a Resource.

Technical meaning: a collapsed modelling term for DDD entity or value object inside an aggregate.

Rules:

```text
A Component does not have its own event stream.
A Component changes only through its owning Resource.
A Component may have local identity or no identity.
```

Examples:

```text
Order Line
Address
Money
Customer Reference
Internal Approval State
```

### Action

A meaningful business capability that can be invoked by a human, agent, workflow/reaction, or system integration.

Technical meaning: command.

Rules:

```text
An Action targets exactly one Resource.
An Action appends Events to exactly one Resource stream.
An Action may emit one or more Events.
An Action may return a Receipt or an error/denied result.
An Action may perform external calls only through declared External Operations.
```

Examples:

```text
Create Offer
Accept Offer Into Sales Order
Create Invoice
Void Invoice
Sync Payment Status
```

### Event

A durable domain fact recorded after an Action succeeds.

Technical meaning: domain event used for event-sourced replay.

Rules:

```text
Events are stored in Resource streams.
Events are the only source for Resource reconstruction.
Events should describe what became true in the domain.
Execution failures are not Events unless explicitly modelled as business facts.
```

Examples:

```text
Offer Created
Offer Accepted Into Sales Order
Sales Order Created
Order Confirmation Created
Invoice Created
Invoice Voided
```

### Receipt

The response returned to the caller of an Action.

Technical meaning: command result.

Rules:

```text
A Receipt is not used for Resource replay.
A Receipt may summarize emitted Events.
A Receipt may contain caller-facing messages, errors, warnings, or ids.
```

Examples:

```text
Create Offer Completed
Accept Offer Completed
Create Invoice Completed
```

### Reaction

A typed subscription from an Event to another Action.

Technical meaning: durable event handler/process step.

Rules:

```text
A Reaction subscribes to a successful Resource Event.
A Reaction invokes exactly one Action.
One Event may trigger many Reactions.
Reaction conditions must be pure in v1.
Reaction execution is durable and idempotent.
```

Example:

```text
When Offer Accepted Into Sales Order happens, call Create Sales Order.
```

### Workflow

A named view or subgraph of Reactions.

Technical meaning: saga/process manager view, not necessarily a separate state machine in v1.

Rules:

```text
Workflows are built from Action -> Event -> Reaction -> Action paths.
Cycles are forbidden in v1.
Workflow status is derived from Events and journals in v1.
```

Example:

```text
Offer To Invoice Workflow:
Accept Offer -> Offer Accepted -> Create Sales Order -> Sales Order Created -> Create Order Confirmation -> Order Confirmation Created -> Create Invoice -> Invoice Created
```

### View

A materialized read model derived from Events.

Technical meaning: projection/read model.

Rules:

```text
A View may subscribe to Events from many Resource types.
A View is eventually consistent by default.
A View is rebuildable from Resource Events.
A View does not own business truth.
```

Examples:

```text
Receivable
Invoice Summary
Open Offers
Workflow Status
```

### Query

A declared read capability against a View.

Rules:

```text
A Query does not change Resources.
A Query does not produce Events.
A Query targets one View.
V1 queries use get_by_id or list_by_index_prefix.
Every list query must use a declared index.
```

Examples:

```text
Get Receivable
List Receivables
List Open Receivables
List Invoices By Customer
```

### Policy

A rule that governs Actions, Resources, lifecycle transitions, or access.

Technical meaning: DMN/FGA/business rule hook.

Rules:

```text
Policies can allow, deny, or require approval.
Policies do not replace Resource behavior.
Policies are not architecture notes.
```

Examples:

```text
Customer Required Policy
Positive Total Policy
Official Document Approval Policy
Allowed Sales Order Transition Policy
```

### External System

A provider or integration boundary used by External Operations.

External Systems are first-class execution metadata, not business Resources.

Examples:

```text
LexOffice
Payment Provider
ERP API
```

### External Operation

A declared external read or write performed during an Action.

Rules:

```text
External Operations must be declared on Actions.
External Operations are journaled separately from Resource Events.
External writes must use idempotency where possible.
External Operation results may be used to create Resource Events.
```

Example:

```text
LexOffice Create Invoice: POST /v1/invoices
```

### OperationJournal

A technical journal that records External Operation calls, completions, and failures.

Rules:

```text
OperationJournal records are not Resource Events.
OperationJournal records are keyed by operation_id.
OperationJournal records carry idempotency metadata for external retries.
Provider response details belong in OperationJournal or Object Store, not Resource Events.
```

### Resource State

The reconstructed current Resource data obtained by replaying Resource Events.

This is a technical/runtime concept. In the model, these are simply Resource fields.

Rules:

```text
Resource State is deterministic.
Resource State is built only from stored Events.
Resource State is used by Action handlers to make decisions.
Replay must never call external systems.
```

### Resource View

An enriched or read-optimized representation for humans and agents.

Rules:

```text
Resource Views may later include live external enrichment.
Resource Views are not used for event-sourced replay.
V1 supports materialized Views only.
```

### Import Action

An Action used to onboard data from a legacy database, external API, CSV file, or other non-event-sourced source.

Rules:

```text
Import Actions create explicit imported/linking Events.
Import Actions do not pretend current state is native history.
Import Actions should record provenance metadata.
```

Examples:

```text
Import Invoice
Link Existing Invoice
Import Customer
```

### Imported Event

A domain Event that establishes a Resource's known starting state from external or legacy data.

Rules:

```text
Imported Events are replayable.
Imported Events are not native historical Events.
Imported Events must be marked with provenance.
```

Examples:

```text
Invoice Imported
Customer Imported
Invoice Linked To External Document
```

### Synthetic Event

An Event generated during import from inferred historical data.

Rules:

```text
Synthetic Events are only allowed when the source data supports the inference.
Synthetic Events must be marked as synthetic/imported.
Synthetic Events should not be created from current state alone.
```

## Naming Rules

Actions are imperative:

```text
Create Offer
Accept Offer
Create Invoice
```

Events are past-tense facts:

```text
Offer Created
Offer Accepted
Invoice Created
```

Receipts may use completion language:

```text
Create Offer Completed
Accept Offer Completed
```

Standard Action kinds may derive default Event names:

```text
create -> Created
update -> Updated
archive -> Archived
cancel -> Cancelled
void -> Voided
submit -> Submitted
approve -> Approved
reject -> Rejected
publish -> Published
sync -> Synced
```

Custom Actions should name their Events explicitly.

## Source Of Truth Terms

### Authority

The owner of truth for a Resource, Component, or field.

Values:

```text
internal
external
mixed
derived
```

### Freshness

How current stored data is expected to be.

Values:

```text
stored
observed_on_action
live_on_read
subscribed
derived
```

V1 supports stored and observed_on_action. Live enrichment is planned but postponed.

### Event Origin

The provenance of an Event.

Values:

```text
native
imported
synthetic
external_observation
```

Meanings:

```text
native = produced by normal Action handling in this system
imported = starting fact from legacy/current-state import
synthetic = inferred historical fact generated during import
external_observation = fact observed from an external source of truth
```

## Out Of Scope For V1

```text
Hard domain delete
Cyclic workflows
Background sync
Live external enrichment
Arbitrary SQL-like queries
Generic rollback/compensation graph
Automatic reducer generation from model field mappings
```
