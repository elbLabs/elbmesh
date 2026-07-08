use serde::{Deserialize, Serialize};

use crate::{
    ActionDefinition, ArchitectureManifest, ComponentDefinition, EventDefinition,
    ExternalOperationDefinition, QueryDefinition, ReactionDefinition, ResourceDefinition,
    ViewDefinition,
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

    pub fn to_markdown(&self) -> String {
        let mut markdown = String::new();

        markdown.push_str("# Capability Document\n\n");
        markdown.push_str(&format!(
            "- Capability schema: `{}` v{}\n",
            self.capability_schema_id, self.capability_schema_version
        ));
        markdown.push_str(&format!(
            "- Manifest schema: `{}` v{}\n",
            self.manifest_schema_id, self.manifest_schema_version
        ));
        markdown.push_str(&format!(
            "- Generator: `{}` v{}\n\n",
            self.generator.name, self.generator.version
        ));

        markdown.push_str("## Runtime Boundaries\n\n");
        markdown.push_str(
            "This document describes declared capabilities and implemented framework boundaries only.\n",
        );
        markdown.push_str(
            "It does not imply real Restate adapter support or complete NATS adapter coverage.\n",
        );
        markdown.push_str("Resource Events remain separate from ActionJournal, ReactionJournal, OperationJournal, ViewStore, provider diagnostics, and generated visibility artifacts.\n\n");

        markdown.push_str("## Resources\n\n");
        markdown.push_str("| Resource | Schema | Version | Components |\n");
        markdown.push_str("| --- | --- | --- | --- |\n");
        for resource in &self.resources {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                code(&resource.resource_type),
                code(&resource.schema_id),
                resource.schema_version,
                format_components(&resource.components)
            ));
        }
        markdown.push('\n');

        markdown.push_str("## Actions\n\n");
        markdown
            .push_str("| Action | Resource | Schema | Version | Emits | External Operations |\n");
        markdown.push_str("| --- | --- | --- | --- | --- | --- |\n");
        for action in &self.actions {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                code(&action.action_type),
                code(&action.resource_type),
                code(&action.schema_id),
                action.schema_version,
                format_code_list(&action.emitted_event_types),
                format_code_list(&action.external_operation_types)
            ));
        }
        markdown.push('\n');

        markdown.push_str("## Events\n\n");
        markdown.push_str("| Event | Resource | Schema | Version |\n");
        markdown.push_str("| --- | --- | --- | --- |\n");
        for event in &self.events {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                code(&event.event_type),
                code(&event.resource_type),
                code(&event.schema_id),
                event.schema_version
            ));
        }
        markdown.push('\n');

        markdown.push_str("## Reactions\n\n");
        markdown.push_str("| Reaction | Trigger Event | Target Action | Schema | Version |\n");
        markdown.push_str("| --- | --- | --- | --- | --- |\n");
        for reaction in &self.reactions {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                code(&reaction.reaction_type),
                code(&reaction.trigger_event_type),
                code(&reaction.target_action_type),
                code(&reaction.schema_id),
                reaction.schema_version
            ));
        }
        markdown.push('\n');

        markdown.push_str("## Views\n\n");
        markdown.push_str("| View | Schema | Version |\n");
        markdown.push_str("| --- | --- | --- |\n");
        for view in &self.views {
            markdown.push_str(&format!(
                "| {} | {} | {} |\n",
                code(&view.view_type),
                code(&view.schema_id),
                view.schema_version
            ));
        }
        markdown.push('\n');

        markdown.push_str("## Queries\n\n");
        markdown.push_str("| Query | Schema | Version |\n");
        markdown.push_str("| --- | --- | --- |\n");
        for query in &self.queries {
            markdown.push_str(&format!(
                "| {} | {} | {} |\n",
                code(&query.query_type),
                code(&query.schema_id),
                query.schema_version
            ));
        }
        markdown.push('\n');

        markdown.push_str("## External Operations\n\n");
        markdown.push_str("External Operations use idempotency keys and OperationJournal records for call/completion/failure boundaries. Provider diagnostics are not Resource Events.\n\n");
        markdown.push_str("| External Operation | Schema | Version |\n");
        markdown.push_str("| --- | --- | --- |\n");
        for operation in &self.external_operations {
            markdown.push_str(&format!(
                "| {} | {} | {} |\n",
                code(&operation.operation_type),
                code(&operation.schema_id),
                operation.schema_version
            ));
        }

        markdown
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityGeneratorMetadata {
    pub name: String,
    pub version: String,
}

fn code(value: &str) -> String {
    format!("`{value}`")
}

fn format_code_list(values: &[String]) -> String {
    if values.is_empty() {
        return "None".to_string();
    }

    values
        .iter()
        .map(|value| code(value))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_components(components: &[ComponentDefinition]) -> String {
    if components.is_empty() {
        return "None".to_string();
    }

    components
        .iter()
        .map(|component| {
            format!(
                "{} ({} v{})",
                code(&component.component_type),
                code(&component.schema_id),
                component.schema_version
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}
