# ADR 0013: Agent Skills Are First-Class Repo Artifacts

Status: Accepted

Date: 2026-07-04

## Context

Elbmesh is intended to be built and extended by humans and coding agents. If agents must infer the architecture from source files alone, the core product thesis fails.

The repository needs explicit agent skills that describe how to plan, test, implement, review, document, and explain framework changes.

These skills must stay aligned with the architecture manifest, ADRs, generated capability docs, and tests.

## Decision

Treat agent skills as first-class repository artifacts.

The repository should maintain a documented skill catalog that maps common agent tasks to repeatable workflows.

Initial skills:

```text
elbmesh-driver
elbmesh-test-writer
elbmesh-implementer
elbmesh-reviewer
elbmesh-doc-maintainer
elbmesh-architecture-checker
elbmesh-flow-explainer
elbmesh-manifest-editor
```

The canonical skill descriptions live in `docs/AGENT_SKILLS.md`.

Concrete project-local opencode skill files live under `.opencode/skills/`.

Until generation/checking exists, the docs and `.opencode/skills/*/SKILL.md` files must be updated together. Once generation exists, concrete skill files must be derived from or checked against the canonical docs and manifest.

## Consequences

Agent workflows become explicit and reviewable.

The Driver/Test/Implementation/Review loop has named skills that agents can follow.

Future generated agent instructions can include manifest hash and generator version, just like capability docs.

Documentation drift in agent skills is a framework defect because stale skills cause agents to make architectural mistakes.

## Rules

```text
Skills must reference relevant ADRs and glossary terms.
Skills must say which files agents are allowed or expected to edit.
Skills must list required verification commands.
Skills must preserve Resource/Action/Event/Reaction/View boundaries.
Skills must never instruct agents to bypass architecture checks.
Generated skills must not be manually edited once generation exists.
```

## Rejected Approach

Do not rely only on generic coding-agent prompts.

Generic prompts cannot reliably preserve Elbmesh-specific rules such as:

```text
Actions target one Resource.
Resource replay never calls external systems.
External calls require declared External Operations.
Execution failures go to journals, not Resource event streams.
Reactions call Actions instead of mutating Resources directly.
Views are rebuildable from Events.
```
