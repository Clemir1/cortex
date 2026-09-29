//! Fundação de tipos: identidades, status, valor tipado (ausência ≠ zero),
//! tempo lógico, erro, proveniência/evidência, versionamento, unidades.
//!
//! Este crate não conhece camadas — é a base comum de todos os contratos
//! L1–L5.
//!
//! ## Convenção de acesso
//!
//! Todos os tipos públicos são re-exportados **na raiz** como API estável
//! para consumidores (contracts, camadas L1–L5, tests): `triad_foundation::Energy`
//! etc. Os caminhos completos por módulo (`triad_foundation::units::Energy`)
//! permanecem válidos — reorganização interna não quebra consumidor.
//! (Decisão: os contratos já consomem a raiz; divergência exigiria ADR.)

// Permite que os testes embutidos usem os caminhos completos
// `triad_foundation::...`, como qualquer consumidor externo.
#[cfg(test)]
extern crate self as triad_foundation;

pub mod error;
pub mod evidence;
pub mod id;
pub mod provenance;
pub mod status;
pub mod time;
pub mod units;
pub mod value;
pub mod version;

// API estável na raiz (ver convenção no doc acima). Sem colisões de nome
// entre os módulos — cada tipo tem um único caminho raiz.
pub use error::{TriadError, TriadResult};
pub use evidence::EvidenceLevel;
pub use id::{
    ClusterId, ConceptId, DecisionId, EpisodeId, EventId, ModuleId, RunId, StepId, TissueId,
};
pub use provenance::Provenance;
pub use status::Status;
pub use time::{LogicalClock, Timestamp};
pub use units::{Aversion, Confidence, Energy, Rate, Reward, Salience, Stress};
pub use value::Qualified;
pub use version::SchemaVersion;
