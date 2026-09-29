//! Eventos de memória episódica (L2/L3).

use triad_foundation as tf;

use serde::{Deserialize, Serialize};

/// Transições do ciclo de vida de uma memória.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryKind {
    Stored,
    Reactivated,
    Consolidated,
    Reconsolidated,
    Decayed,
}

/// Evento de memória; `require_consume_for_validity`: memória de trabalho só valida se consumida.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryEvent {
    pub event_id: tf::id::EventId,
    pub episode_id: tf::id::EpisodeId,
    pub kind: MemoryKind,
    pub strength: tf::units::Confidence,
    pub require_consume: bool,
}
