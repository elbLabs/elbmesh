---
name: elbmesh-mr-reviewer
description: Use only as a compatibility/manual deep-review skill outside the canonical delivery sequence.
---

# Elbmesh MR Reviewer

Use this optional compatibility/manual skill for a requested deep review of a phase-scoped PR/MR linked to a GitHub Issue. It is not an additional required delivery stage and does not own or report merge readiness. Only `elbmesh-reviewer` reports final PR merge readiness in the canonical flow, and a human retains all merge authority.

## Read First

```text
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
Review the PR/MR against its GitHub Issue and phase.
Check that tests exist for every changed behavior.
Check Rust quality and named error rules.
Run or inspect verification commands.
Reject unplanned behavior and unrelated refactors.
Request changes for architecture drift or missing docs.
Return deep-review observations to the active `elbmesh-reviewer` or human without making a readiness determination.
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

## Compatibility Rule

This skill must not merge and must not issue the final merge-readiness report. Flag failed quality gates, unplanned scope, and architecture drift as findings for the active `elbmesh-reviewer` or human. A human performs any merge only after `elbmesh-reviewer` reports readiness and required gates pass.

## Output

Return:

```text
findings ordered by severity
quality gate observations
deep-review findings for the active Reviewer or human
required follow-up tasks
residual risks
```
