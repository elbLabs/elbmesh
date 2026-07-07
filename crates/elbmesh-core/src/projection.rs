use async_trait::async_trait;
use thiserror::Error;

use crate::{Event, RecordedEvent, Resource, StreamType, ViewStore, ViewStoreError};

#[async_trait]
pub trait Projection: Send + Sync + 'static {
    type Source: Event;

    async fn project<V>(&self, event: Self::Source, view_store: &V) -> Result<(), ViewStoreError>
    where
        V: ViewStore;
}

#[derive(Debug, Error)]
pub enum ProjectionExecutionError {
    #[error(
        "failed to deserialize projection source event '{message_type}' v{schema_version}: {source}"
    )]
    SourceEventDeserialization {
        message_type: String,
        schema_version: u32,
        source: serde_json::Error,
    },

    #[error(transparent)]
    ViewStore(#[from] ViewStoreError),
}

pub struct ProjectionRuntime<V> {
    view_store: V,
}

impl<V> ProjectionRuntime<V>
where
    V: ViewStore,
{
    pub fn new(view_store: V) -> Self {
        Self { view_store }
    }

    pub fn view_store(&self) -> &V {
        &self.view_store
    }

    pub async fn apply<P>(
        &self,
        source: &RecordedEvent,
        projection: &P,
    ) -> Result<bool, ProjectionExecutionError>
    where
        P: Projection,
    {
        if !matches_source::<P::Source>(source) {
            return Ok(false);
        }

        let source_event = serde_json::from_value::<P::Source>(source.payload.clone()).map_err(
            |deserialize_source| ProjectionExecutionError::SourceEventDeserialization {
                message_type: source.metadata.message_type.clone(),
                schema_version: source.metadata.schema_version,
                source: deserialize_source,
            },
        )?;

        projection.project(source_event, &self.view_store).await?;
        Ok(true)
    }
}

fn matches_source<E>(source: &RecordedEvent) -> bool
where
    E: Event,
{
    source.metadata.message_type == E::EVENT_TYPE
        && source.metadata.schema_id == E::SCHEMA_ID
        && source.metadata.schema_version == E::SCHEMA_VERSION
        && source.metadata.resource_type == E::Resource::RESOURCE_TYPE
        && source.metadata.stream_type == StreamType::Resource
}
