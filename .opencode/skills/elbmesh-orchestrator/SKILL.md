---
name: elbmesh-orchestrator
description: Use when coordinating Elbmesh task-card setup, dependency-ordered delivery, accepted-test recovery decisions, fresh role sessions, publication, evidence, and merge readiness.
---

# Elbmesh Orchestrator

Use this skill to coordinate task-card issue/worktree setup and dependency-ordered pull request delivery while remaining shell-free and non-editing.

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

Also read the complete task-card payload or expanded issue, explicit dependencies, role reports, worktree/branch/pull-request provenance, and CI evidence supplied by delegated roles.

## Permitted Edit Surface

None. Edit and Bash remain denied. Delegate complete task-card issue creation and worktree setup to Operations. Delegate Git/GitHub delivery publication and automatic status changes to the Publisher.

## Delivery Sequence

```text
Fresh Operations creates/verifies a supplied complete task-card issue when needed.
Fresh Operations lists/fetches/adds a non-conflicting isolated issue worktree when requested.
Fresh Test Writer produces accepted red proof before implementation.
Fresh Publisher uses the verified issue branch/worktree or creates the branch when needed, then publishes the separate test-only commit, linked draft pull request, append-only red issue delta, current pull request body, and sets/keeps status:implementation.
Fresh Implementer preserves immutable accepted tests and produces green proof.
Fresh Publisher creates/pushes the separate implementation/docs commit, appends a green issue delta, and refreshes the current pull request body.
Fresh elbmesh-reviewer performs the final agent review, reports merge readiness or blockers, and returns the evidence-backed Human Review Briefing for the Publisher and current pull request body.
Fresh Publisher verifies no-blocker evidence and required CI, appends a readiness issue delta, places the Reviewer briefing at the top of the current pull request body, marks ready, and changes the issue to status:review.
Human performs final review and merge.
```

Rework repeats Implementer, Publisher, and Reviewer handoffs with fresh sessions, a new append-only issue delta, and a refreshed pull request body.

## Accepted-Test Correction Recovery

When a Reviewer reports an accepted test defect as a blocker, obtain explicit human confirmation before any revision, then spawn a fresh Test Writer to check whether valid semantic red exists. If semantic red exists, require the canonical red/green flow; only when non-test behavior is already correct and corrected tests pass immediately may the Test Writer report an explicitly named test-contract correction. Passing test-contract correction proof is never red proof.

After the Publisher publishes the separate correction commit and correction-stage issue delta, spawn a fresh Implementer to preserve accepted-test immutability and produce focused and full green verification, followed by a fresh Reviewer for the final complete-range review.

If that Implementer reports zero implementation paths, require no empty commit. Retain the earlier separate green implementation/docs commit as provenance. Zero implementation paths still require a fresh Reviewer for the final no-blocker report and required CI before readiness.

## Required Outputs

Return issue/dependency/capability context, every role task/session ID, worktree/branch/base/head and PR provenance, accepted immutable paths, evidence links, the accepted Human Review Briefing, gate/blocker state, publication state, residual risks, and next unblocked issue.

## Verification

No repository command applies to the shell-free Orchestrator. Require exact setup commands/results from Operations and focused/gate/inspection commands and results from Test Writer, Implementer, Publisher, Reviewer, and CI evidence before advancing handoffs.

## Architecture Rules Preserved

Preserve Resource/Action/Event boundaries, deterministic replay, declared External Operations, Reaction-to-Action flow, rebuildable Views, complete dependency-linked task cards, isolated issue worktrees, tests before implementation, immutable accepted tests, separate red and green commits, final Reviewer, append-only stage-specific issue evidence, a current pull request body, and human-only merge. Use `docs/HUMAN_DECISION_LOOP.md` for genuine semantic conflicts, not routine setup or labels.
