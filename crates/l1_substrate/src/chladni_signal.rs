//! Sinal Chladni do substrato — a harmonia Cluster/HOTM/Chladni como
//! observação qualificada e ADITIVA.
//!
//! Origem: integração de atenção do legado (attention_system.py,
//! `_compute_chladni_resonance`). Aqui quem observa é o DONO do estado
//! (o L1), não a atenção: o L1 tem o estado 97D e a energia reais; a
//! observação é publicada no TypedContext e qualquer camada (atenção L3,
//! development) consome o sinal qualificado sem tocar o estado canônico.
//!
//! Lei da casa: nada aqui MUDA o comportamento do substrato — o sinal é
//! aditivo (chave nova no contexto), preservando os testes A/A. Ausência
//! ≠ zero: sem amostra, publica NO_DATA com motivo.

use crate::cluster::LifecycleState;
use crate::runner::L1Runner;
use triad_chladni as ch;
use triad_foundation as tf;
use tracing::{debug, trace};

/// Clusters amostrados por tick. Compute limitado e determinístico
/// (primeiros vivos). Nota honesta: frequências vindas de energia são
/// contínuas — o cache do motor quase sempre erra; a taxa de hit é
/// medida COM denominador nas estatísticas do motor.
pub const SAMPLE_PER_TICK: usize = 4;

/// Observação publicada na chave `l1.substrate.chladni` — tipo do crate
/// neutro `triad-chladni` (`ch::Observation`), para consumidores de outras
/// camadas sem depender do L1.

/// Calculador do sinal — dono do motor Chladni (config injetada via
/// construtor, padrão `[chladni]` do default.toml).
pub struct ChladniSignal {
    system: ch::ChladniFrequencySystem,
}

impl ChladniSignal {
    /// Construtor com config injetada (padrão do piloto config central).
    pub fn new(config: ch::ChladniConfig) -> Self {
        Self {
            system: ch::ChladniFrequencySystem::with_config(config),
        }
    }

    /// Observa uma amostra determinística de clusters (os primeiros
    /// vivos) e devolve a ressonância média qualificada.
    pub fn observe(
        &mut self,
        runner: &L1Runner,
        source: tf::ModuleId,
        step: tf::StepId,
    ) -> ch::Observation {
        let mut sum = 0.0f64;
        let mut energy_sum = 0.0f64;
        let mut n = 0u64;

        for c in runner
            .clusters
            .iter()
            .filter(|c| {
                matches!(
                    c.lifecycle.state,
                    LifecycleState::Active | LifecycleState::Dormant | LifecycleState::Repairing
                )
            })
            .take(SAMPLE_PER_TICK)
        {
            // Normalização L2 do estado (legado: /(‖s‖ + 1e-10)).
            let norm: f64 = c.state.iter().map(|v| v * v).sum::<f64>().sqrt();
            let state_norm: Vec<f64> =
                c.state.iter().map(|v| v / (norm + 1e-10)).collect();

            // Frequência da energia → padrão → comparação de features
            // (legado: _compare_patterns — média de (1 − |Δ|) em 4 chaves).
            let Ok(freq) = self.system.energy_to_frequency(c.energy) else {
                continue;
            };
            let Ok(pattern) =
                self.system.get_pattern_for_frequency(freq, "l1.substrate")
            else {
                continue;
            };
            let sf = ch::pattern::state_features(&state_norm);
            let r = ch::pattern::compare_features(&sf, &pattern.features);
            trace!(
                cluster = ?c.id,
                energia = c.energy,
                freq = freq,
                ressonancia = r,
                "amostra chladni"
            );
            sum += r;
            energy_sum += c.energy;
            n += 1;
        }

        if n == 0 {
            debug!(passo = ?step, "sinal chladni sem amostra viva");
            return ch::Observation {
                step,
                resonance: tf::Qualified::no_data(
                    "sem clusters vivos na amostra",
                    source,
                    step,
                ),
                band: None,
                sample_size: 0,
            };
        }

        let mean = (sum / n as f64) as f32;
        let band = self
            .system
            .energy_to_frequency(energy_sum / n as f64)
            .ok()
            .map(|f| self.system.get_cognitive_band(f));
        debug!(
            passo = ?step,
            ressonancia = mean,
            amostra = n,
            banda = ?band,
            "sinal chladni publicado"
        );
        ch::Observation {
            step,
            resonance: tf::Qualified::value(mean, source, step),
            band,
            sample_size: n,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::L1Config;

    fn runner_apos_um_passo(seed: u64, pop: usize) -> L1Runner {
        let mut r = L1Runner::new_with_config(seed, pop, L1Config::default());
        r.step();
        r
    }

    #[test]
    fn sinal_qualificado_com_clusters_vivos() {
        let runner = runner_apos_um_passo(42, 8);
        let mut signal = ChladniSignal::new(ch::ChladniConfig::default());
        let obs = signal.observe(&runner, tf::ModuleId::new(), tf::StepId::new());
        assert!(obs.resonance.is_value(), "com vivos, sinal é VALUE");
        let v = *obs.resonance.as_ref_value().expect("valor presente");
        assert!((0.0..=1.0).contains(&v), "ressonância em [0,1]: {v}");
        assert_eq!(obs.sample_size, SAMPLE_PER_TICK as u64);
        assert!(obs.band.is_some(), "banda da energia média presente");
    }

    #[test]
    fn deterministico_por_seed() {
        let a = runner_apos_um_passo(42, 8);
        let b = runner_apos_um_passo(42, 8);
        let mut s1 = ChladniSignal::new(ch::ChladniConfig::default());
        let mut s2 = ChladniSignal::new(ch::ChladniConfig::default());
        let o1 = s1.observe(&a, tf::ModuleId::new(), tf::StepId::new());
        let o2 = s2.observe(&b, tf::ModuleId::new(), tf::StepId::new());
        let v1 = *o1.resonance.as_ref_value().expect("valor o1");
        let v2 = *o2.resonance.as_ref_value().expect("valor o2");
        assert!((v1 - v2).abs() < 1e-6, "mesma seed ⇒ mesma ressonância");
    }

    #[test]
    fn sem_clusters_publica_no_data() {
        let runner = runner_apos_um_passo(7, 0);
        let mut signal = ChladniSignal::new(ch::ChladniConfig::default());
        let obs = signal.observe(&runner, tf::ModuleId::new(), tf::StepId::new());
        assert!(
            !obs.resonance.is_value(),
            "sem clusters: NO_DATA, nunca zero fantasma"
        );
        assert_eq!(obs.sample_size, 0);
        assert!(obs.band.is_none());
    }
}
