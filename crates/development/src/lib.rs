//! Development: forma e manutenção do organismo — determinístico no boot, amostral no quente.
//! Transversal morfológico: embriogênese, corticalização com parada, manutenção O(1), regeneração orçada, morfogênese em standby.

//! Módulos públicos do transversal de development.
pub mod corticalization;
pub mod embryogenesis;
pub mod maintenance;
pub mod module;
pub mod morphogenesis;
pub mod regeneration;

pub use corticalization::{Corticalization, StopCriterion};
pub use embryogenesis::{EmbryoPlan, genesis};
pub use maintenance::{Maintenance, DEFAULT_SAMPLE};
pub use module::{DevelopmentModule, DevelopmentStatus};
pub use morphogenesis::{Morphogenesis, WakeReason, DEFAULT_STANDBY};
pub use regeneration::{DamageKind, Regeneration, QUEUE_CAP};
