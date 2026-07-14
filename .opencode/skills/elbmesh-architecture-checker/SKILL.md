---
name: elbmesh-architecture-checker
description: Use when checking Elbmesh implementation against architecture rules before completion, especially Resource, Action, Event, Reaction, External Operation, and View boundaries.
---

# Elbmesh Architecture Checker

Use this skill to verify a change against Elbmesh architecture rules.

## Read First

```text
docs/GOAL.md
docs/GLOSSARY.md
docs/DEVELOPMENT_WORKFLOW.md
docs/HUMAN_DECISION_LOOP.md
docs/PHASED_DELIVERY_PLAN.md
docs/AGENT_SKILLS.md
docs/IMPLEMENTATION_PLAN.md
docs/adr/
```

## Checks

```text
Every Action targets exactly one Resource.
Every Event belongs to exactly one Resource.
Every Action version has exactly one registered handler.
Every Event version has explicit Apply logic.
Resource replay uses stored Resource Events only.
Replay/apply code does not call external systems.
External HTTP calls appear only through declared External Operations.
Execution failures go to journals unless explicitly modelled as domain Events.
Reactions invoke Actions rather than mutating Resources directly.
Views derive from Events and are rebuildable.
Schemas, capability docs, and skills are in sync with manifest/docs.
```

## Output

Return:

```text
Pass/fail summary
Findings ordered by severity
Missing automated checks
Docs or tests that must be updated
```
