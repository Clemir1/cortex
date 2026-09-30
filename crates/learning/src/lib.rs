//! Sistema transversal de aprendizagem: fecha o ciclo decisão→resultado→aprendizagem.

pub mod config;
pub mod credit;
pub mod effect_validator;
pub mod envelope;
pub mod forgetting;
pub mod module;
pub mod policy;

pub use config::LearningCfg;
pub use credit::{assign_credit, credit_rate, CreditEntry};
pub use effect_validator::EffectValidator;
pub use envelope::LearningEnvelope;
pub use forgetting::ForgettingEngine;
pub use module::{LearningModule, LearningSnapshot, LearningStats, LearningStatus};
pub use policy::LearningPolicy;
