use async_trait::async_trait;

use elbmesh_core::{
    apply_recorded_event, Action, ActionContext, ActionDecision, ActionFailure, ActionJournal,
    ActionJournalRecord, ActionJournalStream, ActionMetadata, Apply, Event, EventStore,
    ExpectedVersion, Handle, HandlerError, InMemoryActionJournal, InMemoryEventStore,
    InMemoryReactionJournal, NewEvent, Reaction, ReactionDispatchError, ReactionDispatcher,
    ReactionJournal, ReactionJournalRecord, ReactionJournalStream, ReactionRuntime, RecordedEvent,
    Resource, ResourceError, ResourceStream, StreamType, TypedReactionHandler,
};

use serde::{Deserialize, Serialize};
use serde_json::json;

use std::{
    fmt,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

#[tokio::test]
async fn offer_accepted_reaction_executes_create_sales_order_through_action_executor() {
    let reaction_journal = InMemoryReactionJournal::new();
    let runtime = ReactionRuntime::new(InMemoryEventStore::new(), reaction_journal);
    let trigger = offer_accepted_recorded_event("offer-1");

    let receipt = runtime
        .execute_with_metadata(
            &trigger,
            &OfferAcceptedCreatesSalesOrder,
            reaction_action_metadata("create-sales-order-action-1"),
        )
        .await
        .expect("reaction should execute")
        .expect("matching event should trigger reaction");

    assert_eq!(
        receipt.action_receipt.action_id,
        "create-sales-order-action-1"
    );

    let sales_order_stream = ResourceStream::new("sales_order", "sales-order-for-offer-1");
    let sales_order_events = runtime
        .event_store()
        .load(&sales_order_stream)
        .await
        .expect("load sales order events");

    assert_eq!(sales_order_events.len(), 1);
    assert_eq!(
        sales_order_events[0].metadata.message_type,
        "sales_order_created"
    );
    assert_eq!(
        sales_order_events[0].metadata.stream_type,
        StreamType::Resource
    );
    assert_eq!(
        sales_order_events[0].metadata.action_id,
        "create-sales-order-action-1"
    );
    assert_eq!(
        sales_order_events[0].payload,
        json!({
            "sales_order_id": "sales-order-for-offer-1",
            "offer_id": "offer-1",
        })
    );
}

#[tokio::test]
async fn matching_reaction_records_triggered_and_completed_journal_records() {
    let reaction_journal = InMemoryReactionJournal::new();
    let runtime = ReactionRuntime::new(InMemoryEventStore::new(), reaction_journal.clone());
    let trigger = offer_accepted_recorded_event("offer-1");

    let receipt = runtime
        .execute_with_metadata(
            &trigger,
            &OfferAcceptedCreatesSalesOrder,
            reaction_action_metadata("create-sales-order-action-1"),
        )
        .await
        .expect("reaction should execute")
        .expect("matching event should trigger reaction");

    let reaction_stream = ReactionJournalStream::for_reaction(receipt.reaction_id);
    let records = reaction_journal
        .load(&reaction_stream)
        .await
        .expect("load reaction journal records");

    assert_eq!(records.len(), 2);
    match &records[0] {
        ReactionJournalRecord::ReactionTriggered {
            metadata,
            reaction_type,
            reaction_schema_id,
            reaction_schema_version,
            trigger_event_type,
            trigger_event_id,
            ..
        } => {
            assert_eq!(metadata.message_type, "reaction_triggered");
            assert_eq!(metadata.stream_type, StreamType::Reaction);
            assert_eq!(reaction_type, "offer_accepted_to_create_sales_order");
            assert_eq!(
                reaction_schema_id,
                "reaction.offer_accepted_to_create_sales_order.v1"
            );
            assert_eq!(*reaction_schema_version, 1);
            assert_eq!(trigger_event_type, "offer_accepted");
            assert_eq!(trigger_event_id, "offer-accepted-event-1");
        }
        other => panic!("expected ReactionTriggered record, got {other:?}"),
    }

    match &records[1] {
        ReactionJournalRecord::ReactionCompleted {
            metadata,
            triggered_action_id,
            ..
        } => {
            assert_eq!(metadata.message_type, "reaction_completed");
            assert_eq!(metadata.stream_type, StreamType::Reaction);
            assert_eq!(triggered_action_id, "create-sales-order-action-1");
        }
        other => panic!("expected ReactionCompleted record, got {other:?}"),
    }
}

#[tokio::test]
async fn default_reaction_execution_uses_deterministic_action_id() {
    let reaction_journal = InMemoryReactionJournal::new();
    let runtime = ReactionRuntime::new(InMemoryEventStore::new(), reaction_journal.clone());
    let trigger = offer_accepted_recorded_event("offer-1");
    let expected_action_id =
        deterministic_reaction_action_id::<OfferAcceptedCreatesSalesOrder>(&trigger);

    let receipt = runtime
        .execute(&trigger, &OfferAcceptedCreatesSalesOrder)
        .await
        .expect("reaction should execute")
        .expect("matching event should trigger reaction");

    assert_eq!(receipt.action_receipt.action_id, expected_action_id);
    assert_sales_order_created(
        runtime.event_store(),
        "sales-order-for-offer-1",
        &expected_action_id,
    )
    .await;
    assert_reaction_journal_records(
        &reaction_journal,
        &receipt.reaction_id,
        "offer_accepted_to_create_sales_order",
        &expected_action_id,
    )
    .await;
}

#[tokio::test]
async fn reaction_runtime_with_action_journal_records_triggered_action_lifecycle() {
    let reaction_journal = InMemoryReactionJournal::new();
    let action_journal = InMemoryActionJournal::new();
    let runtime = ReactionRuntime::new(InMemoryEventStore::new(), reaction_journal)
        .with_action_journal(action_journal.clone());
    let trigger = offer_accepted_recorded_event("offer-1");
    let expected_action_id =
        deterministic_reaction_action_id::<OfferAcceptedCreatesSalesOrder>(&trigger);

    let receipt = runtime
        .execute(&trigger, &OfferAcceptedCreatesSalesOrder)
        .await
        .expect("reaction should execute")
        .expect("matching event should trigger reaction");

    assert_eq!(receipt.action_receipt.action_id, expected_action_id);
    let action_records = action_journal
        .load(&ActionJournalStream::for_action(expected_action_id.clone()))
        .await
        .expect("load reaction-triggered action journal records");
    assert_eq!(action_records.len(), 2);
    match &action_records[0] {
        ActionJournalRecord::ActionCalled {
            metadata,
            action_type,
            ..
        } => {
            assert_eq!(metadata.message_type, "action_called");
            assert_eq!(metadata.stream_type, StreamType::Action);
            assert_eq!(metadata.action_id, expected_action_id);
            assert_eq!(action_type, "create_sales_order");
        }
        other => panic!("expected ActionCalled record, got {other:?}"),
    }
    match &action_records[1] {
        ActionJournalRecord::ActionCompleted { metadata, receipt } => {
            assert_eq!(metadata.message_type, "action_completed");
            assert_eq!(metadata.stream_type, StreamType::Action);
            assert_eq!(metadata.action_id, expected_action_id);
            assert_eq!(receipt.action_id, expected_action_id);
        }
        other => panic!("expected ActionCompleted record, got {other:?}"),
    }
}

#[tokio::test]
async fn dispatcher_with_deterministic_handler_uses_reaction_action_metadata() {
    let reaction_journal = InMemoryReactionJournal::new();
    let dispatcher = ReactionDispatcher::new(ReactionRuntime::new(
        InMemoryEventStore::new(),
        reaction_journal.clone(),
    ))
    .with_deterministic_handler(OfferAcceptedCreatesSalesOrder);
    let trigger = offer_accepted_recorded_event("offer-1");
    let expected_action_id =
        deterministic_reaction_action_id::<OfferAcceptedCreatesSalesOrder>(&trigger);

    let receipts = dispatcher
        .dispatch(&trigger)
        .await
        .expect("deterministic handler dispatch should succeed");

    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].action_receipt.action_id, expected_action_id);
    assert_sales_order_created(
        dispatcher.event_store(),
        "sales-order-for-offer-1",
        &expected_action_id,
    )
    .await;
    assert_reaction_journal_records(
        &reaction_journal,
        &receipts[0].reaction_id,
        "offer_accepted_to_create_sales_order",
        &expected_action_id,
    )
    .await;
}

#[tokio::test]
async fn same_reaction_retry_uses_same_deterministic_action_id() {
    let trigger = offer_accepted_recorded_event("offer-1");
    let first_runtime =
        ReactionRuntime::new(InMemoryEventStore::new(), InMemoryReactionJournal::new());
    let second_runtime =
        ReactionRuntime::new(InMemoryEventStore::new(), InMemoryReactionJournal::new());

    let first_receipt = first_runtime
        .execute(&trigger, &OfferAcceptedCreatesSalesOrder)
        .await
        .expect("first reaction should execute")
        .expect("matching event should trigger first reaction");
    let second_receipt = second_runtime
        .execute(&trigger, &OfferAcceptedCreatesSalesOrder)
        .await
        .expect("retry reaction should execute")
        .expect("matching event should trigger retry reaction");

    assert_eq!(
        first_receipt.action_receipt.action_id,
        second_receipt.action_receipt.action_id
    );
}

#[tokio::test]
async fn completed_reaction_retry_on_same_store_and_journal_is_idempotent() {
    let event_store = InMemoryEventStore::new();
    let reaction_journal = InMemoryReactionJournal::new();
    let runtime = ReactionRuntime::new(event_store.clone(), reaction_journal.clone());
    let trigger = offer_accepted_recorded_event("offer-1");

    let first_receipt = runtime
        .execute(&trigger, &OfferAcceptedCreatesSalesOrder)
        .await
        .expect("first reaction should execute")
        .expect("matching event should trigger first reaction");

    let retry = runtime
        .execute(&trigger, &OfferAcceptedCreatesSalesOrder)
        .await;

    assert!(
        retry.is_ok(),
        "completed reaction retry should not re-run the downstream action"
    );

    let sales_order_events = event_store
        .load(&ResourceStream::new(
            "sales_order",
            "sales-order-for-offer-1",
        ))
        .await
        .expect("load sales order events after retry");
    assert_eq!(sales_order_events.len(), 1);

    let records = reaction_journal
        .load(&ReactionJournalStream::for_reaction(
            first_receipt.reaction_id.clone(),
        ))
        .await
        .expect("load reaction journal records after retry");
    assert_eq!(records.len(), 2);
}

#[tokio::test]
async fn completed_reaction_dispatch_retry_on_same_store_and_journal_is_idempotent() {
    let event_store = InMemoryEventStore::new();
    let reaction_journal = InMemoryReactionJournal::new();
    let dispatcher = ReactionDispatcher::new(ReactionRuntime::new(
        event_store.clone(),
        reaction_journal.clone(),
    ))
    .with_handler(TypedReactionHandler::new(
        OfferAcceptedCreatesSalesOrder,
        ReactionRuntime::<InMemoryEventStore, InMemoryReactionJournal>::reaction_action_metadata::<
            OfferAcceptedCreatesSalesOrder,
        >,
    ));
    let trigger = offer_accepted_recorded_event("offer-1");

    let first_receipts = dispatcher
        .dispatch(&trigger)
        .await
        .expect("first dispatch should execute reaction");
    assert_eq!(first_receipts.len(), 1);

    let retry = dispatcher.dispatch(&trigger).await;

    assert!(
        retry.is_ok(),
        "completed reaction dispatch retry should not re-run the downstream action"
    );

    let sales_order_events = event_store
        .load(&ResourceStream::new(
            "sales_order",
            "sales-order-for-offer-1",
        ))
        .await
        .expect("load sales order events after dispatch retry");
    assert_eq!(sales_order_events.len(), 1);

    let records = reaction_journal
        .load(&ReactionJournalStream::for_reaction(
            first_receipts[0].reaction_id.clone(),
        ))
        .await
        .expect("load reaction journal records after dispatch retry");
    assert_eq!(records.len(), 2);
}

#[test]
fn deterministic_reaction_action_id_distinguishes_reaction_type_and_trigger_event() {
    let trigger = offer_accepted_recorded_event("offer-1");
    let mut later_trigger = offer_accepted_recorded_event("offer-2");
    later_trigger.metadata.message_id = "offer-accepted-event-2".to_string();

    let action_id = deterministic_reaction_action_id::<OfferAcceptedCreatesSalesOrder>(&trigger);
    let follow_up_action_id =
        deterministic_reaction_action_id::<OfferAcceptedCreatesFollowUpSalesOrder>(&trigger);
    let later_action_id =
        deterministic_reaction_action_id::<OfferAcceptedCreatesSalesOrder>(&later_trigger);

    assert_ne!(action_id, follow_up_action_id);
    assert_ne!(action_id, later_action_id);
}

#[test]
fn deterministic_reaction_action_id_uses_structured_identity_not_delimiters() {
    let mut left_trigger = offer_accepted_recorded_event("offer-1");
    left_trigger.metadata.message_id = "c".to_string();
    let mut right_trigger = offer_accepted_recorded_event("offer-1");
    right_trigger.metadata.message_id = "b:c".to_string();

    let left_action_id = deterministic_reaction_action_id::<ColonReactionType>(&left_trigger);
    let right_action_id = deterministic_reaction_action_id::<ColonTriggerEventId>(&right_trigger);

    assert_ne!(left_action_id, right_action_id);
}

#[tokio::test]
async fn reaction_rejects_trigger_when_stream_identity_disagrees_with_metadata() {
    let reaction_journal = InMemoryReactionJournal::new();
    let runtime = ReactionRuntime::new(InMemoryEventStore::new(), reaction_journal.clone());
    let mut trigger = offer_accepted_recorded_event("offer-1");
    trigger.stream = ResourceStream::new("offer", "offer-2");
    let reaction_id = ReactionRuntime::<InMemoryEventStore, InMemoryReactionJournal>::reaction_id::<
        OfferAcceptedCreatesSalesOrder,
    >(&trigger);

    let result = runtime
        .execute_with_metadata(
            &trigger,
            &OfferAcceptedCreatesSalesOrder,
            reaction_action_metadata("create-sales-order-action-1"),
        )
        .await;

    assert!(
        result.is_err(),
        "trigger with inconsistent RecordedEvent stream identity should be rejected"
    );
    assert!(runtime.event_store().all_events().is_empty());

    let records = reaction_journal
        .load(&ReactionJournalStream::for_reaction(reaction_id))
        .await
        .expect("load reaction journal records");
    assert!(records.is_empty());
}

#[tokio::test]
async fn reaction_rejects_trigger_when_payload_identity_disagrees_with_metadata() {
    let reaction_journal = InMemoryReactionJournal::new();
    let runtime = ReactionRuntime::new(InMemoryEventStore::new(), reaction_journal.clone());
    let mut trigger = offer_accepted_recorded_event("offer-1");
    trigger.payload = json!({ "offer_id": "offer-2" });
    let reaction_id = ReactionRuntime::<InMemoryEventStore, InMemoryReactionJournal>::reaction_id::<
        OfferAcceptedCreatesSalesOrder,
    >(&trigger);

    let result = runtime
        .execute_with_metadata(
            &trigger,
            &OfferAcceptedCreatesSalesOrder,
            reaction_action_metadata("create-sales-order-action-1"),
        )
        .await;

    assert!(
        result.is_err(),
        "trigger with inconsistent payload resource identity should be rejected"
    );
    assert!(runtime.event_store().all_events().is_empty());

    let records = reaction_journal
        .load(&ReactionJournalStream::for_reaction(reaction_id))
        .await
        .expect("load reaction journal records");
    assert!(records.is_empty());
}

#[tokio::test]
async fn non_matching_event_is_ignored_without_resource_events_or_reaction_journal_records() {
    let reaction_journal = InMemoryReactionJournal::new();
    let runtime = ReactionRuntime::new(InMemoryEventStore::new(), reaction_journal.clone());
    let trigger = offer_created_recorded_event("offer-1");
    let ignored_reaction_id =
        ReactionRuntime::<InMemoryEventStore, InMemoryReactionJournal>::reaction_id::<
            OfferAcceptedCreatesSalesOrder,
        >(&trigger);

    let receipt = runtime
        .execute_with_metadata(
            &trigger,
            &OfferAcceptedCreatesSalesOrder,
            reaction_action_metadata("create-sales-order-action-1"),
        )
        .await
        .expect("non-matching event should not fail");

    assert!(receipt.is_none());
    assert!(runtime.event_store().all_events().is_empty());

    let records = reaction_journal
        .load(&ReactionJournalStream::for_reaction(ignored_reaction_id))
        .await
        .expect("load reaction journal records");
    assert!(records.is_empty());
}

#[tokio::test]
async fn matching_event_type_from_non_resource_stream_is_ignored() {
    let reaction_journal = InMemoryReactionJournal::new();
    let runtime = ReactionRuntime::new(InMemoryEventStore::new(), reaction_journal.clone());
    let mut trigger = offer_accepted_recorded_event("offer-1");
    trigger.metadata.stream_type = StreamType::Reaction;
    let ignored_reaction_id =
        ReactionRuntime::<InMemoryEventStore, InMemoryReactionJournal>::reaction_id::<
            OfferAcceptedCreatesSalesOrder,
        >(&trigger);

    let receipt = runtime
        .execute_with_metadata(
            &trigger,
            &OfferAcceptedCreatesSalesOrder,
            reaction_action_metadata("create-sales-order-action-1"),
        )
        .await
        .expect("non-resource trigger should not fail");

    assert!(receipt.is_none());
    assert!(runtime.event_store().all_events().is_empty());

    let records = reaction_journal
        .load(&ReactionJournalStream::for_reaction(ignored_reaction_id))
        .await
        .expect("load reaction journal records");
    assert!(records.is_empty());
}

#[tokio::test]
async fn one_event_dispatches_to_multiple_matching_reaction_handlers() {
    let reaction_journal = InMemoryReactionJournal::new();
    let dispatcher = ReactionDispatcher::new(ReactionRuntime::new(
        InMemoryEventStore::new(),
        reaction_journal.clone(),
    ))
    .with_handler(TypedReactionHandler::new(
        OfferAcceptedCreatesSalesOrder,
        |_: &RecordedEvent| reaction_action_metadata("create-sales-order-action-1"),
    ))
    .with_handler(TypedReactionHandler::new(
        OfferAcceptedCreatesFollowUpSalesOrder,
        |_: &RecordedEvent| reaction_action_metadata("create-follow-up-sales-order-action-1"),
    ));
    let trigger = offer_accepted_recorded_event("offer-1");

    let receipts = dispatcher
        .dispatch(&trigger)
        .await
        .expect("dispatch should succeed");

    assert_eq!(receipts.len(), 2);
    assert_eq!(
        receipts[0].action_receipt.action_id,
        "create-sales-order-action-1"
    );
    assert_eq!(
        receipts[1].action_receipt.action_id,
        "create-follow-up-sales-order-action-1"
    );

    assert_sales_order_created(
        dispatcher.event_store(),
        "sales-order-for-offer-1",
        "create-sales-order-action-1",
    )
    .await;
    assert_sales_order_created(
        dispatcher.event_store(),
        "follow-up-sales-order-for-offer-1",
        "create-follow-up-sales-order-action-1",
    )
    .await;

    assert_reaction_journal_records(
        &reaction_journal,
        &receipts[0].reaction_id,
        "offer_accepted_to_create_sales_order",
        "create-sales-order-action-1",
    )
    .await;
    assert_reaction_journal_records(
        &reaction_journal,
        &receipts[1].reaction_id,
        "offer_accepted_to_create_follow_up_sales_order",
        "create-follow-up-sales-order-action-1",
    )
    .await;
}

#[tokio::test]
async fn multiple_reaction_dispatch_ignores_non_matching_event_without_side_effects() {
    let reaction_journal = InMemoryReactionJournal::new();
    let metadata_calls = Arc::new(AtomicUsize::new(0));
    let primary_metadata_calls = metadata_calls.clone();
    let follow_up_metadata_calls = metadata_calls.clone();
    let dispatcher = ReactionDispatcher::new(ReactionRuntime::new(
        InMemoryEventStore::new(),
        reaction_journal.clone(),
    ))
    .with_handler(TypedReactionHandler::new(
        OfferAcceptedCreatesSalesOrder,
        move |_: &RecordedEvent| {
            primary_metadata_calls.fetch_add(1, Ordering::SeqCst);
            reaction_action_metadata("create-sales-order-action-1")
        },
    ))
    .with_handler(TypedReactionHandler::new(
        OfferAcceptedCreatesFollowUpSalesOrder,
        move |_: &RecordedEvent| {
            follow_up_metadata_calls.fetch_add(1, Ordering::SeqCst);
            reaction_action_metadata("create-follow-up-sales-order-action-1")
        },
    ));
    let trigger = offer_created_recorded_event("offer-1");

    let receipts = dispatcher
        .dispatch(&trigger)
        .await
        .expect("dispatch should ignore non-matching events");

    assert!(receipts.is_empty());
    assert!(dispatcher.event_store().all_events().is_empty());
    assert_eq!(metadata_calls.load(Ordering::SeqCst), 0);

    let primary_reaction_id =
        ReactionRuntime::<InMemoryEventStore, InMemoryReactionJournal>::reaction_id::<
            OfferAcceptedCreatesSalesOrder,
        >(&trigger);
    let follow_up_reaction_id =
        ReactionRuntime::<InMemoryEventStore, InMemoryReactionJournal>::reaction_id::<
            OfferAcceptedCreatesFollowUpSalesOrder,
        >(&trigger);

    assert!(reaction_journal
        .load(&ReactionJournalStream::for_reaction(primary_reaction_id))
        .await
        .expect("load primary reaction journal")
        .is_empty());
    assert!(reaction_journal
        .load(&ReactionJournalStream::for_reaction(follow_up_reaction_id))
        .await
        .expect("load follow-up reaction journal")
        .is_empty());
}

#[tokio::test]
async fn multiple_reaction_dispatch_continues_after_one_handler_fails() {
    let event_store = InMemoryEventStore::new();
    append_existing_sales_order(&event_store, "sales-order-for-offer-1", "offer-1").await;
    let reaction_journal = InMemoryReactionJournal::new();
    let dispatcher =
        ReactionDispatcher::new(ReactionRuntime::new(event_store.clone(), reaction_journal))
            .with_handler(TypedReactionHandler::new(
                OfferAcceptedCreatesSalesOrder,
                |_: &RecordedEvent| reaction_action_metadata("create-sales-order-action-1"),
            ))
            .with_handler(TypedReactionHandler::new(
                OfferAcceptedCreatesFollowUpSalesOrder,
                |_: &RecordedEvent| {
                    reaction_action_metadata("create-follow-up-sales-order-action-1")
                },
            ));
    let trigger = offer_accepted_recorded_event("offer-1");

    let err = dispatcher
        .dispatch(&trigger)
        .await
        .expect_err("one handler should fail and dispatch should report it");

    match err {
        ReactionDispatchError::HandlerFailures { receipts, failures } => {
            assert_eq!(receipts.len(), 1);
            assert_eq!(
                receipts[0].action_receipt.action_id,
                "create-follow-up-sales-order-action-1"
            );
            assert_eq!(failures.len(), 1);
            assert_eq!(
                failures[0].reaction_type,
                "offer_accepted_to_create_sales_order"
            );
            assert_eq!(failures[0].failure_code, "sales_order.already_exists");
        }
    }

    assert_sales_order_created(
        &event_store,
        "follow-up-sales-order-for-offer-1",
        "create-follow-up-sales-order-action-1",
    )
    .await;
}

#[derive(Debug, Default, Clone)]
struct Offer;

impl Resource for Offer {
    type Id = String;

    const RESOURCE_TYPE: &'static str = "offer";

    fn apply_recorded(&mut self, event: &RecordedEvent) -> Result<(), ResourceError> {
        Err(ResourceError::UnsupportedEvent {
            resource_type: Self::RESOURCE_TYPE.to_string(),
            message_type: event.metadata.message_type.clone(),
            schema_version: event.metadata.schema_version,
        })
    }
}

#[derive(Debug, Default, Clone)]
struct SalesOrder {
    id: Option<String>,
    offer_id: Option<String>,
}

impl Resource for SalesOrder {
    type Id = String;

    const RESOURCE_TYPE: &'static str = "sales_order";

    fn apply_recorded(&mut self, event: &RecordedEvent) -> Result<(), ResourceError> {
        if apply_recorded_event::<Self, SalesOrderCreatedV1>(self, event)? {
            return Ok(());
        }

        Err(ResourceError::UnsupportedEvent {
            resource_type: Self::RESOURCE_TYPE.to_string(),
            message_type: event.metadata.message_type.clone(),
            schema_version: event.metadata.schema_version,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OfferAcceptedV1 {
    offer_id: String,
}

impl Event for OfferAcceptedV1 {
    type Resource = Offer;

    const EVENT_TYPE: &'static str = "offer_accepted";
    const SCHEMA_ID: &'static str = "event.offer_accepted.v1";
    const SCHEMA_VERSION: u32 = 1;

    fn resource_id(&self) -> <Self::Resource as Resource>::Id {
        self.offer_id.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateSalesOrderV1 {
    sales_order_id: String,
    offer_id: String,
}

impl Action for CreateSalesOrderV1 {
    type Resource = SalesOrder;

    const ACTION_TYPE: &'static str = "create_sales_order";
    const SCHEMA_ID: &'static str = "action.create_sales_order.v1";
    const SCHEMA_VERSION: u32 = 1;

    fn resource_id(&self) -> <Self::Resource as Resource>::Id {
        self.sales_order_id.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SalesOrderCreatedV1 {
    sales_order_id: String,
    offer_id: String,
}

impl Event for SalesOrderCreatedV1 {
    type Resource = SalesOrder;

    const EVENT_TYPE: &'static str = "sales_order_created";
    const SCHEMA_ID: &'static str = "event.sales_order_created.v1";
    const SCHEMA_VERSION: u32 = 1;

    fn resource_id(&self) -> <Self::Resource as Resource>::Id {
        self.sales_order_id.clone()
    }
}

impl Apply<SalesOrderCreatedV1> for SalesOrder {
    fn apply(&mut self, event: SalesOrderCreatedV1) -> Result<(), ResourceError> {
        self.id = Some(event.sales_order_id);
        self.offer_id = Some(event.offer_id);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SalesOrderError {
    AlreadyExists,
}

impl fmt::Display for SalesOrderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyExists => write!(f, "sales order already exists"),
        }
    }
}

impl ActionFailure for SalesOrderError {
    fn code(&self) -> &'static str {
        match self {
            Self::AlreadyExists => "sales_order.already_exists",
        }
    }
}

#[async_trait]
impl Handle<CreateSalesOrderV1> for SalesOrder {
    type Error = SalesOrderError;

    async fn handle(
        &mut self,
        action: CreateSalesOrderV1,
        ctx: &mut ActionContext<Self>,
    ) -> Result<ActionDecision, HandlerError<Self::Error>> {
        if self.id.is_some() {
            return Err(HandlerError::domain(SalesOrderError::AlreadyExists));
        }

        ctx.record_applied(
            self,
            SalesOrderCreatedV1 {
                sales_order_id: action.sales_order_id,
                offer_id: action.offer_id,
            },
        )?;

        Ok(ActionDecision::with_message("sales order created"))
    }
}

struct OfferAcceptedCreatesSalesOrder;

#[async_trait]
impl Reaction for OfferAcceptedCreatesSalesOrder {
    type Trigger = OfferAcceptedV1;
    type Resource = SalesOrder;
    type Action = CreateSalesOrderV1;

    const REACTION_TYPE: &'static str = "offer_accepted_to_create_sales_order";
    const SCHEMA_ID: &'static str = "reaction.offer_accepted_to_create_sales_order.v1";
    const SCHEMA_VERSION: u32 = 1;

    async fn react(&self, event: Self::Trigger) -> Self::Action {
        CreateSalesOrderV1 {
            sales_order_id: format!("sales-order-for-{}", event.offer_id),
            offer_id: event.offer_id,
        }
    }
}

struct OfferAcceptedCreatesFollowUpSalesOrder;

#[async_trait]
impl Reaction for OfferAcceptedCreatesFollowUpSalesOrder {
    type Trigger = OfferAcceptedV1;
    type Resource = SalesOrder;
    type Action = CreateSalesOrderV1;

    const REACTION_TYPE: &'static str = "offer_accepted_to_create_follow_up_sales_order";
    const SCHEMA_ID: &'static str = "reaction.offer_accepted_to_create_follow_up_sales_order.v1";
    const SCHEMA_VERSION: u32 = 1;

    async fn react(&self, event: Self::Trigger) -> Self::Action {
        CreateSalesOrderV1 {
            sales_order_id: format!("follow-up-sales-order-for-{}", event.offer_id),
            offer_id: event.offer_id,
        }
    }
}

struct ColonReactionType;

#[async_trait]
impl Reaction for ColonReactionType {
    type Trigger = OfferAcceptedV1;
    type Resource = SalesOrder;
    type Action = CreateSalesOrderV1;

    const REACTION_TYPE: &'static str = "a:b";
    const SCHEMA_ID: &'static str = "reaction.colon_reaction_type.v1";
    const SCHEMA_VERSION: u32 = 1;

    async fn react(&self, event: Self::Trigger) -> Self::Action {
        CreateSalesOrderV1 {
            sales_order_id: format!("colon-reaction-type-for-{}", event.offer_id),
            offer_id: event.offer_id,
        }
    }
}

struct ColonTriggerEventId;

#[async_trait]
impl Reaction for ColonTriggerEventId {
    type Trigger = OfferAcceptedV1;
    type Resource = SalesOrder;
    type Action = CreateSalesOrderV1;

    const REACTION_TYPE: &'static str = "a";
    const SCHEMA_ID: &'static str = "reaction.colon_trigger_event_id.v1";
    const SCHEMA_VERSION: u32 = 1;

    async fn react(&self, event: Self::Trigger) -> Self::Action {
        CreateSalesOrderV1 {
            sales_order_id: format!("colon-trigger-event-id-for-{}", event.offer_id),
            offer_id: event.offer_id,
        }
    }
}

async fn assert_sales_order_created(
    event_store: &InMemoryEventStore,
    sales_order_id: &str,
    action_id: &str,
) {
    let stream = ResourceStream::new("sales_order", sales_order_id);
    let events = event_store
        .load(&stream)
        .await
        .expect("load sales order events");

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].metadata.message_type, "sales_order_created");
    assert_eq!(events[0].metadata.action_id, action_id);
    assert_eq!(events[0].payload["sales_order_id"], sales_order_id);
}

async fn assert_reaction_journal_records(
    reaction_journal: &InMemoryReactionJournal,
    reaction_id: &str,
    reaction_type: &str,
    triggered_action_id: &str,
) {
    let records = reaction_journal
        .load(&ReactionJournalStream::for_reaction(reaction_id))
        .await
        .expect("load reaction journal records");

    assert_eq!(records.len(), 2);
    match &records[0] {
        ReactionJournalRecord::ReactionTriggered {
            reaction_type: actual_reaction_type,
            ..
        } => assert_eq!(actual_reaction_type, reaction_type),
        other => panic!("expected ReactionTriggered record, got {other:?}"),
    }

    match &records[1] {
        ReactionJournalRecord::ReactionCompleted {
            triggered_action_id: actual_action_id,
            ..
        } => assert_eq!(actual_action_id, triggered_action_id),
        other => panic!("expected ReactionCompleted record, got {other:?}"),
    }
}

fn offer_accepted_recorded_event(offer_id: &str) -> RecordedEvent {
    RecordedEvent {
        stream: ResourceStream::new("offer", offer_id),
        sequence: 1,
        metadata: resource_event_metadata(
            "offer-accepted-event-1",
            OfferAcceptedV1::EVENT_TYPE,
            OfferAcceptedV1::SCHEMA_ID,
            OfferAcceptedV1::SCHEMA_VERSION,
            offer_id,
        ),
        payload: json!({ "offer_id": offer_id }),
    }
}

fn offer_created_recorded_event(offer_id: &str) -> RecordedEvent {
    RecordedEvent {
        stream: ResourceStream::new("offer", offer_id),
        sequence: 1,
        metadata: resource_event_metadata(
            "offer-created-event-1",
            "offer_created",
            "event.offer_created.v1",
            1,
            offer_id,
        ),
        payload: json!({ "offer_id": offer_id, "title": "Initial offer" }),
    }
}

async fn append_existing_sales_order(
    event_store: &InMemoryEventStore,
    sales_order_id: &str,
    offer_id: &str,
) {
    let stream = ResourceStream::new("sales_order", sales_order_id);
    event_store
        .append(
            &stream,
            ExpectedVersion::NoStream,
            vec![NewEvent {
                metadata: sales_order_event_metadata(sales_order_id),
                payload: json!({
                    "sales_order_id": sales_order_id,
                    "offer_id": offer_id,
                }),
            }],
        )
        .await
        .expect("append existing sales order");
}

fn sales_order_event_metadata(sales_order_id: &str) -> elbmesh_core::MessageMetadata {
    elbmesh_core::MessageMetadata {
        message_id: format!("existing-sales-order-{sales_order_id}"),
        message_type: "sales_order_created".to_string(),
        message_version: 1,
        resource_type: "sales_order".to_string(),
        resource_id: sales_order_id.to_string(),
        stream_type: StreamType::Resource,
        correlation_id: "correlation-existing-sales-order".to_string(),
        causation_id: "existing-sales-order-action".to_string(),
        action_id: "existing-sales-order-action".to_string(),
        actor_id: "actor-123".to_string(),
        occurred_at: "2026-07-07T00:00:00Z".to_string(),
        schema_id: "event.sales_order_created.v1".to_string(),
        schema_version: 1,
    }
}

fn resource_event_metadata(
    message_id: &str,
    message_type: &str,
    schema_id: &str,
    schema_version: u32,
    offer_id: &str,
) -> elbmesh_core::MessageMetadata {
    elbmesh_core::MessageMetadata {
        message_id: message_id.to_string(),
        message_type: message_type.to_string(),
        message_version: 1,
        resource_type: "offer".to_string(),
        resource_id: offer_id.to_string(),
        stream_type: StreamType::Resource,
        correlation_id: "correlation-offer-accepted".to_string(),
        causation_id: "accept-offer-action-1".to_string(),
        action_id: "accept-offer-action-1".to_string(),
        actor_id: "actor-123".to_string(),
        occurred_at: "2026-07-07T00:00:00Z".to_string(),
        schema_id: schema_id.to_string(),
        schema_version,
    }
}

fn reaction_action_metadata(action_id: &str) -> ActionMetadata {
    ActionMetadata::with_ids(
        action_id,
        "correlation-offer-accepted",
        "offer-accepted-event-1",
        "reaction-runtime",
    )
}

fn deterministic_reaction_action_id<Rxn>(trigger: &RecordedEvent) -> String
where
    Rxn: Reaction,
{
    ReactionRuntime::<InMemoryEventStore, InMemoryReactionJournal>::reaction_action_id::<Rxn>(
        trigger,
    )
}
