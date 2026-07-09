use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[cfg(feature = "nats-adapter")]
use futures_util::StreamExt;

#[cfg(feature = "restate-adapter")]
use restate_sdk::prelude::{
    ContextReadState, ContextWriteState, HandlerError as RestateHandlerError, HandlerResult, Json,
    ObjectContext, SharedObjectContext, TerminalError as RestateTerminalError,
};

use crate::MessageMetadata;

#[cfg(feature = "nats-adapter")]
const DEFAULT_NATS_OPERATION_JOURNAL_BUCKET: &str = "elbmesh_operation_journal";
#[cfg(feature = "nats-adapter")]
const DEFAULT_NATS_OPERATION_JOURNAL_HISTORY: i64 = 64;
#[cfg(feature = "nats-adapter")]
const HEX: &[u8; 16] = b"0123456789ABCDEF";

#[cfg(feature = "restate-adapter")]
const DEFAULT_RESTATE_OPERATION_JOURNAL_OBJECT: &str = "ElbmeshOperationJournal";
#[cfg(feature = "restate-adapter")]
const RESTATE_OPERATION_JOURNAL_RECORDS_STATE: &str = "records";
#[cfg(feature = "restate-adapter")]
const RESTATE_HEX: &[u8; 16] = b"0123456789ABCDEF";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OperationJournalStream {
    pub operation_id: String,
}

impl OperationJournalStream {
    pub fn for_operation(operation_id: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OperationJournalRecord {
    OperationCalled {
        operation_id: String,
        metadata: MessageMetadata,
        operation_type: String,
        operation_schema_id: String,
        operation_schema_version: u32,
        idempotency_key: String,
        payload: Value,
    },
    OperationCompleted {
        operation_id: String,
        metadata: MessageMetadata,
        response: Value,
    },
    OperationFailed {
        operation_id: String,
        metadata: MessageMetadata,
        failure_code: String,
        failure_details: Value,
    },
}

impl OperationJournalRecord {
    fn operation_id(&self) -> &str {
        match self {
            Self::OperationCalled { operation_id, .. }
            | Self::OperationCompleted { operation_id, .. }
            | Self::OperationFailed { operation_id, .. } => operation_id,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum OperationJournalError {
    #[error("operation journal record targets operation '{actual_operation_id}', but stream is for operation '{expected_operation_id}'")]
    WrongOperationStream {
        expected_operation_id: String,
        actual_operation_id: String,
    },

    #[error("operation journal storage is poisoned")]
    StoragePoisoned,

    #[error("failed to connect NATS OperationJournal: {reason}")]
    NatsConnect { reason: String },

    #[error("failed to open NATS OperationJournal bucket '{bucket}': {reason}")]
    NatsBucket { bucket: String, reason: String },

    #[error("failed to serialize operation journal record: {reason}")]
    RecordSerialization { reason: String },

    #[error("failed to deserialize operation journal record from stream '{stream}' revision {revision}: {reason}")]
    RecordDeserialization {
        stream: String,
        revision: u64,
        reason: String,
    },

    #[error("failed to append operation journal record to NATS stream '{stream}': {reason}")]
    NatsAppend { stream: String, reason: String },

    #[error("failed to load operation journal records from NATS stream '{stream}': {reason}")]
    NatsLoad { stream: String, reason: String },

    #[error("failed to build Restate OperationJournal request for stream '{stream}': {reason}")]
    RestateRequest { stream: String, reason: String },

    #[error("failed to append operation journal record to Restate stream '{stream}': {reason}")]
    RestateAppend { stream: String, reason: String },

    #[error("failed to load operation journal records from Restate stream '{stream}': {reason}")]
    RestateLoad { stream: String, reason: String },

    #[error(
        "failed to deserialize Restate operation journal response from stream '{stream}': {reason}"
    )]
    RestateResponseDeserialization { stream: String, reason: String },
}

impl OperationJournalError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::WrongOperationStream { .. } => "operation_journal.wrong_operation_stream",
            Self::StoragePoisoned => "operation_journal.storage_poisoned",
            Self::NatsConnect { .. } => "operation_journal.nats_connect",
            Self::NatsBucket { .. } => "operation_journal.nats_bucket",
            Self::RecordSerialization { .. } => "operation_journal.record_serialization",
            Self::RecordDeserialization { .. } => "operation_journal.record_deserialization",
            Self::NatsAppend { .. } => "operation_journal.nats_append",
            Self::NatsLoad { .. } => "operation_journal.nats_load",
            Self::RestateRequest { .. } => "operation_journal.restate_request",
            Self::RestateAppend { .. } => "operation_journal.restate_append",
            Self::RestateLoad { .. } => "operation_journal.restate_load",
            Self::RestateResponseDeserialization { .. } => {
                "operation_journal.restate_response_deserialization"
            }
        }
    }

    pub fn details(&self) -> Value {
        match self {
            Self::WrongOperationStream {
                expected_operation_id,
                actual_operation_id,
            } => serde_json::json!({
                "error_type": "OperationJournalError",
                "error_variant": "WrongOperationStream",
                "expected_operation_id": expected_operation_id,
                "actual_operation_id": actual_operation_id,
            }),
            Self::StoragePoisoned => serde_json::json!({
                "error_type": "OperationJournalError",
                "error_variant": "StoragePoisoned",
            }),
            Self::NatsConnect { reason } => serde_json::json!({
                "error_type": "OperationJournalError",
                "error_variant": "NatsConnect",
                "reason": reason,
            }),
            Self::NatsBucket { bucket, reason } => serde_json::json!({
                "error_type": "OperationJournalError",
                "error_variant": "NatsBucket",
                "bucket": bucket,
                "reason": reason,
            }),
            Self::RecordSerialization { reason } => serde_json::json!({
                "error_type": "OperationJournalError",
                "error_variant": "RecordSerialization",
                "reason": reason,
            }),
            Self::RecordDeserialization {
                stream,
                revision,
                reason,
            } => serde_json::json!({
                "error_type": "OperationJournalError",
                "error_variant": "RecordDeserialization",
                "stream": stream,
                "revision": revision,
                "reason": reason,
            }),
            Self::NatsAppend { stream, reason } => serde_json::json!({
                "error_type": "OperationJournalError",
                "error_variant": "NatsAppend",
                "stream": stream,
                "reason": reason,
            }),
            Self::NatsLoad { stream, reason } => serde_json::json!({
                "error_type": "OperationJournalError",
                "error_variant": "NatsLoad",
                "stream": stream,
                "reason": reason,
            }),
            Self::RestateRequest { stream, reason } => serde_json::json!({
                "error_type": "OperationJournalError",
                "error_variant": "RestateRequest",
                "stream": stream,
                "reason": reason,
            }),
            Self::RestateAppend { stream, reason } => serde_json::json!({
                "error_type": "OperationJournalError",
                "error_variant": "RestateAppend",
                "stream": stream,
                "reason": reason,
            }),
            Self::RestateLoad { stream, reason } => serde_json::json!({
                "error_type": "OperationJournalError",
                "error_variant": "RestateLoad",
                "stream": stream,
                "reason": reason,
            }),
            Self::RestateResponseDeserialization { stream, reason } => serde_json::json!({
                "error_type": "OperationJournalError",
                "error_variant": "RestateResponseDeserialization",
                "stream": stream,
                "reason": reason,
            }),
        }
    }
}

#[async_trait]
pub trait OperationJournal: Send + Sync + 'static {
    async fn append(
        &self,
        stream: &OperationJournalStream,
        record: OperationJournalRecord,
    ) -> Result<(), OperationJournalError>;

    async fn load(
        &self,
        stream: &OperationJournalStream,
    ) -> Result<Vec<OperationJournalRecord>, OperationJournalError>;
}

#[derive(Clone, Default)]
pub struct InMemoryOperationJournal {
    records: Arc<Mutex<HashMap<OperationJournalStream, Vec<OperationJournalRecord>>>>,
}

impl InMemoryOperationJournal {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl OperationJournal for InMemoryOperationJournal {
    async fn append(
        &self,
        stream: &OperationJournalStream,
        record: OperationJournalRecord,
    ) -> Result<(), OperationJournalError> {
        validate_operation_record_stream(stream, &record)?;

        let mut records = self
            .records
            .lock()
            .map_err(|_| OperationJournalError::StoragePoisoned)?;
        records.entry(stream.clone()).or_default().push(record);

        Ok(())
    }

    async fn load(
        &self,
        stream: &OperationJournalStream,
    ) -> Result<Vec<OperationJournalRecord>, OperationJournalError> {
        let records = self
            .records
            .lock()
            .map_err(|_| OperationJournalError::StoragePoisoned)?;

        Ok(records.get(stream).cloned().unwrap_or_default())
    }
}

#[cfg(feature = "restate-adapter")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestateOperationJournalConfig {
    base_url: String,
    object_name: String,
}

#[cfg(feature = "restate-adapter")]
impl RestateOperationJournalConfig {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: trim_trailing_slashes(base_url.into()),
            object_name: DEFAULT_RESTATE_OPERATION_JOURNAL_OBJECT.to_string(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn object_name(&self) -> &str {
        &self.object_name
    }

    pub fn with_object_name(mut self, object_name: impl Into<String>) -> Self {
        self.object_name = object_name.into();
        self
    }
}

#[cfg(feature = "restate-adapter")]
#[derive(Clone)]
pub struct RestateOperationJournal {
    client: reqwest::Client,
    config: RestateOperationJournalConfig,
}

#[cfg(feature = "restate-adapter")]
impl RestateOperationJournal {
    pub fn new(config: RestateOperationJournalConfig) -> Self {
        Self::with_client(reqwest::Client::new(), config)
    }

    pub fn with_client(client: reqwest::Client, config: RestateOperationJournalConfig) -> Self {
        Self { client, config }
    }

    pub fn config(&self) -> &RestateOperationJournalConfig {
        &self.config
    }

    fn handler_url(
        &self,
        stream: &OperationJournalStream,
        handler: &str,
    ) -> Result<reqwest::Url, OperationJournalError> {
        let url = format!(
            "{}/{}/{}/{}",
            self.config.base_url,
            encode_restate_path_segment(&self.config.object_name),
            encode_restate_path_segment(&stream.operation_id),
            encode_restate_path_segment(handler)
        );

        reqwest::Url::parse(&url).map_err(|source| OperationJournalError::RestateRequest {
            stream: stream.operation_id.clone(),
            reason: source.to_string(),
        })
    }
}

#[cfg(feature = "restate-adapter")]
#[async_trait]
impl OperationJournal for RestateOperationJournal {
    async fn append(
        &self,
        stream: &OperationJournalStream,
        record: OperationJournalRecord,
    ) -> Result<(), OperationJournalError> {
        validate_operation_record_stream(stream, &record)?;

        let url = self.handler_url(stream, "append")?;
        let response = self
            .client
            .post(url)
            .json(&record)
            .send()
            .await
            .map_err(|source| OperationJournalError::RestateAppend {
                stream: stream.operation_id.clone(),
                reason: source.to_string(),
            })?;

        ensure_restate_success(response, stream, "append").await?;
        Ok(())
    }

    async fn load(
        &self,
        stream: &OperationJournalStream,
    ) -> Result<Vec<OperationJournalRecord>, OperationJournalError> {
        let url = self.handler_url(stream, "load")?;
        let response = self.client.post(url).send().await.map_err(|source| {
            OperationJournalError::RestateLoad {
                stream: stream.operation_id.clone(),
                reason: source.to_string(),
            }
        })?;

        let response = ensure_restate_success(response, stream, "load").await?;
        let body = response
            .bytes()
            .await
            .map_err(|source| OperationJournalError::RestateLoad {
                stream: stream.operation_id.clone(),
                reason: source.to_string(),
            })?;
        let records: Vec<OperationJournalRecord> =
            serde_json::from_slice(&body).map_err(|source| {
                OperationJournalError::RestateResponseDeserialization {
                    stream: stream.operation_id.clone(),
                    reason: source.to_string(),
                }
            })?;

        for record in &records {
            validate_operation_record_stream(stream, record)?;
        }

        Ok(records)
    }
}

#[cfg(feature = "restate-adapter")]
#[restate_sdk::object]
#[name = "ElbmeshOperationJournal"]
pub trait RestateOperationJournalObject {
    async fn append(record: Json<OperationJournalRecord>) -> HandlerResult<()>;

    #[shared]
    async fn load() -> HandlerResult<Json<Vec<OperationJournalRecord>>>;
}

#[cfg(feature = "restate-adapter")]
pub struct RestateOperationJournalObjectImpl;

#[cfg(feature = "restate-adapter")]
impl RestateOperationJournalObject for RestateOperationJournalObjectImpl {
    async fn append(
        &self,
        ctx: ObjectContext<'_>,
        Json(record): Json<OperationJournalRecord>,
    ) -> HandlerResult<()> {
        let stream = OperationJournalStream::for_operation(ctx.key());
        validate_operation_record_stream(&stream, &record).map_err(restate_terminal_error)?;

        let mut records = ctx
            .get::<Json<Vec<OperationJournalRecord>>>(RESTATE_OPERATION_JOURNAL_RECORDS_STATE)
            .await?
            .map(Json::into_inner)
            .unwrap_or_default();
        records.push(record);
        ctx.set(RESTATE_OPERATION_JOURNAL_RECORDS_STATE, Json(records));

        Ok(())
    }

    async fn load(
        &self,
        ctx: SharedObjectContext<'_>,
    ) -> HandlerResult<Json<Vec<OperationJournalRecord>>> {
        let stream = OperationJournalStream::for_operation(ctx.key());
        let records = ctx
            .get::<Json<Vec<OperationJournalRecord>>>(RESTATE_OPERATION_JOURNAL_RECORDS_STATE)
            .await?
            .map(Json::into_inner)
            .unwrap_or_default();

        for record in &records {
            validate_operation_record_stream(&stream, record).map_err(restate_terminal_error)?;
        }

        Ok(Json(records))
    }
}

#[cfg(feature = "nats-adapter")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NatsOperationJournalConfig {
    bucket: String,
    history: i64,
}

#[cfg(feature = "nats-adapter")]
impl NatsOperationJournalConfig {
    pub fn new(bucket: impl Into<String>) -> Self {
        Self {
            bucket: bucket.into(),
            history: DEFAULT_NATS_OPERATION_JOURNAL_HISTORY,
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
impl Default for NatsOperationJournalConfig {
    fn default() -> Self {
        Self {
            bucket: DEFAULT_NATS_OPERATION_JOURNAL_BUCKET.to_string(),
            history: DEFAULT_NATS_OPERATION_JOURNAL_HISTORY,
        }
    }
}

#[cfg(feature = "nats-adapter")]
#[derive(Clone)]
pub struct NatsOperationJournal {
    store: async_nats::jetstream::kv::Store,
}

#[cfg(feature = "nats-adapter")]
impl NatsOperationJournal {
    pub async fn connect(
        url: impl AsRef<str>,
        config: NatsOperationJournalConfig,
    ) -> Result<Self, OperationJournalError> {
        let client = async_nats::connect(url.as_ref()).await.map_err(|source| {
            OperationJournalError::NatsConnect {
                reason: source.to_string(),
            }
        })?;

        Self::from_client(client, config).await
    }

    pub async fn from_client(
        client: async_nats::Client,
        config: NatsOperationJournalConfig,
    ) -> Result<Self, OperationJournalError> {
        let jetstream = async_nats::jetstream::new(client);

        Self::from_jetstream(jetstream, config).await
    }

    pub async fn from_jetstream(
        jetstream: async_nats::jetstream::Context,
        config: NatsOperationJournalConfig,
    ) -> Result<Self, OperationJournalError> {
        let bucket = config.bucket.clone();
        let store = jetstream
            .create_or_update_key_value(async_nats::jetstream::kv::Config {
                bucket: config.bucket,
                history: config.history,
                ..Default::default()
            })
            .await
            .map_err(|source| OperationJournalError::NatsBucket {
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
impl OperationJournal for NatsOperationJournal {
    async fn append(
        &self,
        stream: &OperationJournalStream,
        record: OperationJournalRecord,
    ) -> Result<(), OperationJournalError> {
        validate_operation_record_stream(stream, &record)?;

        let key = nats_operation_journal_key(stream);
        let value = serde_json::to_vec(&record).map_err(|source| {
            OperationJournalError::RecordSerialization {
                reason: source.to_string(),
            }
        })?;

        self.store
            .put(key.as_str(), value.into())
            .await
            .map_err(|source| OperationJournalError::NatsAppend {
                stream: stream.operation_id.clone(),
                reason: source.to_string(),
            })?;

        Ok(())
    }

    async fn load(
        &self,
        stream: &OperationJournalStream,
    ) -> Result<Vec<OperationJournalRecord>, OperationJournalError> {
        let key = nats_operation_journal_key(stream);
        let mut history = self.store.history(key.as_str()).await.map_err(|source| {
            OperationJournalError::NatsLoad {
                stream: stream.operation_id.clone(),
                reason: source.to_string(),
            }
        })?;
        let mut records = Vec::new();

        while let Some(entry) = history.next().await {
            let entry = entry.map_err(|source| OperationJournalError::NatsLoad {
                stream: stream.operation_id.clone(),
                reason: source.to_string(),
            })?;

            if entry.operation != async_nats::jetstream::kv::Operation::Put {
                continue;
            }

            let record: OperationJournalRecord =
                serde_json::from_slice(&entry.value).map_err(|source| {
                    OperationJournalError::RecordDeserialization {
                        stream: stream.operation_id.clone(),
                        revision: entry.revision,
                        reason: source.to_string(),
                    }
                })?;
            validate_operation_record_stream(stream, &record)?;

            records.push((entry.revision, record));
        }

        records.sort_by_key(|(revision, _)| *revision);

        Ok(records.into_iter().map(|(_, record)| record).collect())
    }
}

#[cfg(feature = "nats-adapter")]
fn nats_operation_journal_key(stream: &OperationJournalStream) -> String {
    format!(
        "operation.{}.{}",
        stream.operation_id.len(),
        encode_nats_key_token(&stream.operation_id)
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

fn validate_operation_record_stream(
    stream: &OperationJournalStream,
    record: &OperationJournalRecord,
) -> Result<(), OperationJournalError> {
    let actual_operation_id = record.operation_id();
    if stream.operation_id.as_str() != actual_operation_id {
        return Err(OperationJournalError::WrongOperationStream {
            expected_operation_id: stream.operation_id.clone(),
            actual_operation_id: actual_operation_id.to_string(),
        });
    }

    Ok(())
}

#[cfg(feature = "restate-adapter")]
async fn ensure_restate_success(
    response: reqwest::Response,
    stream: &OperationJournalStream,
    operation: &str,
) -> Result<reqwest::Response, OperationJournalError> {
    if response.status().is_success() {
        return Ok(response);
    }

    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|source| format!("failed to read Restate error response: {source}"));
    let reason = format!("Restate {operation} returned HTTP {status}: {body}");

    match operation {
        "append" => Err(OperationJournalError::RestateAppend {
            stream: stream.operation_id.clone(),
            reason,
        }),
        _ => Err(OperationJournalError::RestateLoad {
            stream: stream.operation_id.clone(),
            reason,
        }),
    }
}

#[cfg(feature = "restate-adapter")]
fn restate_terminal_error(error: OperationJournalError) -> RestateHandlerError {
    RestateTerminalError::new(error.to_string()).into()
}

#[cfg(feature = "restate-adapter")]
fn trim_trailing_slashes(mut value: String) -> String {
    while value.ends_with('/') {
        value.pop();
    }

    value
}

#[cfg(feature = "restate-adapter")]
fn encode_restate_path_segment(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char)
            }
            _ => {
                encoded.push('%');
                encoded.push(RESTATE_HEX[(byte >> 4) as usize] as char);
                encoded.push(RESTATE_HEX[(byte & 0x0F) as usize] as char);
            }
        }
    }

    encoded
}

#[cfg(all(test, feature = "nats-adapter"))]
mod nats_tests {
    use super::*;

    #[test]
    fn nats_operation_journal_key_leaves_plain_operation_ids_readable() {
        let stream = OperationJournalStream::for_operation("operation-123");

        assert_eq!(
            nats_operation_journal_key(&stream),
            "operation.13.operation-123"
        );
    }

    #[test]
    fn nats_operation_journal_key_escapes_key_token_separators_and_wildcards() {
        let stream = OperationJournalStream::for_operation("tenant.1/operation*>");
        let key = nats_operation_journal_key(&stream);
        let tokens: Vec<_> = key.split('.').collect();

        assert_eq!(
            tokens,
            vec!["operation", "20", "tenant%2E1%2Foperation%2A%3E"]
        );
        assert!(!tokens[2].contains('*'));
        assert!(!tokens[2].contains('>'));
    }

    #[test]
    fn nats_operation_journal_key_distinguishes_empty_operation_ids() {
        let empty_stream = OperationJournalStream::for_operation("");
        let underscore_stream = OperationJournalStream::for_operation("_");

        assert_eq!(nats_operation_journal_key(&empty_stream), "operation.0._");
        assert_eq!(
            nats_operation_journal_key(&underscore_stream),
            "operation.1._"
        );
    }
}

#[cfg(all(test, feature = "restate-adapter"))]
mod restate_tests {
    use super::*;

    #[test]
    fn restate_operation_journal_config_uses_default_object_name() {
        let config = RestateOperationJournalConfig::new("http://127.0.0.1:8080/");

        assert_eq!(config.base_url(), "http://127.0.0.1:8080");
        assert_eq!(config.object_name(), "ElbmeshOperationJournal");
    }

    #[test]
    fn restate_operation_journal_url_escapes_path_segments() {
        let journal = RestateOperationJournal::new(
            RestateOperationJournalConfig::new("http://127.0.0.1:8080/")
                .with_object_name("Journal/Object"),
        );
        let stream = OperationJournalStream::for_operation("tenant/operation 1");

        let url = journal
            .handler_url(&stream, "load")
            .expect("build Restate OperationJournal URL");

        assert_eq!(
            url.as_str(),
            "http://127.0.0.1:8080/Journal%2FObject/tenant%2Foperation%201/load"
        );
    }
}
