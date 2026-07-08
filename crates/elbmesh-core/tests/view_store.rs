use elbmesh_core::{
    EventStore, InMemoryEventStore, InMemoryViewStore, ResourceStream, ViewDocument,
    ViewIndexEntry, ViewKey, ViewStore, ViewStoreError,
};

#[cfg(feature = "nats-tests")]
use elbmesh_core::{NatsViewStore, NatsViewStoreConfig};

use serde_json::json;

#[cfg(feature = "nats-tests")]
mod support;

#[test]
fn in_memory_view_store_implements_view_store_trait() {
    fn assert_view_store<S: ViewStore>() {}

    assert_view_store::<InMemoryViewStore>();
}

#[tokio::test]
async fn in_memory_view_store_stores_and_loads_view_document_by_identity() {
    let store = InMemoryViewStore::new();

    assert_stores_and_loads_view_document_by_identity(&store).await;
}

#[tokio::test]
async fn in_memory_view_store_returns_none_for_missing_view() {
    let store = InMemoryViewStore::new();

    assert_returns_none_for_missing_view(&store).await;
}

#[tokio::test]
async fn in_memory_view_store_overwrites_existing_view_document() {
    let store = InMemoryViewStore::new();

    assert_overwrites_existing_view_document(&store).await;
}

#[tokio::test]
async fn in_memory_view_store_keeps_view_types_and_ids_isolated() {
    let store = InMemoryViewStore::new();

    assert_keeps_view_types_and_ids_isolated(&store).await;
}

#[tokio::test]
async fn in_memory_view_store_writes_do_not_create_resource_events() {
    let store = InMemoryViewStore::new();

    assert_writes_do_not_create_resource_events(&store).await;
}

#[tokio::test]
async fn in_memory_view_store_lists_all_index_with_empty_prefix() {
    let store = InMemoryViewStore::new();

    assert_lists_all_index_with_empty_prefix(&store).await;
}

#[tokio::test]
async fn in_memory_view_store_lists_matching_index_prefix_only() {
    let store = InMemoryViewStore::new();

    assert_lists_matching_index_prefix_only(&store).await;
}

#[tokio::test]
async fn in_memory_view_store_index_listing_isolates_view_types_and_index_names() {
    let store = InMemoryViewStore::new();

    assert_index_listing_isolates_view_types_and_index_names(&store).await;
}

#[tokio::test]
async fn in_memory_view_store_overwrite_replaces_index_membership() {
    let store = InMemoryViewStore::new();

    assert_overwrite_replaces_index_membership(&store).await;
}

#[tokio::test]
async fn in_memory_view_store_missing_index_returns_empty_list() {
    let store = InMemoryViewStore::new();

    assert_missing_index_returns_empty_list(&store).await;
}

#[tokio::test]
async fn in_memory_view_store_rejects_duplicate_index_names_in_one_document() {
    let store = InMemoryViewStore::new();

    assert_rejects_duplicate_index_names_in_one_document(&store).await;
}

#[cfg(feature = "nats-tests")]
#[test]
fn nats_view_store_implements_view_store_trait() {
    fn assert_view_store<S: ViewStore>() {}

    assert_view_store::<NatsViewStore>();
}

#[cfg(feature = "nats-tests")]
#[tokio::test]
async fn nats_view_store_stores_and_loads_view_document_by_identity() {
    let Some(store) = nats_view_store("stores_loads").await else {
        return;
    };

    assert_stores_and_loads_view_document_by_identity(&store).await;
}

#[cfg(feature = "nats-tests")]
#[tokio::test]
async fn nats_view_store_returns_none_for_missing_view() {
    let Some(store) = nats_view_store("missing_view").await else {
        return;
    };

    assert_returns_none_for_missing_view(&store).await;
}

#[cfg(feature = "nats-tests")]
#[tokio::test]
async fn nats_view_store_overwrites_existing_view_document() {
    let Some(store) = nats_view_store("overwrite_view").await else {
        return;
    };

    assert_overwrites_existing_view_document(&store).await;
}

#[cfg(feature = "nats-tests")]
#[tokio::test]
async fn nats_view_store_keeps_view_types_and_ids_isolated() {
    let Some(store) = nats_view_store("isolated_keys").await else {
        return;
    };

    assert_keeps_view_types_and_ids_isolated(&store).await;
}

#[cfg(feature = "nats-tests")]
#[tokio::test]
async fn nats_view_store_writes_do_not_create_resource_events() {
    let Some(store) = nats_view_store("separate_from_events").await else {
        return;
    };

    assert_writes_do_not_create_resource_events(&store).await;
}

#[cfg(feature = "nats-tests")]
#[tokio::test]
async fn nats_view_store_lists_all_index_with_empty_prefix() {
    let Some(store) = nats_view_store("all_index").await else {
        return;
    };

    assert_lists_all_index_with_empty_prefix(&store).await;
}

#[cfg(feature = "nats-tests")]
#[tokio::test]
async fn nats_view_store_lists_matching_index_prefix_only() {
    let Some(store) = nats_view_store("matching_prefix").await else {
        return;
    };

    assert_lists_matching_index_prefix_only(&store).await;
}

#[cfg(feature = "nats-tests")]
#[tokio::test]
async fn nats_view_store_index_listing_isolates_view_types_and_index_names() {
    let Some(store) = nats_view_store("isolated_indexes").await else {
        return;
    };

    assert_index_listing_isolates_view_types_and_index_names(&store).await;
}

#[cfg(feature = "nats-tests")]
#[tokio::test]
async fn nats_view_store_overwrite_replaces_index_membership() {
    let Some(store) = nats_view_store("replace_index_membership").await else {
        return;
    };

    assert_overwrite_replaces_index_membership(&store).await;
}

#[cfg(feature = "nats-tests")]
#[tokio::test]
async fn nats_view_store_missing_index_returns_empty_list() {
    let Some(store) = nats_view_store("missing_index").await else {
        return;
    };

    assert_missing_index_returns_empty_list(&store).await;
}

#[cfg(feature = "nats-tests")]
#[tokio::test]
async fn nats_view_store_rejects_duplicate_index_names_in_one_document() {
    let Some(store) = nats_view_store("duplicate_index").await else {
        return;
    };

    assert_rejects_duplicate_index_names_in_one_document(&store).await;
}

async fn assert_stores_and_loads_view_document_by_identity<S>(store: &S)
where
    S: ViewStore,
{
    let document = offer_summary_view("offer-1", "Initial offer");

    store.put(document.clone()).await.expect("put view");

    let loaded = store
        .load(&ViewKey::new("offer_summary", "offer-1"))
        .await
        .expect("load view");

    assert_eq!(loaded, Some(document));
}

async fn assert_returns_none_for_missing_view<S>(store: &S)
where
    S: ViewStore,
{
    let loaded = store
        .load(&ViewKey::new("offer_summary", "missing-offer"))
        .await
        .expect("load missing view");

    assert!(loaded.is_none());
}

async fn assert_overwrites_existing_view_document<S>(store: &S)
where
    S: ViewStore,
{
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

async fn assert_keeps_view_types_and_ids_isolated<S>(store: &S)
where
    S: ViewStore,
{
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

async fn assert_writes_do_not_create_resource_events<S>(view_store: &S)
where
    S: ViewStore,
{
    let event_store = InMemoryEventStore::new();

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

async fn assert_lists_all_index_with_empty_prefix<S>(store: &S)
where
    S: ViewStore,
{
    store
        .put(indexed_offer_summary_view(
            "offer-2",
            "Second offer",
            "draft",
        ))
        .await
        .expect("put second offer");
    store
        .put(indexed_offer_summary_view(
            "offer-1",
            "Initial offer",
            "accepted",
        ))
        .await
        .expect("put first offer");

    let listed = store
        .list_by_index_prefix("offer_summary", "all", "")
        .await
        .expect("list all offer summaries");

    assert_eq!(view_ids(&listed), vec!["offer-1", "offer-2"]);
}

async fn assert_lists_matching_index_prefix_only<S>(store: &S)
where
    S: ViewStore,
{
    store
        .put(indexed_offer_summary_view(
            "offer-1",
            "Accepted offer",
            "accepted",
        ))
        .await
        .expect("put accepted offer");
    store
        .put(indexed_offer_summary_view(
            "offer-2",
            "Draft offer",
            "draft",
        ))
        .await
        .expect("put draft offer");
    store
        .put(indexed_offer_summary_view(
            "offer-3",
            "Accepted offer 2",
            "accepted",
        ))
        .await
        .expect("put second accepted offer");

    let listed = store
        .list_by_index_prefix("offer_summary", "by_status", "accepted/")
        .await
        .expect("list accepted offers");

    assert_eq!(view_ids(&listed), vec!["offer-1", "offer-3"]);
}

async fn assert_index_listing_isolates_view_types_and_index_names<S>(store: &S)
where
    S: ViewStore,
{
    store
        .put(indexed_offer_summary_view(
            "shared-id",
            "Initial offer",
            "accepted",
        ))
        .await
        .expect("put offer summary");
    store
        .put(
            ViewDocument::new("flow_status", "shared-id", json!({ "status": "accepted" }))
                .with_indexes(vec![
                    ViewIndexEntry::new("all", "shared-id"),
                    ViewIndexEntry::new("by_status", "accepted/shared-id"),
                ]),
        )
        .await
        .expect("put flow status");

    let wrong_view_type = store
        .list_by_index_prefix("invoice_summary", "all", "")
        .await
        .expect("list wrong view type");
    let wrong_index_name = store
        .list_by_index_prefix("offer_summary", "by_actor", "accepted/")
        .await
        .expect("list wrong index name");
    let offer_summary = store
        .list_by_index_prefix("offer_summary", "by_status", "accepted/")
        .await
        .expect("list offer summary by status");

    assert!(wrong_view_type.is_empty());
    assert!(wrong_index_name.is_empty());
    assert_eq!(view_ids(&offer_summary), vec!["shared-id"]);
}

async fn assert_overwrite_replaces_index_membership<S>(store: &S)
where
    S: ViewStore,
{
    store
        .put(indexed_offer_summary_view(
            "offer-1",
            "Draft offer",
            "draft",
        ))
        .await
        .expect("put draft offer");
    store
        .put(indexed_offer_summary_view(
            "offer-1",
            "Accepted offer",
            "accepted",
        ))
        .await
        .expect("put accepted offer");

    let draft = store
        .list_by_index_prefix("offer_summary", "by_status", "draft/")
        .await
        .expect("list draft offers");
    let accepted = store
        .list_by_index_prefix("offer_summary", "by_status", "accepted/")
        .await
        .expect("list accepted offers");

    assert!(draft.is_empty());
    assert_eq!(view_ids(&accepted), vec!["offer-1"]);
}

async fn assert_missing_index_returns_empty_list<S>(store: &S)
where
    S: ViewStore,
{
    store
        .put(offer_summary_view("offer-1", "Initial offer"))
        .await
        .expect("put unindexed view");

    let listed = store
        .list_by_index_prefix("offer_summary", "all", "")
        .await
        .expect("list missing index");

    assert!(listed.is_empty());
}

async fn assert_rejects_duplicate_index_names_in_one_document<S>(store: &S)
where
    S: ViewStore,
{
    let document = ViewDocument::new(
        "offer_summary",
        "offer-1",
        json!({
            "offer_id": "offer-1",
            "title": "Initial offer",
        }),
    )
    .with_indexes(vec![
        ViewIndexEntry::new("by_status", "draft/offer-1"),
        ViewIndexEntry::new("by_status", "accepted/offer-1"),
    ]);

    let err = store
        .put(document)
        .await
        .expect_err("duplicate index names in one view document should be rejected");

    match err {
        ViewStoreError::DuplicateIndexName {
            view_type,
            view_id,
            index_name,
        } => {
            assert_eq!(view_type, "offer_summary");
            assert_eq!(view_id, "offer-1");
            assert_eq!(index_name, "by_status");
        }
        other => panic!("expected DuplicateIndexName view store error, got {other:?}"),
    }

    let loaded = store
        .load(&ViewKey::new("offer_summary", "offer-1"))
        .await
        .expect("load rejected duplicate-index view");
    assert!(loaded.is_none());
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

fn indexed_offer_summary_view(offer_id: &str, title: &str, status: &str) -> ViewDocument {
    ViewDocument::new(
        "offer_summary",
        offer_id,
        json!({
            "offer_id": offer_id,
            "title": title,
            "status": status,
        }),
    )
    .with_indexes(vec![
        ViewIndexEntry::new("all", offer_id),
        ViewIndexEntry::new("by_status", format!("{status}/{offer_id}")),
    ])
}

fn view_ids(documents: &[ViewDocument]) -> Vec<&str> {
    documents
        .iter()
        .map(|document| document.key.view_id.as_str())
        .collect()
}

#[cfg(feature = "nats-tests")]
async fn nats_view_store(test_name: &str) -> Option<NatsViewStore> {
    let harness = match support::nats::NatsHarnessConfig::from_env() {
        Ok(harness) => harness,
        Err(skip) => {
            eprintln!("{}", skip.reason());
            return None;
        }
    };

    let config = NatsViewStoreConfig::new(unique_nats_bucket_name(test_name));
    Some(
        NatsViewStore::connect(harness.url(), config)
            .await
            .expect("connect NATS ViewStore"),
    )
}

#[cfg(feature = "nats-tests")]
fn unique_nats_bucket_name(test_name: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after UNIX_EPOCH")
        .as_nanos();

    format!("elbmesh_view_store_{test_name}_{nanos}")
}
