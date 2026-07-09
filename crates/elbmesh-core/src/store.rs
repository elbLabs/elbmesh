use async_trait::async_trait;
#[cfg(feature = "nats-adapter")]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::{EventStoreError, NewEvent, RecordedEvent, ResourceStream, StreamType};

#[cfg(feature = "nats-adapter")]
const DEFAULT_NATS_EVENT_STORE_BUCKET: &str = "elbmesh_event_store";
#[cfg(feature = "nats-adapter")]
const HEX: &[u8; 16] = b"0123456789ABCDEF";

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

        validate_expected_version(stream, expected_version, previous_version)?;
        validate_new_events(stream, &events)?;

        let recorded = record_new_events(stream, previous_version, events);

        stream_events.extend(recorded.clone());

        Ok(AppendResult {
            previous_version,
            new_version: previous_version + recorded.len() as u64,
            events: recorded,
        })
    }
}

fn validate_expected_version(
    stream: &ResourceStream,
    expected_version: ExpectedVersion,
    previous_version: u64,
) -> Result<(), EventStoreError> {
    match expected_version {
        ExpectedVersion::Any => Ok(()),
        ExpectedVersion::NoStream if previous_version != 0 => {
            Err(EventStoreError::ConcurrencyConflict {
                stream: stream.key(),
                expected: 0,
                actual: previous_version,
            })
        }
        ExpectedVersion::Exact(expected) if previous_version != expected => {
            Err(EventStoreError::ConcurrencyConflict {
                stream: stream.key(),
                expected,
                actual: previous_version,
            })
        }
        ExpectedVersion::NoStream | ExpectedVersion::Exact(_) => Ok(()),
    }
}

fn validate_new_events(
    stream: &ResourceStream,
    events: &[NewEvent],
) -> Result<(), EventStoreError> {
    for event in events {
        if event.metadata.stream_type != StreamType::Resource {
            return Err(EventStoreError::WrongEventStreamType {
                stream: stream.key(),
                expected_stream_type: StreamType::Resource,
                actual_stream_type: event.metadata.stream_type.clone(),
            });
        }

        if event.metadata.resource_type != stream.resource_type
            || event.metadata.resource_id != stream.resource_id
        {
            return Err(EventStoreError::WrongEventStream {
                stream: stream.key(),
                expected_resource_type: stream.resource_type.clone(),
                expected_resource_id: stream.resource_id.clone(),
                actual_resource_type: event.metadata.resource_type.clone(),
                actual_resource_id: event.metadata.resource_id.clone(),
            });
        }
    }

    Ok(())
}

fn record_new_events(
    stream: &ResourceStream,
    previous_version: u64,
    events: Vec<NewEvent>,
) -> Vec<RecordedEvent> {
    events
        .into_iter()
        .enumerate()
        .map(|(index, event)| RecordedEvent {
            stream: stream.clone(),
            sequence: previous_version + index as u64 + 1,
            metadata: event.metadata,
            payload: event.payload,
        })
        .collect()
}

#[cfg(feature = "nats-adapter")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NatsEventStoreConfig {
    bucket: String,
}

#[cfg(feature = "nats-adapter")]
impl NatsEventStoreConfig {
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
impl Default for NatsEventStoreConfig {
    fn default() -> Self {
        Self {
            bucket: DEFAULT_NATS_EVENT_STORE_BUCKET.to_string(),
        }
    }
}

#[cfg(feature = "nats-adapter")]
#[derive(Clone)]
pub struct NatsEventStore {
    store: async_nats::jetstream::kv::Store,
}

#[cfg(feature = "nats-adapter")]
impl NatsEventStore {
    pub async fn connect(
        url: impl AsRef<str>,
        config: NatsEventStoreConfig,
    ) -> Result<Self, EventStoreError> {
        let client = async_nats::connect(url.as_ref()).await.map_err(|source| {
            EventStoreError::NatsConnect {
                reason: source.to_string(),
            }
        })?;

        Self::from_client(client, config).await
    }

    pub async fn from_client(
        client: async_nats::Client,
        config: NatsEventStoreConfig,
    ) -> Result<Self, EventStoreError> {
        let jetstream = async_nats::jetstream::new(client);

        Self::from_jetstream(jetstream, config).await
    }

    pub async fn from_jetstream(
        jetstream: async_nats::jetstream::Context,
        config: NatsEventStoreConfig,
    ) -> Result<Self, EventStoreError> {
        let bucket = config.bucket.clone();
        let store = jetstream
            .create_or_update_key_value(async_nats::jetstream::kv::Config {
                bucket: config.bucket,
                ..Default::default()
            })
            .await
            .map_err(|source| EventStoreError::NatsBucket {
                bucket,
                reason: source.to_string(),
            })?;

        Ok(Self { store })
    }

    pub fn from_store(store: async_nats::jetstream::kv::Store) -> Self {
        Self { store }
    }

    async fn load_document(
        &self,
        stream: &ResourceStream,
    ) -> Result<NatsEventStreamDocumentLoad, EventStoreError> {
        let key = nats_event_stream_key(stream);
        let entry = self
            .store
            .entry(key)
            .await
            .map_err(|source| EventStoreError::NatsLoad {
                stream: stream.key(),
                reason: source.to_string(),
            })?;

        let Some(entry) = entry else {
            return Ok(NatsEventStreamDocumentLoad::empty());
        };

        if entry.operation != async_nats::jetstream::kv::Operation::Put {
            return Ok(NatsEventStreamDocumentLoad::empty());
        }

        let document: NatsEventStreamDocument =
            serde_json::from_slice(&entry.value).map_err(|source| {
                EventStoreError::StreamDeserialization {
                    stream: stream.key(),
                    revision: entry.revision,
                    reason: source.to_string(),
                }
            })?;

        for event in &document.events {
            if event.stream.resource_type != stream.resource_type
                || event.stream.resource_id != stream.resource_id
            {
                return Err(EventStoreError::WrongEventStream {
                    stream: stream.key(),
                    expected_resource_type: stream.resource_type.clone(),
                    expected_resource_id: stream.resource_id.clone(),
                    actual_resource_type: event.stream.resource_type.clone(),
                    actual_resource_id: event.stream.resource_id.clone(),
                });
            }
        }

        Ok(NatsEventStreamDocumentLoad {
            revision: entry.revision,
            events: document.events,
        })
    }

    async fn version_after_write_conflict(
        &self,
        stream: &ResourceStream,
    ) -> Result<u64, EventStoreError> {
        Ok(self.load_document(stream).await?.events.len() as u64)
    }
}

#[cfg(feature = "nats-adapter")]
#[async_trait]
impl EventStore for NatsEventStore {
    async fn load(&self, stream: &ResourceStream) -> Result<Vec<RecordedEvent>, EventStoreError> {
        Ok(self.load_document(stream).await?.events)
    }

    async fn append(
        &self,
        stream: &ResourceStream,
        expected_version: ExpectedVersion,
        events: Vec<NewEvent>,
    ) -> Result<AppendResult, EventStoreError> {
        loop {
            let loaded = self.load_document(stream).await?;
            let previous_version = loaded.events.len() as u64;
            validate_expected_version(stream, expected_version, previous_version)?;
            validate_new_events(stream, &events)?;
            let recorded = record_new_events(stream, previous_version, events.clone());

            if recorded.is_empty() {
                return Ok(AppendResult {
                    previous_version,
                    new_version: previous_version,
                    events: recorded,
                });
            }

            let mut stored_events = loaded.events;
            stored_events.extend(recorded.clone());
            let document = NatsEventStreamDocument {
                events: stored_events,
            };
            let value = serde_json::to_vec(&document).map_err(|source| {
                EventStoreError::StreamSerialization {
                    stream: stream.key(),
                    reason: source.to_string(),
                }
            })?;

            let key = nats_event_stream_key(stream);
            let write_result = if loaded.revision == 0 {
                self.store
                    .create(key.as_str(), value.into())
                    .await
                    .map(|_| ())
                    .map_err(NatsEventStreamWriteError::from)
            } else {
                self.store
                    .update(key.as_str(), value.into(), loaded.revision)
                    .await
                    .map(|_| ())
                    .map_err(NatsEventStreamWriteError::from)
            };

            match write_result {
                Ok(()) => {
                    return Ok(AppendResult {
                        previous_version,
                        new_version: previous_version + recorded.len() as u64,
                        events: recorded,
                    });
                }
                Err(NatsEventStreamWriteError::Conflict)
                    if expected_version == ExpectedVersion::Any =>
                {
                    continue;
                }
                Err(NatsEventStreamWriteError::Conflict) => {
                    let actual = self.version_after_write_conflict(stream).await?;
                    return Err(EventStoreError::ConcurrencyConflict {
                        stream: stream.key(),
                        expected: previous_version,
                        actual,
                    });
                }
                Err(NatsEventStreamWriteError::Storage(reason)) => {
                    return Err(EventStoreError::NatsAppend {
                        stream: stream.key(),
                        reason,
                    });
                }
            }
        }
    }
}

#[cfg(feature = "nats-adapter")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct NatsEventStreamDocument {
    events: Vec<RecordedEvent>,
}

#[cfg(feature = "nats-adapter")]
struct NatsEventStreamDocumentLoad {
    revision: u64,
    events: Vec<RecordedEvent>,
}

#[cfg(feature = "nats-adapter")]
impl NatsEventStreamDocumentLoad {
    fn empty() -> Self {
        Self {
            revision: 0,
            events: Vec::new(),
        }
    }
}

#[cfg(feature = "nats-adapter")]
enum NatsEventStreamWriteError {
    Conflict,
    Storage(String),
}

#[cfg(feature = "nats-adapter")]
impl From<async_nats::jetstream::kv::CreateError> for NatsEventStreamWriteError {
    fn from(source: async_nats::jetstream::kv::CreateError) -> Self {
        if source.kind() == async_nats::jetstream::kv::CreateErrorKind::AlreadyExists {
            Self::Conflict
        } else {
            Self::Storage(source.to_string())
        }
    }
}

#[cfg(feature = "nats-adapter")]
impl From<async_nats::jetstream::kv::UpdateError> for NatsEventStreamWriteError {
    fn from(source: async_nats::jetstream::kv::UpdateError) -> Self {
        if source.kind() == async_nats::jetstream::kv::UpdateErrorKind::WrongLastRevision {
            Self::Conflict
        } else {
            Self::Storage(source.to_string())
        }
    }
}

#[cfg(feature = "nats-adapter")]
fn nats_event_stream_key(stream: &ResourceStream) -> String {
    format!(
        "resource.{}.{}.{}.{}",
        stream.resource_type.len(),
        encode_nats_key_token(&stream.resource_type),
        stream.resource_id.len(),
        encode_nats_key_token(&stream.resource_id)
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
    fn nats_event_stream_key_leaves_plain_resource_streams_readable() {
        let stream = ResourceStream::new("offer", "offer-123");

        assert_eq!(
            nats_event_stream_key(&stream),
            "resource.5.offer.9.offer-123"
        );
    }

    #[test]
    fn nats_event_stream_key_escapes_key_token_separators_and_wildcards() {
        let stream = ResourceStream::new("sales.offer", "tenant.1/offer*>");
        let key = nats_event_stream_key(&stream);
        let tokens: Vec<_> = key.split('.').collect();

        assert_eq!(
            tokens,
            vec![
                "resource",
                "11",
                "sales%2Eoffer",
                "16",
                "tenant%2E1%2Foffer%2A%3E"
            ]
        );
        assert!(!tokens[2].contains('*'));
        assert!(!tokens[2].contains('>'));
        assert!(!tokens[4].contains('*'));
        assert!(!tokens[4].contains('>'));
    }

    #[test]
    fn nats_event_stream_key_distinguishes_empty_tokens() {
        let empty_stream = ResourceStream::new("", "");
        let underscore_stream = ResourceStream::new("_", "_");

        assert_eq!(nats_event_stream_key(&empty_stream), "resource.0._.0._");
        assert_eq!(
            nats_event_stream_key(&underscore_stream),
            "resource.1._.1._"
        );
    }
}
