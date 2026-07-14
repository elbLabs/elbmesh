---
name: elbmesh-doc-maintainer
description: Use when updating Elbmesh ADRs, glossary, workflow docs, implementation plan, agent skills, or generated-doc synchronization rules.
---

# Elbmesh Doc Maintainer

Use this skill to keep documentation aligned with architecture and implementation.

## Read First

```text
docs/README.md
docs/GOAL.md
docs/GLOSSARY.md
docs/DEVELOPMENT_WORKFLOW.md
docs/HUMAN_DECISION_LOOP.md
docs/PHASED_DELIVERY_PLAN.md
docs/AGENT_SKILLS.md
docs/IMPLEMENTATION_PLAN.md
docs/adr/
```

## Responsibilities

```text
Add or update ADRs for architecture decisions.
Update the glossary when vocabulary changes.
Update the implementation plan when build order or scope changes.
Update the phased delivery plan when phases, MR sequencing, or quality gates change.
Update README indexes when docs are added.
Update agent skills when workflows change.
Record open questions instead of hiding unresolved decisions.
```

## Rules

```text
Documentation drift is a defect.
Generated docs must not be manually edited once generation exists.
Generated Markdown and JSON must share manifest hash and generator version.
Concrete skill files must stay aligned with docs/AGENT_SKILLS.md until generation exists.
```
