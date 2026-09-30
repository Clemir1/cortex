//! O5: horizonte de longo prazo — tendência da série de métricas.
use triad_contracts as tc;
use triad_foundation::id::{ModuleId, StepId};
use tracing::{trace, warn};

/// Horizonte O5: mantém janela de amostras e extrai tendência de longo prazo.
#[derive(Debug, Clone)]
pub struct O5Horizon {
    /// Janela deslizante das amostras observadas.
    window: Vec<f32>,
    /// Tamanho máximo da janela, em clamp 2..=1000.
    max_window: usize,
}

impl O5Horizon {
    /// Cria o horizonte com tamanho de janela em clamp 2..=1000.
    pub fn new(max_window: usize) -> Self {
        Self {
            window: Vec::new(),
            max_window: max_window.clamp(2, 1000),
        }
    }

    /// Empilha uma amostra, descartando a mais antiga ao exceder a janela.
    pub fn push(&mut self, value: f32) {
        self.window.push(value);
        if self.window.len() > self.max_window {
            self.window.remove(0);
        }
        trace!(
            valor = value,
            amostras = self.window.len(),
            "amostra no horizonte"
        );
    }

    /// Extrai a inclinação da série por mínimos quadrados simples.
    pub fn trend(&self, source: ModuleId, step: StepId) -> tc::Qualified<f32> {
        let n = self.window.len();
        if n < 2 {
            return tc::Qualified::no_data("sem amostras suficientes", source, step);
        }
        let sum_x: f32 = (0..n).map(|i| i as f32).sum();
        let sum_y: f32 = self.window.iter().copied().sum();
        let sum_xy: f32 = self
            .window
            .iter()
            .enumerate()
            .map(|(i, y)| i as f32 * y)
            .sum();
        let sum_x2: f32 = (0..n).map(|i| i as f32 * i as f32).sum();
        let denominator = n as f32 * sum_x2 - sum_x * sum_x;
        if denominator == 0.0 {
            return tc::Qualified::invalid("series degenerada", source, step);
        }
        let slope = (n as f32 * sum_xy - sum_x * sum_y) / denominator;
        if slope < -0.1 {
            warn!(inclinação = slope, "tendência de queda no horizonte");
        }
        tc::Qualified::value(slope, source, step)
    }

    /// Número de amostras atualmente retidas na janela.
    pub fn samples(&self) -> usize {
        self.window.len()
    }
}
