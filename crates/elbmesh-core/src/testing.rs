use std::fmt::Debug;
use std::marker::PhantomData;

use serde::Serialize;

use crate::{
    Action, ActionMetadata, Apply, Event, EventStore, ExpectedVersion, Handle, HandlerError,
    InMemoryEventStore, MessageMetadata, NewEvent, Resource, ResourceStream,
};

pub struct ActionScenario<R>
where
    R: Resource,
{
    given: Vec<(ResourceStream, NewEvent)>,
    _resource: PhantomData<R>,
}

impl<R> ActionScenario<R>
where
    R: Resource,
{
    pub fn new() -> Self {
        Self {
            given: Vec::new(),
            _resource: PhantomData,
        }
    }

    pub fn given<E>(mut self, events: Vec<E>) -> Self
    where
        E: Event<Resource = R>,
    {
        self.given.extend(events.into_iter().map(new_event));
        self
    }

    pub fn given_event<E>(mut self, event: E) -> Self
    where
        E: Event<Resource = R>,
    {
        self.given.push(new_event(event));
        self
    }

    pub fn when<A>(self, action: A) -> ScenarioWhen<R, A>
    where
        A: Action<Resource = R>,
    {
        ScenarioWhen {
            given: self.given,
            action,
            _resource: PhantomData,
        }
    }
}

impl<R> Default for ActionScenario<R>
where
    R: Resource,
{
    fn default() -> Self {
        Self::new()
    }
}

pub struct ScenarioWhen<R, A>
where
    R: Resource,
    A: Action<Resource = R>,
{
    given: Vec<(ResourceStream, NewEvent)>,
    action: A,
    _resource: PhantomData<R>,
}

impl<R, A> ScenarioWhen<R, A>
where
    R: Resource + Handle<A>,
    A: Action<Resource = R>,
{
    pub fn then<E>(self, events: Vec<E>) -> ScenarioThenEvents<R, A>
    where
        E: Event<Resource = R>,
    {
        ScenarioThenEvents {
            given: self.given,
            action: self.action,
            expected: events.into_iter().map(expected_event).collect(),
            _resource: PhantomData,
        }
    }

    pub fn then_error(self, error: <R as Handle<A>>::Error) -> ScenarioThenError<R, A> {
        ScenarioThenError {
            given: self.given,
            action: self.action,
            expected: error,
            _resource: PhantomData,
        }
    }
}

pub struct ScenarioThenEvents<R, A>
where
    R: Resource + Handle<A>,
    A: Action<Resource = R>,
{
    given: Vec<(ResourceStream, NewEvent)>,
    action: A,
    expected: Vec<ExpectedEvent>,
    _resource: PhantomData<R>,
}

impl<R, A> ScenarioThenEvents<R, A>
where
    R: Resource + Handle<A>,
    A: Action<Resource = R>,
    <R as Handle<A>>::Error: PartialEq + Debug,
{
    pub async fn assert(self) {
        let resource_id = self.action.resource_id().to_string();
        let stream = ResourceStream::new(R::RESOURCE_TYPE, resource_id);
        let store = store_with_given(self.given).await;
        let previous_version = store
            .load(&stream)
            .await
            .expect("scenario history should load")
            .len();

        let executor = crate::ActionExecutor::new(store.clone());
        executor
            .execute::<R, A>(self.action, scenario_metadata())
            .await
            .expect("scenario action should complete");

        let history = store
            .load(&stream)
            .await
            .expect("scenario events should load");
        let actual: Vec<_> = history
            .into_iter()
            .skip(previous_version)
            .map(|event| ExpectedEvent {
                message_type: event.metadata.message_type,
                schema_id: event.metadata.schema_id,
                schema_version: event.metadata.schema_version,
                payload: event.payload,
            })
            .collect();

        assert_eq!(self.expected, actual);
    }
}

pub struct ScenarioThenError<R, A>
where
    R: Resource + Handle<A>,
    A: Action<Resource = R>,
{
    given: Vec<(ResourceStream, NewEvent)>,
    action: A,
    expected: <R as Handle<A>>::Error,
    _resource: PhantomData<R>,
}

impl<R, A> ScenarioThenError<R, A>
where
    R: Resource + Handle<A>,
    A: Action<Resource = R>,
    <R as Handle<A>>::Error: PartialEq + Debug,
{
    pub async fn assert(self) {
        let store = store_with_given(self.given).await;
        let stream = ResourceStream::new(R::RESOURCE_TYPE, self.action.resource_id().to_string());
        let previous_version = store
            .load(&stream)
            .await
            .expect("scenario events should load")
            .len();
        let executor = crate::ActionExecutor::new(store);
        let actual = executor
            .execute::<R, A>(self.action, scenario_metadata())
            .await
            .expect_err("scenario action should fail");

        match actual {
            crate::ExecutionError::Handler(HandlerError::Domain { error }) => {
                assert_eq!(self.expected, error);
            }
            other => panic!("expected domain error, got {other:?}"),
        }

        let history = executor
            .event_store()
            .load(&stream)
            .await
            .expect("scenario events should load after failed action");
        assert_eq!(
            previous_version,
            history.len(),
            "scenario action should append no resource events on domain error"
        );
    }
}

#[derive(Debug, PartialEq)]
struct ExpectedEvent {
    message_type: String,
    schema_id: String,
    schema_version: u32,
    payload: serde_json::Value,
}

fn new_event<E>(event: E) -> (ResourceStream, NewEvent)
where
    E: Event,
{
    let resource_id = event.resource_id().to_string();
    let metadata = MessageMetadata::resource_event(
        E::EVENT_TYPE,
        E::SCHEMA_ID,
        E::SCHEMA_VERSION,
        E::Resource::RESOURCE_TYPE,
        resource_id.clone(),
        &scenario_metadata(),
    );
    let payload = serialize_event(event);

    (
        ResourceStream::new(E::Resource::RESOURCE_TYPE, resource_id),
        NewEvent { metadata, payload },
    )
}

fn expected_event<E>(event: E) -> ExpectedEvent
where
    E: Event,
{
    ExpectedEvent {
        message_type: E::EVENT_TYPE.to_string(),
        schema_id: E::SCHEMA_ID.to_string(),
        schema_version: E::SCHEMA_VERSION,
        payload: serialize_event(event),
    }
}

fn serialize_event<E>(event: E) -> serde_json::Value
where
    E: Serialize,
{
    serde_json::to_value(event).expect("scenario event should serialize")
}

async fn store_with_given(given: Vec<(ResourceStream, NewEvent)>) -> InMemoryEventStore {
    let store = InMemoryEventStore::new();

    for (stream, event) in given {
        store
            .append(&stream, ExpectedVersion::Any, vec![event])
            .await
            .expect("scenario given event should append");
    }

    store
}

fn scenario_metadata() -> ActionMetadata {
    ActionMetadata::with_ids(
        "scenario-action",
        "scenario-correlation",
        "scenario-cause",
        "scenario",
    )
}

#[allow(dead_code)]
fn _assert_apply_import<R, E>()
where
    R: Resource + Apply<E>,
    E: Event<Resource = R>,
{
}
