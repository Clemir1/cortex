//! Sistema transversal de aprendizagem: fecha o ciclo decisão→resultado→aprendizagem.

pub mod credit;
pub mod effect_validator;
pub mod envelope;
pub mod forgetting;
pub mod policy;

pub use credit::{assign_credit, credit_rate, CreditEntry};
pub use effect_validator::EffectValidator;
pub use envelope::LearningEnvelope;
pub use forgetting::ForgettingEngine;
pub use policy::LearningPolicy;
