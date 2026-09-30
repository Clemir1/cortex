//! Development: forma e manutenção do organismo — determinístico no boot, amostral no quente.
//! Transversal morfológico: embriogênese, ESTÁGIOS por banda de ressonância
//! (16.6, sinal Chladni), corticalização com métricas vivas e parada,
//! manutenção O(1), regeneração orçada por estágio, morfogênese em standby.

//! Módulos públicos do transversal de development.
pub mod config;
pub mod corticalization;
pub mod embryogenesis;
pub mod maintenance;
pub mod module;
pub mod morphogenesis;
pub mod regeneration;
pub mod stages;

pub use config::DevelopmentCfg;
pub use corticalization::{Corticalization, StopCriterion};
pub use embryogenesis::{EmbryoPlan, genesis};
pub use maintenance::{Maintenance, DEFAULT_SAMPLE};
pub use module::{DevelopmentModule, DevelopmentStats, DevelopmentStatus};
pub use morphogenesis::{Morphogenesis, WakeReason, DEFAULT_STANDBY};
pub use regeneration::{DamageKind, Regeneration, QUEUE_CAP};
pub use stages::{
    band_for, DevelopmentStage, StageSnapshot, StageTick, StageTracker, StagesCfg,
};
