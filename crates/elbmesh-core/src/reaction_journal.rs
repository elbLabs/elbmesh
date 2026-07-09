use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "nats-adapter")]
use futures_util::StreamExt;

use crate::MessageMetadata;

#[cfg(feature = "nats-adapter")]
const DEFAULT_NATS_REACTION_JOURNAL_BUCKET: &str = "elbmesh_reaction_journal";
#[cfg(feature = "nats-adapter")]
const DEFAULT_NATS_REACTION_JOURNAL_HISTORY: i64 = 64;
#[cfg(feature = "nats-adapter")]
const HEX: &[u8; 16] = b"0123456789ABCDEF";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReactionJournalStream {
    pub reaction_id: String,
}

impl ReactionJournalStream {
    pub fn for_reaction(reaction_id: impl Into<String>) -> Self {
        Self {
            reaction_id: reaction_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReactionJournalRecord {
    ReactionTriggered {
        reaction_id: String,
        metadata: MessageMetadata,
        reaction_type: String,
        reaction_schema_id: String,
        reaction_schema_version: u32,
        trigger_event_type: String,
        trigger_event_id: String,
    },
    ReactionCompleted {
        reaction_id: String,
        metadata: MessageMetadata,
        triggered_action_id: String,
    },
}

impl ReactionJournalRecord {
    fn reaction_id(&self) -> &str {
        match self {
            Self::ReactionTriggered { reaction_id, .. }
            | Self::ReactionCompleted { reaction_id, .. } => reaction_id,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ReactionJournalError {
    #[error("reaction journal record targets reaction '{actual_reaction_id}', but stream is for reaction '{expected_reaction_id}'")]
    WrongReactionStream {
        expected_reaction_id: String,
        actual_reaction_id: String,
    },

    #[error("reaction journal storage is poisoned")]
    StoragePoisoned,

    #[error("failed to connect NATS ReactionJournal: {reason}")]
    NatsConnect { reason: String },

    #[error("failed to open NATS ReactionJournal bucket '{bucket}': {reason}")]
    NatsBucket { bucket: String, reason: String },

    #[error("failed to serialize reaction journal record: {reason}")]
    RecordSerialization { reason: String },

    #[error("failed to deserialize reaction journal record from stream '{stream}' revision {revision}: {reason}")]
    RecordDeserialization {
        stream: String,
        revision: u64,
        reason: String,
    },

    #[error("failed to append reaction journal record to NATS stream '{stream}': {reason}")]
    NatsAppend { stream: String, reason: String },

    #[error("failed to load reaction journal records from NATS stream '{stream}': {reason}")]
    NatsLoad { stream: String, reason: String },
}

#[async_trait]
pub trait ReactionJournal: Send + Sync + 'static {
    async fn append(
        &self,
        stream: &ReactionJournalStream,
        record: ReactionJournalRecord,
    ) -> Result<(), ReactionJournalError>;

    async fn load(
        &self,
        stream: &ReactionJournalStream,
    ) -> Result<Vec<ReactionJournalRecord>, ReactionJournalError>;
}

#[derive(Clone, Default)]
pub struct InMemoryReactionJournal {
    records: Arc<Mutex<HashMap<ReactionJournalStream, Vec<ReactionJournalRecord>>>>,
}

impl InMemoryReactionJournal {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ReactionJournal for InMemoryReactionJournal {
    async fn append(
        &self,
        stream: &ReactionJournalStream,
        record: ReactionJournalRecord,
    ) -> Result<(), ReactionJournalError> {
        let actual_reaction_id = record.reaction_id();
        if stream.reaction_id.as_str() != actual_reaction_id {
            return Err(ReactionJournalError::WrongReactionStream {
                expected_reaction_id: stream.reaction_id.clone(),
                actual_reaction_id: actual_reaction_id.to_string(),
            });
        }

        let mut records = self
            .records
            .lock()
            .map_err(|_| ReactionJournalError::StoragePoisoned)?;
        records.entry(stream.clone()).or_default().push(record);

        Ok(())
    }

    async fn load(
        &self,
        stream: &ReactionJournalStream,
    ) -> Result<Vec<ReactionJournalRecord>, ReactionJournalError> {
        let records = self
            .records
            .lock()
            .map_err(|_| ReactionJournalError::StoragePoisoned)?;

        Ok(records.get(stream).cloned().unwrap_or_default())
    }
}

#[cfg(feature = "nats-adapter")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NatsReactionJournalConfig {
    bucket: String,
    history: i64,
}

#[cfg(feature = "nats-adapter")]
impl NatsReactionJournalConfig {
    pub fn new(bucket: impl Into<String>) -> Self {
        Self {
            bucket: bucket.into(),
            history: DEFAULT_NATS_REACTION_JOURNAL_HISTORY,
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
impl Default for NatsReactionJournalConfig {
    fn default() -> Self {
        Self {
            bucket: DEFAULT_NATS_REACTION_JOURNAL_BUCKET.to_string(),
            history: DEFAULT_NATS_REACTION_JOURNAL_HISTORY,
        }
    }
}

#[cfg(feature = "nats-adapter")]
#[derive(Clone)]
pub struct NatsReactionJournal {
    store: async_nats::jetstream::kv::Store,
}

#[cfg(feature = "nats-adapter")]
impl NatsReactionJournal {
    pub async fn connect(
        url: impl AsRef<str>,
        config: NatsReactionJournalConfig,
    ) -> Result<Self, ReactionJournalError> {
        let client = async_nats::connect(url.as_ref()).await.map_err(|source| {
            ReactionJournalError::NatsConnect {
                reason: source.to_string(),
            }
        })?;

        Self::from_client(client, config).await
    }

    pub async fn from_client(
        client: async_nats::Client,
        config: NatsReactionJournalConfig,
    ) -> Result<Self, ReactionJournalError> {
        let jetstream = async_nats::jetstream::new(client);

        Self::from_jetstream(jetstream, config).await
    }

    pub async fn from_jetstream(
        jetstream: async_nats::jetstream::Context,
        config: NatsReactionJournalConfig,
    ) -> Result<Self, ReactionJournalError> {
        let bucket = config.bucket.clone();
        let store = jetstream
            .create_or_update_key_value(async_nats::jetstream::kv::Config {
                bucket: config.bucket,
                history: config.history,
                ..Default::default()
            })
            .await
            .map_err(|source| ReactionJournalError::NatsBucket {
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
impl ReactionJournal for NatsReactionJournal {
    async fn append(
        &self,
        stream: &ReactionJournalStream,
        record: ReactionJournalRecord,
    ) -> Result<(), ReactionJournalError> {
        let actual_reaction_id = record.reaction_id();
        if stream.reaction_id.as_str() != actual_reaction_id {
            return Err(ReactionJournalError::WrongReactionStream {
                expected_reaction_id: stream.reaction_id.clone(),
                actual_reaction_id: actual_reaction_id.to_string(),
            });
        }

        let key = nats_reaction_journal_key(stream);
        let value = serde_json::to_vec(&record).map_err(|source| {
            ReactionJournalError::RecordSerialization {
                reason: source.to_string(),
            }
        })?;

        self.store
            .put(key.as_str(), value.into())
            .await
            .map_err(|source| ReactionJournalError::NatsAppend {
                stream: stream.reaction_id.clone(),
                reason: source.to_string(),
            })?;

        Ok(())
    }

    async fn load(
        &self,
        stream: &ReactionJournalStream,
    ) -> Result<Vec<ReactionJournalRecord>, ReactionJournalError> {
        let key = nats_reaction_journal_key(stream);
        let mut history = self.store.history(key.as_str()).await.map_err(|source| {
            ReactionJournalError::NatsLoad {
                stream: stream.reaction_id.clone(),
                reason: source.to_string(),
            }
        })?;
        let mut records = Vec::new();

        while let Some(entry) = history.next().await {
            let entry = entry.map_err(|source| ReactionJournalError::NatsLoad {
                stream: stream.reaction_id.clone(),
                reason: source.to_string(),
            })?;

            if entry.operation != async_nats::jetstream::kv::Operation::Put {
                continue;
            }

            let record: ReactionJournalRecord =
                serde_json::from_slice(&entry.value).map_err(|source| {
                    ReactionJournalError::RecordDeserialization {
                        stream: stream.reaction_id.clone(),
                        revision: entry.revision,
                        reason: source.to_string(),
                    }
                })?;
            let actual_reaction_id = record.reaction_id();
            if stream.reaction_id.as_str() != actual_reaction_id {
                return Err(ReactionJournalError::WrongReactionStream {
                    expected_reaction_id: stream.reaction_id.clone(),
                    actual_reaction_id: actual_reaction_id.to_string(),
                });
            }

            records.push((entry.revision, record));
        }

        records.sort_by_key(|(revision, _)| *revision);

        Ok(records.into_iter().map(|(_, record)| record).collect())
    }
}

#[cfg(feature = "nats-adapter")]
fn nats_reaction_journal_key(stream: &ReactionJournalStream) -> String {
    format!(
        "reaction.{}.{}",
        stream.reaction_id.len(),
        encode_nats_key_token(&stream.reaction_id)
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
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-' => encoded.push(byte as char),
            _ => {
                encoded.push('%');
                encoded.push(HEX[(byte >> 4) as usize] as char);
                encoded.push(HEX[(byte & 0x0F) as usize] as char);
            }
        }
    }

    encoded
}

#[cfg(all(test, feature = "nats-adapter"))]
mod nats_tests {
    use super::*;

    #[test]
    fn nats_reaction_journal_key_leaves_plain_reaction_ids_readable() {
        let stream = ReactionJournalStream::for_reaction("reaction-123");

        assert_eq!(
            nats_reaction_journal_key(&stream),
            "reaction.12.reaction-123"
        );
    }

    #[test]
    fn nats_reaction_journal_key_escapes_key_token_separators_and_wildcards() {
        let stream = ReactionJournalStream::for_reaction("tenant.1/reaction*>");
        let key = nats_reaction_journal_key(&stream);
        let tokens: Vec<_> = key.split('.').collect();

        assert_eq!(
            tokens,
            vec!["reaction", "19", "tenant%2E1%2Freaction%2A%3E"]
        );
        assert!(!tokens[2].contains('*'));
        assert!(!tokens[2].contains('>'));
    }

    #[test]
    fn nats_reaction_journal_key_distinguishes_empty_reaction_ids() {
        let empty_stream = ReactionJournalStream::for_reaction("");
        let underscore_stream = ReactionJournalStream::for_reaction("_");

        assert_eq!(nats_reaction_journal_key(&empty_stream), "reaction.0._");
        assert_eq!(
            nats_reaction_journal_key(&underscore_stream),
            "reaction.1._"
        );
    }
}
