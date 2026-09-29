//! Escalonador de módulos cognitivos por passo lógico.

use std::collections::HashSet;
use std::sync::Arc;
use triad_contracts as tc;
use triad_foundation as tf;

/// Orçamento máximo de eventos publicados por passo.
pub struct StepBudget {
    pub max_events: usize,
}

impl StepBudget {
    /// Cria um orçamento com limite de eventos por passo.
    pub fn new(max_events: usize) -> Self {
        Self { max_events }
    }
}

/// Relatório consolidado da execução de um passo.
pub struct StepReport {
    pub step: tf::id::StepId,
    pub tick: u64,
    pub executed: usize,
    pub skipped: usize,
    pub events_published: usize,
    pub budget_exceeded: bool,
    pub degraded: bool,
}

/// Escalonador que executa módulos cognitivos e publica eventos no barramento.
pub struct Scheduler {
    pub modules: Vec<Arc<dyn crate::module::CognitiveModule>>,
    pub budget: StepBudget,
    pub bus: crate::event_bus::EventBus,
    clock: tf::LogicalClock,
    degraded_overrides: HashSet<tf::id::ModuleId>,
}

impl Scheduler {
    /// Cria escalonador com módulos, orçamento, barramento e relógio novo.
    pub fn new(
        modules: Vec<Arc<dyn crate::module::CognitiveModule>>,
        budget: StepBudget,
        bus: crate::event_bus::EventBus,
    ) -> Self {
        Self {
            modules,
            budget,
            bus,
            clock: tf::LogicalClock::new(),
            degraded_overrides: HashSet::new(),
        }
    }

    /// Acesso imutável ao relógio lógico corrente.
    pub fn clock(&self) -> &tf::LogicalClock {
        &self.clock
    }

    /// Executa um passo: tica módulos ativos, publica eventos e avança o relógio.
    pub fn step(&mut self) -> StepReport {
        let (mut executed, mut skipped, mut published) = (0usize, 0usize, 0usize);
        let (mut budget_exceeded, mut degraded) = (false, false);
        for m in self.modules.iter() {
            let id = m.descriptor().module_id;
            let ativo = !self.degraded_overrides.contains(&id) && m.state().can_tick();
            if !ativo {
                skipped += 1;
                continue;
            }
            let ctx = crate::context::TypedContext::new(self.clock);
            let mut out: Vec<tc::EventEnvelope> = Vec::new();
            match m.tick(&ctx, &mut out) {
                Ok(()) => {
                    self.degraded_overrides.remove(&id);
                    executed += 1;
                    for ev in out {
                        if published >= self.budget.max_events {
                            budget_exceeded = true;
                            break;
                        }
                        if self.bus.publish(ev).is_ok() {
                            published += 1;
                        } else {
                            budget_exceeded = true;
                        }
                    }
                }
                Err(_) => {
                    self.degraded_overrides.insert(id);
                    degraded = true;
                    skipped += 1;
                    eprintln!("módulo {:?} degradado por erro no tick", id);
                }
            }
        }
        self.clock.advance();
        StepReport {
            step: self.clock.step,
            tick: self.clock.tick,
            executed,
            skipped,
            events_published: published,
            budget_exceeded,
            degraded,
        }
    }
}
