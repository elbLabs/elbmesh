use elbmesh_core::{
    ActionDefinition, ArchitectureManifest, ComponentDefinition, EventDefinition,
    ExternalOperationDefinition, ManifestValidationError, QueryDefinition, ReactionDefinition,
    ResourceDefinition, ViewDefinition,
};

use serde_json::json;

#[test]
fn architecture_manifest_describes_resource_action_and_event_schema_identity() {
    let manifest = offer_manifest();

    assert_eq!(manifest.manifest_schema_id, "manifest.elbmesh.v1");
    assert_eq!(manifest.manifest_schema_version, 1);
    assert_eq!(manifest.resources[0].resource_type, "offer");
    assert_eq!(manifest.resources[0].schema_id, "resource.offer.v1");
    assert_eq!(manifest.resources[0].schema_version, 1);
    assert_eq!(manifest.actions[0].action_type, "create_offer");
    assert_eq!(manifest.actions[0].resource_type, "offer");
    assert_eq!(manifest.actions[0].schema_id, "action.create_offer.v1");
    assert_eq!(manifest.actions[0].schema_version, 1);
    assert_eq!(
        manifest.actions[0].emitted_event_types,
        vec!["offer_created"]
    );
    assert_eq!(manifest.events[0].event_type, "offer_created");
    assert_eq!(manifest.events[0].resource_type, "offer");
    assert_eq!(manifest.events[0].schema_id, "event.offer_created.v1");
    assert_eq!(manifest.events[0].schema_version, 1);
}

#[test]
fn manifest_definition_skeletons_carry_schema_identity_and_versions() {
    let manifest = offer_manifest();

    assert_eq!(
        manifest.resources[0].components[0].component_type,
        "offer_terms"
    );
    assert_eq!(
        manifest.resources[0].components[0].schema_id,
        "component.offer_terms.v1"
    );
    assert_eq!(manifest.resources[0].components[0].schema_version, 1);
    assert_eq!(
        manifest.reactions[0].reaction_type,
        "offer_created_to_send_offer_email"
    );
    assert_eq!(manifest.reactions[0].trigger_event_type, "offer_created");
    assert_eq!(manifest.reactions[0].target_action_type, "send_offer_email");
    assert_eq!(
        manifest.reactions[0].schema_id,
        "reaction.offer_created_to_send_offer_email.v1"
    );
    assert_eq!(manifest.reactions[0].schema_version, 1);
    assert_eq!(manifest.views[0].view_type, "offer_summary");
    assert_eq!(manifest.views[0].schema_id, "view.offer_summary.v1");
    assert_eq!(manifest.views[0].schema_version, 1);
    assert_eq!(manifest.queries[0].query_type, "get_offer_summary");
    assert_eq!(manifest.queries[0].schema_id, "query.get_offer_summary.v1");
    assert_eq!(manifest.queries[0].schema_version, 1);
    assert_eq!(
        manifest.external_operations[0].operation_type,
        "create_invoice"
    );
    assert_eq!(
        manifest.external_operations[0].schema_id,
        "external_operation.create_invoice.v1"
    );
    assert_eq!(manifest.external_operations[0].schema_version, 1);
}

#[test]
fn architecture_manifest_round_trips_as_stable_json_shape() {
    let manifest = offer_manifest();
    let encoded = serde_json::to_value(&manifest).expect("serialize architecture manifest");

    assert_eq!(
        encoded,
        json!({
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
                "action_type": "create_offer",
                "resource_type": "offer",
                "schema_id": "action.create_offer.v1",
                "schema_version": 1,
                "emitted_event_types": ["offer_created"],
            }, {
                "action_type": "send_offer_email",
                "resource_type": "offer",
                "schema_id": "action.send_offer_email.v1",
                "schema_version": 1,
                "emitted_event_types": [],
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
                "operation_type": "create_invoice",
                "schema_id": "external_operation.create_invoice.v1",
                "schema_version": 1,
            }],
        })
    );

    let decoded: ArchitectureManifest =
        serde_json::from_value(encoded).expect("deserialize architecture manifest");
    assert_eq!(decoded, manifest);
}

#[test]
fn valid_manifest_resource_ownership_validation_succeeds() {
    offer_manifest()
        .validate()
        .expect("valid manifest should pass resource ownership validation");
}

#[test]
fn manifest_validation_rejects_action_targeting_unknown_resource() {
    let mut manifest = offer_manifest();
    manifest.actions[0].resource_type = "missing-offer".to_string();

    let err = manifest
        .validate()
        .expect_err("unknown action target resource should fail validation");

    assert_eq!(
        err,
        ManifestValidationError::UnknownActionResource {
            action_type: "create_offer".to_string(),
            resource_type: "missing-offer".to_string(),
        }
    );
    assert_eq!(err.code(), "manifest.action_unknown_resource");
}

#[test]
fn manifest_validation_rejects_event_owned_by_unknown_resource() {
    let mut manifest = offer_manifest();
    manifest.events[0].resource_type = "missing-offer".to_string();

    let err = manifest
        .validate()
        .expect_err("unknown event owner resource should fail validation");

    assert_eq!(
        err,
        ManifestValidationError::UnknownEventResource {
            event_type: "offer_created".to_string(),
            resource_type: "missing-offer".to_string(),
        }
    );
    assert_eq!(err.code(), "manifest.event_unknown_resource");
}

#[test]
fn manifest_validation_rejects_duplicate_resource_type() {
    let mut manifest = offer_manifest();
    manifest.resources.push(ResourceDefinition {
        resource_type: "offer".to_string(),
        schema_id: "resource.offer.v2".to_string(),
        schema_version: 2,
        components: Vec::new(),
    });

    let err = manifest
        .validate()
        .expect_err("duplicate resource type should fail validation");

    assert_eq!(
        err,
        ManifestValidationError::DuplicateResourceType {
            resource_type: "offer".to_string(),
        }
    );
    assert_eq!(err.code(), "manifest.duplicate_resource_type");
}

#[test]
fn valid_manifest_reaction_graph_validation_succeeds() {
    offer_manifest()
        .validate()
        .expect("valid acyclic reaction graph should pass validation");
}

#[test]
fn manifest_validation_rejects_reaction_triggering_from_unknown_event() {
    let mut manifest = offer_manifest();
    manifest.reactions[0].trigger_event_type = "missing_event".to_string();

    let err = manifest
        .validate()
        .expect_err("unknown reaction trigger event should fail validation");

    assert_eq!(
        err,
        ManifestValidationError::UnknownReactionTriggerEvent {
            reaction_type: "offer_created_to_send_offer_email".to_string(),
            event_type: "missing_event".to_string(),
        }
    );
    assert_eq!(err.code(), "manifest.reaction_unknown_trigger_event");
}

#[test]
fn manifest_validation_rejects_action_emitting_unknown_event() {
    let mut manifest = offer_manifest();
    manifest.actions[0].emitted_event_types = vec!["missing_event".to_string()];

    let err = manifest
        .validate()
        .expect_err("unknown action emitted event should fail validation");

    assert_eq!(
        err,
        ManifestValidationError::UnknownActionEmittedEvent {
            action_type: "create_offer".to_string(),
            event_type: "missing_event".to_string(),
        }
    );
    assert_eq!(err.code(), "manifest.action_unknown_emitted_event");
}

#[test]
fn manifest_validation_rejects_reaction_targeting_unknown_action() {
    let mut manifest = offer_manifest();
    manifest.reactions[0].target_action_type = "missing_action".to_string();

    let err = manifest
        .validate()
        .expect_err("unknown reaction target action should fail validation");

    assert_eq!(
        err,
        ManifestValidationError::UnknownReactionTargetAction {
            reaction_type: "offer_created_to_send_offer_email".to_string(),
            action_type: "missing_action".to_string(),
        }
    );
    assert_eq!(err.code(), "manifest.reaction_unknown_target_action");
}

#[test]
fn manifest_validation_rejects_reaction_graph_cycle() {
    let mut manifest = offer_manifest();
    manifest.actions[0].emitted_event_types = vec!["offer_created".to_string()];
    manifest.reactions[0].target_action_type = "create_offer".to_string();

    let err = manifest
        .validate()
        .expect_err("reaction graph cycle should fail validation");

    assert_eq!(
        err,
        ManifestValidationError::ReactionGraphCycle {
            path: vec![
                "event:offer_created".to_string(),
                "action:create_offer".to_string(),
                "event:offer_created".to_string(),
            ],
        }
    );
    assert_eq!(err.code(), "manifest.reaction_graph_cycle");
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
        actions: vec![
            ActionDefinition {
                action_type: "create_offer".to_string(),
                resource_type: "offer".to_string(),
                schema_id: "action.create_offer.v1".to_string(),
                schema_version: 1,
                emitted_event_types: vec!["offer_created".to_string()],
            },
            ActionDefinition {
                action_type: "send_offer_email".to_string(),
                resource_type: "offer".to_string(),
                schema_id: "action.send_offer_email.v1".to_string(),
                schema_version: 1,
                emitted_event_types: Vec::new(),
            },
        ],
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
            operation_type: "create_invoice".to_string(),
            schema_id: "external_operation.create_invoice.v1".to_string(),
            schema_version: 1,
        }],
    }
}
