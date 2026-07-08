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

    pub fn to_rust_binding_stubs(&self) -> String {
        let mut rust = String::new();

        rust.push_str(&format!(
            "// @generated by {} v{}\n",
            self.generator.name, self.generator.version
        ));
        rust.push_str(&format!(
            "// capability_schema_id: {} v{}\n",
            self.capability_schema_id, self.capability_schema_version
        ));
        rust.push_str(&format!(
            "// manifest_schema_id: {} v{}\n",
            self.manifest_schema_id, self.manifest_schema_version
        ));
        rust.push_str("// Binding stubs declare types and schema constants only.\n");
        rust.push_str("// Behavior, provider registration, NATS adapters, and Restate execution are not generated here.\n\n");

        rust.push_str("pub mod resources {\n");
        let mut wrote_resource_stub = false;
        for resource in &self.resources {
            push_stub_separator(&mut rust, &mut wrote_resource_stub);
            push_rust_binding_type(
                &mut rust,
                &rust_type_name(&resource.resource_type, ""),
                &[
                    rust_str_constant("RESOURCE_TYPE", &resource.resource_type),
                    rust_str_constant("SCHEMA_ID", &resource.schema_id),
                    rust_u32_constant("SCHEMA_VERSION", resource.schema_version),
                ],
            );

            for component in &resource.components {
                push_stub_separator(&mut rust, &mut wrote_resource_stub);
                push_rust_binding_type(
                    &mut rust,
                    &rust_type_name(&component.component_type, "Component"),
                    &[
                        rust_str_constant("COMPONENT_TYPE", &component.component_type),
                        rust_str_constant("SCHEMA_ID", &component.schema_id),
                        rust_u32_constant("SCHEMA_VERSION", component.schema_version),
                    ],
                );
            }
        }
        rust.push_str("}\n\n");

        rust.push_str("pub mod actions {\n");
        let mut wrote_action_stub = false;
        for action in &self.actions {
            push_stub_separator(&mut rust, &mut wrote_action_stub);
            push_rust_binding_type(
                &mut rust,
                &rust_type_name(&action.action_type, "Action"),
                &[
                    rust_str_constant("ACTION_TYPE", &action.action_type),
                    rust_str_constant("RESOURCE_TYPE", &action.resource_type),
                    rust_str_constant("SCHEMA_ID", &action.schema_id),
                    rust_u32_constant("SCHEMA_VERSION", action.schema_version),
                    rust_str_slice_constant("EMITTED_EVENT_TYPES", &action.emitted_event_types),
                    rust_str_slice_constant(
                        "EXTERNAL_OPERATION_TYPES",
                        &action.external_operation_types,
                    ),
                ],
            );
        }
        rust.push_str("}\n\n");

        rust.push_str("pub mod events {\n");
        let mut wrote_event_stub = false;
        for event in &self.events {
            push_stub_separator(&mut rust, &mut wrote_event_stub);
            push_rust_binding_type(
                &mut rust,
                &rust_type_name(&event.event_type, "Event"),
                &[
                    rust_str_constant("EVENT_TYPE", &event.event_type),
                    rust_str_constant("RESOURCE_TYPE", &event.resource_type),
                    rust_str_constant("SCHEMA_ID", &event.schema_id),
                    rust_u32_constant("SCHEMA_VERSION", event.schema_version),
                ],
            );
        }
        rust.push_str("}\n\n");

        rust.push_str("pub mod reactions {\n");
        let mut wrote_reaction_stub = false;
        for reaction in &self.reactions {
            push_stub_separator(&mut rust, &mut wrote_reaction_stub);
            push_rust_binding_type(
                &mut rust,
                &rust_type_name(&reaction.reaction_type, "Reaction"),
                &[
                    rust_str_constant("REACTION_TYPE", &reaction.reaction_type),
                    rust_str_constant("TRIGGER_EVENT_TYPE", &reaction.trigger_event_type),
                    rust_str_constant("TARGET_ACTION_TYPE", &reaction.target_action_type),
                    rust_str_constant("SCHEMA_ID", &reaction.schema_id),
                    rust_u32_constant("SCHEMA_VERSION", reaction.schema_version),
                ],
            );
        }
        rust.push_str("}\n\n");

        rust.push_str("pub mod views {\n");
        let mut wrote_view_stub = false;
        for view in &self.views {
            push_stub_separator(&mut rust, &mut wrote_view_stub);
            push_rust_binding_type(
                &mut rust,
                &rust_type_name(&view.view_type, "View"),
                &[
                    rust_str_constant("VIEW_TYPE", &view.view_type),
                    rust_str_constant("SCHEMA_ID", &view.schema_id),
                    rust_u32_constant("SCHEMA_VERSION", view.schema_version),
                ],
            );
        }
        rust.push_str("}\n\n");

        rust.push_str("pub mod queries {\n");
        let mut wrote_query_stub = false;
        for query in &self.queries {
            push_stub_separator(&mut rust, &mut wrote_query_stub);
            push_rust_binding_type(
                &mut rust,
                &rust_type_name(&query.query_type, "Query"),
                &[
                    rust_str_constant("QUERY_TYPE", &query.query_type),
                    rust_str_constant("SCHEMA_ID", &query.schema_id),
                    rust_u32_constant("SCHEMA_VERSION", query.schema_version),
                ],
            );
        }
        rust.push_str("}\n\n");

        rust.push_str("pub mod external_operations {\n");
        let mut wrote_operation_stub = false;
        for operation in &self.external_operations {
            push_stub_separator(&mut rust, &mut wrote_operation_stub);
            push_rust_binding_type(
                &mut rust,
                &rust_type_name(&operation.operation_type, "ExternalOperation"),
                &[
                    rust_str_constant("OPERATION_TYPE", &operation.operation_type),
                    rust_str_constant("SCHEMA_ID", &operation.schema_id),
                    rust_u32_constant("SCHEMA_VERSION", operation.schema_version),
                ],
            );
        }
        rust.push_str("}\n");

        rust
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityGeneratorMetadata {
    pub name: String,
    pub version: String,
}

struct RustBindingConstant {
    name: &'static str,
    type_name: &'static str,
    value: String,
}

fn push_stub_separator(rust: &mut String, wrote_stub: &mut bool) {
    if *wrote_stub {
        rust.push('\n');
    }

    *wrote_stub = true;
}

fn push_rust_binding_type(rust: &mut String, type_name: &str, constants: &[RustBindingConstant]) {
    rust.push_str("    #[derive(Debug, Clone, PartialEq, Eq)]\n");
    rust.push_str(&format!("    pub struct {type_name};\n\n"));
    rust.push_str(&format!("    impl {type_name} {{\n"));
    for constant in constants {
        rust.push_str(&format!(
            "        pub const {}: {} = {};\n",
            constant.name, constant.type_name, constant.value
        ));
    }
    rust.push_str("    }\n");
}

fn rust_str_constant(name: &'static str, value: &str) -> RustBindingConstant {
    RustBindingConstant {
        name,
        type_name: "&'static str",
        value: rust_string_literal(value),
    }
}

fn rust_u32_constant(name: &'static str, value: u32) -> RustBindingConstant {
    RustBindingConstant {
        name,
        type_name: "u32",
        value: value.to_string(),
    }
}

fn rust_str_slice_constant(name: &'static str, values: &[String]) -> RustBindingConstant {
    RustBindingConstant {
        name,
        type_name: "&'static [&'static str]",
        value: rust_string_slice(values),
    }
}

fn rust_string_slice(values: &[String]) -> String {
    if values.is_empty() {
        return "&[]".to_string();
    }

    format!(
        "&[{}]",
        values
            .iter()
            .map(|value| rust_string_literal(value))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn rust_string_literal(value: &str) -> String {
    format!("{value:?}")
}

fn rust_type_name(value: &str, suffix: &str) -> String {
    let mut type_name = String::new();

    for part in value.split(|character: char| !character.is_ascii_alphanumeric()) {
        if part.is_empty() {
            continue;
        }

        let mut characters = part.chars();
        if let Some(first) = characters.next() {
            type_name.push(first.to_ascii_uppercase());
            for character in characters {
                type_name.push(character.to_ascii_lowercase());
            }
        }
    }

    if type_name.is_empty()
        || type_name
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_digit())
    {
        type_name.insert_str(0, "Generated");
    }

    type_name.push_str(suffix);
    type_name
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
