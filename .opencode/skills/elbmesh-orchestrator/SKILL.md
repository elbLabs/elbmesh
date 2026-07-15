---
name: elbmesh-orchestrator
description: Use when coordinating dependency-ordered Elbmesh GitHub Issues, fresh role sessions, PR publication, evidence, and merge readiness.
---

# Elbmesh Orchestrator

Use this skill to coordinate dependency-ordered GitHub Issue and pull request delivery while remaining shell-free and non-editing.

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

Also read the expanded issue, explicit dependencies, role reports, branch/pull-request provenance, and CI evidence supplied by delegated roles.

## Permitted Edit Surface

None. Edit and Bash remain denied. Delegate Git/GitHub publication and automatic status changes to the Publisher.

## Delivery Sequence

```text
Fresh Test Writer produces accepted red proof before implementation.
Fresh Publisher creates the branch, separate test-only commit, push, linked draft pull request, append-only red evidence, and sets/keeps status:implementation.
Fresh Implementer preserves immutable accepted tests and produces green proof.
Fresh Publisher creates/pushes the separate implementation/docs commit and appends green evidence.
Fresh elbmesh-reviewer performs the final agent review and reports merge readiness or blockers.
Fresh Publisher verifies no-blocker evidence and required CI, appends readiness evidence, marks ready, and changes the issue to status:review.
Human performs final review and merge.
```

Rework repeats Implementer, Publisher, and Reviewer handoffs with fresh sessions and new append-only evidence.

## Required Outputs

Return issue/dependency/capability context, every role task/session ID, branch/base/head and PR provenance, accepted immutable paths, evidence links, gate/blocker state, publication state, residual risks, and next unblocked issue.

## Verification

No repository command applies to the shell-free Orchestrator. Require exact focused/gate/inspection commands and results from Test Writer, Implementer, Publisher, Reviewer, and CI evidence before advancing handoffs.

## Architecture Rules Preserved

Preserve Resource/Action/Event boundaries, deterministic replay, declared External Operations, Reaction-to-Action flow, rebuildable Views, tests before implementation, immutable accepted tests, separate red and green commits, final Reviewer, append-only evidence, and human-only merge. Use `docs/HUMAN_DECISION_LOOP.md` for genuine semantic conflicts, not routine labels.
