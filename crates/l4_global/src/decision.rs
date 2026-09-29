//! Decisão global: escolhe a melhor opção pendente com identidade obrigatória.
//!
//! REAL (P1 da análise 13-0): a seleção é pelo VALOR PREVISTO
//! (`Reward`, decrescente), depois CONFIANÇA da fonte (decrescente)
//! e por fim a ação em ordem lexicográfica — determinismo total. O
//! `commit_threshold` (defer/expira) fica no `L4Module`, que conhece
//! a política de `[l4.decision]` injetada.

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
    // Seleção REAL: maior valor previsto; empate → maior confiança;
    // empate → ordem lexicográfica da ação (determinismo audível).
    let best = pd
        .options
        .iter()
        .max_by(|a, b| {
            a.predicted_value
                .value()
                .partial_cmp(&b.predicted_value.value())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    a.confidence
                        .value()
                        .partial_cmp(&b.confidence.value())
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
                .then(b.action.cmp(&a.action))
        })
        .expect("opções não vazias garantidas acima");
    debug!(
        action = %best.action,
        valor_previsto = best.predicted_value.value(),
        confianca = best.confidence.value(),
        "opção vencedora escolhida"
    );
    Ok(best.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opt(action: &str, value: f32, confidence: f32) -> tc::DecisionOption {
        tc::DecisionOption {
            action: action.into(),
            predicted_value: tf::Reward::construct(value).expect("faixa válida"),
            confidence: tf::Confidence::construct(confidence).expect("faixa válida"),
        }
    }

    fn pending(options: Vec<tc::DecisionOption>) -> crate::PendingDecision {
        crate::PendingDecision {
            event_id: tf::id::EventId::new(),
            options,
            identity_context: Some(tc::IdentityContext {
                values: vec!["growth".into()],
                continuity: tf::Confidence::construct(1.0).expect("1.0 válido"),
                source_module: tf::id::ModuleId::new(),
            }),
            limbic_modulation: None,
            expected_outcome: "teste".into(),
        }
    }

    #[test]
    fn seleciona_por_valor_previsto_e_confianca_com_determinismo() {
        let pd = pending(vec![
            opt("a-explorar", 0.5, 0.9),
            opt("b-consolidar", 0.8, 0.4), // maior valor ⇒ vence
            opt("c-manter", 0.8, 0.6),    // empate de valor ⇒ maior confiança
        ]);
        let best = decide_best(&pd).expect("seleção");
        assert_eq!(best.action, "c-manter", "valor 0.8 com confiança 0.6");
    }

    #[test]
    fn sem_identidade_ou_sem_opcoes_rejeita() {
        let mut pd = pending(vec![opt("a", 0.5, 0.5)]);
        pd.identity_context = None;
        assert!(decide_best(&pd).is_err(), "identidade obrigatória");
        let pd = pending(Vec::new());
        assert!(decide_best(&pd).is_err(), "ausência ≠ zero");
    }
}
