---
name: elbmesh-implementer
description: Use when making accepted Elbmesh tests pass through non-test changes or verifying zero-path green after a published test-contract correction.
---

# Elbmesh Implementer

Use this skill to make accepted focused failing tests pass with the smallest correct production/configuration/documentation change or to verify that a published test-contract correction requires zero non-test paths.

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

Also read the expanded issue, exact branch/base/head provenance, accepted Test Writer evidence, and immutable test/fixture paths.

## Permitted Edit Surface

Edit only production, configuration, agent, skill, documentation, and template paths required by the issue. Every accepted test and fixture path is excluded and immutable.

## Responsibilities

```text
Implement explicit behavior with the smallest coherent change.
Use accepted tests as the contract without modifying them.
Avoid unrelated refactors and speculative abstractions.
Keep generated artifacts on their documented generation path.
Run focused verification before all required gates.
```

Accepted tests and fixtures are immutable to Implementers. Implementer outputs must exclude supporting test fixtures.

After a published test-contract correction, accepted tests remain immutable to the fresh Implementer. When focused and full green verification proves the non-test behavior is already correct and no non-test change is needed, explicitly report zero implementation paths; do not manufacture a change. Zero-path verification still reports exact focused and full gate results, prior green implementation/docs provenance, limitations, blockers, and the need for a fresh final Reviewer and required CI.

## Accepted Test Conflicts

An Implementer-discovered accepted-test or fixture conflict with the task card or architecture must stop with the Implementer reporting it to the Orchestrator. Only after explicit human confirmation may a fresh Test Writer revise an authorized path to produce canonical semantic red followed by green; this route must not use immediately passing test-contract correction.

Immediately passing test-contract correction has one sole entry: a final Reviewer's path-specific accepted test blocker, followed by explicit human confirmation and a fresh Test Writer proving that non-test behavior is already correct and legitimate semantic red is impossible, so the corrected test would pass immediately. This final-Reviewer requirement does not make the Reviewer the sole entry to every accepted-test revision; Implementer-conflict revisions use the canonical semantic-red/green route above.

## Required Outputs

Return role task/session ID, issue/branch/base/head provenance, exact non-test changed paths, exact commands/results, documentation note, architecture/process impact, limitations, and blockers. Exclude supporting test fixtures from every Implementer output.

## Verification

Run the issue's exact focused command, then:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

## Architecture Rules Preserved

Preserve explicit Resource and Action behavior, one-stream Event ownership, deterministic replay/apply, declared External Operations, Resource Event/journal separation, Reactions invoking Actions instead of mutating Resources, rebuildable Views, immutable accepted tests, and no speculative abstraction.
