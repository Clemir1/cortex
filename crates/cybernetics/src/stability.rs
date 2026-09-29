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
}

impl Default for OscillationMonitor {
    /// Equivalente a [`OscillationMonitor::new`].
    fn default() -> Self {
        Self::new()
    }
}
