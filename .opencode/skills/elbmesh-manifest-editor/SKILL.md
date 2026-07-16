---
name: elbmesh-manifest-editor
description: Use when adding or changing Elbmesh Resources, Components, Actions, Events, Reactions, Views, Queries, Policies, External Operations, schemas, or manifest-driven bindings.
---

# Elbmesh Manifest Editor

Use this skill to change the architecture manifest or manifest-derived contract surface safely.

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

When present, also read `architecture.manifest.json`, `RESOURCE_CAPABILITIES.md`, `resource-capabilities.json`, schema sources, generator metadata, and the expanded issue.

## Permitted Edit Surface

Edit only issue-assigned manifest/schema sources and generated outputs through the documented generator. Do not change unrelated runtime behavior or hand-edit generated files.

## Responsibilities

```text
Update declared Resources, Components, Actions, Events, Reactions, Views, Queries, Policies, or External Operations.
Preserve schema IDs and versions.
Regenerate bindings and capability docs together when required.
Keep manifest hash and generator version synchronized.
Run focused validation and architecture checks.
```

## Required Outputs

Return exact manifest/schema/generated paths, generation command/result, validation and architecture-check results, docs impact, unresolved decisions, and blockers.

## Verification

Run the issue's focused manifest/generation commands, then:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

## Architecture Rules Preserved

Preserve one target Resource per Action, one owner Resource per Event, declared Reaction links, declared External Operations, declared View/Query indexes, deterministic replay, and synchronized generated contracts. Use `docs/HUMAN_DECISION_LOOP.md` for unresolved semantic ownership.
