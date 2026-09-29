//! Eventos cognitivos: ativação e competição de conceitos (L3).

use triad_foundation as tf;

use serde::{Deserialize, Serialize};

/// Ciclo de vida da ativação de um conceito.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CognitiveKind {
    Activated,
    Inhibited,
    Bound,
    Competed,
}

/// Ativação/inibição de um conceito no espaço cognitivo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CognitiveEvent {
    pub event_id: tf::id::EventId,
    pub concept_id: tf::id::ConceptId,
    pub kind: CognitiveKind,
    pub salience: tf::units::Salience,
}
