use elbmesh_core::{
    ActionDefinition, ArchitectureManifest, CapabilityDocument, ComponentDefinition,
    EventDefinition, ExternalOperationDefinition, QueryDefinition, ReactionDefinition,
    ResourceDefinition, ViewDefinition,
};

use serde_json::json;

#[test]
fn capability_document_projects_manifest_identity_and_generator_metadata() {
    let manifest = offer_manifest();
    let capabilities = CapabilityDocument::from_manifest(&manifest);

    assert_eq!(capabilities.capability_schema_id, "capabilities.elbmesh.v1");
    assert_eq!(capabilities.capability_schema_version, 1);
    assert_eq!(capabilities.generator.name, "elbmesh-core");
    assert_eq!(capabilities.generator.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(capabilities.manifest_schema_id, manifest.manifest_schema_id);
    assert_eq!(
        capabilities.manifest_schema_version,
        manifest.manifest_schema_version
    );
}

#[test]
fn capability_document_preserves_manifest_declared_capabilities() {
    let capabilities = CapabilityDocument::from_manifest(&offer_manifest());

    assert_eq!(capabilities.resources[0].resource_type, "offer");
    assert_eq!(capabilities.resources[0].schema_id, "resource.offer.v1");
    assert_eq!(
        capabilities.resources[0].components[0].component_type,
        "offer_terms"
    );
    assert_eq!(capabilities.actions[0].action_type, "send_offer_email");
    assert_eq!(capabilities.actions[0].resource_type, "offer");
    assert_eq!(
        capabilities.actions[0].external_operation_types,
        vec!["lexoffice_create_invoice"]
    );
    assert_eq!(capabilities.events[0].event_type, "offer_created");
    assert_eq!(
        capabilities.reactions[0].trigger_event_type,
        "offer_created"
    );
    assert_eq!(capabilities.views[0].view_type, "offer_summary");
    assert_eq!(capabilities.queries[0].query_type, "get_offer_summary");
    assert_eq!(
        capabilities.external_operations[0].operation_type,
        "lexoffice_create_invoice"
    );
}

#[test]
fn capability_document_serializes_to_stable_json_shape() {
    let capabilities = CapabilityDocument::from_manifest(&offer_manifest());
    let encoded = serde_json::to_value(&capabilities).expect("serialize capabilities");

    assert_eq!(
        encoded,
        json!({
            "capability_schema_id": "capabilities.elbmesh.v1",
            "capability_schema_version": 1,
            "generator": {
                "name": "elbmesh-core",
                "version": env!("CARGO_PKG_VERSION"),
            },
            "manifest_schema_id": "manifest.elbmesh.v1",
            "manifest_schema_version": 1,
            "resources": [{
                "resource_type": "offer",
                "schema_id": "resource.offer.v1",
                "schema_version": 1,
                "components": [{
                    "component_type": "offer_terms",
                    "schema_id": "component.offer_terms.v1",
                    "schema_version": 1,
                }],
            }],
            "actions": [{
                "action_type": "send_offer_email",
                "resource_type": "offer",
                "schema_id": "action.send_offer_email.v1",
                "schema_version": 1,
                "emitted_event_types": ["offer_created"],
                "external_operation_types": ["lexoffice_create_invoice"],
            }],
            "events": [{
                "event_type": "offer_created",
                "resource_type": "offer",
                "schema_id": "event.offer_created.v1",
                "schema_version": 1,
            }],
            "reactions": [{
                "reaction_type": "offer_created_to_send_offer_email",
                "trigger_event_type": "offer_created",
                "target_action_type": "send_offer_email",
                "schema_id": "reaction.offer_created_to_send_offer_email.v1",
                "schema_version": 1,
            }],
            "views": [{
                "view_type": "offer_summary",
                "schema_id": "view.offer_summary.v1",
                "schema_version": 1,
            }],
            "queries": [{
                "query_type": "get_offer_summary",
                "schema_id": "query.get_offer_summary.v1",
                "schema_version": 1,
            }],
            "external_operations": [{
                "operation_type": "lexoffice_create_invoice",
                "schema_id": "external_operation.lexoffice_create_invoice.v1",
                "schema_version": 1,
            }],
        })
    );
}

#[test]
fn capability_document_renders_stable_markdown_shape() {
    let capabilities = CapabilityDocument::from_manifest(&offer_manifest());

    assert_eq!(
        capabilities.to_markdown(),
        format!(
            r#"# Capability Document

- Capability schema: `capabilities.elbmesh.v1` v1
- Manifest schema: `manifest.elbmesh.v1` v1
- Generator: `elbmesh-core` v{version}

## Runtime Boundaries

This document describes declared capabilities and implemented framework boundaries only.
It does not imply real Restate adapter support or complete NATS adapter coverage.
Resource Events remain separate from ActionJournal, ReactionJournal, OperationJournal, ViewStore, provider diagnostics, and generated visibility artifacts.

## Resources

| Resource | Schema | Version | Components |
| --- | --- | --- | --- |
| `offer` | `resource.offer.v1` | 1 | `offer_terms` (`component.offer_terms.v1` v1) |

## Actions

| Action | Resource | Schema | Version | Emits | External Operations |
| --- | --- | --- | --- | --- | --- |
| `send_offer_email` | `offer` | `action.send_offer_email.v1` | 1 | `offer_created` | `lexoffice_create_invoice` |

## Events

| Event | Resource | Schema | Version |
| --- | --- | --- | --- |
| `offer_created` | `offer` | `event.offer_created.v1` | 1 |

## Reactions

| Reaction | Trigger Event | Target Action | Schema | Version |
| --- | --- | --- | --- | --- |
| `offer_created_to_send_offer_email` | `offer_created` | `send_offer_email` | `reaction.offer_created_to_send_offer_email.v1` | 1 |

## Views

| View | Schema | Version |
| --- | --- | --- |
| `offer_summary` | `view.offer_summary.v1` | 1 |

## Queries

| Query | Schema | Version |
| --- | --- | --- |
| `get_offer_summary` | `query.get_offer_summary.v1` | 1 |

## External Operations

External Operations use idempotency keys and OperationJournal records for call/completion/failure boundaries. Provider diagnostics are not Resource Events.

| External Operation | Schema | Version |
| --- | --- | --- |
| `lexoffice_create_invoice` | `external_operation.lexoffice_create_invoice.v1` | 1 |
"#,
            version = env!("CARGO_PKG_VERSION")
        )
    );
}

#[test]
fn capability_markdown_states_runtime_boundaries() {
    let markdown = CapabilityDocument::from_manifest(&offer_manifest()).to_markdown();

    assert!(markdown.contains(
        "It does not imply real Restate adapter support or complete NATS adapter coverage."
    ));
    assert!(markdown.contains(
        "Resource Events remain separate from ActionJournal, ReactionJournal, OperationJournal, ViewStore, provider diagnostics, and generated visibility artifacts."
    ));
}

#[test]
fn capability_markdown_documents_external_operation_recovery_boundary() {
    let markdown = CapabilityDocument::from_manifest(&offer_manifest()).to_markdown();

    assert!(markdown.contains(
        "External Operations use idempotency keys and OperationJournal records for call/completion/failure boundaries."
    ));
    assert!(markdown.contains("Provider diagnostics are not Resource Events."));
}

#[test]
fn capability_document_renders_stable_rust_binding_stub_shape() {
    let capabilities = CapabilityDocument::from_manifest(&offer_manifest());

    assert_eq!(
        capabilities.to_rust_binding_stubs(),
        format!(
            r#"// @generated by elbmesh-core v{version}
// capability_schema_id: capabilities.elbmesh.v1 v1
// manifest_schema_id: manifest.elbmesh.v1 v1
// Binding stubs declare types and schema constants only.
// Behavior, provider registration, NATS adapters, and Restate execution are not generated here.

pub mod resources {{
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Offer;

    impl Offer {{
        pub const RESOURCE_TYPE: &'static str = "offer";
        pub const SCHEMA_ID: &'static str = "resource.offer.v1";
        pub const SCHEMA_VERSION: u32 = 1;
    }}

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct OfferTermsComponent;

    impl OfferTermsComponent {{
        pub const COMPONENT_TYPE: &'static str = "offer_terms";
        pub const SCHEMA_ID: &'static str = "component.offer_terms.v1";
        pub const SCHEMA_VERSION: u32 = 1;
    }}
}}

pub mod actions {{
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SendOfferEmailAction;

    impl SendOfferEmailAction {{
        pub const ACTION_TYPE: &'static str = "send_offer_email";
        pub const RESOURCE_TYPE: &'static str = "offer";
        pub const SCHEMA_ID: &'static str = "action.send_offer_email.v1";
        pub const SCHEMA_VERSION: u32 = 1;
        pub const EMITTED_EVENT_TYPES: &'static [&'static str] = &["offer_created"];
        pub const EXTERNAL_OPERATION_TYPES: &'static [&'static str] = &["lexoffice_create_invoice"];
    }}
}}

pub mod events {{
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct OfferCreatedEvent;

    impl OfferCreatedEvent {{
        pub const EVENT_TYPE: &'static str = "offer_created";
        pub const RESOURCE_TYPE: &'static str = "offer";
        pub const SCHEMA_ID: &'static str = "event.offer_created.v1";
        pub const SCHEMA_VERSION: u32 = 1;
    }}
}}

pub mod reactions {{
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct OfferCreatedToSendOfferEmailReaction;

    impl OfferCreatedToSendOfferEmailReaction {{
        pub const REACTION_TYPE: &'static str = "offer_created_to_send_offer_email";
        pub const TRIGGER_EVENT_TYPE: &'static str = "offer_created";
        pub const TARGET_ACTION_TYPE: &'static str = "send_offer_email";
        pub const SCHEMA_ID: &'static str = "reaction.offer_created_to_send_offer_email.v1";
        pub const SCHEMA_VERSION: u32 = 1;
    }}
}}

pub mod views {{
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct OfferSummaryView;

    impl OfferSummaryView {{
        pub const VIEW_TYPE: &'static str = "offer_summary";
        pub const SCHEMA_ID: &'static str = "view.offer_summary.v1";
        pub const SCHEMA_VERSION: u32 = 1;
    }}
}}

pub mod queries {{
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct GetOfferSummaryQuery;

    impl GetOfferSummaryQuery {{
        pub const QUERY_TYPE: &'static str = "get_offer_summary";
        pub const SCHEMA_ID: &'static str = "query.get_offer_summary.v1";
        pub const SCHEMA_VERSION: u32 = 1;
    }}
}}

pub mod external_operations {{
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct LexofficeCreateInvoiceExternalOperation;

    impl LexofficeCreateInvoiceExternalOperation {{
        pub const OPERATION_TYPE: &'static str = "lexoffice_create_invoice";
        pub const SCHEMA_ID: &'static str = "external_operation.lexoffice_create_invoice.v1";
        pub const SCHEMA_VERSION: u32 = 1;
    }}
}}
"#,
            version = env!("CARGO_PKG_VERSION")
        )
    );
}

#[test]
fn capability_rust_binding_stubs_state_runtime_generation_boundaries() {
    let stubs = CapabilityDocument::from_manifest(&offer_manifest()).to_rust_binding_stubs();

    assert!(stubs.contains("Binding stubs declare types and schema constants only."));
    assert!(stubs.contains(
        "Behavior, provider registration, NATS adapters, and Restate execution are not generated here."
    ));
    assert!(!stubs.contains("ProviderRegistry"));
    assert!(!stubs.contains("RestateClient"));
}

fn offer_manifest() -> ArchitectureManifest {
    ArchitectureManifest {
        manifest_schema_id: "manifest.elbmesh.v1".to_string(),
        manifest_schema_version: 1,
        resources: vec![ResourceDefinition {
            resource_type: "offer".to_string(),
            schema_id: "resource.offer.v1".to_string(),
            schema_version: 1,
            components: vec![ComponentDefinition {
                component_type: "offer_terms".to_string(),
                schema_id: "component.offer_terms.v1".to_string(),
                schema_version: 1,
            }],
        }],
        actions: vec![ActionDefinition {
            action_type: "send_offer_email".to_string(),
            resource_type: "offer".to_string(),
            schema_id: "action.send_offer_email.v1".to_string(),
            schema_version: 1,
            emitted_event_types: vec!["offer_created".to_string()],
            external_operation_types: vec!["lexoffice_create_invoice".to_string()],
        }],
        events: vec![EventDefinition {
            event_type: "offer_created".to_string(),
            resource_type: "offer".to_string(),
            schema_id: "event.offer_created.v1".to_string(),
            schema_version: 1,
        }],
        reactions: vec![ReactionDefinition {
            reaction_type: "offer_created_to_send_offer_email".to_string(),
            trigger_event_type: "offer_created".to_string(),
            target_action_type: "send_offer_email".to_string(),
            schema_id: "reaction.offer_created_to_send_offer_email.v1".to_string(),
            schema_version: 1,
        }],
        views: vec![ViewDefinition {
            view_type: "offer_summary".to_string(),
            schema_id: "view.offer_summary.v1".to_string(),
            schema_version: 1,
        }],
        queries: vec![QueryDefinition {
            query_type: "get_offer_summary".to_string(),
            schema_id: "query.get_offer_summary.v1".to_string(),
            schema_version: 1,
        }],
        external_operations: vec![ExternalOperationDefinition {
            operation_type: "lexoffice_create_invoice".to_string(),
            schema_id: "external_operation.lexoffice_create_invoice.v1".to_string(),
            schema_version: 1,
        }],
    }
}
