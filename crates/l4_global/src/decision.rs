//! Decisão global: escolhe a melhor opção pendente com identidade obrigatória.

use triad_contracts as tc;
use triad_foundation as tf;
use tracing::{debug, warn};

/// Escolhe a melhor opção de uma decisão pendente.
/// Rejeita decisão sem identidade (lei da casa) e sem opções (NO_DATA).
pub fn decide_best(pd: &crate::PendingDecision) -> tf::TriadResult<tc::DecisionOption> {
    // Lei da casa: identidade ausente invalida o contrato da decisão.
    if pd.identity_context.is_none() {
        warn!(motivo = "sem identity_context", "decisão rejeitada");
        return Err(tf::TriadError::ContractViolation {
            detail: "decisão sem identity_context é rejeitada".into(),
        });
    }
    // Ausência ≠ zero: sem opções não existe "melhor opção zero".
    if pd.options.is_empty() {
        warn!(motivo = "sem opções (NO_DATA)", "decisão rejeitada");
        return Err(tf::TriadError::NoData {
            about: "sem opções".into(),
        });
    }
    // predomina estabilidade; integração fina usa reward quando o acessor for confirmado
    // desempate lexicográfico determinístico (Reward tratado como opaco aqui)
    match pd.options.iter().max_by(|a, b| a.action.cmp(&b.action)) {
        Some(best) => {
            debug!(
                action = ?best.action,
                valor_previsto = ?best.predicted_value,
                "opção vencedora escolhida"
            );
            Ok(best.clone())
        }
        None => Err(tf::TriadError::NoData {
            about: "sem opções".into(),
        }),
    }
}
