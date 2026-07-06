# ADR 0007: Model Workflows as Event-Action Graphs

Status: Accepted

Date: 2026-07-03

## Context

Linear workflows are too restrictive. The desired model is a graph that shows how Actions produce Events and how Events trigger later Actions across Resources.

## Decision

Use this graph model:

```text
Action -> Event -> Reaction -> Action
```

Definitions:

```text
Reaction = typed subscription from one Event to one Action.
Workflow = named view/subgraph of Reactions.
```

Rules for v1:

```text
One Event may trigger many Reactions.
Cycles are forbidden.
Reaction conditions are allowed, but must be pure.
Actions still target exactly one Resource.
Actions still append Events to exactly one Resource stream.
External side effects happen only inside declared Actions.
Reactions subscribe only to successful Resource Events.
```

## Consequences

The model can be visualized and traversed forward or backward.

Restate runs Reactions durably under the hood.

Reactions call Actions through the same Action execution API used by humans, agents, and tests.

Workflow status can be derived from Resource Events, Action Journals, Reaction Journals, and Operation Journals instead of a custom saga state machine in v1.
