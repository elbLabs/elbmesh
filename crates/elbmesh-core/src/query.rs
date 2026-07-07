use thiserror::Error;

use crate::{ArchitectureManifest, ViewDocument, ViewKey, ViewStore, ViewStoreError};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum QueryError {
    #[error("query '{query_type}' is not declared")]
    UnknownQuery { query_type: String },

    #[error("query '{query_type}' does not declare index '{index_name}'")]
    UndeclaredIndex {
        query_type: String,
        index_name: String,
    },

    #[error("query '{query_type}' found no '{view_type}' view document with id '{view_id}'")]
    ViewDocumentNotFound {
        query_type: String,
        view_type: String,
        view_id: String,
    },

    #[error(transparent)]
    ViewStore(#[from] ViewStoreError),
}

impl QueryError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownQuery { .. } => "query.unknown_query",
            Self::UndeclaredIndex { .. } => "query.undeclared_index",
            Self::ViewDocumentNotFound { .. } => "query.view_document_not_found",
            Self::ViewStore(_) => "query.view_store_error",
        }
    }
}

#[derive(Clone)]
pub struct QueryExecutor<S> {
    manifest: ArchitectureManifest,
    view_store: S,
}

impl<S> QueryExecutor<S>
where
    S: ViewStore,
{
    pub fn new(manifest: ArchitectureManifest, view_store: S) -> Self {
        Self {
            manifest,
            view_store,
        }
    }

    pub async fn get_by_id(
        &self,
        query_type: &str,
        view_id: &str,
    ) -> Result<ViewDocument, QueryError> {
        let query = self.query(query_type)?;
        let view_type = query.view_type.clone();
        let key = ViewKey::new(&view_type, view_id);

        self.view_store
            .load(&key)
            .await?
            .ok_or_else(|| QueryError::ViewDocumentNotFound {
                query_type: query_type.to_string(),
                view_type,
                view_id: view_id.to_string(),
            })
    }

    pub async fn list_by_index_prefix(
        &self,
        query_type: &str,
        index_name: &str,
        prefix: &str,
    ) -> Result<Vec<ViewDocument>, QueryError> {
        let query = self.query(query_type)?;
        if !query.index_names.iter().any(|name| name == index_name) {
            return Err(QueryError::UndeclaredIndex {
                query_type: query_type.to_string(),
                index_name: index_name.to_string(),
            });
        }

        Ok(self
            .view_store
            .list_by_index_prefix(&query.view_type, index_name, prefix)
            .await?)
    }

    fn query(&self, query_type: &str) -> Result<&crate::QueryDefinition, QueryError> {
        self.manifest
            .queries
            .iter()
            .find(|query| query.query_type == query_type)
            .ok_or_else(|| QueryError::UnknownQuery {
                query_type: query_type.to_string(),
            })
    }
}
