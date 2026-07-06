use async_trait::async_trait;
use elbmesh_core::{
    apply_recorded_event, Action, ActionContext, ActionDecision, ActionError, ActionExecutor,
    ActionFailure, ActionMetadata, ActionScenario, AppendResult, Apply, Event, EventStore,
    EventStoreError, ExecutionError, ExpectedVersion, Handle, HandlerError, InMemoryEventStore,
    MessageMetadata, NewEvent, RecordedEvent, Resource, ResourceError, ResourceStream,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    sync::{Arc, Mutex},
};

#[derive(Debug, Default, Clone)]
struct Offer {
    id: Option<String>,
    title: Option<String>,
    measured_title_lengths: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OfferError {
    AlreadyExists,
    MissingReplayState,
}

impl fmt::Display for OfferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyExists => write!(f, "offer already exists"),
            Self::MissingReplayState => write!(f, "offer replay state is incomplete"),
        }
    }
}

impl ActionFailure for OfferError {
    fn code(&self) -> &'static str {
        match self {
            Self::AlreadyExists => "offer.already_exists",
            Self::MissingReplayState => "offer.missing_replay_state",
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

        if apply_recorded_event::<Self, OfferTitleUpdatedV1>(self, event)? {
            return Ok(());
        }

        if apply_recorded_event::<Self, OfferReplayStateCapturedV1>(self, event)? {
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
struct RecordWrongOfferEventV1 {
    offer_id: String,
    event_offer_id: String,
    title: String,
}

impl Action for RecordWrongOfferEventV1 {
    type Resource = Offer;

    const ACTION_TYPE: &'static str = "record_wrong_offer_event";
    const SCHEMA_ID: &'static str = "action.record_wrong_offer_event.v1";
    const SCHEMA_VERSION: u32 = 1;

    fn resource_id(&self) -> <Self::Resource as Resource>::Id {
        self.offer_id.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecordAppliedWrongOfferEventV1 {
    offer_id: String,
    event_offer_id: String,
    title: String,
}

impl Action for RecordAppliedWrongOfferEventV1 {
    type Resource = Offer;

    const ACTION_TYPE: &'static str = "record_applied_wrong_offer_event";
    const SCHEMA_ID: &'static str = "action.record_applied_wrong_offer_event.v1";
    const SCHEMA_VERSION: u32 = 1;

    fn resource_id(&self) -> <Self::Resource as Resource>::Id {
        self.offer_id.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CaptureOfferReplayStateV1 {
    offer_id: String,
}

impl Action for CaptureOfferReplayStateV1 {
    type Resource = Offer;

    const ACTION_TYPE: &'static str = "capture_offer_replay_state";
    const SCHEMA_ID: &'static str = "action.capture_offer_replay_state.v1";
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OfferTitleUpdatedV1 {
    offer_id: String,
    title: String,
}

impl Event for OfferTitleUpdatedV1 {
    type Resource = Offer;

    const EVENT_TYPE: &'static str = "offer_title_updated";
    const SCHEMA_ID: &'static str = "event.offer_title_updated.v1";
    const SCHEMA_VERSION: u32 = 1;

    fn resource_id(&self) -> <Self::Resource as Resource>::Id {
        self.offer_id.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OfferReplayStateCapturedV1 {
    offer_id: String,
    observed_title: String,
    observed_version: u64,
}

impl Event for OfferReplayStateCapturedV1 {
    type Resource = Offer;

    const EVENT_TYPE: &'static str = "offer_replay_state_captured";
    const SCHEMA_ID: &'static str = "event.offer_replay_state_captured.v1";
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

impl Apply<OfferTitleUpdatedV1> for Offer {
    fn apply(&mut self, event: OfferTitleUpdatedV1) -> Result<(), ResourceError> {
        self.title = Some(event.title);
        Ok(())
    }
}

impl Apply<OfferReplayStateCapturedV1> for Offer {
    fn apply(&mut self, _event: OfferReplayStateCapturedV1) -> Result<(), ResourceError> {
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

#[async_trait]
impl Handle<RecordWrongOfferEventV1> for Offer {
    type Error = OfferError;

    async fn handle(
        &mut self,
        action: RecordWrongOfferEventV1,
        ctx: &mut ActionContext<Self>,
    ) -> Result<ActionDecision, HandlerError<Self::Error>> {
        ctx.record(OfferTitleUpdatedV1 {
            offer_id: action.event_offer_id,
            title: action.title,
        })?;

        Ok(ActionDecision::completed())
    }
}

#[async_trait]
impl Handle<RecordAppliedWrongOfferEventV1> for Offer {
    type Error = OfferError;

    async fn handle(
        &mut self,
        action: RecordAppliedWrongOfferEventV1,
        ctx: &mut ActionContext<Self>,
    ) -> Result<ActionDecision, HandlerError<Self::Error>> {
        ctx.record_applied(
            self,
            OfferCreatedV1 {
                offer_id: action.event_offer_id,
                title: action.title,
            },
        )?;

        Ok(ActionDecision::completed())
    }
}

#[async_trait]
impl Handle<CaptureOfferReplayStateV1> for Offer {
    type Error = OfferError;

    async fn handle(
        &mut self,
        action: CaptureOfferReplayStateV1,
        ctx: &mut ActionContext<Self>,
    ) -> Result<ActionDecision, HandlerError<Self::Error>> {
        if self.id.as_deref() != Some(action.offer_id.as_str()) || self.title.is_none() {
            return Err(HandlerError::domain(OfferError::MissingReplayState));
        }

        ctx.record(OfferReplayStateCapturedV1 {
            offer_id: action.offer_id,
            observed_title: self.title.clone().expect("title checked above"),
            observed_version: ctx.current_version(),
        })?;

        Ok(ActionDecision::with_message("offer replay state captured"))
    }
}

#[derive(Clone)]
struct OutOfOrderLoadEventStore {
    stream: ResourceStream,
    history: Vec<RecordedEvent>,
    appends: Arc<Mutex<Vec<AppendCall>>>,
}

#[derive(Debug, Clone)]
struct AppendCall {
    expected_version: ExpectedVersion,
    events: Vec<RecordedEvent>,
}

impl OutOfOrderLoadEventStore {
    fn new(stream: ResourceStream, history: Vec<RecordedEvent>) -> Self {
        Self {
            stream,
            history,
            appends: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn append_calls(&self) -> Vec<AppendCall> {
        self.appends
            .lock()
            .expect("append call storage should not be poisoned")
            .clone()
    }
}

#[async_trait]
impl EventStore for OutOfOrderLoadEventStore {
    async fn load(&self, stream: &ResourceStream) -> Result<Vec<RecordedEvent>, EventStoreError> {
        assert_eq!(stream, &self.stream);
        Ok(self.history.clone())
    }

    async fn append(
        &self,
        stream: &ResourceStream,
        expected_version: ExpectedVersion,
        events: Vec<NewEvent>,
    ) -> Result<AppendResult, EventStoreError> {
        assert_eq!(stream, &self.stream);
        let latest_sequence = self
            .history
            .iter()
            .map(|event| event.sequence)
            .max()
            .unwrap_or_default();

        match expected_version {
            ExpectedVersion::Any => {}
            ExpectedVersion::NoStream if latest_sequence != 0 => {
                return Err(EventStoreError::ConcurrencyConflict {
                    stream: stream.key(),
                    expected: 0,
                    actual: latest_sequence,
                });
            }
            ExpectedVersion::Exact(expected) if latest_sequence != expected => {
                return Err(EventStoreError::ConcurrencyConflict {
                    stream: stream.key(),
                    expected,
                    actual: latest_sequence,
                });
            }
            ExpectedVersion::NoStream | ExpectedVersion::Exact(_) => {}
        }

        let recorded: Vec<_> = events
            .into_iter()
            .enumerate()
            .map(|(index, event)| RecordedEvent {
                stream: stream.clone(),
                sequence: latest_sequence + index as u64 + 1,
                metadata: event.metadata,
                payload: event.payload,
            })
            .collect();

        self.appends
            .lock()
            .expect("append call storage should not be poisoned")
            .push(AppendCall {
                expected_version,
                events: recorded.clone(),
            });

        Ok(AppendResult {
            previous_version: latest_sequence,
            new_version: latest_sequence + recorded.len() as u64,
            events: recorded,
        })
    }
}

fn event_to_new_event<E>(event: E) -> NewEvent
where
    E: Event,
{
    let resource_id = event.resource_id().to_string();
    NewEvent {
        metadata: MessageMetadata::resource_event(
            E::EVENT_TYPE,
            E::SCHEMA_ID,
            E::SCHEMA_VERSION,
            E::Resource::RESOURCE_TYPE,
            resource_id,
            &ActionMetadata::with_ids(
                "history-action",
                "history-correlation",
                "history-cause",
                "history",
            ),
        ),
        payload: serde_json::to_value(event).expect("event should serialize"),
    }
}

fn recorded_event<E>(event: E, sequence: u64) -> RecordedEvent
where
    E: Event,
{
    let stream = ResourceStream::new(E::Resource::RESOURCE_TYPE, event.resource_id().to_string());
    let event = event_to_new_event(event);

    RecordedEvent {
        stream,
        sequence,
        metadata: event.metadata,
        payload: event.payload,
    }
}

fn assert_wrong_resource_execution_error<E>(err: ExecutionError<E>, expected: &str, actual: &str)
where
    E: ActionFailure,
{
    let ExecutionError::Handler(HandlerError::Runtime(error)) = err else {
        panic!("expected wrong-resource runtime error");
    };

    assert_wrong_resource_action_error(error, expected, actual);
}

fn assert_wrong_resource_action_error(error: ActionError, expected: &str, actual: &str) {
    assert_eq!(error.code(), "action.wrong_resource");

    match error {
        ActionError::WrongResource {
            expected: observed_expected,
            actual: observed_actual,
        } => {
            assert_eq!(observed_expected, expected);
            assert_eq!(observed_actual, actual);
        }
        other => panic!("expected wrong-resource error, got {other:?}"),
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
async fn receipt_includes_one_emitted_event_summary_and_resource_versions() {
    let store = InMemoryEventStore::new();
    let executor = ActionExecutor::new(store.clone());

    let receipt = executor
        .execute::<Offer, _>(
            CreateOfferV1 {
                offer_id: "offer-receipt-one".to_string(),
                title: "Migration project".to_string(),
            },
            ActionMetadata::with_ids(
                "action-receipt-one",
                "correlation-receipt-one",
                "causation-receipt-one",
                "agent-1",
            ),
        )
        .await
        .expect("action should complete");

    assert_eq!(receipt.action_id, "action-receipt-one");
    assert_eq!(receipt.status, elbmesh_core::ActionStatus::Completed);
    assert_eq!(receipt.resource_type, "offer");
    assert_eq!(receipt.resource_id, "offer-receipt-one");
    assert_eq!(receipt.previous_version, 0);
    assert_eq!(receipt.new_version, 1);

    assert_eq!(receipt.emitted_events.len(), 1);
    let emitted = &receipt.emitted_events[0];
    assert!(!emitted.message_id.is_empty());
    assert_eq!(emitted.message_type, OfferCreatedV1::EVENT_TYPE);
    assert_eq!(emitted.schema_id, OfferCreatedV1::SCHEMA_ID);
    assert_eq!(emitted.schema_version, OfferCreatedV1::SCHEMA_VERSION);
    assert_eq!(emitted.sequence, 1);
}

#[tokio::test]
async fn receipt_includes_multiple_emitted_event_summaries_in_append_order() {
    let store = InMemoryEventStore::new();
    let executor = ActionExecutor::new(store.clone());

    let receipt = executor
        .execute::<Offer, _>(
            CreateOfferAndMeasureTitleV1 {
                offer_id: "offer-receipt-many".to_string(),
                title: "mesh".to_string(),
            },
            ActionMetadata::with_ids(
                "action-receipt-many",
                "correlation-receipt-many",
                "causation-receipt-many",
                "agent-1",
            ),
        )
        .await
        .expect("action should complete");

    assert_eq!(receipt.action_id, "action-receipt-many");
    assert_eq!(receipt.resource_type, "offer");
    assert_eq!(receipt.resource_id, "offer-receipt-many");
    assert_eq!(receipt.previous_version, 0);
    assert_eq!(receipt.new_version, 2);

    let stream = ResourceStream::new("offer", "offer-receipt-many");
    let history = store.load(&stream).await.expect("history should load");
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].metadata.message_type, OfferCreatedV1::EVENT_TYPE);
    assert_eq!(
        history[1].metadata.message_type,
        OfferTitleMeasuredV1::EVENT_TYPE
    );

    assert_eq!(receipt.emitted_events.len(), history.len());
    for (emitted, recorded) in receipt.emitted_events.iter().zip(history.iter()) {
        assert_eq!(emitted.message_id, recorded.metadata.message_id);
        assert_eq!(emitted.message_type, recorded.metadata.message_type);
        assert_eq!(emitted.schema_id, recorded.metadata.schema_id);
        assert_eq!(emitted.schema_version, recorded.metadata.schema_version);
        assert_eq!(emitted.sequence, recorded.sequence);
    }
}

#[tokio::test]
async fn receipt_emitted_event_summary_preserves_recorded_metadata() {
    let store = InMemoryEventStore::new();
    let executor = ActionExecutor::new(store.clone());

    let receipt = executor
        .execute::<Offer, _>(
            CreateOfferV1 {
                offer_id: "offer-receipt-metadata".to_string(),
                title: "Metadata project".to_string(),
            },
            ActionMetadata::with_ids(
                "action-receipt-metadata",
                "correlation-receipt-metadata",
                "causation-receipt-metadata",
                "agent-1",
            ),
        )
        .await
        .expect("action should complete");

    let stream = ResourceStream::new("offer", "offer-receipt-metadata");
    let history = store.load(&stream).await.expect("history should load");
    assert_eq!(history.len(), 1);

    let recorded = &history[0];
    assert_eq!(recorded.metadata.action_id, "action-receipt-metadata");
    assert_eq!(recorded.metadata.resource_type, "offer");
    assert_eq!(recorded.metadata.resource_id, "offer-receipt-metadata");
    assert_eq!(recorded.metadata.message_type, OfferCreatedV1::EVENT_TYPE);
    assert_eq!(recorded.metadata.schema_id, OfferCreatedV1::SCHEMA_ID);
    assert_eq!(
        recorded.metadata.schema_version,
        OfferCreatedV1::SCHEMA_VERSION
    );
    assert_eq!(recorded.sequence, 1);

    assert_eq!(receipt.emitted_events.len(), 1);
    let emitted = &receipt.emitted_events[0];
    assert_eq!(emitted.message_id, recorded.metadata.message_id);
    assert_eq!(emitted.message_type, recorded.metadata.message_type);
    assert_eq!(emitted.schema_id, recorded.metadata.schema_id);
    assert_eq!(emitted.schema_version, recorded.metadata.schema_version);
    assert_eq!(emitted.sequence, recorded.sequence);
}

#[tokio::test]
async fn wrong_resource_id_during_record_rejects_and_appends_no_event() {
    let store = InMemoryEventStore::new();
    let executor = ActionExecutor::new(store.clone());

    let err = executor
        .execute::<Offer, _>(
            RecordWrongOfferEventV1 {
                offer_id: "offer-1".to_string(),
                event_offer_id: "offer-2".to_string(),
                title: "Cross-resource title".to_string(),
            },
            ActionMetadata::for_actor("agent-1"),
        )
        .await
        .expect_err("wrong-resource event should be rejected");

    assert_wrong_resource_execution_error(err, "offer-1", "offer-2");
    assert!(store.all_events().is_empty());
}

#[tokio::test]
async fn wrong_resource_id_during_record_applied_rejects_and_appends_no_event() {
    let store = InMemoryEventStore::new();
    let executor = ActionExecutor::new(store.clone());

    let err = executor
        .execute::<Offer, _>(
            RecordAppliedWrongOfferEventV1 {
                offer_id: "offer-1".to_string(),
                event_offer_id: "offer-2".to_string(),
                title: "Cross-resource title".to_string(),
            },
            ActionMetadata::for_actor("agent-1"),
        )
        .await
        .expect_err("wrong-resource applied event should be rejected");

    assert_wrong_resource_execution_error(err, "offer-1", "offer-2");
    assert!(store.all_events().is_empty());
}

#[test]
fn record_applied_wrong_resource_id_leaves_resource_state_unchanged() {
    let mut ctx = ActionContext::<Offer>::new(
        ActionMetadata::with_ids("action-1", "correlation-1", "causation-1", "agent-1"),
        Offer::RESOURCE_TYPE,
        "offer-1",
        0,
    );
    let mut resource = Offer::default();

    let err = ctx
        .record_applied(
            &mut resource,
            OfferCreatedV1 {
                offer_id: "offer-2".to_string(),
                title: "Cross-resource title".to_string(),
            },
        )
        .expect_err("wrong-resource applied event should be rejected");

    assert_wrong_resource_action_error(err, "offer-1", "offer-2");
    assert_eq!(resource.id.as_deref(), None);
    assert_eq!(resource.title.as_deref(), None);
    assert!(resource.measured_title_lengths.is_empty());
    assert!(ctx.pending_events().is_empty());
}

#[tokio::test]
async fn replays_multiple_historical_events_before_handling_and_appends_next_sequence() {
    let store = InMemoryEventStore::new();
    let stream = ResourceStream::new("offer", "offer-1");

    store
        .append(
            &stream,
            ExpectedVersion::Any,
            vec![event_to_new_event(OfferCreatedV1 {
                offer_id: "offer-1".to_string(),
                title: "draft title".to_string(),
            })],
        )
        .await
        .expect("first historical event should append");
    store
        .append(
            &stream,
            ExpectedVersion::Any,
            vec![event_to_new_event(OfferTitleUpdatedV1 {
                offer_id: "offer-1".to_string(),
                title: "final title".to_string(),
            })],
        )
        .await
        .expect("second historical event should append");

    let executor = ActionExecutor::new(store.clone());
    let receipt = executor
        .execute::<Offer, _>(
            CaptureOfferReplayStateV1 {
                offer_id: "offer-1".to_string(),
            },
            ActionMetadata::for_actor("agent-1"),
        )
        .await
        .expect("action should complete after replay");

    assert_eq!(receipt.previous_version, 2);
    assert_eq!(receipt.new_version, 3);
    assert_eq!(receipt.emitted_events.len(), 1);
    assert_eq!(
        receipt.emitted_events[0].message_type,
        "offer_replay_state_captured"
    );
    assert_eq!(receipt.emitted_events[0].sequence, 3);

    let history = store.load(&stream).await.expect("history should load");
    assert_eq!(history.len(), 3);
    assert_eq!(history[2].sequence, 3);
    assert_eq!(
        history[2]
            .payload
            .get("offer_id")
            .and_then(|value| value.as_str()),
        Some("offer-1")
    );
    assert_eq!(
        history[2]
            .payload
            .get("observed_title")
            .and_then(|value| value.as_str()),
        Some("final title")
    );
    assert_eq!(
        history[2]
            .payload
            .get("observed_version")
            .and_then(|value| value.as_u64()),
        Some(2)
    );
}

#[tokio::test]
async fn replays_historical_events_in_stream_sequence_order_before_handling() {
    let stream = ResourceStream::new("offer", "offer-1");
    let created = recorded_event(
        OfferCreatedV1 {
            offer_id: "offer-1".to_string(),
            title: "draft title".to_string(),
        },
        1,
    );
    let updated = recorded_event(
        OfferTitleUpdatedV1 {
            offer_id: "offer-1".to_string(),
            title: "final title".to_string(),
        },
        2,
    );
    let store = OutOfOrderLoadEventStore::new(stream, vec![updated, created]);
    let executor = ActionExecutor::new(store.clone());

    let receipt = executor
        .execute::<Offer, _>(
            CaptureOfferReplayStateV1 {
                offer_id: "offer-1".to_string(),
            },
            ActionMetadata::for_actor("agent-1"),
        )
        .await
        .expect("action should complete after replay");

    assert_eq!(receipt.previous_version, 2);
    assert_eq!(receipt.new_version, 3);
    assert_eq!(receipt.emitted_events.len(), 1);
    assert_eq!(receipt.emitted_events[0].sequence, 3);

    let append_calls = store.append_calls();
    assert_eq!(append_calls.len(), 1);
    assert_eq!(append_calls[0].expected_version, ExpectedVersion::Exact(2));
    assert_eq!(append_calls[0].events.len(), 1);
    assert_eq!(append_calls[0].events[0].sequence, 3);
    assert_eq!(
        append_calls[0].events[0]
            .payload
            .get("observed_title")
            .and_then(|value| value.as_str()),
        Some("final title")
    );
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
