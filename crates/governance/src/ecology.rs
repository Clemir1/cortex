//! Ecologia de módulos: saúde por nicho com denominador explícito.

use std::collections::HashMap;

use triad_contracts as tc;
use triad_foundation as tf;
use triad_foundation::id::{ModuleId, StepId};

use tracing::{debug, warn};

/// Contadores de saúde de um nicho.
#[derive(Debug, Clone, Copy)]
pub struct NicheHealth {
    /// Sucessos registrados no nicho.
    pub successes: u64,
    /// Tentativas registradas no nicho (denominador explícito).
    pub attempts: u64,
}

/// Motor de ecologia: registra tentativas e sucessos por nicho.
pub struct EcologyEngine {
    /// Saúde por nome de nicho.
    niches: HashMap<String, NicheHealth>,
}

impl EcologyEngine {
    /// Cria um motor de ecologia sem nichos.
    pub fn new() -> Self {
        Self {
            niches: HashMap::new(),
        }
    }

    /// Garante que o nicho exista com contadores zerados.
    pub fn register(&mut self, niche: &str) {
        if !self.niches.contains_key(niche) {
            self.niches
                .insert(niche.to_string(), NicheHealth { successes: 0, attempts: 0 });
            debug!("nicho registrado");
        }
    }

    /// Reporta uma tentativa no nicho, marcando sucesso quando houver.
    pub fn report(&mut self, niche: &str, success: bool) {
        let health = self
            .niches
            .entry(niche.to_string())
            .or_insert(NicheHealth { successes: 0, attempts: 0 });
        health.attempts += 1;
        if success {
            health.successes += 1;
        }
        let total = health.attempts;
        debug!(
            nicho = %niche,
            sucesso = success,
            tentativas = total,
            "resultado reportado ao nicho"
        );
    }

    /// Retorna a taxa de sucesso do nicho com denominador explícito.
    pub fn health(&self, niche: &str, source: ModuleId, step: StepId) -> tc::Qualified<tf::Rate> {
        match self.niches.get(niche) {
            None => tc::Qualified::no_data("nicho sem tentativas", source, step),
            Some(health) if health.attempts == 0 => {
                tc::Qualified::no_data("nicho sem tentativas", source, step)
            }
            Some(health) => match tf::Rate::from_ratio(health.successes, health.attempts) {
                Some(rate) => tc::Qualified::value(rate, source, step),
                None => tc::Qualified::invalid("taxa fora do dominio", source, step),
            },
        }
    }

    /// Retorna o nome do nicho com a menor taxa de sucesso entre os avaliáveis.
    pub fn weakest(&self, source: ModuleId, step: StepId) -> tc::Qualified<String> {
        let mut chosen: Option<(&String, &NicheHealth)> = None;
        for (name, health) in self.niches.iter() {
            if health.attempts == 0 {
                continue;
            }
            let replace = match chosen {
                None => true,
                Some((chosen_name, chosen_health)) => {
                    // Multiplicação cruzada compara as taxas sem perder precisão.
                    let candidate = (health.successes as u128) * (chosen_health.attempts as u128);
                    let incumbent = (chosen_health.successes as u128) * (health.attempts as u128);
                    candidate < incumbent
                        || (candidate == incumbent && health.attempts < chosen_health.attempts)
                        || (candidate == incumbent
                            && health.attempts == chosen_health.attempts
                            && name < chosen_name)
                }
            };
            if replace {
                chosen = Some((name, health));
            }
        }
        let (name, health) = match chosen {
            Some(weakest) => weakest,
            None => return tc::Qualified::no_data("sem nichos avaliaveis", source, step),
        };
        // Taxa 0.0 com denominador positivo equivale a nenhum sucesso.
        if health.successes == 0 && health.attempts >= 10 {
            warn!(nicho = %name, "nicho em extincao");
        }
        tc::Qualified::value((*name).clone(), source, step)
    }

    /// Retorna a quantidade de nichos conhecidos.
    pub fn len(&self) -> usize {
        self.niches.len()
    }
}

impl Default for EcologyEngine {
    /// Cria um motor de ecologia sem nichos.
    fn default() -> Self {
        Self::new()
    }
}
