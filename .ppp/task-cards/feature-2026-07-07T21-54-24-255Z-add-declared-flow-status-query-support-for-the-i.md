# PPP Feature Task Card

## Goal

Add declared flow_status query support for the in-memory Elbmesh reference flow.

## Context

The reference flow now produces a derived flow_status view through projections. This feature should add the smallest useful declared Query capability so the final Offer -> Invoice flow status can be read through query semantics over ViewStore rather than direct ad hoc ViewStore access or EventStore scanning.

## Inputs

```json
[
  "Use existing ViewStore and flow_status projection behavior",
  "Add or extend manifest query declarations/validation as needed",
  "Keep queries as read-side access over Views only",
  "Preserve Resource replay from Resource Events only",
  "Avoid NATS, Restate, CLI, external operations, or broad infrastructure work"
]
```

## Candidate Paths

- crates/elbmesh-core/src/manifest.rs
- crates/elbmesh-core/src/view_store.rs
- crates/elbmesh-core/src/lib.rs
- crates/elbmesh-core/src/query.rs
- crates/elbmesh-core/tests/architecture_manifest.rs
- crates/elbmesh-core/tests/view_store.rs
- crates/elbmesh-core/tests/query_runtime.rs
- crates/elbmesh-core/tests/reference_flow.rs

## Acceptance Criteria

- Reference-flow manifest declares at least one query against the flow_status view
- Manifest validation rejects a query targeting an undeclared view
- Query execution can read an existing flow_status document by offer id
- Query execution can list flow_status documents through the declared all index if included in scope
- Query results come only from ViewStore, not Resource replay, Reaction journals, or EventStore scanning
- Missing view documents return a typed not-found/empty result behavior
- Existing reference-flow reaction and projection tests continue to pass
- No NATS, Restate, external operation, CLI, or broad infrastructure implementation is introduced

## Non-Goals

- Do not implement NATS-backed query storage
- Do not add CLI or generated query bindings
- Do not scan EventStore or journals to answer queries
- Do not add external operations or Operation Journal behavior
- Do not refactor the runtime broadly beyond a minimal query slice

## Verification Commands

- cargo test -p elbmesh-core query_runtime
- cargo test -p elbmesh-core architecture_manifest
- cargo test -p elbmesh-core reference_flow
- cargo fmt --check
- ppp_validate_workflows

## PPP Workflow / Phases

- plan: Plan -> task.elbmesh-plan-implementation-slice
- test: Test -> task.elbmesh-write-failing-tests (depends on: plan)
- implement: Implement -> task.elbmesh-implement-runtime-slice (depends on: test)
- architecture-check: Architecture Check -> task.elbmesh-check-architecture-boundaries (depends on: implement)
- review: Review -> task.elbmesh-review-change (depends on: architecture-check)