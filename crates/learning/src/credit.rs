//! Atribuição de crédito entre módulos e taxa de sucesso.

use triad_contracts as tc;
use triad_foundation as tf;
use tracing::debug;

/// Contribuição de um módulo para o resultado de uma decisão.
pub struct CreditEntry {
    pub module: String,
    pub contribution: f32,
}

/// Normaliza contribuições pela soma total (E5: taxa sempre com denominador).
pub fn assign_credit(contributions: Vec<CreditEntry>) -> Vec<(String, f32)> {
    let total: f32 = contributions.iter().map(|c| c.contribution).sum();
    if !(total > 0.0) {
        return contributions.into_iter().map(|c| (c.module, 0.0)).collect();
    }
    contributions
        .into_iter()
        .map(|c| (c.module, c.contribution / total))
        .collect()
}

/// Taxa de sucesso sobre as tentativas; `no_data` quando attempts == 0.
pub fn credit_rate(
    successes: u64,
    attempts: u64,
    source: tf::id::ModuleId,
    step: tf::id::StepId,
) -> tc::Qualified<tf::Rate> {
    if attempts == 0 {
        debug!("sem tentativas: taxa ausente, não zero");
        return tc::Qualified::no_data("sem tentativas", source, step);
    }
    debug!(
        taxa = successes as f32 / attempts as f32,
        tentativas = attempts,
        "atribuição de crédito com taxa de sucesso"
    );
    match tf::Rate::construct(successes as f32 / attempts as f32) {
        Some(rate) => tc::Qualified::value(rate, source, step),
        None => tc::Qualified::invalid("taxa fora do intervalo válido", source, step),
    }
}
