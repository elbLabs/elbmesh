# Phase 4 Reference Flow Checkpoint

This checkpoint covers the Phase 3 manifest skeleton plus the Phase 4 handwritten Offer-to-Invoice reference flow before Reaction work starts.

## Checkpoint Answers

| Question | Answer |
| --- | --- |
| Can a human understand the runtime or architecture flow? | Yes. The current flow is visible as typed Actions, Resource Events, and a manual `ArchitectureManifest` fixture. |
| Can behavior be demonstrated without reading source code? | Partially. `cargo test -p elbmesh-core --test reference_flow` demonstrates the success and rejection paths, and this document summarizes the flow. There is no CLI or generated diagram yet. |
| Do tests cover key success, rejection, failure, and recovery paths? | Phase 4 covers reference-flow success and domain rejection paths. Phase 1 and Phase 2 still cover core runtime failure and journal behavior. Reaction recovery is not implemented yet. |
| What debt or ambiguity should be resolved before the next phases? | Reaction input mapping, deterministic downstream Action IDs, and trace/journal visibility for reactions must be specified in Phase 5 issues. |
| Which future adapter/tool observations must match the logical model? | NATS, Restate, generated docs, and CLI output must preserve the current Action -> Event proof points and the Phase 5 Reaction graph without writing journal records into Resource Event streams. |

## Architecture Flow

```text
CreateOfferV1
  -> offer_created

AcceptOfferV1
  -> offer_accepted

CreateSalesOrderV1
  -> sales_order_created

CreateOrderConfirmationV1
  -> order_confirmation_created

CreateInvoiceV1
  -> invoice_created
```

The Phase 4 manifest fixture declares these Resources:

```text
offer
sales_order
order_confirmation
invoice
```

The manifest fixture intentionally leaves these sections empty until later phases:

```text
reactions
views
queries
external_operations
```

## Demonstration Run

Run the reference-flow proof directly:

```bash
cargo test -p elbmesh-core --test reference_flow
```

Expected result:

```text
14 reference-flow tests pass.
Create and duplicate-create behavior is proven for Sales Order, Order Confirmation, and Invoice.
Create, Accept, missing-offer, duplicate-create, and already-accepted behavior is proven for Offer.
The manual manifest validates and the architecture check report passes with no findings.
```

## Test Coverage Matrix

| Capability | Proof Test | Coverage |
| --- | --- | --- |
| Offer create success | `create_offer_emits_offer_created` | Appends `offer_created`. |
| Offer duplicate create rejection | `create_offer_twice_returns_typed_already_exists_error` | Returns `OfferError::AlreadyExists`; `then_error` asserts no new Resource Event. |
| Offer accept success | `accept_offer_after_create_emits_offer_accepted` | Replays `offer_created`, then appends `offer_accepted`. |
| Offer accept before create rejection | `accept_offer_before_create_returns_typed_missing_offer_error` | Returns `OfferError::MissingOffer`; no new Resource Event. |
| Offer accept twice rejection | `accept_offer_twice_returns_typed_already_accepted_error` | Returns `OfferError::AlreadyAccepted`; no new Resource Event. |
| Sales Order create success | `create_sales_order_emits_sales_order_created` | Appends `sales_order_created`. |
| Sales Order duplicate create rejection | `create_sales_order_twice_returns_typed_already_exists_error` | Returns `SalesOrderError::AlreadyExists`; no new Resource Event. |
| Order Confirmation create success | `create_order_confirmation_emits_order_confirmation_created` | Appends `order_confirmation_created`. |
| Order Confirmation duplicate create rejection | `create_order_confirmation_twice_returns_typed_already_exists_error` | Returns `OrderConfirmationError::AlreadyExists`; no new Resource Event. |
| Invoice create success | `create_invoice_emits_invoice_created` | Appends `invoice_created`. |
| Invoice duplicate create rejection | `create_invoice_twice_returns_typed_already_exists_error` | Returns `InvoiceError::AlreadyExists`; no new Resource Event. |
| Manifest validation | `reference_flow_manifest_validates_successfully` | Validates Resource ownership and declared Action/Event references. |
| Architecture check report | `reference_flow_architecture_check_report_passes` | Produces `passed` and no findings. |
| Manifest JSON names | `reference_flow_manifest_json_names_resources_actions_and_events` | Serializes expected Resource, Action, and Event type names. |

## Failure Mode Matrix

Phase 4 adds a reference-flow fixture, not new runtime failure behavior. The checkpoint therefore reuses Phase 1 and Phase 2 failure proofs and records which gaps remain before Phase 5.

| Failure Mode | Current Proof | Status Before Phase 5 |
| --- | --- | --- |
| Domain rejection appends no Resource Event | `then_error` scenarios in `reference_flow.rs`; executor rejection tests in `action_executor.rs` | Covered for Offer, Sales Order, Order Confirmation, and Invoice duplicate paths. |
| Handler runtime failure keeps Resource Event stream clean | `handler_runtime_failure_with_journal_keeps_resource_stream_clean` | Covered in core runtime; not repeated in Phase 4 fixture. |
| EventStore load failure is classified and journaled | `load_failure_with_journal_records_failed_event_store_classification` | Covered in core runtime; unchanged by Phase 4. |
| EventStore append failure is classified and journaled | `append_failure_with_journal_records_failed_event_store_classification` | Covered in core runtime; unchanged by Phase 4. |
| Replay/deserialization failure is classified as Resource failure | `replay_failure_with_journal_records_failed_resource_classification` | Covered in core runtime; unchanged by Phase 4. |
| Action payload serialization failure before `ActionCalled` | `action_called_serialization_failure_with_journal_records_failed_handler_runtime_classification` | Covered in core runtime; remains a trace gap case for future visualization. |
| Reaction execution failure | No Phase 4 proof | Deferred to Phase 5 because Reactions do not exist yet. |
| Reaction retry/idempotency failure | No Phase 4 proof | Deferred to Phase 5 issue cards for deterministic downstream Action IDs. |

## Technical Debt And Ambiguities

| Item | Risk | Next Phase Handling |
| --- | --- | --- |
| No Reaction runtime yet | The reference flow is a set of independently executable Actions, not an automatic workflow. | Phase 5 must add ReactionJournal and ReactionRuntime. |
| Manual manifest can drift | The fixture uses handwritten manifest definitions until generation exists. | Phase 9 should generate contracts/docs from the manifest. |
| No cross-Resource existence checks | `CreateSalesOrderV1`, `CreateOrderConfirmationV1`, and `CreateInvoiceV1` carry upstream IDs but do not query upstream Resources. | Phase 5 should connect upstream Events to downstream Actions through Reactions, not direct Resource reads. |
| Reference-flow tests do not inspect ActionJournal lanes | Phase 4 focuses Resource scenario ergonomics; journal behavior remains covered by Phase 2 tests. | Phase 5 trace tests should include Action -> Event -> Reaction -> Action lanes. |
| Architecture check report still wraps first validation error | Aggregate architecture findings are not implemented. | Defer unless Phase 5 manifest/reaction debugging requires aggregate findings. |

## Phase 5 Decision List

These constraints should shape the Phase 5 issue cards:

1. Reactions must call Actions; they must not mutate Resources directly.
2. Reaction outputs must be journaled separately from Resource Events.
3. Downstream Action IDs must be deterministic enough to support idempotent Reaction retries.
4. The first workflow edge should connect `offer_accepted` to `create_sales_order` through a Reaction, not by changing `Offer` behavior.
5. Reaction manifest definitions should extend the current manual reference-flow manifest without adding Views or infrastructure adapters.
6. The logical trace model should be able to show `Action -> Event -> Reaction -> Action` before NATS or Restate overlays exist.

## Gate Results

Checkpoint tests were run before this artifact was written:

```text
cargo test -p elbmesh-core --test reference_flow
cargo test --all
```

The required gates were run again after this artifact was written:

```text
cargo fmt --check: passed
cargo clippy --all-targets --all-features -- -D warnings: passed
cargo test --all: passed
```
