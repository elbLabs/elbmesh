---
name: elbmesh-implementer
description: Use when implementing Elbmesh production code after failing tests exist, preserving Resource/Action/Event boundaries and explicit behavior.
---

# Elbmesh Implementer

Use this skill to make confirmed failing tests pass with the smallest correct production change.

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

## Responsibilities

```text
Implement behavior through explicit traits.
Keep the implementation slice-focused.
Keep the implementation inside the task card and active phase.
Preserve documented vocabulary and boundaries.
Use accepted tests as the contract.
Add only minimal scaffolding needed for the slice.
Run required verification.
```

Accepted tests and fixtures are immutable to Implementers. Implementer outputs must exclude supporting test fixtures.

## Preserve

```text
No domain behavior hidden behind macros.
Replay/apply code is deterministic.
Replay/apply code never calls external systems.
External calls happen only through declared External Operations.
Resource event streams contain only Resource Events.
Reactions invoke Actions rather than mutating Resource state directly.
No unplanned refactors.
No speculative abstraction.
```

## Accepted Test Conflicts

If an accepted test or fixture conflicts with the task card or architecture, stop and report the conflict to the Orchestrator for human confirmation. Only after human confirmation may a fresh Test Writer revise accepted tests or fixtures; the Implementer must not revise them.
