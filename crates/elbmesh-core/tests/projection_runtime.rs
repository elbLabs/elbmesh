use async_trait::async_trait;

use elbmesh_core::{
    Event, InMemoryViewStore, Projection, ProjectionExecutionError, ProjectionRuntime,
    RecordedEvent, Resource, ResourceError, ResourceStream, StreamType, ViewDocument, ViewKey,
    ViewStore, ViewStoreError,
};

use serde::{Deserialize, Serialize};
use serde_json::json;

#[tokio::test]
async fn matching_resource_event_projects_view_document() {
    let runtime = ProjectionRuntime::new(InMemoryViewStore::new());
    let trigger = offer_created_recorded_event("offer-1", "Initial offer");

    let applied = runtime
        .apply(&trigger, &OfferSummaryProjection)
        .await
        .expect("projection should apply");

    assert!(applied);
    let view = runtime
        .view_store()
        .load(&ViewKey::new("offer_summary", "offer-1"))
        .await
        .expect("load offer summary")
        .expect("offer summary should exist");
    assert_eq!(view.payload["offer_id"], "offer-1");
    assert_eq!(view.payload["title"], "Initial offer");
}

#[tokio::test]
async fn non_matching_event_is_ignored_without_view_writes() {
    let runtime = ProjectionRuntime::new(InMemoryViewStore::new());
    let mut trigger = offer_created_recorded_event("offer-1", "Initial offer");
    trigger.metadata.message_type = "offer_renamed".to_string();

    let applied = runtime
        .apply(&trigger, &OfferSummaryProjection)
        .await
        .expect("non-matching event should not fail");

    assert!(!applied);
    let view = runtime
        .view_store()
        .load(&ViewKey::new("offer_summary", "offer-1"))
        .await
        .expect("load ignored offer summary");
    assert!(view.is_none());
}

#[tokio::test]
async fn matching_event_type_from_non_resource_stream_is_ignored_without_view_writes() {
    let runtime = ProjectionRuntime::new(InMemoryViewStore::new());
    let mut trigger = offer_created_recorded_event("offer-1", "Initial offer");
    trigger.metadata.stream_type = StreamType::Reaction;

    let applied = runtime
        .apply(&trigger, &OfferSummaryProjection)
        .await
        .expect("non-resource event should not fail");

    assert!(!applied);
    let view = runtime
        .view_store()
        .load(&ViewKey::new("offer_summary", "offer-1"))
        .await
        .expect("load ignored offer summary");
    assert!(view.is_none());
}

#[tokio::test]
async fn schema_or_resource_mismatch_is_ignored_without_view_writes() {
    let runtime = ProjectionRuntime::new(InMemoryViewStore::new());
    let mut wrong_schema = offer_created_recorded_event("offer-1", "Initial offer");
    wrong_schema.metadata.schema_id = "event.offer_created.v2".to_string();
    let mut wrong_schema_version = offer_created_recorded_event("offer-2", "Second offer");
    wrong_schema_version.metadata.schema_version = 2;
    let mut wrong_resource = offer_created_recorded_event("offer-3", "Third offer");
    wrong_resource.metadata.resource_type = "invoice".to_string();

    let schema_applied = runtime
        .apply(&wrong_schema, &OfferSummaryProjection)
        .await
        .expect("schema mismatch should not fail");
    let schema_version_applied = runtime
        .apply(&wrong_schema_version, &OfferSummaryProjection)
        .await
        .expect("schema version mismatch should not fail");
    let resource_applied = runtime
        .apply(&wrong_resource, &OfferSummaryProjection)
        .await
        .expect("resource mismatch should not fail");

    assert!(!schema_applied);
    assert!(!schema_version_applied);
    assert!(!resource_applied);
    let offer_1 = runtime
        .view_store()
        .load(&ViewKey::new("offer_summary", "offer-1"))
        .await
        .expect("load schema-mismatched offer summary");
    let offer_2 = runtime
        .view_store()
        .load(&ViewKey::new("offer_summary", "offer-2"))
        .await
        .expect("load schema-version-mismatched offer summary");
    let offer_3 = runtime
        .view_store()
        .load(&ViewKey::new("offer_summary", "offer-3"))
        .await
        .expect("load resource-mismatched offer summary");
    assert!(offer_1.is_none());
    assert!(offer_2.is_none());
    assert!(offer_3.is_none());
}

#[tokio::test]
async fn matching_event_with_invalid_payload_returns_named_deserialization_error() {
    let runtime = ProjectionRuntime::new(InMemoryViewStore::new());
    let mut trigger = offer_created_recorded_event("offer-1", "Initial offer");
    trigger.payload = json!({ "offer_id": 123 });

    let error = runtime
        .apply(&trigger, &OfferSummaryProjection)
        .await
        .expect_err("invalid matching event should fail");

    match error {
        ProjectionExecutionError::SourceEventDeserialization {
            message_type,
            schema_version,
            ..
        } => {
            assert_eq!(message_type, "offer_created");
            assert_eq!(schema_version, 1);
        }
        other => panic!("expected SourceEventDeserialization error, got {other:?}"),
    }
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

struct OfferSummaryProjection;

#[async_trait]
impl Projection for OfferSummaryProjection {
    type Source = OfferCreatedV1;

    async fn project<V>(&self, event: Self::Source, view_store: &V) -> Result<(), ViewStoreError>
    where
        V: ViewStore,
    {
        view_store
            .put(ViewDocument::new(
                "offer_summary",
                event.offer_id.clone(),
                json!({
                    "offer_id": event.offer_id,
                    "title": event.title,
                }),
            ))
            .await
    }
}

fn offer_created_recorded_event(offer_id: &str, title: &str) -> RecordedEvent {
    RecordedEvent {
        stream: ResourceStream::new("offer", offer_id),
        sequence: 1,
        metadata: resource_event_metadata(
            "offer-created-event-1",
            "offer_created",
            "event.offer_created.v1",
            1,
            offer_id,
        ),
        payload: json!({ "offer_id": offer_id, "title": title }),
    }
}

fn resource_event_metadata(
    message_id: &str,
    message_type: &str,
    schema_id: &str,
    schema_version: u32,
    offer_id: &str,
) -> elbmesh_core::MessageMetadata {
    elbmesh_core::MessageMetadata {
        message_id: message_id.to_string(),
        message_type: message_type.to_string(),
        message_version: 1,
        resource_type: "offer".to_string(),
        resource_id: offer_id.to_string(),
        stream_type: StreamType::Resource,
        correlation_id: "correlation-projection".to_string(),
        causation_id: "create-offer-action-1".to_string(),
        action_id: "create-offer-action-1".to_string(),
        actor_id: "actor-123".to_string(),
        occurred_at: "2026-07-07T00:00:00Z".to_string(),
        schema_id: schema_id.to_string(),
        schema_version,
    }
}
