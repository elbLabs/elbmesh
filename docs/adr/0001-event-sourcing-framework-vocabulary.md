# ADR 0001: Use Domain-Friendly Vocabulary Over Raw Event-Sourcing Terms

Status: Accepted

Date: 2026-07-03

## Context

The framework should make event-sourced systems easier to build for humans and agents. Classical event-sourcing language is precise, but it can feel implementation-driven when used directly in a domain model.

We want the model to express business capability while still mapping cleanly to event-sourcing mechanics.

## Decision

Use a small domain-friendly vocabulary at the modelling layer:

| Model Term | Technical Meaning |
| --- | --- |
| Resource | Event-sourced aggregate root |
| Component | Owned state inside a Resource |
| Action | Command |
| Event | Stored domain fact |
| Receipt | Response returned to an Action caller |
| Reaction | Typed `Event -> Action` subscription |
| Workflow | Named view/subgraph of Reactions |
| View | Materialized read model/projection |
| Query | Read capability against a View |
| Policy | DMN/FGA/business rule hook |
| External Operation | Declared external read/write side effect |

Use `Event` explicitly in the modelling language. Events are not hidden, but obvious events may be generated from standard Action kinds.

## Consequences

The framework can teach users to think in events without forcing repetitive manual naming for simple cases.

Simple generated examples:

```text
Action: Create Offer
Event: Offer Created
Receipt: Create Offer Completed
```

Custom business actions should name their Events explicitly:

```text
Action: Accept Offer Into Sales Order
Event: Offer Accepted Into Sales Order
```

Failures in the execution layer are not Resource Events. They are recorded in execution journals. If a failure is a business fact, it must be explicitly modelled as a domain Event.
