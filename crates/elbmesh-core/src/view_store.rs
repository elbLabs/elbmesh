use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

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
