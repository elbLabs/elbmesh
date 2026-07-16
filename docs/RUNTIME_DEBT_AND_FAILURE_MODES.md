# Runtime Debt And Failure Modes

This document is the Phase 2.5 review artifact for the typed core and execution journal runtime. It records known debt before the project moves into manifests, reference flows, NATS, Restate, generation, and tooling.

The intent is not to stop delivery. The intent is to keep deferred decisions visible so future phases resolve them deliberately.

## Scope

Reviewed areas:

```text
ActionExecutor
ActionContext
EventStore
InMemoryEventStore
ActionJournal
InMemoryActionJournal
ActionJournalRecord
ExecutionError
Resource/Action/Event/Apply/Handle traits
current action_executor and action_journal integration tests
```

Out of scope for this artifact:

```text
implementing fixes
NATS adapter implementation
Restate adapter implementation
OperationJournal implementation
ReactionJournal implementation
CLI or generated visualization output
```

## Severity Scale

```text
Critical: can create incorrect state, unrecoverable retries, or invalid distributed-runtime semantics.
High: likely to block or distort NATS, Restate, generation, or user-facing correctness.
Medium: important operational or maintainability gap, but not blocking the next local slice.
Low: cleanup, naming, or contract clarity issue that should not drive architecture by itself.
```

## Technical Debt Register

Debt categories in the register are intentionally mixed:

```text
Current-core debt: behavior already present in the in-memory runtime that needs a semantic decision.
Future-adapter debt: behavior that is acceptable locally but must be made explicit before NATS, Restate, or generation.
```

Rows D1, D2, D8, D9, D10, D13, and D14 are primarily current-core debt. Rows D3, D4, D5, D6, D7, D11, and D12 are primarily future-adapter or generation risks.

| ID | Severity | Debt | Evidence | Consequence | Detection Method | Resolution Phase |
| --- | --- | --- | --- | --- | --- | --- |
| D1 | Critical | Partial commit when `ActionCompleted` journaling fails after Resource Events append. | `ActionExecutor::execute`, `EventStore::append`, `ActionJournalRecord::ActionCompleted` | Resource state can change while caller receives `ExecutionError::ActionJournal`; retry can re-run without recovery semantics. | Add a failing `ActionCompleted` journal test after Resource Event append; later add Restate retry test. | Phase 2.5 decision, Phase 3/5 idempotency contract, Phase 8 Restate proof |
| D2 | Critical | No action idempotency or journal replay gate. | `ActionMetadata.action_id`, `ActionJournalStream::for_action`, `ActionExecutor::execute` | Duplicate delivery or Restate retry can append repeated `ActionCalled` records and re-run handlers. | Execute the same `action_id` twice and assert journal/resource outcomes. | Phase 2.5 decision, Phase 5 reaction retry IDs, Phase 8 Restate |
| D3 | High | EventStore load/version contract is underspecified. | `EventStore::load`, `RecordedEvent.sequence`, `InMemoryEventStore::append` | NATS contract can drift around ordering, gaps, duplicate records, wrong-stream records, and version semantics. | Reusable EventStore contract tests for gaps, duplicates, wrong stream, ordering, expected version, and empty append. | Phase 3 contract docs, Phase 7 NATS EventStore |
| D4 | High | Handlers are arbitrary async functions and can perform non-durable side effects. | `Handle<A>::handle` | Domain handlers can establish patterns that Restate later must prohibit or wrap. | Architecture review/static checks for forbidden direct external calls in handlers. | Phase 8 External Operations, architecture checks before then |
| D5 | High | Replay validation does not fully enforce stream/schema identity. | `apply_recorded_event`, `MessageMetadata`, `RecordedEvent.stream` | A future adapter returning malformed or misrouted events could let wrong data enter replay. | Replay tests for wrong stream, wrong resource metadata, wrong schema ID, duplicate sequence, and gaps. | Phase 3 manifest validation, Phase 7 adapter contract tests |
| D6 | High | NATS subject/key safety is not modeled. | `ResourceStream::key`, `Resource::RESOURCE_TYPE`, `Resource::Id` | Dots, wildcards, spaces, or reserved subject tokens can change NATS routing semantics. | Subject construction tests with reserved characters and explicit validation/escaping cases. | Phase 7 NATS adapters |
| D7 | High | `ActionJournalError` is too narrow for real adapters. | `ActionJournalError` | NATS-backed journals need connection, publish ack, serialization, timeout, authorization, and stream conflict errors. | Adapter contract tests that force each named adapter failure mode. | Phase 7 NATS-backed journals |
| D8 | Medium | Core `ActionFailed.failure_details` was semantically thin; issue #137 resolved the core mapping, while adapter-specific overlays remain future work. | `ActionJournalRecord::ActionFailed`, `action_failed_record`, focused EventStore/Resource/handler/External Operation tests | Operators now receive a stable failure code plus error type/variant and structured nested details without moving provider request/response diagnostics into Resource Events. | Focused tests assert stable structured details, original typed caller errors, and Resource Event separation; future adapter contracts must cover adapter-specific variants. | Core resolved in Phase 2 by #137; Phase 7/8 adapter overlays remain |
| D9 | Medium | Rejected, validation, and runtime failure semantics can be confused. | `ActionError`, `HandlerError`, `ActionRejected`, `ActionFailed` | Domain errors become `ActionRejected`, but `ActionError::Rejected` and `ActionError::Validation` are runtime paths today. | Tests that exercise `ActionError::Rejected` and `ActionError::Validation` journal classification. | Phase 2.5 vocabulary decision or Phase 3 validation naming |
| D10 | Medium | UUID/time generation is embedded in constructors. | `MessageMetadata::resource_event`, `ActionMetadata::for_actor`, `action_journal_metadata` | Deterministic replay tests, trace export, and adapter conformance tests need stable IDs/timestamps or injectable providers. | Snapshot-safe tests using fixed metadata or injectable clock/ID decisions. | Phase 3/7 test contracts |
| D11 | Medium | Journal record wire format is accidental serde default. | `ActionJournalRecord` derives `Serialize`/`Deserialize` without explicit tagging | Persisted NATS records may turn Rust enum serialization into an accidental public contract. | JSON shape tests for journal records before persistence adapters. | Phase 7 NATS journal adapter or Phase 9 generation |
| D12 | Low | Adapter contract tests are incomplete. | `action_journal.rs` tests are generic; EventStore tests are mostly concrete | NATS adapters may pass behavior not covered by reusable contracts. | Extract shared contract tests and run them against in-memory before NATS. | Phase 7 adapter test harness |
| D13 | Low | In-memory store poison handling differs between EventStore and ActionJournal. | `InMemoryEventStore`, `InMemoryActionJournal` | Reference adapters have inconsistent failure behavior. | Poisoning tests or named-error audit before adapter contract reuse. | Opportunistic before Phase 7 |
| D14 | Low | `ActionContext::current_version` means committed replay version, not pending version. | `ActionContext::current_version`, `record`, `record_applied` | The name can mislead multi-event handlers or future operation logic. | Documentation/tests that distinguish committed version from pending event count. | Phase 2.5 trace vocabulary or future docs |

## Failure Mode Matrix

| Scenario | Current Behavior | Visible Records | Risk | Detection Today | Future Requirement | Resolution Phase |
| --- | --- | --- | --- | --- | --- | --- |
| Successful action with Resource Events | Loads history, replays Resource, handler records events, appends events, returns completed receipt, journals `ActionCompleted`. | `ActionCalled`, Resource Events, `ActionCompleted` | If `ActionCompleted` journal write fails after append, state changed but caller sees journal error. | Executor tests cover success and receipt content. | Recovery/idempotency semantics before Restate retries. | Phase 2.5 decision, Phase 8 proof |
| Successful action with no Resource Events | Builds synthetic `AppendResult` without appending. | `ActionCalled`, `ActionCompleted` | No direct test currently proves no-event success. | Indirect through code only. | Add trace/test coverage when logical trace exists. | Phase 2.5 trace model |
| Domain rejection | Handler returns typed domain error; executor journals `ActionRejected`; no Resource Event append. | `ActionCalled`, `ActionRejected` | Clear today. Retry semantics still undefined. | Executor tests cover no Resource Events and typed error. | Idempotency should return stable rejection for same `action_id`. | Phase 2.5 decision, Phase 5/8 retries |
| Handler runtime failure | Executor journals `ActionFailed(HandlerRuntime)` best-effort with stable `ActionError` code, variant, and structured details, then returns the original typed runtime error. | `ActionCalled`, `ActionFailed` | Failure-journal append can still fail best-effort and leave an audit gap. | Executor tests cover wrong-resource and nested External Operation failures plus Resource Event separation. | Trace gaps should represent missing best-effort failure records. | Phase 2.5 trace model |
| Action payload serialization failure before `ActionCalled` | Executor cannot build `ActionCalled`; journals `ActionFailed(HandlerRuntime)` best-effort. | `ActionFailed` only | Trace must allow missing `ActionCalled` and explain why. | Executor regression test covers this. | Trace gaps should represent missing expected lifecycle entries. | Phase 2.5 trace model |
| EventStore load failure | Executor journals `ActionFailed(EventStore)` best-effort with stable EventStore code, variant, and structured details, then returns the original EventStore error. | `ActionCalled`, `ActionFailed` | Adapter-specific diagnostics still need contract coverage. | Executor test covers typed load failure and journal details. | NATS adapter must preserve the stable core envelope while exposing meaningful adapter details. | Phase 7 adapters |
| Resource replay/deserialization failure | Executor journals `ActionFailed(Resource)` best-effort with stable Resource code, variant, and structured details, then returns the original Resource error. | `ActionCalled`, `ActionFailed` | Replay validation and malformed event handling still need stronger contracts. | Executor test covers typed malformed historical event details. | Adapter contract must reject wrong-stream/schema/gap cases. | Phase 3/7 contracts |
| EventStore append failure | Pending Resource Events are not persisted; executor journals `ActionFailed(EventStore)` best-effort with stable EventStore diagnostics. | `ActionCalled`, `ActionFailed`; attempted pending events are not generally durable | Attempted event payloads remain outside the core failure envelope and require a safe overlay if operationally needed. | Executor tests capture the fake append batch and assert Resource Event separation. | Trace or adapter overlay should show attempted append safely without copying provider diagnostics into Resource Events. | Phase 2.5 trace model, Phase 7 overlays |
| `ActionFailed` journal append failure after runtime failure | Executor intentionally preserves original runtime error and ignores failure-journal error. | `ActionCalled` may exist; `ActionFailed` missing | Audit gap is possible by design. | Executor test covers handler-runtime branch. | Trace gaps should indicate missing failure journal record when observable. | Phase 2.5 trace model |
| `ActionCalled` journal append failure | Executor returns `ExecutionError::ActionJournal` before loading Resource or running handler. | None or partial adapter-dependent write | This is fail-fast and probably acceptable, but not directly tested. | Not directly tested. | Contract tests should decide if this remains required-journal behavior. | Phase 2.5 journal policy decision |
| `ActionRejected` journal append failure | Executor returns `ExecutionError::ActionJournal` instead of domain error. | `ActionCalled`; missing `ActionRejected` | Caller may lose typed domain rejection path. | Not directly tested. | Decide if rejection journaling is required or best-effort. | Phase 2.5 journal policy decision |
| `ActionCompleted` journal append failure | Resource Events already appended; executor returns `ExecutionError::ActionJournal`. | `ActionCalled`, Resource Events; missing `ActionCompleted` | Critical partial commit/retry ambiguity. | Observing journal test proves completed journal happens after append, not failure semantics. | Must be resolved before Restate retry semantics. | Phase 2.5 decision, Phase 8 proof |
| Duplicate delivery with same `action_id` | Executor does not load journal first; handler can run again. | Repeated `ActionCalled`; domain behavior decides outcome | Critical for NATS/Restate delivery and retries. | No direct duplicate-action test. | Add idempotency/recovery contract before distributed adapters. | Phase 2.5 decision, Phase 5/8 retries |
| EventStore returns out-of-order events | Executor sorts by `sequence` before replay. | No journal-specific marker | Out-of-order is tolerated, but gaps/duplicates are unspecified. | Tests prove out-of-order replay sorts. | Define EventStore load contract before NATS. | Phase 7 EventStore contract |
| EventStore returns gapped or duplicate sequences | Executor sorts by sequence and uses max sequence as previous version. | No journal-specific marker | Resource replay may hide adapter corruption or version ambiguity. | No direct gap/duplicate replay tests. | Define and enforce gap/duplicate behavior in EventStore contract tests. | Phase 7 EventStore contract |
| EventStore returns wrong-stream event | Replay may apply if type/version match because validation is incomplete. | No explicit marker | Potential replay corruption if adapter misroutes records. | Wrong-resource event recording tests cover new events, not stored wrong-stream replay. | Adapter contract and replay validation need explicit rules. | Phase 3/7 contracts |

## Phase Resolution Map

| Phase | Debt To Resolve Or Revisit |
| --- | --- |
| Phase 2.5 | Decide desired semantics for partial commit, idempotency, trace gaps, and required vs best-effort journals. |
| Phase 3 | Represent architecture rules and schema ownership in manifest fixtures and validation errors. |
| Phase 4 | Demonstrate current semantics with a generated-like reference flow humans can inspect. |
| Phase 5 | Define deterministic downstream action IDs and retry behavior for Reactions. |
| Phase 6 | Prove Views rebuild from Resource Events only, not journals. |
| Phase 7 | Make NATS adapters pass explicit EventStore and ActionJournal contracts. |
| Phase 8 | Prove Restate retry semantics do not duplicate external calls or Resource Events. |
| Phase 9 | Stabilize generated docs and persisted wire/schema shapes. |
| Phase 10 | Expose architecture and flow explanations through CLI/agentic tooling. |

## Immediate Decisions To Bring Forward

Before implementing distributed adapters, decide:

```text
Should ActionExecutor load ActionJournal at the start and short-circuit completed/rejected actions?
Is ActionCompleted journaling required, best-effort, or recoverable after Resource Event append?
What is the exact EventStore load contract for gaps, duplicates, ordering, and wrong-stream records?
Which journal writes are mandatory and which are best-effort?
```

The `ActionFailed.failure_details` decision is settled: it carries a stable failure code, error type/variant, and structured nested diagnostics. Provider request/response payloads remain in the OperationJournal or object storage, while future adapter overlays may add safe execution context.

## Current Recommendation

Do not start NATS or Restate implementation until the logical visibility model and idempotency/partial-commit decisions are documented.

Proceed next with:

```text
MR 2.5.3: logical ExecutionTrace model and test plan
MR 2.5.4: Offer demonstration and visualization run plan
Phase 3.1 only after the Phase 2.5 checkpoint artifacts are merged
```
