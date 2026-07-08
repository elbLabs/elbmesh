use std::fmt;

use async_trait::async_trait;
use elbmesh_core::{
    apply_recorded_event, Action, ActionContext, ActionDecision, ActionError, ActionExecutor,
    ActionFailure, ActionMetadata, ActionStatus, Apply, CreateLexOfficeInvoiceRequest, Event,
    EventStore, ExternalOperation, Handle, HandlerError, InMemoryEventStore,
    MockLexOfficeCreateInvoice, Resource, ResourceError, ResourceStream, StreamType,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

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
