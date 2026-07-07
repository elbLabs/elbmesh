# PPP Feature Task Card

## Goal

Smoke test persisted task card

## Context

Verify PPP workflow persistence MVP

## Inputs

_Not provided._

## Candidate Paths

- .opencode/plugins/ppp.ts

## Acceptance Criteria

- ok=true
- task card file is written

## Non-Goals

_Not provided._

## Verification Commands

_Not provided._

## PPP Workflow / Phases

- plan: Plan -> task.elbmesh-plan-implementation-slice
- test: Test -> task.elbmesh-write-failing-tests (depends on: plan)
- implement: Implement -> task.elbmesh-implement-runtime-slice (depends on: test)
- architecture-check: Architecture Check -> task.elbmesh-check-architecture-boundaries (depends on: implement)
- review: Review -> task.elbmesh-review-change (depends on: architecture-check)