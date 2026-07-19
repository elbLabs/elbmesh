//! Core traits and in-memory runtime for the elbmesh event-sourcing framework.

mod action_journal;
mod capability;

mod error;
mod external_operation;
mod manifest;
mod message;
mod operation_journal;
mod projection;
mod reaction;
mod reaction_journal;
mod runtime;
mod store;
mod testing;
mod traits;
mod view_store;

pub use action_journal::{
    ActionFailureClassification, ActionJournal, ActionJournalError, ActionJournalRecord,
    ActionJournalStream, InMemoryActionJournal,
};

#[cfg(feature = "nats-adapter")]
pub use action_journal::{NatsActionJournal, NatsActionJournalConfig};

pub use capability::{
    CapabilityDocument, CapabilityGeneratorMetadata, CAPABILITY_GENERATOR_NAME,
    CAPABILITY_GENERATOR_VERSION, CAPABILITY_SCHEMA_ID, CAPABILITY_SCHEMA_VERSION,
};

pub use error::{
    ActionError, ActionFailure, EventStoreError, ExecutionError, HandlerError, ResourceError,
};
pub use external_operation::{
    CreateLexOfficeInvoiceRequest, ExternalOperation, ExternalOperationCall,
    ExternalOperationFailure, LexOfficeCreateInvoiceError, LexOfficeCreateInvoiceResult,
    MockLexOfficeCreateInvoice,
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
pub use operation_journal::{
    InMemoryOperationJournal, OperationJournal, OperationJournalError, OperationJournalRecord,
    OperationJournalStream,
};

#[cfg(feature = "nats-adapter")]
pub use operation_journal::{NatsOperationJournal, NatsOperationJournalConfig};

#[cfg(feature = "restate-adapter")]
pub use operation_journal::{
    RestateOperationJournal, RestateOperationJournalConfig, RestateOperationJournalObject,
    RestateOperationJournalObjectClient, RestateOperationJournalObjectImpl,
};
pub use projection::{
    Projection, ProjectionCheckpoint, ProjectionCheckpointError, ProjectionCheckpointStore,
    ProjectionContext, ProjectionCursor, ProjectionDispatchError, ProjectionDispatchFailure,
    ProjectionDispatchReport, ProjectionDispatcher, ProjectionExecutionError, ProjectionRebuild,
    ProjectionRebuildSelection, ProjectionRuntime, TypedProjectionHandler,
};
pub use reaction::{
    Reaction, ReactionDispatchError, ReactionDispatchFailure, ReactionDispatcher,
    ReactionExecutionError, ReactionReceipt, ReactionRuntime, TypedReactionHandler,
};
pub use reaction_journal::{
    InMemoryReactionJournal, ReactionJournal, ReactionJournalError, ReactionJournalRecord,
    ReactionJournalStream,
};

#[cfg(feature = "nats-adapter")]
pub use reaction_journal::{NatsReactionJournal, NatsReactionJournalConfig};
pub use runtime::{ActionContext, ActionExecutor};
pub use store::{AppendResult, EventStore, ExpectedVersion, InMemoryEventStore};

#[cfg(feature = "nats-adapter")]
pub use store::{NatsEventStore, NatsEventStoreConfig};

pub use testing::ActionScenario;
pub use traits::{apply_recorded_event, Action, Apply, Event, Handle, Resource};
pub use view_store::{
    InMemoryViewStore, ViewDocument, ViewIndexEntry, ViewKey, ViewStore, ViewStoreError,
};

#[cfg(feature = "nats-adapter")]
pub use view_store::{NatsViewStore, NatsViewStoreConfig};
