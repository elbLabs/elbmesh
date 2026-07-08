use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[cfg(feature = "nats-adapter")]
use futures_util::StreamExt;

#[cfg(feature = "nats-adapter")]
const DEFAULT_NATS_VIEW_STORE_BUCKET: &str = "elbmesh_view_store";
#[cfg(feature = "nats-adapter")]
const HEX: &[u8; 16] = b"0123456789ABCDEF";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ViewKey {
    pub view_type: String,
    pub view_id: String,
}

impl ViewKey {
    pub fn new(view_type: impl Into<String>, view_id: impl Into<String>) -> Self {
        Self {
            view_type: view_type.into(),
            view_id: view_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewDocument {
    pub key: ViewKey,
    pub payload: Value,
    pub indexes: Vec<ViewIndexEntry>,
}

impl ViewDocument {
    pub fn new(view_type: impl Into<String>, view_id: impl Into<String>, payload: Value) -> Self {
        Self {
            key: ViewKey::new(view_type, view_id),
            payload,
            indexes: Vec::new(),
        }
    }

    pub fn with_indexes(mut self, indexes: Vec<ViewIndexEntry>) -> Self {
        self.indexes = indexes;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ViewIndexEntry {
    pub index_name: String,
    pub index_key: String,
}

impl ViewIndexEntry {
    pub fn new(index_name: impl Into<String>, index_key: impl Into<String>) -> Self {
        Self {
            index_name: index_name.into(),
            index_key: index_key.into(),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ViewStoreError {
    #[error("view store storage is poisoned")]
    StoragePoisoned,

    #[error("view document '{view_type}/{view_id}' declares index '{index_name}' more than once")]
    DuplicateIndexName {
        view_type: String,
        view_id: String,
        index_name: String,
    },

    #[error("failed to connect NATS ViewStore: {reason}")]
    NatsConnect { reason: String },

    #[error("failed to open NATS ViewStore bucket '{bucket}': {reason}")]
    NatsBucket { bucket: String, reason: String },

    #[error("failed to serialize view document '{view_type}/{view_id}': {reason}")]
    DocumentSerialization {
        view_type: String,
        view_id: String,
        reason: String,
    },

    #[error(
        "failed to deserialize view document from NATS key '{key}' revision {revision}: {reason}"
    )]
    DocumentDeserialization {
        key: String,
        revision: u64,
        reason: String,
    },

    #[error("failed to put view document '{view_type}/{view_id}' in NATS: {reason}")]
    NatsPut {
        view_type: String,
        view_id: String,
        reason: String,
    },

    #[error("failed to load view document '{view_type}/{view_id}' from NATS: {reason}")]
    NatsLoad {
        view_type: String,
        view_id: String,
        reason: String,
    },

    #[error("failed to list view index '{view_type}/{index_name}' with prefix '{prefix}' from NATS: {reason}")]
    NatsList {
        view_type: String,
        index_name: String,
        prefix: String,
        reason: String,
    },
}

#[async_trait]
pub trait ViewStore: Send + Sync + 'static {
    async fn put(&self, document: ViewDocument) -> Result<(), ViewStoreError>;

    async fn load(&self, key: &ViewKey) -> Result<Option<ViewDocument>, ViewStoreError>;

    async fn list_by_index_prefix(
        &self,
        view_type: &str,
        index_name: &str,
        prefix: &str,
    ) -> Result<Vec<ViewDocument>, ViewStoreError>;
}

#[derive(Clone, Default)]
pub struct InMemoryViewStore {
    documents: Arc<Mutex<HashMap<ViewKey, ViewDocument>>>,
}

impl InMemoryViewStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ViewStore for InMemoryViewStore {
    async fn put(&self, document: ViewDocument) -> Result<(), ViewStoreError> {
        validate_unique_index_names(&document)?;

        let mut documents = self
            .documents
            .lock()
            .map_err(|_| ViewStoreError::StoragePoisoned)?;

        documents.insert(document.key.clone(), document);
        Ok(())
    }

    async fn load(&self, key: &ViewKey) -> Result<Option<ViewDocument>, ViewStoreError> {
        let documents = self
            .documents
            .lock()
            .map_err(|_| ViewStoreError::StoragePoisoned)?;

        Ok(documents.get(key).cloned())
    }

    async fn list_by_index_prefix(
        &self,
        view_type: &str,
        index_name: &str,
        prefix: &str,
    ) -> Result<Vec<ViewDocument>, ViewStoreError> {
        let documents = self
            .documents
            .lock()
            .map_err(|_| ViewStoreError::StoragePoisoned)?;
        let mut matches = Vec::new();

        for document in documents.values() {
            if document.key.view_type != view_type {
                continue;
            }

            if let Some(index) = document
                .indexes
                .iter()
                .find(|index| index.index_name == index_name && index.index_key.starts_with(prefix))
            {
                matches.push((
                    index.index_key.clone(),
                    document.key.view_id.clone(),
                    document.clone(),
                ));
            }
        }

        matches.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
        Ok(matches
            .into_iter()
            .map(|(_, _, document)| document)
            .collect())
    }
}

fn validate_unique_index_names(document: &ViewDocument) -> Result<(), ViewStoreError> {
    let mut index_names = HashSet::new();
    for index in &document.indexes {
        if !index_names.insert(index.index_name.clone()) {
            return Err(ViewStoreError::DuplicateIndexName {
                view_type: document.key.view_type.clone(),
                view_id: document.key.view_id.clone(),
                index_name: index.index_name.clone(),
            });
        }
    }

    Ok(())
}

#[cfg(feature = "nats-adapter")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NatsViewStoreConfig {
    bucket: String,
}

#[cfg(feature = "nats-adapter")]
impl NatsViewStoreConfig {
    pub fn new(bucket: impl Into<String>) -> Self {
        Self {
            bucket: bucket.into(),
        }
    }

    pub fn bucket(&self) -> &str {
        &self.bucket
    }
}

#[cfg(feature = "nats-adapter")]
impl Default for NatsViewStoreConfig {
    fn default() -> Self {
        Self {
            bucket: DEFAULT_NATS_VIEW_STORE_BUCKET.to_string(),
        }
    }
}

#[cfg(feature = "nats-adapter")]
#[derive(Clone)]
pub struct NatsViewStore {
    store: async_nats::jetstream::kv::Store,
}

#[cfg(feature = "nats-adapter")]
impl NatsViewStore {
    pub async fn connect(
        url: impl AsRef<str>,
        config: NatsViewStoreConfig,
    ) -> Result<Self, ViewStoreError> {
        let client = async_nats::connect(url.as_ref()).await.map_err(|source| {
            ViewStoreError::NatsConnect {
                reason: source.to_string(),
            }
        })?;

        Self::from_client(client, config).await
    }

    pub async fn from_client(
        client: async_nats::Client,
        config: NatsViewStoreConfig,
    ) -> Result<Self, ViewStoreError> {
        let jetstream = async_nats::jetstream::new(client);

        Self::from_jetstream(jetstream, config).await
    }

    pub async fn from_jetstream(
        jetstream: async_nats::jetstream::Context,
        config: NatsViewStoreConfig,
    ) -> Result<Self, ViewStoreError> {
        let bucket = config.bucket.clone();
        let store = jetstream
            .create_or_update_key_value(async_nats::jetstream::kv::Config {
                bucket: config.bucket,
                ..Default::default()
            })
            .await
            .map_err(|source| ViewStoreError::NatsBucket {
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
impl ViewStore for NatsViewStore {
    async fn put(&self, document: ViewDocument) -> Result<(), ViewStoreError> {
        validate_unique_index_names(&document)?;

        let key = nats_view_document_key(&document.key);
        let value = serde_json::to_vec(&document).map_err(|source| {
            ViewStoreError::DocumentSerialization {
                view_type: document.key.view_type.clone(),
                view_id: document.key.view_id.clone(),
                reason: source.to_string(),
            }
        })?;

        self.store
            .put(key.as_str(), value.into())
            .await
            .map_err(|source| ViewStoreError::NatsPut {
                view_type: document.key.view_type,
                view_id: document.key.view_id,
                reason: source.to_string(),
            })?;

        Ok(())
    }

    async fn load(&self, key: &ViewKey) -> Result<Option<ViewDocument>, ViewStoreError> {
        let nats_key = nats_view_document_key(key);
        let entry = self.store.entry(nats_key.clone()).await.map_err(|source| {
            ViewStoreError::NatsLoad {
                view_type: key.view_type.clone(),
                view_id: key.view_id.clone(),
                reason: source.to_string(),
            }
        })?;

        let Some(entry) = entry else {
            return Ok(None);
        };
        if entry.operation != async_nats::jetstream::kv::Operation::Put {
            return Ok(None);
        }

        let document = deserialize_view_document(&nats_key, entry.revision, &entry.value)?;

        Ok(Some(document))
    }

    async fn list_by_index_prefix(
        &self,
        view_type: &str,
        index_name: &str,
        prefix: &str,
    ) -> Result<Vec<ViewDocument>, ViewStoreError> {
        let mut keys = self
            .store
            .keys()
            .await
            .map_err(|source| ViewStoreError::NatsList {
                view_type: view_type.to_string(),
                index_name: index_name.to_string(),
                prefix: prefix.to_string(),
                reason: source.to_string(),
            })?;
        let mut matches = Vec::new();

        while let Some(key) = keys.next().await {
            let key = key.map_err(|source| ViewStoreError::NatsList {
                view_type: view_type.to_string(),
                index_name: index_name.to_string(),
                prefix: prefix.to_string(),
                reason: source.to_string(),
            })?;
            let Some(entry) =
                self.store
                    .entry(key.clone())
                    .await
                    .map_err(|source| ViewStoreError::NatsList {
                        view_type: view_type.to_string(),
                        index_name: index_name.to_string(),
                        prefix: prefix.to_string(),
                        reason: source.to_string(),
                    })?
            else {
                continue;
            };

            if entry.operation != async_nats::jetstream::kv::Operation::Put {
                continue;
            }

            let document = deserialize_view_document(&key, entry.revision, &entry.value)?;
            if document.key.view_type != view_type {
                continue;
            }

            if let Some(index) = document
                .indexes
                .iter()
                .find(|index| index.index_name == index_name && index.index_key.starts_with(prefix))
            {
                matches.push((
                    index.index_key.clone(),
                    document.key.view_id.clone(),
                    document,
                ));
            }
        }

        matches.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
        Ok(matches
            .into_iter()
            .map(|(_, _, document)| document)
            .collect())
    }
}

#[cfg(feature = "nats-adapter")]
fn deserialize_view_document(
    key: &str,
    revision: u64,
    value: &[u8],
) -> Result<ViewDocument, ViewStoreError> {
    serde_json::from_slice(value).map_err(|source| ViewStoreError::DocumentDeserialization {
        key: key.to_string(),
        revision,
        reason: source.to_string(),
    })
}

#[cfg(feature = "nats-adapter")]
fn nats_view_document_key(key: &ViewKey) -> String {
    format!(
        "view.{}.{}.{}.{}",
        key.view_type.len(),
        encode_nats_key_token(&key.view_type),
        key.view_id.len(),
        encode_nats_key_token(&key.view_id)
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
    fn nats_view_document_key_leaves_plain_view_keys_readable() {
        let key = ViewKey::new("offer_summary", "offer-1");

        assert_eq!(
            nats_view_document_key(&key),
            "view.13.offer_summary.7.offer-1"
        );
    }

    #[test]
    fn nats_view_document_key_escapes_key_token_separators_and_wildcards() {
        let key = ViewKey::new("flow.status", "id/*>");
        let nats_key = nats_view_document_key(&key);
        let tokens: Vec<_> = nats_key.split('.').collect();

        assert_eq!(
            tokens,
            vec!["view", "11", "flow%2Estatus", "5", "id%2F%2A%3E"]
        );
        assert!(!tokens[2].contains('*'));
        assert!(!tokens[2].contains('>'));
        assert!(!tokens[4].contains('*'));
        assert!(!tokens[4].contains('>'));
    }

    #[test]
    fn nats_view_document_key_distinguishes_empty_tokens() {
        let empty_key = ViewKey::new("", "");
        let underscore_key = ViewKey::new("_", "_");

        assert_eq!(nats_view_document_key(&empty_key), "view.0._.0._");
        assert_eq!(nats_view_document_key(&underscore_key), "view.1._.1._");
    }
}
