use std::collections::{HashMap, HashSet};

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

        let action_types: HashSet<_> = self
            .actions
            .iter()
            .map(|action| action.action_type.as_str())
            .collect();
        let event_types: HashSet<_> = self
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect();

        for action in &self.actions {
            for event_type in &action.emitted_event_types {
                if !event_types.contains(event_type.as_str()) {
                    return Err(ManifestValidationError::UnknownActionEmittedEvent {
                        action_type: action.action_type.clone(),
                        event_type: event_type.clone(),
                    });
                }
            }
        }

        for reaction in &self.reactions {
            if !event_types.contains(reaction.trigger_event_type.as_str()) {
                return Err(ManifestValidationError::UnknownReactionTriggerEvent {
                    reaction_type: reaction.reaction_type.clone(),
                    event_type: reaction.trigger_event_type.clone(),
                });
            }

            if !action_types.contains(reaction.target_action_type.as_str()) {
                return Err(ManifestValidationError::UnknownReactionTargetAction {
                    reaction_type: reaction.reaction_type.clone(),
                    action_type: reaction.target_action_type.clone(),
                });
            }
        }

        if let Some(path) = self.reaction_graph_cycle() {
            return Err(ManifestValidationError::ReactionGraphCycle { path });
        }

        Ok(())
    }

    fn reaction_graph_cycle(&self) -> Option<Vec<String>> {
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        for reaction in &self.reactions {
            graph
                .entry(event_node(&reaction.trigger_event_type))
                .or_default()
                .push(action_node(&reaction.target_action_type));
        }

        for action in &self.actions {
            for event_type in &action.emitted_event_types {
                graph
                    .entry(action_node(&action.action_type))
                    .or_default()
                    .push(event_node(event_type));
            }
        }

        for reaction in &self.reactions {
            let start = event_node(&reaction.trigger_event_type);
            let mut path = vec![start.clone()];

            if let Some(cycle) = find_cycle_to_start(&graph, &start, &start, &mut path) {
                return Some(cycle);
            }
        }

        None
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

    #[error("manifest action '{action_type}' emits undeclared event '{event_type}'")]
    UnknownActionEmittedEvent {
        action_type: String,
        event_type: String,
    },

    #[error("manifest reaction '{reaction_type}' triggers from undeclared event '{event_type}'")]
    UnknownReactionTriggerEvent {
        reaction_type: String,
        event_type: String,
    },

    #[error("manifest reaction '{reaction_type}' targets undeclared action '{action_type}'")]
    UnknownReactionTargetAction {
        reaction_type: String,
        action_type: String,
    },

    #[error("manifest reaction graph contains a cycle: {path:?}")]
    ReactionGraphCycle { path: Vec<String> },
}

impl ManifestValidationError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::DuplicateResourceType { .. } => "manifest.duplicate_resource_type",
            Self::UnknownActionResource { .. } => "manifest.action_unknown_resource",
            Self::UnknownEventResource { .. } => "manifest.event_unknown_resource",
            Self::UnknownActionEmittedEvent { .. } => "manifest.action_unknown_emitted_event",
            Self::UnknownReactionTriggerEvent { .. } => "manifest.reaction_unknown_trigger_event",
            Self::UnknownReactionTargetAction { .. } => "manifest.reaction_unknown_target_action",
            Self::ReactionGraphCycle { .. } => "manifest.reaction_graph_cycle",
        }
    }
}

fn find_cycle_to_start(
    graph: &HashMap<String, Vec<String>>,
    start: &str,
    current: &str,
    path: &mut Vec<String>,
) -> Option<Vec<String>> {
    for next in graph.get(current).into_iter().flatten() {
        if next == start {
            let mut cycle = path.clone();
            cycle.push(next.clone());
            return Some(cycle);
        }

        if path.iter().any(|entry| entry == next) {
            continue;
        }

        path.push(next.clone());
        if let Some(cycle) = find_cycle_to_start(graph, start, next, path) {
            return Some(cycle);
        }
        path.pop();
    }

    None
}

fn event_node(event_type: &str) -> String {
    format!("event:{event_type}")
}

fn action_node(action_type: &str) -> String {
    format!("action:{action_type}")
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
    pub emitted_event_types: Vec<String>,
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
    pub trigger_event_type: String,
    pub target_action_type: String,
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
