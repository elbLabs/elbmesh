use elbmesh_core::{
    EventStore, InMemoryEventStore, InMemoryViewStore, ResourceStream, ViewDocument,
    ViewIndexEntry, ViewKey, ViewStore, ViewStoreError,
};

#[cfg(feature = "nats-tests")]
use elbmesh_core::{NatsViewStore, NatsViewStoreConfig, ProjectionContext, ProjectionCursor};

use serde_json::json;

#[cfg(feature = "nats-tests")]
use std::sync::Arc;

#[cfg(feature = "nats-tests")]
use tokio::sync::Barrier;

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

#[cfg(feature = "nats-tests")]
#[tokio::test]
async fn nats_view_store_atomically_persists_document_and_source_application_metadata() {
    let Some((store, raw_store)) = nats_view_store_with_raw_store("atomic_projection").await else {
        return;
    };
    let document = offer_summary_view("offer-1", "Projected offer");
    let context = projection_context("event-5", "offer-1", 5, b"opaque-cursor");

    assert!(store
        .apply_projection("offer_summary_from_offer", &context, document.clone())
        .await
        .expect("apply projection"));

    let (_, stored) = raw_stored_view(&raw_store).await;
    assert_eq!(
        stored["document"],
        serde_json::to_value(&document).expect("serialize expected View document")
    );
    let applications = stored["applications"]
        .as_array()
        .expect("stored View should contain application metadata");
    assert_eq!(applications.len(), 1);
    assert_eq!(
        applications[0]["projection_type"],
        "offer_summary_from_offer"
    );
    assert_eq!(
        applications[0]["source_stream"],
        serde_json::to_value(context.source_stream()).expect("serialize expected source stream")
    );
    assert_eq!(applications[0]["source_message_id"], "event-5");
    assert_eq!(applications[0]["aggregate_sequence"], 5);
}

#[cfg(feature = "nats-tests")]
#[tokio::test]
async fn nats_view_store_retries_cas_conflicts_without_losing_cross_stream_metadata() {
    const CONCURRENT_SOURCES: usize = 12;

    let Some((store, raw_store)) = nats_view_store_with_raw_store("cas_retry").await else {
        return;
    };
    let store = Arc::new(store);
    let start = Arc::new(Barrier::new(CONCURRENT_SOURCES + 1));
    let mut applications = Vec::new();

    for source in 0..CONCURRENT_SOURCES {
        let store = Arc::clone(&store);
        let start = Arc::clone(&start);
        applications.push(tokio::spawn(async move {
            let source_id = format!("offer-{source}");
            let context =
                projection_context(&format!("event-{source}"), &source_id, 1, &[source as u8]);
            start.wait().await;
            store
                .apply_projection(
                    "cross_stream_summary",
                    &context,
                    offer_summary_view("offer-1", "Concurrent projection"),
                )
                .await
        }));
    }

    start.wait().await;
    for application in applications {
        assert!(application
            .await
            .expect("concurrent projection task should join")
            .expect("CAS conflict should retry instead of escaping"));
    }

    let (_, stored) = raw_stored_view(&raw_store).await;
    let metadata = stored["applications"]
        .as_array()
        .expect("stored View should contain application metadata");
    assert_eq!(metadata.len(), CONCURRENT_SOURCES);
    for source in 0..CONCURRENT_SOURCES {
        let source_id = format!("offer-{source}");
        assert!(metadata.iter().any(|application| {
            application["projection_type"] == "cross_stream_summary"
                && application["source_stream"]["resource_id"] == source_id
                && application["aggregate_sequence"] == 1
        }));
    }
}

#[cfg(feature = "nats-tests")]
#[tokio::test]
async fn nats_view_store_duplicate_and_older_same_source_are_exact_no_ops() {
    let Some((store, raw_store)) = nats_view_store_with_raw_store("same_source_no_op").await else {
        return;
    };
    let context = projection_context("event-5", "offer-1", 5, b"cursor-5");
    let newest = offer_summary_view("offer-1", "Newest offer");

    assert!(store
        .apply_projection("offer_summary_from_offer", &context, newest.clone())
        .await
        .expect("apply newest projection"));
    let (revision, _) = raw_stored_view(&raw_store).await;

    assert!(!store
        .apply_projection(
            "offer_summary_from_offer",
            &context,
            offer_summary_view("offer-1", "Duplicate must not write"),
        )
        .await
        .expect("exact duplicate should be a no-op"));
    let older = projection_context("event-4", "offer-1", 4, b"cursor-4");
    assert!(!store
        .apply_projection(
            "offer_summary_from_offer",
            &older,
            offer_summary_view("offer-1", "Older must not write"),
        )
        .await
        .expect("older same-source position should be a no-op"));

    let (revision_after_no_ops, _) = raw_stored_view(&raw_store).await;
    assert_eq!(revision_after_no_ops, revision);
    assert_eq!(
        store
            .load(&ViewKey::new("offer_summary", "offer-1"))
            .await
            .expect("load newest View")
            .expect("newest View should exist"),
        newest
    );
}

#[cfg(feature = "nats-tests")]
#[tokio::test]
async fn nats_view_store_decodes_and_migrates_legacy_documents_on_projection() {
    let Some((store, raw_store)) = nats_view_store_with_raw_store("legacy_migration").await else {
        return;
    };
    let legacy = offer_summary_view("offer-1", "Legacy offer");
    raw_store
        .put(
            NATS_OFFER_SUMMARY_KEY,
            serde_json::to_vec(&legacy)
                .expect("serialize legacy View document")
                .into(),
        )
        .await
        .expect("write legacy View document");

    assert_eq!(
        store
            .load(&legacy.key)
            .await
            .expect("decode legacy View document")
            .expect("legacy View should exist"),
        legacy
    );

    let migrated = offer_summary_view("offer-1", "Migrated offer");
    let context = projection_context("event-1", "offer-1", 1, b"legacy-migration");
    assert!(store
        .apply_projection("offer_summary_from_offer", &context, migrated.clone())
        .await
        .expect("projection should migrate legacy storage"));

    let (_, stored) = raw_stored_view(&raw_store).await;
    assert_eq!(
        stored["document"],
        serde_json::to_value(&migrated).expect("serialize migrated View")
    );
    assert_eq!(
        stored["applications"]
            .as_array()
            .expect("migrated storage should contain application metadata")
            .len(),
        1
    );
    assert!(!store
        .apply_projection("offer_summary_from_offer", &context, migrated)
        .await
        .expect("migrated duplicate should be a no-op"));
}

#[cfg(feature = "nats-tests")]
#[tokio::test]
async fn nats_view_store_projection_reset_is_idempotent_and_clears_application_metadata() {
    let Some((store, _raw_store)) = nats_view_store_with_raw_store("idempotent_reset").await else {
        return;
    };
    let key = ViewKey::new("offer_summary", "offer-1");
    let context = projection_context("event-1", "offer-1", 1, b"reset-cursor");

    assert!(store
        .apply_projection(
            "offer_summary_from_offer",
            &context,
            offer_summary_view("offer-1", "Before reset"),
        )
        .await
        .expect("apply projection before reset"));
    store
        .reset_projection("offer_summary_from_offer", &key)
        .await
        .expect("first projection reset");
    store
        .reset_projection("offer_summary_from_offer", &key)
        .await
        .expect("second projection reset should be idempotent");
    assert!(store.load(&key).await.expect("load reset View").is_none());

    assert!(store
        .apply_projection(
            "offer_summary_from_offer",
            &context,
            offer_summary_view("offer-1", "After reset"),
        )
        .await
        .expect("reset metadata should allow the same source position to replay"));
    assert!(!store
        .apply_projection(
            "offer_summary_from_offer",
            &context,
            offer_summary_view("offer-1", "Duplicate replay"),
        )
        .await
        .expect("post-reset duplicate should be a no-op"));
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
const NATS_OFFER_SUMMARY_KEY: &str = "view.13.offer_summary.7.offer-1";

#[cfg(feature = "nats-tests")]
fn projection_context(
    message_id: &str,
    source_id: &str,
    sequence: u64,
    cursor: &[u8],
) -> ProjectionContext {
    ProjectionContext::new(
        message_id,
        ResourceStream::new("offer", source_id),
        sequence,
        ProjectionCursor::new(cursor.to_vec()),
    )
}

#[cfg(feature = "nats-tests")]
async fn raw_stored_view(store: &async_nats::jetstream::kv::Store) -> (u64, serde_json::Value) {
    let entry = store
        .entry(NATS_OFFER_SUMMARY_KEY)
        .await
        .expect("load raw stored View")
        .expect("raw stored View should exist");
    (
        entry.revision,
        serde_json::from_slice(&entry.value).expect("decode raw stored View"),
    )
}

#[cfg(feature = "nats-tests")]
async fn nats_view_store_with_raw_store(
    test_name: &str,
) -> Option<(NatsViewStore, async_nats::jetstream::kv::Store)> {
    let harness = match support::nats::NatsHarnessConfig::from_env() {
        Ok(harness) => harness,
        Err(skip) => {
            eprintln!("{}", skip.reason());
            return None;
        }
    };
    let client = async_nats::connect(harness.url())
        .await
        .expect("connect live NATS test client");
    let jetstream = async_nats::jetstream::new(client);
    let raw_store = jetstream
        .create_or_update_key_value(async_nats::jetstream::kv::Config {
            bucket: unique_nats_bucket_name(test_name),
            ..Default::default()
        })
        .await
        .expect("create live NATS ViewStore bucket");
    Some((NatsViewStore::from_store(raw_store.clone()), raw_store))
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
