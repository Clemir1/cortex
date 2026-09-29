//! Plataforma do Triad_AEE: hospedagem, configuração e telemetria.
//! Centraliza as chaves de contexto usadas por todas as camadas.

pub mod config;
pub mod keys;
pub mod telemetry;

pub use config::PlatformConfig;
pub use telemetry::Counters;
