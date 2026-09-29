//! Cognição global (L4): espaço de trabalho, modelo do mundo, decisão e ação.

pub mod action;
pub mod causal;
pub mod config;
pub mod decision;
pub mod degraded;
pub mod integration;
pub mod memory;
pub mod twin;
pub mod workspace;
pub mod world_model;

pub use config::L4Config;
pub use integration::{L4Module, L4Stats};
pub use twin::{twin_verdicts, TwinDecision, TwinVerdict};

use triad_contracts as tc;
use triad_foundation as tf;

/// Decisão pendente no espaço de trabalho global.
#[derive(Debug, Clone)]
pub struct PendingDecision {
    pub event_id: tf::id::EventId,
    pub options: Vec<tc::DecisionOption>,
    pub identity_context: Option<tc::IdentityContext>,
    pub limbic_modulation: Option<tc::LimbicModulation>,
    pub expected_outcome: String,
}

/// Estado de uma ação executada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionStatus { Executed, Failed, Skipped }

/// Registro de ação executada.
#[derive(Debug, Clone)]
pub struct ActionRecord {
    pub action: String,
    pub decision_id: tf::id::DecisionId,
    pub step: tf::id::StepId,
    pub predicted_reward: tf::Reward,
    pub status: ActionStatus,
}

/// Estágios do espaço de trabalho global.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceStage { Sensory, Local, Competition, Broadcast, Decision, Action, Consolidation }

impl WorkspaceStage {
    /// Nome canônico do estágio.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Sensory => "sensory",
            Self::Local => "local",
            Self::Competition => "competition",
            Self::Broadcast => "broadcast",
            Self::Decision => "decision",
            Self::Action => "action",
            Self::Consolidation => "consolidation",
        }
    }
}

/// Perdas por estágio do espaço de trabalho.
#[derive(Debug, Clone)]
pub struct StageLosses { pub stage: WorkspaceStage, pub dropped: usize, pub total: usize }

impl StageLosses {
    /// Razão de perda; `None` quando não houve amostras (ausência ≠ zero).
    pub fn loss_ratio(&self) -> Option<f32> {
        if self.total == 0 { None } else { Some(self.dropped as f32 / self.total as f32) }
    }
}
