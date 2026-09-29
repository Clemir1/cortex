//! Controlador metacognitivo: intervenções com reversão obrigatória por TTL.

use triad_foundation as tf;

/// Registro de uma intervenção e do efeito observado (se houver).
#[derive(Clone)]
pub struct InterventionReport {
    /// Identificador da intervenção.
    pub intervention_id: tf::ModuleId,
    /// Alvo da intervenção (ex.: módulo ou subsistema).
    pub target: String,
    /// Motivo declarado da intervenção.
    pub reason: String,
    /// Efeito observado; `None` até haver verificação.
    pub observed_effect: Option<f32>,
}

/// Controla intervenções ativas e aplica a reversão obrigatória por TTL.
pub struct MetaController {
    /// Ticks restantes no ciclo corrente antes da reversão.
    ttl: u32,
    /// Intervenções atualmente ativas.
    active: Vec<InterventionReport>,
}

impl MetaController {
    /// Novo controlador com TTL de 10 ticks.
    pub fn new() -> Self {
        Self { ttl: 10, active: Vec::new() }
    }

    /// Registra uma intervenção e a ativa; devolve o relatório.
    pub fn intervene(&mut self, target: &str, reason: &str) -> InterventionReport {
        let report = InterventionReport {
            intervention_id: tf::ModuleId::new(),
            target: target.to_string(),
            reason: reason.to_string(),
            observed_effect: None,
        };
        self.active.push(report.clone());
        report
    }

    /// Marca o efeito observado; true se a intervenção foi encontrada.
    pub fn observe_effect(&mut self, intervention_id: tf::ModuleId, effect: f32) -> bool {
        for report in self.active.iter_mut() {
            if report.intervention_id == intervention_id {
                report.observed_effect = Some(effect);
                return true;
            }
        }
        false
    }

    /// Avança um tick: expiradas sem efeito são revertidas (devolvidas);
    /// com efeito observado permanecem e reiniciam o TTL.
    pub fn tick_revert(&mut self) -> Vec<InterventionReport> {
        if self.active.is_empty() {
            return Vec::new();
        }
        self.ttl = self.ttl.saturating_sub(1);
        if self.ttl > 0 {
            return Vec::new();
        }
        let mut revert = Vec::new();
        let mut keep = Vec::new();
        for report in self.active.drain(..) {
            if report.observed_effect.is_some() {
                keep.push(report);
            } else {
                revert.push(report);
            }
        }
        self.active = keep;
        self.ttl = 10;
        revert
    }
}
