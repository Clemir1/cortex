//! Validação de efeitos futuros: só fecha o ciclo com efeito observado no ttl.

// Lei da casa 5: decisão→resultado→aprendizagem fecha SÓ com verified_future_effect.
// Lei da casa 6: sem observed_effect dentro do ttl, reverte.

use std::collections::HashMap;

use triad_contracts as tc;
use triad_foundation as tf;
use triad_foundation::id::{DecisionId, ModuleId, StepId};
use tracing::{debug, trace, warn};

/// Validador de efeitos futuros: fecha o ciclo decisão→resultado só com efeito observado.
pub struct EffectValidator {
    /// ttl em ticks dentro do qual um efeito ainda verifica a decisão.
    ttl: u64,
    /// decisões pendentes: valor previsto e tick de registro.
    pending: HashMap<DecisionId, (f32, u64)>,
    /// ciclos fechados com efeito observado dentro do ttl.
    closed: u64,
    /// decisões expiradas sem efeito observado (revertidas).
    expired: u64,
}

impl EffectValidator {
    /// cria um validador com ttl dado em ticks.
    pub fn new(ttl: u64) -> Self {
        Self { ttl, pending: HashMap::new(), closed: 0, expired: 0 }
    }

    /// registra a decisão com valor previsto e passa a aguardar o efeito.
    pub fn observe_decision(&mut self, decision_id: DecisionId, predicted: f32, at_tick: u64) {
        self.pending.insert(decision_id, (predicted, at_tick));
        trace!("decisão aguardando efeito");
    }

    /// observa um efeito e fecha o ciclo se a decisão está pendente e dentro do ttl.
    pub fn observe_effect(&mut self, decision_id: DecisionId, observed: f32, at_tick: u64) -> bool {
        let entry = self.pending.get(&decision_id).copied();
        match entry {
            Some((predicted, registered_tick)) if at_tick <= registered_tick.saturating_add(self.ttl) => {
                self.closed += 1;
                self.pending.remove(&decision_id);
                let delta = observed - predicted;
                debug!(delta = delta, "efeito verificado: ciclo fechado");
                true
            }
            _ => {
                trace!("efeito sem decisão pendente ou fora do ttl");
                false
            }
        }
    }

    /// expira pendentes cujo ttl venceu antes de now e devolve os ids revertidos.
    pub fn expire(&mut self, now: u64) -> Vec<DecisionId> {
        let expired_ids: Vec<DecisionId> = self
            .pending
            .iter()
            .filter(|(_, v)| v.1.saturating_add(self.ttl) < now)
            .map(|(id, _)| *id)
            .collect();
        let count = expired_ids.len();
        for id in expired_ids.iter() {
            self.pending.remove(id);
        }
        self.expired += count as u64;
        warn!(expiradas = count, "sem efeito observado no ttl: reverte");
        expired_ids
    }

    /// quantidade de decisões ainda aguardando efeito observado.
    pub fn pending(&self) -> usize {
        self.pending.len()
    }

    /// taxa de decisões validadas com denominador explícito (fechadas sobre total).
    pub fn validated_rate(&self, source: ModuleId, step: StepId) -> tc::Qualified<tf::Rate> {
        let total = self.closed + self.expired;
        if total == 0 {
            return tc::Qualified::no_data("sem decisões validadas", source, step);
        }
        match tf::Rate::from_ratio(self.closed, total) {
            Some(rate) => tc::Qualified::value(rate, source, step),
            None => tc::Qualified::invalid("taxa fora do dominio", source, step),
        }
    }
}
