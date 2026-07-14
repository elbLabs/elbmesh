---
name: elbmesh-manifest-editor
description: Use when adding or changing Elbmesh Resources, Components, Actions, Events, Reactions, Views, Queries, Policies, External Operations, schemas, or manifest-driven bindings.
---

# Elbmesh Manifest Editor

Use this skill to safely change the architecture manifest or manifest-derived contract surface.

## Read First

```text
docs/GOAL.md
docs/GLOSSARY.md
docs/DEVELOPMENT_WORKFLOW.md
docs/HUMAN_DECISION_LOOP.md
docs/AGENT_SKILLS.md
docs/PHASED_DELIVERY_PLAN.md
docs/IMPLEMENTATION_PLAN.md
docs/adr/
```

When present, also read:

```text
architecture.manifest.json
RESOURCE_CAPABILITIES.md
resource-capabilities.json
```

## Responsibilities

```text
Update Resources, Components, Actions, Events, Reactions, Views, Queries, Policies, or External Operations.
Preserve schema IDs and versions.
Update generated binding stubs or document regeneration needed.
Update capability docs or mark them for regeneration.
Run architecture checks when available.
```

## Preserve

```text
One Action target Resource.
One Event owner Resource.
Declared External Operations for external calls.
View Queries use declared Views and indexes.
Generated docs and agent metadata stay in sync with the manifest.
```
