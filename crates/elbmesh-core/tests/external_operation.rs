use elbmesh_core::{
    CreateLexOfficeInvoiceRequest, ExternalOperation, ExternalOperationFailure,
    LexOfficeCreateInvoiceError, MockLexOfficeCreateInvoice,
};

use serde_json::json;

#[test]
fn mock_lexoffice_create_invoice_implements_external_operation_trait() {
    fn assert_external_operation<O: ExternalOperation>() {}

    assert_external_operation::<MockLexOfficeCreateInvoice>();
}

#[test]
fn lexoffice_create_invoice_declares_stable_operation_schema_identity() {
    assert_eq!(
        MockLexOfficeCreateInvoice::OPERATION_TYPE,
        "lexoffice_create_invoice"
    );
    assert_eq!(
        MockLexOfficeCreateInvoice::SCHEMA_ID,
        "external_operation.lexoffice_create_invoice.v1"
    );
    assert_eq!(MockLexOfficeCreateInvoice::SCHEMA_VERSION, 1);
}

#[test]
fn lexoffice_create_invoice_uses_deterministic_idempotency_key() {
    let operation = MockLexOfficeCreateInvoice::new();
    let request = create_invoice_request("invoice-123", "order-confirmation-123");

    let key = operation.idempotency_key(&request);
    let same_key = operation.idempotency_key(&request);
    let different_key = operation.idempotency_key(&create_invoice_request(
        "invoice-456",
        "order-confirmation-123",
    ));

    assert_eq!(key, same_key);
    assert_ne!(key, different_key);
    assert_eq!(
        key,
        "lexoffice_create_invoice.v1.invoice.11.invoice-123.order_confirmation.22.order-confirmation-123"
    );
}

#[tokio::test]
async fn mock_lexoffice_create_invoice_returns_original_result_for_idempotent_retry() {
    let operation = MockLexOfficeCreateInvoice::new();
    let request = create_invoice_request("invoice-123", "order-confirmation-123");
    let idempotency_key = operation.idempotency_key(&request);

    let first = operation
        .execute(request.clone(), idempotency_key.clone())
        .await
        .expect("first invoice create succeeds");
    let retry = operation
        .execute(request, idempotency_key.clone())
        .await
        .expect("idempotent invoice create retry succeeds");

    assert_eq!(retry, first);
    assert_eq!(first.idempotency_key, idempotency_key);
    assert_eq!(first.provider_invoice_id, "lexoffice-invoice-1");
    assert_eq!(
        operation.created_invoice_count().expect("count invoices"),
        1
    );
}

#[tokio::test]
async fn mock_lexoffice_create_invoice_rejects_idempotency_key_conflict_with_named_error() {
    let operation = MockLexOfficeCreateInvoice::new();
    let request = create_invoice_request("invoice-123", "order-confirmation-123");
    let idempotency_key = operation.idempotency_key(&request);

    operation
        .execute(request, idempotency_key.clone())
        .await
        .expect("first invoice create succeeds");

    let err = operation
        .execute(
            create_invoice_request("invoice-456", "order-confirmation-123"),
            idempotency_key.clone(),
        )
        .await
        .expect_err("conflicting idempotency key should fail");

    assert_eq!(
        err,
        LexOfficeCreateInvoiceError::IdempotencyKeyConflict { idempotency_key }
    );
    assert_eq!(
        err.code(),
        "lexoffice.create_invoice.idempotency_key_conflict"
    );
    assert_eq!(
        err.details(),
        json!({
            "error_type": "LexOfficeCreateInvoiceError",
            "error_variant": "IdempotencyKeyConflict",
        })
    );
    assert_eq!(
        operation.created_invoice_count().expect("count invoices"),
        1
    );
}

#[tokio::test]
async fn mock_lexoffice_create_invoice_surfaces_provider_failure_as_named_error() {
    let operation = MockLexOfficeCreateInvoice::new();
    let request = create_invoice_request("invoice-123", "order-confirmation-123");
    let idempotency_key = operation.idempotency_key(&request);

    operation
        .fail_next_create()
        .expect("mark next invoice create as failed");

    let err = operation
        .execute(request, idempotency_key)
        .await
        .expect_err("provider failure should fail operation");

    assert_eq!(err, LexOfficeCreateInvoiceError::ProviderUnavailable);
    assert_eq!(err.code(), "lexoffice.create_invoice.provider_unavailable");
    assert_eq!(
        err.details(),
        json!({
            "error_type": "LexOfficeCreateInvoiceError",
            "error_variant": "ProviderUnavailable",
        })
    );
    assert_eq!(
        operation.created_invoice_count().expect("count invoices"),
        0
    );
}

fn create_invoice_request(
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
