---
name: elbmesh-mr-reviewer
description: Use when reviewing and merging Elbmesh MRs after implementation, checking tests, named errors, Rust quality gates, docs, and architecture boundaries.
---

# Elbmesh MR Reviewer

Use this skill to review and merge phase-scoped PRs/MRs linked to GitHub Issues.

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

## Responsibilities

```text
Review the PR/MR against its GitHub Issue and phase.
Check that tests exist for every changed behavior.
Check Rust quality and named error rules.
Run or inspect verification commands.
Reject unplanned behavior and unrelated refactors.
Request changes for architecture drift or missing docs.
Merge only when all gates pass.
Record residual risks and follow-up tasks.
```

## Must Check

```text
MR matches active phase and GitHub Issue.
MR links or closes its GitHub Issue.
Tests were written before implementation.
All tests pass.
Formatting passes.
Clippy passes with warnings denied once configured.
Framework boundary errors are named error types.
Domain Action errors implement ActionFailure where relevant.
Resource/Action/Event/Reaction/View boundaries are preserved.
External calls are declared External Operations.
Docs and skills are updated when needed.
```

## Merge Rule

Do not merge if any quality gate fails, if scope is unplanned, or if architecture drift is unresolved.

## Output

Return:

```text
findings ordered by severity
quality gate status
merge or change-request decision
required follow-up tasks
residual risks
```
