use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceStream {
    pub resource_type: String,
    pub resource_id: String,
}

impl ResourceStream {
    pub fn new(resource_type: impl Into<String>, resource_id: impl Into<String>) -> Self {
        Self {
            resource_type: resource_type.into(),
            resource_id: resource_id.into(),
        }
    }

    pub fn key(&self) -> String {
        format!("resources.{}.{}", self.resource_type, self.resource_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamType {
    Resource,
    Action,
    Operation,
    Reaction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub message_id: String,
    pub message_type: String,
    pub message_version: u32,
    pub resource_type: String,
    pub resource_id: String,
    pub stream_type: StreamType,
    pub correlation_id: String,
    pub causation_id: String,
    pub action_id: String,
    pub actor_id: String,
    pub occurred_at: String,
    pub schema_id: String,
    pub schema_version: u32,
}

impl MessageMetadata {
    pub fn resource_event(
        message_type: impl Into<String>,
        schema_id: impl Into<String>,
        schema_version: u32,
        resource_type: impl Into<String>,
        resource_id: impl Into<String>,
        action: &ActionMetadata,
    ) -> Self {
        Self {
            message_id: Uuid::new_v4().to_string(),
            message_type: message_type.into(),
            message_version: schema_version,
            resource_type: resource_type.into(),
            resource_id: resource_id.into(),
            stream_type: StreamType::Resource,
            correlation_id: action.correlation_id.clone(),
            causation_id: action.causation_id.clone(),
            action_id: action.action_id.clone(),
            actor_id: action.actor_id.clone(),
            occurred_at: Utc::now().to_rfc3339(),
            schema_id: schema_id.into(),
            schema_version,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewEvent {
    pub metadata: MessageMetadata,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordedEvent {
    pub stream: ResourceStream,
    pub sequence: u64,
    pub metadata: MessageMetadata,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionMetadata {
    pub action_id: String,
    pub correlation_id: String,
    pub causation_id: String,
    pub actor_id: String,
}

impl ActionMetadata {
    pub fn for_actor(actor_id: impl Into<String>) -> Self {
        let action_id = Uuid::new_v4().to_string();

        Self {
            correlation_id: action_id.clone(),
            causation_id: action_id.clone(),
            action_id,
            actor_id: actor_id.into(),
        }
    }

    pub fn with_ids(
        action_id: impl Into<String>,
        correlation_id: impl Into<String>,
        causation_id: impl Into<String>,
        actor_id: impl Into<String>,
    ) -> Self {
        Self {
            action_id: action_id.into(),
            correlation_id: correlation_id.into(),
            causation_id: causation_id.into(),
            actor_id: actor_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmittedEvent {
    pub message_id: String,
    pub message_type: String,
    pub schema_id: String,
    pub schema_version: u32,
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionReceipt {
    pub action_id: String,
    pub status: ActionStatus,
    pub resource_type: String,
    pub resource_id: String,
    pub previous_version: u64,
    pub new_version: u64,
    pub emitted_events: Vec<EmittedEvent>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActionDecision {
    pub message: Option<String>,
}

impl ActionDecision {
    pub fn completed() -> Self {
        Self { message: None }
    }

    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            message: Some(message.into()),
        }
    }
}
