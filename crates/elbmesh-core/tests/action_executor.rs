use async_trait::async_trait;
use elbmesh_core::{
    apply_recorded_event, Action, ActionContext, ActionDecision, ActionExecutor, ActionFailure,
    ActionMetadata, ActionScenario, Apply, Event, EventStore, ExpectedVersion, Handle,
    HandlerError, InMemoryEventStore, Resource, ResourceError, ResourceStream,
};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Default, Clone)]
struct Offer {
    id: Option<String>,
    title: Option<String>,
    measured_title_lengths: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OfferError {
    AlreadyExists,
}

impl fmt::Display for OfferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyExists => write!(f, "offer already exists"),
        }
    }
}

impl ActionFailure for OfferError {
    fn code(&self) -> &'static str {
        match self {
            Self::AlreadyExists => "offer.already_exists",
        }
    }
}

impl Resource for Offer {
    type Id = String;

    const RESOURCE_TYPE: &'static str = "offer";

    fn apply_recorded(&mut self, event: &elbmesh_core::RecordedEvent) -> Result<(), ResourceError> {
        if apply_recorded_event::<Self, OfferCreatedV1>(self, event)? {
            return Ok(());
        }

        if apply_recorded_event::<Self, OfferTitleMeasuredV1>(self, event)? {
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
struct CreateOfferV1 {
    offer_id: String,
    title: String,
}

impl Action for CreateOfferV1 {
    type Resource = Offer;

    const ACTION_TYPE: &'static str = "create_offer";
    const SCHEMA_ID: &'static str = "action.create_offer.v1";
    const SCHEMA_VERSION: u32 = 1;

    fn resource_id(&self) -> <Self::Resource as Resource>::Id {
        self.offer_id.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateOfferAndMeasureTitleV1 {
    offer_id: String,
    title: String,
}

impl Action for CreateOfferAndMeasureTitleV1 {
    type Resource = Offer;

    const ACTION_TYPE: &'static str = "create_offer_and_measure_title";
    const SCHEMA_ID: &'static str = "action.create_offer_and_measure_title.v1";
    const SCHEMA_VERSION: u32 = 1;

    fn resource_id(&self) -> <Self::Resource as Resource>::Id {
        self.offer_id.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OfferCreatedV1 {
    offer_id: String,
    title: String,
}

impl Event for OfferCreatedV1 {
    type Resource = Offer;

    const EVENT_TYPE: &'static str = "offer_created";
    const SCHEMA_ID: &'static str = "event.offer_created.v1";
    const SCHEMA_VERSION: u32 = 1;

    fn resource_id(&self) -> <Self::Resource as Resource>::Id {
        self.offer_id.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OfferTitleMeasuredV1 {
    offer_id: String,
    title_length: usize,
}

impl Event for OfferTitleMeasuredV1 {
    type Resource = Offer;

    const EVENT_TYPE: &'static str = "offer_title_measured";
    const SCHEMA_ID: &'static str = "event.offer_title_measured.v1";
    const SCHEMA_VERSION: u32 = 1;

    fn resource_id(&self) -> <Self::Resource as Resource>::Id {
        self.offer_id.clone()
    }
}

impl Apply<OfferCreatedV1> for Offer {
    fn apply(&mut self, event: OfferCreatedV1) -> Result<(), ResourceError> {
        self.id = Some(event.offer_id);
        self.title = Some(event.title);
        Ok(())
    }
}

impl Apply<OfferTitleMeasuredV1> for Offer {
    fn apply(&mut self, event: OfferTitleMeasuredV1) -> Result<(), ResourceError> {
        self.measured_title_lengths.push(event.title_length);
        Ok(())
    }
}

#[async_trait]
impl Handle<CreateOfferV1> for Offer {
    type Error = OfferError;

    async fn handle(
        &mut self,
        action: CreateOfferV1,
        ctx: &mut ActionContext<Self>,
    ) -> Result<ActionDecision, HandlerError<Self::Error>> {
        if self.id.is_some() {
            return Err(HandlerError::domain(OfferError::AlreadyExists));
        }

        ctx.record_applied(
            self,
            OfferCreatedV1 {
                offer_id: action.offer_id,
                title: action.title,
            },
        )?;

        Ok(ActionDecision::with_message("offer created"))
    }
}

#[async_trait]
impl Handle<CreateOfferAndMeasureTitleV1> for Offer {
    type Error = OfferError;

    async fn handle(
        &mut self,
        action: CreateOfferAndMeasureTitleV1,
        ctx: &mut ActionContext<Self>,
    ) -> Result<ActionDecision, HandlerError<Self::Error>> {
        if self.id.is_some() {
            return Err(HandlerError::domain(OfferError::AlreadyExists));
        }

        let offer_id = action.offer_id.clone();
        ctx.record_applied(
            self,
            OfferCreatedV1 {
                offer_id: action.offer_id,
                title: action.title,
            },
        )?;

        let title_length = self.title.as_deref().map(str::len).unwrap_or_default();
        ctx.record_applied(
            self,
            OfferTitleMeasuredV1 {
                offer_id,
                title_length,
            },
        )?;

        Ok(ActionDecision::with_message("offer created and measured"))
    }
}

#[tokio::test]
async fn executes_action_and_records_event() {
    let store = InMemoryEventStore::new();
    let executor = ActionExecutor::new(store.clone());

    let receipt = executor
        .execute::<Offer, _>(
            CreateOfferV1 {
                offer_id: "offer-1".to_string(),
                title: "Migration project".to_string(),
            },
            ActionMetadata::for_actor("agent-1"),
        )
        .await
        .expect("action should complete");

    assert_eq!(receipt.resource_type, "offer");
    assert_eq!(receipt.resource_id, "offer-1");
    assert_eq!(receipt.previous_version, 0);
    assert_eq!(receipt.new_version, 1);
    assert_eq!(receipt.emitted_events.len(), 1);
    assert_eq!(receipt.emitted_events[0].message_type, "offer_created");

    let stream = ResourceStream::new("offer", "offer-1");
    let history = store.load(&stream).await.expect("history should load");
    assert_eq!(history.len(), 1);

    let mut replayed = Offer::default();
    for event in &history {
        replayed.apply_recorded(event).expect("event should replay");
    }

    assert_eq!(replayed.id.as_deref(), Some("offer-1"));
    assert_eq!(replayed.title.as_deref(), Some("Migration project"));
}

#[tokio::test]
async fn record_applied_multi_event_action_observes_updated_resource_state() {
    let store = InMemoryEventStore::new();
    let executor = ActionExecutor::new(store.clone());

    let receipt = executor
        .execute::<Offer, _>(
            CreateOfferAndMeasureTitleV1 {
                offer_id: "offer-1".to_string(),
                title: "mesh".to_string(),
            },
            ActionMetadata::for_actor("agent-1"),
        )
        .await
        .expect("action should complete");

    let stream = ResourceStream::new("offer", "offer-1");
    let history = store.load(&stream).await.expect("history should load");
    assert_eq!(history.len(), 2);

    let appended_types: Vec<_> = history
        .iter()
        .map(|event| event.metadata.message_type.as_str())
        .collect();
    assert_eq!(
        appended_types,
        vec!["offer_created", "offer_title_measured"]
    );
    assert_eq!(history[0].sequence, 1);
    assert_eq!(history[1].sequence, 2);
    assert_eq!(
        history[1]
            .payload
            .get("title_length")
            .and_then(|value| value.as_u64()),
        Some(4)
    );

    assert_eq!(receipt.previous_version, 0);
    assert_eq!(receipt.new_version, 2);
    assert_eq!(receipt.emitted_events.len(), 2);
    assert_eq!(receipt.emitted_events[0].message_type, "offer_created");
    assert_eq!(receipt.emitted_events[0].sequence, 1);
    assert_eq!(
        receipt.emitted_events[1].message_type,
        "offer_title_measured"
    );
    assert_eq!(receipt.emitted_events[1].sequence, 2);
}

#[tokio::test]
async fn rejected_action_records_no_event() {
    let store = InMemoryEventStore::new();
    let executor = ActionExecutor::new(store.clone());

    executor
        .execute::<Offer, _>(
            CreateOfferV1 {
                offer_id: "offer-1".to_string(),
                title: "Migration project".to_string(),
            },
            ActionMetadata::for_actor("agent-1"),
        )
        .await
        .expect("first action should complete");

    let err = executor
        .execute::<Offer, _>(
            CreateOfferV1 {
                offer_id: "offer-1".to_string(),
                title: "Duplicate".to_string(),
            },
            ActionMetadata::for_actor("agent-1"),
        )
        .await
        .expect_err("duplicate create should be rejected");

    assert!(matches!(
        err,
        elbmesh_core::ExecutionError::Handler(HandlerError::Domain {
            error: OfferError::AlreadyExists
        })
    ));
    assert_eq!(store.all_events().len(), 1);
}

#[tokio::test]
async fn scenario_asserts_emitted_events() {
    ActionScenario::<Offer>::new()
        .when(CreateOfferV1 {
            offer_id: "offer-1".to_string(),
            title: "Migration project".to_string(),
        })
        .then(vec![OfferCreatedV1 {
            offer_id: "offer-1".to_string(),
            title: "Migration project".to_string(),
        }])
        .assert()
        .await;
}

#[tokio::test]
async fn scenario_asserts_typed_error() {
    ActionScenario::<Offer>::new()
        .given(vec![OfferCreatedV1 {
            offer_id: "offer-1".to_string(),
            title: "Migration project".to_string(),
        }])
        .when(CreateOfferV1 {
            offer_id: "offer-1".to_string(),
            title: "Duplicate".to_string(),
        })
        .then_error(OfferError::AlreadyExists)
        .assert()
        .await;
}

#[tokio::test]
async fn event_store_enforces_expected_version() {
    let store = InMemoryEventStore::new();
    let stream = ResourceStream::new("offer", "offer-1");

    let err = store
        .append(&stream, ExpectedVersion::Exact(1), Vec::new())
        .await
        .expect_err("wrong expected version should fail");

    assert!(matches!(
        err,
        elbmesh_core::EventStoreError::ConcurrencyConflict { .. }
    ));
}
