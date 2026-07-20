use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;

use elbmesh_core::{
    Event, InMemoryViewStore, Projection, ProjectionDispatchError, ProjectionDispatcher,
    ProjectionExecutionError, ProjectionRuntime, RecordedEvent, Resource, ResourceError,
    ResourceStream, StreamType, TypedProjectionHandler, ViewDocument, ViewKey, ViewStore,
    ViewStoreError,
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
async fn source_aware_out_of_order_delivery_cannot_overwrite_newer_view_state() {
    let dispatcher = ProjectionDispatcher::new(ProjectionRuntime::new(InMemoryViewStore::new()))
        .with_handler(TypedProjectionHandler::new(OfferSummaryProjection));
    let mut newer = offer_created_recorded_event("offer-1", "Newest offer");
    newer.sequence = 2;
    newer.metadata.message_id = "offer-created-event-2".to_string();
    let older = offer_created_recorded_event("offer-1", "Stale offer");

    dispatcher
        .dispatch(&newer)
        .await
        .expect("newer source should project");
    dispatcher
        .dispatch(&older)
        .await
        .expect("older source should be safely ignored");

    let view = dispatcher
        .view_store()
        .load(&ViewKey::new("offer_summary", "offer-1"))
        .await
        .expect("load offer summary")
        .expect("newer offer summary should remain");
    assert_eq!(view.payload["title"], "Newest offer");
}

#[tokio::test]
async fn source_aware_duplicate_delivery_does_not_reapply_projection_after_view_write() {
    let dispatcher = ProjectionDispatcher::new(ProjectionRuntime::new(InMemoryViewStore::new()))
        .with_handler(TypedProjectionHandler::new(OfferApplicationCountProjection));
    let source = offer_created_recorded_event("offer-1", "Initial offer");

    dispatcher
        .dispatch(&source)
        .await
        .expect("first delivery should project");
    dispatcher
        .dispatch(&source)
        .await
        .expect("duplicate delivery should be safely ignored");

    let view = dispatcher
        .view_store()
        .load(&ViewKey::new("offer_application_count", "offer-1"))
        .await
        .expect("load application count")
        .expect("application count should exist");
    assert_eq!(view.payload["applications"], 1);
}

#[tokio::test]
async fn source_aware_partial_multi_handler_failure_retries_without_reapplying_successful_view() {
    let dispatcher =
        ProjectionDispatcher::new(ProjectionRuntime::new(FailFirstAuditWriteViewStore::new()))
            .with_handler(TypedProjectionHandler::new(OfferApplicationCountProjection))
            .with_handler(TypedProjectionHandler::new(OfferAuditProjection));
    let source = offer_created_recorded_event("offer-1", "Initial offer");

    let first_error = dispatcher
        .dispatch(&source)
        .await
        .expect_err("first audit View write should fail dispatch");
    match first_error {
        ProjectionDispatchError::HandlerFailures { applied, failures } => {
            assert_eq!(applied, 1);
            assert_eq!(failures.len(), 1);
            assert_eq!(failures[0].projection_type, "offer_audit");
            assert_eq!(failures[0].failure_code, "projection.view_store.nats_put");
        }
        unexpected => panic!("unexpected projection dispatch error: {unexpected:?}"),
    }

    dispatcher
        .dispatch(&source)
        .await
        .expect("source should remain retryable after one required handler fails");

    let count_view = dispatcher
        .view_store()
        .load(&ViewKey::new("offer_application_count", "offer-1"))
        .await
        .expect("load application count")
        .expect("successful View should remain");
    let audit_view = dispatcher
        .view_store()
        .load(&ViewKey::new("offer_audit", "offer-1"))
        .await
        .expect("load audit View")
        .expect("failed handler should apply on retry");
    assert_eq!(count_view.payload["applications"], 1);
    assert_eq!(audit_view.payload["title"], "Initial offer");
}

#[tokio::test]
async fn source_aware_application_identity_includes_projection_and_target_view_key() {
    let dispatcher = ProjectionDispatcher::new(ProjectionRuntime::new(InMemoryViewStore::new()))
        .with_handler(TypedProjectionHandler::new(OfferDualTargetProjection))
        .with_handler(TypedProjectionHandler::new(
            OfferSharedTargetAuditProjection,
        ));
    let source = offer_created_recorded_event("offer-1", "Initial offer");

    dispatcher
        .dispatch(&source)
        .await
        .expect("both projections and both target View keys should apply");

    let shared_view = dispatcher
        .view_store()
        .load(&ViewKey::new("offer_shared", "offer-1"))
        .await
        .expect("load shared View")
        .expect("shared View should exist");
    let copy_view = dispatcher
        .view_store()
        .load(&ViewKey::new("offer_shared", "offer-1-copy"))
        .await
        .expect("load second target View")
        .expect("second target View should exist");

    assert_eq!(shared_view.payload["summary_applied"], true);
    assert_eq!(shared_view.payload["audit_applied"], true);
    assert_eq!(copy_view.payload["summary_applied"], true);
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
async fn projection_rejects_source_when_stream_identity_disagrees_with_metadata() {
    let runtime = ProjectionRuntime::new(InMemoryViewStore::new());
    let mut source = offer_created_recorded_event("offer-1", "Initial offer");
    source.stream = ResourceStream::new("offer", "offer-2");

    let result = runtime.apply(&source, &OfferSummaryProjection).await;

    assert!(
        result.is_err(),
        "source with inconsistent RecordedEvent stream identity should be rejected"
    );
    let metadata_view = runtime
        .view_store()
        .load(&ViewKey::new("offer_summary", "offer-1"))
        .await
        .expect("load metadata identity view");
    let stream_view = runtime
        .view_store()
        .load(&ViewKey::new("offer_summary", "offer-2"))
        .await
        .expect("load stream identity view");
    assert!(metadata_view.is_none());
    assert!(stream_view.is_none());
}

#[tokio::test]
async fn projection_rejects_source_when_payload_identity_disagrees_with_metadata() {
    let runtime = ProjectionRuntime::new(InMemoryViewStore::new());
    let mut source = offer_created_recorded_event("offer-1", "Initial offer");
    source.payload = json!({ "offer_id": "offer-2", "title": "Initial offer" });

    let result = runtime.apply(&source, &OfferSummaryProjection).await;

    assert!(
        result.is_err(),
        "source with inconsistent payload resource identity should be rejected"
    );
    let metadata_view = runtime
        .view_store()
        .load(&ViewKey::new("offer_summary", "offer-1"))
        .await
        .expect("load metadata identity view");
    let payload_view = runtime
        .view_store()
        .load(&ViewKey::new("offer_summary", "offer-2"))
        .await
        .expect("load payload identity view");
    assert!(metadata_view.is_none());
    assert!(payload_view.is_none());
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

#[tokio::test]
async fn dispatcher_applies_projections_from_multiple_resource_types_to_same_view() {
    let dispatcher = ProjectionDispatcher::new(ProjectionRuntime::new(InMemoryViewStore::new()))
        .with_handler(TypedProjectionHandler::new(OfferFlowStatusProjection))
        .with_handler(TypedProjectionHandler::new(SalesOrderFlowStatusProjection));
    let mut offer_created = offer_created_recorded_event("offer-1", "Initial offer");
    offer_created.sequence = 42;
    let sales_order_created = sales_order_created_recorded_event("sales-order-1", "offer-1");

    let offer_report = dispatcher
        .dispatch(&offer_created)
        .await
        .expect("offer projection dispatch should succeed");

    assert_eq!(offer_report.applied, 1);
    let initial_flow_status = dispatcher
        .view_store()
        .load(&ViewKey::new("flow_status", "offer-1"))
        .await
        .expect("load initial flow status")
        .expect("initial flow status should exist");
    assert_eq!(initial_flow_status.payload["offer_id"], "offer-1");
    assert_eq!(initial_flow_status.payload["status"], "offer_created");
    assert_eq!(initial_flow_status.payload["title"], "Initial offer");

    let sales_order_report = dispatcher
        .dispatch(&sales_order_created)
        .await
        .expect("sales order projection dispatch should succeed");

    assert_eq!(sales_order_report.applied, 1);
    let updated_flow_status = dispatcher
        .view_store()
        .load(&ViewKey::new("flow_status", "offer-1"))
        .await
        .expect("load updated flow status")
        .expect("updated flow status should exist");
    assert_eq!(updated_flow_status.payload["offer_id"], "offer-1");
    assert_eq!(updated_flow_status.payload["status"], "sales_order_created");
    assert_eq!(
        updated_flow_status.payload["sales_order_id"],
        "sales-order-1"
    );
}

#[tokio::test]
async fn dispatcher_ignores_non_matching_and_non_resource_events_without_view_writes() {
    let dispatcher = ProjectionDispatcher::new(ProjectionRuntime::new(InMemoryViewStore::new()))
        .with_handler(TypedProjectionHandler::new(OfferFlowStatusProjection))
        .with_handler(TypedProjectionHandler::new(SalesOrderFlowStatusProjection));
    let mut renamed_offer = offer_created_recorded_event("offer-1", "Initial offer");
    renamed_offer.metadata.message_type = "offer_renamed".to_string();
    let mut non_resource_sales_order =
        sales_order_created_recorded_event("sales-order-1", "offer-1");
    non_resource_sales_order.metadata.stream_type = StreamType::Reaction;

    let renamed_report = dispatcher
        .dispatch(&renamed_offer)
        .await
        .expect("renamed event dispatch should not fail");
    let non_resource_report = dispatcher
        .dispatch(&non_resource_sales_order)
        .await
        .expect("non-resource event dispatch should not fail");

    assert_eq!(renamed_report.applied, 0);
    assert_eq!(non_resource_report.applied, 0);
    let view = dispatcher
        .view_store()
        .load(&ViewKey::new("flow_status", "offer-1"))
        .await
        .expect("load ignored flow status");
    assert!(view.is_none());
}

#[tokio::test]
async fn dispatcher_returns_named_failures_with_details() {
    let dispatcher = ProjectionDispatcher::new(ProjectionRuntime::new(InMemoryViewStore::new()))
        .with_handler(TypedProjectionHandler::new(OfferFlowStatusProjection));
    let mut trigger = offer_created_recorded_event("offer-1", "Initial offer");
    trigger.payload = json!({ "offer_id": 123 });

    let error = dispatcher
        .dispatch(&trigger)
        .await
        .expect_err("invalid matching event should fail dispatch");

    match error {
        ProjectionDispatchError::HandlerFailures { applied, failures } => {
            assert_eq!(applied, 0);
            assert_eq!(failures.len(), 1);
            assert_eq!(failures[0].projection_type, "offer_flow_status");
            assert_eq!(
                failures[0].failure_code,
                "projection.source_event_deserialization"
            );
            assert_eq!(failures[0].failure_details["message_type"], "offer_created");
            assert_eq!(failures[0].failure_details["schema_version"], 1);
        }
        unexpected => panic!("unexpected projection dispatch error: {unexpected:?}"),
    }
}

#[tokio::test]
async fn dispatcher_reports_mixed_success_and_failure_and_keeps_successful_view() {
    let dispatcher = ProjectionDispatcher::new(ProjectionRuntime::new(InMemoryViewStore::new()))
        .with_handler(TypedProjectionHandler::new(OfferFlowStatusProjection))
        .with_handler(TypedProjectionHandler::new(
            OfferCreatedRequiresAuditProjection,
        ));
    let source = offer_created_recorded_event("offer-1", "Initial offer");

    let error = dispatcher
        .dispatch(&source)
        .await
        .expect_err("one matching projection should fail dispatch");

    match error {
        ProjectionDispatchError::HandlerFailures { applied, failures } => {
            assert_eq!(applied, 1);
            assert_eq!(failures.len(), 1);
            assert_eq!(failures[0].projection_type, "offer_created_requires_audit");
            assert_eq!(
                failures[0].failure_code,
                "projection.source_event_deserialization"
            );
        }
        unexpected => panic!("unexpected projection dispatch error: {unexpected:?}"),
    }

    let view = dispatcher
        .view_store()
        .load(&ViewKey::new("flow_status", "offer-1"))
        .await
        .expect("load successfully projected flow status")
        .expect("successful projection view should remain");
    assert_eq!(view.payload["offer_id"], "offer-1");
    assert_eq!(view.payload["status"], "offer_created");
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OfferCreatedRequiresAuditV1 {
    offer_id: String,
    title: String,
    required_audit_marker: String,
}

impl Event for OfferCreatedRequiresAuditV1 {
    type Resource = Offer;

    const EVENT_TYPE: &'static str = "offer_created";
    const SCHEMA_ID: &'static str = "event.offer_created.v1";
    const SCHEMA_VERSION: u32 = 1;

    fn resource_id(&self) -> <Self::Resource as Resource>::Id {
        self.offer_id.clone()
    }
}

#[derive(Debug, Default, Clone)]
struct SalesOrder;

impl Resource for SalesOrder {
    type Id = String;

    const RESOURCE_TYPE: &'static str = "sales_order";

    fn apply_recorded(&mut self, _event: &RecordedEvent) -> Result<(), ResourceError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SalesOrderCreatedV1 {
    sales_order_id: String,
    offer_id: String,
}

impl Event for SalesOrderCreatedV1 {
    type Resource = SalesOrder;

    const EVENT_TYPE: &'static str = "sales_order_created";
    const SCHEMA_ID: &'static str = "event.sales_order_created.v1";
    const SCHEMA_VERSION: u32 = 1;

    fn resource_id(&self) -> <Self::Resource as Resource>::Id {
        self.sales_order_id.clone()
    }
}

struct OfferSummaryProjection;

#[async_trait]
impl Projection for OfferSummaryProjection {
    type Source = OfferCreatedV1;

    const PROJECTION_TYPE: &'static str = "offer_summary";

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

struct OfferDualTargetProjection;

#[async_trait]
impl Projection for OfferDualTargetProjection {
    type Source = OfferCreatedV1;

    const PROJECTION_TYPE: &'static str = "offer_dual_target";

    async fn project<V>(&self, event: Self::Source, view_store: &V) -> Result<(), ViewStoreError>
    where
        V: ViewStore,
    {
        for view_id in [event.offer_id.clone(), format!("{}-copy", event.offer_id)] {
            view_store
                .put(ViewDocument::new(
                    "offer_shared",
                    view_id,
                    json!({
                        "offer_id": event.offer_id,
                        "summary_applied": true,
                        "audit_applied": false,
                    }),
                ))
                .await?;
        }

        Ok(())
    }
}

struct OfferSharedTargetAuditProjection;

#[async_trait]
impl Projection for OfferSharedTargetAuditProjection {
    type Source = OfferCreatedV1;

    const PROJECTION_TYPE: &'static str = "offer_shared_target_audit";

    async fn project<V>(&self, event: Self::Source, view_store: &V) -> Result<(), ViewStoreError>
    where
        V: ViewStore,
    {
        let key = ViewKey::new("offer_shared", event.offer_id.clone());
        let mut document = view_store.load(&key).await?.unwrap_or_else(|| {
            ViewDocument::new(
                key.view_type,
                key.view_id,
                json!({
                    "offer_id": event.offer_id,
                    "summary_applied": false,
                    "audit_applied": false,
                }),
            )
        });
        document.payload["audit_applied"] = json!(true);

        view_store.put(document).await
    }
}

#[derive(Clone)]
struct FailFirstAuditWriteViewStore {
    inner: InMemoryViewStore,
    fail_first_audit_write: Arc<AtomicBool>,
}

impl FailFirstAuditWriteViewStore {
    fn new() -> Self {
        Self {
            inner: InMemoryViewStore::new(),
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
            return Err(ViewStoreError::NatsPut {
                view_type: document.key.view_type,
                view_id: document.key.view_id,
                reason: "injected first audit write failure".to_string(),
            });
        }

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

struct OfferFlowStatusProjection;

#[async_trait]
impl Projection for OfferFlowStatusProjection {
    type Source = OfferCreatedV1;

    const PROJECTION_TYPE: &'static str = "offer_flow_status";

    async fn project<V>(&self, event: Self::Source, view_store: &V) -> Result<(), ViewStoreError>
    where
        V: ViewStore,
    {
        view_store
            .put(ViewDocument::new(
                "flow_status",
                event.offer_id.clone(),
                json!({
                    "offer_id": event.offer_id,
                    "status": "offer_created",
                    "title": event.title,
                }),
            ))
            .await
    }
}

struct OfferCreatedRequiresAuditProjection;

#[async_trait]
impl Projection for OfferCreatedRequiresAuditProjection {
    type Source = OfferCreatedRequiresAuditV1;

    const PROJECTION_TYPE: &'static str = "offer_created_requires_audit";

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
                    "required_audit_marker": event.required_audit_marker,
                }),
            ))
            .await
    }
}

struct SalesOrderFlowStatusProjection;

#[async_trait]
impl Projection for SalesOrderFlowStatusProjection {
    type Source = SalesOrderCreatedV1;

    const PROJECTION_TYPE: &'static str = "sales_order_flow_status";

    async fn project<V>(&self, event: Self::Source, view_store: &V) -> Result<(), ViewStoreError>
    where
        V: ViewStore,
    {
        view_store
            .put(ViewDocument::new(
                "flow_status",
                event.offer_id.clone(),
                json!({
                    "offer_id": event.offer_id,
                    "status": "sales_order_created",
                    "sales_order_id": event.sales_order_id,
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

fn sales_order_created_recorded_event(sales_order_id: &str, offer_id: &str) -> RecordedEvent {
    RecordedEvent {
        stream: ResourceStream::new("sales_order", sales_order_id),
        sequence: 1,
        metadata: event_metadata(
            "sales-order-created-event-1",
            "sales_order_created",
            "event.sales_order_created.v1",
            1,
            "sales_order",
            sales_order_id,
        ),
        payload: json!({
            "sales_order_id": sales_order_id,
            "offer_id": offer_id,
        }),
    }
}

fn resource_event_metadata(
    message_id: &str,
    message_type: &str,
    schema_id: &str,
    schema_version: u32,
    offer_id: &str,
) -> elbmesh_core::MessageMetadata {
    event_metadata(
        message_id,
        message_type,
        schema_id,
        schema_version,
        "offer",
        offer_id,
    )
}

fn event_metadata(
    message_id: &str,
    message_type: &str,
    schema_id: &str,
    schema_version: u32,
    resource_type: &str,
    resource_id: &str,
) -> elbmesh_core::MessageMetadata {
    elbmesh_core::MessageMetadata {
        message_id: message_id.to_string(),
        message_type: message_type.to_string(),
        message_version: 1,
        resource_type: resource_type.to_string(),
        resource_id: resource_id.to_string(),
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
