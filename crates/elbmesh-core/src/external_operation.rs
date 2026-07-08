use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

pub trait ExternalOperationFailure: Debug + Display + Send + Sync + 'static {
    fn code(&self) -> &'static str;

    fn details(&self) -> Value {
        json!({ "code": self.code() })
    }
}

#[async_trait]
pub trait ExternalOperation: Send + Sync + 'static {
    type Request: Clone + Serialize + DeserializeOwned + Send + Sync + 'static;
    type Response: Clone + Serialize + DeserializeOwned + Send + Sync + 'static;
    type Error: ExternalOperationFailure;

    const OPERATION_TYPE: &'static str;
    const SCHEMA_ID: &'static str;
    const SCHEMA_VERSION: u32;

    fn idempotency_key(&self, request: &Self::Request) -> String;

    async fn execute(
        &self,
        request: Self::Request,
        idempotency_key: String,
    ) -> Result<Self::Response, Self::Error>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateLexOfficeInvoiceRequest {
    pub invoice_id: String,
    pub order_confirmation_id: String,
    pub customer_id: String,
    pub amount_cents: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexOfficeCreateInvoiceResult {
    pub invoice_id: String,
    pub order_confirmation_id: String,
    pub provider_invoice_id: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LexOfficeCreateInvoiceError {
    #[error("mock LexOffice create invoice provider is unavailable")]
    ProviderUnavailable,

    #[error("LexOffice create invoice idempotency key '{idempotency_key}' was reused with a different request")]
    IdempotencyKeyConflict { idempotency_key: String },

    #[error("mock LexOffice create invoice storage is poisoned")]
    StoragePoisoned,
}

impl ExternalOperationFailure for LexOfficeCreateInvoiceError {
    fn code(&self) -> &'static str {
        match self {
            Self::ProviderUnavailable => "lexoffice.create_invoice.provider_unavailable",
            Self::IdempotencyKeyConflict { .. } => {
                "lexoffice.create_invoice.idempotency_key_conflict"
            }
            Self::StoragePoisoned => "lexoffice.create_invoice.storage_poisoned",
        }
    }

    fn details(&self) -> Value {
        json!({
            "error_type": "LexOfficeCreateInvoiceError",
            "error_variant": match self {
                Self::ProviderUnavailable => "ProviderUnavailable",
                Self::IdempotencyKeyConflict { .. } => "IdempotencyKeyConflict",
                Self::StoragePoisoned => "StoragePoisoned",
            },
        })
    }
}

#[derive(Clone, Default)]
pub struct MockLexOfficeCreateInvoice {
    state: Arc<Mutex<MockLexOfficeCreateInvoiceState>>,
}

impl MockLexOfficeCreateInvoice {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn created_invoice_count(&self) -> Result<usize, LexOfficeCreateInvoiceError> {
        let state = self.lock_state()?;

        Ok(state.invoices_by_idempotency_key.len())
    }

    pub fn fail_next_create(&self) -> Result<(), LexOfficeCreateInvoiceError> {
        let mut state = self.lock_state()?;
        state.fail_next_create = true;

        Ok(())
    }

    fn lock_state(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, MockLexOfficeCreateInvoiceState>,
        LexOfficeCreateInvoiceError,
    > {
        self.state
            .lock()
            .map_err(|_| LexOfficeCreateInvoiceError::StoragePoisoned)
    }
}

#[async_trait]
impl ExternalOperation for MockLexOfficeCreateInvoice {
    type Request = CreateLexOfficeInvoiceRequest;
    type Response = LexOfficeCreateInvoiceResult;
    type Error = LexOfficeCreateInvoiceError;

    const OPERATION_TYPE: &'static str = "lexoffice_create_invoice";
    const SCHEMA_ID: &'static str = "external_operation.lexoffice_create_invoice.v1";
    const SCHEMA_VERSION: u32 = 1;

    fn idempotency_key(&self, request: &Self::Request) -> String {
        create_lexoffice_invoice_idempotency_key(request)
    }

    async fn execute(
        &self,
        request: Self::Request,
        idempotency_key: String,
    ) -> Result<Self::Response, Self::Error> {
        let mut state = self.lock_state()?;

        if let Some(stored_invoice) = state.invoices_by_idempotency_key.get(&idempotency_key) {
            if stored_invoice.request != request {
                return Err(LexOfficeCreateInvoiceError::IdempotencyKeyConflict {
                    idempotency_key,
                });
            }

            return Ok(stored_invoice.result.clone());
        }

        if state.fail_next_create {
            state.fail_next_create = false;

            return Err(LexOfficeCreateInvoiceError::ProviderUnavailable);
        }

        state.next_invoice_number += 1;
        let result = LexOfficeCreateInvoiceResult {
            invoice_id: request.invoice_id.clone(),
            order_confirmation_id: request.order_confirmation_id.clone(),
            provider_invoice_id: format!("lexoffice-invoice-{}", state.next_invoice_number),
            idempotency_key: idempotency_key.clone(),
        };

        state.invoices_by_idempotency_key.insert(
            idempotency_key,
            StoredLexOfficeInvoice {
                request,
                result: result.clone(),
            },
        );

        Ok(result)
    }
}

#[derive(Default)]
struct MockLexOfficeCreateInvoiceState {
    invoices_by_idempotency_key: HashMap<String, StoredLexOfficeInvoice>,
    next_invoice_number: u64,
    fail_next_create: bool,
}

struct StoredLexOfficeInvoice {
    request: CreateLexOfficeInvoiceRequest,
    result: LexOfficeCreateInvoiceResult,
}

fn create_lexoffice_invoice_idempotency_key(request: &CreateLexOfficeInvoiceRequest) -> String {
    format!(
        "lexoffice_create_invoice.v1.invoice.{}.{}.order_confirmation.{}.{}",
        request.invoice_id.len(),
        request.invoice_id,
        request.order_confirmation_id.len(),
        request.order_confirmation_id
    )
}
