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
