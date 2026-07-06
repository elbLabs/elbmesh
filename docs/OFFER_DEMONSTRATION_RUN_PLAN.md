# Offer Demonstration And Visualization Run Plan

This Phase 2.5 artifact defines how a human can inspect the current typed core and execution journal behavior before the project adds manifests, reference flows, NATS, Restate, generation, or CLI tooling.

The demo starts with the existing Offer test model because it already exercises success, rejection, runtime failure, replay, receipts, and journals.

## Goals

```text
Make the current runtime behavior visible without reading Rust source.
Show Resource Events and ActionJournal records as separate lanes.
Show receipt/error outcomes for each scenario.
Reuse current tests as proof points.
Create a path from in-memory visibility to later NATS/Restate overlays.
```

## Non-Goals

```text
No demo binary yet.
No CLI output yet.
No generated diagrams yet.
No NATS or Restate integration yet.
No production Rust changes in this issue.
```

## Demo Format

For each scenario, present five sections:

```text
1. Given: prior Resource Events and Action metadata.
2. When: Action payload submitted to ActionExecutor.
3. Resource Event Stream: before and after records.
4. Action Journal: ordered journal records for the Action.
5. Outcome: ActionReceipt or typed ExecutionError.
```

The first implementation can render this as Markdown tables. A later CLI can render the same model as text, JSON, or diagrams.

## Visualization Lanes

```text
Action Input        | CreateOfferV1 { offer_id, title }
Resource Stream     | offer/offer-1: offer_created@1, ...
Action Journal      | action_called -> action_completed
Receipt/Error       | Completed receipt or typed ExecutionError
Trace Gaps          | Missing terminal record, duplicate action, replay anomaly
Future Overlays     | NATS publish/ack, Restate retry, external operation attempt
```

## Scenario Matrix

| Scenario | Purpose | Current Proof Test | Expected Resource Events | Expected ActionJournal | Expected Outcome |
| --- | --- | --- | --- | --- | --- |
| Create Offer succeeds | Prove normal Action -> Event -> Receipt flow. | `executes_action_and_records_event` | `offer_created` appended. | Optional journal in journal-specific tests: `ActionCalled`, `ActionCompleted`. | `ActionReceipt::Completed` with emitted event summary. |
| Multi-event action succeeds | Prove `record_applied` updates handler-visible Resource state before later events. | `record_applied_multi_event_action_observes_updated_resource_state` | `offer_created`, `offer_title_measured` appended in order. | Future demo should show one Action with two emitted Resource Events. | Receipt `new_version=2`. |
| Existing Offer rejected | Prove domain rejection does not append Resource Events. | `rejected_action_with_journal_records_called_and_rejected_in_order` | No new Resource Events. | `ActionCalled`, `ActionRejected`. | `ExecutionError::Handler(HandlerError::Domain)`. |
| Handler runtime failure | Prove runtime errors are journaled as failed, not Resource Events. | `handler_runtime_failure_with_journal_keeps_resource_stream_clean` | No new Resource Events. | `ActionCalled`, `ActionFailed(HandlerRuntime)`. | Typed wrong-resource runtime error. |
| EventStore load failure | Prove storage load failure is visible as runtime failure. | `load_failure_with_journal_records_failed_event_store_classification` | No Resource Events appended. | `ActionCalled`, `ActionFailed(EventStore)`. | `ExecutionError::EventStore`. |
| Replay/deserialization failure | Prove malformed historical event fails replay. | `replay_failure_with_journal_records_failed_resource_classification` | Existing malformed historical event only. | `ActionCalled`, `ActionFailed(Resource)`. | `ExecutionError::Resource`. |
| EventStore append failure | Prove pending Resource Events are not mixed with journals. | `append_failure_with_journal_records_failed_event_store_classification` | Append attempt fails; Resource stream unchanged. | `ActionCalled`, `ActionFailed(EventStore)`. | `ExecutionError::EventStore`. |
| Action payload serialization failure | Prove failure can happen before `ActionCalled`. | `action_called_serialization_failure_with_journal_records_failed_handler_runtime_classification` | No Resource Events. | `ActionFailed(HandlerRuntime)` only. | `ExecutionError::Handler(Runtime(Serialization))`. |
| Failure journal append fails | Prove original runtime error remains caller-facing. | `failed_runtime_action_returns_typed_error_when_action_failed_journal_append_fails` | No Resource Events. | `ActionCalled`; missing `ActionFailed` is a trace gap. | Original typed runtime error. |

## Example Timeline: Successful Create Offer

```text
Given Resource Stream offer/offer-1:
  <empty>

When Action:
  create_offer { offer_id: "offer-1", title: "Initial offer" }

Action Journal:
  1. ActionCalled create_offer
  2. ActionCompleted new_version=1 emitted=[offer_created@1]

Resource Stream offer/offer-1:
  1. offer_created { offer_id: "offer-1", title: "Initial offer" }

Outcome:
  Completed receipt, previous_version=0, new_version=1
```

## Example Timeline: Domain Rejection

```text
Given Resource Stream offer/offer-1:
  1. offer_created { offer_id: "offer-1", title: "Initial offer" }

When Action:
  create_offer { offer_id: "offer-1", title: "Duplicate offer" }

Action Journal:
  1. ActionCalled create_offer
  2. ActionRejected failure_code=offer.already_exists

Resource Stream offer/offer-1:
  unchanged

Outcome:
  typed domain error OfferError::AlreadyExists
```

## Example Timeline: Runtime Failure

```text
Given Resource Stream offer/offer-runtime-clean:
  <empty>

When Action:
  record_wrong_offer_event { offer_id: "offer-runtime-clean", event_offer_id: "other" }

Action Journal:
  1. ActionCalled record_wrong_offer_event
  2. ActionFailed classification=handler_runtime

Resource Stream offer/offer-runtime-clean:
  unchanged

Outcome:
  typed wrong-resource runtime error
```

## Example Timeline: Replay Failure

```text
Given Resource Stream offer/offer-replay-failure:
  1. malformed offer_created { title: "missing required offer_id" }

When Action:
  create_offer { offer_id: "offer-replay-failure", title: "Replay failure" }

Action Journal:
  1. ActionCalled create_offer
  2. ActionFailed classification=resource

Resource Stream offer/offer-replay-failure:
  unchanged; no new Resource Events appended

Outcome:
  typed ResourceError::Deserialization for offer_created v1
```

## Example Timeline: Append Failure

```text
Given Resource Stream offer/offer-append-failure:
  <empty>

When Action:
  create_offer { offer_id: "offer-append-failure", title: "Append failure" }

Attempted Resource Events:
  offer_created { offer_id: "offer-append-failure", title: "Append failure" }

Persisted Resource Stream offer/offer-append-failure:
  unchanged because EventStore.append failed

Action Journal:
  1. ActionCalled create_offer
  2. ActionFailed classification=event_store

Outcome:
  typed EventStoreError::Other("append unavailable")
```

## Human Review Run

Until a CLI exists, the human review run is manual and test-backed:

```text
1. Run cargo test --all.
2. Inspect the scenario matrix above.
3. For each scenario, compare expected Resource Events, ActionJournal records, and outcome against the named test.
4. Review RUNTIME_DEBT_AND_FAILURE_MODES.md for known gaps before approving the next phase.
5. Review EXECUTION_TRACE_MODEL.md for how each scenario would become a logical trace.
```

The human does not need to read every source line. The named tests are proof anchors, and the timelines explain expected behavior.

## Evolution By Phase

| Phase | Demo Evolution |
| --- | --- |
| Phase 2.5 | In-memory Offer demo with manual Markdown timelines. |
| Phase 3 | Manifest fixture can describe which Resources, Actions, and Events appear in the demo. |
| Phase 4 | Reference flow expands from Offer to Sales Order, Order Confirmation, and Invoice. |
| Phase 5 | Demo becomes an Action -> Event -> Reaction -> Action workflow graph. |
| Phase 6 | Demo includes Views rebuilt from Resource Events. |
| Phase 7 | NATS overlays show subjects, streams, headers, and ack sequences for the same logical trace. |
| Phase 8 | Restate overlays show invocation, retry, and idempotency behavior for external operations. |
| Phase 9 | Generated docs explain demo capabilities from the manifest. |
| Phase 10 | CLI can run `explain-flow` or equivalent for the demo. |

## Future Test/Visualization Work

The next implementation issue for visibility should not start by adding a CLI. It should start with a small trace projection test:

```text
Given an in-memory ActionJournal and Resource Event stream
When a trace is assembled for one action_id
Then the logical timeline matches the expected scenario
And Resource Events remain separate from journal records
```

Recommended first trace tests:

```text
successful Create Offer trace
domain rejection trace
handler runtime failure trace
serialization failure trace with missing ActionCalled gap
partial commit gap once ActionCompleted failure semantics are decided
```

## Open Questions

```text
Should demo output include payloads by default, or only metadata summaries?
Should failed append attempts show pending event payloads, or only event metadata/type summaries?
Should human review runs be stored as generated markdown artifacts, or only produced by future CLI commands?
What is the first acceptable visual format: table, timeline text, Mermaid, or CLI JSON?
```
