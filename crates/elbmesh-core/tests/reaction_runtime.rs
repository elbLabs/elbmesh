use async_trait::async_trait;

use elbmesh_core::{
    apply_recorded_event, Action, ActionContext, ActionDecision, ActionFailure, ActionMetadata,
    Apply, Event, EventStore, Handle, HandlerError, InMemoryEventStore, InMemoryReactionJournal,
    Reaction, ReactionJournal, ReactionJournalRecord, ReactionJournalStream, ReactionRuntime,
    RecordedEvent, Resource, ResourceError, ResourceStream, StreamType,
};

use serde::{Deserialize, Serialize};
use serde_json::json;

use std::fmt;

#[tokio::test]
async fn offer_accepted_reaction_executes_create_sales_order_through_action_executor() {
    let reaction_journal = InMemoryReactionJournal::new();
    let runtime = ReactionRuntime::new(InMemoryEventStore::new(), reaction_journal);
    let trigger = offer_accepted_recorded_event("offer-1");

    let receipt = runtime
        .execute(
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
        .execute(
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
async fn non_matching_event_is_ignored_without_resource_events_or_reaction_journal_records() {
    let reaction_journal = InMemoryReactionJournal::new();
    let runtime = ReactionRuntime::new(InMemoryEventStore::new(), reaction_journal.clone());
    let trigger = offer_created_recorded_event("offer-1");
    let ignored_reaction_id =
        ReactionRuntime::<InMemoryEventStore, InMemoryReactionJournal>::reaction_id::<
            OfferAcceptedCreatesSalesOrder,
        >(&trigger);

    let receipt = runtime
        .execute(
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
        .execute(
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
