//! Eventos e estado de tecidos neurais (L2).

use triad_foundation as tf;

use serde::{Deserialize, Serialize};

/// Transições estruturais de um tecido.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TissueKind {
    Formed,
    Specialized,
    Integrated,
    Fragmented,
    Damaged,
    Repaired,
}

/// Evento estrutural de um tecido neural.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TissueEvent {
    pub event_id: tf::id::EventId,
    pub tissue_id: tf::id::TissueId,
    pub kind: TissueKind,
}

/// Estado agregado de um tecido e seus clusters membros.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TissueState {
    pub tissue_id: tf::id::TissueId,
    pub cohesion: tf::units::Confidence,
    pub specialization: tf::units::Confidence,
    pub integration: tf::units::Confidence,
    pub member_clusters: Vec<tf::id::ClusterId>,
}
