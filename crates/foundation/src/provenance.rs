//! Proveniência de um valor tipado.

use crate::evidence::EvidenceLevel;
use crate::id::{EventId, ModuleId};
use serde::{Deserialize, Serialize};

/// Cadeia de origem de um valor: de onde veio e com qual prova.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Provenance {
    /// Eventos que produziram o valor (cadeia ascendente).
    pub source_chain: Vec<EventId>,
    /// Nível de evidência do elo produtor.
    pub evidence_level: EvidenceLevel,
    /// Módulo que forneceu o valor.
    pub provider_id: ModuleId,
}
