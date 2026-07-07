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
}

impl ViewDocument {
    pub fn new(view_type: impl Into<String>, view_id: impl Into<String>, payload: Value) -> Self {
        Self {
            key: ViewKey::new(view_type, view_id),
            payload,
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
}
