//! Core traits and in-memory runtime for the elbmesh event-sourcing framework.

mod error;
mod message;
mod runtime;
mod store;
mod testing;
mod traits;

pub use error::{
    ActionError, ActionFailure, EventStoreError, ExecutionError, HandlerError, ResourceError,
};
pub use message::{
    ActionDecision, ActionMetadata, ActionReceipt, ActionStatus, EmittedEvent, MessageMetadata,
    NewEvent, RecordedEvent, ResourceStream, StreamType,
};
pub use runtime::{ActionContext, ActionExecutor};
pub use store::{AppendResult, EventStore, ExpectedVersion, InMemoryEventStore};
pub use testing::ActionScenario;
pub use traits::{apply_recorded_event, Action, Apply, Event, Handle, Resource};
