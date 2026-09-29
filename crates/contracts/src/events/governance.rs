//! Intervenções de governança (L5) sobre parâmetros de qualquer camada.

use triad_foundation as tf;

use serde::{Deserialize, Serialize};

/// Ciclo de vida de uma intervenção.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InterventionStatus {
    Proposed,
    Applied,
    Kept,
    Reverted,
}

/// Ajuste de parâmetro proposto/aplicado pela governança, com TTL e efeito observado.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GovernanceIntervention {
    pub governance_id: tf::id::EventId,
    pub target_layer: u8,
    pub target_parameter: String,
    pub old_value: String,
    pub new_value: String,
    pub reason: String,
    pub expected_effect: String,
    pub ttl: tf::id::StepId,
    pub confidence: tf::units::Confidence,
    pub observed_effect: Option<String>,
    pub status: InterventionStatus,
}
