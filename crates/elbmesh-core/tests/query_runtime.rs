use elbmesh_core::{
    ArchitectureManifest, EventStore, InMemoryEventStore, InMemoryViewStore, QueryDefinition,
    QueryError, QueryExecutor, ResourceStream, ViewDefinition, ViewDocument, ViewIndexEntry,
    ViewStore,
};

use serde_json::json;

#[tokio::test]
async fn query_runtime_declared_query_reads_flow_status_document_by_offer_id_from_view_store() {
    let view_store = InMemoryViewStore::new();
    let event_store = InMemoryEventStore::new();
    let query = QueryExecutor::new(flow_status_manifest(), view_store.clone());

    view_store
        .put(flow_status_document("offer-1", "invoice_created"))
        .await
        .expect("put flow status view");

    let document = query
        .get_by_id("get_flow_status", "offer-1")
        .await
        .expect("query flow status by offer id");

    assert_eq!(document.key.view_type, "flow_status");
    assert_eq!(document.key.view_id, "offer-1");
    assert_eq!(document.payload["status"], "invoice_created");
    assert!(event_store
        .load(&ResourceStream::new("offer", "offer-1"))
        .await
        .expect("query should not need resource events")
        .is_empty());
}

#[tokio::test]
async fn query_runtime_declared_query_lists_flow_status_documents_through_declared_all_index() {
    let view_store = InMemoryViewStore::new();
    let query = QueryExecutor::new(flow_status_manifest(), view_store.clone());

    view_store
        .put(flow_status_document("offer-2", "offer_accepted"))
        .await
        .expect("put second flow status");
    view_store
        .put(flow_status_document("offer-1", "invoice_created"))
        .await
        .expect("put first flow status");

    let documents = query
        .list_by_index_prefix("get_flow_status", "all", "")
        .await
        .expect("list flow status by declared all index");

    assert_eq!(
        documents
            .iter()
            .map(|document| document.key.view_id.as_str())
            .collect::<Vec<_>>(),
        vec!["offer-1", "offer-2"]
    );
}

#[tokio::test]
async fn query_runtime_declared_query_rejects_undeclared_index_access() {
    let view_store = InMemoryViewStore::new();
    let query = QueryExecutor::new(flow_status_manifest(), view_store);

    let err = query
        .list_by_index_prefix("get_flow_status", "by_status", "invoice_created/")
        .await
        .expect_err("undeclared query index should fail");

    assert_eq!(
        err,
        QueryError::UndeclaredIndex {
            query_type: "get_flow_status".to_string(),
            index_name: "by_status".to_string(),
        }
    );
    assert_eq!(err.code(), "query.undeclared_index");
}

#[tokio::test]
async fn query_runtime_declared_query_returns_typed_not_found_for_missing_view_document() {
    let view_store = InMemoryViewStore::new();
    let query = QueryExecutor::new(flow_status_manifest(), view_store);

    let err = query
        .get_by_id("get_flow_status", "missing-offer")
        .await
        .expect_err("missing view document should be typed not found");

    assert_eq!(
        err,
        QueryError::ViewDocumentNotFound {
            query_type: "get_flow_status".to_string(),
            view_type: "flow_status".to_string(),
            view_id: "missing-offer".to_string(),
        }
    );
    assert_eq!(err.code(), "query.view_document_not_found");
}

fn flow_status_manifest() -> ArchitectureManifest {
    ArchitectureManifest {
        manifest_schema_id: "manifest.elbmesh.v1".to_string(),
        manifest_schema_version: 1,
        resources: Vec::new(),
        actions: Vec::new(),
        events: Vec::new(),
        reactions: Vec::new(),
        views: vec![ViewDefinition {
            view_type: "flow_status".to_string(),
            schema_id: "view.flow_status.v1".to_string(),
            schema_version: 1,
        }],
        queries: vec![QueryDefinition {
            query_type: "get_flow_status".to_string(),
            view_type: "flow_status".to_string(),
            index_names: vec!["all".to_string()],
            schema_id: "query.get_flow_status.v1".to_string(),
            schema_version: 1,
        }],
        external_operations: Vec::new(),
    }
}

fn flow_status_document(offer_id: &str, status: &str) -> ViewDocument {
    ViewDocument::new(
        "flow_status",
        offer_id,
        json!({
            "offer_id": offer_id,
            "status": status,
        }),
    )
    .with_indexes(vec![
        ViewIndexEntry::new("all", offer_id),
        ViewIndexEntry::new("by_status", format!("{status}/{offer_id}")),
    ])
}
