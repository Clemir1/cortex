//! 17.3 — L1 DINÂMICA CANÔNICA, conforme auditoria 17-1:
//! RecurrentDynamics multi-motor (HOTM vira UMA estratégia, não o
//! cortex — audit §2: `z' = sign(Wz)·|Wz|^(1/3)`, normalizado),
//! HarmonicDynamics (φ += 2π·f·dt/1000; banda de frequência por
//! energia, hotm_optimizer.py:324-333/453-471), LocalInhibition
//! (top 30% por score acima de 0.8 recebe força 1.0, decai ×0.95,
//! libera <0.1 — sparse_cognition.py:119-150) e
//! LocalPredictionError (err vetorial observado−predito por
//! cluster, surpresa = entropia do erro normalizado,
//! semantic_competition.py:425-437; gate cosseno
//! ALIGNED/OPPOSING, system.py:5595-5651). O elo
//! prediction_error→plasticidade LOCAL é EXTENSÃO declarada da
//! audit (§nota do auditor): o gate é MECÂNICA; o acoplamento
//! η(err) fica como POLÍTICA (audit §5). ClusterStore SoA
//! paralelizável (Rayon) JÁ é realidade do runner
//! (`par_iter_mut` bit-idêntico, caixas 17.2/17.5 da sessão 6) —
//! aqui não se duplica, usa-se. Habituation do L4 intacta.

use std::collections::HashMap;

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use triad_foundation::id::ClusterId;

use crate::cluster::ClusterBio;

/// Escala do passo dos motores sobre o estado (suave por
/// construção — não desloca a física: transforma ao redor).
pub const MOTOR_SCALE: f64 = 0.01;
/// dt do harmônico (legado: dt/1000 no incremento de fase).
pub const HARMONIC_DT_MS: f64 = 10.0;
/// Bandas de frequência por energia (hotm_optimizer.py:315-333).
pub const HARMONIC_BANDS: [f64; 4] = [165.0, 475.0, 1100.0, 2750.0];
/// Inibição: fração do topo que compete (config.py:1331).
pub const INHIBITION_TOP_RATIO: f64 = 0.3;
/// Inibição: score mínimo para competir (config.py:1332).
pub const INHIBITION_THRESHOLD: f64 = 0.8;
/// Inibição: decaimento da força por passo (config.py:1333).
pub const INHIBITION_DECAY: f64 = 0.95;
/// Inibição: força abaixo disso libera (sparse_cognition.py:150).
pub const INHIBITION_RELEASE: f64 = 0.1;
/// Rebaixamento aplicado ao estado por unidade de força.
pub const INHIBITION_DAMP: f64 = 0.05;

/// Um motor de dinâmica recorrente — HOTM vira UMA estratégia
/// (não o cortex): motores são plugáveis e determinísticos.
pub trait RecurrentDynamics: Send + Sync {
    /// Nome canônico (contrato de telemetria).
    fn name(&self) -> &'static str;
    /// Um passo do motor sobre o estado (retorna o estado
    /// transformado; NÃO muta entrada).
    fn step(&self, state: &[f64], rng: &mut SmallRng) -> Vec<f64>;
}

/// Motor HOTM (audit §2): `wz = W·z; z' = sign(wz)·|wz|^(1/3)`;
/// normalizado por ‖z'‖+1e-10. W pequena e determinística por
/// cluster (esparsa 10%).
pub struct HotmDynamics {
    dim: usize,
    w: Vec<f64>,
}

impl HotmDynamics {
    /// Motor com W determinística derivada da seed do cluster.
    pub fn for_cluster(osc_seed: u64, dim: usize) -> Self {
        let mut rng = SmallRng::seed_from_u64(osc_seed ^ 0x1107_5EED);
        let mut w = vec![0.0; dim * dim];
        for i in 0..dim {
            for j in 0..dim {
                if i != j && rng.gen::<f64>() < 0.1 {
                    w[i * dim + j] = 0.5 * crate::math::normal(&mut rng);
                }
            }
        }
        Self { dim, w }
    }
}

impl RecurrentDynamics for HotmDynamics {
    fn name(&self) -> &'static str {
        "hotm"
    }

    fn step(&self, state: &[f64], _rng: &mut SmallRng) -> Vec<f64> {
        let d = self.dim.min(state.len());
        // wz = W·z (bloco d×d).
        let mut wz = vec![0.0; d];
        for i in 0..d {
            let mut acc = 0.0;
            for j in 0..d {
                acc += self.w[i * self.dim + j] * state[j];
            }
            wz[i] = acc;
        }
        // z' = sign(wz)·|wz|^(1/3), normalizado.
        let mut norm = 0.0f64;
        let mut out = vec![0.0; state.len()];
        for i in 0..d {
            let v = wz[i].signum() * wz[i].abs().cbrt();
            out[i] = v;
            norm += v * v;
        }
        let inv = 1.0 / (norm.sqrt() + 1e-10);
        for v in out.iter_mut().take(d) {
            *v *= inv;
        }
        out
    }
}

/// Motor harmônico (audit §2): banda por energia do cluster,
/// fase φ += 2π·f·dt/1000, ressonância 1−Δf/2000+0.3·cos(Δfase).
pub struct HarmonicDynamics {
    phase: f64,
    freq: f64,
}

impl HarmonicDynamics {
    /// Banda pela energia (0..1 → 4 bandas do legado).
    pub fn for_energy(energy: f64) -> Self {
        let idx = ((energy.clamp(0.0, 0.9999) * HARMONIC_BANDS.len() as f64) as usize)
            .min(HARMONIC_BANDS.len() - 1);
        Self {
            phase: 0.0,
            freq: HARMONIC_BANDS[idx],
        }
    }

    /// Avança a fase (dt/1000 do legado).
    pub fn advance(&mut self) {
        self.phase += std::f64::consts::TAU * self.freq * HARMONIC_DT_MS / 1000.0;
        if self.phase > std::f64::consts::TAU {
            self.phase -= std::f64::consts::TAU;
        }
    }

    /// Ressonância com a média dos vizinhos (hotm_optimizer.py:453-467).
    pub fn resonance(&self, other_freq: f64, other_phase: f64) -> f64 {
        let df = (self.freq - other_freq).abs();
        let dphase = self.phase - other_phase;
        (1.0 - df / 2000.0 + 0.3 * dphase.cos()).clamp(0.0, 1.5)
    }
}

impl RecurrentDynamics for HarmonicDynamics {
    fn name(&self) -> &'static str {
        "harmonic"
    }

    fn step(&self, state: &[f64], _rng: &mut SmallRng) -> Vec<f64> {
        // Onda sobre o estado: sin(k·i + φ) modulada — padrão
        // Chladni 1D (chladni_frequencies.py:449-479).
        let mut out = state.to_vec();
        for (i, v) in out.iter_mut().enumerate() {
            let k = 1.0 + (i % 4) as f64;
            *v += MOTOR_SCALE * (k * i as f64 + self.phase).sin();
        }
        out
    }
}

/// Relatório da inibição lateral — COM DENOMINADOR.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct InhibitionReport {
    /// Clusters inibidos neste passo (força aplicada agora).
    pub inibidos: u64,
    /// Clusters liberados neste passo (força caiu <0.1).
    pub liberados: u64,
    /// Competidores elegíveis (score>threshold, top 30%).
    pub competidores: u64,
    /// População viva considerada (denominador).
    pub vivos: u64,
}

/// LocalInhibition (sparse_cognition.py:129-150): competição
/// lateral — top 30% por score acima de 0.8 recebe força 1.0,
/// decai ×0.95 por passo, libera <0.1.
#[derive(Debug, Default)]
pub struct LocalInhibition {
    forces: HashMap<ClusterId, f64>,
    pub total_inibicoes: u64,
    pub total_liberacoes: u64,
    pub steps: u64,
}

impl LocalInhibition {
    /// Aplica a inibição: rebaixa o estado dos inibidos
    /// (damp por força) e atualiza os contadores tipados.
    pub fn apply(&mut self, clusters: &mut [ClusterBio]) -> InhibitionReport {
        self.steps += 1;
        let vivos: Vec<usize> = (0..clusters.len())
            .filter(|&i| clusters[i].is_alive())
            .collect();
        // Score = norma do estado (ativação).
        let mut scored: Vec<(usize, f64)> = vivos
            .iter()
            .map(|&i| (i, clusters[i].state.iter().map(|v| v * v).sum::<f64>().sqrt()))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let elegiveis = scored.iter().filter(|(_, s)| *s > INHIBITION_THRESHOLD).count() as u64;
        let top_n = ((vivos.len() as f64 * INHIBITION_TOP_RATIO).ceil() as usize)
            .min(scored.len());
        let mut inibidos = 0u64;
        for &(i, score) in scored.iter().take(top_n) {
            if score <= INHIBITION_THRESHOLD {
                break;
            }
            *self.forces.entry(clusters[i].id).or_insert(0.0) = 1.0;
            inibidos += 1;
        }
        // Damping + decaimento + liberação.
        let mut liberados = 0u64;
        for &i in &vivos {
            let id = clusters[i].id;
            if let Some(f) = self.forces.get_mut(&id) {
                if *f > 0.0 {
                    let damp = INHIBITION_DAMP * *f;
                    for v in clusters[i].state.iter_mut() {
                        *v = (*v * (1.0 - damp)).clamp(-1.0, 1.0);
                    }
                }
                *f *= INHIBITION_DECAY;
                if *f < INHIBITION_RELEASE {
                    self.forces.remove(&id);
                    liberados += 1;
                }
            }
        }
        self.total_inibicoes += inibidos;
        self.total_liberacoes += liberados;
        InhibitionReport {
            inibidos,
            liberados,
            competidores: elegiveis,
            vivos: vivos.len() as u64,
        }
    }
}

/// Gate do prediction error (system.py:5595-5651).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrGate {
    /// Predição e observação alinhadas: plasticidade liberada.
    Aligned,
    /// Opostas: plasticidade bloqueada neste passo (mecânica).
    Opposing,
}

/// Relatório do LPE de um passo — COM DENOMINADOR.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LpeReport {
    /// Passos com amostra (prev+obs presentes).
    pub amostrados: u64,
    /// Passos sem predição prévia (ausência ≠ zero — contada).
    pub sem_predicao: u64,
    /// Erro L2 médio (denominador amostrados).
    pub err_medio: f64,
    /// Surpresa média (entropia do erro normalizado).
    pub surpresa_media: f64,
    /// Gates ALIGNED (denominador amostrados).
    pub gates_aligned: u64,
    /// Gates OPPOSING.
    pub gates_opposing: u64,
}

/// LocalPredictionError: prediction→observation→local error.
/// Predição local = persistência (estado do passo anterior);
/// surpresa = entropia do erro normalizado; gate cosseno
/// ALIGNED/OPPOSING controla a plasticidade (η(err) é POLÍTICA).
#[derive(Debug, Default)]
pub struct LocalPredictionError {
    prev: HashMap<ClusterId, Vec<f64>>,
    pub total_gates_aligned: u64,
    pub total_gates_opposing: u64,
}

impl LocalPredictionError {
    fn entropy_of_norm(err: &[f64]) -> f64 {
        // Distribuição |err| normalizada → entropia natural
        // (semantic_competition.py:428-437).
        let sum: f64 = err.iter().map(|e| e.abs()).sum();
        if sum <= 1e-12 {
            return 0.0;
        }
        -err
            .iter()
            .map(|e| {
                let p = e.abs() / sum;
                if p > 1e-12 {
                    p * p.ln()
                } else {
                    0.0
                }
            })
            .sum::<f64>()
    }

    /// Observa os estados correntes contra a predição (prev).
    pub fn observe(&mut self, clusters: &[ClusterBio]) -> LpeReport {
        let mut r = LpeReport::default();
        let mut err_acc = 0.0f64;
        let mut surp_acc = 0.0f64;
        for c in clusters.iter().filter(|c| c.is_alive()) {
            match self.prev.get(&c.id) {
                Some(pred) if pred.len() == c.state.len() => {
                    let err: Vec<f64> =
                        c.state.iter().zip(pred.iter()).map(|(o, p)| o - p).collect();
                    let l2 = err.iter().map(|e| e * e).sum::<f64>().sqrt();
                    err_acc += l2;
                    surp_acc += Self::entropy_of_norm(&err);
                    // Gate cosseno pred×obs.
                    let dot: f64 =
                        pred.iter().zip(c.state.iter()).map(|(p, o)| p * o).sum();
                    let np: f64 = pred.iter().map(|p| p * p).sum::<f64>().sqrt();
                    let no: f64 = c.state.iter().map(|o| o * o).sum::<f64>().sqrt();
                    let cos = dot / (np * no + 1e-12);
                    if cos > 0.0 {
                        r.gates_aligned += 1;
                        self.total_gates_aligned += 1;
                    } else {
                        r.gates_opposing += 1;
                        self.total_gates_opposing += 1;
                    }
                    r.amostrados += 1;
                }
                _ => {
                    r.sem_predicao += 1;
                }
            }
        }
        // Guarda o CORRENTE como predição do próximo passo.
        self.prev.clear();
        for c in clusters.iter().filter(|c| c.is_alive()) {
            self.prev.insert(c.id, c.state.clone());
        }
        if r.amostrados > 0 {
            r.err_medio = err_acc / r.amostrados as f64;
            r.surpresa_media = surp_acc / r.amostrados as f64;
        }
        r
    }
}

/// Motor multi-motor por cluster: HOTM ou HARMONIC pelo
/// osc_seed (determinístico por construção — A/A bit-identico).
/// O passo do motor é SUAVE (escala 0.01) e preserva o clip.
/// HOTM é construído por cluster (W por osc_seed) — nada de
/// estado morto compartilhado; o harmônico mantém fase viva.
pub struct MotorSelector {
    harmonic: HarmonicDynamics,
    pub passos_hotm: u64,
    pub passos_harmonic: u64,
}

impl MotorSelector {
    /// Motor por dimensionalidade + energia corrente.
    pub fn new(dim: usize) -> Self {
        debug_assert!(dim > 0, "dimensionalidade positiva");
        Self {
            harmonic: HarmonicDynamics::for_energy(0.5),
            passos_hotm: 0,
            passos_harmonic: 0,
        }
    }

    /// Escolhe e aplica o motor do cluster (osc_seed % 2).
    pub fn step_cluster(&mut self, c: &mut ClusterBio, rng: &mut SmallRng) -> &'static str {
        let motor = if c.osc_seed % 2 == 0 { "hotm" } else { "harmonic" };
        let mut next = match motor {
            "hotm" => {
                let m = HotmDynamics::for_cluster(c.osc_seed, c.state.len());
                m.step(&c.state, rng)
            }
            _ => {
                self.harmonic.advance();
                self.harmonic.step(&c.state, rng)
            }
        };
        // Blend suave: estado + escala·(motor − estado).
        for (v, m) in c.state.iter_mut().zip(next.iter_mut()) {
            *v = (*v + MOTOR_SCALE * (*m - *v)).clamp(-1.0, 1.0);
        }
        next.clear();
        if motor == "hotm" {
            self.passos_hotm += 1;
            "hotm"
        } else {
            self.passos_harmonic += 1;
            "harmonic"
        }
    }
}

/// Engine local completa — observável COM DENOMINADORES.
pub struct LocalDynamicsEngine {
    pub inhibition: LocalInhibition,
    pub lpe: LocalPredictionError,
    pub motors: MotorSelector,
    pub ultimo_inh: InhibitionReport,
    pub ultimo_lpe: LpeReport,
}

impl LocalDynamicsEngine {
    pub fn new(dim: usize) -> Self {
        Self {
            inhibition: LocalInhibition::default(),
            lpe: LocalPredictionError::default(),
            motors: MotorSelector::new(dim),
            ultimo_inh: InhibitionReport::default(),
            ultimo_lpe: LpeReport::default(),
        }
    }

    /// Um passo do estágio local: (1) LPE observa prev→obs com
    /// gates; (2) motores por cluster (não-dormant — dormant é
    /// metabolismo mínimo); (3) inibição lateral. Determinístico:
    /// mesmas entradas → mesmos estados.
    pub fn apply(&mut self, clusters: &mut [ClusterBio], step: u64) {
        self.ultimo_lpe = self.lpe.observe(clusters);
        // Motores: rng determinístico por (osc_seed, step).
        for c in clusters.iter_mut() {
            if !c.is_alive()
                || c.lifecycle.state == crate::cluster::LifecycleState::Dormant
            {
                continue;
            }
            let mut rng = SmallRng::seed_from_u64(c.osc_seed ^ step ^ 0x4D07_0A50);
            self.motors.step_cluster(c, &mut rng);
        }
        self.ultimo_inh = self.inhibition.apply(clusters);
    }

    /// Fotografia para o app (tudo com denominador).
    pub fn report(&self) -> LocalDynamicsReport {
        let amostras = self.ultimo_lpe.amostrados;
        LocalDynamicsReport {
            hotm_passos: self.motors.passos_hotm,
            harmonic_passos: self.motors.passos_harmonic,
            inibidos_agora: self.ultimo_inh.inibidos,
            liberados_agora: self.ultimo_inh.liberados,
            competidores: self.ultimo_inh.competidores,
            vivos: self.ultimo_inh.vivos,
            inibicoes_total: self.inhibition.total_inibicoes,
            err_medio: if amostras > 0 {
                Some(self.ultimo_lpe.err_medio)
            } else {
                None
            },
            surpresa_media: if amostras > 0 {
                Some(self.ultimo_lpe.surpresa_media)
            } else {
                None
            },
            gates_aligned: self.ultimo_lpe.gates_aligned,
            gates_opposing: self.ultimo_lpe.gates_opposing,
            amostrados: amostras,
            sem_predicao: self.ultimo_lpe.sem_predicao,
        }
    }
}

/// Fotografia COM DENOMINADORES (ausência ≠ zero nos Option).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LocalDynamicsReport {
    pub hotm_passos: u64,
    pub harmonic_passos: u64,
    pub inibidos_agora: u64,
    pub liberados_agora: u64,
    pub competidores: u64,
    pub vivos: u64,
    pub inibicoes_total: u64,
    pub err_medio: Option<f64>,
    pub surpresa_media: Option<f64>,
    pub gates_aligned: u64,
    pub gates_opposing: u64,
    pub amostrados: u64,
    pub sem_predicao: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_cluster(id: u64, state: Vec<f64>) -> ClusterBio {
        // ClusterBio mínima para o teste: construtor real do crate.
        let mut rng = SmallRng::seed_from_u64(id);
        let mut c = ClusterBio::new(
            ClusterId::from_uuid(uuid::Uuid::from_u64_pair(id, id)),
            0,
            &mut rng,
        );
        c.state = state;
        c.osc_seed = id;
        c
    }

    #[test]
    fn hotm_normaliza_e_preserva_determinismo() {
        let m = HotmDynamics::for_cluster(7, 16);
        let state: Vec<f64> = (0..16).map(|i| (i as f64 - 8.0) / 8.0).collect();
        let mut rng = SmallRng::seed_from_u64(1);
        let a = m.step(&state, &mut rng);
        let b = m.step(&state, &mut rng);
        assert_eq!(a, b, "A/A bit-identico");
        let norm = a.iter().map(|v| v * v).sum::<f64>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6, "normalizado, norma {norm}");
        assert_eq!(a.len(), 16);
    }

    #[test]
    fn harmonic_avanca_fase_e_modula() {
        let mut h = HarmonicDynamics::for_energy(0.3);
        // 0.3×4=1.2 → banda idx 1 (475 Hz).
        assert!((h.freq - HARMONIC_BANDS[1]).abs() < 1e-9, "banda por energia");
        let p0 = h.phase;
        h.advance();
        assert!(h.phase > p0, "fase avança 2pi*f*dt/1000");
        let state = vec![0.5f64; 8];
        let mut rng = SmallRng::seed_from_u64(2);
        let out = RecurrentDynamics::step(&h, &state, &mut rng);
        assert_eq!(out.len(), 8);
        assert!(out.iter().any(|v| *v != 0.5), "onda aplicada");
        // Ressonância monotônica em Δf.
        let r1 = h.resonance(h.freq, 0.0);
        let r2 = h.resonance(h.freq + 500.0, 0.0);
        assert!(r1 > r2, "Δf maior → ressonância menor");
    }

    #[test]
    fn inibicao_topo_forca_decaimento_liberacao() {
        let mut inh = LocalInhibition::default();
        // 10 clusters: 3 fortes (norma>0.8), 7 fracos.
        let mut clusters: Vec<ClusterBio> = (0..10)
            .map(|i| {
                let amp = if i < 3 { 0.9 } else { 0.1 };
                fake_cluster(i, vec![amp; 8])
            })
            .collect();
        let r = inh.apply(&mut clusters);
        assert_eq!(r.vivos, 10, "denominador");
        assert_eq!(r.competidores, 3, "score>0.8");
        assert!(r.inibidos >= 1, "topo inibido");
        // O inibido foi rebaixado (damp aplicado).
        let forte0 = clusters.iter().find(|c| c.osc_seed == 0).unwrap();
        assert!(
            forte0.state.iter().all(|v| *v < 0.9),
            "estado rebaixado pela inibição"
        );
        // Score = NORMA (8×0.81=2.55): cai <0.8 com amplitude ~0.283
        // — ~22 re-setadas + ~45 de decaimento = ~67 passos.
        let mut r2 = r;
        for _ in 0..80 {
            r2 = inh.apply(&mut clusters);
        }
        assert!(r2.liberados > 0 || inh.total_liberacoes > 0, "liberados <0.1");
        assert!(inh.total_inibicoes >= r.inibidos, "total com denominador");
    }

    #[test]
    fn lpe_erro_surpresa_e_gates_tipados() {
        let mut lpe = LocalPredictionError::default();
        // Passo 1: sem predição (ausência contada).
        let mut clusters = vec![
            fake_cluster(1, vec![0.5; 8]),
            fake_cluster(2, vec![-0.5; 8]),
        ];
        let r1 = lpe.observe(&clusters);
        assert_eq!(r1.sem_predicao, 2, "ausência ≠ zero: contada");
        assert_eq!(r1.amostrados, 0);
        // Passo 2: alinhado (mesma direção) → gate ALIGNED.
        clusters[0].state = vec![0.55; 8];
        // Oposto (inverte sinal) → gate OPPOSING.
        clusters[1].state = vec![0.5; 8];
        let r2 = lpe.observe(&clusters);
        assert_eq!(r2.amostrados, 2, "denominador");
        assert_eq!(r2.gates_aligned, 1);
        assert_eq!(r2.gates_opposing, 1);
        assert!(r2.err_medio > 0.0, "erro real medido");
        assert!(r2.surpresa_media >= 0.0, "entropia do erro");
    }

    #[test]
    fn engine_multimotor_e_a_a_bit_identico() {
        let mk = || {
            let mut cs: Vec<ClusterBio> = (0..6)
                .map(|i| fake_cluster(i, vec![0.3; 12]))
                .collect();
            cs
        };
        let mut e1 = LocalDynamicsEngine::new(12);
        let mut e2 = LocalDynamicsEngine::new(12);
        let mut c1 = mk();
        let mut c2 = mk();
        for step in 1..=5 {
            e1.apply(&mut c1, step);
            e2.apply(&mut c2, step);
        }
        for (a, b) in c1.iter().zip(c2.iter()) {
            assert_eq!(a.state, b.state, "A/A bit-identico entre engines");
        }
        assert!(e1.motors.passos_hotm > 0 && e1.motors.passos_harmonic > 0, "multi-motor real");
        let rep = e1.report();
        assert!(rep.hotm_passos + rep.harmonic_passos > 0, "motores com denominador");
        assert_eq!(rep.vivos, 6);
    }
}
