# ADR 0011: Import Non-Event-Sourced Data as Provenanced Starting Facts

Status: Accepted

Date: 2026-07-03

## Context

Onboarding data from a non-event-sourced system into an event-sourced system is difficult because the source usually does not contain the actual historical domain Events.

If only current state is available, generating a fake perfect event history would make replay misleading.

The framework needs a safe migration model for legacy databases, external APIs, CSV imports, and existing provider documents.

## Decision

Support imports as explicit Actions that create provenanced Events.

Do not pretend imported current state is native event history.

Use three import styles:

### Snapshot Import

Use when only current state is known.

```text
Action: Import Invoice
Event: Invoice Imported
```

The Event records the current known Resource fields and provenance metadata.

Example:

```text
Invoice Imported
  invoice_id
  external_ref
  customer_id
  total
  status
  issued_at
  due_date
  imported_at
  source_system
  source_record_id
```

Future changes are recorded as normal native Events.

### Synthetic History

Use only when meaningful historical facts can be inferred from trustworthy source data.

Example source fields:

```text
created_at
approved_at
sent_at
paid_at
```

Possible generated Events:

```text
Invoice Created
Invoice Approved
Invoice Sent
Payment Observed
```

These Events must be marked as synthetic/imported in metadata.

### External Reference Import

Use when an external system remains the source of truth.

```text
Action: Link Existing Invoice
Event: Invoice Linked To External Document
```

The Event records the external reference and selected observed domain fields. It does not claim to reconstruct the external document's full history.

## Provenance Metadata

Imported Events should include or be accompanied by metadata such as:

```text
event_origin = native | imported | synthetic | external_observation
source_system
source_record_id
source_observed_at
import_batch_id
confidence
```

## Consequences

The event stream remains honest.

Imported Resources have a clear starting point.

Reconstructed state is deterministic from Events, but historical interpretation can distinguish native Events from imported/synthetic Events.

Views can be rebuilt from imported Events exactly like native Events.

Import tooling can be implemented through normal Actions and EventStore append semantics.

## Rejected Approach

Do not generate fake native history from current state alone.

Bad:

```text
Invoice Created
Invoice Approved
Invoice Sent
Invoice Paid
```

if the source only says:

```text
status = paid
```

Better:

```text
Invoice Imported
  status = paid
  source_system = legacy_erp
  imported_at = ...
```

## V1 Rule

V1 should support Snapshot Import and External Reference Import as first-class modelling patterns.

Synthetic History is allowed only when explicitly configured and marked with provenance metadata.
