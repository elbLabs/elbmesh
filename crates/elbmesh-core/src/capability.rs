use serde::{Deserialize, Serialize};

use crate::{
    ActionDefinition, ArchitectureManifest, EventDefinition, ExternalOperationDefinition,
    QueryDefinition, ReactionDefinition, ResourceDefinition, ViewDefinition,
};

pub const CAPABILITY_SCHEMA_ID: &str = "capabilities.elbmesh.v1";
pub const CAPABILITY_SCHEMA_VERSION: u32 = 1;
pub const CAPABILITY_GENERATOR_NAME: &str = "elbmesh-core";
pub const CAPABILITY_GENERATOR_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityDocument {
    pub capability_schema_id: String,
    pub capability_schema_version: u32,
    pub generator: CapabilityGeneratorMetadata,
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

impl CapabilityDocument {
    pub fn from_manifest(manifest: &ArchitectureManifest) -> Self {
        Self {
            capability_schema_id: CAPABILITY_SCHEMA_ID.to_string(),
            capability_schema_version: CAPABILITY_SCHEMA_VERSION,
            generator: CapabilityGeneratorMetadata {
                name: CAPABILITY_GENERATOR_NAME.to_string(),
                version: CAPABILITY_GENERATOR_VERSION.to_string(),
            },
            manifest_schema_id: manifest.manifest_schema_id.clone(),
            manifest_schema_version: manifest.manifest_schema_version,
            resources: manifest.resources.clone(),
            actions: manifest.actions.clone(),
            events: manifest.events.clone(),
            reactions: manifest.reactions.clone(),
            views: manifest.views.clone(),
            queries: manifest.queries.clone(),
            external_operations: manifest.external_operations.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityGeneratorMetadata {
    pub name: String,
    pub version: String,
}
