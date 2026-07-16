use std::marker::PhantomData;
use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::{
    Action, ActionDecision, ActionError, ActionFailure, ActionFailureClassification, ActionJournal,
    ActionJournalRecord, ActionJournalStream, ActionMetadata, ActionReceipt, ActionStatus, Apply,
    EmittedEvent, Event, EventStore, ExecutionError, ExpectedVersion, Handle, HandlerError,
    MessageMetadata, NewEvent, OperationJournal, OperationJournalRecord, OperationJournalStream,
    Resource, ResourceStream, StreamType,
};
use crate::{ExternalOperation, ExternalOperationCall, ExternalOperationFailure};

pub struct ActionContext<R: Resource> {
    metadata: ActionMetadata,
    resource_type: String,
    resource_id: String,
    current_version: u64,
    events: Vec<NewEvent>,
    operation_journal: Option<Arc<dyn OperationJournal>>,
    _resource: PhantomData<R>,
}

impl<R: Resource> ActionContext<R> {
    pub fn new(
        metadata: ActionMetadata,
        resource_type: impl Into<String>,
        resource_id: impl Into<String>,
        current_version: u64,
    ) -> Self {
        Self {
            metadata,
            resource_type: resource_type.into(),
            resource_id: resource_id.into(),
            current_version,
            events: Vec::new(),
            operation_journal: None,
            _resource: PhantomData,
        }
    }

    fn with_operation_journal(
        mut self,
        operation_journal: Option<Arc<dyn OperationJournal>>,
    ) -> Self {
        self.operation_journal = operation_journal;
        self
    }

    pub fn metadata(&self) -> &ActionMetadata {
        &self.metadata
    }

    pub fn current_version(&self) -> u64 {
        self.current_version
    }

    pub fn record<E>(&mut self, event: E) -> Result<(), ActionError>
    where
        E: Event<Resource = R>,
    {
        let event = self.new_event::<E>(&event)?;
        self.events.push(event);

        Ok(())
    }

    pub fn record_applied<E>(&mut self, resource: &mut R, event: E) -> Result<(), ActionError>
    where
        R: Apply<E>,
        E: Event<Resource = R>,
    {
        let new_event = self.new_event::<E>(&event)?;
        resource
            .apply(event)
            .map_err(|err| ActionError::state_transition(err.to_string()))?;
        self.events.push(new_event);

        Ok(())
    }

    pub async fn execute_external_operation<O>(
        &self,
        operation: &O,
        request: O::Request,
    ) -> Result<O::Response, ActionError>
    where
        O: ExternalOperation,
    {
        let idempotency_key = operation.idempotency_key(&request);
        let call = ExternalOperationCall {
            operation_id: external_operation_id(
                &self.metadata.action_id,
                O::OPERATION_TYPE,
                &idempotency_key,
            ),
            operation_type: O::OPERATION_TYPE.to_string(),
            operation_schema_id: O::SCHEMA_ID.to_string(),
            operation_schema_version: O::SCHEMA_VERSION,
            idempotency_key,
        };

        if let Some(operation_journal) = &self.operation_journal {
            let stream = OperationJournalStream::for_operation(call.operation_id.clone());
            let records = operation_journal
                .load(&stream)
                .await
                .map_err(|error| ActionError::operation_journal(&call.operation_id, &error))?;

            if let Some(response) = records.iter().rev().find_map(|record| match record {
                OperationJournalRecord::OperationCompleted { response, .. } => Some(response),
                _ => None,
            }) {
                return serde_json::from_value(response.clone()).map_err(|error| {
                    ActionError::Serialization(format!(
                        "external operation '{}' completed response replay failed: {error}",
                        O::OPERATION_TYPE
                    ))
                });
            }

            let payload = serde_json::to_value(&request)
                .map_err(|error| ActionError::Serialization(error.to_string()))?;
            operation_journal
                .append(
                    &stream,
                    OperationJournalRecord::OperationCalled {
                        operation_id: call.operation_id.clone(),
                        metadata: operation_journal_metadata(
                            "operation_called",
                            "journal.operation_called.v1",
                            &self.resource_type,
                            &self.resource_id,
                            &self.metadata,
                        ),
                        operation_type: call.operation_type.clone(),
                        operation_schema_id: call.operation_schema_id.clone(),
                        operation_schema_version: call.operation_schema_version,
                        idempotency_key: call.idempotency_key.clone(),
                        payload,
                    },
                )
                .await
                .map_err(|error| ActionError::operation_journal(&call.operation_id, &error))?;

            let response = match operation.execute(request, call.clone()).await {
                Ok(response) => response,
                Err(error) => {
                    operation_journal
                        .append(
                            &stream,
                            OperationJournalRecord::OperationFailed {
                                operation_id: call.operation_id.clone(),
                                metadata: operation_journal_metadata(
                                    "operation_failed",
                                    "journal.operation_failed.v1",
                                    &self.resource_type,
                                    &self.resource_id,
                                    &self.metadata,
                                ),
                                failure_code: error.code().to_string(),
                                failure_details: error.details(),
                            },
                        )
                        .await
                        .map_err(|journal_error| {
                            ActionError::operation_journal(&call.operation_id, &journal_error)
                        })?;

                    return Err(ActionError::external_operation(O::OPERATION_TYPE, &error));
                }
            };
            let response_value = serde_json::to_value(&response)
                .map_err(|error| ActionError::Serialization(error.to_string()))?;
            operation_journal
                .append(
                    &stream,
                    OperationJournalRecord::OperationCompleted {
                        operation_id: call.operation_id.clone(),
                        metadata: operation_journal_metadata(
                            "operation_completed",
                            "journal.operation_completed.v1",
                            &self.resource_type,
                            &self.resource_id,
                            &self.metadata,
                        ),
                        response: response_value,
                    },
                )
                .await
                .map_err(|error| ActionError::operation_journal(&call.operation_id, &error))?;

            return Ok(response);
        }

        operation
            .execute(request, call)
            .await
            .map_err(|error| ActionError::external_operation(O::OPERATION_TYPE, &error))
    }

    fn new_event<E>(&self, event: &E) -> Result<NewEvent, ActionError>
    where
        E: Event<Resource = R>,
    {
        let actual_resource_id = event.resource_id().to_string();
        if actual_resource_id != self.resource_id {
            return Err(ActionError::WrongResource {
                expected: self.resource_id.clone(),
                actual: actual_resource_id,
            });
        }

        let payload = serde_json::to_value(event)
            .map_err(|err| ActionError::Serialization(err.to_string()))?;

        Ok(NewEvent {
            metadata: MessageMetadata::resource_event(
                E::EVENT_TYPE,
                E::SCHEMA_ID,
                E::SCHEMA_VERSION,
                self.resource_type.clone(),
                self.resource_id.clone(),
                &self.metadata,
            ),
            payload,
        })
    }

    pub fn pending_events(&self) -> &[NewEvent] {
        &self.events
    }

    pub fn into_events(self) -> Vec<NewEvent> {
        self.events
    }
}

fn external_operation_id(action_id: &str, operation_type: &str, idempotency_key: &str) -> String {
    format!(
        "action.{}.{}.operation.{}.{}.idempotency.{}.{}",
        action_id.len(),
        action_id,
        operation_type.len(),
        operation_type,
        idempotency_key.len(),
        idempotency_key
    )
}

pub struct ActionExecutor<S> {
    event_store: S,
    action_journal: Option<Arc<dyn ActionJournal>>,
    operation_journal: Option<Arc<dyn OperationJournal>>,
}

impl<S> ActionExecutor<S>
where
    S: EventStore,
{
    pub fn new(event_store: S) -> Self {
        Self {
            event_store,
            action_journal: None,
            operation_journal: None,
        }
    }

    pub fn event_store(&self) -> &S {
        &self.event_store
    }

    pub fn with_action_journal<J>(mut self, action_journal: J) -> Self
    where
        J: ActionJournal,
    {
        self.action_journal = Some(Arc::new(action_journal));
        self
    }

    pub fn with_operation_journal<J>(mut self, operation_journal: J) -> Self
    where
        J: OperationJournal,
    {
        self.operation_journal = Some(Arc::new(operation_journal));
        self
    }

    pub async fn execute<R, A>(
        &self,
        action: A,
        metadata: ActionMetadata,
    ) -> Result<ActionReceipt, ExecutionError<<R as Handle<A>>::Error>>
    where
        R: Resource + Handle<A>,
        A: Action<Resource = R>,
    {
        let resource_id = action.resource_id().to_string();
        let action_metadata = metadata.clone();
        let action_id = action_metadata.action_id.clone();
        let journal_stream = ActionJournalStream::for_action(action_id.clone());
        if let Some(action_journal) = &self.action_journal {
            let action_called =
                match action_called_record::<R, A>(&action_metadata, &resource_id, &action) {
                    Ok(record) => record,
                    Err(error) => {
                        append_action_failed::<R>(
                            Some(action_journal),
                            &journal_stream,
                            &action_metadata,
                            &resource_id,
                            ActionFailureClassification::HandlerRuntime,
                            error.code(),
                            error.details(),
                        )
                        .await;
                        return Err(HandlerError::Runtime(error).into());
                    }
                };

            action_journal
                .append(&journal_stream, action_called)
                .await?;
        }

        let stream = ResourceStream::new(R::RESOURCE_TYPE, resource_id.clone());
        let mut history = match self.event_store.load(&stream).await {
            Ok(history) => history,
            Err(error) => {
                append_action_failed::<R>(
                    self.action_journal.as_ref(),
                    &journal_stream,
                    &action_metadata,
                    &resource_id,
                    ActionFailureClassification::EventStore,
                    error.code(),
                    error.details(),
                )
                .await;
                return Err(error.into());
            }
        };
        history.sort_by_key(|event| event.sequence);
        let previous_version = history.last().map_or(0, |event| event.sequence);

        let mut resource = R::default();
        for event in &history {
            if let Err(error) = resource.apply_recorded(event) {
                append_action_failed::<R>(
                    self.action_journal.as_ref(),
                    &journal_stream,
                    &action_metadata,
                    &resource_id,
                    ActionFailureClassification::Resource,
                    error.code(),
                    error.details(),
                )
                .await;
                return Err(error.into());
            }
        }

        let mut ctx = ActionContext::<R>::new(
            metadata,
            R::RESOURCE_TYPE,
            resource_id.clone(),
            previous_version,
        )
        .with_operation_journal(self.operation_journal.clone());

        let decision = match resource.handle(action, &mut ctx).await {
            Ok(decision) => decision,
            Err(HandlerError::Domain { error }) => {
                if let Some(action_journal) = &self.action_journal {
                    action_journal
                        .append(
                            &journal_stream,
                            action_rejected_record::<R, A>(&action_metadata, &resource_id, &error),
                        )
                        .await?;
                }

                return Err(HandlerError::Domain { error }.into());
            }
            Err(HandlerError::Runtime(error)) => {
                append_action_failed::<R>(
                    self.action_journal.as_ref(),
                    &journal_stream,
                    &action_metadata,
                    &resource_id,
                    ActionFailureClassification::HandlerRuntime,
                    error.code(),
                    error.details(),
                )
                .await;
                return Err(HandlerError::Runtime(error).into());
            }
        };
        let pending_events = ctx.into_events();

        let append_result = if pending_events.is_empty() {
            crate::AppendResult {
                previous_version,
                new_version: previous_version,
                events: Vec::new(),
            }
        } else {
            match self
                .event_store
                .append(
                    &stream,
                    ExpectedVersion::Exact(previous_version),
                    pending_events,
                )
                .await
            {
                Ok(append_result) => append_result,
                Err(error) => {
                    append_action_failed::<R>(
                        self.action_journal.as_ref(),
                        &journal_stream,
                        &action_metadata,
                        &resource_id,
                        ActionFailureClassification::EventStore,
                        error.code(),
                        error.details(),
                    )
                    .await;
                    return Err(error.into());
                }
            }
        };

        let receipt = receipt(
            action_id,
            R::RESOURCE_TYPE,
            resource_id,
            decision,
            append_result,
        );

        if let Some(action_journal) = &self.action_journal {
            action_journal
                .append(
                    &journal_stream,
                    ActionJournalRecord::ActionCompleted {
                        metadata: action_journal_metadata(
                            "action_completed",
                            "journal.action_completed.v1",
                            &receipt.resource_type,
                            &receipt.resource_id,
                            &action_metadata,
                        ),
                        receipt: receipt.clone(),
                    },
                )
                .await?;
        }

        Ok(receipt)
    }
}

async fn append_action_failed<R>(
    action_journal: Option<&Arc<dyn ActionJournal>>,
    journal_stream: &ActionJournalStream,
    action_metadata: &ActionMetadata,
    resource_id: &str,
    failure_classification: ActionFailureClassification,
    failure_code: &str,
    failure_details: serde_json::Value,
) where
    R: Resource,
{
    if let Some(action_journal) = action_journal {
        // Preserve the caller-facing runtime error if failure journaling also fails.
        let _ = action_journal
            .append(
                journal_stream,
                action_failed_record::<R>(
                    action_metadata,
                    resource_id,
                    failure_classification,
                    failure_code,
                    failure_details,
                ),
            )
            .await;
    }
}

fn action_called_record<R, A>(
    action_metadata: &ActionMetadata,
    resource_id: &str,
    action: &A,
) -> Result<ActionJournalRecord, ActionError>
where
    R: Resource,
    A: Action<Resource = R>,
{
    let payload =
        serde_json::to_value(action).map_err(|err| ActionError::Serialization(err.to_string()))?;

    Ok(ActionJournalRecord::ActionCalled {
        metadata: action_journal_metadata(
            "action_called",
            "journal.action_called.v1",
            R::RESOURCE_TYPE,
            resource_id,
            action_metadata,
        ),
        action_type: A::ACTION_TYPE.to_string(),
        action_schema_id: A::SCHEMA_ID.to_string(),
        action_schema_version: A::SCHEMA_VERSION,
        payload,
    })
}

fn action_rejected_record<R, A>(
    action_metadata: &ActionMetadata,
    resource_id: &str,
    error: &<R as Handle<A>>::Error,
) -> ActionJournalRecord
where
    R: Resource + Handle<A>,
    A: Action<Resource = R>,
{
    ActionJournalRecord::ActionRejected {
        metadata: action_journal_metadata(
            "action_rejected",
            "journal.action_rejected.v1",
            R::RESOURCE_TYPE,
            resource_id,
            action_metadata,
        ),
        failure_code: error.code().to_string(),
        failure_details: error.details(),
    }
}

fn action_failed_record<R>(
    action_metadata: &ActionMetadata,
    resource_id: &str,
    failure_classification: ActionFailureClassification,
    failure_code: &str,
    failure_details: serde_json::Value,
) -> ActionJournalRecord
where
    R: Resource,
{
    ActionJournalRecord::ActionFailed {
        metadata: action_journal_metadata(
            "action_failed",
            "journal.action_failed.v1",
            R::RESOURCE_TYPE,
            resource_id,
            action_metadata,
        ),
        failure_classification,
        failure_details: serde_json::json!({
            "failure_code": failure_code,
            "failure_details": failure_details,
        }),
    }
}

fn action_journal_metadata(
    message_type: impl Into<String>,
    schema_id: impl Into<String>,
    resource_type: impl Into<String>,
    resource_id: impl Into<String>,
    action: &ActionMetadata,
) -> MessageMetadata {
    MessageMetadata {
        message_id: Uuid::new_v4().to_string(),
        message_type: message_type.into(),
        message_version: 1,
        resource_type: resource_type.into(),
        resource_id: resource_id.into(),
        stream_type: StreamType::Action,
        correlation_id: action.correlation_id.clone(),
        causation_id: action.causation_id.clone(),
        action_id: action.action_id.clone(),
        actor_id: action.actor_id.clone(),
        occurred_at: Utc::now().to_rfc3339(),
        schema_id: schema_id.into(),
        schema_version: 1,
    }
}

fn operation_journal_metadata(
    message_type: impl Into<String>,
    schema_id: impl Into<String>,
    resource_type: impl Into<String>,
    resource_id: impl Into<String>,
    action: &ActionMetadata,
) -> MessageMetadata {
    MessageMetadata {
        message_id: Uuid::new_v4().to_string(),
        message_type: message_type.into(),
        message_version: 1,
        resource_type: resource_type.into(),
        resource_id: resource_id.into(),
        stream_type: StreamType::Operation,
        correlation_id: action.correlation_id.clone(),
        causation_id: action.causation_id.clone(),
        action_id: action.action_id.clone(),
        actor_id: action.actor_id.clone(),
        occurred_at: Utc::now().to_rfc3339(),
        schema_id: schema_id.into(),
        schema_version: 1,
    }
}

fn receipt(
    action_id: String,
    resource_type: impl Into<String>,
    resource_id: impl Into<String>,
    decision: ActionDecision,
    append_result: crate::AppendResult,
) -> ActionReceipt {
    let emitted_events = append_result
        .events
        .iter()
        .map(|event| EmittedEvent {
            message_id: event.metadata.message_id.clone(),
            message_type: event.metadata.message_type.clone(),
            schema_id: event.metadata.schema_id.clone(),
            schema_version: event.metadata.schema_version,
            sequence: event.sequence,
        })
        .collect();

    ActionReceipt {
        action_id,
        status: ActionStatus::Completed,
        resource_type: resource_type.into(),
        resource_id: resource_id.into(),
        previous_version: append_result.previous_version,
        new_version: append_result.new_version,
        emitted_events,
        message: decision.message,
    }
}

#[allow(dead_code)]
fn _assert_apply_object_safety<R, E>()
where
    R: Resource + Apply<E>,
    E: Event<Resource = R>,
{
}
