//! Runtime cognitivo: contexto tipado, barramento de eventos e escalonador.

pub mod context;
pub mod envelope;
pub mod event_auditor;
pub mod event_bus;
pub mod module;
pub mod module_state;
pub mod scheduler;
pub mod tracing;

pub use context::TypedContext;
pub use envelope::envelope;
pub use event_auditor::EventAuditor;
pub use event_bus::{EventBus, EventSubscription};
pub use module::CognitiveModule;
pub use module_state::ModuleState;
pub use scheduler::{Scheduler, StepBudget, StepReport};
pub use tracing::tracing_init;
