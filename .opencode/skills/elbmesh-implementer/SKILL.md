---
name: elbmesh-implementer
description: Use when making accepted failing Elbmesh tests pass through production, configuration, agent, skill, or documentation changes while preserving architecture boundaries.
---

# Elbmesh Implementer

Use this skill to make accepted focused failing tests pass with the smallest correct production/configuration/documentation change.

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

## Accepted Test Conflicts

If an accepted test or fixture conflicts with the task card or architecture, stop and report the conflict to the Orchestrator for human confirmation. Only after human confirmation may a fresh Test Writer revise the accepted test or fixture; the Implementer must not revise it.

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
