use elbmesh_core::{
    EventStore, EventStoreError, ExpectedVersion, InMemoryEventStore, MessageMetadata, NewEvent,
    ResourceStream, StreamType,
};

use serde_json::json;

#[tokio::test]
async fn in_memory_event_store_appends_and_loads_resource_events_in_sequence() {
    event_store_appends_and_loads_resource_events_in_sequence(InMemoryEventStore::new()).await;
}

#[tokio::test]
async fn in_memory_event_store_isolates_resource_streams() {
    event_store_isolates_resource_streams(InMemoryEventStore::new()).await;
}

#[tokio::test]
async fn in_memory_event_store_rejects_event_metadata_resource_identity_mismatch() {
    event_store_rejects_event_metadata_resource_identity_mismatch(InMemoryEventStore::new()).await;
}

#[tokio::test]
async fn in_memory_event_store_rejects_non_resource_event_metadata() {
    event_store_rejects_non_resource_event_metadata(InMemoryEventStore::new()).await;
}

async fn event_store_appends_and_loads_resource_events_in_sequence<S>(store: S)
where
    S: EventStore,
{
    let stream = ResourceStream::new("offer", "offer-1");
    let first = new_event("offer-created-event-1", "offer_created", "offer-1");
    let second = new_event("offer-accepted-event-1", "offer_accepted", "offer-1");

    let append = store
        .append(&stream, ExpectedVersion::NoStream, vec![first, second])
        .await
        .expect("append should succeed");

    assert_eq!(append.previous_version, 0);
    assert_eq!(append.new_version, 2);
    assert_eq!(append.events.len(), 2);
    assert_eq!(append.events[0].sequence, 1);
    assert_eq!(append.events[1].sequence, 2);

    let loaded = store.load(&stream).await.expect("load should succeed");
    assert_eq!(loaded, append.events);
}

async fn event_store_isolates_resource_streams<S>(store: S)
where
    S: EventStore,
{
    let first_stream = ResourceStream::new("offer", "offer-1");
    let second_stream = ResourceStream::new("offer", "offer-2");

    store
        .append(
            &first_stream,
            ExpectedVersion::NoStream,
            vec![new_event(
                "offer-created-event-1",
                "offer_created",
                "offer-1",
            )],
        )
        .await
        .expect("append first stream");
    store
        .append(
            &second_stream,
            ExpectedVersion::NoStream,
            vec![new_event(
                "offer-created-event-2",
                "offer_created",
                "offer-2",
            )],
        )
        .await
        .expect("append second stream");

    let first_loaded = store.load(&first_stream).await.expect("load first stream");
    let second_loaded = store
        .load(&second_stream)
        .await
        .expect("load second stream");

    assert_eq!(first_loaded.len(), 1);
    assert_eq!(first_loaded[0].metadata.resource_id, "offer-1");
    assert_eq!(second_loaded.len(), 1);
    assert_eq!(second_loaded[0].metadata.resource_id, "offer-2");
}

async fn event_store_rejects_event_metadata_resource_identity_mismatch<S>(store: S)
where
    S: EventStore,
{
    let stream = ResourceStream::new("offer", "offer-1");
    let wrong_identity = new_event("offer-created-event-1", "offer_created", "offer-2");

    let err = store
        .append(&stream, ExpectedVersion::NoStream, vec![wrong_identity])
        .await
        .expect_err("metadata resource identity mismatch should fail");

    match err {
        EventStoreError::WrongEventStream {
            stream,
            expected_resource_type,
            expected_resource_id,
            actual_resource_type,
            actual_resource_id,
        } => {
            assert_eq!(stream, "resources.offer.offer-1");
            assert_eq!(expected_resource_type, "offer");
            assert_eq!(expected_resource_id, "offer-1");
            assert_eq!(actual_resource_type, "offer");
            assert_eq!(actual_resource_id, "offer-2");
        }
        other => panic!("expected WrongEventStream, got {other:?}"),
    }

    let loaded = store
        .load(&stream)
        .await
        .expect("load rejected stream should succeed");
    assert!(loaded.is_empty());
}

async fn event_store_rejects_non_resource_event_metadata<S>(store: S)
where
    S: EventStore,
{
    let stream = ResourceStream::new("offer", "offer-1");
    let mut wrong_stream_type = new_event("offer-created-event-1", "offer_created", "offer-1");
    wrong_stream_type.metadata.stream_type = StreamType::Reaction;

    let err = store
        .append(&stream, ExpectedVersion::NoStream, vec![wrong_stream_type])
        .await
        .expect_err("non-resource event metadata should fail");

    match err {
        EventStoreError::WrongEventStreamType {
            stream,
            expected_stream_type,
            actual_stream_type,
        } => {
            assert_eq!(stream, "resources.offer.offer-1");
            assert_eq!(expected_stream_type, StreamType::Resource);
            assert_eq!(actual_stream_type, StreamType::Reaction);
        }
        other => panic!("expected WrongEventStreamType, got {other:?}"),
    }

    let loaded = store
        .load(&stream)
        .await
        .expect("load rejected stream should succeed");
    assert!(loaded.is_empty());
}

fn new_event(message_id: &str, message_type: &str, offer_id: &str) -> NewEvent {
    NewEvent {
        metadata: MessageMetadata {
            message_id: message_id.to_string(),
            message_type: message_type.to_string(),
            message_version: 1,
            resource_type: "offer".to_string(),
            resource_id: offer_id.to_string(),
            stream_type: StreamType::Resource,
            correlation_id: "correlation-1".to_string(),
            causation_id: "causation-1".to_string(),
            action_id: "action-1".to_string(),
            actor_id: "actor-1".to_string(),
            occurred_at: "2026-01-01T00:00:00Z".to_string(),
            schema_id: format!("event.{message_type}.v1"),
            schema_version: 1,
        },
        payload: json!({ "offer_id": offer_id }),
    }
}
