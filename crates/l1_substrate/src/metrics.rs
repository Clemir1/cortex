//! Métricas do substrato por janelas 1/5/20/50/100 (requisito da
//! auditoria L1 §3: séries com janelas múltiplas para detectar tendência
//! vs ruído).
//!
//! Telemetria PURA: janelas não participam de estado, hash nem decisões
//! (a regra "telemetria não consome RNG" do legado vale — nada aqui
//! chama o rng). Tempo de CPU é observacional, display-only.

use std::collections::VecDeque;
use tracing::trace;

/// Estatísticas de uma janela.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowStats {
    pub mean: f64,
    pub min: f64,
    pub max: f64,
    pub std_dev: f64,
    /// Inclinação simples (último − primeiro) / (n − 1); 0 se n < 2.
    pub trend: f64,
    /// Desvio-padrão das diferenças consecutivas (agitação da série).
    pub volatility: f64,
    pub n: usize,
}

/// Uma janela de tamanho fixo.
#[derive(Debug, Clone)]
pub struct MetricWindow {
    data: VecDeque<f64>,
    cap: usize,
}

impl MetricWindow {
    pub fn new(cap: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(cap.min(128)),
            cap,
        }
    }

    pub fn push(&mut self, v: f64) {
        if self.data.len() == self.cap {
            self.data.pop_front();
        }
        self.data.push_back(v);
    }

    pub fn stats(&self) -> Option<WindowStats> {
        let n = self.data.len();
        if n == 0 {
            return None;
        }
        let mean = self.data.iter().sum::<f64>() / n as f64;
        let min = self.data.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = self.data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let var = self
            .data
            .iter()
            .map(|v| (v - mean) * (v - mean))
            .sum::<f64>()
            / n as f64;
        let trend = if n < 2 {
            0.0
        } else {
            (self.data.back().copied().unwrap() - self.data.front().copied().unwrap())
                / (n - 1) as f64
        };
        let mut diffs = Vec::new();
        let mut prev: Option<f64> = None;
        for &v in &self.data {
            if let Some(p) = prev {
                diffs.push(v - p);
            }
            prev = Some(v);
        }
        let volatility = if diffs.is_empty() {
            0.0
        } else {
            let dm = diffs.iter().sum::<f64>() / diffs.len() as f64;
            (diffs.iter().map(|d| (d - dm) * (d - dm)).sum::<f64>() / diffs.len() as f64).sqrt()
        };
        Some(WindowStats {
            mean,
            min,
            max,
            std_dev: var.sqrt(),
            trend,
            volatility,
            n,
        })
    }

    pub fn last(&self) -> Option<f64> {
        self.data.back().copied()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// Janelas múltiplas (1/5/20/50/100) de UMA métrica.
#[derive(Debug, Clone)]
pub struct Windows {
    pub w1: MetricWindow,
    pub w5: MetricWindow,
    pub w20: MetricWindow,
    pub w50: MetricWindow,
    pub w100: MetricWindow,
}

impl Windows {
    pub fn new() -> Self {
        Self {
            w1: MetricWindow::new(1),
            w5: MetricWindow::new(5),
            w20: MetricWindow::new(20),
            w50: MetricWindow::new(50),
            w100: MetricWindow::new(100),
        }
    }

    pub fn push(&mut self, v: f64) {
        self.w1.push(v);
        self.w5.push(v);
        self.w20.push(v);
        self.w50.push(v);
        self.w100.push(v);
    }

    pub fn stats(&self, window: u32) -> Option<WindowStats> {
        match window {
            1 => self.w1.stats(),
            5 => self.w5.stats(),
            20 => self.w20.stats(),
            50 => self.w50.stats(),
            100 => self.w100.stats(),
            _ => None,
        }
    }
}

impl Default for Windows {
    fn default() -> Self {
        Self::new()
    }
}

/// Cartão de métricas do L1 — cada linha é uma série com janelas.
#[derive(Debug, Clone, Default)]
pub struct L1Metrics {
    /// População viva.
    pub population: Windows,
    /// Média de energia (sensor do O1).
    pub mean_energy: Windows,
    /// Mortes por step (denominador: população do início do step).
    pub deaths: Windows,
    /// Divisões por step.
    pub divisions: Windows,
    /// Fusões por step.
    pub merges: Windows,
    /// Fração de dormentes.
    pub dormancy_fraction: Windows,
    /// Fração em repair.
    pub repair_fraction: Windows,
    /// Entropia média.
    pub entropy: Windows,
    /// Firing médio.
    pub firing: Windows,
    /// Custo O(active): fração da população executada no step.
    pub active_fraction: Windows,
}

impl L1Metrics {
    pub fn push_step(
        &mut self,
        population: usize,
        mean_energy: f64,
        deaths: f64,
        divisions: f64,
        merges: f64,
        dormancy_fraction: f64,
        repair_fraction: f64,
        entropy: f64,
        firing: f64,
        active_fraction: f64,
    ) {
        self.population.push(population as f64);
        self.mean_energy.push(mean_energy);
        self.deaths.push(deaths);
        self.divisions.push(divisions);
        self.merges.push(merges);
        self.dormancy_fraction.push(dormancy_fraction);
        self.repair_fraction.push(repair_fraction);
        self.entropy.push(entropy);
        self.firing.push(firing);
        self.active_fraction.push(active_fraction);
    }

    /// Resumo legível de todas as métricas na janela 100 (verificação
    /// humana rápida — sem taxas sem denominador). Janela vazia é
    /// AUSÊNCIA declarada (NO_DATA), nunca 0.0 fabricado (Lei 2).
    pub fn summary_w100(&self) -> String {
        let pop = self.population.stats(100).map(|s| s.mean);
        let en = self.mean_energy.stats(100).map(|s| s.mean);
        let dth = self.deaths.stats(100).map(|s| s.mean);
        let div = self.divisions.stats(100).map(|s| s.mean);
        let act = self.active_fraction.stats(100).map(|s| s.mean);
        let fmt = |o: Option<f64>| match o {
            Some(v) => format!("{v:.3}"),
            None => "NO_DATA".to_string(),
        };
        if let (Some(p), Some(e), Some(a)) = (pop, en, act) {
            trace!(pop = p, en = e, act = a, "snapshot de métricas w100");
        }
        format!(
            "pop~{} energy~{} deaths/step~{} divisions/step~{} active~{}",
            fmt(pop),
            fmt(en),
            fmt(dth),
            fmt(div),
            fmt(act)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn janelas_respeitam_caps() {
        let mut w = Windows::new();
        for i in 0..150 {
            w.push(i as f64);
        }
        assert_eq!(w.w1.len(), 1);
        assert_eq!(w.w5.len(), 5);
        assert_eq!(w.w20.len(), 20);
        assert_eq!(w.w50.len(), 50);
        assert_eq!(w.w100.len(), 100);
        // Janela 100 cheia: média dos últimos 100 (50..149).
        let s = w.stats(100).unwrap();
        let expected = (50..150).map(|i| i as f64).sum::<f64>() / 100.0;
        assert!((s.mean - expected).abs() < 1e-9);
        assert_eq!(s.min, 50.0);
        assert_eq!(s.max, 149.0);
    }

    #[test]
    fn stats_de_vazia_e_none() {
        let w = MetricWindow::new(10);
        assert!(w.stats().is_none());
        assert!(w.last().is_none());
    }

    #[test]
    fn tendencia_e_volatilidade() {
        let mut w = MetricWindow::new(10);
        for i in 0..10 {
            w.push(i as f64);
        }
        let s = w.stats().unwrap();
        assert!((s.trend - 1.0).abs() < 1e-9, "série linear: trend 1");
        assert!(s.volatility < 1e-9, "passo constante: volatilidade 0");
        // Variância populacional de 0..9 uniforme: (n²−1)/12 = 8.25.
        assert!((s.std_dev - 8.25_f64.sqrt()).abs() < 1e-9);
    }
}
