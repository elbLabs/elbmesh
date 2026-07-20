use std::{
    fmt,
    sync::{Arc, Mutex, OnceLock},
};

use async_trait::async_trait;
use elbmesh_core::{
    apply_recorded_event, Action, ActionContext, ActionDecision, ActionError, ActionExecutor,
    ActionFailure, ActionFailureClassification, ActionJournal, ActionJournalRecord,
    ActionJournalStream, ActionMetadata, ActionStatus, AppendResult, Apply,
    CreateLexOfficeInvoiceRequest, Event, EventStore, EventStoreError, ExecutionError,
    ExpectedVersion, ExternalOperation, Handle, HandlerError, InMemoryActionJournal,
    InMemoryEventStore, InMemoryOperationJournal, MockLexOfficeCreateInvoice, NewEvent,
    OperationJournal, OperationJournalRecord, OperationJournalStream, RecordedEvent, Resource,
    ResourceError, ResourceStream, StreamType,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[cfg(feature = "restate-adapter")]
use elbmesh_core::{RestateOperationJournal, RestateOperationJournalConfig};

#[cfg(feature = "restate-tests")]
mod support;

#[tokio::test]
async fn action_context_executes_external_operation_with_deterministic_operation_metadata() {
    let operation = MockLexOfficeCreateInvoice::new();
    let request = lexoffice_request("inv-1", "oc-1");
    let ctx = ActionContext::<Invoice>::new(action_metadata("act-1"), "invoice", "inv-1", 0);

    let result = ctx
        .execute_external_operation(&operation, request.clone())
        .await
        .expect("external operation should succeed");

    let expected_idempotency_key = operation.idempotency_key(&request);
    assert_eq!(result.idempotency_key, expected_idempotency_key);
    assert_eq!(
        result.operation_id,
        "action.5.act-1.operation.24.lexoffice_create_invoice.idempotency.69.lexoffice_create_invoice.v1.invoice.5.inv-1.order_confirmation.4.oc-1"
    );
    assert_eq!(
        operation.created_invoice_count().expect("count invoices"),
        1
    );
}

#[tokio::test]
async fn action_context_maps_external_operation_failure_to_structured_action_error() {
    let operation = MockLexOfficeCreateInvoice::new();
    let request = lexoffice_request("inv-1", "oc-1");
    let ctx = ActionContext::<Invoice>::new(action_metadata("act-1"), "invoice", "inv-1", 0);

    operation
        .fail_next_create()
        .expect("mark next create as failed");

    let err = ctx
        .execute_external_operation(&operation, request)
        .await
        .expect_err("provider failure should become ActionError");

    assert_eq!(err.code(), "action.external_operation");
    assert_eq!(
        err.details(),
        json!({
            "error_type": "ActionError",
            "error_variant": "ExternalOperation",
            "operation_type": "lexoffice_create_invoice",
            "failure_code": "lexoffice.create_invoice.provider_unavailable",
            "failure_details": {
                "error_type": "LexOfficeCreateInvoiceError",
                "error_variant": "ProviderUnavailable",
            },
        })
    );

    match err {
        ActionError::ExternalOperation {
            operation_type,
            failure_code,
            failure_details,
        } => {
            assert_eq!(operation_type, "lexoffice_create_invoice");
            assert_eq!(
                failure_code,
                "lexoffice.create_invoice.provider_unavailable"
            );
            assert_eq!(
                failure_details,
                json!({
                    "error_type": "LexOfficeCreateInvoiceError",
                    "error_variant": "ProviderUnavailable",
                })
            );
        }
        other => panic!("expected external operation ActionError, got {other:?}"),
    }
}

#[tokio::test]
async fn action_failed_preserves_external_operation_details_and_resource_event_separation() {
    let event_store = InMemoryEventStore::new();
    let action_journal = InMemoryActionJournal::new();
    let executor =
        ActionExecutor::new(event_store.clone()).with_action_journal(action_journal.clone());
    let metadata = action_metadata("action-failing-external-operation");

    let err = executor
        .execute::<Invoice, _>(
            CreateInvoiceWithFailingLexOfficeV1 {
                invoice_id: "invoice-failing-external-operation".to_string(),
                order_confirmation_id: "order-confirmation-failing-external-operation".to_string(),
                customer_id: "customer-failing-external-operation".to_string(),
                amount_cents: 12_345,
            },
            metadata.clone(),
        )
        .await
        .expect_err("provider failure should fail action execution");

    match err {
        ExecutionError::Handler(HandlerError::Runtime(ActionError::ExternalOperation {
            operation_type,
            failure_code,
            failure_details,
        })) => {
            assert_eq!(operation_type, "lexoffice_create_invoice");
            assert_eq!(
                failure_code,
                "lexoffice.create_invoice.provider_unavailable"
            );
            assert_eq!(
                failure_details,
                json!({
                    "error_type": "LexOfficeCreateInvoiceError",
                    "error_variant": "ProviderUnavailable",
                })
            );
        }
        other => panic!("expected typed external operation execution error, got {other:?}"),
    }

    let records = action_journal
        .load(&ActionJournalStream::for_action(&metadata.action_id))
        .await
        .expect("load ActionJournal records");
    assert_eq!(records.len(), 2);
    match &records[1] {
        ActionJournalRecord::ActionFailed {
            failure_classification,
            failure_details,
            ..
        } => {
            assert_eq!(
                failure_classification,
                &ActionFailureClassification::HandlerRuntime
            );
            assert_eq!(
                failure_details,
                &json!({
                    "failure_code": "action.external_operation",
                    "failure_details": {
                        "error_type": "ActionError",
                        "error_variant": "ExternalOperation",
                        "operation_type": "lexoffice_create_invoice",
                        "failure_code": "lexoffice.create_invoice.provider_unavailable",
                        "failure_details": {
                            "error_type": "LexOfficeCreateInvoiceError",
                            "error_variant": "ProviderUnavailable",
                        },
                    },
                })
            );
        }
        other => panic!("expected ActionFailed record, got {other:?}"),
    }

    let resource_stream =
        ResourceStream::new(Invoice::RESOURCE_TYPE, "invoice-failing-external-operation");
    let events = event_store
        .load(&resource_stream)
        .await
        .expect("load Resource Events");
    assert!(events.is_empty());
    assert!(event_store.all_events().is_empty());
}

#[tokio::test]
async fn action_executor_handler_records_domain_event_from_external_operation_result_only() {
    let event_store = InMemoryEventStore::new();
    let executor = ActionExecutor::new(event_store);

    let receipt = executor
        .execute::<Invoice, CreateInvoiceThroughLexOfficeV1>(
            CreateInvoiceThroughLexOfficeV1 {
                invoice_id: "invoice-123".to_string(),
                order_confirmation_id: "order-confirmation-123".to_string(),
                customer_id: "customer-123".to_string(),
                amount_cents: 12_345,
            },
            action_metadata("action-create-invoice-through-lexoffice"),
        )
        .await
        .expect("invoice action should succeed");

    assert_eq!(receipt.status, ActionStatus::Completed);
    assert_eq!(receipt.emitted_events.len(), 1);

    let stream = ResourceStream::new("invoice", "invoice-123");
    let events = executor
        .event_store()
        .load(&stream)
        .await
        .expect("load invoice events");

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].metadata.stream_type, StreamType::Resource);
    assert_eq!(
        events[0].metadata.message_type,
        "invoice_created_through_lexoffice"
    );
    assert_eq!(
        events[0].payload,
        json!({
            "invoice_id": "invoice-123",
            "provider_invoice_id": "lexoffice-invoice-1",
        })
    );
    assert!(events[0].payload.get("operation_id").is_none());
    assert!(events[0].payload.get("operation_type").is_none());
    assert!(events[0].payload.get("idempotency_key").is_none());
}

#[tokio::test]
async fn retry_after_append_failure_reuses_completed_external_operation() {
    let event_store = AppendFailsOnceEventStore::new(InMemoryEventStore::new());
    let operation_journal = InMemoryOperationJournal::new();
    let executor =
        ActionExecutor::new(event_store.clone()).with_operation_journal(operation_journal.clone());
    let action = CreateRetryInvoiceThroughLexOfficeV1 {
        invoice_id: "retry-invoice-123".to_string(),
        order_confirmation_id: "retry-order-confirmation-123".to_string(),
        customer_id: "customer-123".to_string(),
        amount_cents: 12_345,
    };
    let metadata = action_metadata("retry-action-1");
    let request = lexoffice_request("retry-invoice-123", "retry-order-confirmation-123");
    let operation = retry_lexoffice_operation();
    let idempotency_key = operation.idempotency_key(&request);
    let operation_id = expected_operation_id(
        &metadata.action_id,
        MockLexOfficeCreateInvoice::OPERATION_TYPE,
        &idempotency_key,
    );

    let first_err = executor
        .execute::<Invoice, CreateRetryInvoiceThroughLexOfficeV1>(action.clone(), metadata.clone())
        .await
        .expect_err("first attempt should fail after external operation succeeds");

    assert!(matches!(
        first_err,
        ExecutionError::EventStore(EventStoreError::Other(reason)) if reason == "append failed once"
    ));
    assert_eq!(
        operation.created_invoice_count().expect("count invoices"),
        1
    );

    let stream = OperationJournalStream::for_operation(operation_id.clone());
    let records = operation_journal
        .load(&stream)
        .await
        .expect("load operation records after failed append");
    assert_eq!(records.len(), 2);
    assert_operation_called_record(&records[0], &operation_id, &idempotency_key);
    assert_operation_completed_record(&records[1], &operation_id);

    let receipt = executor
        .execute::<Invoice, CreateRetryInvoiceThroughLexOfficeV1>(action, metadata)
        .await
        .expect("retry should reuse completed operation and append event");

    assert_eq!(receipt.status, ActionStatus::Completed);
    assert_eq!(
        operation.created_invoice_count().expect("count invoices"),
        1
    );
    assert_eq!(event_store.append_attempts(), 2);

    let resource_stream = ResourceStream::new("invoice", "retry-invoice-123");
    let events = executor
        .event_store()
        .load(&resource_stream)
        .await
        .expect("load retry invoice events");

    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].payload,
        json!({
            "invoice_id": "retry-invoice-123",
            "provider_invoice_id": "lexoffice-invoice-1",
        })
    );

    let records_after_retry = operation_journal
        .load(&stream)
        .await
        .expect("load operation records after retry");
    assert_eq!(records_after_retry.len(), 2);
}

#[cfg(feature = "restate-adapter")]
#[tokio::test]
async fn restate_operation_journal_retry_after_append_failure_reuses_completed_external_operation()
{
    let restate = FakeRestateOperationJournalServer::start();
    let event_store = AppendFailsOnceEventStore::new(InMemoryEventStore::new());
    let operation_journal =
        RestateOperationJournal::new(RestateOperationJournalConfig::new(restate.base_url()));
    let executor =
        ActionExecutor::new(event_store.clone()).with_operation_journal(operation_journal.clone());
    let action = CreateRestateRetryInvoiceThroughLexOfficeV1 {
        invoice_id: "restate-retry-invoice-123".to_string(),
        order_confirmation_id: "restate-retry-order-confirmation-123".to_string(),
        customer_id: "customer-123".to_string(),
        amount_cents: 12_345,
    };
    let metadata = action_metadata("restate-retry-action-1");
    let request = lexoffice_request(
        "restate-retry-invoice-123",
        "restate-retry-order-confirmation-123",
    );
    let operation = restate_retry_lexoffice_operation();
    let idempotency_key = operation.idempotency_key(&request);
    let operation_id = expected_operation_id(
        &metadata.action_id,
        MockLexOfficeCreateInvoice::OPERATION_TYPE,
        &idempotency_key,
    );

    let first_err = executor
        .execute::<Invoice, CreateRestateRetryInvoiceThroughLexOfficeV1>(
            action.clone(),
            metadata.clone(),
        )
        .await
        .expect_err("first Restate-backed attempt should fail after external operation succeeds");

    assert!(matches!(
        first_err,
        ExecutionError::EventStore(EventStoreError::Other(reason)) if reason == "append failed once"
    ));
    assert_eq!(
        operation.created_invoice_count().expect("count invoices"),
        1
    );

    let stream = OperationJournalStream::for_operation(operation_id.clone());
    let records = operation_journal
        .load(&stream)
        .await
        .expect("load Restate operation records after failed append");
    assert_eq!(records.len(), 2);
    assert_operation_called_record(&records[0], &operation_id, &idempotency_key);
    assert_operation_completed_record(&records[1], &operation_id);

    let receipt = executor
        .execute::<Invoice, CreateRestateRetryInvoiceThroughLexOfficeV1>(action, metadata)
        .await
        .expect("Restate-backed retry should reuse completed operation and append event");

    assert_eq!(receipt.status, ActionStatus::Completed);
    assert_eq!(
        operation.created_invoice_count().expect("count invoices"),
        1
    );
    assert_eq!(event_store.append_attempts(), 2);

    let resource_stream = ResourceStream::new("invoice", "restate-retry-invoice-123");
    let events = executor
        .event_store()
        .load(&resource_stream)
        .await
        .expect("load Restate retry invoice events");

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].metadata.stream_type, StreamType::Resource);
    assert_eq!(
        events[0].payload,
        json!({
            "invoice_id": "restate-retry-invoice-123",
            "provider_invoice_id": "lexoffice-invoice-1",
        })
    );
    assert!(events[0].payload.get("operation_id").is_none());
    assert!(events[0].payload.get("operation_type").is_none());
    assert!(events[0].payload.get("idempotency_key").is_none());

    let records_after_retry = operation_journal
        .load(&stream)
        .await
        .expect("load Restate operation records after retry");
    assert_eq!(records_after_retry.len(), 2);
}

#[cfg(feature = "restate-tests")]
#[tokio::test]
async fn live_restate_operation_journal_retry_after_append_failure_reuses_completed_external_operation(
) {
    let Some((operation_journal, _endpoint)) = live_restate_operation_journal().await else {
        return;
    };
    let unique = unique_live_restate_suffix();
    let event_store = AppendFailsOnceEventStore::new(InMemoryEventStore::new());
    let executor =
        ActionExecutor::new(event_store.clone()).with_operation_journal(operation_journal.clone());
    let action = CreateLiveRestateRetryInvoiceThroughLexOfficeV1 {
        invoice_id: format!("live-restate-retry-invoice-{unique}"),
        order_confirmation_id: format!("live-restate-retry-order-confirmation-{unique}"),
        customer_id: "customer-123".to_string(),
        amount_cents: 12_345,
    };
    let metadata = action_metadata(&format!("live-restate-retry-action-{unique}"));
    let request = lexoffice_request(&action.invoice_id, &action.order_confirmation_id);
    let operation = live_restate_retry_lexoffice_operation();
    let idempotency_key = operation.idempotency_key(&request);
    let operation_id = expected_operation_id(
        &metadata.action_id,
        MockLexOfficeCreateInvoice::OPERATION_TYPE,
        &idempotency_key,
    );

    let first_err = executor
        .execute::<Invoice, CreateLiveRestateRetryInvoiceThroughLexOfficeV1>(
            action.clone(),
            metadata.clone(),
        )
        .await
        .expect_err("first live Restate-backed attempt should fail after operation succeeds");

    assert!(matches!(
        first_err,
        ExecutionError::EventStore(EventStoreError::Other(reason)) if reason == "append failed once"
    ));
    assert_eq!(
        operation.created_invoice_count().expect("count invoices"),
        1
    );

    let stream = OperationJournalStream::for_operation(operation_id.clone());
    let records = operation_journal
        .load(&stream)
        .await
        .expect("load live Restate operation records after failed append");
    assert_eq!(records.len(), 2);
    assert_operation_called_record(&records[0], &operation_id, &idempotency_key);
    assert_operation_completed_record(&records[1], &operation_id);

    let receipt = executor
        .execute::<Invoice, CreateLiveRestateRetryInvoiceThroughLexOfficeV1>(
            action.clone(),
            metadata,
        )
        .await
        .expect("live Restate-backed retry should reuse completed operation and append event");

    assert_eq!(receipt.status, ActionStatus::Completed);
    assert_eq!(
        operation.created_invoice_count().expect("count invoices"),
        1
    );
    assert_eq!(event_store.append_attempts(), 2);

    let resource_stream = ResourceStream::new("invoice", &action.invoice_id);
    let events = executor
        .event_store()
        .load(&resource_stream)
        .await
        .expect("load live Restate retry invoice events");

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].metadata.stream_type, StreamType::Resource);
    assert_eq!(
        events[0].payload,
        json!({
            "invoice_id": action.invoice_id,
            "provider_invoice_id": "lexoffice-invoice-1",
        })
    );
    assert!(events[0].payload.get("operation_id").is_none());
    assert!(events[0].payload.get("operation_type").is_none());
    assert!(events[0].payload.get("idempotency_key").is_none());

    let records_after_retry = operation_journal
        .load(&stream)
        .await
        .expect("load live Restate operation records after retry");
    assert_eq!(records_after_retry.len(), 2);
}

#[derive(Debug, Default, Clone)]
struct Invoice {
    id: Option<String>,
    provider_invoice_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum InvoiceError {
    AlreadyExists,
}

impl fmt::Display for InvoiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyExists => write!(f, "invoice already exists"),
        }
    }
}

impl ActionFailure for InvoiceError {
    fn code(&self) -> &'static str {
        match self {
            Self::AlreadyExists => "invoice.already_exists",
        }
    }

    fn details(&self) -> serde_json::Value {
        json!({
            "error_type": "InvoiceError",
            "error_variant": match self {
                Self::AlreadyExists => "AlreadyExists",
            },
        })
    }
}

impl Resource for Invoice {
    type Id = String;

    const RESOURCE_TYPE: &'static str = "invoice";

    fn apply_recorded(&mut self, event: &elbmesh_core::RecordedEvent) -> Result<(), ResourceError> {
        if apply_recorded_event::<Self, InvoiceCreatedThroughLexOfficeV1>(self, event)? {
            return Ok(());
        }

        Err(ResourceError::UnsupportedEvent {
            resource_type: Self::RESOURCE_TYPE.to_string(),
            message_type: event.metadata.message_type.clone(),
            schema_version: event.metadata.schema_version,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateInvoiceThroughLexOfficeV1 {
    invoice_id: String,
    order_confirmation_id: String,
    customer_id: String,
    amount_cents: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateInvoiceWithFailingLexOfficeV1 {
    invoice_id: String,
    order_confirmation_id: String,
    customer_id: String,
    amount_cents: u64,
}

impl Action for CreateInvoiceWithFailingLexOfficeV1 {
    type Resource = Invoice;

    const ACTION_TYPE: &'static str = "create_invoice_with_failing_lexoffice";
    const SCHEMA_ID: &'static str = "action.create_invoice_with_failing_lexoffice.v1";
    const SCHEMA_VERSION: u32 = 1;

    fn resource_id(&self) -> <Self::Resource as Resource>::Id {
        self.invoice_id.clone()
    }
}

impl Action for CreateInvoiceThroughLexOfficeV1 {
    type Resource = Invoice;

    const ACTION_TYPE: &'static str = "create_invoice_through_lexoffice";
    const SCHEMA_ID: &'static str = "action.create_invoice_through_lexoffice.v1";
    const SCHEMA_VERSION: u32 = 1;

    fn resource_id(&self) -> <Self::Resource as Resource>::Id {
        self.invoice_id.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateRetryInvoiceThroughLexOfficeV1 {
    invoice_id: String,
    order_confirmation_id: String,
    customer_id: String,
    amount_cents: u64,
}

impl Action for CreateRetryInvoiceThroughLexOfficeV1 {
    type Resource = Invoice;

    const ACTION_TYPE: &'static str = "create_retry_invoice_through_lexoffice";
    const SCHEMA_ID: &'static str = "action.create_retry_invoice_through_lexoffice.v1";
    const SCHEMA_VERSION: u32 = 1;

    fn resource_id(&self) -> <Self::Resource as Resource>::Id {
        self.invoice_id.clone()
    }
}

#[cfg(feature = "restate-adapter")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateRestateRetryInvoiceThroughLexOfficeV1 {
    invoice_id: String,
    order_confirmation_id: String,
    customer_id: String,
    amount_cents: u64,
}

#[cfg(feature = "restate-tests")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateLiveRestateRetryInvoiceThroughLexOfficeV1 {
    invoice_id: String,
    order_confirmation_id: String,
    customer_id: String,
    amount_cents: u64,
}

#[cfg(feature = "restate-tests")]
impl Action for CreateLiveRestateRetryInvoiceThroughLexOfficeV1 {
    type Resource = Invoice;

    const ACTION_TYPE: &'static str = "create_live_restate_retry_invoice_through_lexoffice";
    const SCHEMA_ID: &'static str = "action.create_live_restate_retry_invoice_through_lexoffice.v1";
    const SCHEMA_VERSION: u32 = 1;

    fn resource_id(&self) -> <Self::Resource as Resource>::Id {
        self.invoice_id.clone()
    }
}

#[cfg(feature = "restate-adapter")]
impl Action for CreateRestateRetryInvoiceThroughLexOfficeV1 {
    type Resource = Invoice;

    const ACTION_TYPE: &'static str = "create_restate_retry_invoice_through_lexoffice";
    const SCHEMA_ID: &'static str = "action.create_restate_retry_invoice_through_lexoffice.v1";
    const SCHEMA_VERSION: u32 = 1;

    fn resource_id(&self) -> <Self::Resource as Resource>::Id {
        self.invoice_id.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InvoiceCreatedThroughLexOfficeV1 {
    invoice_id: String,
    provider_invoice_id: String,
}

impl Event for InvoiceCreatedThroughLexOfficeV1 {
    type Resource = Invoice;

    const EVENT_TYPE: &'static str = "invoice_created_through_lexoffice";
    const SCHEMA_ID: &'static str = "event.invoice_created_through_lexoffice.v1";
    const SCHEMA_VERSION: u32 = 1;

    fn resource_id(&self) -> <Self::Resource as Resource>::Id {
        self.invoice_id.clone()
    }
}

impl Apply<InvoiceCreatedThroughLexOfficeV1> for Invoice {
    fn apply(&mut self, event: InvoiceCreatedThroughLexOfficeV1) -> Result<(), ResourceError> {
        self.id = Some(event.invoice_id);
        self.provider_invoice_id = Some(event.provider_invoice_id);
        Ok(())
    }
}

#[async_trait]
impl Handle<CreateInvoiceThroughLexOfficeV1> for Invoice {
    type Error = InvoiceError;

    async fn handle(
        &mut self,
        action: CreateInvoiceThroughLexOfficeV1,
        ctx: &mut ActionContext<Self>,
    ) -> Result<ActionDecision, HandlerError<Self::Error>> {
        if self.id.is_some() {
            return Err(HandlerError::domain(InvoiceError::AlreadyExists));
        }

        let result = ctx
            .execute_external_operation(
                &MockLexOfficeCreateInvoice::new(),
                CreateLexOfficeInvoiceRequest {
                    invoice_id: action.invoice_id.clone(),
                    order_confirmation_id: action.order_confirmation_id,
                    customer_id: action.customer_id,
                    amount_cents: action.amount_cents,
                },
            )
            .await?;

        ctx.record_applied(
            self,
            InvoiceCreatedThroughLexOfficeV1 {
                invoice_id: action.invoice_id,
                provider_invoice_id: result.provider_invoice_id,
            },
        )?;

        Ok(ActionDecision::with_message(
            "invoice created through lexoffice",
        ))
    }
}

#[async_trait]
impl Handle<CreateInvoiceWithFailingLexOfficeV1> for Invoice {
    type Error = InvoiceError;

    async fn handle(
        &mut self,
        action: CreateInvoiceWithFailingLexOfficeV1,
        ctx: &mut ActionContext<Self>,
    ) -> Result<ActionDecision, HandlerError<Self::Error>> {
        let operation = MockLexOfficeCreateInvoice::new();
        operation
            .fail_next_create()
            .expect("configure provider failure fixture");

        ctx.execute_external_operation(
            &operation,
            CreateLexOfficeInvoiceRequest {
                invoice_id: action.invoice_id,
                order_confirmation_id: action.order_confirmation_id,
                customer_id: action.customer_id,
                amount_cents: action.amount_cents,
            },
        )
        .await?;

        Ok(ActionDecision::with_message(
            "provider unexpectedly accepted invoice",
        ))
    }
}

#[async_trait]
impl Handle<CreateRetryInvoiceThroughLexOfficeV1> for Invoice {
    type Error = InvoiceError;

    async fn handle(
        &mut self,
        action: CreateRetryInvoiceThroughLexOfficeV1,
        ctx: &mut ActionContext<Self>,
    ) -> Result<ActionDecision, HandlerError<Self::Error>> {
        if self.id.is_some() {
            return Err(HandlerError::domain(InvoiceError::AlreadyExists));
        }

        let result = ctx
            .execute_external_operation(
                &retry_lexoffice_operation(),
                CreateLexOfficeInvoiceRequest {
                    invoice_id: action.invoice_id.clone(),
                    order_confirmation_id: action.order_confirmation_id,
                    customer_id: action.customer_id,
                    amount_cents: action.amount_cents,
                },
            )
            .await?;

        ctx.record_applied(
            self,
            InvoiceCreatedThroughLexOfficeV1 {
                invoice_id: action.invoice_id,
                provider_invoice_id: result.provider_invoice_id,
            },
        )?;

        Ok(ActionDecision::with_message(
            "retry invoice created through lexoffice",
        ))
    }
}

#[cfg(feature = "restate-adapter")]
#[async_trait]
impl Handle<CreateRestateRetryInvoiceThroughLexOfficeV1> for Invoice {
    type Error = InvoiceError;

    async fn handle(
        &mut self,
        action: CreateRestateRetryInvoiceThroughLexOfficeV1,
        ctx: &mut ActionContext<Self>,
    ) -> Result<ActionDecision, HandlerError<Self::Error>> {
        if self.id.is_some() {
            return Err(HandlerError::domain(InvoiceError::AlreadyExists));
        }

        let result = ctx
            .execute_external_operation(
                &restate_retry_lexoffice_operation(),
                CreateLexOfficeInvoiceRequest {
                    invoice_id: action.invoice_id.clone(),
                    order_confirmation_id: action.order_confirmation_id,
                    customer_id: action.customer_id,
                    amount_cents: action.amount_cents,
                },
            )
            .await?;

        ctx.record_applied(
            self,
            InvoiceCreatedThroughLexOfficeV1 {
                invoice_id: action.invoice_id,
                provider_invoice_id: result.provider_invoice_id,
            },
        )?;

        Ok(ActionDecision::with_message(
            "Restate retry invoice created through lexoffice",
        ))
    }
}

#[cfg(feature = "restate-tests")]
#[async_trait]
impl Handle<CreateLiveRestateRetryInvoiceThroughLexOfficeV1> for Invoice {
    type Error = InvoiceError;

    async fn handle(
        &mut self,
        action: CreateLiveRestateRetryInvoiceThroughLexOfficeV1,
        ctx: &mut ActionContext<Self>,
    ) -> Result<ActionDecision, HandlerError<Self::Error>> {
        if self.id.is_some() {
            return Err(HandlerError::domain(InvoiceError::AlreadyExists));
        }

        let result = ctx
            .execute_external_operation(
                &live_restate_retry_lexoffice_operation(),
                CreateLexOfficeInvoiceRequest {
                    invoice_id: action.invoice_id.clone(),
                    order_confirmation_id: action.order_confirmation_id,
                    customer_id: action.customer_id,
                    amount_cents: action.amount_cents,
                },
            )
            .await?;

        ctx.record_applied(
            self,
            InvoiceCreatedThroughLexOfficeV1 {
                invoice_id: action.invoice_id,
                provider_invoice_id: result.provider_invoice_id,
            },
        )?;

        Ok(ActionDecision::with_message(
            "live Restate retry invoice created through lexoffice",
        ))
    }
}

#[derive(Clone)]
struct AppendFailsOnceEventStore {
    inner: InMemoryEventStore,
    attempts: Arc<Mutex<u32>>,
}

impl AppendFailsOnceEventStore {
    fn new(inner: InMemoryEventStore) -> Self {
        Self {
            inner,
            attempts: Arc::new(Mutex::new(0)),
        }
    }

    fn append_attempts(&self) -> u32 {
        *self.attempts.lock().expect("append attempts poisoned")
    }
}

#[async_trait]
impl EventStore for AppendFailsOnceEventStore {
    async fn load(&self, stream: &ResourceStream) -> Result<Vec<RecordedEvent>, EventStoreError> {
        self.inner.load(stream).await
    }

    async fn append(
        &self,
        stream: &ResourceStream,
        expected_version: ExpectedVersion,
        events: Vec<NewEvent>,
    ) -> Result<AppendResult, EventStoreError> {
        let should_fail = {
            let mut attempts = self.attempts.lock().expect("append attempts poisoned");
            *attempts += 1;
            *attempts == 1
        };

        if should_fail {
            return Err(EventStoreError::Other("append failed once".to_string()));
        }

        self.inner.append(stream, expected_version, events).await
    }
}

fn lexoffice_request(
    invoice_id: impl Into<String>,
    order_confirmation_id: impl Into<String>,
) -> CreateLexOfficeInvoiceRequest {
    CreateLexOfficeInvoiceRequest {
        invoice_id: invoice_id.into(),
        order_confirmation_id: order_confirmation_id.into(),
        customer_id: "customer-123".to_string(),
        amount_cents: 12_345,
    }
}

fn action_metadata(action_id: &str) -> ActionMetadata {
    ActionMetadata::with_ids(action_id, "correlation-123", "cause-123", "tester")
}

fn retry_lexoffice_operation() -> MockLexOfficeCreateInvoice {
    static OPERATION: OnceLock<MockLexOfficeCreateInvoice> = OnceLock::new();

    OPERATION
        .get_or_init(MockLexOfficeCreateInvoice::new)
        .clone()
}

#[cfg(feature = "restate-adapter")]
fn restate_retry_lexoffice_operation() -> MockLexOfficeCreateInvoice {
    static OPERATION: OnceLock<MockLexOfficeCreateInvoice> = OnceLock::new();

    OPERATION
        .get_or_init(MockLexOfficeCreateInvoice::new)
        .clone()
}

#[cfg(feature = "restate-tests")]
fn live_restate_retry_lexoffice_operation() -> MockLexOfficeCreateInvoice {
    static OPERATION: OnceLock<MockLexOfficeCreateInvoice> = OnceLock::new();

    OPERATION
        .get_or_init(MockLexOfficeCreateInvoice::new)
        .clone()
}

#[cfg(feature = "restate-tests")]
async fn live_restate_operation_journal() -> Option<(
    RestateOperationJournal,
    support::restate::RestateLiveEndpoint,
)> {
    let harness = match support::restate::RestateHarnessConfig::from_env() {
        Ok(harness) => harness,
        Err(skip) => {
            eprintln!("{}", skip.reason());
            return None;
        }
    };
    let endpoint = harness
        .start_operation_journal_endpoint()
        .await
        .expect("start and register live Restate OperationJournal endpoint");
    let journal = RestateOperationJournal::new(RestateOperationJournalConfig::new(harness.url()));

    Some((journal, endpoint))
}

#[cfg(feature = "restate-tests")]
fn unique_live_restate_suffix() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after UNIX_EPOCH")
        .as_nanos();

    nanos.to_string()
}

fn expected_operation_id(action_id: &str, operation_type: &str, idempotency_key: &str) -> String {
    format!(
        "action.{}.{}.operation.{}.{}.idempotency.{}.{}",
        action_id.len(),
        action_id,
        operation_type.len(),
        operation_type,
        idempotency_key.len(),
        idempotency_key
    )
}

fn assert_operation_called_record(
    record: &OperationJournalRecord,
    operation_id: &str,
    idempotency_key: &str,
) {
    match record {
        OperationJournalRecord::OperationCalled {
            operation_id: actual_operation_id,
            operation_type,
            operation_schema_id,
            operation_schema_version,
            idempotency_key: actual_idempotency_key,
            ..
        } => {
            assert_eq!(actual_operation_id, operation_id);
            assert_eq!(operation_type, MockLexOfficeCreateInvoice::OPERATION_TYPE);
            assert_eq!(operation_schema_id, MockLexOfficeCreateInvoice::SCHEMA_ID);
            assert_eq!(
                *operation_schema_version,
                MockLexOfficeCreateInvoice::SCHEMA_VERSION
            );
            assert_eq!(actual_idempotency_key, idempotency_key);
        }
        other => panic!("expected OperationCalled record, got {other:?}"),
    }
}

fn assert_operation_completed_record(record: &OperationJournalRecord, operation_id: &str) {
    match record {
        OperationJournalRecord::OperationCompleted {
            operation_id: actual_operation_id,
            response,
            ..
        } => {
            assert_eq!(actual_operation_id, operation_id);
            assert_eq!(response["provider_invoice_id"], "lexoffice-invoice-1");
        }
        other => panic!("expected OperationCompleted record, got {other:?}"),
    }
}

#[cfg(feature = "restate-adapter")]
struct FakeRestateOperationJournalServer {
    base_url: String,
    address: std::net::SocketAddr,
    shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

#[cfg(feature = "restate-adapter")]
impl FakeRestateOperationJournalServer {
    fn start() -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .expect("bind fake Restate OperationJournal server");
        let address = listener
            .local_addr()
            .expect("read fake Restate OperationJournal address");
        listener
            .set_nonblocking(true)
            .expect("set fake Restate OperationJournal listener nonblocking");
        let records = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::<
            String,
            Vec<OperationJournalRecord>,
        >::new()));
        let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let thread_records = records.clone();
        let thread_shutdown = shutdown.clone();
        let handle = std::thread::spawn(move || {
            while !thread_shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => fake_restate_handle_connection(stream, &thread_records),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    Err(error) => panic!("fake Restate OperationJournal accept failed: {error}"),
                }
            }
        });

        Self {
            base_url: format!("http://{address}"),
            address,
            shutdown,
            handle: Some(handle),
        }
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[cfg(feature = "restate-adapter")]
impl Drop for FakeRestateOperationJournalServer {
    fn drop(&mut self) {
        self.shutdown
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = std::net::TcpStream::connect(self.address);

        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .expect("join fake Restate OperationJournal server");
        }
    }
}

#[cfg(feature = "restate-adapter")]
struct FakeRestateRequest {
    method: String,
    path: String,
    body: Vec<u8>,
}

#[cfg(feature = "restate-adapter")]
fn fake_restate_handle_connection(
    mut stream: std::net::TcpStream,
    records: &std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<String, Vec<OperationJournalRecord>>>,
    >,
) {
    use std::io::Write;

    let response = match fake_restate_read_request(&mut stream) {
        Ok(request) => fake_restate_response_for(request, records),
        Err(error) => fake_restate_response(
            "500 Internal Server Error",
            format!("failed to read fake Restate request: {error}").into_bytes(),
        ),
    };

    stream
        .write_all(&response)
        .expect("write fake Restate response");
}

#[cfg(feature = "restate-adapter")]
fn fake_restate_read_request(
    stream: &mut std::net::TcpStream,
) -> Result<FakeRestateRequest, std::io::Error> {
    use std::io::{BufRead, Read};

    let mut reader = std::io::BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();
    let mut content_length = 0usize;

    loop {
        let mut header = String::new();
        reader.read_line(&mut header)?;
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }

        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap_or(0);
            }
        }
    }

    let mut body = vec![0; content_length];
    reader.read_exact(&mut body)?;

    Ok(FakeRestateRequest { method, path, body })
}

#[cfg(feature = "restate-adapter")]
fn fake_restate_response_for(
    request: FakeRestateRequest,
    records: &std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<String, Vec<OperationJournalRecord>>>,
    >,
) -> Vec<u8> {
    let path = request.path.split('?').next().unwrap_or(&request.path);
    let segments: Vec<_> = path.trim_start_matches('/').split('/').collect();
    if request.method != "POST" || segments.len() != 3 || segments[0] != "ElbmeshOperationJournal" {
        return fake_restate_response("404 Not Found", b"unknown fake Restate route".to_vec());
    }

    let operation_id = segments[1].to_string();
    match segments[2] {
        "append" => fake_restate_append(operation_id, request.body, records),
        "load" => fake_restate_load(operation_id, records),
        _ => fake_restate_response("404 Not Found", b"unknown fake Restate handler".to_vec()),
    }
}

#[cfg(feature = "restate-adapter")]
fn fake_restate_append(
    operation_id: String,
    body: Vec<u8>,
    records: &std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<String, Vec<OperationJournalRecord>>>,
    >,
) -> Vec<u8> {
    let record: OperationJournalRecord = match serde_json::from_slice(&body) {
        Ok(record) => record,
        Err(error) => {
            return fake_restate_response(
                "400 Bad Request",
                format!("invalid operation journal record: {error}").into_bytes(),
            )
        }
    };
    if fake_restate_record_operation_id(&record) != operation_id {
        return fake_restate_response(
            "409 Conflict",
            b"operation journal record targets another operation".to_vec(),
        );
    }

    records
        .lock()
        .expect("fake Restate OperationJournal records poisoned")
        .entry(operation_id)
        .or_default()
        .push(record);

    fake_restate_response("200 OK", Vec::new())
}

#[cfg(feature = "restate-adapter")]
fn fake_restate_load(
    operation_id: String,
    records: &std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<String, Vec<OperationJournalRecord>>>,
    >,
) -> Vec<u8> {
    let records = records
        .lock()
        .expect("fake Restate OperationJournal records poisoned")
        .get(&operation_id)
        .cloned()
        .unwrap_or_default();
    let body =
        serde_json::to_vec(&records).expect("serialize fake Restate OperationJournal records");

    fake_restate_response("200 OK", body)
}

#[cfg(feature = "restate-adapter")]
fn fake_restate_response(status: &str, body: Vec<u8>) -> Vec<u8> {
    let headers = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let mut response = headers.into_bytes();
    response.extend(body);
    response
}

#[cfg(feature = "restate-adapter")]
fn fake_restate_record_operation_id(record: &OperationJournalRecord) -> String {
    match record {
        OperationJournalRecord::OperationCalled { operation_id, .. }
        | OperationJournalRecord::OperationCompleted { operation_id, .. }
        | OperationJournalRecord::OperationFailed { operation_id, .. } => operation_id.clone(),
    }
}
