use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    ActionContext, ActionDecision, ActionFailure, HandlerError, RecordedEvent, ResourceError,
};

pub trait Resource: Default + Send + Sync + Sized + 'static {
    type Id: Clone + Send + Sync + ToString + 'static;

    const RESOURCE_TYPE: &'static str;

    fn apply_recorded(&mut self, event: &RecordedEvent) -> Result<(), ResourceError>;
}

pub trait Action: Serialize + DeserializeOwned + Send + Sync + 'static {
    type Resource: Resource;

    const ACTION_TYPE: &'static str;
    const SCHEMA_ID: &'static str;
    const SCHEMA_VERSION: u32;

    fn resource_id(&self) -> <Self::Resource as Resource>::Id;
}

pub trait Event: Serialize + DeserializeOwned + Send + Sync + 'static {
    type Resource: Resource;

    const EVENT_TYPE: &'static str;
    const SCHEMA_ID: &'static str;
    const SCHEMA_VERSION: u32;

    fn resource_id(&self) -> <Self::Resource as Resource>::Id;
}

pub trait Apply<E>: Resource
where
    E: Event<Resource = Self>,
{
    fn apply(&mut self, event: E) -> Result<(), ResourceError>;
}

#[async_trait]
pub trait Handle<A>: Resource
where
    A: Action<Resource = Self>,
{
    type Error: ActionFailure;

    async fn handle(
        &mut self,
        action: A,
        ctx: &mut ActionContext<Self>,
    ) -> Result<ActionDecision, HandlerError<Self::Error>>;
}

pub fn apply_recorded_event<R, E>(
    resource: &mut R,
    event: &RecordedEvent,
) -> Result<bool, ResourceError>
where
    R: Apply<E>,
    E: Event<Resource = R>,
{
    if event.metadata.message_type != E::EVENT_TYPE
        || event.metadata.schema_version != E::SCHEMA_VERSION
    {
        return Ok(false);
    }

    let typed = serde_json::from_value::<E>(event.payload.clone()).map_err(|source| {
        ResourceError::Deserialization {
            message_type: event.metadata.message_type.clone(),
            schema_version: event.metadata.schema_version,
            source,
        }
    })?;

    resource.apply(typed)?;
    Ok(true)
}
