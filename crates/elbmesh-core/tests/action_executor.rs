use async_trait::async_trait;
use elbmesh_core::{
    apply_recorded_event, Action, ActionContext, ActionDecision, ActionError, ActionExecutor,
    ActionFailure, ActionFailureClassification, ActionJournal, ActionJournalError,
    ActionJournalRecord, ActionJournalStream, ActionMetadata, ActionReceipt, ActionScenario,
    ActionStatus, AppendResult, Apply, Event, EventStore, EventStoreError, ExecutionError,
    ExpectedVersion, Handle, HandlerError, InMemoryActionJournal, InMemoryEventStore,
    MessageMetadata, NewEvent, RecordedEvent, Resource, ResourceError, ResourceStream, StreamType,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fmt,
    sync::{Arc, Mutex, OnceLock},
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
    ActionCalledJournalMissing,
}

impl fmt::Display for OfferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyExists => write!(f, "offer already exists"),
            Self::MissingReplayState => write!(f, "offer replay state is incomplete"),
            Self::ActionCalledJournalMissing => {
                write!(f, "action called journal record is missing")
            }
        }
    }
}

impl ActionFailure for OfferError {
    fn code(&self) -> &'static str {
        match self {
            Self::AlreadyExists => "offer.already_exists",
            Self::MissingReplayState => "offer.missing_replay_state",
            Self::ActionCalledJournalMissing => "offer.action_called_journal_missing",
        }
    }

    fn details(&self) -> serde_json::Value {
        json!({
            "error_type": "OfferError",
            "error_variant": match self {
                Self::AlreadyExists => "AlreadyExists",
                Self::MissingReplayState => "MissingReplayState",
                Self::ActionCalledJournalMissing => "ActionCalledJournalMissing",
            },
        })
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
struct CreateOfferAfterObservingCalledJournalV1 {
    offer_id: String,
    title: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SerializationFailingActionV1 {
    offer_id: String,
}

impl Serialize for SerializationFailingActionV1 {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Err(serde::ser::Error::custom("action payload unavailable"))
    }
}

impl Action for CreateOfferAfterObservingCalledJournalV1 {
    type Resource = Offer;

    const ACTION_TYPE: &'static str = "create_offer_after_observing_called_journal";
    const SCHEMA_ID: &'static str = "action.create_offer_after_observing_called_journal.v1";
    const SCHEMA_VERSION: u32 = 1;

    fn resource_id(&self) -> <Self::Resource as Resource>::Id {
        self.offer_id.clone()
    }
}

impl Action for SerializationFailingActionV1 {
    type Resource = Offer;

    const ACTION_TYPE: &'static str = "serialization_failing_action";

    const SCHEMA_ID: &'static str = "action.serialization_failing_action.v1";

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
impl Handle<CreateOfferAfterObservingCalledJournalV1> for Offer {
    type Error = OfferError;

    async fn handle(
        &mut self,
        action: CreateOfferAfterObservingCalledJournalV1,
        ctx: &mut ActionContext<Self>,
    ) -> Result<ActionDecision, HandlerError<Self::Error>> {
        let stream = ActionJournalStream::for_action(ctx.metadata().action_id.clone());
        let records = handler_visible_action_journal()
            .load(&stream)
            .await
            .expect("handler-visible action journal should load");

        let saw_called_before_handler_side_effects = matches!(
            records.as_slice(),
            [ActionJournalRecord::ActionCalled { metadata, .. }]
                if metadata.action_id == ctx.metadata().action_id
                    && metadata.resource_id == action.offer_id
                    && metadata.stream_type == StreamType::Action
        );
        if !saw_called_before_handler_side_effects {
            return Err(HandlerError::domain(OfferError::ActionCalledJournalMissing));
        }

        ctx.record_applied(
            self,
            OfferCreatedV1 {
                offer_id: action.offer_id,
                title: action.title,
            },
        )?;

        Ok(ActionDecision::with_message(
            "offer created after observing action called journal record",
        ))
    }
}

#[async_trait]
impl Handle<SerializationFailingActionV1> for Offer {
    type Error = OfferError;

    async fn handle(
        &mut self,
        _action: SerializationFailingActionV1,
        _ctx: &mut ActionContext<Self>,
    ) -> Result<ActionDecision, HandlerError<Self::Error>> {
        panic!("serialization failure should happen before the handler runs")
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

#[derive(Clone)]
struct LoadFailingEventStore {
    reason: String,
}

impl LoadFailingEventStore {
    fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl EventStore for LoadFailingEventStore {
    async fn load(&self, _stream: &ResourceStream) -> Result<Vec<RecordedEvent>, EventStoreError> {
        Err(EventStoreError::Other(self.reason.clone()))
    }

    async fn append(
        &self,
        _stream: &ResourceStream,
        _expected_version: ExpectedVersion,
        _events: Vec<NewEvent>,
    ) -> Result<AppendResult, EventStoreError> {
        panic!("append should not run after load failure")
    }
}

#[derive(Clone)]
struct AppendFailingEventStore {
    stream: ResourceStream,
    reason: String,
    append_batches: Arc<Mutex<Vec<Vec<NewEvent>>>>,
}

impl AppendFailingEventStore {
    fn new(stream: ResourceStream, reason: impl Into<String>) -> Self {
        Self {
            stream,
            reason: reason.into(),
            append_batches: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn append_batches(&self) -> Vec<Vec<NewEvent>> {
        self.append_batches
            .lock()
            .expect("append batch storage should not be poisoned")
            .clone()
    }
}

#[async_trait]
impl EventStore for AppendFailingEventStore {
    async fn load(&self, stream: &ResourceStream) -> Result<Vec<RecordedEvent>, EventStoreError> {
        assert_eq!(stream, &self.stream);
        Ok(Vec::new())
    }

    async fn append(
        &self,
        stream: &ResourceStream,
        _expected_version: ExpectedVersion,
        events: Vec<NewEvent>,
    ) -> Result<AppendResult, EventStoreError> {
        assert_eq!(stream, &self.stream);
        self.append_batches
            .lock()
            .expect("append batch storage should not be poisoned")
            .push(events);

        Err(EventStoreError::Other(self.reason.clone()))
    }
}

#[derive(Clone)]
struct EventStoreObservingActionJournal {
    inner: InMemoryActionJournal,
    event_store: InMemoryEventStore,
    completed_receipts_seen_after_append: Arc<Mutex<Vec<ActionReceipt>>>,
}

impl EventStoreObservingActionJournal {
    fn new(inner: InMemoryActionJournal, event_store: InMemoryEventStore) -> Self {
        Self {
            inner,
            event_store,
            completed_receipts_seen_after_append: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn completed_receipts_seen_after_append(&self) -> Vec<ActionReceipt> {
        self.completed_receipts_seen_after_append
            .lock()
            .expect("completion observations poisoned")
            .clone()
    }
}

#[async_trait]
impl ActionJournal for EventStoreObservingActionJournal {
    async fn append(
        &self,
        stream: &ActionJournalStream,
        record: ActionJournalRecord,
    ) -> Result<(), ActionJournalError> {
        if let ActionJournalRecord::ActionCompleted { receipt, .. } = &record {
            let resource_stream =
                ResourceStream::new(receipt.resource_type.clone(), receipt.resource_id.clone());
            let history = self
                .event_store
                .load(&resource_stream)
                .await
                .expect("resource events should load before ActionCompleted is journaled");

            assert_eq!(history.len() as u64, receipt.new_version);
            assert_receipt_summarizes_resource_events(receipt, &history);

            self.completed_receipts_seen_after_append
                .lock()
                .expect("completion observations poisoned")
                .push(receipt.clone());
        }

        self.inner.append(stream, record).await
    }

    async fn load(
        &self,
        stream: &ActionJournalStream,
    ) -> Result<Vec<ActionJournalRecord>, ActionJournalError> {
        self.inner.load(stream).await
    }
}

#[derive(Clone)]
struct ActionFailedFailingActionJournal {
    inner: InMemoryActionJournal,
}

impl ActionFailedFailingActionJournal {
    fn new(inner: InMemoryActionJournal) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl ActionJournal for ActionFailedFailingActionJournal {
    async fn append(
        &self,
        stream: &ActionJournalStream,
        record: ActionJournalRecord,
    ) -> Result<(), ActionJournalError> {
        if matches!(&record, ActionJournalRecord::ActionFailed { .. }) {
            return Err(ActionJournalError::StoragePoisoned);
        }

        self.inner.append(stream, record).await
    }

    async fn load(
        &self,
        stream: &ActionJournalStream,
    ) -> Result<Vec<ActionJournalRecord>, ActionJournalError> {
        self.inner.load(stream).await
    }
}

fn handler_visible_action_journal() -> InMemoryActionJournal {
    static JOURNAL: OnceLock<InMemoryActionJournal> = OnceLock::new();

    JOURNAL.get_or_init(InMemoryActionJournal::new).clone()
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

fn assert_event_store_other_execution_error<E>(err: ExecutionError<E>, expected_reason: &str)
where
    E: ActionFailure,
{
    let ExecutionError::EventStore(EventStoreError::Other(reason)) = err else {
        panic!("expected typed event-store execution error");
    };

    assert_eq!(reason, expected_reason);
}

fn assert_offer_created_deserialization_execution_error<E>(err: ExecutionError<E>)
where
    E: ActionFailure,
{
    let ExecutionError::Resource(ResourceError::Deserialization {
        message_type,
        schema_version,
        ..
    }) = err
    else {
        panic!("expected typed resource deserialization execution error");
    };

    assert_eq!(message_type, OfferCreatedV1::EVENT_TYPE);
    assert_eq!(schema_version, OfferCreatedV1::SCHEMA_VERSION);
}

fn assert_action_serialization_execution_error<E>(err: ExecutionError<E>, expected_reason: &str)
where
    E: ActionFailure,
{
    let ExecutionError::Handler(HandlerError::Runtime(ActionError::Serialization(reason))) = err
    else {
        panic!("expected typed action serialization execution error");
    };

    assert_eq!(reason, expected_reason);
}

fn action_journal_record_metadata(record: &ActionJournalRecord) -> &MessageMetadata {
    match record {
        ActionJournalRecord::ActionCalled { metadata, .. }
        | ActionJournalRecord::ActionCompleted { metadata, .. }
        | ActionJournalRecord::ActionRejected { metadata, .. }
        | ActionJournalRecord::ActionFailed { metadata, .. } => metadata,
    }
}

fn action_journal_message_type(record: &ActionJournalRecord) -> &str {
    action_journal_record_metadata(record).message_type.as_str()
}

fn assert_action_journal_metadata(
    metadata: &MessageMetadata,
    message_type: &str,
    schema_id: &str,
    action: &ActionMetadata,
    offer_id: &str,
) {
    assert!(!metadata.message_id.is_empty());
    assert_eq!(metadata.message_type, message_type);
    assert_eq!(metadata.message_version, 1);
    assert_eq!(metadata.resource_type, Offer::RESOURCE_TYPE);
    assert_eq!(metadata.resource_id, offer_id);
    assert_eq!(metadata.stream_type, StreamType::Action);
    assert_eq!(metadata.correlation_id, action.correlation_id);
    assert_eq!(metadata.causation_id, action.causation_id);
    assert_eq!(metadata.action_id, action.action_id);
    assert_eq!(metadata.actor_id, action.actor_id);
    assert!(!metadata.occurred_at.is_empty());
    assert_eq!(metadata.schema_id, schema_id);
    assert_eq!(metadata.schema_version, 1);
}

fn assert_action_called_journal_record(
    record: &ActionJournalRecord,
    action: &ActionMetadata,
    offer_id: &str,
    action_type: &str,
    action_schema_id: &str,
    action_schema_version: u32,
    expected_payload: serde_json::Value,
) {
    match record {
        ActionJournalRecord::ActionCalled {
            metadata,
            action_type: actual_action_type,
            action_schema_id: actual_action_schema_id,
            action_schema_version: actual_action_schema_version,
            payload,
        } => {
            assert_action_journal_metadata(
                metadata,
                "action_called",
                "journal.action_called.v1",
                action,
                offer_id,
            );
            assert_eq!(actual_action_type, action_type);
            assert_eq!(actual_action_schema_id, action_schema_id);
            assert_eq!(*actual_action_schema_version, action_schema_version);
            assert_eq!(payload, &expected_payload);
        }
        ActionJournalRecord::ActionCompleted { .. }
        | ActionJournalRecord::ActionRejected { .. }
        | ActionJournalRecord::ActionFailed { .. } => panic!("expected ActionCalled record"),
    }
}

fn assert_action_completed_journal_record(
    record: &ActionJournalRecord,
    action: &ActionMetadata,
    receipt: &ActionReceipt,
) {
    match record {
        ActionJournalRecord::ActionCompleted {
            metadata,
            receipt: journal_receipt,
        } => {
            assert_action_journal_metadata(
                metadata,
                "action_completed",
                "journal.action_completed.v1",
                action,
                &receipt.resource_id,
            );
            assert_eq!(journal_receipt, receipt);
            assert_eq!(journal_receipt.action_id, action.action_id);
            assert_eq!(journal_receipt.status, ActionStatus::Completed);
        }
        ActionJournalRecord::ActionCalled { .. }
        | ActionJournalRecord::ActionRejected { .. }
        | ActionJournalRecord::ActionFailed { .. } => {
            panic!("expected ActionCompleted record")
        }
    }
}

fn assert_action_rejected_journal_record(
    record: &ActionJournalRecord,
    action: &ActionMetadata,
    offer_id: &str,
    expected_failure_code: &str,
    expected_failure_details: serde_json::Value,
) {
    match record {
        ActionJournalRecord::ActionRejected {
            metadata,
            failure_code,
            failure_details,
        } => {
            assert_action_journal_metadata(
                metadata,
                "action_rejected",
                "journal.action_rejected.v1",
                action,
                offer_id,
            );
            assert_eq!(failure_code, expected_failure_code);
            assert_eq!(failure_details, &expected_failure_details);
        }
        ActionJournalRecord::ActionCalled { .. }
        | ActionJournalRecord::ActionCompleted { .. }
        | ActionJournalRecord::ActionFailed { .. } => {
            panic!("expected ActionRejected record")
        }
    }
}

fn assert_action_failed_journal_record(
    record: &ActionJournalRecord,
    action: &ActionMetadata,
    offer_id: &str,
    expected_classification: ActionFailureClassification,
) {
    match record {
        ActionJournalRecord::ActionFailed {
            metadata,
            failure_classification,
            ..
        } => {
            assert_action_journal_metadata(
                metadata,
                "action_failed",
                "journal.action_failed.v1",
                action,
                offer_id,
            );
            assert_eq!(failure_classification, &expected_classification);
        }
        ActionJournalRecord::ActionCalled { .. }
        | ActionJournalRecord::ActionCompleted { .. }
        | ActionJournalRecord::ActionRejected { .. } => panic!("expected ActionFailed record"),
    }
}

fn assert_resource_stream_contains_only_events(
    history: &[RecordedEvent],
    expected_message_types: &[&str],
) {
    let message_types: Vec<_> = history
        .iter()
        .map(|event| event.metadata.message_type.as_str())
        .collect();

    assert_eq!(message_types, expected_message_types);
    for event in history {
        assert_eq!(event.metadata.stream_type, StreamType::Resource);
        assert_ne!(event.metadata.message_type, "action_called");
        assert_ne!(event.metadata.message_type, "action_completed");
        assert_ne!(event.metadata.message_type, "action_rejected");
        assert_ne!(event.metadata.message_type, "action_failed");
    }
}

fn assert_new_events_contain_only_resource_events(
    events: &[NewEvent],
    expected_message_types: &[&str],
) {
    let message_types: Vec<_> = events
        .iter()
        .map(|event| event.metadata.message_type.as_str())
        .collect();

    assert_eq!(message_types, expected_message_types);
    for event in events {
        assert_eq!(event.metadata.stream_type, StreamType::Resource);
        assert_ne!(event.metadata.message_type, "action_called");
        assert_ne!(event.metadata.message_type, "action_completed");
        assert_ne!(event.metadata.message_type, "action_rejected");
        assert_ne!(event.metadata.message_type, "action_failed");
    }
}

async fn execute_rejected_create_offer_with_journal(
    offer_id: &str,
    rejected_action_id: &str,
) -> (
    ExecutionError<OfferError>,
    InMemoryEventStore,
    InMemoryActionJournal,
    ActionMetadata,
    CreateOfferV1,
) {
    let store = InMemoryEventStore::new();
    let journal = InMemoryActionJournal::new();
    let executor = ActionExecutor::new(store.clone()).with_action_journal(journal.clone());

    executor
        .execute::<Offer, _>(
            CreateOfferV1 {
                offer_id: offer_id.to_string(),
                title: "Original offer".to_string(),
            },
            test_action_metadata(&format!("{rejected_action_id}-seed")),
        )
        .await
        .expect("seed action should complete");

    let action = CreateOfferV1 {
        offer_id: offer_id.to_string(),
        title: "Duplicate offer".to_string(),
    };
    let metadata = test_action_metadata(rejected_action_id);
    let err = executor
        .execute::<Offer, _>(action.clone(), metadata.clone())
        .await
        .expect_err("duplicate create should be rejected");

    (err, store, journal, metadata, action)
}

async fn execute_wrong_resource_runtime_failure_with_journal(
    offer_id: &str,
    event_offer_id: &str,
    action_id: &str,
) -> (
    ExecutionError<OfferError>,
    InMemoryEventStore,
    InMemoryActionJournal,
    ActionMetadata,
    RecordWrongOfferEventV1,
) {
    let store = InMemoryEventStore::new();
    let journal = InMemoryActionJournal::new();
    let executor = ActionExecutor::new(store.clone()).with_action_journal(journal.clone());
    let action = RecordWrongOfferEventV1 {
        offer_id: offer_id.to_string(),
        event_offer_id: event_offer_id.to_string(),
        title: "Cross-resource title".to_string(),
    };
    let metadata = test_action_metadata(action_id);
    let err = executor
        .execute::<Offer, _>(action.clone(), metadata.clone())
        .await
        .expect_err("wrong-resource runtime error should fail action execution");

    (err, store, journal, metadata, action)
}

async fn load_action_journal_records(
    journal: &InMemoryActionJournal,
    action: &ActionMetadata,
) -> Vec<ActionJournalRecord> {
    let journal_stream = ActionJournalStream::for_action(action.action_id.clone());
    journal
        .load(&journal_stream)
        .await
        .expect("action journal records should load")
}

fn malformed_offer_created_recorded_event(offer_id: &str) -> RecordedEvent {
    let mut event = event_to_new_event(OfferCreatedV1 {
        offer_id: offer_id.to_string(),
        title: "Malformed historical title".to_string(),
    });
    event.payload = json!({
        "title": "missing required offer_id",
    });

    RecordedEvent {
        stream: ResourceStream::new(Offer::RESOURCE_TYPE, offer_id),
        sequence: 1,
        metadata: event.metadata,
        payload: event.payload,
    }
}

fn test_action_metadata(action_id: &str) -> ActionMetadata {
    ActionMetadata::with_ids(
        action_id,
        format!("correlation-{action_id}"),
        format!("causation-{action_id}"),
        format!("actor-{action_id}"),
    )
}

fn rejected_offer_failure_details() -> serde_json::Value {
    json!({
        "error_type": "OfferError",
        "error_variant": "AlreadyExists",
    })
}

fn assert_receipt_summarizes_resource_events(receipt: &ActionReceipt, history: &[RecordedEvent]) {
    let previous_version = receipt.previous_version as usize;
    let new_version = receipt.new_version as usize;
    let appended_events = &history[previous_version..new_version];

    assert_eq!(receipt.emitted_events.len(), appended_events.len());
    for (summary, recorded) in receipt.emitted_events.iter().zip(appended_events.iter()) {
        assert_eq!(summary.message_id, recorded.metadata.message_id);
        assert_eq!(summary.message_type, recorded.metadata.message_type);
        assert_eq!(summary.schema_id, recorded.metadata.schema_id);
        assert_eq!(summary.schema_version, recorded.metadata.schema_version);
        assert_eq!(summary.sequence, recorded.sequence);
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
async fn successful_action_with_journal_records_called_before_handler_and_completed_in_order() {
    let store = InMemoryEventStore::new();
    let journal = handler_visible_action_journal();
    let executor = ActionExecutor::new(store.clone()).with_action_journal(journal.clone());
    let action = CreateOfferAfterObservingCalledJournalV1 {
        offer_id: "offer-journal-order".to_string(),
        title: "Journal order".to_string(),
    };
    let metadata = ActionMetadata::with_ids(
        "action-journal-order",
        "correlation-journal-order",
        "causation-journal-order",
        "actor-journal-order",
    );

    let receipt = executor
        .execute::<Offer, _>(action.clone(), metadata.clone())
        .await
        .expect("action should complete after observing ActionCalled in the journal");

    let stream = ActionJournalStream::for_action(metadata.action_id.clone());
    let records = journal
        .load(&stream)
        .await
        .expect("action journal records should load");

    assert_eq!(records.len(), 2);
    let journal_message_types: Vec<_> = records.iter().map(action_journal_message_type).collect();
    assert_eq!(
        journal_message_types,
        vec!["action_called", "action_completed"]
    );

    assert_action_called_journal_record(
        &records[0],
        &metadata,
        "offer-journal-order",
        CreateOfferAfterObservingCalledJournalV1::ACTION_TYPE,
        CreateOfferAfterObservingCalledJournalV1::SCHEMA_ID,
        CreateOfferAfterObservingCalledJournalV1::SCHEMA_VERSION,
        json!({
            "offer_id": action.offer_id,
            "title": action.title,
        }),
    );
    assert_action_completed_journal_record(&records[1], &metadata, &receipt);

    let resource_stream = ResourceStream::new(Offer::RESOURCE_TYPE, "offer-journal-order");
    let history = store
        .load(&resource_stream)
        .await
        .expect("resource events should load");
    assert_resource_stream_contains_only_events(&history, &[OfferCreatedV1::EVENT_TYPE]);
}

#[tokio::test]
async fn action_executor_journal_keeps_resource_event_stream_clean() {
    let store = InMemoryEventStore::new();
    let journal = InMemoryActionJournal::new();
    let executor = ActionExecutor::new(store.clone()).with_action_journal(journal.clone());
    let metadata = ActionMetadata::with_ids(
        "action-journal-stream-clean",
        "correlation-journal-stream-clean",
        "causation-journal-stream-clean",
        "actor-journal-stream-clean",
    );

    executor
        .execute::<Offer, _>(
            CreateOfferAndMeasureTitleV1 {
                offer_id: "offer-journal-stream-clean".to_string(),
                title: "mesh".to_string(),
            },
            metadata.clone(),
        )
        .await
        .expect("action should complete");

    let resource_stream = ResourceStream::new(Offer::RESOURCE_TYPE, "offer-journal-stream-clean");
    let history = store
        .load(&resource_stream)
        .await
        .expect("resource events should load");
    assert_resource_stream_contains_only_events(
        &history,
        &[OfferCreatedV1::EVENT_TYPE, OfferTitleMeasuredV1::EVENT_TYPE],
    );
    assert_eq!(store.all_events().len(), 2);

    let journal_stream = ActionJournalStream::for_action(metadata.action_id.clone());
    let records = journal
        .load(&journal_stream)
        .await
        .expect("action journal records should load");
    assert_eq!(records.len(), 2);
    for record in records {
        let metadata = action_journal_record_metadata(&record);
        assert_eq!(metadata.stream_type, StreamType::Action);
        assert_eq!(metadata.resource_type, Offer::RESOURCE_TYPE);
        assert_eq!(metadata.resource_id, "offer-journal-stream-clean");
        assert_eq!(metadata.action_id, "action-journal-stream-clean");
    }
}

#[tokio::test]
async fn action_completed_journal_record_contains_final_receipt_without_becoming_resource_event() {
    let store = InMemoryEventStore::new();
    let inner_journal = InMemoryActionJournal::new();
    let journal = EventStoreObservingActionJournal::new(inner_journal, store.clone());
    let executor = ActionExecutor::new(store.clone()).with_action_journal(journal.clone());
    let metadata = ActionMetadata::with_ids(
        "action-journal-completion-receipt",
        "correlation-journal-completion-receipt",
        "causation-journal-completion-receipt",
        "actor-journal-completion-receipt",
    );

    let receipt = executor
        .execute::<Offer, _>(
            CreateOfferAndMeasureTitleV1 {
                offer_id: "offer-journal-completion-receipt".to_string(),
                title: "mesh".to_string(),
            },
            metadata.clone(),
        )
        .await
        .expect("action should complete");

    let observations = journal.completed_receipts_seen_after_append();
    assert_eq!(observations, vec![receipt.clone()]);

    let journal_stream = ActionJournalStream::for_action(metadata.action_id.clone());
    let records = journal
        .load(&journal_stream)
        .await
        .expect("action journal records should load");
    assert_eq!(records.len(), 2);
    assert_action_completed_journal_record(&records[1], &metadata, &receipt);

    let resource_stream =
        ResourceStream::new(Offer::RESOURCE_TYPE, "offer-journal-completion-receipt");
    let history = store
        .load(&resource_stream)
        .await
        .expect("resource events should load");
    assert_receipt_summarizes_resource_events(&receipt, &history);
    assert_resource_stream_contains_only_events(
        &history,
        &[OfferCreatedV1::EVENT_TYPE, OfferTitleMeasuredV1::EVENT_TYPE],
    );
}

#[tokio::test]
async fn rejected_action_with_journal_records_called_and_rejected_in_order() {
    let (_err, _store, journal, metadata, action) = execute_rejected_create_offer_with_journal(
        "offer-rejected-journal-order",
        "action-rejected-journal-order",
    )
    .await;

    let journal_stream = ActionJournalStream::for_action(metadata.action_id.clone());
    let records = journal
        .load(&journal_stream)
        .await
        .expect("action journal records should load");

    assert_eq!(records.len(), 2);
    let journal_message_types: Vec<_> = records.iter().map(action_journal_message_type).collect();
    assert_eq!(
        journal_message_types,
        vec!["action_called", "action_rejected"]
    );

    assert_action_called_journal_record(
        &records[0],
        &metadata,
        "offer-rejected-journal-order",
        CreateOfferV1::ACTION_TYPE,
        CreateOfferV1::SCHEMA_ID,
        CreateOfferV1::SCHEMA_VERSION,
        json!({
            "offer_id": action.offer_id,
            "title": action.title,
        }),
    );
    assert_action_rejected_journal_record(
        &records[1],
        &metadata,
        "offer-rejected-journal-order",
        OfferError::AlreadyExists.code(),
        rejected_offer_failure_details(),
    );
}

#[tokio::test]
async fn rejected_action_with_journal_appends_no_resource_events() {
    let (_err, store, _journal, _metadata, _action) = execute_rejected_create_offer_with_journal(
        "offer-rejected-no-events",
        "action-rejected-no-events",
    )
    .await;

    let resource_stream = ResourceStream::new(Offer::RESOURCE_TYPE, "offer-rejected-no-events");
    let history = store
        .load(&resource_stream)
        .await
        .expect("resource events should load");

    assert_resource_stream_contains_only_events(&history, &[OfferCreatedV1::EVENT_TYPE]);
    assert_eq!(store.all_events().len(), 1);
}

#[tokio::test]
async fn action_rejected_journal_record_carries_stable_action_failure_code_and_details() {
    let (_err, _store, journal, metadata, _action) = execute_rejected_create_offer_with_journal(
        "offer-rejected-failure-code",
        "action-rejected-failure-code",
    )
    .await;

    let journal_stream = ActionJournalStream::for_action(metadata.action_id.clone());
    let records = journal
        .load(&journal_stream)
        .await
        .expect("action journal records should load");

    assert_action_rejected_journal_record(
        &records[1],
        &metadata,
        "offer-rejected-failure-code",
        OfferError::AlreadyExists.code(),
        rejected_offer_failure_details(),
    );
}

#[tokio::test]
async fn rejected_action_with_journal_returns_typed_domain_error_to_caller() {
    let (err, _store, _journal, _metadata, _action) = execute_rejected_create_offer_with_journal(
        "offer-rejected-typed-error",
        "action-rejected-typed-error",
    )
    .await;

    assert!(matches!(
        err,
        ExecutionError::Handler(HandlerError::Domain {
            error: OfferError::AlreadyExists
        })
    ));
}

#[tokio::test]
async fn append_failure_with_journal_records_failed_event_store_classification() {
    let store = AppendFailingEventStore::new(
        ResourceStream::new(Offer::RESOURCE_TYPE, "offer-append-failure"),
        "append unavailable",
    );
    let journal = InMemoryActionJournal::new();
    let executor = ActionExecutor::new(store.clone()).with_action_journal(journal.clone());
    let action = CreateOfferV1 {
        offer_id: "offer-append-failure".to_string(),
        title: "Append failure".to_string(),
    };
    let metadata = test_action_metadata("action-append-failure");

    let err = executor
        .execute::<Offer, _>(action.clone(), metadata.clone())
        .await
        .expect_err("resource event append failure should fail action execution");

    assert_event_store_other_execution_error(err, "append unavailable");

    let records = load_action_journal_records(&journal, &metadata).await;
    assert_eq!(records.len(), 2);
    let journal_message_types: Vec<_> = records.iter().map(action_journal_message_type).collect();
    assert_eq!(
        journal_message_types,
        vec!["action_called", "action_failed"]
    );
    assert_action_called_journal_record(
        &records[0],
        &metadata,
        "offer-append-failure",
        CreateOfferV1::ACTION_TYPE,
        CreateOfferV1::SCHEMA_ID,
        CreateOfferV1::SCHEMA_VERSION,
        json!({
            "offer_id": action.offer_id,
            "title": action.title,
        }),
    );
    assert_action_failed_journal_record(
        &records[1],
        &metadata,
        "offer-append-failure",
        ActionFailureClassification::EventStore,
    );

    let append_batches = store.append_batches();
    assert_eq!(append_batches.len(), 1);
    assert_new_events_contain_only_resource_events(
        &append_batches[0],
        &[OfferCreatedV1::EVENT_TYPE],
    );
}

#[tokio::test]
async fn load_failure_with_journal_records_failed_event_store_classification() {
    let store = LoadFailingEventStore::new("load unavailable");
    let journal = InMemoryActionJournal::new();
    let executor = ActionExecutor::new(store).with_action_journal(journal.clone());
    let action = CreateOfferV1 {
        offer_id: "offer-load-failure".to_string(),
        title: "Load failure".to_string(),
    };
    let metadata = test_action_metadata("action-load-failure");

    let err = executor
        .execute::<Offer, _>(action.clone(), metadata.clone())
        .await
        .expect_err("resource event load failure should fail action execution");

    assert_event_store_other_execution_error(err, "load unavailable");

    let records = load_action_journal_records(&journal, &metadata).await;
    assert_eq!(records.len(), 2);
    let journal_message_types: Vec<_> = records.iter().map(action_journal_message_type).collect();
    assert_eq!(
        journal_message_types,
        vec!["action_called", "action_failed"]
    );
    assert_action_called_journal_record(
        &records[0],
        &metadata,
        "offer-load-failure",
        CreateOfferV1::ACTION_TYPE,
        CreateOfferV1::SCHEMA_ID,
        CreateOfferV1::SCHEMA_VERSION,
        json!({
            "offer_id": action.offer_id,
            "title": action.title,
        }),
    );
    assert_action_failed_journal_record(
        &records[1],
        &metadata,
        "offer-load-failure",
        ActionFailureClassification::EventStore,
    );
}

#[tokio::test]
async fn replay_failure_with_journal_records_failed_resource_classification() {
    let offer_id = "offer-replay-failure";
    let resource_stream = ResourceStream::new(Offer::RESOURCE_TYPE, offer_id);
    let store = OutOfOrderLoadEventStore::new(
        resource_stream,
        vec![malformed_offer_created_recorded_event(offer_id)],
    );
    let journal = InMemoryActionJournal::new();
    let executor = ActionExecutor::new(store.clone()).with_action_journal(journal.clone());
    let action = CreateOfferV1 {
        offer_id: offer_id.to_string(),
        title: "Replay failure".to_string(),
    };
    let metadata = test_action_metadata("action-replay-failure");

    let err = executor
        .execute::<Offer, _>(action.clone(), metadata.clone())
        .await
        .expect_err("resource replay failure should fail action execution");

    assert_offer_created_deserialization_execution_error(err);

    let records = load_action_journal_records(&journal, &metadata).await;
    assert_eq!(records.len(), 2);
    let journal_message_types: Vec<_> = records.iter().map(action_journal_message_type).collect();
    assert_eq!(
        journal_message_types,
        vec!["action_called", "action_failed"]
    );
    assert_action_called_journal_record(
        &records[0],
        &metadata,
        offer_id,
        CreateOfferV1::ACTION_TYPE,
        CreateOfferV1::SCHEMA_ID,
        CreateOfferV1::SCHEMA_VERSION,
        json!({
            "offer_id": action.offer_id,
            "title": action.title,
        }),
    );
    assert_action_failed_journal_record(
        &records[1],
        &metadata,
        offer_id,
        ActionFailureClassification::Resource,
    );
    assert!(store.append_calls().is_empty());
}

#[tokio::test]
async fn handler_runtime_failure_with_journal_keeps_resource_stream_clean() {
    let (err, store, journal, metadata, action) =
        execute_wrong_resource_runtime_failure_with_journal(
            "offer-runtime-clean",
            "offer-runtime-clean-other",
            "action-runtime-clean",
        )
        .await;

    assert_wrong_resource_execution_error(err, "offer-runtime-clean", "offer-runtime-clean-other");

    let resource_stream = ResourceStream::new(Offer::RESOURCE_TYPE, "offer-runtime-clean");
    let history = store
        .load(&resource_stream)
        .await
        .expect("resource events should load");
    assert_resource_stream_contains_only_events(&history, &[]);
    assert!(store.all_events().is_empty());

    let records = load_action_journal_records(&journal, &metadata).await;
    assert_eq!(records.len(), 2);
    let journal_message_types: Vec<_> = records.iter().map(action_journal_message_type).collect();
    assert_eq!(
        journal_message_types,
        vec!["action_called", "action_failed"]
    );
    assert_action_called_journal_record(
        &records[0],
        &metadata,
        "offer-runtime-clean",
        RecordWrongOfferEventV1::ACTION_TYPE,
        RecordWrongOfferEventV1::SCHEMA_ID,
        RecordWrongOfferEventV1::SCHEMA_VERSION,
        json!({
            "event_offer_id": action.event_offer_id,
            "offer_id": action.offer_id,
            "title": action.title,
        }),
    );
    assert_action_failed_journal_record(
        &records[1],
        &metadata,
        "offer-runtime-clean",
        ActionFailureClassification::HandlerRuntime,
    );
}

#[tokio::test]
async fn action_called_serialization_failure_with_journal_records_failed_handler_runtime_classification(
) {
    let store = InMemoryEventStore::new();
    let journal = InMemoryActionJournal::new();
    let executor = ActionExecutor::new(store.clone()).with_action_journal(journal.clone());
    let action = SerializationFailingActionV1 {
        offer_id: "offer-action-called-serialization-failure".to_string(),
    };
    let metadata = test_action_metadata("action-called-serialization-failure");

    let err = executor
        .execute::<Offer, _>(action, metadata.clone())
        .await
        .expect_err("action payload serialization failure should fail action execution");

    assert_action_serialization_execution_error(err, "action payload unavailable");

    let records = load_action_journal_records(&journal, &metadata).await;
    assert_eq!(records.len(), 1);
    let journal_message_types: Vec<_> = records.iter().map(action_journal_message_type).collect();
    assert_eq!(journal_message_types, vec!["action_failed"]);
    assert_action_failed_journal_record(
        &records[0],
        &metadata,
        "offer-action-called-serialization-failure",
        ActionFailureClassification::HandlerRuntime,
    );

    let resource_stream = ResourceStream::new(
        Offer::RESOURCE_TYPE,
        "offer-action-called-serialization-failure",
    );
    let history = store
        .load(&resource_stream)
        .await
        .expect("resource events should load");
    assert_resource_stream_contains_only_events(&history, &[]);
}

#[tokio::test]
async fn failed_runtime_action_with_journal_returns_typed_execution_error_to_caller() {
    let (err, _store, _journal, _metadata, _action) =
        execute_wrong_resource_runtime_failure_with_journal(
            "offer-runtime-typed-error",
            "offer-runtime-typed-error-other",
            "action-runtime-typed-error",
        )
        .await;

    assert_wrong_resource_execution_error(
        err,
        "offer-runtime-typed-error",
        "offer-runtime-typed-error-other",
    );
}

#[tokio::test]
async fn failed_runtime_action_returns_typed_error_when_action_failed_journal_append_fails() {
    let store = InMemoryEventStore::new();
    let inner_journal = InMemoryActionJournal::new();
    let journal = ActionFailedFailingActionJournal::new(inner_journal);
    let executor = ActionExecutor::new(store.clone()).with_action_journal(journal.clone());
    let action = RecordWrongOfferEventV1 {
        offer_id: "offer-runtime-journal-failure".to_string(),
        event_offer_id: "offer-runtime-journal-failure-other".to_string(),
        title: "Runtime journal failure".to_string(),
    };
    let metadata = test_action_metadata("action-runtime-journal-failure");

    let err = executor
        .execute::<Offer, _>(action.clone(), metadata.clone())
        .await
        .expect_err("runtime failure should fail action execution");

    assert_wrong_resource_execution_error(
        err,
        "offer-runtime-journal-failure",
        "offer-runtime-journal-failure-other",
    );

    let journal_stream = ActionJournalStream::for_action(metadata.action_id.clone());
    let records = journal
        .load(&journal_stream)
        .await
        .expect("action journal records should load");
    assert_eq!(records.len(), 1);
    let journal_message_types: Vec<_> = records.iter().map(action_journal_message_type).collect();
    assert_eq!(journal_message_types, vec!["action_called"]);
    assert_action_called_journal_record(
        &records[0],
        &metadata,
        "offer-runtime-journal-failure",
        RecordWrongOfferEventV1::ACTION_TYPE,
        RecordWrongOfferEventV1::SCHEMA_ID,
        RecordWrongOfferEventV1::SCHEMA_VERSION,
        json!({
            "event_offer_id": action.event_offer_id,
            "offer_id": action.offer_id,
            "title": action.title,
        }),
    );

    assert!(store.all_events().is_empty());
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
    let expected = OfferError::AlreadyExists;

    assert_eq!(expected.code(), "offer.already_exists");

    ActionScenario::<Offer>::new()
        .given(vec![OfferCreatedV1 {
            offer_id: "offer-1".to_string(),
            title: "Migration project".to_string(),
        }])
        .when(CreateOfferV1 {
            offer_id: "offer-1".to_string(),
            title: "Duplicate".to_string(),
        })
        .then_error(expected)
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

    match err {
        EventStoreError::ConcurrencyConflict {
            stream,
            expected,
            actual,
        } => {
            assert_eq!(stream, "resources.offer.offer-1");
            assert_eq!(expected, 1);
            assert_eq!(actual, 0);
        }
        other => panic!("expected concurrency conflict, got {other:?}"),
    }
}
