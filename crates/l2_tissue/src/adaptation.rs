//! Gate de adaptação estrutural L3→L2 — o elo que no legado existia e
//! estava INERTE; aqui é first-class, com histerese contada.
//!
//! Regras (config `[l2.adaptation]`):
//! - pedidos chegam como `AdaptationRequest` (parâmetro/valor em string);
//! - micro-ajustes dentro da `hysteresis_band` são ADIADOS (insignificantes);
//! - saltos acima de `max_delta_per_request` são CLAMPADOS ao teto na
//!   direção pedida (o pedido anda, nunca pula);
//! - cadência mínima `min_interval_steps` POR PARÂMETRO entre aplicações;
//! - valores fora da faixa estrutural do parâmetro são rejeitados com razão;
//! - NUNCA desliga sistemas (a crise muda política, não existência).
//!
//! O valor `current` é SEMPRE lido do estado real do L2 — o valor
//! declarado pelo L3 no pedido é apenas sugestão.

use std::collections::HashMap;

/// Parâmetros estruturais ajustáveis do L2 por feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdaptParam {
    AffinityThreshold,
    MinMembers,
    MaxMembers,
    CoherenceTarget,
    BridgeMinEdges,
    /// 17.14: limiar de especialização do feedback top-down
    /// (L3→L2) — fecha o canal `l2.tissue.specialization_threshold`.
    SpecializationThreshold,
}

impl AdaptParam {
    /// Chave canônica (casa com `config/default.toml`).
    pub const fn as_key(self) -> &'static str {
        match self {
            Self::AffinityThreshold => "l2.affinity.threshold",
            Self::MinMembers => "l2.tissues.min_members",
            Self::MaxMembers => "l2.tissues.max_members",
            Self::CoherenceTarget => "l2.tissues.coherence_target",
            Self::BridgeMinEdges => "l2.affinity.bridge_min_edges",
            Self::SpecializationThreshold => "l2.tissue.specialization_threshold",
        }
    }

    /// Resolve pela chave canônica (case-insensitive, aceita também o
    /// nome curto do parâmetro).
    pub fn from_key(key: &str) -> Option<Self> {
        Some(match key.trim().to_ascii_lowercase().as_str() {
            "l2.affinity.threshold" | "affinity_threshold" => Self::AffinityThreshold,
            "l2.tissues.min_members" | "min_members" => Self::MinMembers,
            "l2.tissues.max_members" | "max_members" => Self::MaxMembers,
            "l2.tissues.coherence_target" | "coherence_target" => Self::CoherenceTarget,
            "l2.affinity.bridge_min_edges" | "bridge_min_edges" => Self::BridgeMinEdges,
            "l2.tissue.specialization_threshold" | "specialization_threshold" => {
                Self::SpecializationThreshold
            }
            _ => return None,
        })
    }

    /// Faixa estrutural válida (fora disso é rejeitado com razão).
    pub const fn range(self) -> (f64, f64) {
        match self {
            Self::AffinityThreshold => (0.05, 0.95),
            Self::MinMembers => (2.0, 16.0),
            Self::MaxMembers => (8.0, 256.0),
            Self::CoherenceTarget => (0.1, 0.9),
            Self::BridgeMinEdges => (1.0, 8.0),
            // Paridade com SPEC_THRESHOLD_MIN/MAX do topdown.
            Self::SpecializationThreshold => (0.05, 0.95),
        }
    }

    /// Parâmetro de domínio discreto (unidades inteiras): o clamp de
    /// salto tem piso de 1 unidade — senão um `max_delta = 0.1`
    /// "aplicaria" sem nunca mudar o inteiro.
    pub const fn is_discrete(self) -> bool {
        matches!(self, Self::MinMembers | Self::MaxMembers | Self::BridgeMinEdges)
    }
}

/// Decisão do gate para um pedido.
#[derive(Debug, Clone, PartialEq)]
pub enum GateDecision {
    /// Aplicado (valor pode ter sido clampado ao salto máximo).
    Applied {
        parameter: AdaptParam,
        old_value: f64,
        new_value: f64,
        clamped: bool,
    },
    /// Adiado com razão (nunca silêncio).
    Deferred { reason: &'static str },
}

/// Gate com histerese por parâmetro.
pub struct AdaptationGate {
    /// `[l2.adaptation] feedback_enabled`.
    pub enabled: bool,
    /// `[l2.adaptation] hysteresis_band` — dentro da banda, adia.
    pub hysteresis_band: f64,
    /// `[l2.adaptation] min_interval_steps` — cadência por parâmetro.
    pub min_interval_steps: u64,
    /// `[l2.adaptation] max_delta_per_request` — salto máximo aplicado.
    pub max_delta: f64,
    /// Último (step, valor aplicado) por parâmetro.
    last_applied: HashMap<AdaptParam, (u64, f64)>,
    // Contadores (a telemetria nunca mente).
    pub received: u64,
    pub applied: u64,
    pub applied_clamped: u64,
    pub deferred_band: u64,
    pub deferred_interval: u64,
    pub deferred_range: u64,
    pub deferred_invalid: u64,
    pub deferred_unknown: u64,
    pub deferred_unparseable: u64,
}

impl AdaptationGate {
    pub fn new() -> Self {
        Self::new_with_config(&crate::config::AdaptationCfg::default())
    }

    /// Cria o gate com a política injetada de `config/default.toml
    /// [l2.adaptation]` (banda, cadência, teto por pedido).
    pub fn new_with_config(cfg: &crate::config::AdaptationCfg) -> Self {
        Self {
            enabled: cfg.feedback_enabled,
            hysteresis_band: cfg.hysteresis_band,
            min_interval_steps: cfg.min_interval_steps,
            max_delta: cfg.max_delta_per_request,
            last_applied: HashMap::new(),
            received: 0,
            applied: 0,
            applied_clamped: 0,
            deferred_band: 0,
            deferred_interval: 0,
            deferred_range: 0,
            deferred_invalid: 0,
            deferred_unknown: 0,
            deferred_unparseable: 0,
        }
    }

    /// Pedido em formato de mensagem (chave + strings), como chega do
    /// L3: resolve o parâmetro, parseia o valor e aplica a histerese.
    pub fn request_str(
        &mut self,
        parameter_key: &str,
        current: f64,
        proposed_str: &str,
        step: u64,
    ) -> GateDecision {
        let Some(parameter) = AdaptParam::from_key(parameter_key) else {
            self.received += 1;
            self.deferred_unknown += 1;
            return GateDecision::Deferred {
                reason: "unknown_parameter",
            };
        };
        let Ok(proposed) = proposed_str.trim().parse::<f64>() else {
            self.received += 1;
            self.deferred_unparseable += 1;
            return GateDecision::Deferred {
                reason: "value_unparseable",
            };
        };
        self.request(parameter, current, proposed, step)
    }

    /// Avalia um pedido contra a histerese. `current` é o valor vigente
    /// lido do estado REAL do L2.
    pub fn request(
        &mut self,
        parameter: AdaptParam,
        current: f64,
        proposed: f64,
        step: u64,
    ) -> GateDecision {
        self.received += 1;
        if !self.enabled {
            return GateDecision::Deferred {
                reason: "feedback_disabled",
            };
        }
        if !proposed.is_finite() || !current.is_finite() {
            self.deferred_invalid += 1;
            return GateDecision::Deferred {
                reason: "value_non_finite",
            };
        }
        let (lo, hi) = parameter.range();
        if !(lo..=hi).contains(&proposed) {
            self.deferred_range += 1;
            return GateDecision::Deferred {
                reason: "out_of_range",
            };
        }
        let delta = proposed - current;
        if delta.abs() <= self.hysteresis_band {
            self.deferred_band += 1;
            return GateDecision::Deferred {
                reason: "within_hysteresis_band",
            };
        }
        if let Some(&(last_step, _)) = self.last_applied.get(&parameter) {
            if step < last_step.saturating_add(self.min_interval_steps) {
                self.deferred_interval += 1;
                return GateDecision::Deferred {
                    reason: "min_interval",
                };
            }
        }
        // Salto maior que o máximo: clampa ao teto na direção pedida.
        // Parâmetros discretos andam no mínimo 1 unidade (piso do clamp).
        let clamp_step = if parameter.is_discrete() {
            self.max_delta.max(1.0)
        } else {
            self.max_delta
        };
        let (new_value, clamped) = if delta.abs() > clamp_step {
            (current + clamp_step * delta.signum(), true)
        } else {
            (proposed, false)
        };
        self.applied += 1;
        if clamped {
            self.applied_clamped += 1;
        }
        self.last_applied.insert(parameter, (step, new_value));
        GateDecision::Applied {
            parameter,
            old_value: current,
            new_value,
            clamped,
        }
    }

    /// Aplica a decisão nos parâmetros de formação (o único caminho de
    /// mutação estrutural do L2).
    pub fn apply_to_params(
        decision: &GateDecision,
        params: &mut crate::formation::FormationParams,
    ) {
        if let GateDecision::Applied { parameter, new_value, .. } = decision {
            let new_value = *new_value;
            match parameter {
                AdaptParam::AffinityThreshold => {
                    params.affinity_threshold = new_value;
                }
                AdaptParam::MinMembers => {
                    params.min_members = new_value.round() as usize;
                }
                AdaptParam::MaxMembers => {
                    params.max_members = new_value.round() as usize;
                }
                AdaptParam::CoherenceTarget => {
                    params.coherence_target = new_value;
                }
                AdaptParam::BridgeMinEdges => {
                    params.bridge_min_edges = new_value.round() as usize;
                }
                AdaptParam::SpecializationThreshold => {
                    params.specialization_threshold = new_value;
                }
            }
            params.sanitize();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dentro_da_banda_e_adiado() {
        let mut g = AdaptationGate::new();
        let d = g.request(AdaptParam::AffinityThreshold, 0.25, 0.30, 100);
        assert!(matches!(
            d,
            GateDecision::Deferred { reason: "within_hysteresis_band" }
        ));
        assert_eq!(g.deferred_band, 1);
        assert_eq!(g.applied, 0);
    }

    #[test]
    fn salto_grande_e_clampado_na_direcao() {
        let mut g = AdaptationGate::new();
        let d = g.request(AdaptParam::AffinityThreshold, 0.25, 0.60, 100);
        match d {
            GateDecision::Applied { old_value, new_value, clamped, .. } => {
                assert!((old_value - 0.25).abs() < 1e-12);
                assert!((new_value - 0.35).abs() < 1e-12); // 0.25 + 0.10
                assert!(clamped);
            }
            other => panic!("esperava Applied, veio {other:?}"),
        }
        assert_eq!(g.applied_clamped, 1);
    }

    #[test]
    fn continuo_anda_o_delta_exato_abaixo_do_teto() {
        let mut g = AdaptationGate::new();
        // 0.25 → 0.4: delta 0.15 > banda 0.1 e > teto 0.1 → clamp 0.35.
        let d = g.request(AdaptParam::AffinityThreshold, 0.25, 0.40, 100);
        match d {
            GateDecision::Applied { new_value, clamped, .. } => {
                assert!((new_value - 0.35).abs() < 1e-12);
                assert!(clamped);
            }
            other => panic!("esperava Applied, veio {other:?}"),
        }
    }

    #[test]
    fn cadencia_minima_por_parametro() {
        let mut g = AdaptationGate::new();
        g.request(AdaptParam::CoherenceTarget, 0.5, 0.7, 100); // aplica
        let d = g.request(AdaptParam::CoherenceTarget, 0.6, 0.85, 105);
        assert!(matches!(d, GateDecision::Deferred { reason: "min_interval" }));
        // Outro parâmetro não é afetado pela cadência do primeiro.
        let d2 = g.request(AdaptParam::AffinityThreshold, 0.25, 0.45, 105);
        assert!(matches!(d2, GateDecision::Applied { .. }));
        // Após o intervalo, aplica.
        let d3 = g.request(AdaptParam::CoherenceTarget, 0.6, 0.85, 120);
        assert!(matches!(d3, GateDecision::Applied { .. }));
    }

    #[test]
    fn fora_da_faixa_e_rejeitado_com_razao() {
        let mut g = AdaptationGate::new();
        let d = g.request(AdaptParam::MinMembers, 3.0, 100.0, 100);
        assert!(matches!(d, GateDecision::Deferred { reason: "out_of_range" }));
        let d2 = g.request(AdaptParam::AffinityThreshold, 0.25, f64::NAN, 100);
        assert!(matches!(d2, GateDecision::Deferred { reason: "value_non_finite" }));
    }

    #[test]
    fn aplicacao_altera_params_com_sanitize() {
        let mut g = AdaptationGate::new();
        let mut params = crate::formation::FormationParams::default();
        // Discreto: clamp anda no mínimo 1 unidade (64 → 65).
        let d = g.request(AdaptParam::MaxMembers, 64.0, 90.0, 100);
        AdaptationGate::apply_to_params(&d, &mut params);
        assert_eq!(params.max_members, 65);
        assert!(matches!(
            &d,
            GateDecision::Applied { clamped: true, .. }
        ));
    }

    #[test]
    fn chave_canonica_resolver() {
        assert_eq!(
            AdaptParam::from_key("l2.affinity.threshold"),
            Some(AdaptParam::AffinityThreshold)
        );
        assert_eq!(
            AdaptParam::from_key("l2.tissues.max_members"),
            Some(AdaptParam::MaxMembers)
        );
        assert_eq!(AdaptParam::from_key("nao.existe"), None);
        assert_eq!(AdaptParam::MinMembers.as_key(), "l2.tissues.min_members");
    }

    #[test]
    fn specialization_threshold_fecha_o_canal_l3_l2() {
        // 17.14: o pedido real do wiring 17.4 era Deferred
        // "unknown_parameter" — agora resolve e APLICA de verdade.
        assert_eq!(
            AdaptParam::from_key("l2.tissue.specialization_threshold"),
            Some(AdaptParam::SpecializationThreshold)
        );
        assert_eq!(
            AdaptParam::SpecializationThreshold.as_key(),
            "l2.tissue.specialization_threshold"
        );
        assert_eq!(AdaptParam::SpecializationThreshold.range(), (0.05, 0.95));
        let mut g = AdaptationGate::new();
        // 0.60→0.45: salto 0.15 > banda 0.1 → APLICA clampado ao
        // teto por pedido (0.10): o limiar ANDA para 0.50.
        let d = g.request(AdaptParam::SpecializationThreshold, 0.60, 0.45, 100);
        assert!(
            matches!(d, GateDecision::Applied { new_value, clamped: true, .. }
                if (new_value - 0.50).abs() < 1e-9),
            "aplica clampado ao teto na direção pedida"
        );
        let mut params = crate::formation::FormationParams::default();
        AdaptationGate::apply_to_params(&d, &mut params);
        assert!(
            (params.specialization_threshold - 0.50).abs() < 1e-9,
            "efeito REAL nos parâmetros do L2"
        );
        // Fora da faixa: rejeitado com razão tipada.
        let fora = g.request(AdaptParam::SpecializationThreshold, 0.60, 5.0, 100);
        assert!(matches!(fora, GateDecision::Deferred { reason: "out_of_range" }));
    }

    #[test]
    fn request_str_valida_unknown_e_unparseable() {
        let mut g = AdaptationGate::new();
        let d = g.request_str("l2.nada.disso", 0.25, "0.4", 100);
        assert!(matches!(d, GateDecision::Deferred { reason: "unknown_parameter" }));
        let d2 = g.request_str("l2.affinity.threshold", 0.25, "banana", 100);
        assert!(matches!(d2, GateDecision::Deferred { reason: "value_unparseable" }));
        // Válido: aplica com clamp.
        let d3 = g.request_str("l2.affinity.threshold", 0.25, "0.6", 100);
        assert!(matches!(d3, GateDecision::Applied { clamped: true, .. }));
        assert_eq!(g.received, 3);
        assert_eq!(g.deferred_unknown, 1);
        assert_eq!(g.deferred_unparseable, 1);
        assert_eq!(g.applied, 1);
    }
}
