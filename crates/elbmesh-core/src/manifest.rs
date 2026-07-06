use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchitectureManifest {
    pub manifest_schema_id: String,
    pub manifest_schema_version: u32,
    pub resources: Vec<ResourceDefinition>,
    pub actions: Vec<ActionDefinition>,
    pub events: Vec<EventDefinition>,
    pub reactions: Vec<ReactionDefinition>,
    pub views: Vec<ViewDefinition>,
    pub queries: Vec<QueryDefinition>,
    pub external_operations: Vec<ExternalOperationDefinition>,
}

impl ArchitectureManifest {
    pub fn validate(&self) -> Result<(), ManifestValidationError> {
        let mut resource_types = HashSet::new();

        for resource in &self.resources {
            if !resource_types.insert(resource.resource_type.as_str()) {
                return Err(ManifestValidationError::DuplicateResourceType {
                    resource_type: resource.resource_type.clone(),
                });
            }
        }

        for action in &self.actions {
            if !resource_types.contains(action.resource_type.as_str()) {
                return Err(ManifestValidationError::UnknownActionResource {
                    action_type: action.action_type.clone(),
                    resource_type: action.resource_type.clone(),
                });
            }
        }

        for event in &self.events {
            if !resource_types.contains(event.resource_type.as_str()) {
                return Err(ManifestValidationError::UnknownEventResource {
                    event_type: event.event_type.clone(),
                    resource_type: event.resource_type.clone(),
                });
            }
        }

        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ManifestValidationError {
    #[error("manifest declares resource type '{resource_type}' more than once")]
    DuplicateResourceType { resource_type: String },

    #[error("manifest action '{action_type}' targets undeclared resource '{resource_type}'")]
    UnknownActionResource {
        action_type: String,
        resource_type: String,
    },

    #[error("manifest event '{event_type}' belongs to undeclared resource '{resource_type}'")]
    UnknownEventResource {
        event_type: String,
        resource_type: String,
    },
}

impl ManifestValidationError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::DuplicateResourceType { .. } => "manifest.duplicate_resource_type",
            Self::UnknownActionResource { .. } => "manifest.action_unknown_resource",
            Self::UnknownEventResource { .. } => "manifest.event_unknown_resource",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceDefinition {
    pub resource_type: String,
    pub schema_id: String,
    pub schema_version: u32,
    pub components: Vec<ComponentDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentDefinition {
    pub component_type: String,
    pub schema_id: String,
    pub schema_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionDefinition {
    pub action_type: String,
    pub resource_type: String,
    pub schema_id: String,
    pub schema_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventDefinition {
    pub event_type: String,
    pub resource_type: String,
    pub schema_id: String,
    pub schema_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactionDefinition {
    pub reaction_type: String,
    pub schema_id: String,
    pub schema_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewDefinition {
    pub view_type: String,
    pub schema_id: String,
    pub schema_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryDefinition {
    pub query_type: String,
    pub schema_id: String,
    pub schema_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalOperationDefinition {
    pub operation_type: String,
    pub schema_id: String,
    pub schema_version: u32,
}
