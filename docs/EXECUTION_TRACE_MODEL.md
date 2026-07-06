# Execution Trace Model

This document defines the Phase 2.5 logical visibility model. It is a test and visualization target, not a new source of truth.

The trace model explains what the framework believes happened during execution. NATS, Restate, CLI, and agent tooling can later add observations as overlays, but those overlays must not change Resource replay semantics.

## Goals

```text
Make Action execution explainable without reading source code.
Show Resource Events separately from execution journal records.
Represent success, rejection, runtime failure, and trace gaps.
Give future NATS and Restate adapters a conformance target.
Support human review and visualization checkpoints after every two phases.
```

## Non-Goals

```text
No production implementation in this phase.
No new authoritative event or journal stream.
No NATS or Restate adapter behavior.
No CLI output format yet.
No generated schema contract yet.
```

## Principle

The logical trace is derived from existing framework artifacts:

```text
ActionMetadata
ActionJournal records
Resource Event stream records
ActionReceipt or ExecutionError
```

It must not introduce a fifth execution truth beside Actions, Resource Events, Receipts, and Execution Journal Records.

## Conceptual Shape

```rust
pub struct ExecutionTrace {
    pub trace_id: String,
    pub root_action_id: String,
    pub scope: TraceScope,
    pub outcome: TraceOutcome,
    pub logical_entries: Vec<LogicalTraceEntry>,
    pub gaps: Vec<TraceGap>,
    pub overlays: Vec<TraceOverlay>,
}
```

The shape above is illustrative. It is not an implementation requirement until a future issue introduces code.

## Identity Rules

| Field | Meaning |
| --- | --- |
| `trace_id` | Usually `ActionMetadata.correlation_id`; can group future workflows. |
| `root_action_id` | The first Action in the trace from the caller perspective. |
| `action_id` | The concrete Action attempt represented by ActionJournal records. |
| `message_id` | The concrete Resource Event or journal record identifier. |
| `causation_id` | The immediate parent message/action that caused this record. |
| `actor_id` | The user/system actor carried by Action metadata. |

Current `ActionMetadata::for_actor` sets `correlation_id` and `causation_id` to the generated action id. Future Reactions can reuse the same `correlation_id` while creating downstream action ids.

## Logical Entries

Logical entries are framework-level facts derived from current records.

| Entry | Derived From | Meaning |
| --- | --- | --- |
| `ActionCalled` | `ActionJournalRecord::ActionCalled` | The executor accepted an Action attempt and captured its payload. |
| `ResourceReplay` | `EventStore::load` result or loaded `RecordedEvent`s | The Resource was rebuilt from historical Resource Events before handling. |
| `ResourceEventAppended` | `RecordedEvent` included in successful append/receipt | A domain fact was stored in the Resource Event stream. |
| `ActionCompleted` | `ActionJournalRecord::ActionCompleted` and `ActionReceipt` | The Action completed and returned a receipt. |
| `ActionRejected` | `ActionJournalRecord::ActionRejected` | The domain rejected the Action without appending Resource Events. |
| `ActionFailed` | `ActionJournalRecord::ActionFailed` | Runtime execution failed outside domain rejection. |

## Outcomes

```text
Completed
Rejected
Failed
Incomplete
Unknown
```

`Incomplete` is important because the current runtime can have gaps, for example Resource Events appended but `ActionCompleted` journaling failed.

## Trace Gaps

A trace gap is an explicit missing or inconsistent observation.

Examples:

```text
ActionFailed exists without ActionCalled because action payload serialization failed.
ActionCalled exists without terminal ActionCompleted/ActionRejected/ActionFailed.
Resource Events exist for an action without ActionCompleted.
ActionFailed was expected after runtime failure but failure journaling failed best-effort.
Loaded Resource Events have duplicate or gapped sequences.
```

Trace gaps are not Resource Events. They are visibility diagnostics.

## Overlays

Overlays are future tool or adapter observations.

| Overlay | Later Phase | Examples |
| --- | --- | --- |
| NATS | Phase 7 | publish subject, stream name, ack sequence, consumer delivery, headers |
| Restate | Phase 8 | invocation id, retry count, durable promise state, idempotency key |
| Operation | Phase 8 | external system, operation id, provider request id, idempotency key |
| Generation | Phase 9 | manifest hash, schema id, generated binding version |
| CLI/Agent | Phase 10 | explain-flow output, architecture-check report, demo command output |

Overlay rule:

```text
Core logical trace == stable framework truth.
Adapter/tool overlays == observed infrastructure details correlated to the logical trace.
```

## Example Timelines

Successful Action:

```text
ActionCalled(create_offer)
ResourceReplay(offer/offer-1, previous_version=0)
ResourceEventAppended(offer_created, sequence=1)
ActionCompleted(new_version=1)
```

Domain rejection:

```text
ActionCalled(create_offer)
ResourceReplay(offer/offer-1, previous_version=1)
ActionRejected(code=offer.already_exists)
```

Runtime failure:

```text
ActionCalled(record_wrong_offer_event)
ResourceReplay(offer/offer-1, previous_version=0)
ActionFailed(classification=handler_runtime)
```

Serialization failure before `ActionCalled`:

```text
ActionFailed(classification=handler_runtime)
TraceGap(missing_action_called, reason=action_payload_serialization_failed)
```

Partial commit gap:

```text
ActionCalled(create_offer)
ResourceReplay(offer/offer-1, previous_version=0)
ResourceEventAppended(offer_created, sequence=1)
TraceGap(missing_action_completed, reason=action_completed_journal_failed)
```

## Test Plan For Future Implementation

| Scenario | Existing Proof | Future Trace Assertion |
| --- | --- | --- |
| Successful single-event Action | `executes_action_and_records_event` | Trace contains `ActionCalled`, replay, one appended Resource Event, `ActionCompleted`. |
| Successful multi-event Action | `record_applied_multi_event_action_observes_updated_resource_state` | Trace preserves event append order and final receipt version. |
| Domain rejection | `rejected_action_with_journal_records_called_and_rejected_in_order` and `rejected_action_with_journal_appends_no_resource_events` | Trace contains `ActionCalled`, `ActionRejected`, no newly appended Resource Events. |
| EventStore load failure | load failure journal test | Trace contains `ActionCalled`, `ActionFailed(EventStore)`. |
| Resource replay failure | malformed replay test | Trace contains `ActionCalled`, `ActionFailed(Resource)`. |
| Handler runtime failure | `handler_runtime_failure_with_journal_keeps_resource_stream_clean` | Trace contains `ActionCalled`, `ActionFailed(HandlerRuntime)`, no newly appended Resource Events. |
| Action serialization failure | serialization failure journal test | Trace contains `ActionFailed(HandlerRuntime)` and a missing `ActionCalled` gap. |
| ActionFailed journal append failure | best-effort journal failure test | Trace contains original error outcome and a missing failure-journal gap when observable. |
| ActionCompleted journal append failure | not directly tested yet | Trace marks Resource Events present but terminal journal missing. |
| Duplicate same `action_id` | not directly tested yet | Trace marks duplicate/in-progress/completed replay behavior once idempotency is decided. |
| Gapped or duplicate event sequences | not directly tested yet | Trace marks replay anomaly or adapter contract violation. |

## Visualization Targets

Three views should be supported eventually:

```text
Timeline: ordered logical entries for one Action or correlation.
Swimlane: Resource Event stream, ActionJournal, and later NATS/Restate overlays.
Graph: Action -> Event -> Reaction -> Action workflow edges.
```

The first human-facing visualization can be plain text or Markdown tables. A CLI or generated diagram should wait until Phase 10.

## Implementation Guidance For Later

When this becomes code, start as a projection over existing stores:

```text
load ActionJournalStream(action_id)
load ResourceStream(resource_type, resource_id) when available
combine receipt/error from execution result when available
derive entries and gaps
render deterministic timeline/table
```

`ResourceReplay` can be reconstructed from `EventStore` when the Resource stream is known. If the stream is unavailable or failed to load, the trace should use `TraceGap` rather than inventing replay facts.

Do not make `ExecutionTrace` an input to replay. Resource replay must continue to use Resource Events only.

## Open Decisions

```text
Should trace assembly require access to both EventStore and ActionJournal?
Should traces expose payloads by default or only metadata summaries?
Should failure details be copied into ActionFailed, represented as TraceGap diagnostics, or left to overlays?
How should partial commits be classified before idempotency/recovery semantics are implemented?
```
