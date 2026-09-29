//! Eventos tipados por domínio; todos viajam dentro de um `EventEnvelope`.

pub mod cognitive;
pub mod decision;
pub mod governance;
pub mod learning;
pub mod memory;
pub mod tissue;

pub use cognitive::{CognitiveEvent, CognitiveKind};
pub use decision::{DecisionOption, DecisionProposal, IdentityContext, LearningStatus, LimbicModulation};
pub use governance::{GovernanceIntervention, InterventionStatus};
pub use learning::{LearningEnvelope, OutcomeObserved, VerifiedEffect};
pub use memory::{MemoryEvent, MemoryKind};
pub use tissue::{TissueEvent, TissueKind, TissueState};
