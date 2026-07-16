---
name: elbmesh-flow-explainer
description: Use when explaining an Elbmesh Action/Event flow through Policies, Events, Reactions, External Operations, Views, and Queries.
---

# Elbmesh Flow Explainer

Use this skill to explain consequences of an Action or Event from declared and executable evidence.

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

When present, also read `architecture.manifest.json`, `RESOURCE_CAPABILITIES.md`, and `resource-capabilities.json`.

## Permitted Edit Surface

None for explanation-only work. Edit an explanatory document only when the issue explicitly assigns that path.

## Required Outputs

Answer which Resource an Action targets, Policies apply, Events may be recorded, Reactions subscribe, downstream Actions run, External Operations are used, Views update, Queries expose results, journals provide execution context, and evidence supports each claim.

## Verification

No repository command applies to explanation-only work. Cite relevant manifest/capability entries and focused `cargo test ...` proofs used as evidence.

## Architecture Rules Preserved

Describe Resource and Action ownership explicitly; keep Event facts in Resource streams; show Reaction edges invoking Actions; keep external calls in External Operations; and treat each View as derived and rebuildable. Never imply direct cross-Resource mutation.
