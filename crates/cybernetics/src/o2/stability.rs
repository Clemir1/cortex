//! Monitor de oscilação e estabilidade (ordem O2 de Ashby).

use tracing::{debug, warn};

/// Detecta oscilação contando cruzamentos da média em uma janela deslizante.
#[derive(Debug, Clone)]
pub struct OscillationMonitor {
    history: Vec<f32>,
    window: usize,
}

impl OscillationMonitor {
    /// Cria um monitor com janela de 32 amostras.
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            window: 32,
        }
    }

    /// Cria um monitor com janela explícita (clamp 2..=4096).
    pub fn with_window(window: usize) -> Self {
        Self {
            history: Vec::new(),
            window: window.clamp(2, 4096),
        }
    }

    /// Registra uma amostra, mantendo apenas as últimas `window` leituras.
    pub fn observe(&mut self, value: f32) {
        self.history.push(value);
        while self.history.len() > self.window {
            self.history.remove(0);
        }
    }

    /// Conta quantas vezes o desvio em relação à média muda de sinal.
    pub fn zero_crossings(&self) -> u64 {
        if self.history.len() < 2 {
            return 0;
        }
        let mean = self.history.iter().sum::<f32>() / self.history.len() as f32;
        let mut crossings = 0u64;
        let mut last: Option<bool> = None;
        for &v in &self.history {
            let dev = v - mean;
            if dev == 0.0 {
                continue;
            }
            let above = dev > 0.0;
            if let Some(prev) = last {
                if prev != above {
                    crossings += 1;
                }
            }
            last = Some(above);
        }
        crossings
    }

    /// Retorna true quando os cruzamentos indicam oscilação (O2).
    pub fn is_oscillating(&self) -> bool {
        let crossings = self.zero_crossings();
        let oscillating = crossings > self.window as u64 / 2;
        if oscillating {
            warn!(cruzamentos = crossings, "oscilação detectada");
        } else {
            debug!(cruzamentos = crossings, "sem oscilação");
        }
        oscillating
    }

    /// TAXA DE ESTABILIDADE COM DENOMINADOR (16.8): 1 − cruzamentos/n
    /// onde n = amostras retidas na janela. Menos de 2 amostras ⇒
    /// NO_DATA (ausência ≠ zero); série degenerada ⇒ INVALID.
    pub fn stability_rate(
        &self,
        source: triad_foundation::id::ModuleId,
        step: triad_foundation::id::StepId,
    ) -> triad_contracts::Qualified<triad_foundation::Rate> {
        let n = self.history.len();
        if n < 2 {
            return triad_contracts::Qualified::no_data(
                "sem amostras suficientes",
                source,
                step,
            );
        }
        // n−1 intervalos entre n amostras: denominador natural do
        // cruzamento de zero.
        let intervals = (n - 1) as f32;
        let crossings = self.zero_crossings() as f32;
        let stability = 1.0 - (crossings / intervals).clamp(0.0, 1.0);
        match triad_foundation::Rate::construct(stability) {
            Some(rate) => triad_contracts::Qualified::value(rate, source, step),
            None => triad_contracts::Qualified::invalid(
                "estabilidade fora do dominio",
                source,
                step,
            ),
        }
    }

    /// Amostras retidas na janela (denominador visível).
    pub fn samples(&self) -> usize {
        self.history.len()
    }
}

impl Default for OscillationMonitor {
    /// Equivalente a [`OscillationMonitor::new`].
    fn default() -> Self {
        Self::new()
    }
}
