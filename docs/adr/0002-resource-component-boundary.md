# ADR 0002: Resources Are Aggregates, Components Are Owned State

Status: Accepted

Date: 2026-07-03

## Context

Allowing `Resource` to mean aggregate, entity, value object, external reference, or derived view makes model-to-code transformation ambiguous.

The framework needs a simple rule that supports code generation and agent reasoning.

## Decision

In v1:

```text
Resource = event-sourced aggregate root
Component = owned state inside a Resource
```

Rules:

```text
A Resource has its own event stream.
A Component never has its own event stream.
A Component can only change through its owning Resource.
A Component may have local identity or no identity.
An Action always targets exactly one Resource.
An Action appends Events to exactly one Resource stream.
Cross-Resource progress is handled through Reactions and Workflows.
```

Normal domain hard delete is out of scope. Business lifecycles should use actions such as cancel, withdraw, void, archive, supersede, or mark deleted.

## Consequences

The model-to-code path is deterministic:

| Model Node | Generated/Runtime Role |
| --- | --- |
| Resource | State, Action enum, Event enum, stream, handler dispatch, policy hooks |
| Component with local identity | Nested type with ID scoped to owner Resource |
| Component without identity | Nested value type |

Derived/cross-Resource questions are modelled as Views, not Resources.
