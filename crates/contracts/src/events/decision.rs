//! Contratos de decisão (L4): proposta, opções, identidade e modulação límbica.

use triad_foundation as tf;

use serde::{Deserialize, Serialize};

/// Fase da decisão no ciclo decide→age→verifica.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LearningStatus {
    Blocked,
    Learning,
    Learned,
    Validated,
}

/// Opção de ação candidata com valor predito e confiança.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionOption {
    pub action: String,
    pub predicted_value: tf::units::Reward,
    pub confidence: tf::units::Confidence,
}

/// Identidade do agente no momento da decisão (valores + continuidade).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IdentityContext {
    pub values: Vec<String>,
    pub continuity: tf::units::Confidence,
    pub source_module: tf::id::ModuleId,
}

/// Modulação límbica (stress, energia, aversão) aplicada sobre a decisão.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LimbicModulation {
    pub stress: tf::units::Stress,
    pub energy: tf::units::Energy,
    pub aversion: Option<tf::units::Aversion>,
}

/// Proposta de decisão a resolver no módulo de decisão.
///
/// DECISÃO SEM `identity_context` É REJEITADA (o jogador não pode jogar sem 自我) —
/// o DecisionModule do L4 aplica a lei; o contrato só carrega o campo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionProposal {
    pub event_id: tf::id::EventId,
    pub options: Vec<DecisionOption>,
    pub identity_context: Option<IdentityContext>,
    pub limbic_modulation: Option<LimbicModulation>,
    pub workspace_contents: Vec<tf::id::ConceptId>,
    pub learning_status: LearningStatus,
    pub expected_outcome: String,
}
