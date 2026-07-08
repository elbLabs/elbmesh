use elbmesh_core::{
    EventStore, InMemoryEventStore, InMemoryOperationJournal, MessageMetadata, OperationJournal,
    OperationJournalError, OperationJournalRecord, OperationJournalStream, ResourceStream,
    StreamType,
};

use serde_json::json;

#[test]
fn in_memory_operation_journal_implements_operation_journal_trait() {
    fn assert_operation_journal<J: OperationJournal>() {}

    assert_operation_journal::<InMemoryOperationJournal>();
}

#[tokio::test]
async fn in_memory_operation_journal_appends_called_and_completed_records() {
    let journal = InMemoryOperationJournal::new();

    assert_appends_called_and_completed_records(&journal).await;
}

#[tokio::test]
async fn in_memory_operation_journal_reads_records_in_append_order_for_operation_stream() {
    let journal = InMemoryOperationJournal::new();

    assert_reads_records_in_append_order_for_operation_stream(&journal).await;
}

#[tokio::test]
async fn in_memory_operation_journal_writes_do_not_create_resource_events() {
    let event_store = InMemoryEventStore::new();
    let journal = InMemoryOperationJournal::new();
    let operation_id = "operation-journal-separated-from-events";
    let stream = OperationJournalStream::for_operation(operation_id);

    journal
        .append(&stream, operation_called_record(operation_id, "offer-123"))
        .await
        .expect("append OperationCalled record");
    journal
        .append(
            &stream,
            operation_completed_record(operation_id, "offer-123"),
        )
        .await
        .expect("append OperationCompleted record");

    let resource_stream = ResourceStream::new("offer", "offer-123");
    let resource_events = event_store
        .load(&resource_stream)
        .await
        .expect("load resource events");

    assert!(resource_events.is_empty());
    assert!(event_store.all_events().is_empty());

    let operation_records = journal
        .load(&stream)
        .await
        .expect("load operation journal records");
    assert_eq!(operation_records.len(), 2);
}

#[tokio::test]
async fn in_memory_operation_journal_rejects_wrong_operation_stream_with_named_error() {
    let journal = InMemoryOperationJournal::new();

    assert_rejects_wrong_operation_stream_with_named_error(&journal).await;
}

async fn assert_appends_called_and_completed_records<J>(journal: &J)
where
    J: OperationJournal,
{
    let operation_id = "operation-journal-called-completed";
    let stream = OperationJournalStream::for_operation(operation_id);

    journal
        .append(&stream, operation_called_record(operation_id, "offer-123"))
        .await
        .expect("append OperationCalled record");
    journal
        .append(
            &stream,
            operation_completed_record(operation_id, "offer-123"),
        )
        .await
        .expect("append OperationCompleted record");

    let records = journal
        .load(&stream)
        .await
        .expect("load operation journal records");

    assert_eq!(records.len(), 2);
    assert_operation_called_record(&records[0], operation_id, "offer-123");
    assert_operation_completed_record(&records[1], operation_id, "offer-123");
}

async fn assert_reads_records_in_append_order_for_operation_stream<J>(journal: &J)
where
    J: OperationJournal,
{
    let operation_id = "operation-journal-append-order";
    let stream = OperationJournalStream::for_operation(operation_id);

    journal
        .append(&stream, operation_called_record(operation_id, "offer-123"))
        .await
        .expect("append first operation journal record");
    journal
        .append(
            &stream,
            operation_completed_record(operation_id, "offer-123"),
        )
        .await
        .expect("append second operation journal record");

    let records = journal
        .load(&stream)
        .await
        .expect("load operation journal records");

    let message_types: Vec<_> = records
        .iter()
        .map(|record| match record {
            OperationJournalRecord::OperationCalled { metadata, .. } => {
                metadata.message_type.as_str()
            }
            OperationJournalRecord::OperationCompleted { metadata, .. } => {
                metadata.message_type.as_str()
            }
            OperationJournalRecord::OperationFailed { metadata, .. } => {
                metadata.message_type.as_str()
            }
        })
        .collect();

    assert_eq!(
        message_types,
        vec!["operation_called", "operation_completed"]
    );
}

async fn assert_rejects_wrong_operation_stream_with_named_error<J>(journal: &J)
where
    J: OperationJournal,
{
    let stream = OperationJournalStream::for_operation("expected-operation-id");

    let err = journal
        .append(
            &stream,
            operation_called_record("actual-operation-id", "offer-123"),
        )
        .await
        .expect_err("wrong operation stream should fail");

    match err {
        OperationJournalError::WrongOperationStream {
            expected_operation_id,
            actual_operation_id,
        } => {
            assert_eq!(expected_operation_id, "expected-operation-id");
            assert_eq!(actual_operation_id, "actual-operation-id");
        }
        other => panic!("expected WrongOperationStream journal error, got {other:?}"),
    }
}

fn operation_called_record(operation_id: &str, offer_id: &str) -> OperationJournalRecord {
    OperationJournalRecord::OperationCalled {
        operation_id: operation_id.to_string(),
        metadata: operation_record_metadata(
            "operation_called",
            "journal.operation_called.v1",
            operation_id,
            offer_id,
        ),
        operation_type: "create_invoice".to_string(),
        operation_schema_id: "operation.create_invoice.v1".to_string(),
        operation_schema_version: 1,
        idempotency_key: format!("create_invoice:{operation_id}"),
        payload: json!({
            "offer_id": offer_id,
            "amount": 1200,
        }),
    }
}

fn operation_completed_record(operation_id: &str, offer_id: &str) -> OperationJournalRecord {
    OperationJournalRecord::OperationCompleted {
        operation_id: operation_id.to_string(),
        metadata: operation_record_metadata(
            "operation_completed",
            "journal.operation_completed.v1",
            operation_id,
            offer_id,
        ),
        response: json!({
            "provider": "lexoffice",
            "provider_invoice_id": "invoice-123",
        }),
    }
}

fn operation_record_metadata(
    message_type: &str,
    schema_id: &str,
    operation_id: &str,
    offer_id: &str,
) -> MessageMetadata {
    MessageMetadata {
        message_id: format!("{message_type}-{operation_id}"),
        message_type: message_type.to_string(),
        message_version: 1,
        resource_type: "offer".to_string(),
        resource_id: offer_id.to_string(),
        stream_type: StreamType::Operation,
        correlation_id: format!("correlation-{operation_id}"),
        causation_id: format!("causation-{operation_id}"),
        action_id: "action-123".to_string(),
        actor_id: "actor-123".to_string(),
        occurred_at: "2026-07-08T00:00:00Z".to_string(),
        schema_id: schema_id.to_string(),
        schema_version: 1,
    }
}

fn assert_operation_called_record(
    record: &OperationJournalRecord,
    operation_id: &str,
    offer_id: &str,
) {
    match record {
        OperationJournalRecord::OperationCalled {
            operation_id: actual_operation_id,
            metadata,
            operation_type,
            operation_schema_id,
            operation_schema_version,
            idempotency_key,
            payload,
        } => {
            assert_eq!(actual_operation_id, operation_id);
            assert_operation_record_metadata(metadata, "operation_called", operation_id, offer_id);
            assert_eq!(operation_type, "create_invoice");
            assert_eq!(operation_schema_id, "operation.create_invoice.v1");
            assert_eq!(*operation_schema_version, 1);
            assert_eq!(idempotency_key, &format!("create_invoice:{operation_id}"));
            assert_eq!(
                payload,
                &json!({
                    "offer_id": offer_id,
                    "amount": 1200,
                })
            );
        }
        OperationJournalRecord::OperationCompleted { .. }
        | OperationJournalRecord::OperationFailed { .. } => {
            panic!("expected OperationCalled record")
        }
    }
}

fn assert_operation_completed_record(
    record: &OperationJournalRecord,
    operation_id: &str,
    offer_id: &str,
) {
    match record {
        OperationJournalRecord::OperationCompleted {
            operation_id: actual_operation_id,
            metadata,
            response,
        } => {
            assert_eq!(actual_operation_id, operation_id);
            assert_operation_record_metadata(
                metadata,
                "operation_completed",
                operation_id,
                offer_id,
            );
            assert_eq!(response["provider"], "lexoffice");
            assert_eq!(response["provider_invoice_id"], "invoice-123");
        }
        OperationJournalRecord::OperationCalled { .. }
        | OperationJournalRecord::OperationFailed { .. } => {
            panic!("expected OperationCompleted record")
        }
    }
}

fn assert_operation_record_metadata(
    metadata: &MessageMetadata,
    message_type: &str,
    operation_id: &str,
    offer_id: &str,
) {
    assert_eq!(
        metadata.message_id,
        format!("{message_type}-{operation_id}")
    );
    assert_eq!(metadata.message_type, message_type);
    assert_eq!(metadata.message_version, 1);
    assert_eq!(metadata.resource_type, "offer");
    assert_eq!(metadata.resource_id, offer_id);
    assert_eq!(metadata.stream_type, StreamType::Operation);
    assert_eq!(
        metadata.correlation_id,
        format!("correlation-{operation_id}")
    );
    assert_eq!(metadata.causation_id, format!("causation-{operation_id}"));
    assert_eq!(metadata.action_id, "action-123");
    assert_eq!(metadata.actor_id, "actor-123");
    assert_eq!(metadata.schema_id, format!("journal.{message_type}.v1"));
    assert_eq!(metadata.schema_version, 1);
}
