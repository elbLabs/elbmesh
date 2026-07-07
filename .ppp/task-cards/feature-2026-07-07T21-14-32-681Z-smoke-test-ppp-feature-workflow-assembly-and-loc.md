# PPP Feature Task Card

## Goal

Smoke test PPP feature workflow assembly and local task-card persistence after OpenCode reload.

## Context

This is an end-to-end MVP smoke test for intent-first PPP workflow routing. No application code should be changed.

## Inputs

```json
[
  "Use configured feature workflow from ppp.workflows.json",
  "Generate task-card Markdown",
  "Persist it locally under .ppp/task-cards"
]
```

## Candidate Paths

- ppp.workflows.json
- .opencode/plugins/ppp.ts
- .opencode/skills/ppp/SKILL.md
- .ppp/README.md

## Acceptance Criteria

- Workflow validation passes
- Feature work type is discoverable
- Workflow assembly returns five ordered phases
- Task card is persisted locally as Markdown
- No GitHub issue is created

## Non-Goals

- Do not modify runtime application code
- Do not run broad cargo tests
- Do not create PRs or GitHub issues

## Verification Commands

- ppp_validate_workflows
- ppp_list_work_types
- ppp_assemble_workflow with persistTaskCard=true

## PPP Workflow / Phases

- plan: Plan -> task.elbmesh-plan-implementation-slice
- test: Test -> task.elbmesh-write-failing-tests (depends on: plan)
- implement: Implement -> task.elbmesh-implement-runtime-slice (depends on: test)
- architecture-check: Architecture Check -> task.elbmesh-check-architecture-boundaries (depends on: implement)
- review: Review -> task.elbmesh-review-change (depends on: architecture-check)