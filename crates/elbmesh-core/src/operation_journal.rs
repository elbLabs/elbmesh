use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::MessageMetadata;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OperationJournalStream {
    pub operation_id: String,
}

impl OperationJournalStream {
    pub fn for_operation(operation_id: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OperationJournalRecord {
    OperationCalled {
        operation_id: String,
        metadata: MessageMetadata,
        operation_type: String,
        operation_schema_id: String,
        operation_schema_version: u32,
        idempotency_key: String,
        payload: Value,
    },
    OperationCompleted {
        operation_id: String,
        metadata: MessageMetadata,
        response: Value,
    },
    OperationFailed {
        operation_id: String,
        metadata: MessageMetadata,
        failure_code: String,
        failure_details: Value,
    },
}

impl OperationJournalRecord {
    fn operation_id(&self) -> &str {
        match self {
            Self::OperationCalled { operation_id, .. }
            | Self::OperationCompleted { operation_id, .. }
            | Self::OperationFailed { operation_id, .. } => operation_id,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum OperationJournalError {
    #[error("operation journal record targets operation '{actual_operation_id}', but stream is for operation '{expected_operation_id}'")]
    WrongOperationStream {
        expected_operation_id: String,
        actual_operation_id: String,
    },

    #[error("operation journal storage is poisoned")]
    StoragePoisoned,
}

#[async_trait]
pub trait OperationJournal: Send + Sync + 'static {
    async fn append(
        &self,
        stream: &OperationJournalStream,
        record: OperationJournalRecord,
    ) -> Result<(), OperationJournalError>;

    async fn load(
        &self,
        stream: &OperationJournalStream,
    ) -> Result<Vec<OperationJournalRecord>, OperationJournalError>;
}

#[derive(Clone, Default)]
pub struct InMemoryOperationJournal {
    records: Arc<Mutex<HashMap<OperationJournalStream, Vec<OperationJournalRecord>>>>,
}

impl InMemoryOperationJournal {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl OperationJournal for InMemoryOperationJournal {
    async fn append(
        &self,
        stream: &OperationJournalStream,
        record: OperationJournalRecord,
    ) -> Result<(), OperationJournalError> {
        let actual_operation_id = record.operation_id();
        if stream.operation_id.as_str() != actual_operation_id {
            return Err(OperationJournalError::WrongOperationStream {
                expected_operation_id: stream.operation_id.clone(),
                actual_operation_id: actual_operation_id.to_string(),
            });
        }

        let mut records = self
            .records
            .lock()
            .map_err(|_| OperationJournalError::StoragePoisoned)?;
        records.entry(stream.clone()).or_default().push(record);

        Ok(())
    }

    async fn load(
        &self,
        stream: &OperationJournalStream,
    ) -> Result<Vec<OperationJournalRecord>, OperationJournalError> {
        let records = self
            .records
            .lock()
            .map_err(|_| OperationJournalError::StoragePoisoned)?;

        Ok(records.get(stream).cloned().unwrap_or_default())
    }
}
