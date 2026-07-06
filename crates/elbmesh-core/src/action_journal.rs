use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::{ActionReceipt, MessageMetadata};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionFailureClassification {
    EventStore,
    Resource,
    HandlerRuntime,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActionJournalStream {
    pub action_id: String,
}

impl ActionJournalStream {
    pub fn for_action(action_id: impl Into<String>) -> Self {
        Self {
            action_id: action_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActionJournalRecord {
    ActionCalled {
        metadata: MessageMetadata,
        action_type: String,
        action_schema_id: String,
        action_schema_version: u32,
        payload: Value,
    },
    ActionRejected {
        metadata: MessageMetadata,
        failure_code: String,
        failure_details: Value,
    },
    ActionFailed {
        metadata: MessageMetadata,
        failure_classification: ActionFailureClassification,
        failure_details: Value,
    },
    ActionCompleted {
        metadata: MessageMetadata,
        receipt: ActionReceipt,
    },
}

impl ActionJournalRecord {
    fn action_id(&self) -> &str {
        match self {
            Self::ActionCalled { metadata, .. }
            | Self::ActionRejected { metadata, .. }
            | Self::ActionFailed { metadata, .. }
            | Self::ActionCompleted { metadata, .. } => &metadata.action_id,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ActionJournalError {
    #[error("action journal record targets action '{actual_action_id}', but stream is for action '{expected_action_id}'")]
    WrongActionStream {
        expected_action_id: String,
        actual_action_id: String,
    },

    #[error("action journal storage is poisoned")]
    StoragePoisoned,
}

#[async_trait]
pub trait ActionJournal: Send + Sync + 'static {
    async fn append(
        &self,
        stream: &ActionJournalStream,
        record: ActionJournalRecord,
    ) -> Result<(), ActionJournalError>;

    async fn load(
        &self,
        stream: &ActionJournalStream,
    ) -> Result<Vec<ActionJournalRecord>, ActionJournalError>;
}

#[derive(Clone, Default)]
pub struct InMemoryActionJournal {
    records: Arc<Mutex<HashMap<ActionJournalStream, Vec<ActionJournalRecord>>>>,
}

impl InMemoryActionJournal {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ActionJournal for InMemoryActionJournal {
    async fn append(
        &self,
        stream: &ActionJournalStream,
        record: ActionJournalRecord,
    ) -> Result<(), ActionJournalError> {
        let actual_action_id = record.action_id();
        if stream.action_id.as_str() != actual_action_id {
            return Err(ActionJournalError::WrongActionStream {
                expected_action_id: stream.action_id.clone(),
                actual_action_id: actual_action_id.to_string(),
            });
        }

        let mut records = self
            .records
            .lock()
            .map_err(|_| ActionJournalError::StoragePoisoned)?;
        records.entry(stream.clone()).or_default().push(record);

        Ok(())
    }

    async fn load(
        &self,
        stream: &ActionJournalStream,
    ) -> Result<Vec<ActionJournalRecord>, ActionJournalError> {
        let records = self
            .records
            .lock()
            .map_err(|_| ActionJournalError::StoragePoisoned)?;

        Ok(records.get(stream).cloned().unwrap_or_default())
    }
}
