use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::{EventStoreError, NewEvent, RecordedEvent, ResourceStream};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedVersion {
    Any,
    NoStream,
    Exact(u64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppendResult {
    pub previous_version: u64,
    pub new_version: u64,
    pub events: Vec<RecordedEvent>,
}

#[async_trait]
pub trait EventStore: Send + Sync + 'static {
    async fn load(&self, stream: &ResourceStream) -> Result<Vec<RecordedEvent>, EventStoreError>;

    async fn append(
        &self,
        stream: &ResourceStream,
        expected_version: ExpectedVersion,
        events: Vec<NewEvent>,
    ) -> Result<AppendResult, EventStoreError>;
}

#[derive(Clone, Default)]
pub struct InMemoryEventStore {
    events: Arc<Mutex<HashMap<ResourceStream, Vec<RecordedEvent>>>>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn all_events(&self) -> Vec<RecordedEvent> {
        let events = self.events.lock().expect("in-memory event store poisoned");

        events.values().flatten().cloned().collect()
    }
}

#[async_trait]
impl EventStore for InMemoryEventStore {
    async fn load(&self, stream: &ResourceStream) -> Result<Vec<RecordedEvent>, EventStoreError> {
        let events = self.events.lock().expect("in-memory event store poisoned");
        Ok(events.get(stream).cloned().unwrap_or_default())
    }

    async fn append(
        &self,
        stream: &ResourceStream,
        expected_version: ExpectedVersion,
        events: Vec<NewEvent>,
    ) -> Result<AppendResult, EventStoreError> {
        let mut stored = self.events.lock().expect("in-memory event store poisoned");
        let stream_events = stored.entry(stream.clone()).or_default();
        let previous_version = stream_events.len() as u64;

        match expected_version {
            ExpectedVersion::Any => {}
            ExpectedVersion::NoStream if previous_version != 0 => {
                return Err(EventStoreError::ConcurrencyConflict {
                    stream: stream.key(),
                    expected: 0,
                    actual: previous_version,
                });
            }
            ExpectedVersion::Exact(expected) if previous_version != expected => {
                return Err(EventStoreError::ConcurrencyConflict {
                    stream: stream.key(),
                    expected,
                    actual: previous_version,
                });
            }
            ExpectedVersion::NoStream | ExpectedVersion::Exact(_) => {}
        }

        let recorded: Vec<_> = events
            .into_iter()
            .enumerate()
            .map(|(index, event)| RecordedEvent {
                stream: stream.clone(),
                sequence: previous_version + index as u64 + 1,
                metadata: event.metadata,
                payload: event.payload,
            })
            .collect();

        stream_events.extend(recorded.clone());

        Ok(AppendResult {
            previous_version,
            new_version: previous_version + recorded.len() as u64,
            events: recorded,
        })
    }
}
