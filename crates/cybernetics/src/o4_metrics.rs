//! Métricas de metacognição/auditoria (ordem O4 de Ashby).

use triad_contracts as tc;
use triad_foundation as tf;

use tracing::{debug, trace};

/// Auditoria O4: eventos por tick — taxa SEMPRE com denominador; sem ticks não há dado.
pub fn audit_rate(
    events: u64,
    ticks: u64,
    source: tf::id::ModuleId,
    step: tf::id::StepId,
) -> tc::Qualified<tf::Rate> {
    if ticks == 0 {
        debug!("sem ticks");
        return tc::Qualified::no_data("sem ticks", source, step);
    }
    match tf::Rate::construct(events as f32 / ticks as f32) {
        Some(rate) => {
            trace!(eventos = events, ticks = ticks, "taxa auditada");
            tc::Qualified::value(rate, source, step)
        }
        None => tc::Qualified::invalid(
            &format!("rate fora do domínio: {events}/{ticks}"),
            source,
            step,
        ),
    }
}

/// Auditoria O4: taxa de fechamento por tentativa; sem tentativas não há dado.
pub fn audit_envelope(
    closure_rate: f32,
    attempts: u64,
    source: tf::id::ModuleId,
    step: tf::id::StepId,
) -> tc::Qualified<f32> {
    if attempts == 0 {
        debug!("sem tentativas");
        return tc::Qualified::no_data("sem tentativas", source, step);
    }
    trace!(tentativas = attempts, "taxa de fechamento auditada");
    tc::Qualified::value(closure_rate, source, step)
}
