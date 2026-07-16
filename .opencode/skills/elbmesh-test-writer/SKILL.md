---
name: elbmesh-test-writer
description: Use when writing failing Elbmesh tests first, especially given/when/then Resource scenarios, adapter contracts, and architecture-rule tests.
---

# Elbmesh Test Writer

Use this skill to express an expanded issue's acceptance criteria as focused failing tests before production implementation.

## Read First

```text
docs/GOAL.md
docs/GLOSSARY.md
docs/DEVELOPMENT_WORKFLOW.md
docs/HUMAN_DECISION_LOOP.md
docs/DELIVERY_ROADMAP.md
docs/AGENT_SKILLS.md
docs/IMPLEMENTATION_PLAN.md
docs/adr/
```

Also read the expanded issue, dependencies, branch/base/head provenance, and relevant existing test helpers.

## Permitted Edit Surface

Edit only assigned `tests/**`, `fixtures/**`, `test-fixtures/**`, and nested equivalents. Do not edit production, workflow, agent, or generated behavior paths.

## Preferred Test Shapes

```text
Given Events -> When Action -> Then Events
Given Events -> When Action -> Then typed error
Given journal state -> When External Operation retry -> Then no duplicate provider call
Given Event -> When Reaction runs -> Then Action is invoked once
Given Events -> When projection rebuilds -> Then View matches
```

## Required Outputs

Return role task/session ID, issue/branch/base/head provenance, exact test/fixture paths, exact focused command/output, intended failure reason, untestable criteria, and blockers. Stop after accepted red proof; do not implement production behavior.

## Verification

Run the issue's exact focused `cargo test ...` command and confirm it fails for intended missing behavior, not compilation noise, infrastructure, or an unrelated regression.

## Architecture Rules Preserved

Tests must enforce Resource ownership, one-Resource Action targeting, one-stream Event ownership, deterministic replay, declared External Operations, Reaction-to-Action execution, rebuildable Views, journal/Event separation, and tests before implementation. Fixtures must not hide architecture decisions.
