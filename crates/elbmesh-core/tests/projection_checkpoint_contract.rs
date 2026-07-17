use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use elbmesh_core::{
    Event, InMemoryViewStore, Projection, ProjectionCheckpoint, ProjectionCheckpointError,
    ProjectionCheckpointStore, ProjectionContext, ProjectionCursor, ProjectionDispatchError,
    ProjectionDispatcher, ProjectionRebuildSelection, ProjectionRuntime, RecordedEvent, Resource,
    ResourceError, ResourceStream, StreamType, TypedProjectionHandler, ViewDocument, ViewKey,
    ViewStore, ViewStoreError,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

const CHECKPOINT_ID: &str = "offer-read-model";

#[tokio::test]
async fn projection_handler_receives_complete_source_context_with_opaque_cursor() {
    let seen_context = Arc::new(Mutex::new(None));
    let projection = ContextCapturingProjection {
        seen_context: Arc::clone(&seen_context),
    };
    let dispatcher = ProjectionDispatcher::new(ProjectionRuntime::new(InMemoryViewStore::new()))
        .with_handler(TypedProjectionHandler::new(projection));
    let mut source = offer_created_recorded_event("offer-1", "Initial offer");
    source.sequence = 7;
    let cursor = ProjectionCursor::new(vec![0x00, 0xff, 0x2a]);

    dispatcher
        .dispatch_with_cursor(&source, cursor.clone())
        .await
        .expect("projection should receive source context");

    let context = seen_context
        .lock()
        .expect("context capture lock")
        .clone()
        .expect("projection should observe context");
    assert_eq!(context.source_message_id(), "offer-created-event-1");
    assert_eq!(
        context.source_stream(),
        &ResourceStream::new("offer", "offer-1")
    );
    assert_eq!(context.aggregate_sequence(), 7);
    assert_eq!(context.transport_cursor(), &cursor);
}

#[tokio::test]
async fn event_checkpoint_waits_for_every_required_projection_and_partial_retry_is_safe() {
    let operations = Arc::new(Mutex::new(Vec::new()));
    let view_store = FailFirstAuditWriteViewStore::new(Arc::clone(&operations));
    let checkpoints = RecordingCheckpointStore::new(false, Arc::clone(&operations));
    let dispatcher = ProjectionDispatcher::new(ProjectionRuntime::new(view_store))
        .with_checkpoint(CHECKPOINT_ID, checkpoints.clone())
        .with_handler(TypedProjectionHandler::new(OfferApplicationCountProjection))
        .with_handler(TypedProjectionHandler::new(OfferAuditProjection));
    let source = offer_created_recorded_event("offer-1", "Initial offer");
    let cursor = ProjectionCursor::new(b"delivery-41".to_vec());

    let first_error = dispatcher
        .dispatch_with_cursor(&source, cursor.clone())
        .await
        .expect_err("one required View write should fail the Event application");
    match first_error {
        ProjectionDispatchError::HandlerFailures { applied, failures } => {
            assert_eq!(applied, 1);
            assert_eq!(failures.len(), 1);
            assert_eq!(failures[0].projection_type, "offer_audit");
        }
        other => panic!("expected handler failure, got {other:?}"),
    }
    assert!(checkpoints.checkpoint(CHECKPOINT_ID).is_none());
    assert_eq!(checkpoints.save_attempts(), 0);
    assert_eq!(
        take_operations(&operations),
        vec![
            "view:offer_application_count/offer-1",
            "view:offer_audit/offer-1",
        ]
    );

    dispatcher
        .dispatch_with_cursor(&source, cursor.clone())
        .await
        .expect("failed handler should complete on retry");

    let count_view = dispatcher
        .view_store()
        .load(&ViewKey::new("offer_application_count", "offer-1"))
        .await
        .expect("load application count")
        .expect("application count should exist");
    let audit_view = dispatcher
        .view_store()
        .load(&ViewKey::new("offer_audit", "offer-1"))
        .await
        .expect("load audit View")
        .expect("audit View should exist");
    assert_eq!(count_view.payload["applications"], 1);
    assert_eq!(audit_view.payload["title"], "Initial offer");
    assert_eq!(
        take_operations(&operations),
        vec!["view:offer_audit/offer-1", "checkpoint:offer-read-model"]
    );
    assert_checkpoint_context(
        checkpoints
            .checkpoint(CHECKPOINT_ID)
            .expect("checkpoint should advance after every View write succeeds"),
        &source,
        &cursor,
    );
}

#[tokio::test]
async fn checkpoint_save_failure_retries_without_regressing_the_written_view() {
    let operations = Arc::new(Mutex::new(Vec::new()));
    let view_store = RecordingViewStore::new(Arc::clone(&operations));
    let checkpoints = RecordingCheckpointStore::new(true, Arc::clone(&operations));
    let dispatcher = ProjectionDispatcher::new(ProjectionRuntime::new(view_store))
        .with_checkpoint(CHECKPOINT_ID, checkpoints.clone())
        .with_handler(TypedProjectionHandler::new(OfferApplicationCountProjection));
    let source = offer_created_recorded_event("offer-1", "Initial offer");
    let cursor = ProjectionCursor::new(b"delivery-42".to_vec());

    let error = dispatcher
        .dispatch_with_cursor(&source, cursor.clone())
        .await
        .expect_err("injected checkpoint save should fail after the View write");
    match error {
        ProjectionDispatchError::Checkpoint(ProjectionCheckpointError::Save {
            checkpoint_id,
            reason,
        }) => {
            assert_eq!(checkpoint_id, CHECKPOINT_ID);
            assert_eq!(reason, "injected checkpoint save failure");
        }
        other => panic!("expected named checkpoint save failure, got {other:?}"),
    }
    assert!(checkpoints.checkpoint(CHECKPOINT_ID).is_none());
    assert_eq!(
        take_operations(&operations),
        vec![
            "view:offer_application_count/offer-1",
            "checkpoint:offer-read-model",
        ]
    );

    dispatcher
        .dispatch_with_cursor(&source, cursor.clone())
        .await
        .expect("same delivery should safely complete its checkpoint on retry");

    let view = dispatcher
        .view_store()
        .load(&ViewKey::new("offer_application_count", "offer-1"))
        .await
        .expect("load application count")
        .expect("application count should exist");
    assert_eq!(view.payload["applications"], 1);
    assert_eq!(checkpoints.save_attempts(), 2);
    assert_eq!(
        take_operations(&operations),
        vec!["checkpoint:offer-read-model"]
    );
    assert_checkpoint_context(
        checkpoints
            .checkpoint(CHECKPOINT_ID)
            .expect("retry should persist the Event checkpoint"),
        &source,
        &cursor,
    );
}

#[tokio::test]
async fn rebuild_pauses_delivery_and_idempotently_resets_views_metadata_and_checkpoint() {
    let checkpoints = RecordingCheckpointStore::new(false, Arc::default());
    let dispatcher = ProjectionDispatcher::new(ProjectionRuntime::new(InMemoryViewStore::new()))
        .with_checkpoint(CHECKPOINT_ID, checkpoints.clone())
        .with_handler(TypedProjectionHandler::new(OfferApplicationCountProjection));
    let source = offer_created_recorded_event("offer-1", "Initial offer");
    let cursor = ProjectionCursor::new(b"delivery-43".to_vec());
    let selection = ProjectionRebuildSelection::new(vec![(
        "offer_application_count".to_string(),
        ViewKey::new("offer_application_count", "offer-1"),
    )]);

    dispatcher
        .dispatch_with_cursor(&source, cursor.clone())
        .await
        .expect("initial projection should complete");

    let first_reset = dispatcher
        .begin_rebuild(selection.clone())
        .await
        .expect("first reset should begin a paused rebuild");
    assert!(dispatcher
        .view_store()
        .load(&ViewKey::new("offer_application_count", "offer-1"))
        .await
        .expect("load reset View")
        .is_none());
    assert!(checkpoints.checkpoint(CHECKPOINT_ID).is_none());
    let mut next_source = offer_created_recorded_event("offer-1", "Updated offer");
    next_source.sequence = 2;
    next_source.metadata.message_id = "offer-created-event-2".to_string();
    match dispatcher
        .dispatch_with_cursor(&next_source, ProjectionCursor::new(b"delivery-44".to_vec()))
        .await
        .expect_err("normal delivery must remain paused during rebuild")
    {
        ProjectionDispatchError::RebuildInProgress { checkpoint_id } => {
            assert_eq!(checkpoint_id, CHECKPOINT_ID);
        }
        other => panic!("expected rebuild-in-progress error, got {other:?}"),
    }
    first_reset
        .finish()
        .await
        .expect("empty first rebuild should finish");

    let second_reset = dispatcher
        .begin_rebuild(selection)
        .await
        .expect("resetting already-empty state should be idempotent");
    second_reset
        .replay(&source, cursor.clone())
        .await
        .expect("reset application metadata should allow replay of the same Event");
    second_reset
        .finish()
        .await
        .expect("rebuild should resume normal delivery");

    dispatcher
        .dispatch_with_cursor(&source, cursor.clone())
        .await
        .expect("post-rebuild duplicate delivery should remain a no-op");
    let rebuilt_view = dispatcher
        .view_store()
        .load(&ViewKey::new("offer_application_count", "offer-1"))
        .await
        .expect("load rebuilt View")
        .expect("replayed Event should rebuild selected View");
    assert_eq!(rebuilt_view.payload["applications"], 1);
    assert_eq!(checkpoints.reset_attempts(), 2);
    assert_checkpoint_context(
        checkpoints
            .checkpoint(CHECKPOINT_ID)
            .expect("replay should restore the checkpoint"),
        &source,
        &cursor,
    );
}

#[derive(Debug, Default, Clone)]
struct Offer;

impl Resource for Offer {
    type Id = String;

    const RESOURCE_TYPE: &'static str = "offer";

    fn apply_recorded(&mut self, _event: &RecordedEvent) -> Result<(), ResourceError> {
        Ok(())
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

struct ContextCapturingProjection {
    seen_context: Arc<Mutex<Option<ProjectionContext>>>,
}

#[async_trait]
impl Projection for ContextCapturingProjection {
    type Source = OfferCreatedV1;

    const PROJECTION_TYPE: &'static str = "context_capture";

    async fn project<V>(&self, _event: Self::Source, _view_store: &V) -> Result<(), ViewStoreError>
    where
        V: ViewStore,
    {
        panic!("ProjectionRuntime must call project_with_context")
    }

    async fn project_with_context<V>(
        &self,
        event: Self::Source,
        context: &ProjectionContext,
        view_store: &V,
    ) -> Result<(), ViewStoreError>
    where
        V: ViewStore,
    {
        *self.seen_context.lock().expect("context capture lock") = Some(context.clone());
        view_store
            .put(ViewDocument::new(
                "context_capture",
                event.offer_id,
                json!({ "seen": true }),
            ))
            .await
    }
}

struct OfferApplicationCountProjection;

#[async_trait]
impl Projection for OfferApplicationCountProjection {
    type Source = OfferCreatedV1;

    const PROJECTION_TYPE: &'static str = "offer_application_count";

    async fn project<V>(&self, event: Self::Source, view_store: &V) -> Result<(), ViewStoreError>
    where
        V: ViewStore,
    {
        let key = ViewKey::new("offer_application_count", event.offer_id.clone());
        let applications = view_store
            .load(&key)
            .await?
            .and_then(|document| document.payload["applications"].as_u64())
            .unwrap_or_default()
            + 1;

        view_store
            .put(ViewDocument::new(
                key.view_type,
                key.view_id,
                json!({
                    "offer_id": event.offer_id,
                    "applications": applications,
                }),
            ))
            .await
    }
}

struct OfferAuditProjection;

#[async_trait]
impl Projection for OfferAuditProjection {
    type Source = OfferCreatedV1;

    const PROJECTION_TYPE: &'static str = "offer_audit";

    async fn project<V>(&self, event: Self::Source, view_store: &V) -> Result<(), ViewStoreError>
    where
        V: ViewStore,
    {
        view_store
            .put(ViewDocument::new(
                "offer_audit",
                event.offer_id.clone(),
                json!({
                    "offer_id": event.offer_id,
                    "title": event.title,
                }),
            ))
            .await
    }
}

#[derive(Clone)]
struct RecordingViewStore {
    inner: InMemoryViewStore,
    operations: Arc<Mutex<Vec<String>>>,
}

impl RecordingViewStore {
    fn new(operations: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            inner: InMemoryViewStore::new(),
            operations,
        }
    }
}

#[async_trait]
impl ViewStore for RecordingViewStore {
    async fn put(&self, document: ViewDocument) -> Result<(), ViewStoreError> {
        self.operations
            .lock()
            .expect("operation log lock")
            .push(format!(
                "view:{}/{}",
                document.key.view_type, document.key.view_id
            ));
        self.inner.put(document).await
    }

    async fn load(&self, key: &ViewKey) -> Result<Option<ViewDocument>, ViewStoreError> {
        self.inner.load(key).await
    }

    async fn list_by_index_prefix(
        &self,
        view_type: &str,
        index_name: &str,
        prefix: &str,
    ) -> Result<Vec<ViewDocument>, ViewStoreError> {
        self.inner
            .list_by_index_prefix(view_type, index_name, prefix)
            .await
    }
}

#[derive(Clone)]
struct FailFirstAuditWriteViewStore {
    recording: RecordingViewStore,
    fail_first_audit_write: Arc<AtomicBool>,
}

impl FailFirstAuditWriteViewStore {
    fn new(operations: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            recording: RecordingViewStore::new(operations),
            fail_first_audit_write: Arc::new(AtomicBool::new(true)),
        }
    }
}

#[async_trait]
impl ViewStore for FailFirstAuditWriteViewStore {
    async fn put(&self, document: ViewDocument) -> Result<(), ViewStoreError> {
        if document.key.view_type == "offer_audit"
            && self.fail_first_audit_write.swap(false, Ordering::SeqCst)
        {
            self.recording
                .operations
                .lock()
                .expect("operation log lock")
                .push(format!(
                    "view:{}/{}",
                    document.key.view_type, document.key.view_id
                ));
            return Err(ViewStoreError::NatsPut {
                view_type: document.key.view_type,
                view_id: document.key.view_id,
                reason: "injected first audit write failure".to_string(),
            });
        }

        self.recording.put(document).await
    }

    async fn load(&self, key: &ViewKey) -> Result<Option<ViewDocument>, ViewStoreError> {
        self.recording.load(key).await
    }

    async fn list_by_index_prefix(
        &self,
        view_type: &str,
        index_name: &str,
        prefix: &str,
    ) -> Result<Vec<ViewDocument>, ViewStoreError> {
        self.recording
            .list_by_index_prefix(view_type, index_name, prefix)
            .await
    }
}

#[derive(Clone)]
struct RecordingCheckpointStore {
    checkpoint: Arc<Mutex<Option<(String, ProjectionCheckpoint)>>>,
    fail_next_save: Arc<AtomicBool>,
    save_attempts: Arc<AtomicUsize>,
    reset_attempts: Arc<AtomicUsize>,
    operations: Arc<Mutex<Vec<String>>>,
}

impl RecordingCheckpointStore {
    fn new(fail_next_save: bool, operations: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            checkpoint: Arc::new(Mutex::new(None)),
            fail_next_save: Arc::new(AtomicBool::new(fail_next_save)),
            save_attempts: Arc::new(AtomicUsize::new(0)),
            reset_attempts: Arc::new(AtomicUsize::new(0)),
            operations,
        }
    }

    fn checkpoint(&self, checkpoint_id: &str) -> Option<ProjectionCheckpoint> {
        self.checkpoint
            .lock()
            .expect("checkpoint lock")
            .as_ref()
            .filter(|(stored_id, _)| stored_id == checkpoint_id)
            .map(|(_, checkpoint)| checkpoint.clone())
    }

    fn save_attempts(&self) -> usize {
        self.save_attempts.load(Ordering::SeqCst)
    }

    fn reset_attempts(&self) -> usize {
        self.reset_attempts.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ProjectionCheckpointStore for RecordingCheckpointStore {
    async fn load(
        &self,
        checkpoint_id: &str,
    ) -> Result<Option<ProjectionCheckpoint>, ProjectionCheckpointError> {
        Ok(self.checkpoint(checkpoint_id))
    }

    async fn save(
        &self,
        checkpoint_id: &str,
        checkpoint: ProjectionCheckpoint,
    ) -> Result<(), ProjectionCheckpointError> {
        self.save_attempts.fetch_add(1, Ordering::SeqCst);
        self.operations
            .lock()
            .expect("operation log lock")
            .push(format!("checkpoint:{checkpoint_id}"));

        if self.fail_next_save.swap(false, Ordering::SeqCst) {
            return Err(ProjectionCheckpointError::Save {
                checkpoint_id: checkpoint_id.to_string(),
                reason: "injected checkpoint save failure".to_string(),
            });
        }

        *self.checkpoint.lock().expect("checkpoint lock") =
            Some((checkpoint_id.to_string(), checkpoint));
        Ok(())
    }

    async fn reset(&self, checkpoint_id: &str) -> Result<(), ProjectionCheckpointError> {
        self.reset_attempts.fetch_add(1, Ordering::SeqCst);
        let mut checkpoint = self.checkpoint.lock().expect("checkpoint lock");
        if checkpoint
            .as_ref()
            .is_some_and(|(stored_id, _)| stored_id == checkpoint_id)
        {
            *checkpoint = None;
        }
        Ok(())
    }
}

fn assert_checkpoint_context(
    checkpoint: ProjectionCheckpoint,
    source: &RecordedEvent,
    cursor: &ProjectionCursor,
) {
    assert_eq!(checkpoint.source_message_id(), source.metadata.message_id);
    assert_eq!(checkpoint.source_stream(), &source.stream);
    assert_eq!(checkpoint.aggregate_sequence(), source.sequence);
    assert_eq!(checkpoint.transport_cursor(), cursor);
}

fn take_operations(operations: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
    std::mem::take(&mut *operations.lock().expect("operation log lock"))
}

fn offer_created_recorded_event(offer_id: &str, title: &str) -> RecordedEvent {
    RecordedEvent {
        stream: ResourceStream::new("offer", offer_id),
        sequence: 1,
        metadata: elbmesh_core::MessageMetadata {
            message_id: "offer-created-event-1".to_string(),
            message_type: "offer_created".to_string(),
            message_version: 1,
            resource_type: "offer".to_string(),
            resource_id: offer_id.to_string(),
            stream_type: StreamType::Resource,
            correlation_id: "correlation-projection".to_string(),
            causation_id: "create-offer-action-1".to_string(),
            action_id: "create-offer-action-1".to_string(),
            actor_id: "actor-123".to_string(),
            occurred_at: "2026-07-17T00:00:00Z".to_string(),
            schema_id: "event.offer_created.v1".to_string(),
            schema_version: 1,
        },
        payload: json!({ "offer_id": offer_id, "title": title }),
    }
}
