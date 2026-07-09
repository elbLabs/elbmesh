use async_trait::async_trait;
use chrono::Utc;
use serde_json::{json, Value};
use thiserror::Error;

use crate::{
    Action as ActionTrait, ActionError, ActionExecutor, ActionFailure, ActionJournal,
    ActionJournalError, ActionMetadata, ActionReceipt, Event, EventStore, EventStoreError,
    ExecutionError, Handle, HandlerError, MessageMetadata, ReactionJournal, ReactionJournalError,
    ReactionJournalRecord, ReactionJournalStream, RecordedEvent, Resource, ResourceError,
    StreamType,
};

#[async_trait]
pub trait Reaction: Send + Sync + 'static {
    type Trigger: Event;
    type Resource: Resource + Handle<Self::Action>;
    type Action: ActionTrait<Resource = Self::Resource>;

    const REACTION_TYPE: &'static str;
    const SCHEMA_ID: &'static str;
    const SCHEMA_VERSION: u32;

    async fn react(&self, event: Self::Trigger) -> Self::Action;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionReceipt {
    pub reaction_id: String,
    pub action_receipt: ActionReceipt,
}

#[derive(Debug, Error)]
pub enum ReactionExecutionError<E>
where
    E: ActionFailure,
{
    #[error(
        "failed to deserialize reaction trigger event '{message_type}' v{schema_version}: {source}"
    )]
    TriggerEventDeserialization {
        message_type: String,
        schema_version: u32,
        source: serde_json::Error,
    },

    #[error("reaction trigger stream identity mismatch: metadata resource '{metadata_resource_type}/{metadata_resource_id}', stream resource '{stream_resource_type}/{stream_resource_id}'")]
    TriggerStreamIdentityMismatch {
        metadata_resource_type: String,
        metadata_resource_id: String,
        stream_resource_type: String,
        stream_resource_id: String,
    },

    #[error("reaction trigger payload identity mismatch: metadata resource id '{metadata_resource_id}', payload resource id '{payload_resource_id}'")]
    TriggerPayloadIdentityMismatch {
        metadata_resource_id: String,
        payload_resource_id: String,
    },

    #[error(transparent)]
    ReactionJournal(#[from] ReactionJournalError),

    #[error(transparent)]
    Action(#[from] ExecutionError<E>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReactionDispatchFailure {
    pub reaction_type: String,
    pub failure_code: String,
    pub failure_details: Value,
}

#[derive(Debug, Error, PartialEq)]
pub enum ReactionDispatchError {
    #[error("reaction dispatch failed for one or more handlers")]
    HandlerFailures {
        receipts: Vec<ReactionReceipt>,
        failures: Vec<ReactionDispatchFailure>,
    },
}

pub struct ReactionRuntime<S, J> {
    action_executor: ActionExecutor<S>,
    reaction_journal: J,
}

impl<S, J> ReactionRuntime<S, J>
where
    S: EventStore,
    J: ReactionJournal,
{
    pub fn new(event_store: S, reaction_journal: J) -> Self {
        Self {
            action_executor: ActionExecutor::new(event_store),
            reaction_journal,
        }
    }

    pub fn with_action_journal<AJ>(mut self, action_journal: AJ) -> Self
    where
        AJ: ActionJournal,
    {
        self.action_executor = self.action_executor.with_action_journal(action_journal);
        self
    }

    pub fn event_store(&self) -> &S {
        self.action_executor.event_store()
    }

    pub fn reaction_id<Rxn>(trigger: &RecordedEvent) -> String
    where
        Rxn: Reaction,
    {
        format!(
            "reaction:{}",
            deterministic_identity(&[
                ("reaction_type", Rxn::REACTION_TYPE),
                ("trigger_event_id", &trigger.metadata.message_id),
            ])
        )
    }

    pub fn reaction_action_id<Rxn>(trigger: &RecordedEvent) -> String
    where
        Rxn: Reaction,
    {
        deterministic_reaction_action_id::<Rxn>(trigger)
    }

    pub fn reaction_action_metadata<Rxn>(trigger: &RecordedEvent) -> ActionMetadata
    where
        Rxn: Reaction,
    {
        deterministic_reaction_action_metadata::<Rxn>(trigger)
    }

    pub async fn execute<Rxn>(
        &self,
        trigger: &RecordedEvent,
        reaction: &Rxn,
    ) -> Result<
        Option<ReactionReceipt>,
        ReactionExecutionError<
            <<Rxn as Reaction>::Resource as Handle<<Rxn as Reaction>::Action>>::Error,
        >,
    >
    where
        Rxn: Reaction,
    {
        self.execute_with_metadata(
            trigger,
            reaction,
            Self::reaction_action_metadata::<Rxn>(trigger),
        )
        .await
    }

    pub async fn execute_with_metadata<Rxn>(
        &self,
        trigger: &RecordedEvent,
        reaction: &Rxn,
        action_metadata: ActionMetadata,
    ) -> Result<
        Option<ReactionReceipt>,
        ReactionExecutionError<
            <<Rxn as Reaction>::Resource as Handle<<Rxn as Reaction>::Action>>::Error,
        >,
    >
    where
        Rxn: Reaction,
    {
        if !matches_trigger::<Rxn::Trigger>(trigger) {
            return Ok(None);
        }

        if trigger.stream.resource_type != trigger.metadata.resource_type
            || trigger.stream.resource_id != trigger.metadata.resource_id
        {
            return Err(ReactionExecutionError::TriggerStreamIdentityMismatch {
                metadata_resource_type: trigger.metadata.resource_type.clone(),
                metadata_resource_id: trigger.metadata.resource_id.clone(),
                stream_resource_type: trigger.stream.resource_type.clone(),
                stream_resource_id: trigger.stream.resource_id.clone(),
            });
        }

        let reaction_id = Self::reaction_id::<Rxn>(trigger);
        let journal_stream = ReactionJournalStream::for_reaction(reaction_id.clone());
        let trigger_event = serde_json::from_value::<Rxn::Trigger>(trigger.payload.clone())
            .map_err(
                |source| ReactionExecutionError::TriggerEventDeserialization {
                    message_type: trigger.metadata.message_type.clone(),
                    schema_version: trigger.metadata.schema_version,
                    source,
                },
            )?;

        let payload_resource_id = trigger_event.resource_id().to_string();
        if payload_resource_id != trigger.metadata.resource_id {
            return Err(ReactionExecutionError::TriggerPayloadIdentityMismatch {
                metadata_resource_id: trigger.metadata.resource_id.clone(),
                payload_resource_id,
            });
        }

        let records = self.reaction_journal.load(&journal_stream).await?;
        if records.iter().any(|record| {
            matches!(
                record,
                ReactionJournalRecord::ReactionCompleted {
                    reaction_id: completed_reaction_id,
                    ..
                } if completed_reaction_id == &reaction_id
            )
        }) {
            return Ok(None);
        }

        let action = reaction.react(trigger_event).await;
        self.reaction_journal
            .append(
                &journal_stream,
                ReactionJournalRecord::ReactionTriggered {
                    reaction_id: reaction_id.clone(),
                    metadata: reaction_journal_metadata(
                        "reaction_triggered",
                        "journal.reaction_triggered.v1",
                        trigger,
                        &action_metadata,
                    ),
                    reaction_type: Rxn::REACTION_TYPE.to_string(),
                    reaction_schema_id: Rxn::SCHEMA_ID.to_string(),
                    reaction_schema_version: Rxn::SCHEMA_VERSION,
                    trigger_event_type: <Rxn::Trigger as Event>::EVENT_TYPE.to_string(),
                    trigger_event_id: trigger.metadata.message_id.clone(),
                },
            )
            .await?;

        let action_receipt = self
            .action_executor
            .execute::<Rxn::Resource, Rxn::Action>(action, action_metadata.clone())
            .await?;

        self.reaction_journal
            .append(
                &journal_stream,
                ReactionJournalRecord::ReactionCompleted {
                    reaction_id: reaction_id.clone(),
                    metadata: reaction_journal_metadata(
                        "reaction_completed",
                        "journal.reaction_completed.v1",
                        trigger,
                        &action_metadata,
                    ),
                    triggered_action_id: action_receipt.action_id.clone(),
                },
            )
            .await?;

        Ok(Some(ReactionReceipt {
            reaction_id,
            action_receipt,
        }))
    }
}

#[async_trait]
trait EventReactionHandler<S, J>: Send + Sync + 'static
where
    S: EventStore,
    J: ReactionJournal,
{
    async fn handle(
        &self,
        runtime: &ReactionRuntime<S, J>,
        trigger: &RecordedEvent,
    ) -> Result<Option<ReactionReceipt>, ReactionDispatchFailure>;
}

pub struct TypedReactionHandler<Rxn, F> {
    reaction: Rxn,
    action_metadata: F,
}

impl<Rxn, F> TypedReactionHandler<Rxn, F> {
    pub fn new(reaction: Rxn, action_metadata: F) -> Self {
        Self {
            reaction,
            action_metadata,
        }
    }
}

impl<Rxn> TypedReactionHandler<Rxn, fn(&RecordedEvent) -> ActionMetadata>
where
    Rxn: Reaction,
{
    pub fn deterministic(reaction: Rxn) -> Self {
        Self {
            reaction,
            action_metadata: deterministic_reaction_action_metadata::<Rxn>
                as fn(&RecordedEvent) -> ActionMetadata,
        }
    }
}

#[async_trait]
impl<S, J, Rxn, F> EventReactionHandler<S, J> for TypedReactionHandler<Rxn, F>
where
    S: EventStore,
    J: ReactionJournal,
    Rxn: Reaction,
    F: for<'a> Fn(&'a RecordedEvent) -> ActionMetadata + Send + Sync + 'static,
{
    async fn handle(
        &self,
        runtime: &ReactionRuntime<S, J>,
        trigger: &RecordedEvent,
    ) -> Result<Option<ReactionReceipt>, ReactionDispatchFailure> {
        if !matches_trigger::<Rxn::Trigger>(trigger) {
            return Ok(None);
        }

        let action_metadata = (self.action_metadata)(trigger);

        runtime
            .execute_with_metadata(trigger, &self.reaction, action_metadata)
            .await
            .map_err(reaction_dispatch_failure::<Rxn>)
    }
}

pub struct ReactionDispatcher<S, J> {
    runtime: ReactionRuntime<S, J>,
    handlers: Vec<Box<dyn EventReactionHandler<S, J>>>,
}

impl<S, J> ReactionDispatcher<S, J>
where
    S: EventStore,
    J: ReactionJournal,
{
    pub fn new(runtime: ReactionRuntime<S, J>) -> Self {
        Self {
            runtime,
            handlers: Vec::new(),
        }
    }

    pub fn event_store(&self) -> &S {
        self.runtime.event_store()
    }

    pub fn with_handler<Rxn, F>(mut self, handler: TypedReactionHandler<Rxn, F>) -> Self
    where
        Rxn: Reaction,
        F: for<'a> Fn(&'a RecordedEvent) -> ActionMetadata + Send + Sync + 'static,
    {
        self.handlers.push(Box::new(handler));
        self
    }

    pub fn with_deterministic_handler<Rxn>(self, reaction: Rxn) -> Self
    where
        Rxn: Reaction,
    {
        self.with_handler(TypedReactionHandler::deterministic(reaction))
    }

    pub async fn dispatch(
        &self,
        trigger: &RecordedEvent,
    ) -> Result<Vec<ReactionReceipt>, ReactionDispatchError> {
        let mut receipts = Vec::new();
        let mut failures = Vec::new();

        for handler in &self.handlers {
            match handler.handle(&self.runtime, trigger).await {
                Ok(Some(receipt)) => receipts.push(receipt),
                Ok(None) => {}
                Err(failure) => failures.push(failure),
            }
        }

        if failures.is_empty() {
            Ok(receipts)
        } else {
            Err(ReactionDispatchError::HandlerFailures { receipts, failures })
        }
    }
}

fn reaction_dispatch_failure<Rxn>(
    error: ReactionExecutionError<
        <<Rxn as Reaction>::Resource as Handle<<Rxn as Reaction>::Action>>::Error,
    >,
) -> ReactionDispatchFailure
where
    Rxn: Reaction,
{
    ReactionDispatchFailure {
        reaction_type: Rxn::REACTION_TYPE.to_string(),
        failure_code: reaction_execution_failure_code(&error).to_string(),
        failure_details: reaction_execution_failure_details(&error),
    }
}

fn reaction_execution_failure_code<E>(error: &ReactionExecutionError<E>) -> &'static str
where
    E: ActionFailure,
{
    match error {
        ReactionExecutionError::TriggerEventDeserialization { .. } => {
            "reaction.trigger_event_deserialization"
        }
        ReactionExecutionError::TriggerStreamIdentityMismatch { .. } => {
            "reaction.trigger_stream_identity_mismatch"
        }
        ReactionExecutionError::TriggerPayloadIdentityMismatch { .. } => {
            "reaction.trigger_payload_identity_mismatch"
        }
        ReactionExecutionError::ReactionJournal(_) => "reaction.journal_error",
        ReactionExecutionError::Action(ExecutionError::Handler(HandlerError::Domain { error })) => {
            error.code()
        }
        ReactionExecutionError::Action(ExecutionError::Handler(HandlerError::Runtime(error))) => {
            error.code()
        }
        ReactionExecutionError::Action(ExecutionError::Resource(_)) => {
            "reaction.action_resource_error"
        }
        ReactionExecutionError::Action(ExecutionError::EventStore(_)) => {
            "reaction.action_event_store_error"
        }
        ReactionExecutionError::Action(ExecutionError::ActionJournal(_)) => {
            "reaction.action_journal_error"
        }
    }
}

fn reaction_execution_failure_details<E>(error: &ReactionExecutionError<E>) -> Value
where
    E: ActionFailure,
{
    match error {
        ReactionExecutionError::TriggerEventDeserialization {
            message_type,
            schema_version,
            ..
        } => json!({
            "message_type": message_type,
            "schema_version": schema_version,
        }),
        ReactionExecutionError::TriggerStreamIdentityMismatch {
            metadata_resource_type,
            metadata_resource_id,
            stream_resource_type,
            stream_resource_id,
        } => json!({
            "error_type": "ReactionExecutionError",
            "error_variant": "TriggerStreamIdentityMismatch",
            "metadata_resource_type": metadata_resource_type,
            "metadata_resource_id": metadata_resource_id,
            "stream_resource_type": stream_resource_type,
            "stream_resource_id": stream_resource_id,
        }),
        ReactionExecutionError::TriggerPayloadIdentityMismatch {
            metadata_resource_id,
            payload_resource_id,
        } => json!({
            "error_type": "ReactionExecutionError",
            "error_variant": "TriggerPayloadIdentityMismatch",
            "metadata_resource_id": metadata_resource_id,
            "payload_resource_id": payload_resource_id,
        }),
        ReactionExecutionError::ReactionJournal(error) => reaction_journal_error_details(error),
        ReactionExecutionError::Action(ExecutionError::Handler(HandlerError::Domain { error })) => {
            error.details()
        }
        ReactionExecutionError::Action(ExecutionError::Handler(HandlerError::Runtime(error))) => {
            action_error_details(error)
        }
        ReactionExecutionError::Action(ExecutionError::Resource(error)) => {
            resource_error_details(error)
        }
        ReactionExecutionError::Action(ExecutionError::EventStore(error)) => {
            event_store_error_details(error)
        }
        ReactionExecutionError::Action(ExecutionError::ActionJournal(error)) => {
            action_journal_error_details(error)
        }
    }
}

fn action_error_details(error: &ActionError) -> Value {
    match error {
        ActionError::Rejected { reason } => json!({
            "error_type": "ActionError",
            "error_variant": "Rejected",
            "reason": reason,
        }),
        ActionError::Validation { reason } => json!({
            "error_type": "ActionError",
            "error_variant": "Validation",
            "reason": reason,
        }),
        ActionError::ExternalOperation {
            operation_type,
            failure_code,
            failure_details,
        } => json!({
            "error_type": "ActionError",
            "error_variant": "ExternalOperation",
            "operation_type": operation_type,
            "failure_code": failure_code,
            "failure_details": failure_details,
        }),
        ActionError::OperationJournal {
            operation_id,
            failure_code,
            failure_details,
        } => json!({
            "error_type": "ActionError",
            "error_variant": "OperationJournal",
            "operation_id": operation_id,
            "failure_code": failure_code,
            "failure_details": failure_details,
        }),
        ActionError::StateTransition { reason } => json!({
            "error_type": "ActionError",
            "error_variant": "StateTransition",
            "reason": reason,
        }),
        ActionError::Serialization(reason) => json!({
            "error_type": "ActionError",
            "error_variant": "Serialization",
            "reason": reason,
        }),
        ActionError::WrongResource { expected, actual } => json!({
            "error_type": "ActionError",
            "error_variant": "WrongResource",
            "expected": expected,
            "actual": actual,
        }),
        ActionError::Other(reason) => json!({
            "error_type": "ActionError",
            "error_variant": "Other",
            "reason": reason,
        }),
    }
}

fn resource_error_details(error: &ResourceError) -> Value {
    match error {
        ResourceError::UnsupportedEvent {
            resource_type,
            message_type,
            schema_version,
        } => json!({
            "error_type": "ResourceError",
            "error_variant": "UnsupportedEvent",
            "resource_type": resource_type,
            "message_type": message_type,
            "schema_version": schema_version,
        }),
        ResourceError::Deserialization {
            message_type,
            schema_version,
            source,
        } => json!({
            "error_type": "ResourceError",
            "error_variant": "Deserialization",
            "message_type": message_type,
            "schema_version": schema_version,
            "source": source.to_string(),
        }),
        ResourceError::Apply(reason) => json!({
            "error_type": "ResourceError",
            "error_variant": "Apply",
            "reason": reason,
        }),
    }
}

fn event_store_error_details(error: &EventStoreError) -> Value {
    match error {
        EventStoreError::ConcurrencyConflict {
            stream,
            expected,
            actual,
        } => json!({
            "error_type": "EventStoreError",
            "error_variant": "ConcurrencyConflict",
            "stream": stream,
            "expected": expected,
            "actual": actual,
        }),
        EventStoreError::WrongEventStream {
            stream,
            expected_resource_type,
            expected_resource_id,
            actual_resource_type,
            actual_resource_id,
        } => json!({
            "error_type": "EventStoreError",
            "error_variant": "WrongEventStream",
            "stream": stream,
            "expected_resource_type": expected_resource_type,
            "expected_resource_id": expected_resource_id,
            "actual_resource_type": actual_resource_type,
            "actual_resource_id": actual_resource_id,
        }),
        EventStoreError::WrongEventStreamType {
            stream,
            expected_stream_type,
            actual_stream_type,
        } => json!({
            "error_type": "EventStoreError",
            "error_variant": "WrongEventStreamType",
            "stream": stream,
            "expected_stream_type": expected_stream_type,
            "actual_stream_type": actual_stream_type,
        }),
        EventStoreError::NatsConnect { reason } => json!({
            "error_type": "EventStoreError",
            "error_variant": "NatsConnect",
            "reason": reason,
        }),
        EventStoreError::NatsBucket { bucket, reason } => json!({
            "error_type": "EventStoreError",
            "error_variant": "NatsBucket",
            "bucket": bucket,
            "reason": reason,
        }),
        EventStoreError::StreamSerialization { stream, reason } => json!({
            "error_type": "EventStoreError",
            "error_variant": "StreamSerialization",
            "stream": stream,
            "reason": reason,
        }),
        EventStoreError::StreamDeserialization {
            stream,
            revision,
            reason,
        } => json!({
            "error_type": "EventStoreError",
            "error_variant": "StreamDeserialization",
            "stream": stream,
            "revision": revision,
            "reason": reason,
        }),
        EventStoreError::NatsLoad { stream, reason } => json!({
            "error_type": "EventStoreError",
            "error_variant": "NatsLoad",
            "stream": stream,
            "reason": reason,
        }),
        EventStoreError::NatsAppend { stream, reason } => json!({
            "error_type": "EventStoreError",
            "error_variant": "NatsAppend",
            "stream": stream,
            "reason": reason,
        }),
        EventStoreError::Other(reason) => json!({
            "error_type": "EventStoreError",
            "error_variant": "Other",
            "reason": reason,
        }),
    }
}

fn action_journal_error_details(error: &ActionJournalError) -> Value {
    match error {
        ActionJournalError::WrongActionStream {
            expected_action_id,
            actual_action_id,
        } => json!({
            "error_type": "ActionJournalError",
            "error_variant": "WrongActionStream",
            "expected_action_id": expected_action_id,
            "actual_action_id": actual_action_id,
        }),
        ActionJournalError::StoragePoisoned => json!({
            "error_type": "ActionJournalError",
            "error_variant": "StoragePoisoned",
        }),
        ActionJournalError::NatsConnect { reason } => json!({
            "error_type": "ActionJournalError",
            "error_variant": "NatsConnect",
            "reason": reason,
        }),
        ActionJournalError::NatsBucket { bucket, reason } => json!({
            "error_type": "ActionJournalError",
            "error_variant": "NatsBucket",
            "bucket": bucket,
            "reason": reason,
        }),
        ActionJournalError::RecordSerialization { reason } => json!({
            "error_type": "ActionJournalError",
            "error_variant": "RecordSerialization",
            "reason": reason,
        }),
        ActionJournalError::RecordDeserialization {
            stream,
            revision,
            reason,
        } => json!({
            "error_type": "ActionJournalError",
            "error_variant": "RecordDeserialization",
            "stream": stream,
            "revision": revision,
            "reason": reason,
        }),
        ActionJournalError::NatsAppend { stream, reason } => json!({
            "error_type": "ActionJournalError",
            "error_variant": "NatsAppend",
            "stream": stream,
            "reason": reason,
        }),
        ActionJournalError::NatsLoad { stream, reason } => json!({
            "error_type": "ActionJournalError",
            "error_variant": "NatsLoad",
            "stream": stream,
            "reason": reason,
        }),
    }
}

fn reaction_journal_error_details(error: &ReactionJournalError) -> Value {
    match error {
        ReactionJournalError::WrongReactionStream {
            expected_reaction_id,
            actual_reaction_id,
        } => json!({
            "error_type": "ReactionJournalError",
            "error_variant": "WrongReactionStream",
            "expected_reaction_id": expected_reaction_id,
            "actual_reaction_id": actual_reaction_id,
        }),
        ReactionJournalError::StoragePoisoned => json!({
            "error_type": "ReactionJournalError",
            "error_variant": "StoragePoisoned",
        }),
        ReactionJournalError::NatsConnect { reason } => json!({
            "error_type": "ReactionJournalError",
            "error_variant": "NatsConnect",
            "reason": reason,
        }),
        ReactionJournalError::NatsBucket { bucket, reason } => json!({
            "error_type": "ReactionJournalError",
            "error_variant": "NatsBucket",
            "bucket": bucket,
            "reason": reason,
        }),
        ReactionJournalError::RecordSerialization { reason } => json!({
            "error_type": "ReactionJournalError",
            "error_variant": "RecordSerialization",
            "reason": reason,
        }),
        ReactionJournalError::RecordDeserialization {
            stream,
            revision,
            reason,
        } => json!({
            "error_type": "ReactionJournalError",
            "error_variant": "RecordDeserialization",
            "stream": stream,
            "revision": revision,
            "reason": reason,
        }),
        ReactionJournalError::NatsAppend { stream, reason } => json!({
            "error_type": "ReactionJournalError",
            "error_variant": "NatsAppend",
            "stream": stream,
            "reason": reason,
        }),
        ReactionJournalError::NatsLoad { stream, reason } => json!({
            "error_type": "ReactionJournalError",
            "error_variant": "NatsLoad",
            "stream": stream,
            "reason": reason,
        }),
    }
}

fn matches_trigger<E>(trigger: &RecordedEvent) -> bool
where
    E: Event,
{
    trigger.metadata.message_type == E::EVENT_TYPE
        && trigger.metadata.schema_id == E::SCHEMA_ID
        && trigger.metadata.schema_version == E::SCHEMA_VERSION
        && trigger.metadata.resource_type == E::Resource::RESOURCE_TYPE
        && trigger.metadata.stream_type == StreamType::Resource
}

fn deterministic_identity(parts: &[(&str, &str)]) -> String {
    serde_json::to_string(parts).expect("deterministic identity parts should serialize")
}

fn deterministic_reaction_action_id<Rxn>(trigger: &RecordedEvent) -> String
where
    Rxn: Reaction,
{
    format!(
        "reaction_action:{}",
        deterministic_identity(&[
            ("reaction_type", Rxn::REACTION_TYPE),
            ("trigger_event_id", &trigger.metadata.message_id),
            ("action_type", <Rxn::Action as ActionTrait>::ACTION_TYPE),
        ])
    )
}

fn deterministic_reaction_action_metadata<Rxn>(trigger: &RecordedEvent) -> ActionMetadata
where
    Rxn: Reaction,
{
    ActionMetadata::with_ids(
        deterministic_reaction_action_id::<Rxn>(trigger),
        trigger.metadata.correlation_id.clone(),
        trigger.metadata.message_id.clone(),
        trigger.metadata.actor_id.clone(),
    )
}

fn reaction_journal_metadata(
    message_type: impl Into<String>,
    schema_id: impl Into<String>,
    trigger: &RecordedEvent,
    action: &ActionMetadata,
) -> MessageMetadata {
    let message_type = message_type.into();

    MessageMetadata {
        message_id: uuid::Uuid::new_v4().to_string(),
        message_type,
        message_version: 1,
        resource_type: trigger.metadata.resource_type.clone(),
        resource_id: trigger.metadata.resource_id.clone(),
        stream_type: StreamType::Reaction,
        correlation_id: action.correlation_id.clone(),
        causation_id: trigger.metadata.message_id.clone(),
        action_id: action.action_id.clone(),
        actor_id: action.actor_id.clone(),
        occurred_at: Utc::now().to_rfc3339(),
        schema_id: schema_id.into(),
        schema_version: 1,
    }
}
