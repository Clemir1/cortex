//! Controlador metacognitivo: intervenções com reversão obrigatória por TTL.
//!
//! REAL (seção 16.1): TTL INDIVIDUAL por intervenção (o esqueleto
//! compartilhava um contador global), teto de intervenções simultâneas
//! e baseline da métrica monitorada — o efeito é observado na própria
//! métrica que motivou a proposta; sem `observed_effect` dentro do TTL
//! a intervenção EXPIRA com razão tipada (Lei 6: reversão obrigatória).
//! O L5 NUNCA aplica política diretamente: propõe, monitora, classifica.

use crate::config::MetaControllerCfg;
use tracing::warn;
use triad_foundation as tf;

/// Registro de uma intervenção e do efeito observado (se houver).
#[derive(Clone)]
pub struct InterventionReport {
    /// Identificador da intervenção.
    pub intervention_id: tf::ModuleId,
    /// Alvo da intervenção (ex.: métrica ou subsistema monitorado).
    pub target: String,
    /// Motivo declarado da intervenção.
    pub reason: String,
    /// Valor da métrica-alvo no momento da proposta (baseline).
    pub baseline: f32,
    /// Efeito observado; `None` até haver verificação.
    pub observed_effect: Option<f32>,
}

/// Desfecho tipado de uma intervenção (nunca só um booleano).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterventionOutcome {
    /// Métrica melhorou ou estabilizou dentro do TTL (efeito observado).
    Kept,
    /// TTL esgotado sem efeito — revertida (Lei 6).
    Reverted,
    /// Recusada no registro: teto de intervenções simultâneas.
    RejectedFull,
    /// Recusada: `require_reason` e motivo vazio.
    RejectedNoReason,
}

impl InterventionOutcome {
    /// Nome canônico do desfecho.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Kept => "KEPT",
            Self::Reverted => "REVERTED",
            Self::RejectedFull => "REJECTED_FULL",
            Self::RejectedNoReason => "REJECTED_NO_REASON",
        }
    }
}

/// Controla intervenções ativas e aplica a reversão obrigatória por TTL.
pub struct MetaController {
    /// Política de `[l5.meta_controller]`.
    cfg: MetaControllerCfg,
    /// Intervenções ativas com TTL individual restante.
    active: Vec<(InterventionReport, u32)>,
}

impl MetaController {
    /// Novo controlador com TTL de 10 ticks (compatibilidade).
    pub fn new() -> Self {
        Self::with_config(MetaControllerCfg::default())
    }

    /// Controlador com a política injetada de `[l5.meta_controller]`.
    pub fn with_config(cfg: MetaControllerCfg) -> Self {
        Self {
            cfg,
            active: Vec::new(),
        }
    }

    /// Registra uma intervenção e a ativa; `None` com desfecho tipado
    /// quando recusada (teto cheio ou sem motivo exigido).
    pub fn intervene(
        &mut self,
        target: &str,
        reason: &str,
        baseline: f32,
    ) -> Result<InterventionReport, InterventionOutcome> {
        if self.cfg.require_reason && reason.trim().is_empty() {
            return Err(InterventionOutcome::RejectedNoReason);
        }
        if self.active.len() >= self.cfg.max_concurrent_interventions {
            return Err(InterventionOutcome::RejectedFull);
        }
        let report = InterventionReport {
            intervention_id: tf::ModuleId::new(),
            target: target.to_string(),
            reason: reason.to_string(),
            baseline,
            observed_effect: None,
        };
        self.active.push((report.clone(), self.cfg.intervention_ttl_steps));
        Ok(report)
    }

    /// Intervenções ativas correntes (auditoria read-only, clones).
    pub fn active_reports(&self) -> Vec<InterventionReport> {
        self.active.iter().map(|(r, _)| r.clone()).collect()
    }

    /// Marca o efeito observado na métrica-alvo; true se encontrou.
    pub fn observe_effect(&mut self, intervention_id: tf::ModuleId, effect: f32) -> bool {
        for (report, _) in self.active.iter_mut() {
            if report.intervention_id == intervention_id {
                report.observed_effect = Some(effect);
                return true;
            }
        }
        false
    }

    /// Avança um tick: valida efeitos, expira sem efeito e devolve os
    /// desfechos [Kept | Reverted] deste tick (com relatório e razão).
    pub fn tick_validate(&mut self) -> Vec<(InterventionReport, InterventionOutcome)> {
        let mut resolved = Vec::new();
        let mut keep = Vec::new();
        for (report, ttl) in self.active.drain(..) {
            // Efeito observado na métrica-alvo desde o baseline?
            if report.observed_effect.is_some() {
                resolved.push((report, InterventionOutcome::Kept));
                continue;
            }
            let ttl = ttl.saturating_sub(1);
            if ttl == 0 {
                warn!(
                    alvo = %report.target,
                    motivo = %report.reason,
                    ttl = self.cfg.intervention_ttl_steps,
                    "intervencao sem efeito observado no TTL — revertida (Lei 6)"
                );
                resolved.push((report, InterventionOutcome::Reverted));
            } else {
                keep.push((report, ttl));
            }
        }
        self.active = keep;
        resolved
    }

    /// Compatibilidade: tick de reversão sem classificação de keep.
    pub fn tick_revert(&mut self) -> Vec<InterventionReport> {
        self.tick_validate()
            .into_iter()
            .filter(|(_, outcome)| *outcome == InterventionOutcome::Reverted)
            .map(|(report, _)| report)
            .collect()
    }
}

impl Default for MetaController {
    /// Estado inicial: política default.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ttl_individual_cada_intervencao_tem_o_proprio_relogio() {
        let cfg = MetaControllerCfg {
            intervention_ttl_steps: 3,
            max_concurrent_interventions: 4,
            ..MetaControllerCfg::default()
        };
        let mut m = MetaController::with_config(cfg);
        let a = m.intervene("l4.verificacao", "confirmacao_baixa", 0.4).expect("a");
        // Segunda nasce um tick depois: TTL individual, não compartilhado.
        m.tick_validate();
        let _b = m.intervene("l4.timeout", "expirou", 0.1).expect("b");
        // A observa efeito antes de expirar.
        assert!(m.observe_effect(a.intervention_id, 0.9));
        // Tick 2: a é KEPT (efeito); b ainda tem TTL.
        let outcomes = m.tick_validate();
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].1, InterventionOutcome::Kept);
        // Ticks 3 e 4: b expira sozinho no seu próprio prazo.
        assert!(m.tick_validate().is_empty());
        let outcomes = m.tick_validate();
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].1, InterventionOutcome::Reverted);
    }

    #[test]
    fn teto_de_concorrencia_e_motivo_exigido() {
        let cfg = MetaControllerCfg {
            intervention_ttl_steps: 10,
            max_concurrent_interventions: 2,
            require_reason: true,
            ..MetaControllerCfg::default()
        };
        let mut m = MetaController::with_config(cfg);
        assert!(m.intervene("t1", "r1", 0.0).is_ok());
        assert!(m.intervene("t2", "r2", 0.0).is_ok());
        assert!(matches!(
            m.intervene("t3", "r3", 0.0),
            Err(InterventionOutcome::RejectedFull)
        ));
        // Sem motivo: recusada mesmo com vagas.
        let mut m2 = MetaController::with_config(MetaControllerCfg {
            require_reason: true,
            ..MetaControllerCfg::default()
        });
        assert!(matches!(
            m2.intervene("t", "   ", 0.0),
            Err(InterventionOutcome::RejectedNoReason)
        ));
    }
}
