//! Cybernetics: transversal de controle homeostático (ordens de Ashby O1–O5).
//! Publica métricas na chave `cybernetics.metrics`.

/// Chave de publicação das métricas cibernéticas no barramento.
pub const METRICS_KEY: &str = "cybernetics.metrics";

pub mod config;
pub mod genome;
pub mod homeostat;
pub mod metrics;
pub mod module;
pub mod o3_governor;
pub mod o4_metrics;
pub mod o5_horizon;
pub mod stability;

pub use config::{CyberneticsCfg, O5Cfg, OrderCfg};
pub use genome::{Candidate, Genome};
pub use homeostat::Homeostat;
pub use metrics::{snapshot, CyberneticMetrics};
pub use module::{CyberneticStatus, CyberneticsModule, CyberneticsStats};
pub use o3_governor::{GoverningAction, O3Governor};
pub use o4_metrics::{audit_envelope, audit_rate};
pub use o5_horizon::O5Horizon;
pub use stability::OscillationMonitor;
