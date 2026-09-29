//! Rastreamento causal: hipóteses causa→efeito sustentadas por evidência.
//!
//! REAL (P1 da análise 13-0): cada hipótese carrega `hits/total` e a
//! confiança EVOLUI por outcome confirmado (razão com denominador —
//! a lição do legado é que `CauPct` sem denominador mente). Poda por
//! intervalo descarta hipóteses jamais confirmadas com contagem.

use triad_foundation as tf;
use tracing::{debug, trace};

/// Tolerância padrão para confirmar um efeito (|observado − esperado|).
/// A política real vem de `config/default.toml [l4.causal]
/// confirm_tolerance` — a const é o default congelado.
pub const CONFIRM_TOLERANCE: f32 = 0.2;

/// Hipótese causal acumulada por observações repetidas.
#[derive(Debug, Clone)]
pub struct CausalHypothesis {
    pub cause: String,
    pub effect: String,
    /// Observações que confirmaram a hipótese.
    pub hits: u32,
    /// Observações totais do par causa→efeito (denominador).
    pub total: u32,
}

impl CausalHypothesis {
    /// Taxa de confirmação com denominador; sem amostras = None
    /// (ausência ≠ zero).
    pub fn hit_rate(&self) -> Option<tf::Rate> {
        tf::Rate::from_ratio(self.hits as u64, self.total as u64)
    }

    /// Confiança corrente derivada da taxa; None sem amostras.
    pub fn confidence(&self) -> Option<tf::Confidence> {
        self.hit_rate().and_then(|r| tf::Confidence::construct(r.value()))
    }
}

/// Acumulador de hipóteses causais observadas.
pub struct CausalTracker {
    hypotheses: Vec<CausalHypothesis>,
    /// Hipóteses descartadas pela poda (telemetria honesta).
    pruned: u64,
}

impl CausalTracker {
    /// Cria rastreador causal vazio.
    pub fn new() -> Self {
        Self {
            hypotheses: Vec::new(),
            pruned: 0,
        }
    }

    /// Observa um par causa→efeito com o resultado REAL: reforça a
    /// hipótese existente ou cria nova; contadores sempre com
    /// denominador.
    pub fn observe(&mut self, cause: &str, effect: &str, confirmed: bool) {
        if let Some(h) = self
            .hypotheses
            .iter_mut()
            .find(|h| h.cause == cause && h.effect == effect)
        {
            h.total += 1;
            if confirmed {
                h.hits += 1;
            }
            trace!(
                causa = %h.cause,
                efeito = %h.effect,
                hits = h.hits,
                total = h.total,
                "hipótese causal atualizada"
            );
            return;
        }
        let h = CausalHypothesis {
            cause: cause.to_string(),
            effect: effect.to_string(),
            hits: if confirmed { 1 } else { 0 },
            total: 1,
        };
        trace!(causa = %h.cause, efeito = %h.effect, "hipótese causal nascida");
        self.hypotheses.push(h);
    }

    /// Hipótese com maior taxa de confirmação (desempate: mais amostras,
    /// depois ordem determinística); vazio devolve NO_DATA.
    pub fn best(&self) -> tf::Qualified<&CausalHypothesis> {
        let best = self.hypotheses.iter().max_by(|a, b| {
            let ra = a.hit_rate().map_or(0.0, |r| r.value());
            let rb = b.hit_rate().map_or(0.0, |r| r.value());
            ra.partial_cmp(&rb)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.total.cmp(&b.total))
        });
        match best {
            Some(h) => {
                debug!(
                    causa = %h.cause,
                    hits = h.hits,
                    total = h.total,
                    "melhor hipótese causal consultada"
                );
                tf::Qualified::value(h, tf::ModuleId::new(), tf::StepId::new())
            }
            None => {
                debug!("sem hipóteses causais (NO_DATA)");
                tf::Qualified::no_data("sem hipóteses causais", tf::ModuleId::new(), tf::StepId::new())
            }
        }
    }

    /// Percentual causal GLOBAL: soma de hits / soma de totais.
    /// Sem nenhuma observação = None (ausência ≠ zero — lição do
    /// `CauPct` do legado, que publicava taxa sem denominador).
    pub fn causal_pct(&self) -> Option<tf::Rate> {
        let total: u32 = self.hypotheses.iter().map(|h| h.total).sum();
        let hits: u32 = self.hypotheses.iter().map(|h| h.hits).sum();
        tf::Rate::from_ratio(hits as u64, total as u64)
    }

    /// Poda por intervalo: descarta hipóteses com amostras suficientes
    /// e NENHUMA confirmação (razão tipada, contada — nunca silenciosa).
    pub fn prune(&mut self, min_samples: u32) -> usize {
        let before = self.hypotheses.len();
        self.hypotheses
            .retain(|h| !(h.hits == 0 && h.total >= min_samples));
        let removed = before - self.hypotheses.len();
        self.pruned += removed as u64;
        if removed > 0 {
            debug!(descartadas = removed, "poda causal: hipóteses nunca confirmadas");
        }
        removed
    }

    /// Hipóteses descartadas pela poda ao longo da vida.
    pub fn pruned_total(&self) -> u64 {
        self.pruned
    }

    /// Número de hipóteses correntes.
    pub fn len(&self) -> usize {
        self.hypotheses.len()
    }
}

impl Default for CausalTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confianca_evolui_e_taxa_tem_denominador() {
        let mut t = CausalTracker::new();
        // Sem observações: ausência ≠ zero.
        assert!(t.causal_pct().is_none());
        t.observe("decisao:explorar", "saliencia_mantida", true);
        t.observe("decisao:explorar", "saliencia_mantida", false);
        assert!((t.causal_pct().expect("com denominador").value() - 0.5).abs() < 1e-6);
        let h = t.best().as_ref_value().copied().expect("melhor hipótese");
        assert_eq!(h.hits, 1);
        assert_eq!(h.total, 2);
        // Confiança derivada da taxa, não constante.
        assert!((h.confidence().expect("0.5").value() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn poda_descarta_nunca_confirmadas_com_contagem() {
        let mut t = CausalTracker::new();
        for _ in 0..3 {
            t.observe("decisao:x", "efeito_nunca_visto", false);
        }
        t.observe("decisao:y", "efeito_confirmado", true);
        assert_eq!(t.prune(3), 1, "só a nunca-confirmada cai");
        assert_eq!(t.pruned_total(), 1);
        assert_eq!(t.len(), 1);
    }
}
