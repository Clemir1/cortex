//! Escada de evidência E0–E5 (doc/README.md §3).

use serde::{Deserialize, Serialize};
use std::fmt;

/// Nível de evidência de um elo da cadeia.
///
/// Sem promoção automática: cada nível exige prova própria.
/// E5 exige efeito identificado no tempo (t+1/t+5 com sham pareado
/// quando houver mudança comportamental). A ordem das variantes
/// define a ordem total: `E0Declared < E1Initialized < ... < E5EffectValidated`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EvidenceLevel {
    /// Declarado. Não é existência funcional.
    #[serde(rename = "E0")]
    E0Declared,
    /// Inicializado. Não é execução produtiva.
    #[serde(rename = "E1")]
    E1Initialized,
    /// Executado. Não é saída válida nem consumo.
    #[serde(rename = "E2")]
    E2Executed,
    /// Produtivo: estado/saída nova e válida. Não é integração.
    #[serde(rename = "E3")]
    E3Productive,
    /// Consumido por consumidor identificado. Não é efeito downstream.
    #[serde(rename = "E4")]
    E4Consumed,
    /// Efeito posterior validado no tempo. Não é maturidade global.
    #[serde(rename = "E5")]
    E5EffectValidated,
}

impl EvidenceLevel {
    /// Nome canônico (`"E0"`..=`"E5"`).
    pub const fn as_str(&self) -> &'static str {
        match self {
            EvidenceLevel::E0Declared => "E0",
            EvidenceLevel::E1Initialized => "E1",
            EvidenceLevel::E2Executed => "E2",
            EvidenceLevel::E3Productive => "E3",
            EvidenceLevel::E4Consumed => "E4",
            EvidenceLevel::E5EffectValidated => "E5",
        }
    }
}

impl fmt::Display for EvidenceLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
