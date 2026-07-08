use elbmesh_core::{
    ActionDefinition, ArchitectureCheckStatus, ArchitectureManifest, ComponentDefinition,
    EventDefinition, ExternalOperationDefinition, ManifestValidationError, QueryDefinition,
    ReactionDefinition, ResourceDefinition, ViewDefinition,
};

use serde_json::json;

type ManifestMutation = fn(&mut ArchitectureManifest);
type SchemaIdentityCase = (ManifestMutation, &'static str, &'static str);

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
    assert!(manifest.actions[0].external_operation_types.is_empty());
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
                "external_operation_types": [],
            }, {
                "action_type": "send_offer_email",
                "resource_type": "offer",
                "schema_id": "action.send_offer_email.v1",
                "schema_version": 1,
                "emitted_event_types": [],
                "external_operation_types": ["create_invoice"],
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
fn manifest_validation_rejects_missing_manifest_schema_id() {
    let mut manifest = offer_manifest();
    manifest.manifest_schema_id.clear();

    let err = manifest
        .validate()
        .expect_err("missing manifest schema id should fail validation");

    assert_eq!(
        err,
        ManifestValidationError::MissingSchemaId {
            definition_kind: "manifest".to_string(),
            definition_name: "manifest".to_string(),
        }
    );
    assert_eq!(err.code(), "manifest.missing_schema_id");
}

#[test]
fn manifest_validation_rejects_zero_manifest_schema_version() {
    let mut manifest = offer_manifest();
    manifest.manifest_schema_version = 0;

    let err = manifest
        .validate()
        .expect_err("zero manifest schema version should fail validation");

    assert_eq!(
        err,
        ManifestValidationError::InvalidSchemaVersion {
            definition_kind: "manifest".to_string(),
            definition_name: "manifest".to_string(),
            schema_version: 0,
        }
    );
    assert_eq!(err.code(), "manifest.invalid_schema_version");
}

#[test]
fn manifest_validation_rejects_missing_definition_schema_ids() {
    let cases: Vec<SchemaIdentityCase> = vec![
        (
            |manifest| manifest.resources[0].schema_id.clear(),
            "resource",
            "offer",
        ),
        (
            |manifest| manifest.resources[0].components[0].schema_id.clear(),
            "component",
            "offer_terms",
        ),
        (
            |manifest| manifest.actions[0].schema_id.clear(),
            "action",
            "create_offer",
        ),
        (
            |manifest| manifest.events[0].schema_id.clear(),
            "event",
            "offer_created",
        ),
        (
            |manifest| manifest.reactions[0].schema_id.clear(),
            "reaction",
            "offer_created_to_send_offer_email",
        ),
        (
            |manifest| manifest.views[0].schema_id.clear(),
            "view",
            "offer_summary",
        ),
        (
            |manifest| manifest.queries[0].schema_id.clear(),
            "query",
            "get_offer_summary",
        ),
        (
            |manifest| manifest.external_operations[0].schema_id.clear(),
            "external_operation",
            "create_invoice",
        ),
    ];

    for (mutate, definition_kind, definition_name) in cases {
        let mut manifest = offer_manifest();
        mutate(&mut manifest);

        let err = manifest
            .validate()
            .expect_err("missing definition schema id should fail validation");

        assert_eq!(
            err,
            ManifestValidationError::MissingSchemaId {
                definition_kind: definition_kind.to_string(),
                definition_name: definition_name.to_string(),
            }
        );
        assert_eq!(err.code(), "manifest.missing_schema_id");
    }
}

#[test]
fn manifest_validation_rejects_zero_definition_schema_versions() {
    let cases: Vec<SchemaIdentityCase> = vec![
        (
            |manifest| manifest.resources[0].schema_version = 0,
            "resource",
            "offer",
        ),
        (
            |manifest| manifest.resources[0].components[0].schema_version = 0,
            "component",
            "offer_terms",
        ),
        (
            |manifest| manifest.actions[0].schema_version = 0,
            "action",
            "create_offer",
        ),
        (
            |manifest| manifest.events[0].schema_version = 0,
            "event",
            "offer_created",
        ),
        (
            |manifest| manifest.reactions[0].schema_version = 0,
            "reaction",
            "offer_created_to_send_offer_email",
        ),
        (
            |manifest| manifest.views[0].schema_version = 0,
            "view",
            "offer_summary",
        ),
        (
            |manifest| manifest.queries[0].schema_version = 0,
            "query",
            "get_offer_summary",
        ),
        (
            |manifest| manifest.external_operations[0].schema_version = 0,
            "external_operation",
            "create_invoice",
        ),
    ];

    for (mutate, definition_kind, definition_name) in cases {
        let mut manifest = offer_manifest();
        mutate(&mut manifest);

        let err = manifest
            .validate()
            .expect_err("zero definition schema version should fail validation");

        assert_eq!(
            err,
            ManifestValidationError::InvalidSchemaVersion {
                definition_kind: definition_kind.to_string(),
                definition_name: definition_name.to_string(),
                schema_version: 0,
            }
        );
        assert_eq!(err.code(), "manifest.invalid_schema_version");
    }
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

#[test]
fn valid_manifest_declared_external_operation_validation_succeeds() {
    offer_manifest()
        .validate()
        .expect("valid declared external operation reference should pass validation");
}

#[test]
fn manifest_validation_rejects_action_referencing_unknown_external_operation() {
    let mut manifest = offer_manifest();
    manifest.actions[1].external_operation_types = vec!["missing_operation".to_string()];

    let err = manifest
        .validate()
        .expect_err("unknown action external operation should fail validation");

    assert_eq!(
        err,
        ManifestValidationError::UnknownActionExternalOperation {
            action_type: "send_offer_email".to_string(),
            operation_type: "missing_operation".to_string(),
        }
    );
    assert_eq!(err.code(), "manifest.action_unknown_external_operation");
}

#[test]
fn manifest_validation_rejects_action_referencing_same_external_operation_twice() {
    let mut manifest = offer_manifest();
    manifest.actions[1].external_operation_types =
        vec!["create_invoice".to_string(), "create_invoice".to_string()];

    let err = manifest
        .validate()
        .expect_err("duplicate action external operation reference should fail validation");

    assert_eq!(
        err,
        ManifestValidationError::DuplicateActionExternalOperation {
            action_type: "send_offer_email".to_string(),
            operation_type: "create_invoice".to_string(),
        }
    );
    assert_eq!(err.code(), "manifest.action_duplicate_external_operation");
}

#[test]
fn manifest_validation_rejects_duplicate_external_operation_type() {
    let mut manifest = offer_manifest();
    manifest
        .external_operations
        .push(ExternalOperationDefinition {
            operation_type: "create_invoice".to_string(),
            schema_id: "external_operation.create_invoice.v2".to_string(),
            schema_version: 2,
        });

    let err = manifest
        .validate()
        .expect_err("duplicate external operation type should fail validation");

    assert_eq!(
        err,
        ManifestValidationError::DuplicateExternalOperationType {
            operation_type: "create_invoice".to_string(),
        }
    );
    assert_eq!(err.code(), "manifest.duplicate_external_operation_type");
}

#[test]
fn valid_manifest_architecture_check_report_passes_with_stable_json_shape() {
    let report = offer_manifest().check_architecture();

    assert_eq!(report.manifest_schema_id, "manifest.elbmesh.v1");
    assert_eq!(report.manifest_schema_version, 1);
    assert_eq!(report.status, ArchitectureCheckStatus::Passed);
    assert!(report.findings.is_empty());

    let encoded = serde_json::to_value(&report).expect("serialize architecture check report");
    assert_eq!(
        encoded,
        json!({
            "manifest_schema_id": "manifest.elbmesh.v1",
            "manifest_schema_version": 1,
            "status": "passed",
            "findings": [],
        })
    );
}

#[test]
fn invalid_manifest_architecture_check_report_contains_named_finding() {
    let mut manifest = offer_manifest();
    manifest.resources.push(ResourceDefinition {
        resource_type: "offer".to_string(),
        schema_id: "resource.offer.v2".to_string(),
        schema_version: 2,
        components: Vec::new(),
    });

    let report = manifest.check_architecture();

    assert_eq!(report.status, ArchitectureCheckStatus::Failed);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].code, "manifest.duplicate_resource_type");
    assert_eq!(
        report.findings[0].message,
        "manifest declares resource type 'offer' more than once"
    );

    let encoded = serde_json::to_value(&report).expect("serialize architecture check report");
    assert_eq!(
        encoded,
        json!({
            "manifest_schema_id": "manifest.elbmesh.v1",
            "manifest_schema_version": 1,
            "status": "failed",
            "findings": [{
                "code": "manifest.duplicate_resource_type",
                "message": "manifest declares resource type 'offer' more than once",
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
        actions: vec![
            ActionDefinition {
                action_type: "create_offer".to_string(),
                resource_type: "offer".to_string(),
                schema_id: "action.create_offer.v1".to_string(),
                schema_version: 1,
                emitted_event_types: vec!["offer_created".to_string()],
                external_operation_types: Vec::new(),
            },
            ActionDefinition {
                action_type: "send_offer_email".to_string(),
                resource_type: "offer".to_string(),
                schema_id: "action.send_offer_email.v1".to_string(),
                schema_version: 1,
                emitted_event_types: Vec::new(),
                external_operation_types: vec!["create_invoice".to_string()],
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
