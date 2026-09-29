//! Erro canônico da fundação.

use crate::id::ModuleId;

/// Erro do organismo. Cada variante ausente (`NoData`, `Stale`, `Invalid`,
/// `Fallback`) espelha um [`crate::status::Status`] não publicável.
#[derive(thiserror::Error, Debug)]
pub enum TriadError {
    /// Sem provider ou sem amostra elegível.
    #[error("sem dado: {about}")]
    NoData {
        /// Sobre o que falta dado.
        about: String,
    },
    /// Fora da janela temporal.
    #[error("stale: {about}")]
    Stale {
        /// Sobre o que está stale.
        about: String,
    },
    /// Rejeitado por contrato.
    #[error("inválido: {about} — {reason}")]
    Invalid {
        /// Sobre o que foi rejeitado.
        about: String,
        /// Motivo da rejeição.
        reason: String,
    },
    /// Fallback estrutural usado no lugar do valor medido.
    #[error("fallback de {provider:?} — {about}: {reason}")]
    Fallback {
        /// Sobre o que foi usado fallback.
        about: String,
        /// Motivo estrutural do fallback.
        reason: String,
        /// Módulo que forneceu o fallback.
        provider: ModuleId,
    },
    /// Quebra de lei-contrato (contratos_camadas.md §5).
    #[error("violação de contrato: {detail}")]
    ContractViolation {
        /// Descrição da quebra.
        detail: String,
    },
    /// Erro de serialização.
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
    /// Erro de E/S.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Resultado canônico do organismo.
pub type TriadResult<T> = Result<T, TriadError>;
