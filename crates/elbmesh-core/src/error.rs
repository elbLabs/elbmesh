use thiserror::Error;

use crate::{
    action_journal::ActionJournalError, external_operation::ExternalOperationFailure,
    message::StreamType, operation_journal::OperationJournalError,
};

pub trait ActionFailure: std::fmt::Debug + std::fmt::Display + Send + Sync + 'static {
    fn code(&self) -> &'static str;

    fn details(&self) -> serde_json::Value {
        serde_json::json!({ "code": self.code() })
    }
}

#[derive(Debug, Error)]
pub enum ActionError {
    #[error("action rejected: {reason}")]
    Rejected { reason: String },

    #[error("validation failed: {reason}")]
    Validation { reason: String },

    #[error("external operation '{operation_type}' failed with {failure_code}")]
    ExternalOperation {
        operation_type: String,
        failure_code: String,
        failure_details: serde_json::Value,
    },

    #[error("operation journal for operation '{operation_id}' failed with {failure_code}")]
    OperationJournal {
        operation_id: String,
        failure_code: String,
        failure_details: serde_json::Value,
    },

    #[error("state transition failed: {reason}")]
    StateTransition { reason: String },

    #[error("failed to serialize action output: {0}")]
    Serialization(String),

    #[error("event targets resource '{actual}', but action targets resource '{expected}'")]
    WrongResource { expected: String, actual: String },

    #[error("action failed: {0}")]
    Other(String),
}

impl ActionFailure for ActionError {
    fn code(&self) -> &'static str {
        match self {
            Self::Rejected { .. } => "action.rejected",
            Self::Validation { .. } => "action.validation",
            Self::ExternalOperation { .. } => "action.external_operation",
            Self::OperationJournal { .. } => "action.operation_journal",
            Self::StateTransition { .. } => "action.state_transition",
            Self::Serialization(_) => "action.serialization",
            Self::WrongResource { .. } => "action.wrong_resource",
            Self::Other(_) => "action.other",
        }
    }

    fn details(&self) -> serde_json::Value {
        match self {
            Self::Rejected { reason } => serde_json::json!({
                "error_type": "ActionError",
                "error_variant": "Rejected",
                "reason": reason,
            }),
            Self::Validation { reason } => serde_json::json!({
                "error_type": "ActionError",
                "error_variant": "Validation",
                "reason": reason,
            }),
            Self::ExternalOperation {
                operation_type,
                failure_code,
                failure_details,
            } => serde_json::json!({
                "error_type": "ActionError",
                "error_variant": "ExternalOperation",
                "operation_type": operation_type,
                "failure_code": failure_code,
                "failure_details": failure_details,
            }),
            Self::OperationJournal {
                operation_id,
                failure_code,
                failure_details,
            } => serde_json::json!({
                "error_type": "ActionError",
                "error_variant": "OperationJournal",
                "operation_id": operation_id,
                "failure_code": failure_code,
                "failure_details": failure_details,
            }),
            Self::StateTransition { reason } => serde_json::json!({
                "error_type": "ActionError",
                "error_variant": "StateTransition",
                "reason": reason,
            }),
            Self::Serialization(reason) => serde_json::json!({
                "error_type": "ActionError",
                "error_variant": "Serialization",
                "reason": reason,
            }),
            Self::WrongResource { expected, actual } => serde_json::json!({
                "error_type": "ActionError",
                "error_variant": "WrongResource",
                "expected": expected,
                "actual": actual,
            }),
            Self::Other(reason) => serde_json::json!({
                "error_type": "ActionError",
                "error_variant": "Other",
                "reason": reason,
            }),
        }
    }
}

#[derive(Debug, Error)]
pub enum HandlerError<E>
where
    E: ActionFailure,
{
    #[error("domain error {code}: {error}", code = error.code())]
    Domain { error: E },

    #[error(transparent)]
    Runtime(#[from] ActionError),
}

impl<E> HandlerError<E>
where
    E: ActionFailure,
{
    pub fn domain(error: E) -> Self {
        Self::Domain { error }
    }
}

impl ActionError {
    pub fn rejected(reason: impl Into<String>) -> Self {
        Self::Rejected {
            reason: reason.into(),
        }
    }

    pub fn validation(reason: impl Into<String>) -> Self {
        Self::Validation {
            reason: reason.into(),
        }
    }

    pub fn external_operation<E>(operation_type: impl Into<String>, error: &E) -> Self
    where
        E: ExternalOperationFailure,
    {
        Self::ExternalOperation {
            operation_type: operation_type.into(),
            failure_code: error.code().to_string(),
            failure_details: error.details(),
        }
    }

    pub fn operation_journal(
        operation_id: impl Into<String>,
        error: &OperationJournalError,
    ) -> Self {
        Self::OperationJournal {
            operation_id: operation_id.into(),
            failure_code: error.code().to_string(),
            failure_details: error.details(),
        }
    }

    pub fn state_transition(reason: impl Into<String>) -> Self {
        Self::StateTransition {
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ResourceError {
    #[error(
        "resource '{resource_type}' does not support event '{message_type}' v{schema_version}"
    )]
    UnsupportedEvent {
        resource_type: String,
        message_type: String,
        schema_version: u32,
    },

    #[error("failed to deserialize event '{message_type}' v{schema_version}: {source}")]
    Deserialization {
        message_type: String,
        schema_version: u32,
        source: serde_json::Error,
    },

    #[error("failed to apply event: {0}")]
    Apply(String),
}

impl ResourceError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedEvent { .. } => "resource.unsupported_event",
            Self::Deserialization { .. } => "resource.deserialization",
            Self::Apply(_) => "resource.apply",
        }
    }

    pub(crate) fn details(&self) -> serde_json::Value {
        match self {
            Self::UnsupportedEvent {
                resource_type,
                message_type,
                schema_version,
            } => serde_json::json!({
                "error_type": "ResourceError",
                "error_variant": "UnsupportedEvent",
                "resource_type": resource_type,
                "message_type": message_type,
                "schema_version": schema_version,
            }),
            Self::Deserialization {
                message_type,
                schema_version,
                source,
            } => serde_json::json!({
                "error_type": "ResourceError",
                "error_variant": "Deserialization",
                "message_type": message_type,
                "schema_version": schema_version,
                "source": source.to_string(),
            }),
            Self::Apply(reason) => serde_json::json!({
                "error_type": "ResourceError",
                "error_variant": "Apply",
                "reason": reason,
            }),
        }
    }
}

#[derive(Debug, Error)]
pub enum EventStoreError {
    #[error("concurrency conflict on stream '{stream}': expected version {expected}, actual version {actual}")]
    ConcurrencyConflict {
        stream: String,
        expected: u64,
        actual: u64,
    },

    #[error("event targets resource '{actual_resource_type}/{actual_resource_id}', but stream '{stream}' is for resource '{expected_resource_type}/{expected_resource_id}'")]
    WrongEventStream {
        stream: String,
        expected_resource_type: String,
        expected_resource_id: String,
        actual_resource_type: String,
        actual_resource_id: String,
    },

    #[error("event targets stream type '{actual_stream_type:?}', but stream '{stream}' requires '{expected_stream_type:?}'")]
    WrongEventStreamType {
        stream: String,
        expected_stream_type: StreamType,
        actual_stream_type: StreamType,
    },

    #[error("failed to connect NATS EventStore: {reason}")]
    NatsConnect { reason: String },

    #[error("failed to open NATS EventStore bucket '{bucket}': {reason}")]
    NatsBucket { bucket: String, reason: String },

    #[error("failed to serialize event stream '{stream}': {reason}")]
    StreamSerialization { stream: String, reason: String },

    #[error("failed to deserialize event stream '{stream}' revision {revision}: {reason}")]
    StreamDeserialization {
        stream: String,
        revision: u64,
        reason: String,
    },

    #[error("failed to load event stream '{stream}' from NATS: {reason}")]
    NatsLoad { stream: String, reason: String },

    #[error("failed to append event stream '{stream}' to NATS: {reason}")]
    NatsAppend { stream: String, reason: String },

    #[error("event store failed: {0}")]
    Other(String),
}

impl EventStoreError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::ConcurrencyConflict { .. } => "event_store.concurrency_conflict",
            Self::WrongEventStream { .. } => "event_store.wrong_event_stream",
            Self::WrongEventStreamType { .. } => "event_store.wrong_event_stream_type",
            Self::NatsConnect { .. } => "event_store.nats_connect",
            Self::NatsBucket { .. } => "event_store.nats_bucket",
            Self::StreamSerialization { .. } => "event_store.stream_serialization",
            Self::StreamDeserialization { .. } => "event_store.stream_deserialization",
            Self::NatsLoad { .. } => "event_store.nats_load",
            Self::NatsAppend { .. } => "event_store.nats_append",
            Self::Other(_) => "event_store.other",
        }
    }

    pub(crate) fn details(&self) -> serde_json::Value {
        match self {
            Self::ConcurrencyConflict {
                stream,
                expected,
                actual,
            } => serde_json::json!({
                "error_type": "EventStoreError",
                "error_variant": "ConcurrencyConflict",
                "stream": stream,
                "expected": expected,
                "actual": actual,
            }),
            Self::WrongEventStream {
                stream,
                expected_resource_type,
                expected_resource_id,
                actual_resource_type,
                actual_resource_id,
            } => serde_json::json!({
                "error_type": "EventStoreError",
                "error_variant": "WrongEventStream",
                "stream": stream,
                "expected_resource_type": expected_resource_type,
                "expected_resource_id": expected_resource_id,
                "actual_resource_type": actual_resource_type,
                "actual_resource_id": actual_resource_id,
            }),
            Self::WrongEventStreamType {
                stream,
                expected_stream_type,
                actual_stream_type,
            } => serde_json::json!({
                "error_type": "EventStoreError",
                "error_variant": "WrongEventStreamType",
                "stream": stream,
                "expected_stream_type": expected_stream_type,
                "actual_stream_type": actual_stream_type,
            }),
            Self::NatsConnect { reason } => serde_json::json!({
                "error_type": "EventStoreError",
                "error_variant": "NatsConnect",
                "reason": reason,
            }),
            Self::NatsBucket { bucket, reason } => serde_json::json!({
                "error_type": "EventStoreError",
                "error_variant": "NatsBucket",
                "bucket": bucket,
                "reason": reason,
            }),
            Self::StreamSerialization { stream, reason } => serde_json::json!({
                "error_type": "EventStoreError",
                "error_variant": "StreamSerialization",
                "stream": stream,
                "reason": reason,
            }),
            Self::StreamDeserialization {
                stream,
                revision,
                reason,
            } => serde_json::json!({
                "error_type": "EventStoreError",
                "error_variant": "StreamDeserialization",
                "stream": stream,
                "revision": revision,
                "reason": reason,
            }),
            Self::NatsLoad { stream, reason } => serde_json::json!({
                "error_type": "EventStoreError",
                "error_variant": "NatsLoad",
                "stream": stream,
                "reason": reason,
            }),
            Self::NatsAppend { stream, reason } => serde_json::json!({
                "error_type": "EventStoreError",
                "error_variant": "NatsAppend",
                "stream": stream,
                "reason": reason,
            }),
            Self::Other(reason) => serde_json::json!({
                "error_type": "EventStoreError",
                "error_variant": "Other",
                "reason": reason,
            }),
        }
    }
}

#[derive(Debug, Error)]
pub enum ExecutionError<E>
where
    E: ActionFailure,
{
    #[error(transparent)]
    Handler(#[from] HandlerError<E>),

    #[error(transparent)]
    Resource(#[from] ResourceError),

    #[error(transparent)]
    EventStore(#[from] EventStoreError),

    #[error(transparent)]
    ActionJournal(#[from] ActionJournalError),
}
