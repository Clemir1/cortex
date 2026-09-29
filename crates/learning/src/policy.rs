//! Política de aprendizagem: eta adaptativo; aprendizagem nunca é suspensa.

use tracing::{trace, warn};

/// Política de taxa de aprendizado; `suspended` é sempre false (o runtime apenas lê).
pub struct LearningPolicy {
    pub eta: f32,
    pub min_eta: f32,
    pub max_eta: f32,
    pub suspended: bool,
}

impl Default for LearningPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl LearningPolicy {
    /// Política padrão: eta=0.01, limites [0.005, 0.02], nunca suspensa.
    pub fn new() -> Self {
        Self {
            eta: 0.01,
            min_eta: 0.005,
            max_eta: 0.02,
            suspended: false,
        }
    }

    /// Adapta eta pelo erro médio, dentro dos limites; nunca toca `suspended`.
    pub fn adapt_eta(&mut self, mean_error: f32) -> f32 {
        self.eta = (0.005 + mean_error * 0.1).clamp(self.min_eta, self.max_eta);
        trace!(eta = self.eta, erro_medio = mean_error, "ajuste normal de eta");
        self.eta
    }

    /// Crise reduz eta em vez de suspender; clamp no mínimo.
    pub fn crisis_mode(&mut self, stress: f32) -> f32 {
        warn!(stress = stress, "crise: aprendizagem reduzida, NUNCA suspensa");
        eprintln!("crise ajusta política, não suspende");
        self.eta = (self.eta * (1.0 - stress * 0.5)).max(self.min_eta);
        self.eta
    }
}
