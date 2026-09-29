//! Contratos do ciclo de aprendizado: decisão → resultado → efeito futuro verificado.

use triad_foundation as tf;

use serde::{Deserialize, Serialize};

/// Efeito futuro efetivamente observado, com nível de evidência.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerifiedEffect {
    pub observed_step: tf::id::StepId,
    pub description: String,
    pub evidence: tf::evidence::EvidenceLevel,
}

/// Observação do resultado real de uma decisão já executada.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutcomeObserved {
    pub event_id: tf::id::EventId,
    pub decision_id: tf::id::DecisionId,
    pub outcome: String,
    pub expected: String,
}

/// Pacote do ciclo decisão→resultado→aprendizado.
///
/// Sem `verified_future_effect` o ciclo decision→outcome→learning NÃO fecha.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LearningEnvelope {
    pub event_id: tf::id::EventId,
    pub decision_id: tf::id::DecisionId,
    pub prediction: String,
    pub action: String,
    pub expected_outcome: String,
    pub actual_outcome: String,
    pub error: Option<f32>,
    pub credit_assignment: Vec<tf::id::ModuleId>,
    pub updated_modules: Vec<tf::id::ModuleId>,
    pub memory_update: Option<tf::id::EpisodeId>,
    pub policy_update: Option<tf::id::ModuleId>,
    pub verified_future_effect: Option<VerifiedEffect>,
}

impl LearningEnvelope {
    /// O ciclo só fecha com um efeito futuro verificado.
    pub fn is_closed(&self) -> bool {
        self.verified_future_effect.is_some()
    }
}
