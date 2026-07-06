use thiserror::Error;

use crate::action_journal::ActionJournalError;

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

    #[error("external operation failed: {reason}")]
    ExternalOperation { reason: String },

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
            Self::StateTransition { .. } => "action.state_transition",
            Self::Serialization(_) => "action.serialization",
            Self::WrongResource { .. } => "action.wrong_resource",
            Self::Other(_) => "action.other",
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
