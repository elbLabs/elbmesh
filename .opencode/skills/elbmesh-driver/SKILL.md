---
name: elbmesh-driver
description: Use when planning an Elbmesh implementation slice, writing task cards, coordinating test-first work, or resolving architecture direction.
---

# Elbmesh Driver

Use this skill to define the smallest coherent, dependency-linked issue slice and test-first plan. Use `elbmesh-orchestrator` for role and publication coordination.

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

## Permitted Edit Surface

Edit only assigned planning documentation or proposed issue task-card text. Do not edit production code or accepted tests/fixtures.

## Responsibilities

```text
Confirm explicit Depends on and Blocks relationships.
Define one smallest coherent capability slice.
Write executable acceptance criteria and tests to write first.
Name non-goals, architecture rules, docs impact, and exact quality gates.
Keep one implementation direction active.
Stop for genuine semantic conflicts rather than hiding a decision in tests.
```

## Required Outputs

Produce a task card containing goal, dependency/capability context, acceptance criteria, first-test plan, non-goals, permitted edit surface, documentation updates, architecture impact, and exact verification commands.

## Verification

No repository command applies to planning-only output. The task card must name a focused `cargo test ...` command plus:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

## Architecture Rules Preserved

Preserve one-Resource Action targeting, one-stream Event ownership, deterministic Resource replay, declared External Operations, Reaction-to-Action flow, rebuildable View derivation, tests before implementation, and no speculative abstraction. Use `docs/HUMAN_DECISION_LOOP.md` when a semantic decision is unresolved.
