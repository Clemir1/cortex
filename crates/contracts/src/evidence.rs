//! Evidência: cadeias, rastro de fronteira e o quinto de fase.

use triad_foundation as tf;

use serde::{Deserialize, Serialize};

/// As sete cadeias de evidência do Cortex.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Chain {
    Physical,
    Tissue,
    Cognitive,
    Decision,
    Learning,
    Governance,
    Development,
}

/// Rastro de uma travessia de camada.
///
/// Toda travessia de camada deixa rastro.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundaryRecord {
    pub from_layer: u8,
    pub to_layer: u8,
    pub event_id: tf::id::EventId,
    pub ts: tf::time::Timestamp,
}

/// O quinto de fase: input→aceito→atuado→efeito→aprendido; cada estágio tipado.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StageQuintet {
    pub input_events: Vec<tf::id::EventId>,
    pub accepted: Vec<tf::id::EventId>,
    pub acted: Vec<tf::id::EventId>,
    pub effect: Vec<tf::id::EventId>,
    pub learned: Vec<tf::id::EventId>,
}
