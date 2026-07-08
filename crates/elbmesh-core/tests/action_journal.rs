use elbmesh_core::{
    ActionJournal, ActionJournalError, ActionJournalRecord, ActionJournalStream, ActionMetadata,
    ActionReceipt, ActionStatus, EventStore, InMemoryActionJournal, InMemoryEventStore,
    MessageMetadata, ResourceStream, StreamType,
};

#[cfg(feature = "nats-tests")]
use elbmesh_core::{NatsActionJournal, NatsActionJournalConfig};

use serde_json::json;

#[cfg(feature = "nats-tests")]
mod support;

#[test]
fn in_memory_action_journal_implements_action_journal_trait() {
    fn assert_action_journal<T: ActionJournal>() {}

    assert_action_journal::<InMemoryActionJournal>();
}

#[tokio::test]
async fn in_memory_action_journal_appends_action_called_and_action_completed_records() {
    let journal = InMemoryActionJournal::new();

    assert_appends_action_called_and_completed_records(&journal).await;
}

#[tokio::test]
async fn in_memory_action_journal_reads_records_in_append_order_for_action_stream() {
    let journal = InMemoryActionJournal::new();

    assert_reads_records_in_append_order_for_action_stream(&journal).await;
}

#[tokio::test]
async fn in_memory_action_journal_writes_do_not_create_resource_events() {
    let event_store = InMemoryEventStore::new();
    let journal = InMemoryActionJournal::new();
    let action = action_metadata("action-journal-separated-from-events");
    let stream = ActionJournalStream::for_action(action.action_id.clone());

    journal
        .append(&stream, action_called_record(&action, "offer-123"))
        .await
        .expect("append ActionCalled record");
    journal
        .append(&stream, action_completed_record(&action, "offer-123"))
        .await
        .expect("append ActionCompleted record");

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
        .expect("load action journal records");
    assert_eq!(journal_records.len(), 2);
}

#[tokio::test]
async fn in_memory_action_journal_rejects_wrong_action_stream_with_named_error() {
    let journal = InMemoryActionJournal::new();

    assert_rejects_wrong_action_stream_with_named_error(&journal).await;
}

#[cfg(feature = "nats-tests")]
#[test]
fn nats_action_journal_implements_action_journal_trait() {
    fn assert_action_journal<T: ActionJournal>() {}

    assert_action_journal::<NatsActionJournal>();
}

#[cfg(feature = "nats-tests")]
#[tokio::test]
async fn nats_action_journal_appends_action_called_and_action_completed_records() {
    let Some(journal) = nats_action_journal("called_completed").await else {
        return;
    };

    assert_appends_action_called_and_completed_records(&journal).await;
}

#[cfg(feature = "nats-tests")]
#[tokio::test]
async fn nats_action_journal_reads_records_in_append_order_for_action_stream() {
    let Some(journal) = nats_action_journal("append_order").await else {
        return;
    };

    assert_reads_records_in_append_order_for_action_stream(&journal).await;
}

#[cfg(feature = "nats-tests")]
#[tokio::test]
async fn nats_action_journal_writes_do_not_create_resource_events() {
    let Some(journal) = nats_action_journal("separate_from_events").await else {
        return;
    };
    let event_store = InMemoryEventStore::new();
    let action = action_metadata("nats-action-journal-separated-from-events");
    let stream = ActionJournalStream::for_action(action.action_id.clone());

    journal
        .append(&stream, action_called_record(&action, "offer-123"))
        .await
        .expect("append ActionCalled record");
    journal
        .append(&stream, action_completed_record(&action, "offer-123"))
        .await
        .expect("append ActionCompleted record");

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
        .expect("load action journal records");
    assert_eq!(journal_records.len(), 2);
}

#[cfg(feature = "nats-tests")]
#[tokio::test]
async fn nats_action_journal_rejects_wrong_action_stream_with_named_error() {
    let Some(journal) = nats_action_journal("wrong_action_stream").await else {
        return;
    };

    assert_rejects_wrong_action_stream_with_named_error(&journal).await;
}

async fn assert_appends_action_called_and_completed_records<J>(journal: &J)
where
    J: ActionJournal,
{
    let action = action_metadata("action-journal-called-completed");
    let stream = ActionJournalStream::for_action(action.action_id.clone());

    journal
        .append(&stream, action_called_record(&action, "offer-123"))
        .await
        .expect("append ActionCalled record");
    journal
        .append(&stream, action_completed_record(&action, "offer-123"))
        .await
        .expect("append ActionCompleted record");

    let records = journal
        .load(&stream)
        .await
        .expect("load action journal records");

    assert_eq!(records.len(), 2);
    assert_action_called_record(&records[0], &action, "offer-123");
    assert_action_completed_record(&records[1], &action, "offer-123");
}

async fn assert_reads_records_in_append_order_for_action_stream<J>(journal: &J)
where
    J: ActionJournal,
{
    let action = action_metadata("action-journal-append-order");
    let stream = ActionJournalStream::for_action(action.action_id.clone());

    let called = action_called_record(&action, "offer-123");
    let completed = action_completed_record(&action, "offer-123");

    journal
        .append(&stream, called)
        .await
        .expect("append first action journal record");
    journal
        .append(&stream, completed)
        .await
        .expect("append second action journal record");

    let records = journal
        .load(&stream)
        .await
        .expect("load action journal records");

    let message_types: Vec<_> = records
        .iter()
        .map(|record| match record {
            ActionJournalRecord::ActionCalled { metadata, .. } => metadata.message_type.as_str(),
            ActionJournalRecord::ActionCompleted { metadata, .. } => metadata.message_type.as_str(),
            ActionJournalRecord::ActionRejected { metadata, .. } => metadata.message_type.as_str(),
            ActionJournalRecord::ActionFailed { metadata, .. } => metadata.message_type.as_str(),
        })
        .collect();

    assert_eq!(message_types, vec!["action_called", "action_completed"]);
}

async fn assert_rejects_wrong_action_stream_with_named_error<J>(journal: &J)
where
    J: ActionJournal,
{
    let stream = ActionJournalStream::for_action("expected-action-id");
    let action = action_metadata("actual-action-id");

    let err = journal
        .append(&stream, action_called_record(&action, "offer-123"))
        .await
        .expect_err("wrong action stream should fail");

    match err {
        ActionJournalError::WrongActionStream {
            expected_action_id,
            actual_action_id,
        } => {
            assert_eq!(expected_action_id, "expected-action-id");
            assert_eq!(actual_action_id, "actual-action-id");
        }
        other => panic!("expected WrongActionStream journal error, got {other:?}"),
    }
}

fn action_metadata(action_id: &str) -> ActionMetadata {
    ActionMetadata::with_ids(
        action_id,
        format!("correlation-{action_id}"),
        format!("causation-{action_id}"),
        "actor-123",
    )
}

fn action_called_record(action: &ActionMetadata, offer_id: &str) -> ActionJournalRecord {
    ActionJournalRecord::ActionCalled {
        metadata: action_record_metadata(
            "action_called",
            "journal.action_called.v1",
            action,
            offer_id,
        ),
        action_type: "create_offer".to_string(),
        action_schema_id: "action.create_offer.v1".to_string(),
        action_schema_version: 1,
        payload: json!({
            "offer_id": offer_id,
            "title": "Initial offer",
        }),
    }
}

fn action_completed_record(action: &ActionMetadata, offer_id: &str) -> ActionJournalRecord {
    ActionJournalRecord::ActionCompleted {
        metadata: action_record_metadata(
            "action_completed",
            "journal.action_completed.v1",
            action,
            offer_id,
        ),
        receipt: ActionReceipt {
            action_id: action.action_id.clone(),
            status: ActionStatus::Completed,
            resource_type: "offer".to_string(),
            resource_id: offer_id.to_string(),
            previous_version: 0,
            new_version: 0,
            emitted_events: Vec::new(),
            message: Some("offer created".to_string()),
        },
    }
}

fn action_record_metadata(
    message_type: &str,
    schema_id: &str,
    action: &ActionMetadata,
    offer_id: &str,
) -> MessageMetadata {
    MessageMetadata {
        message_id: format!("{message_type}-{}", action.action_id),
        message_type: message_type.to_string(),
        message_version: 1,
        resource_type: "offer".to_string(),
        resource_id: offer_id.to_string(),
        stream_type: StreamType::Action,
        correlation_id: action.correlation_id.clone(),
        causation_id: action.causation_id.clone(),
        action_id: action.action_id.clone(),
        actor_id: action.actor_id.clone(),
        occurred_at: "2026-07-06T00:00:00Z".to_string(),
        schema_id: schema_id.to_string(),
        schema_version: 1,
    }
}

fn assert_action_called_record(
    record: &ActionJournalRecord,
    action: &ActionMetadata,
    offer_id: &str,
) {
    match record {
        ActionJournalRecord::ActionCalled {
            metadata,
            action_type,
            action_schema_id,
            action_schema_version,
            payload,
        } => {
            assert_action_record_metadata(metadata, "action_called", action, offer_id);
            assert_eq!(action_type, "create_offer");
            assert_eq!(action_schema_id, "action.create_offer.v1");
            assert_eq!(*action_schema_version, 1);
            assert_eq!(
                payload,
                &json!({
                    "offer_id": offer_id,
                    "title": "Initial offer",
                })
            );
        }
        ActionJournalRecord::ActionCompleted { .. }
        | ActionJournalRecord::ActionRejected { .. }
        | ActionJournalRecord::ActionFailed { .. } => panic!("expected ActionCalled record"),
    }
}

fn assert_action_completed_record(
    record: &ActionJournalRecord,
    action: &ActionMetadata,
    offer_id: &str,
) {
    match record {
        ActionJournalRecord::ActionCompleted { metadata, receipt } => {
            assert_action_record_metadata(metadata, "action_completed", action, offer_id);
            assert_eq!(receipt.action_id, action.action_id);
            assert_eq!(receipt.status, ActionStatus::Completed);
            assert_eq!(receipt.resource_type, "offer");
            assert_eq!(receipt.resource_id, offer_id);
            assert_eq!(receipt.previous_version, 0);
            assert_eq!(receipt.new_version, 0);
            assert!(receipt.emitted_events.is_empty());
            assert_eq!(receipt.message.as_deref(), Some("offer created"));
        }
        ActionJournalRecord::ActionCalled { .. }
        | ActionJournalRecord::ActionRejected { .. }
        | ActionJournalRecord::ActionFailed { .. } => {
            panic!("expected ActionCompleted record")
        }
    }
}

fn assert_action_record_metadata(
    metadata: &MessageMetadata,
    message_type: &str,
    action: &ActionMetadata,
    offer_id: &str,
) {
    assert_eq!(metadata.message_type, message_type);
    assert_eq!(metadata.message_version, 1);
    assert_eq!(metadata.resource_type, "offer");
    assert_eq!(metadata.resource_id, offer_id);
    assert_eq!(metadata.stream_type, StreamType::Action);
    assert_eq!(metadata.correlation_id, action.correlation_id);
    assert_eq!(metadata.causation_id, action.causation_id);
    assert_eq!(metadata.action_id, action.action_id);
    assert_eq!(metadata.actor_id, action.actor_id);
    assert_eq!(metadata.schema_id, format!("journal.{message_type}.v1"));
    assert_eq!(metadata.schema_version, 1);
}

#[cfg(feature = "nats-tests")]
async fn nats_action_journal(test_name: &str) -> Option<NatsActionJournal> {
    let harness = match support::nats::NatsHarnessConfig::from_env() {
        Ok(harness) => harness,
        Err(skip) => {
            eprintln!("{}", skip.reason());
            return None;
        }
    };

    let config = NatsActionJournalConfig::new(unique_nats_bucket_name(test_name));
    Some(
        NatsActionJournal::connect(harness.url(), config)
            .await
            .expect("connect NATS ActionJournal"),
    )
}

#[cfg(feature = "nats-tests")]
fn unique_nats_bucket_name(test_name: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after UNIX_EPOCH")
        .as_nanos();

    format!("elbmesh_action_journal_{test_name}_{nanos}")
}
