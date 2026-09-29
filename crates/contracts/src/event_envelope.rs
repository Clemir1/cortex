//! Envelope padrão que todo evento carrega ao cruzar fronteiras de camada.

use triad_foundation as tf;

use serde::{Deserialize, Serialize};

/// Prioridade de entrega do evento (totalmente ordenável).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

/// Classificação funcional do evento por domínio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    Cognitive,
    Memory,
    Decision,
    Learning,
    Tissue,
    Governance,
    Development,
    Federation,
    Law,
    Ecology,
}

bitflags::bitflags! {
    /// Máscara das partes do estado sujas após o evento.
    #[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
    pub struct DirtyMask: u32 {
        const STATE = 1 << 0;
        const METRICS = 1 << 1;
        const STRUCTURE = 1 << 2;
        const LAW = 1 << 3;
    }
}

/// Envelope único de evento: identidade, versão, tipo, prioridade, prazo e sujeira.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub event_id: tf::id::EventId,
    pub parent_event_id: Option<tf::id::EventId>,
    pub entity_id: String,
    pub entity_version: u64,
    pub event_type: EventType,
    pub priority: Priority,
    pub deadline: Option<tf::id::StepId>,
    pub dirty_mask: DirtyMask,
}
