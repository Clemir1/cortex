//! Rastreamento causal: hipóteses causa→efeito sustentadas por evidência.

use triad_foundation as tf;
use tracing::{debug, trace};

/// Hipótese causal acumulada por observações repetidas.
#[derive(Debug, Clone)]
pub struct CausalHypothesis {
    pub cause: String,
    pub effect: String,
    pub evidence_count: u32,
    pub confidence: tf::Confidence,
}

/// Acumulador de hipóteses causais observadas.
pub struct CausalTracker {
    hypotheses: Vec<CausalHypothesis>,
}

impl CausalTracker {
    /// Cria rastreador causal vazio.
    pub fn new() -> Self {
        Self {
            hypotheses: Vec::new(),
        }
    }

    /// Observa um par causa→efeito: reforça hipótese existente ou cria nova.
    pub fn observe(&mut self, cause: &str, effect: &str) {
        if let Some(h) = self
            .hypotheses
            .iter_mut()
            .find(|h| h.cause == cause && h.effect == effect)
        {
            h.evidence_count += 1;
            return;
        }
        // construct sempre tratado (nunca .unwrap()); sem confiança válida
        // a hipótese não nasce: ausência ≠ zero.
        if let Some(confidence) = tf::Confidence::construct(0.2) {
            self.hypotheses.push(CausalHypothesis {
                cause: cause.to_string(),
                effect: effect.to_string(),
                evidence_count: 1,
                confidence,
            });
        }
    }

    /// Hipótese com maior evidência acumulada; vazio devolve NO_DATA.
    pub fn best(&self) -> tf::Qualified<&CausalHypothesis> {
        match self.hypotheses.iter().max_by_key(|h| h.evidence_count) {
            Some(h) => {
                trace!(
                    causa = %h.cause,
                    efeito = %h.effect,
                    evidencias = h.evidence_count,
                    "melhor hipótese causal consultada"
                );
                tf::Qualified::value(h, tf::ModuleId::new(), tf::StepId::new())
            }
            None => {
                debug!("sem hipóteses causais (NO_DATA)");
                tf::Qualified::no_data(
                    "sem hipóteses causais",
                    tf::ModuleId::new(),
                    tf::StepId::new(),
                )
            }
        }
    }
}
