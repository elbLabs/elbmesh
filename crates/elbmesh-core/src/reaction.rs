use async_trait::async_trait;
use chrono::Utc;
use thiserror::Error;

use crate::{
    Action as ActionTrait, ActionExecutor, ActionFailure, ActionMetadata, ActionReceipt, Event,
    EventStore, ExecutionError, Handle, MessageMetadata, ReactionJournal, ReactionJournalError,
    ReactionJournalRecord, ReactionJournalStream, RecordedEvent, Resource, StreamType,
};

#[async_trait]
pub trait Reaction: Send + Sync + 'static {
    type Trigger: Event;
    type Resource: Resource + Handle<Self::Action>;
    type Action: ActionTrait<Resource = Self::Resource>;

    const REACTION_TYPE: &'static str;
    const SCHEMA_ID: &'static str;
    const SCHEMA_VERSION: u32;

    async fn react(&self, event: Self::Trigger) -> Self::Action;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionReceipt {
    pub reaction_id: String,
    pub action_receipt: ActionReceipt,
}

#[derive(Debug, Error)]
pub enum ReactionExecutionError<E>
where
    E: ActionFailure,
{
    #[error(
        "failed to deserialize reaction trigger event '{message_type}' v{schema_version}: {source}"
    )]
    TriggerEventDeserialization {
        message_type: String,
        schema_version: u32,
        source: serde_json::Error,
    },

    #[error(transparent)]
    ReactionJournal(#[from] ReactionJournalError),

    #[error(transparent)]
    Action(#[from] ExecutionError<E>),
}

pub struct ReactionRuntime<S, J> {
    action_executor: ActionExecutor<S>,
    reaction_journal: J,
}

impl<S, J> ReactionRuntime<S, J>
where
    S: EventStore,
    J: ReactionJournal,
{
    pub fn new(event_store: S, reaction_journal: J) -> Self {
        Self {
            action_executor: ActionExecutor::new(event_store),
            reaction_journal,
        }
    }

    pub fn event_store(&self) -> &S {
        self.action_executor.event_store()
    }

    pub fn reaction_id<Rxn>(trigger: &RecordedEvent) -> String
    where
        Rxn: Reaction,
    {
        format!("{}:{}", Rxn::REACTION_TYPE, trigger.metadata.message_id)
    }

    pub async fn execute<Rxn>(
        &self,
        trigger: &RecordedEvent,
        reaction: &Rxn,
        action_metadata: ActionMetadata,
    ) -> Result<
        Option<ReactionReceipt>,
        ReactionExecutionError<
            <<Rxn as Reaction>::Resource as Handle<<Rxn as Reaction>::Action>>::Error,
        >,
    >
    where
        Rxn: Reaction,
    {
        if !matches_trigger::<Rxn::Trigger>(trigger) {
            return Ok(None);
        }

        let reaction_id = Self::reaction_id::<Rxn>(trigger);
        let journal_stream = ReactionJournalStream::for_reaction(reaction_id.clone());
        let trigger_event = serde_json::from_value::<Rxn::Trigger>(trigger.payload.clone())
            .map_err(
                |source| ReactionExecutionError::TriggerEventDeserialization {
                    message_type: trigger.metadata.message_type.clone(),
                    schema_version: trigger.metadata.schema_version,
                    source,
                },
            )?;

        let action = reaction.react(trigger_event).await;
        self.reaction_journal
            .append(
                &journal_stream,
                ReactionJournalRecord::ReactionTriggered {
                    reaction_id: reaction_id.clone(),
                    metadata: reaction_journal_metadata(
                        "reaction_triggered",
                        "journal.reaction_triggered.v1",
                        trigger,
                        &action_metadata,
                    ),
                    reaction_type: Rxn::REACTION_TYPE.to_string(),
                    reaction_schema_id: Rxn::SCHEMA_ID.to_string(),
                    reaction_schema_version: Rxn::SCHEMA_VERSION,
                    trigger_event_type: <Rxn::Trigger as Event>::EVENT_TYPE.to_string(),
                    trigger_event_id: trigger.metadata.message_id.clone(),
                },
            )
            .await?;

        let action_receipt = self
            .action_executor
            .execute::<Rxn::Resource, Rxn::Action>(action, action_metadata.clone())
            .await?;

        self.reaction_journal
            .append(
                &journal_stream,
                ReactionJournalRecord::ReactionCompleted {
                    reaction_id: reaction_id.clone(),
                    metadata: reaction_journal_metadata(
                        "reaction_completed",
                        "journal.reaction_completed.v1",
                        trigger,
                        &action_metadata,
                    ),
                    triggered_action_id: action_receipt.action_id.clone(),
                },
            )
            .await?;

        Ok(Some(ReactionReceipt {
            reaction_id,
            action_receipt,
        }))
    }
}

fn matches_trigger<E>(trigger: &RecordedEvent) -> bool
where
    E: Event,
{
    trigger.metadata.message_type == E::EVENT_TYPE
        && trigger.metadata.schema_id == E::SCHEMA_ID
        && trigger.metadata.schema_version == E::SCHEMA_VERSION
        && trigger.metadata.resource_type == E::Resource::RESOURCE_TYPE
        && trigger.metadata.stream_type == StreamType::Resource
}

fn reaction_journal_metadata(
    message_type: impl Into<String>,
    schema_id: impl Into<String>,
    trigger: &RecordedEvent,
    action: &ActionMetadata,
) -> MessageMetadata {
    let message_type = message_type.into();

    MessageMetadata {
        message_id: uuid::Uuid::new_v4().to_string(),
        message_type,
        message_version: 1,
        resource_type: trigger.metadata.resource_type.clone(),
        resource_id: trigger.metadata.resource_id.clone(),
        stream_type: StreamType::Reaction,
        correlation_id: action.correlation_id.clone(),
        causation_id: trigger.metadata.message_id.clone(),
        action_id: action.action_id.clone(),
        actor_id: action.actor_id.clone(),
        occurred_at: Utc::now().to_rfc3339(),
        schema_id: schema_id.into(),
        schema_version: 1,
    }
}
