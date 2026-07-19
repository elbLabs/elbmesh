use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use tokio::sync::Mutex;

use crate::{
    Event, RecordedEvent, Resource, ResourceStream, StreamType, ViewDocument, ViewKey, ViewStore,
    ViewStoreError,
};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionCursor {
    bytes: Vec<u8>,
}

impl ProjectionCursor {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionContext {
    source_message_id: String,
    source_stream: ResourceStream,
    aggregate_sequence: u64,
    transport_cursor: ProjectionCursor,
}

impl ProjectionContext {
    pub fn new(
        source_message_id: impl Into<String>,
        source_stream: ResourceStream,
        aggregate_sequence: u64,
        transport_cursor: ProjectionCursor,
    ) -> Self {
        Self {
            source_message_id: source_message_id.into(),
            source_stream,
            aggregate_sequence,
            transport_cursor,
        }
    }

    fn from_source(source: &RecordedEvent, transport_cursor: ProjectionCursor) -> Self {
        Self::new(
            source.metadata.message_id.clone(),
            source.stream.clone(),
            source.sequence,
            transport_cursor,
        )
    }

    pub fn source_message_id(&self) -> &str {
        &self.source_message_id
    }

    pub fn source_stream(&self) -> &ResourceStream {
        &self.source_stream
    }

    pub fn aggregate_sequence(&self) -> u64 {
        self.aggregate_sequence
    }

    pub fn transport_cursor(&self) -> &ProjectionCursor {
        &self.transport_cursor
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionCheckpoint {
    source_message_id: String,
    source_stream: ResourceStream,
    aggregate_sequence: u64,
    transport_cursor: ProjectionCursor,
}

impl ProjectionCheckpoint {
    pub fn new(
        source_message_id: impl Into<String>,
        source_stream: ResourceStream,
        aggregate_sequence: u64,
        transport_cursor: ProjectionCursor,
    ) -> Self {
        Self {
            source_message_id: source_message_id.into(),
            source_stream,
            aggregate_sequence,
            transport_cursor,
        }
    }

    fn from_context(context: &ProjectionContext) -> Self {
        Self::new(
            context.source_message_id.clone(),
            context.source_stream.clone(),
            context.aggregate_sequence,
            context.transport_cursor.clone(),
        )
    }

    pub fn source_message_id(&self) -> &str {
        &self.source_message_id
    }

    pub fn source_stream(&self) -> &ResourceStream {
        &self.source_stream
    }

    pub fn aggregate_sequence(&self) -> u64 {
        self.aggregate_sequence
    }

    pub fn transport_cursor(&self) -> &ProjectionCursor {
        &self.transport_cursor
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProjectionCheckpointError {
    #[error("failed to load projection checkpoint '{checkpoint_id}': {reason}")]
    Load {
        checkpoint_id: String,
        reason: String,
    },

    #[error("failed to save projection checkpoint '{checkpoint_id}': {reason}")]
    Save {
        checkpoint_id: String,
        reason: String,
    },

    #[error("failed to reset projection checkpoint '{checkpoint_id}': {reason}")]
    Reset {
        checkpoint_id: String,
        reason: String,
    },

    #[error("failed to reset Views for projection checkpoint '{checkpoint_id}': {reason}")]
    ViewReset {
        checkpoint_id: String,
        reason: String,
    },

    #[error("projection dispatcher has no checkpoint configured")]
    NotConfigured,

    #[error("a projection rebuild is already in progress for checkpoint '{checkpoint_id}'")]
    RebuildInProgress { checkpoint_id: String },
}

#[async_trait]
pub trait ProjectionCheckpointStore: Send + Sync + 'static {
    async fn load(
        &self,
        checkpoint_id: &str,
    ) -> Result<Option<ProjectionCheckpoint>, ProjectionCheckpointError>;

    async fn save(
        &self,
        checkpoint_id: &str,
        checkpoint: ProjectionCheckpoint,
    ) -> Result<(), ProjectionCheckpointError>;

    async fn reset(&self, checkpoint_id: &str) -> Result<(), ProjectionCheckpointError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionRebuildSelection {
    targets: Vec<ProjectionRebuildTarget>,
}

impl ProjectionRebuildSelection {
    pub fn new(targets: Vec<(String, ViewKey)>) -> Self {
        Self {
            targets: targets
                .into_iter()
                .map(|(projection_type, view_key)| ProjectionRebuildTarget {
                    projection_type,
                    view_key,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectionRebuildTarget {
    projection_type: String,
    view_key: ViewKey,
}

#[async_trait]
pub trait Projection: Send + Sync + 'static {
    type Source: Event;

    const PROJECTION_TYPE: &'static str;

    async fn project<V>(&self, event: Self::Source, view_store: &V) -> Result<(), ViewStoreError>
    where
        V: ViewStore;

    async fn project_with_context<V>(
        &self,
        event: Self::Source,
        _context: &ProjectionContext,
        view_store: &V,
    ) -> Result<(), ViewStoreError>
    where
        V: ViewStore,
    {
        self.project(event, view_store).await
    }
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

    #[error("projection source stream identity mismatch: metadata resource '{metadata_resource_type}/{metadata_resource_id}', stream resource '{stream_resource_type}/{stream_resource_id}'")]
    SourceStreamIdentityMismatch {
        metadata_resource_type: String,
        metadata_resource_id: String,
        stream_resource_type: String,
        stream_resource_id: String,
    },

    #[error("projection source payload identity mismatch: metadata resource id '{metadata_resource_id}', payload resource id '{payload_resource_id}'")]
    SourcePayloadIdentityMismatch {
        metadata_resource_id: String,
        payload_resource_id: String,
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

    #[error(transparent)]
    Checkpoint(#[from] ProjectionCheckpointError),

    #[error("projection rebuild is in progress for checkpoint '{checkpoint_id}'")]
    RebuildInProgress { checkpoint_id: String },
}

pub struct ProjectionRuntime<V> {
    view_store: Arc<V>,
    application_positions: Arc<Mutex<HashMap<ProjectionApplicationKey, u64>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ProjectionApplicationKey {
    projection_type: String,
    source_stream: ResourceStream,
    target_view: ViewKey,
}

struct SourceAwareViewStore<V> {
    view_store: Arc<V>,
    application_positions: Arc<Mutex<HashMap<ProjectionApplicationKey, u64>>>,
    projection_type: String,
    context: ProjectionContext,
}

#[async_trait]
impl<V> ViewStore for SourceAwareViewStore<V>
where
    V: ViewStore,
{
    async fn put(&self, document: ViewDocument) -> Result<(), ViewStoreError> {
        let application_key = ProjectionApplicationKey {
            projection_type: self.projection_type.clone(),
            source_stream: self.context.source_stream.clone(),
            target_view: document.key.clone(),
        };
        let mut positions = self.application_positions.lock().await;

        if positions
            .get(&application_key)
            .is_some_and(|sequence| *sequence >= self.context.aggregate_sequence)
        {
            return Ok(());
        }

        if self
            .view_store
            .apply_projection(&self.projection_type, &self.context, document)
            .await?
        {
            positions.insert(application_key, self.context.aggregate_sequence);
        }

        Ok(())
    }

    async fn load(&self, key: &ViewKey) -> Result<Option<ViewDocument>, ViewStoreError> {
        self.view_store.load(key).await
    }

    async fn list_by_index_prefix(
        &self,
        view_type: &str,
        index_name: &str,
        prefix: &str,
    ) -> Result<Vec<ViewDocument>, ViewStoreError> {
        self.view_store
            .list_by_index_prefix(view_type, index_name, prefix)
            .await
    }
}

impl<V> ProjectionRuntime<V>
where
    V: ViewStore,
{
    pub fn new(view_store: V) -> Self {
        Self {
            view_store: Arc::new(view_store),
            application_positions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn view_store(&self) -> &V {
        self.view_store.as_ref()
    }

    pub async fn apply<P>(
        &self,
        source: &RecordedEvent,
        projection: &P,
    ) -> Result<bool, ProjectionExecutionError>
    where
        P: Projection,
    {
        let context = ProjectionContext::from_source(source, ProjectionCursor::default());
        self.apply_with_context(source, &context, projection).await
    }

    async fn apply_with_context<P>(
        &self,
        source: &RecordedEvent,
        context: &ProjectionContext,
        projection: &P,
    ) -> Result<bool, ProjectionExecutionError>
    where
        P: Projection,
    {
        if !matches_source::<P::Source>(source) {
            return Ok(false);
        }

        if source.stream.resource_type != source.metadata.resource_type
            || source.stream.resource_id != source.metadata.resource_id
        {
            return Err(ProjectionExecutionError::SourceStreamIdentityMismatch {
                metadata_resource_type: source.metadata.resource_type.clone(),
                metadata_resource_id: source.metadata.resource_id.clone(),
                stream_resource_type: source.stream.resource_type.clone(),
                stream_resource_id: source.stream.resource_id.clone(),
            });
        }

        let source_event = serde_json::from_value::<P::Source>(source.payload.clone()).map_err(
            |deserialize_source| ProjectionExecutionError::SourceEventDeserialization {
                message_type: source.metadata.message_type.clone(),
                schema_version: source.metadata.schema_version,
                source: deserialize_source,
            },
        )?;

        let payload_resource_id = source_event.resource_id().to_string();
        if payload_resource_id != source.metadata.resource_id {
            return Err(ProjectionExecutionError::SourcePayloadIdentityMismatch {
                metadata_resource_id: source.metadata.resource_id.clone(),
                payload_resource_id,
            });
        }

        let view_store = SourceAwareViewStore {
            view_store: Arc::clone(&self.view_store),
            application_positions: Arc::clone(&self.application_positions),
            projection_type: P::PROJECTION_TYPE.to_string(),
            context: context.clone(),
        };
        projection
            .project_with_context(source_event, context, &view_store)
            .await?;
        Ok(true)
    }

    async fn reset_projection(
        &self,
        projection_type: &str,
        view_key: &ViewKey,
    ) -> Result<(), ViewStoreError> {
        self.view_store
            .reset_projection(projection_type, view_key)
            .await?;
        self.application_positions.lock().await.retain(|key, _| {
            key.projection_type != projection_type || key.target_view != *view_key
        });
        Ok(())
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
        context: &ProjectionContext,
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
        context: &ProjectionContext,
    ) -> Result<bool, ProjectionDispatchFailure> {
        runtime
            .apply_with_context(source, context, &self.projection)
            .await
            .map_err(projection_dispatch_failure::<P>)
    }
}

struct ProjectionCheckpointBinding {
    checkpoint_id: String,
    store: Arc<dyn ProjectionCheckpointStore>,
}

pub struct ProjectionDispatcher<V> {
    runtime: ProjectionRuntime<V>,
    handlers: Vec<Box<dyn EventProjectionHandler<V>>>,
    checkpoint: Option<ProjectionCheckpointBinding>,
    rebuild_in_progress: AtomicBool,
    delivery_gate: Mutex<()>,
}

impl<V> ProjectionDispatcher<V>
where
    V: ViewStore,
{
    pub fn new(runtime: ProjectionRuntime<V>) -> Self {
        Self {
            runtime,
            handlers: Vec::new(),
            checkpoint: None,
            rebuild_in_progress: AtomicBool::new(false),
            delivery_gate: Mutex::new(()),
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

    pub fn with_checkpoint<C>(
        mut self,
        checkpoint_id: impl Into<String>,
        checkpoint_store: C,
    ) -> Self
    where
        C: ProjectionCheckpointStore,
    {
        self.checkpoint = Some(ProjectionCheckpointBinding {
            checkpoint_id: checkpoint_id.into(),
            store: Arc::new(checkpoint_store),
        });
        self
    }

    pub async fn dispatch(
        &self,
        source: &RecordedEvent,
    ) -> Result<ProjectionDispatchReport, ProjectionDispatchError> {
        self.dispatch_with_cursor(source, ProjectionCursor::default())
            .await
    }

    pub async fn dispatch_with_cursor(
        &self,
        source: &RecordedEvent,
        cursor: ProjectionCursor,
    ) -> Result<ProjectionDispatchReport, ProjectionDispatchError> {
        self.dispatch_internal(source, cursor, false).await
    }

    pub async fn begin_rebuild(
        &self,
        selection: ProjectionRebuildSelection,
    ) -> Result<ProjectionRebuild<'_, V>, ProjectionCheckpointError> {
        let checkpoint = self
            .checkpoint
            .as_ref()
            .ok_or(ProjectionCheckpointError::NotConfigured)?;
        if self
            .rebuild_in_progress
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(ProjectionCheckpointError::RebuildInProgress {
                checkpoint_id: checkpoint.checkpoint_id.clone(),
            });
        }

        let reset_result = async {
            let _delivery = self.delivery_gate.lock().await;
            for target in selection.targets {
                self.runtime
                    .reset_projection(&target.projection_type, &target.view_key)
                    .await
                    .map_err(|source| ProjectionCheckpointError::ViewReset {
                        checkpoint_id: checkpoint.checkpoint_id.clone(),
                        reason: source.to_string(),
                    })?;
            }
            checkpoint.store.reset(&checkpoint.checkpoint_id).await
        }
        .await;

        if let Err(error) = reset_result {
            self.rebuild_in_progress.store(false, Ordering::SeqCst);
            return Err(error);
        }

        Ok(ProjectionRebuild { dispatcher: self })
    }

    async fn dispatch_internal(
        &self,
        source: &RecordedEvent,
        cursor: ProjectionCursor,
        allow_during_rebuild: bool,
    ) -> Result<ProjectionDispatchReport, ProjectionDispatchError> {
        self.ensure_delivery_allowed(allow_during_rebuild)?;
        let _delivery = self.delivery_gate.lock().await;
        self.ensure_delivery_allowed(allow_during_rebuild)?;
        let context = ProjectionContext::from_source(source, cursor);
        let mut applied = 0;
        let mut failures = Vec::new();

        for handler in &self.handlers {
            match handler.handle(&self.runtime, source, &context).await {
                Ok(true) => applied += 1,
                Ok(false) => {}
                Err(failure) => failures.push(failure),
            }
        }

        if !failures.is_empty() {
            return Err(ProjectionDispatchError::HandlerFailures { applied, failures });
        }

        if let Some(checkpoint) = &self.checkpoint {
            checkpoint
                .store
                .save(
                    &checkpoint.checkpoint_id,
                    ProjectionCheckpoint::from_context(&context),
                )
                .await?;
        }

        Ok(ProjectionDispatchReport { applied })
    }

    fn ensure_delivery_allowed(
        &self,
        allow_during_rebuild: bool,
    ) -> Result<(), ProjectionDispatchError> {
        if !allow_during_rebuild && self.rebuild_in_progress.load(Ordering::SeqCst) {
            let checkpoint_id = self
                .checkpoint
                .as_ref()
                .map(|checkpoint| checkpoint.checkpoint_id.clone())
                .unwrap_or_default();
            return Err(ProjectionDispatchError::RebuildInProgress { checkpoint_id });
        }

        Ok(())
    }
}

pub struct ProjectionRebuild<'a, V>
where
    V: ViewStore,
{
    dispatcher: &'a ProjectionDispatcher<V>,
}

impl<V> ProjectionRebuild<'_, V>
where
    V: ViewStore,
{
    pub async fn replay(
        &self,
        source: &RecordedEvent,
        cursor: ProjectionCursor,
    ) -> Result<ProjectionDispatchReport, ProjectionDispatchError> {
        self.dispatcher
            .dispatch_internal(source, cursor, true)
            .await
    }

    pub async fn finish(self) -> Result<(), ProjectionCheckpointError> {
        self.dispatcher
            .rebuild_in_progress
            .store(false, Ordering::SeqCst);
        Ok(())
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
        ProjectionExecutionError::SourceStreamIdentityMismatch { .. } => {
            "projection.source_stream_identity_mismatch"
        }
        ProjectionExecutionError::SourcePayloadIdentityMismatch { .. } => {
            "projection.source_payload_identity_mismatch"
        }
        ProjectionExecutionError::ViewStore(ViewStoreError::StoragePoisoned) => {
            "projection.view_store.storage_poisoned"
        }
        ProjectionExecutionError::ViewStore(ViewStoreError::DuplicateIndexName { .. }) => {
            "projection.view_store.duplicate_index_name"
        }
        ProjectionExecutionError::ViewStore(ViewStoreError::DeleteUnsupported { .. }) => {
            "projection.view_store.delete_unsupported"
        }
        ProjectionExecutionError::ViewStore(ViewStoreError::NatsConnect { .. }) => {
            "projection.view_store.nats_connect"
        }
        ProjectionExecutionError::ViewStore(ViewStoreError::NatsBucket { .. }) => {
            "projection.view_store.nats_bucket"
        }
        ProjectionExecutionError::ViewStore(ViewStoreError::DocumentSerialization { .. }) => {
            "projection.view_store.document_serialization"
        }
        ProjectionExecutionError::ViewStore(ViewStoreError::DocumentDeserialization { .. }) => {
            "projection.view_store.document_deserialization"
        }
        ProjectionExecutionError::ViewStore(ViewStoreError::NatsPut { .. }) => {
            "projection.view_store.nats_put"
        }
        ProjectionExecutionError::ViewStore(ViewStoreError::NatsDelete { .. }) => {
            "projection.view_store.nats_delete"
        }
        ProjectionExecutionError::ViewStore(ViewStoreError::NatsLoad { .. }) => {
            "projection.view_store.nats_load"
        }
        ProjectionExecutionError::ViewStore(ViewStoreError::NatsList { .. }) => {
            "projection.view_store.nats_list"
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
        ProjectionExecutionError::SourceStreamIdentityMismatch {
            metadata_resource_type,
            metadata_resource_id,
            stream_resource_type,
            stream_resource_id,
        } => json!({
            "error_type": "ProjectionExecutionError",
            "error_variant": "SourceStreamIdentityMismatch",
            "metadata_resource_type": metadata_resource_type,
            "metadata_resource_id": metadata_resource_id,
            "stream_resource_type": stream_resource_type,
            "stream_resource_id": stream_resource_id,
        }),
        ProjectionExecutionError::SourcePayloadIdentityMismatch {
            metadata_resource_id,
            payload_resource_id,
        } => json!({
            "error_type": "ProjectionExecutionError",
            "error_variant": "SourcePayloadIdentityMismatch",
            "metadata_resource_id": metadata_resource_id,
            "payload_resource_id": payload_resource_id,
        }),
        ProjectionExecutionError::ViewStore(ViewStoreError::StoragePoisoned) => json!({
            "error_type": "ViewStoreError",
            "error_variant": "StoragePoisoned",
        }),
        ProjectionExecutionError::ViewStore(ViewStoreError::DuplicateIndexName {
            view_type,
            view_id,
            index_name,
        }) => json!({
            "error_type": "ViewStoreError",
            "error_variant": "DuplicateIndexName",
            "view_type": view_type,
            "view_id": view_id,
            "index_name": index_name,
        }),
        ProjectionExecutionError::ViewStore(ViewStoreError::DeleteUnsupported {
            view_type,
            view_id,
        }) => json!({
            "error_type": "ViewStoreError",
            "error_variant": "DeleteUnsupported",
            "view_type": view_type,
            "view_id": view_id,
        }),
        ProjectionExecutionError::ViewStore(ViewStoreError::NatsConnect { reason }) => json!({
            "error_type": "ViewStoreError",
            "error_variant": "NatsConnect",
            "reason": reason,
        }),
        ProjectionExecutionError::ViewStore(ViewStoreError::NatsBucket { bucket, reason }) => {
            json!({
                "error_type": "ViewStoreError",
                "error_variant": "NatsBucket",
                "bucket": bucket,
                "reason": reason,
            })
        }
        ProjectionExecutionError::ViewStore(ViewStoreError::DocumentSerialization {
            view_type,
            view_id,
            reason,
        }) => json!({
            "error_type": "ViewStoreError",
            "error_variant": "DocumentSerialization",
            "view_type": view_type,
            "view_id": view_id,
            "reason": reason,
        }),
        ProjectionExecutionError::ViewStore(ViewStoreError::DocumentDeserialization {
            key,
            revision,
            reason,
        }) => json!({
            "error_type": "ViewStoreError",
            "error_variant": "DocumentDeserialization",
            "key": key,
            "revision": revision,
            "reason": reason,
        }),
        ProjectionExecutionError::ViewStore(ViewStoreError::NatsPut {
            view_type,
            view_id,
            reason,
        }) => json!({
            "error_type": "ViewStoreError",
            "error_variant": "NatsPut",
            "view_type": view_type,
            "view_id": view_id,
            "reason": reason,
        }),
        ProjectionExecutionError::ViewStore(ViewStoreError::NatsDelete {
            view_type,
            view_id,
            reason,
        }) => json!({
            "error_type": "ViewStoreError",
            "error_variant": "NatsDelete",
            "view_type": view_type,
            "view_id": view_id,
            "reason": reason,
        }),
        ProjectionExecutionError::ViewStore(ViewStoreError::NatsLoad {
            view_type,
            view_id,
            reason,
        }) => json!({
            "error_type": "ViewStoreError",
            "error_variant": "NatsLoad",
            "view_type": view_type,
            "view_id": view_id,
            "reason": reason,
        }),
        ProjectionExecutionError::ViewStore(ViewStoreError::NatsList {
            view_type,
            index_name,
            prefix,
            reason,
        }) => json!({
            "error_type": "ViewStoreError",
            "error_variant": "NatsList",
            "view_type": view_type,
            "index_name": index_name,
            "prefix": prefix,
            "reason": reason,
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
