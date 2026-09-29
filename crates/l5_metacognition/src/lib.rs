//! L5 Metacognição: observa L1-L4, regula crises e mantém a identidade.
//! Promoção de status é sempre manual (leis E0-E5).
//!
//! REAL (seção 16.1): identidade derivada do comportamento real do L4,
//! límbico com histerese, metacontrolador com TTL individual (Lei 6),
//! wake por causa declarada — tudo com política de `[l5.*]` injetada.

pub mod config;
pub mod identity;
pub mod governor;
pub mod meta_controller;
pub mod module;
pub mod wake;

pub use config::L5Config;
pub use governor::{BandReason, LimbicBand, LimbicGovernor};
pub use identity::{IdentityArc, IdentityStore, IdentityValues};
pub use meta_controller::{InterventionOutcome, InterventionReport, MetaController};
pub use module::{L5Module, L5Stats};
pub use wake::{should_wake, WakeReason};
