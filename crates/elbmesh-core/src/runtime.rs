use std::marker::PhantomData;

use crate::{
    Action, ActionDecision, ActionError, ActionMetadata, ActionReceipt, ActionStatus, Apply,
    EmittedEvent, Event, EventStore, ExecutionError, ExpectedVersion, Handle, MessageMetadata,
    NewEvent, Resource, ResourceStream,
};

pub struct ActionContext<R: Resource> {
    metadata: ActionMetadata,
    resource_type: String,
    resource_id: String,
    current_version: u64,
    events: Vec<NewEvent>,
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
            _resource: PhantomData,
        }
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

pub struct ActionExecutor<S> {
    event_store: S,
}

impl<S> ActionExecutor<S>
where
    S: EventStore,
{
    pub fn new(event_store: S) -> Self {
        Self { event_store }
    }

    pub fn event_store(&self) -> &S {
        &self.event_store
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
        let stream = ResourceStream::new(R::RESOURCE_TYPE, resource_id.clone());
        let mut history = self.event_store.load(&stream).await?;
        history.sort_by_key(|event| event.sequence);
        let previous_version = history.last().map_or(0, |event| event.sequence);

        let mut resource = R::default();
        for event in &history {
            resource.apply_recorded(event)?;
        }

        let action_id = metadata.action_id.clone();
        let mut ctx = ActionContext::<R>::new(
            metadata,
            R::RESOURCE_TYPE,
            resource_id.clone(),
            previous_version,
        );

        let decision = resource.handle(action, &mut ctx).await?;
        let pending_events = ctx.into_events();

        let append_result = if pending_events.is_empty() {
            crate::AppendResult {
                previous_version,
                new_version: previous_version,
                events: Vec::new(),
            }
        } else {
            self.event_store
                .append(
                    &stream,
                    ExpectedVersion::Exact(previous_version),
                    pending_events,
                )
                .await?
        };

        Ok(receipt(
            action_id,
            R::RESOURCE_TYPE,
            resource_id,
            decision,
            append_result,
        ))
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
