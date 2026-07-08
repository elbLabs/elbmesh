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

    #[error("event store failed: {0}")]
    Other(String),
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
