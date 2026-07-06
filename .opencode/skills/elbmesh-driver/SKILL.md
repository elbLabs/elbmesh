---
name: elbmesh-driver
description: Use when planning an Elbmesh implementation slice, writing task cards, coordinating test-first work, or resolving architecture direction.
---

# Elbmesh Driver

Use this skill to define the next smallest useful implementation slice and coordinate the test-first loop within the active phase. Use `elbmesh-orchestrator` for phase and GitHub Issue/PR queue ownership.

## Read First

```text
docs/GOAL.md
docs/GLOSSARY.md
docs/DEVELOPMENT_WORKFLOW.md
docs/PHASED_DELIVERY_PLAN.md
docs/AGENT_SKILLS.md
docs/IMPLEMENTATION_PLAN.md
docs/adr/
```

## Responsibilities

```text
Define the smallest useful slice.
Write a task card with acceptance criteria.
Prefer creating/updating the GitHub Issue task card.
Keep the task card inside the active phase.
Identify relevant ADRs and glossary terms.
Require tests or a test plan before implementation starts.
Keep one implementation direction active.
Update docs or open questions when decisions change.
```

## Output

Produce a task card with:

```text
Goal
Architecture context
Acceptance criteria
Tests to write first
Non-goals
Documentation updates
Verification commands
```

## Preserve

```text
Resource = event-sourced aggregate root.
Action targets exactly one Resource.
Event belongs to exactly one Resource stream.
External calls require declared External Operations.
Tests come before implementation.
Docs are part of done.
No unplanned refactors.
```
