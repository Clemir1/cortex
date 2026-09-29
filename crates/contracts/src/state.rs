//! Estado versionado e visões por contrato entre camadas.

use triad_foundation as tf;

use serde::{Deserialize, Serialize};

/// Fase do estado de um cluster na pipeline por passos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatePhase {
    PostPhysical,
    PostTissue,
    PreCognitive,
    PostCognitive,
}

/// Modo de plasticidade vigente; `Bypassed` carrega o motivo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlasticityMode {
    Hebbian,
    Bypassed { reason: String },
}

/// Visão por contrato do estado de um cluster, cruzando camadas.
///
/// Ausência ≠ zero — amostra vazia = NoData, nunca zero.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClusterStateRef {
    pub cluster_id: tf::id::ClusterId,
    pub energy: tf::value::Qualified<tf::units::Energy>,
    pub stress: tf::value::Qualified<tf::units::Stress>,
    pub activity: tf::value::Qualified<f32>,
    pub phase: StatePhase,
    pub neighbors: Vec<tf::id::ClusterId>,
    pub plasticity: PlasticityMode,
}

/// Valor carimbado com versão monótona.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Versioned<T> {
    pub value: T,
    pub version: u64,
}

/// Delta incremental de estado de uma entidade.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Delta {
    pub entity_id: String,
    pub changes: Vec<(String, String)>,
}

/// Linhagem de um checkpoint: run pai, hashes e versão de schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckpointLineage {
    pub parent_run_id: tf::id::RunId,
    pub checkpoint_hash: String,
    pub state_hash: String,
    pub schema_version: tf::version::SchemaVersion,
}
