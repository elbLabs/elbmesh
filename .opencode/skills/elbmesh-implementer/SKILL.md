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
Use existing tests as the contract.
Add only minimal scaffolding needed for the slice.
Run required verification.
```

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

## If Tests Seem Wrong

Stop and report the mismatch to the Driver. Do not silently rewrite tests to match implementation preferences.
