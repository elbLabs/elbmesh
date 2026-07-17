---
name: elbmesh-doc-maintainer
description: Use when updating Elbmesh ADRs, glossary, workflow docs, implementation plan, agent skills, or generated-doc synchronization rules.
---

# Elbmesh Doc Maintainer

Use this skill to keep documentation aligned with architecture, implementation, issue dependencies, and project-local agent contracts.

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

Also read `docs/README.md`, the expanded issue, affected templates/agents/skills, and generated capability artifacts when relevant.

## Permitted Edit Surface

Edit only documentation, ADR/index, issue/PR template, and project-local agent/skill/config-time paths assigned by the issue. Change generated artifacts only through their documented generator.

## Responsibilities

```text
Add or supersede ADRs without erasing historical decision text.
Update glossary vocabulary and implementation guidance together.
Keep DELIVERY_ROADMAP capability context aligned with explicit issue dependencies.
Keep workflow, harness, templates, agents, skills, and tests synchronized.
Keep canonical AGENT_SKILLS entries and concrete skills aligned.
Record open questions instead of hiding unresolved decisions.
State the OpenCode post-merge restart requirement for config-time changes.
```

## Required Outputs

Return exact changed paths, ADR supersession/index result, canonical/concrete synchronization result, generated-doc or no-generated-doc note, restart note, verification results, limitations, and blockers.

## Verification

Run the issue's exact focused documentation tests, then the required repository gates:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

## Architecture Rules Preserved

Preserve Resource, Action, Event, Reaction, and View terminology and boundaries; deterministic replay; declared External Operations; Event/journal separation; historical ADR integrity; and synchronized manifest-derived docs. Use `docs/HUMAN_DECISION_LOOP.md` for genuine semantic conflicts.
