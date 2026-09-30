//! Cognição local (L3): semântica, memória, atenção, predição e metas.

/// Campo de atenção com limiar e capacidade limitada de focos.
pub mod attention;

/// Metas da cognição local.
pub mod goals;

/// Memória episódica local.
pub mod memory;

/// Ponte da L3 para o runtime cognitivo.
pub mod module;

/// Predição de eventos futuros a partir de evidências.
pub mod prediction;

/// 17.5: representação real — ConceptStore, binding, memória
/// ontogenética janelada, ponte temporal e metaestabilidade.
pub mod representation;

/// Reforço de memória L4→L3 (seção 16.2 — trilha do ciclo confirmado).
pub mod reinforcement;

/// Sinais top-down tipados L3→L2 (17.4).
pub mod topdown;

/// Campo semântico de conceitos da cognição local.
pub mod semantic;

pub use attention::{AttentionCfg, AttentionField, AttentionFoci};
pub use goals::{Goal, GoalStack};
pub use memory::{Episode, LocalMemory, ResonanceSignature};
pub use module::{L3Config, L3Module, L3Policy, L3Snapshot};
pub use prediction::{Prediction, PredictionCfg, Predictor};
pub use reinforcement::{Reinforcement, ReinforcementReceipt};
pub use representation::{
    metaestability, BindingEdge, ConceptRecord, ConceptStore, MetaReport,
    OntogeneticMemory, OntoStats, SemanticBinding, SemanticRepresentation,
    TemporalRepresentation, TraceVerdict, ACTIVATION_DECAY, ATTRACTOR_THRESHOLD,
    CONSOLIDATION_WINDOW,
};
pub use semantic::{Concept, SemanticField};
