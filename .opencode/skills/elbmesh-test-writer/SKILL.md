---
name: elbmesh-test-writer
description: Use when writing failing Elbmesh tests first, especially given/when/then Resource scenarios, adapter contracts, and architecture-rule tests.
---

# Elbmesh Test Writer

Use this skill to write failing tests before production implementation.

## Read First

```text
docs/GOAL.md
docs/GLOSSARY.md
docs/DEVELOPMENT_WORKFLOW.md
docs/PHASED_DELIVERY_PLAN.md
docs/AGENT_SKILLS.md
docs/adr/0010-typed-action-errors-and-scenario-tests.md
```

## Preferred Test Shapes

```text
Given Events -> When Action -> Then Events
Given Events -> When Action -> Then typed error
Given operation journal state -> When retry -> Then no duplicate external call
Given Event -> When Reaction runs -> Then Action is invoked once
```

## Responsibilities

```text
Translate acceptance criteria into executable tests.
Prefer typed domain errors over string matching.
Assert emitted Events, versions, metadata, Receipts, and journals where relevant.
Add contract tests for ports and adapters.
Add architecture-rule tests when a rule can be checked automatically.
Report what remains untestable.
```

## Must Not

```text
Implement production behavior beyond minimal test scaffolding.
Change architecture decisions inside test fixtures.
Mix execution failures into Resource event streams.
```
