---
name: elbmesh-test-writer
description: Use when writing failing Elbmesh tests first or, after a human-confirmed Reviewer blocker, making an authorized test-contract correction.
---

# Elbmesh Test Writer

Use this skill to express an expanded issue's acceptance criteria as focused failing tests before production implementation or to perform the narrow human-confirmed correction exception below.

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

For canonical red, return role task/session ID, issue/branch/base/head provenance, exact test/fixture paths, exact focused failing command/output, intended failure reason, untestable criteria, and blockers. For a correction, return the same provenance plus authorization, authorized paths, old/new hashes, exact passing proof, why semantic red is impossible, and blockers. Stop after the Test Writer report; do not implement production behavior.

## Human-Confirmed Test-Contract Correction

The only accepted-test revision exception begins with a Reviewer blocker and explicit human confirmation naming the authorized test or fixture paths. In a fresh Test Writer session, first determine whether missing non-test behavior can produce valid semantic red. If it can, preserve the canonical failing-red flow. If non-test behavior is already correct and corrected tests pass immediately, revise only the authorized test paths and report an explicitly named test-contract correction with old/new hashes, the exact focused passing proof, and why semantic red is impossible. Passing test-contract correction proof is not red proof and must never be called red.

## Verification

For canonical red, run the issue's exact focused `cargo test ...` command and confirm it fails for intended missing behavior, not compilation noise, infrastructure, or an unrelated regression. For an authorized test-contract correction, run the exact focused command and report its immediate pass as passing correction proof, never red proof.

## Architecture Rules Preserved

Tests must enforce Resource ownership, one-Resource Action targeting, one-stream Event ownership, deterministic replay, declared External Operations, Reaction-to-Action execution, rebuildable Views, journal/Event separation, and tests before implementation. Fixtures must not hide architecture decisions.
