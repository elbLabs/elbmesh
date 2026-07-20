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

An Implementer-discovered accepted-test or fixture conflict with the task card or architecture must stop with the Implementer reporting it to the Orchestrator. Only after explicit human confirmation may a fresh Test Writer revise an authorized path to produce canonical semantic red followed by green; this route must not use immediately passing test-contract correction.

Immediately passing test-contract correction has one sole entry: a final Reviewer's path-specific accepted test blocker, followed by explicit human confirmation and a fresh Test Writer proving that non-test behavior is already correct and legitimate semantic red is impossible, so the corrected test would pass immediately. This final-Reviewer requirement does not make the Reviewer the sole entry to every accepted-test revision; Implementer-conflict revisions use the canonical semantic-red/green route above.

On that sole-entry correction route, revise only the authorized test or fixture paths and report old/new hashes, the exact focused passing proof, and why semantic red is impossible. Passing test-contract correction proof is not red proof and must never be called red.

## Verification

For canonical red, run the issue's exact focused `cargo test ...` command and confirm it fails for intended missing behavior, not compilation noise, infrastructure, or an unrelated regression. For an authorized test-contract correction, run the exact focused command and report its immediate pass as passing correction proof, never red proof.

## Architecture Rules Preserved

Tests must enforce Resource ownership, one-Resource Action targeting, one-stream Event ownership, deterministic replay, declared External Operations, Reaction-to-Action execution, rebuildable Views, journal/Event separation, and tests before implementation. Fixtures must not hide architecture decisions.
