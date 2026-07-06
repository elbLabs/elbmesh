---
name: elbmesh-flow-explainer
description: Use when explaining an Elbmesh Action/Event flow through Policies, Events, Reactions, External Operations, Views, and Queries.
---

# Elbmesh Flow Explainer

Use this skill to explain consequences of an Action or Event.

## Read First

```text
docs/GOAL.md
docs/GLOSSARY.md
docs/AGENT_SKILLS.md
docs/PHASED_DELIVERY_PLAN.md
docs/IMPLEMENTATION_PLAN.md
docs/adr/
```

When generated artifacts exist, also read:

```text
RESOURCE_CAPABILITIES.md
resource-capabilities.json
architecture.manifest.json
```

## Explain

Answer:

```text
Which Resource does the Action target?
Which Policies apply?
Which Events may be recorded?
Which Reactions subscribe?
Which downstream Actions may run?
Which External Operations are used?
Which Views are updated?
Which Queries expose the result?
Which journals provide audit/recovery context?
```

## Preserve

Do not describe direct Action-to-Action mutation. Flows go through Events and Reactions:

```text
Action -> Event -> Reaction -> Action
```
