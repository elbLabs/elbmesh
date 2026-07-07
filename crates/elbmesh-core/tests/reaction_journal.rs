use elbmesh_core::{
    EventStore, InMemoryEventStore, InMemoryReactionJournal, ReactionJournal, ReactionJournalError,
    ReactionJournalRecord, ReactionJournalStream, ResourceStream, StreamType,
};

#[test]
fn in_memory_reaction_journal_implements_reaction_journal_trait() {
    fn assert_reaction_journal<T: ReactionJournal>() {}

    assert_reaction_journal::<InMemoryReactionJournal>();
}

#[tokio::test]
async fn in_memory_reaction_journal_appends_triggered_and_completed_records() {
    let journal = InMemoryReactionJournal::new();

    assert_appends_triggered_and_completed_records(&journal).await;
}

#[tokio::test]
async fn in_memory_reaction_journal_reads_records_in_append_order_for_reaction_stream() {
    let journal = InMemoryReactionJournal::new();

    assert_reads_records_in_append_order_for_reaction_stream(&journal).await;
}

#[tokio::test]
async fn in_memory_reaction_journal_writes_do_not_create_resource_events() {
    let event_store = InMemoryEventStore::new();
    let journal = InMemoryReactionJournal::new();
    let reaction_id = "reaction-journal-separated-from-events";
    let stream = ReactionJournalStream::for_reaction(reaction_id);

    journal
        .append(&stream, reaction_triggered_record(reaction_id, "offer-123"))
        .await
        .expect("append ReactionTriggered record");
    journal
        .append(&stream, reaction_completed_record(reaction_id, "offer-123"))
        .await
        .expect("append ReactionCompleted record");

    let resource_stream = ResourceStream::new("offer", "offer-123");
    let resource_events = event_store
        .load(&resource_stream)
        .await
        .expect("load resource events");

    assert!(resource_events.is_empty());
    assert!(event_store.all_events().is_empty());

    let journal_records = journal
        .load(&stream)
        .await
        .expect("load reaction journal records");
    assert_eq!(journal_records.len(), 2);
}

#[tokio::test]
async fn in_memory_reaction_journal_rejects_wrong_reaction_stream_with_named_error() {
    let journal = InMemoryReactionJournal::new();

    assert_rejects_wrong_reaction_stream_with_named_error(&journal).await;
}

async fn assert_appends_triggered_and_completed_records<J>(journal: &J)
where
    J: ReactionJournal,
{
    let reaction_id = "reaction-journal-triggered-completed";
    let stream = ReactionJournalStream::for_reaction(reaction_id);

    journal
        .append(&stream, reaction_triggered_record(reaction_id, "offer-123"))
        .await
        .expect("append ReactionTriggered record");
    journal
        .append(&stream, reaction_completed_record(reaction_id, "offer-123"))
        .await
        .expect("append ReactionCompleted record");

    let records = journal
        .load(&stream)
        .await
        .expect("load reaction journal records");

    assert_eq!(records.len(), 2);
    assert_reaction_triggered_record(&records[0], reaction_id, "offer-123");
    assert_reaction_completed_record(&records[1], reaction_id, "offer-123");
}

async fn assert_reads_records_in_append_order_for_reaction_stream<J>(journal: &J)
where
    J: ReactionJournal,
{
    let reaction_id = "reaction-journal-append-order";
    let stream = ReactionJournalStream::for_reaction(reaction_id);

    let triggered = reaction_triggered_record(reaction_id, "offer-123");
    let completed = reaction_completed_record(reaction_id, "offer-123");

    journal
        .append(&stream, triggered)
        .await
        .expect("append first reaction journal record");
    journal
        .append(&stream, completed)
        .await
        .expect("append second reaction journal record");

    let records = journal
        .load(&stream)
        .await
        .expect("load reaction journal records");

    let message_types: Vec<_> = records
        .iter()
        .map(|record| match record {
            ReactionJournalRecord::ReactionTriggered { metadata, .. } => {
                metadata.message_type.as_str()
            }
            ReactionJournalRecord::ReactionCompleted { metadata, .. } => {
                metadata.message_type.as_str()
            }
        })
        .collect();

    assert_eq!(
        message_types,
        vec!["reaction_triggered", "reaction_completed"]
    );
}

async fn assert_rejects_wrong_reaction_stream_with_named_error<J>(journal: &J)
where
    J: ReactionJournal,
{
    let stream = ReactionJournalStream::for_reaction("expected-reaction-id");

    let err = journal
        .append(
            &stream,
            reaction_triggered_record("actual-reaction-id", "offer-123"),
        )
        .await
        .expect_err("wrong reaction stream should fail");

    match err {
        ReactionJournalError::WrongReactionStream {
            expected_reaction_id,
            actual_reaction_id,
        } => {
            assert_eq!(expected_reaction_id, "expected-reaction-id");
            assert_eq!(actual_reaction_id, "actual-reaction-id");
        }
        other => panic!("expected WrongReactionStream journal error, got {other:?}"),
    }
}

fn reaction_triggered_record(reaction_id: &str, offer_id: &str) -> ReactionJournalRecord {
    ReactionJournalRecord::ReactionTriggered {
        reaction_id: reaction_id.to_string(),
        metadata: reaction_record_metadata(
            "reaction_triggered",
            "journal.reaction_triggered.v1",
            reaction_id,
            offer_id,
        ),
        reaction_type: "offer_accepted_to_create_sales_order".to_string(),
        trigger_event_type: "offer_accepted".to_string(),
        trigger_event_id: "offer-accepted-event-1".to_string(),
    }
}

fn reaction_completed_record(reaction_id: &str, offer_id: &str) -> ReactionJournalRecord {
    ReactionJournalRecord::ReactionCompleted {
        reaction_id: reaction_id.to_string(),
        metadata: reaction_record_metadata(
            "reaction_completed",
            "journal.reaction_completed.v1",
            reaction_id,
            offer_id,
        ),
        triggered_action_ids: vec!["create-sales-order-action-1".to_string()],
    }
}

fn reaction_record_metadata(
    message_type: &str,
    schema_id: &str,
    reaction_id: &str,
    offer_id: &str,
) -> elbmesh_core::MessageMetadata {
    elbmesh_core::MessageMetadata {
        message_id: format!("{message_type}-{reaction_id}"),
        message_type: message_type.to_string(),
        message_version: 1,
        resource_type: "offer".to_string(),
        resource_id: offer_id.to_string(),
        stream_type: StreamType::Reaction,
        correlation_id: format!("correlation-{reaction_id}"),
        causation_id: "offer-accepted-event-1".to_string(),
        action_id: "accept-offer-action-1".to_string(),
        actor_id: "reaction-runtime".to_string(),
        occurred_at: "2026-07-07T00:00:00Z".to_string(),
        schema_id: schema_id.to_string(),
        schema_version: 1,
    }
}

fn assert_reaction_triggered_record(
    record: &ReactionJournalRecord,
    reaction_id: &str,
    offer_id: &str,
) {
    match record {
        ReactionJournalRecord::ReactionTriggered {
            reaction_id: actual_reaction_id,
            metadata,
            reaction_type,
            trigger_event_type,
            trigger_event_id,
        } => {
            assert_eq!(actual_reaction_id, reaction_id);
            assert_reaction_record_metadata(metadata, "reaction_triggered", reaction_id, offer_id);
            assert_eq!(reaction_type, "offer_accepted_to_create_sales_order");
            assert_eq!(trigger_event_type, "offer_accepted");
            assert_eq!(trigger_event_id, "offer-accepted-event-1");
        }
        other => panic!("expected ReactionTriggered record, got {other:?}"),
    }
}

fn assert_reaction_completed_record(
    record: &ReactionJournalRecord,
    reaction_id: &str,
    offer_id: &str,
) {
    match record {
        ReactionJournalRecord::ReactionCompleted {
            reaction_id: actual_reaction_id,
            metadata,
            triggered_action_ids,
        } => {
            assert_eq!(actual_reaction_id, reaction_id);
            assert_reaction_record_metadata(metadata, "reaction_completed", reaction_id, offer_id);
            assert_eq!(triggered_action_ids, &["create-sales-order-action-1"]);
        }
        other => panic!("expected ReactionCompleted record, got {other:?}"),
    }
}

fn assert_reaction_record_metadata(
    metadata: &elbmesh_core::MessageMetadata,
    message_type: &str,
    reaction_id: &str,
    offer_id: &str,
) {
    assert_eq!(metadata.message_id, format!("{message_type}-{reaction_id}"));
    assert_eq!(metadata.message_type, message_type);
    assert_eq!(metadata.message_version, 1);
    assert_eq!(metadata.resource_type, "offer");
    assert_eq!(metadata.resource_id, offer_id);
    assert_eq!(metadata.stream_type, StreamType::Reaction);
    assert_eq!(
        metadata.correlation_id,
        format!("correlation-{reaction_id}")
    );
    assert_eq!(metadata.causation_id, "offer-accepted-event-1");
    assert_eq!(metadata.action_id, "accept-offer-action-1");
    assert_eq!(metadata.actor_id, "reaction-runtime");
    assert_eq!(metadata.schema_id, format!("journal.{message_type}.v1"));
    assert_eq!(metadata.schema_version, 1);
}
