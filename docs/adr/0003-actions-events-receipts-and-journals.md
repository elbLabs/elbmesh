# ADR 0003: Separate Actions, Events, Receipts, and Execution Journals

Status: Accepted

Date: 2026-07-03

## Context

An Action request, the fact recorded after successful handling, and the immediate response to a caller are different things. Mixing them makes replay and auditing unclear.

## Decision

Use four separate concepts:

| Concept | Purpose | Stored In Resource Stream? |
| --- | --- | --- |
| Action | Request to do something meaningful | No |
| Event | Durable domain fact used for replay | Yes |
| Receipt | Caller-facing result of an Action | No |
| Execution Journal Record | Audit/recovery record for actions, reactions, operations, errors | No |

Only successful domain facts are stored in Resource event streams.

Failed or denied Action attempts are stored in execution journals, not Resource streams, unless the failure is explicitly modelled as a business Event.

## Consequences

Resource replay remains clean and deterministic.

Action callers still get useful feedback through Receipts.

Execution failures remain auditable without polluting aggregate history.

Example:

```text
Action: Accept Offer
Event: Offer Accepted
Receipt: Accept Offer Completed
```

If acceptance is denied by policy, no Resource Event is appended. The denial is recorded in the Action Journal and returned as an error/denied Receipt.
