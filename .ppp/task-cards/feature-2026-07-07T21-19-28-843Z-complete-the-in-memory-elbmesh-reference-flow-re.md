# PPP Feature Task Card

## Goal

Complete the in-memory Elbmesh reference flow reactions from accepted offer through invoice creation.

## Context

Current reference flow partially wires OfferAccepted to CreateSalesOrder. The feature should complete the documented first reference flow: Offer Accepted -> Create Sales Order -> Sales Order Created -> Create Order Confirmation -> Order Confirmation Created -> Create Invoice -> Invoice Created. Keep it local, typed, in-memory, and test-driven.

## Inputs

```json
[
  "Use existing reference_flow.rs domain concepts where possible",
  "Preserve explicit Action/Event/Reaction boundaries",
  "Keep reaction journals separate from Resource Event streams",
  "Avoid NATS, Restate, CLI, external operation, or broad infrastructure work"
]
```

## Candidate Paths

- crates/elbmesh-core/tests/reference_flow.rs
- crates/elbmesh-core/src/reaction.rs
- crates/elbmesh-core/src/projection.rs
- crates/elbmesh-core/src/manifest.rs

## Acceptance Criteria

- Add typed reaction from SalesOrderCreatedV1 to CreateOrderConfirmationV1
- Add typed reaction from OrderConfirmationCreatedV1 to CreateInvoiceV1
- Reference manifest includes all three reaction edges
- Dispatching SalesOrderCreatedV1 creates one OrderConfirmationCreatedV1
- Dispatching OrderConfirmationCreatedV1 creates one InvoiceCreatedV1
- Full reference flow executes from offer creation/acceptance through invoice creation
- Reaction journal records remain separate from Resource Event streams
- flow_status view reaches final invoice_created state

## Non-Goals

- Do not add NATS or Restate integration
- Do not implement external operation journaling
- Do not add CLI or generated artifacts
- Do not refactor core runtime unless tests expose a small necessary gap

## Verification Commands

- cargo test -p elbmesh-core reference_flow
- cargo test -p elbmesh-core architecture_manifest
- ppp_validate_workflows

## PPP Workflow / Phases

- plan: Plan -> task.elbmesh-plan-implementation-slice
- test: Test -> task.elbmesh-write-failing-tests (depends on: plan)
- implement: Implement -> task.elbmesh-implement-runtime-slice (depends on: test)
- architecture-check: Architecture Check -> task.elbmesh-check-architecture-boundaries (depends on: implement)
- review: Review -> task.elbmesh-review-change (depends on: architecture-check)