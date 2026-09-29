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

/// Reforço de memória L4→L3 (seção 16.2 — trilha do ciclo confirmado).
pub mod reinforcement;

/// Campo semântico de conceitos da cognição local.
pub mod semantic;

pub use attention::{AttentionCfg, AttentionField, AttentionFoci};
pub use goals::{Goal, GoalStack};
pub use memory::{Episode, LocalMemory, ResonanceSignature};
pub use module::{L3Config, L3Module, L3Policy, L3Snapshot};
pub use prediction::{Prediction, PredictionCfg, Predictor};
pub use reinforcement::{Reinforcement, ReinforcementReceipt};
pub use semantic::{Concept, SemanticField};
