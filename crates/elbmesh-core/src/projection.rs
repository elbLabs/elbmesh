use async_trait::async_trait;
use serde_json::{json, Value};
use thiserror::Error;

use crate::{Event, RecordedEvent, Resource, StreamType, ViewStore, ViewStoreError};

#[async_trait]
pub trait Projection: Send + Sync + 'static {
    type Source: Event;

    const PROJECTION_TYPE: &'static str;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionDispatchReport {
    pub applied: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectionDispatchFailure {
    pub projection_type: String,
    pub failure_code: String,
    pub failure_details: Value,
}

#[derive(Debug, Error, PartialEq)]
pub enum ProjectionDispatchError {
    #[error("projection dispatch failed for one or more handlers")]
    HandlerFailures {
        applied: usize,
        failures: Vec<ProjectionDispatchFailure>,
    },
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

#[async_trait]
trait EventProjectionHandler<V>: Send + Sync + 'static
where
    V: ViewStore,
{
    async fn handle(
        &self,
        runtime: &ProjectionRuntime<V>,
        source: &RecordedEvent,
    ) -> Result<bool, ProjectionDispatchFailure>;
}

pub struct TypedProjectionHandler<P> {
    projection: P,
}

impl<P> TypedProjectionHandler<P> {
    pub fn new(projection: P) -> Self {
        Self { projection }
    }
}

#[async_trait]
impl<V, P> EventProjectionHandler<V> for TypedProjectionHandler<P>
where
    V: ViewStore,
    P: Projection,
{
    async fn handle(
        &self,
        runtime: &ProjectionRuntime<V>,
        source: &RecordedEvent,
    ) -> Result<bool, ProjectionDispatchFailure> {
        runtime
            .apply(source, &self.projection)
            .await
            .map_err(projection_dispatch_failure::<P>)
    }
}

pub struct ProjectionDispatcher<V> {
    runtime: ProjectionRuntime<V>,
    handlers: Vec<Box<dyn EventProjectionHandler<V>>>,
}

impl<V> ProjectionDispatcher<V>
where
    V: ViewStore,
{
    pub fn new(runtime: ProjectionRuntime<V>) -> Self {
        Self {
            runtime,
            handlers: Vec::new(),
        }
    }

    pub fn view_store(&self) -> &V {
        self.runtime.view_store()
    }

    pub fn with_handler<P>(mut self, handler: TypedProjectionHandler<P>) -> Self
    where
        P: Projection,
    {
        self.handlers.push(Box::new(handler));
        self
    }

    pub async fn dispatch(
        &self,
        source: &RecordedEvent,
    ) -> Result<ProjectionDispatchReport, ProjectionDispatchError> {
        let mut applied = 0;
        let mut failures = Vec::new();

        for handler in &self.handlers {
            match handler.handle(&self.runtime, source).await {
                Ok(true) => applied += 1,
                Ok(false) => {}
                Err(failure) => failures.push(failure),
            }
        }

        if failures.is_empty() {
            Ok(ProjectionDispatchReport { applied })
        } else {
            Err(ProjectionDispatchError::HandlerFailures { applied, failures })
        }
    }
}

fn projection_dispatch_failure<P>(error: ProjectionExecutionError) -> ProjectionDispatchFailure
where
    P: Projection,
{
    ProjectionDispatchFailure {
        projection_type: P::PROJECTION_TYPE.to_string(),
        failure_code: projection_execution_failure_code(&error).to_string(),
        failure_details: projection_execution_failure_details(&error),
    }
}

fn projection_execution_failure_code(error: &ProjectionExecutionError) -> &'static str {
    match error {
        ProjectionExecutionError::SourceEventDeserialization { .. } => {
            "projection.source_event_deserialization"
        }
        ProjectionExecutionError::ViewStore(ViewStoreError::StoragePoisoned) => {
            "projection.view_store.storage_poisoned"
        }
    }
}

fn projection_execution_failure_details(error: &ProjectionExecutionError) -> Value {
    match error {
        ProjectionExecutionError::SourceEventDeserialization {
            message_type,
            schema_version,
            source,
        } => json!({
            "error_type": "ProjectionExecutionError",
            "error_variant": "SourceEventDeserialization",
            "message_type": message_type,
            "schema_version": schema_version,
            "source": source.to_string(),
        }),
        ProjectionExecutionError::ViewStore(ViewStoreError::StoragePoisoned) => json!({
            "error_type": "ViewStoreError",
            "error_variant": "StoragePoisoned",
        }),
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
