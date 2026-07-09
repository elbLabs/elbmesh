# Capability Document

- Capability schema: `capabilities.elbmesh.v1` v1
- Manifest schema: `manifest.elbmesh.v1` v1
- Manifest hash: `fnv1a64:6a2d2e43edfa832f`
- Generator: `elbmesh-core` v0.1.0

## Runtime Boundaries

This document describes declared capabilities and implemented framework boundaries only.
Live NATS and Restate infrastructure are feature-gated and not required by default.
Resource Events remain separate from ActionJournal, ReactionJournal, OperationJournal, ViewStore, provider diagnostics, and generated visibility artifacts.

## Resources

| Resource | Schema | Version | Components |
| --- | --- | --- | --- |
| `offer` | `resource.offer.v1` | 1 | `offer_terms` (`component.offer_terms.v1` v1) |

## Actions

| Action | Resource | Schema | Version | Emits | External Operations |
| --- | --- | --- | --- | --- | --- |
| `send_offer_email` | `offer` | `action.send_offer_email.v1` | 1 | `offer_created` | `lexoffice_create_invoice` |

## Events

| Event | Resource | Schema | Version |
| --- | --- | --- | --- |
| `offer_created` | `offer` | `event.offer_created.v1` | 1 |

## Reactions

| Reaction | Trigger Event | Target Action | Schema | Version |
| --- | --- | --- | --- | --- |
| `offer_created_to_send_offer_email` | `offer_created` | `send_offer_email` | `reaction.offer_created_to_send_offer_email.v1` | 1 |

## Views

| View | Schema | Version |
| --- | --- | --- |
| `offer_summary` | `view.offer_summary.v1` | 1 |

## Queries

| Query | Schema | Version |
| --- | --- | --- |
| `get_offer_summary` | `query.get_offer_summary.v1` | 1 |

## External Operations

External Operations use idempotency keys and OperationJournal records for call/completion/failure boundaries. Provider diagnostics are not Resource Events.

| External Operation | Schema | Version |
| --- | --- | --- |
| `lexoffice_create_invoice` | `external_operation.lexoffice_create_invoice.v1` | 1 |
