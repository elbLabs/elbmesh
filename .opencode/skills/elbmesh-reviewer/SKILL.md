---
name: elbmesh-reviewer
description: Use when reviewing Elbmesh changes for bugs, architecture drift, missing tests, stale docs, and event-sourcing boundary violations.
---

# Elbmesh Reviewer

Use this skill to review completed or proposed changes. Use `elbmesh-mr-reviewer` when making the final merge decision for an MR.

## Read First

```text
docs/GOAL.md
docs/GLOSSARY.md
docs/DEVELOPMENT_WORKFLOW.md
docs/PHASED_DELIVERY_PLAN.md
docs/AGENT_SKILLS.md
docs/IMPLEMENTATION_PLAN.md
docs/adr/
```

## Review Focus

Report findings first, ordered by severity.

Check:

```text
Resource/Action/Event boundaries.
Typed errors and Receipts.
Expected version handling.
Event/journal separation.
External Operation idempotency.
Replay purity.
Reaction execution through Actions.
View rebuildability.
Missing tests.
Documentation drift.
Unplanned scope.
```

## Must Flag

```text
Action mutates multiple Resources directly.
Event belongs to no clear Resource stream.
External call is hidden in replay/apply or undeclared handler code.
Execution failure is stored as a Resource Event without domain modelling.
Generated docs or skills drift from canonical docs/manifest.
```
