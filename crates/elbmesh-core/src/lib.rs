//! Core traits and in-memory runtime for the elbmesh event-sourcing framework.

mod action_journal;

mod error;
mod manifest;
mod message;
mod reaction;
mod reaction_journal;
mod runtime;
mod store;
mod testing;
mod traits;

pub use action_journal::{
    ActionFailureClassification, ActionJournal, ActionJournalError, ActionJournalRecord,
    ActionJournalStream, InMemoryActionJournal,
};

pub use error::{
    ActionError, ActionFailure, EventStoreError, ExecutionError, HandlerError, ResourceError,
};
pub use manifest::{
    ActionDefinition, ArchitectureCheckFinding, ArchitectureCheckReport, ArchitectureCheckStatus,
    ArchitectureManifest, ComponentDefinition, EventDefinition, ExternalOperationDefinition,
    ManifestValidationError, QueryDefinition, ReactionDefinition, ResourceDefinition,
    ViewDefinition,
};
pub use message::{
    ActionDecision, ActionMetadata, ActionReceipt, ActionStatus, EmittedEvent, MessageMetadata,
    NewEvent, RecordedEvent, ResourceStream, StreamType,
};
pub use reaction::{Reaction, ReactionExecutionError, ReactionReceipt, ReactionRuntime};
pub use reaction_journal::{
    InMemoryReactionJournal, ReactionJournal, ReactionJournalError, ReactionJournalRecord,
    ReactionJournalStream,
};
pub use runtime::{ActionContext, ActionExecutor};
pub use store::{AppendResult, EventStore, ExpectedVersion, InMemoryEventStore};
pub use testing::ActionScenario;
pub use traits::{apply_recorded_event, Action, Apply, Event, Handle, Resource};
