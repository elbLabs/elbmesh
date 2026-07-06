# ADR 0004: External Operations Are First-Class Execution Metadata

Status: Accepted

Date: 2026-07-03

## Context

The framework must support Resources whose source of truth is external or mixed, such as LexOffice documents. External HTTP calls create partial failure problems because external side effects and NATS event appends cannot be committed atomically.

We do not want to expose fake domain concepts such as `InvoiceCreationRequested` only to model recovery mechanics.

## Decision

External systems are first-class in the execution model, not Resource concepts in the business ontology.

Actions that perform external side effects must declare their External Operations. Resource source of truth alone does not permit arbitrary external calls.

Restate is mandatory for the v1 execution runtime, but hidden behind framework abstractions.

Framework users interact with:

```text
ActionContext
ExternalOperation
EventStore
ActionJournal
OperationJournal
```

They do not call Restate APIs directly from domain handlers.

## Consequences

The domain surface can stay clean:

```text
Action: Create Invoice
External Operation: LexOffice POST /v1/invoices
Event: Invoice Created
```

The execution layer records operational facts separately:

```text
Action called
External operation reserved
External operation succeeded
Resource Event appended
Action completed
```

If an external call succeeds and appending the Resource Event fails, Restate retries the append using the already-recorded external result. It must not repeat the external HTTP call unless the operation result is absent and idempotency allows retry.

Resource Events store selected domain facts, not full provider responses. Provider request/response diagnostics belong in the Operation Journal or object storage.
