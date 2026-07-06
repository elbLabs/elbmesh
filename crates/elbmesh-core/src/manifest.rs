use serde::{Deserialize, Serialize};

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
