//! L5 Metacognição: observa L1-L4, regula crises e mantém a identidade.
//! Promoção de status é sempre manual (leis E0-E5).

pub mod identity;
pub mod governor;
pub mod meta_controller;
pub mod wake;
pub mod module;

pub use governor::LimbicGovernor;
pub use identity::{IdentityArc, IdentityStore};
pub use meta_controller::{InterventionReport, MetaController};
pub use wake::{should_wake, WakeReason};
pub use module::L5Module;
