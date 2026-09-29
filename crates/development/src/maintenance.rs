//! Manutenção: verificação O(1) por amostragem no loop quente.

use std::collections::HashSet;
use tracing::{debug, warn};
use triad_foundation as tf;

/// Tamanho padrão da amostra por checagem.
pub const DEFAULT_SAMPLE: usize = 8;

/// Manutenção amostral: olha poucos candidatos, repara o que encontra.
#[derive(Debug)]
pub struct Maintenance {
    /// Checagens realizadas até agora.
    checks: u64,
    /// Total de módulos reparados.
    repaired: u64,
    /// Candidatos examinados por checagem.
    sample_size: usize,
}

impl Maintenance {
    /// Nova manutenção com a amostra padrão.
    pub fn new() -> Self {
        Self {
            checks: 0,
            repaired: 0,
            sample_size: DEFAULT_SAMPLE,
        }
    }

    /// Checagem O(1): examina só a amostra (stride) e repara os danificados encontrados.
    ///
    /// LEI: nunca varre a população inteira no loop quente — o custo por tick é limitado pela amostra.
    pub fn check(&mut self, candidates: &[tf::ModuleId], damaged: &HashSet<tf::ModuleId>) -> usize {
        let len = candidates.len();
        // Amostra zero não faz sentido; mínimo 1 examinado por checagem.
        let sample = self.sample_size.max(1);
        let mut repaired_now = 0usize;
        if len <= sample {
            // População pequena: examina todos (barato por definição).
            for id in candidates {
                if damaged.contains(id) {
                    repaired_now += 1;
                }
            }
        } else {
            // População grande: stride = len/sample, apenas `sample` olhadas.
            let stride = len / sample;
            let mut i = 0usize;
            let mut seen = 0usize;
            while seen < sample && i < len {
                if damaged.contains(&candidates[i]) {
                    repaired_now += 1;
                }
                i += stride;
                seen += 1;
            }
            warn!(seen, total = len, "manutenção adiada: população excede a amostra");
        }
        self.repaired += repaired_now as u64;
        self.checks += 1;
        debug!(repaired_now, checks = self.checks, "manutenção executada na amostra");
        repaired_now
    }

    /// Taxa de reparo com denominador explícito: sem checagens, 0.0 (nunca divide por zero).
    pub fn repair_rate(&self) -> f64 {
        if self.checks == 0 {
            0.0
        } else {
            self.repaired as f64 / self.checks as f64
        }
    }

    /// Ajusta o tamanho da amostra por checagem.
    pub fn set_sample(&mut self, n: usize) {
        self.sample_size = n;
    }

    /// Total de módulos reparados desde o início.
    pub fn repaired(&self) -> u64 {
        self.repaired
    }

    /// Total de checagens realizadas.
    pub fn checks(&self) -> u64 {
        self.checks
    }
}
