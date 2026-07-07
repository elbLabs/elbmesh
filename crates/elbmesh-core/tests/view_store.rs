use elbmesh_core::{
    EventStore, InMemoryEventStore, InMemoryViewStore, ResourceStream, ViewDocument, ViewKey,
    ViewStore,
};

use serde_json::json;

#[test]
fn in_memory_view_store_implements_view_store_trait() {
    fn assert_view_store<S: ViewStore>() {}

    assert_view_store::<InMemoryViewStore>();
}

#[tokio::test]
async fn in_memory_view_store_stores_and_loads_view_document_by_identity() {
    let store = InMemoryViewStore::new();
    let document = offer_summary_view("offer-1", "Initial offer");

    store.put(document.clone()).await.expect("put view");

    let loaded = store
        .load(&ViewKey::new("offer_summary", "offer-1"))
        .await
        .expect("load view");

    assert_eq!(loaded, Some(document));
}

#[tokio::test]
async fn in_memory_view_store_returns_none_for_missing_view() {
    let store = InMemoryViewStore::new();

    let loaded = store
        .load(&ViewKey::new("offer_summary", "missing-offer"))
        .await
        .expect("load missing view");

    assert!(loaded.is_none());
}

#[tokio::test]
async fn in_memory_view_store_overwrites_existing_view_document() {
    let store = InMemoryViewStore::new();

    store
        .put(offer_summary_view("offer-1", "Initial offer"))
        .await
        .expect("put initial view");
    store
        .put(offer_summary_view("offer-1", "Accepted offer"))
        .await
        .expect("put updated view");

    let loaded = store
        .load(&ViewKey::new("offer_summary", "offer-1"))
        .await
        .expect("load updated view")
        .expect("updated view should exist");

    assert_eq!(loaded.payload["title"], "Accepted offer");
}

#[tokio::test]
async fn in_memory_view_store_keeps_view_types_and_ids_isolated() {
    let store = InMemoryViewStore::new();
    let offer_summary = offer_summary_view("shared-id", "Initial offer");
    let flow_status = ViewDocument::new(
        "flow_status",
        "shared-id",
        json!({
            "status": "offer_accepted",
        }),
    );
    let other_offer = offer_summary_view("offer-2", "Second offer");

    store
        .put(offer_summary.clone())
        .await
        .expect("put offer summary");
    store
        .put(flow_status.clone())
        .await
        .expect("put flow status");
    store
        .put(other_offer.clone())
        .await
        .expect("put other offer");

    assert_eq!(
        store
            .load(&ViewKey::new("offer_summary", "shared-id"))
            .await
            .expect("load offer summary"),
        Some(offer_summary)
    );
    assert_eq!(
        store
            .load(&ViewKey::new("flow_status", "shared-id"))
            .await
            .expect("load flow status"),
        Some(flow_status)
    );
    assert_eq!(
        store
            .load(&ViewKey::new("offer_summary", "offer-2"))
            .await
            .expect("load other offer"),
        Some(other_offer)
    );
}

#[tokio::test]
async fn in_memory_view_store_writes_do_not_create_resource_events() {
    let event_store = InMemoryEventStore::new();
    let view_store = InMemoryViewStore::new();

    view_store
        .put(offer_summary_view("offer-1", "Initial offer"))
        .await
        .expect("put view");

    let events = event_store
        .load(&ResourceStream::new("offer", "offer-1"))
        .await
        .expect("load resource stream");

    assert!(events.is_empty());
    assert!(event_store.all_events().is_empty());
}

fn offer_summary_view(offer_id: &str, title: &str) -> ViewDocument {
    ViewDocument::new(
        "offer_summary",
        offer_id,
        json!({
            "offer_id": offer_id,
            "title": title,
        }),
    )
}
