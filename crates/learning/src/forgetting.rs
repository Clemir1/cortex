//! Esquecimento de traços fracos: decadência com denominador explícito.

use std::collections::HashMap;

use triad_contracts as tc;
use triad_foundation::id::{ModuleId, StepId};
use tracing::{debug, trace};

/// força mínima para um traço sobreviver à poda.
pub const MIN_STRENGTH: f32 = 0.05;

/// motor de esquecimento: reforça, decai e poda traços fracos.
pub struct ForgettingEngine {
    /// limite máximo de traços mantidos após a poda.
    cap: usize,
    /// traços ativos: chave e força atual.
    traces: HashMap<String, f32>,
}

impl ForgettingEngine {
    /// cria o motor com limite de traços (mínimo 1).
    pub fn new(cap: usize) -> Self {
        Self { cap: cap.max(1), traces: HashMap::new() }
    }

    /// reforça um traço somando amount e limitando a força entre 0.0 e 1.0.
    pub fn reinforce(&mut self, key: &str, amount: f32) {
        let current = self.traces.get(key).copied().unwrap_or(0.0);
        let novo_valor = (current + amount).clamp(0.0, 1.0);
        self.traces.insert(key.to_string(), novo_valor);
        debug!(chave = key, forca = novo_valor, "traço reforçado");
    }

    /// aplica fator de decadência a todos os traços (0.0 < fator <= 1.0).
    pub fn decay(&mut self, factor: f32) {
        if !(factor > 0.0 && factor <= 1.0) {
            return;
        }
        for strength in self.traces.values_mut() {
            *strength *= factor;
        }
        trace!(fator = factor, total = self.traces.len(), "decaimento aplicado");
    }

    /// remove traços abaixo de MIN_STRENGTH e os mais fracos acima do cap.
    pub fn prune(&mut self) -> usize {
        let antes = self.traces.len();
        self.traces.retain(|_, v| *v >= MIN_STRENGTH);
        if self.traces.len() > self.cap {
            let mut ordenados: Vec<(String, f32)> =
                std::mem::take(&mut self.traces).into_iter().collect();
            ordenados.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            let excesso = ordenados.len() - self.cap;
            ordenados.drain(..excesso);
            self.traces = ordenados.into_iter().collect();
        }
        let descartados = antes - self.traces.len();
        debug!(descartados = descartados, restantes = self.traces.len(), "poda de traços fracos");
        descartados
    }

    /// força atual do traço como valor qualificado.
    pub fn strength(&self, key: &str, source: ModuleId, step: StepId) -> tc::Qualified<f32> {
        match self.traces.get(key) {
            Some(&v) => tc::Qualified::value(v, source, step),
            None => tc::Qualified::no_data("traço ausente", source, step),
        }
    }

    /// quantidade de traços ativos.
    pub fn len(&self) -> usize {
        self.traces.len()
    }
}
