use elbmesh_core::{
    ActionDefinition, ArchitectureManifest, EventDefinition, ManifestValidationError,
    ResourceDefinition,
};

#[test]
fn manifest_validation_accepts_action_events_owned_by_the_target_resource() {
    manifest_with_action_event_owner("offer")
        .validate()
        .expect("an Action may declare Events owned by its target Resource");
}

#[test]
fn manifest_validation_rejects_cross_resource_action_event_with_stable_details() {
    let error: ManifestValidationError = manifest_with_action_event_owner("invoice")
        .validate()
        .expect_err("an Action must not declare an Event owned by another Resource");

    assert_eq!(error.code(), "manifest.action_cross_resource_emitted_event");
    assert_eq!(
        error.to_string(),
        "manifest action 'create_offer' targets resource 'offer' but emits event 'offer_created' owned by resource 'invoice'"
    );
}

fn manifest_with_action_event_owner(event_resource_type: &str) -> ArchitectureManifest {
    ArchitectureManifest {
        manifest_schema_id: "manifest.elbmesh.v1".to_string(),
        manifest_schema_version: 1,
        resources: vec![
            ResourceDefinition {
                resource_type: "offer".to_string(),
                schema_id: "resource.offer.v1".to_string(),
                schema_version: 1,
                components: Vec::new(),
            },
            ResourceDefinition {
                resource_type: "invoice".to_string(),
                schema_id: "resource.invoice.v1".to_string(),
                schema_version: 1,
                components: Vec::new(),
            },
        ],
        actions: vec![ActionDefinition {
            action_type: "create_offer".to_string(),
            resource_type: "offer".to_string(),
            schema_id: "action.create_offer.v1".to_string(),
            schema_version: 1,
            emitted_event_types: vec!["offer_created".to_string()],
            external_operation_types: Vec::new(),
        }],
        events: vec![EventDefinition {
            event_type: "offer_created".to_string(),
            resource_type: event_resource_type.to_string(),
            schema_id: "event.offer_created.v1".to_string(),
            schema_version: 1,
        }],
        reactions: Vec::new(),
        views: Vec::new(),
        queries: Vec::new(),
        external_operations: Vec::new(),
    }
}
