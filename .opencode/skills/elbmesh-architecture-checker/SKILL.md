---
name: elbmesh-architecture-checker
description: Use when checking Elbmesh implementation against architecture rules before completion, especially Resource, Action, Event, Reaction, External Operation, and View boundaries.
---

# Elbmesh Architecture Checker

Use this skill to verify an accepted change against Elbmesh architecture rules and its expanded GitHub Issue.

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

Also read the issue, accepted role evidence, manifest/capability artifacts when present, and the complete change range.

## Permitted Edit Surface

None while checking. Remain read-only and report required changes to the responsible role.

## Checks

```text
Every Action targets exactly one Resource.
Every Event belongs to exactly one Resource stream.
Every Action/Event version has explicit handling/apply behavior.
Resource replay uses stored Resource Events only and never calls external systems.
External calls use declared External Operations.
Execution failures remain in journals unless explicitly modelled as domain Events.
Reactions invoke Actions instead of mutating Resources directly.
Views derive from Events and remain rebuildable.
Schemas, generated capabilities, skills, and docs remain synchronized.
```

## Required Outputs

Return issue/branch/range provenance, findings ordered by severity, pass/fail summary, missing automated checks, required docs/tests, residual risks, and blocker state.

## Verification

Use the issue's focused command and, when the assigned role permits them, run exactly:

```bash
codehud . --diff origin/main
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

## Architecture Rules Preserved

Preserve Resource ownership, one-Resource Action targeting, one-stream Event ownership, Reaction-to-Action coordination, rebuildable View derivation, declared External Operations, deterministic replay, and journal/Event separation. Follow `docs/HUMAN_DECISION_LOOP.md` rather than guessing when a semantic conflict appears.
