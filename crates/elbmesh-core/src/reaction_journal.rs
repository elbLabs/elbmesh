use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::MessageMetadata;

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
        trigger_event_type: String,
        trigger_event_id: String,
    },
    ReactionCompleted {
        reaction_id: String,
        metadata: MessageMetadata,
        triggered_action_ids: Vec<String>,
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
