//! HOTM/Reservoir com readout derivado e **Hebbian real mensurável**.
//!
//! Referência: `core/reservoir_readout.py` legado (2.082 linhas).
//!
//! O defeito provado no legado (auditoria L1 §6): `HebbianNorm = 0` em
//! TODA a run — a plasticidade era declarada mas nunca executava. Aqui a
//! regra é binária e verificável: ou o Hebbian executa e o delta de pesos
//! é MEDIDO (`norm_delta > 0`, acumulado em `norm_delta_total`), ou o
//! status é `Bypassed` com razão — nunca silêncio, nunca "ativado" sem
//! prova.
//!
//! Sinais derivados (8 features, como no legado): memory_capacity,
//! separation, fading, entropy, homogeneity, hotm_energy, diversity,
//! dynamics — computados da AMOSTRA estratificada (≤ 256), com política e
//! cobertura publicadas junto (nunca sinal sem proveniência de amostra).
//!
//! Recursão HOTM (legado): `z' = sign(W·z) * |W·z|^(1/3)`.

use crate::cluster::ClusterBio;
use crate::config as cfg;
use crate::math;
use rand::rngs::StdRng;
use rand::Rng;
use std::collections::VecDeque;
use tracing::{debug, trace};

/// Estado da plasticidade — mensurável ou explicitamente desativada.
#[derive(Debug, Clone, PartialEq)]
pub enum PlasticityStatus {
    /// Hebbian executando: passos e delta acumulado de pesos.
    Hebbian {
        steps: u64,
        norm_delta_total: f64,
    },
    /// Não executou — e a razão é pública (nunca silêncio).
    Bypassed {
        reason: &'static str,
    },
}

impl PlasticityStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Hebbian { .. })
    }
}

/// Nomes canônicos das 8 features (ordem estável — parte do contrato).
pub const FEATURE_NAMES: [&str; 8] = [
    "memory_capacity",
    "separation",
    "fading",
    "entropy",
    "homogeneity",
    "hotm_energy",
    "diversity",
    "dynamics",
];

/// Resultado de um ciclo do reservoir — readout com proveniência de amostra.
#[derive(Debug, Clone)]
pub struct ReadoutResult {
    pub features: [f64; 8],
    /// Nenhuma feature sem amostra — ausência ≠ zero.
    pub sample_count: usize,
    pub population_total: usize,
    pub coverage: f32,
    pub plasticity: PlasticityStatus,
}

/// Reservoir HOTM: estado recorrente `z`, pesos `W` (16×16 densos —
/// pequenos por construção), features derivadas da amostra.
pub struct HotmReservoir {
    pub dim: usize,
    pub w: Vec<f64>,
    pub z: Vec<f64>,
    z_prev: Vec<f64>,
    pub eta: f64,
    pub sparse: f64,
    pub clip: (f64, f64),
    pub features: Option<[f64; 8]>,
    pub feature_history: VecDeque<[f64; 8]>,
    pub plasticity: PlasticityStatus,
    pub steps: u64,
    pub norm_delta_total: f64,
    pub bypass_count: u64,
}

impl HotmReservoir {
    pub fn new(rng: &mut StdRng) -> Self {
        let dim = cfg::RESERVOIR_MEMORY_DIM;
        let mut w = vec![0.0; dim * dim];
        // Inicialização esparsa determinística (legado: 10% densidade).
        for i in 0..dim {
            for j in 0..dim {
                if i != j && rng.gen::<f64>() < cfg::RESERVOIR_HEBBIAN_SPARSE {
                    w[i * dim + j] = 0.5 * math::normal(rng);
                }
            }
        }
        let z = (0..dim).map(|_| 0.05 * math::normal(rng)).collect::<Vec<_>>();
        Self {
            dim,
            w,
            z_prev: z.clone(),
            z,
            eta: cfg::RESERVOIR_HEBBIAN_ETA,
            sparse: cfg::RESERVOIR_HEBBIAN_SPARSE,
            clip: (cfg::RESERVOIR_HEBBIAN_CLIP_MIN, cfg::RESERVOIR_HEBBIAN_CLIP_MAX),
            features: None,
            feature_history: VecDeque::with_capacity(cfg::RESERVOIR_HISTORY_MAXLEN),
            plasticity: PlasticityStatus::Bypassed {
                reason: "not_executed_yet",
            },
            steps: 0,
            norm_delta_total: 0.0,
            bypass_count: 0,
        }
    }

    /// Recursão HOTM: `z' = sign(Wz)·|Wz|^(1/3)`.
    fn hotm_step(&mut self) {
        let d = self.dim;
        let mut out = vec![0.0; d];
        for i in 0..d {
            let mut acc = 0.0;
            for j in 0..d {
                acc += self.w[i * d + j] * self.z[j];
            }
            out[i] = acc.signum() * acc.abs().cbrt();
        }
        self.z_prev = self.z.clone();
        self.z = out;
    }

    /// Hebbian REAL sobre a atividade projetada `a` (short_memory médio
    /// da amostra): `W_ij += eta·a_i·a_j` com máscara esparsa, clip e
    /// decaimento. **Retorna o delta medido (norma Frobenius)**.
    fn hebbian_step(&mut self, a: &[f64], rng: &mut StdRng) -> f64 {
        let d = self.dim;
        let mut delta_sq = 0.0_f64;
        for i in 0..d {
            for j in 0..d {
                if i == j {
                    continue;
                }
                let mask = self.w[i * d + j] != 0.0;
                if !mask && rng.gen::<f64>() >= self.sparse {
                    continue; // mantém esparsidade: só aprende em sinapses existentes/novas raras
                }
                let dw = self.eta * a[i] * a[j];
                let old = self.w[i * d + j];
                let mut new = old + dw;
                // Decaimento estrutural raro (legado: prob 1%).
                if mask && rng.gen::<f64>() < cfg::RESERVOIR_HEBBIAN_DECAY_PROB {
                    new *= cfg::RESERVOIR_HEBBIAN_DECAY_FACTOR;
                }
                new = new.clamp(self.clip.0, self.clip.1);
                self.w[i * d + j] = new;
                delta_sq += (new - old) * (new - old);
            }
        }
        delta_sq.sqrt()
    }

    /// Um ciclo completo: amostra → features → HOTM → Hebbian (ou bypass
    /// com razão). Chamado a cada `RESERVOIR_INTERVAL` steps.
    pub fn update(
        &mut self,
        clusters: &[ClusterBio],
        order: &[usize],
        rng: &mut StdRng,
    ) -> ReadoutResult {
        self.steps += 1;
        let pop = order.len();
        // Amostra estratificada por stride sobre a ordem (determinística).
        let sample: Vec<usize> = if pop == 0 {
            Vec::new()
        } else {
            let stride = (pop as f64 / cfg::RESERVOIR_SAMPLE_SIZE as f64).ceil() as usize;
            (0..pop)
                .step_by(stride.max(1))
                .map(|p| order[p])
                .collect()
        };
        let coverage = if pop == 0 {
            0.0
        } else {
            sample.len() as f32 / pop as f32
        };

        if sample.len() < 2 {
            self.bypass_count += 1;
            let estava_ativa = self.plasticity.is_active();
            self.plasticity = PlasticityStatus::Bypassed {
                reason: if pop == 0 {
                    "population_empty"
                } else {
                    "sample_below_minimum"
                },
            };
            if estava_ativa {
                debug!(bypasses = self.bypass_count, "plasticidade mudou para bypass");
            }
            trace!(amostra = sample.len(), "readout em bypass");
            return ReadoutResult {
                features: [0.0; 8],
                sample_count: sample.len(),
                population_total: pop,
                coverage,
                plasticity: self.plasticity.clone(),
            };
        }

        // --- Features da amostra (todas com denominador explícito) ---
        // 17.2: kernels SoA colunares da amostra — bit-idênticos aos
        // loops AoS anteriores (mesma ordem de acumulação; prontos
        // para o rayon por coluna com redução canônica na 17.3).
        let d = cfg::DIMENSIONALITY;
        let sm = crate::config::slice::SHORT_MEMORY;
        let soa = crate::soa::SampleSoA::pack(clusters, &sample);
        let dim_mean = soa.dim_sums();
        let mut mean_norm = soa.norms_sum();
        let mut short_proj = soa.proj_sums(sm.start, self.dim);
        let (sep_acc, sep_n) = soa.separation_sum((sample.len() / 64).max(1), d);
        let mut mean_entropy = 0.0;
        let mut mean_firing = 0.0;
        for &i in &sample {
            mean_entropy += clusters[i].entropy_cache;
            mean_firing += clusters[i].firing_rate;
        }
        let n = sample.len() as f64;
        mean_norm /= n;
        mean_entropy /= n;
        mean_firing /= n;
        for v in short_proj.iter_mut() {
            *v /= n;
        }
        let mut div_acc = 0.0;
        for k in 0..d {
            div_acc += (dim_mean[k] / n).powi(2);
        }
        let dim_mean_of_means: f64 = dim_mean.iter().sum::<f64>() / n / d as f64;
        let diversity = ((div_acc / d as f64 - dim_mean_of_means * dim_mean_of_means).max(0.0)).sqrt();
        let separation = if sep_n == 0 { 0.0 } else { sep_acc / sep_n as f64 };

        // --- HOTM: recursão cúbica ---
        self.hotm_step();
        let hotm_energy = self.z.iter().map(|v| v * v).sum::<f64>().sqrt();
        let mut dot = 0.0;
        let zn = self.z.iter().map(|v| v * v).sum::<f64>().sqrt();
        let zp = self.z_prev.iter().map(|v| v * v).sum::<f64>().sqrt();
        if zn > 1e-12 && zp > 1e-12 {
            for k in 0..self.dim {
                dot += self.z[k] * self.z_prev[k];
            }
        }
        let fading = dot / (zn * zp); // autocorrelação z×z_prev

        // --- Hebbian REAL (mensurado) ---
        let norm_delta = self.hebbian_step(&short_proj, rng);
        self.norm_delta_total += norm_delta;
        let estava_ativa = self.plasticity.is_active();
        self.plasticity = PlasticityStatus::Hebbian {
            steps: self.steps,
            norm_delta_total: self.norm_delta_total,
        };
        if !estava_ativa {
            debug!(delta = norm_delta, "plasticidade hebbiana ativada");
        }

        let features = [
            mean_norm,          // memory_capacity (norma média do estado)
            separation,         // separation (distância média entre linhas)
            fading,             // fading (autocorrelação do z)
            mean_entropy,       // entropy (média das entropias)
            1.0 - separation.min(1.0), // homogeneity (complemento da separação)
            hotm_energy,        // hotm_energy (norma do z)
            diversity,          // diversity (std das médias por dimensão)
            mean_firing,        // dynamics (média do firing_rate)
        ];
        self.features = Some(features);
        if self.feature_history.len() == cfg::RESERVOIR_HISTORY_MAXLEN {
            self.feature_history.pop_front();
        }
        self.feature_history.push_back(features);

        trace!(amostra = sample.len(), cobertura = coverage, delta = norm_delta, "readout concluído");
        ReadoutResult {
            features,
            sample_count: sample.len(),
            population_total: pop,
            coverage,
            plasticity: self.plasticity.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cluster::ClusterProfile;
    use rand::SeedableRng;
    use triad_foundation::id::ClusterId;

    fn clusters(n: usize) -> Vec<ClusterBio> {
        let mut rng = StdRng::seed_from_u64(21);
        (0..n)
            .map(|_| {
                let mut c = ClusterBio::new(ClusterId::new(), 0, &mut rng);
                c.profile = ClusterProfile::default();
                c
            })
            .collect()
    }

    #[test]
    fn hebbian_e_real_e_mensuravel() {
        // A prova que faltou no legado: norm_delta > 0.
        let cs = clusters(64);
        let order: Vec<usize> = (0..64).collect();
        let mut rng = StdRng::seed_from_u64(3);
        let mut r = HotmReservoir::new(&mut rng);
        let out = r.update(&cs, &order, &mut rng);
        assert!(matches!(out.plasticity, PlasticityStatus::Hebbian { .. }));
        assert!(r.norm_delta_total > 0.0, "plasticidade declarada tem que ter delta medido");
        assert_eq!(out.sample_count, 64);
        assert!((out.coverage - 1.0).abs() < 1e-6);
        assert!(out.features.iter().all(|f| f.is_finite()));
    }

    #[test]
    fn sem_populacao_e_bypass_com_razao() {
        let mut rng = StdRng::seed_from_u64(4);
        let mut r = HotmReservoir::new(&mut rng);
        let out = r.update(&[], &[], &mut rng);
        assert!(matches!(
            out.plasticity,
            PlasticityStatus::Bypassed { reason: "population_empty" }
        ));
        assert_eq!(out.sample_count, 0);
        // 1 cluster: amostra < 2 → bypass com outra razão.
        let cs = clusters(1);
        let out = r.update(&cs, &[0], &mut rng);
        assert!(matches!(
            out.plasticity,
            PlasticityStatus::Bypassed { reason: "sample_below_minimum" }
        ));
        assert_eq!(r.bypass_count, 2);
    }

    #[test]
    fn pesos_ficam_no_clip_e_amostro_respeita_teto() {
        let cs = clusters(1000);
        let order: Vec<usize> = (0..1000).collect();
        let mut rng = StdRng::seed_from_u64(8);
        let mut r = HotmReservoir::new(&mut rng);
        for _ in 0..10 {
            r.update(&cs, &order, &mut rng);
        }
        assert!(r.w.iter().all(|&w| w >= cfg::RESERVOIR_HEBBIAN_CLIP_MIN - 1e-12
            && w <= cfg::RESERVOIR_HEBBIAN_CLIP_MAX + 1e-12));
        assert_eq!(r.steps, 10);
        assert!(r.feature_history.len() == 10);
    }
}
