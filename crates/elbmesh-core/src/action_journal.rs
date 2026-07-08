use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[cfg(feature = "nats-adapter")]
use futures_util::StreamExt;

use crate::{ActionReceipt, MessageMetadata};

#[cfg(feature = "nats-adapter")]
const DEFAULT_NATS_ACTION_JOURNAL_BUCKET: &str = "elbmesh_action_journal";
#[cfg(feature = "nats-adapter")]
const DEFAULT_NATS_ACTION_JOURNAL_HISTORY: i64 = 64;
#[cfg(feature = "nats-adapter")]
const HEX: &[u8; 16] = b"0123456789ABCDEF";

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

    #[error("failed to connect NATS ActionJournal: {reason}")]
    NatsConnect { reason: String },

    #[error("failed to open NATS ActionJournal bucket '{bucket}': {reason}")]
    NatsBucket { bucket: String, reason: String },

    #[error("failed to serialize action journal record: {reason}")]
    RecordSerialization { reason: String },

    #[error("failed to deserialize action journal record from stream '{stream}' revision {revision}: {reason}")]
    RecordDeserialization {
        stream: String,
        revision: u64,
        reason: String,
    },

    #[error("failed to append action journal record to NATS stream '{stream}': {reason}")]
    NatsAppend { stream: String, reason: String },

    #[error("failed to load action journal records from NATS stream '{stream}': {reason}")]
    NatsLoad { stream: String, reason: String },
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

#[cfg(feature = "nats-adapter")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NatsActionJournalConfig {
    bucket: String,
    history: i64,
}

#[cfg(feature = "nats-adapter")]
impl NatsActionJournalConfig {
    pub fn new(bucket: impl Into<String>) -> Self {
        Self {
            bucket: bucket.into(),
            ..Self::default()
        }
    }

    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    pub fn history(&self) -> i64 {
        self.history
    }

    pub fn with_history(mut self, history: i64) -> Self {
        self.history = history;
        self
    }
}

#[cfg(feature = "nats-adapter")]
impl Default for NatsActionJournalConfig {
    fn default() -> Self {
        Self {
            bucket: DEFAULT_NATS_ACTION_JOURNAL_BUCKET.to_string(),
            history: DEFAULT_NATS_ACTION_JOURNAL_HISTORY,
        }
    }
}

#[cfg(feature = "nats-adapter")]
#[derive(Clone)]
pub struct NatsActionJournal {
    store: async_nats::jetstream::kv::Store,
}

#[cfg(feature = "nats-adapter")]
impl NatsActionJournal {
    pub async fn connect(
        url: impl AsRef<str>,
        config: NatsActionJournalConfig,
    ) -> Result<Self, ActionJournalError> {
        let client = async_nats::connect(url.as_ref()).await.map_err(|source| {
            ActionJournalError::NatsConnect {
                reason: source.to_string(),
            }
        })?;

        Self::from_client(client, config).await
    }

    pub async fn from_client(
        client: async_nats::Client,
        config: NatsActionJournalConfig,
    ) -> Result<Self, ActionJournalError> {
        let jetstream = async_nats::jetstream::new(client);

        Self::from_jetstream(jetstream, config).await
    }

    pub async fn from_jetstream(
        jetstream: async_nats::jetstream::Context,
        config: NatsActionJournalConfig,
    ) -> Result<Self, ActionJournalError> {
        let bucket = config.bucket.clone();
        let store = jetstream
            .create_or_update_key_value(async_nats::jetstream::kv::Config {
                bucket: config.bucket,
                history: config.history,
                ..Default::default()
            })
            .await
            .map_err(|source| ActionJournalError::NatsBucket {
                bucket,
                reason: source.to_string(),
            })?;

        Ok(Self { store })
    }

    pub fn from_store(store: async_nats::jetstream::kv::Store) -> Self {
        Self { store }
    }
}

#[cfg(feature = "nats-adapter")]
#[async_trait]
impl ActionJournal for NatsActionJournal {
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

        let key = nats_action_journal_key(stream);
        let value = serde_json::to_vec(&record).map_err(|source| {
            ActionJournalError::RecordSerialization {
                reason: source.to_string(),
            }
        })?;

        self.store
            .put(key.as_str(), value.into())
            .await
            .map_err(|source| ActionJournalError::NatsAppend {
                stream: stream.action_id.clone(),
                reason: source.to_string(),
            })?;

        Ok(())
    }

    async fn load(
        &self,
        stream: &ActionJournalStream,
    ) -> Result<Vec<ActionJournalRecord>, ActionJournalError> {
        let key = nats_action_journal_key(stream);
        let mut history = self.store.history(key.as_str()).await.map_err(|source| {
            ActionJournalError::NatsLoad {
                stream: stream.action_id.clone(),
                reason: source.to_string(),
            }
        })?;
        let mut records = Vec::new();

        while let Some(entry) = history.next().await {
            let entry = entry.map_err(|source| ActionJournalError::NatsLoad {
                stream: stream.action_id.clone(),
                reason: source.to_string(),
            })?;

            if entry.operation != async_nats::jetstream::kv::Operation::Put {
                continue;
            }

            let record: ActionJournalRecord =
                serde_json::from_slice(&entry.value).map_err(|source| {
                    ActionJournalError::RecordDeserialization {
                        stream: stream.action_id.clone(),
                        revision: entry.revision,
                        reason: source.to_string(),
                    }
                })?;
            let actual_action_id = record.action_id();
            if stream.action_id.as_str() != actual_action_id {
                return Err(ActionJournalError::WrongActionStream {
                    expected_action_id: stream.action_id.clone(),
                    actual_action_id: actual_action_id.to_string(),
                });
            }

            records.push((entry.revision, record));
        }

        records.sort_by_key(|(revision, _)| *revision);

        Ok(records.into_iter().map(|(_, record)| record).collect())
    }
}

#[cfg(feature = "nats-adapter")]
fn nats_action_journal_key(stream: &ActionJournalStream) -> String {
    format!(
        "action.{}.{}",
        stream.action_id.len(),
        encode_nats_key_token(&stream.action_id)
    )
}

#[cfg(feature = "nats-adapter")]
fn encode_nats_key_token(value: &str) -> String {
    if value.is_empty() {
        return "_".to_string();
    }

    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' => encoded.push(byte as char),
            _ => {
                encoded.push('%');
                encoded.push(HEX[(byte >> 4) as usize] as char);
                encoded.push(HEX[(byte & 0x0f) as usize] as char);
            }
        }
    }

    encoded
}

#[cfg(all(test, feature = "nats-adapter"))]
mod nats_tests {
    use super::*;

    #[test]
    fn nats_action_journal_key_leaves_plain_action_ids_readable() {
        let stream = ActionJournalStream::for_action("action-123");

        assert_eq!(nats_action_journal_key(&stream), "action.10.action-123");
    }

    #[test]
    fn nats_action_journal_key_escapes_key_token_separators_and_wildcards() {
        let stream = ActionJournalStream::for_action("tenant.1/action*>");
        let key = nats_action_journal_key(&stream);
        let tokens: Vec<_> = key.split('.').collect();

        assert_eq!(tokens, vec!["action", "17", "tenant%2E1%2Faction%2A%3E"]);
        assert!(!tokens[2].contains('*'));
        assert!(!tokens[2].contains('>'));
    }

    #[test]
    fn nats_action_journal_key_distinguishes_empty_action_ids() {
        let empty_stream = ActionJournalStream::for_action("");
        let underscore_stream = ActionJournalStream::for_action("_");

        assert_eq!(nats_action_journal_key(&empty_stream), "action.0._");
        assert_eq!(nats_action_journal_key(&underscore_stream), "action.1._");
    }
}
