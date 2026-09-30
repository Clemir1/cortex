//! Runner L1 — um passo completo do substrato, custo O(active).
//!
//! Ordem canônica do step (determinística — um único RNG semeado):
//!
//! 1. **Física** dos clusters devidos: banda tau (`step_interval`) ou
//!    sujos; vizinhos amostrados do grafo (min 1, ratio 4); regra
//!    completa do `update_state`. Dormentes/repairing pausam física
//!    (economia REAL da dormência — custo e intake reduzidos).
//! 2. **Movimento** dos que executaram + grafo incremental.
//! 3. **Energia O1**: custo metabólico (reduzido p/ dormente) + intake
//!    pelo budget homeostático (sensor = média do step anterior).
//! 4. **Survival**: emergência coletiva (quorum com histerese) e ciclo
//!    por cluster (dormancy/wake/repair com histerese contada).
//! 5. **Morte** por `DEATH_THRESHOLD`; **divisão** sob demanda (cap por
//!    step e teto populacional — nunca explosiva); **fusão** de fracos
//!    em fortes (vizinhos com estado próximo).
//! 6. **Compactação** (só em steps com morte/fusão) + sync incremental
//!    da matriz (soma por delta — O(active)).
//! 7. **Reservoir** a cada `RESERVOIR_INTERVAL` steps (Hebbian real).
//! 8. **Publicação**: POST_PHYSICAL (população inteira) e
//!    PRE_COGNITIVE (amostra estratificada ≤ 256) — ambos versionados
//!    pelo ledger, com hashes e cobertura.
//! 9. **Métricas** nas janelas 1/5/20/50/100.
//!
//! Eventos pequenos: o relatório carrega escalares e hashes — nenhum
//! payload 97D cruza fronteiras (ADR-0004).

use crate::cluster::{ClusterBio, ClusterProfile, LifecycleState};
use crate::config as cfg;
use crate::contract::L1Ledger;
use crate::energy::EnergyBudget;
use crate::graph::LocalGraph;
use crate::metrics::L1Metrics;
use crate::reservoir::{HotmReservoir, ReadoutResult};
use crate::state_matrix::ClusterStateMatrix;
use crate::survival::{CollectiveEmergency, SurvivalDecision, SurvivalState};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;
use triad_contracts::l1::{L1Snapshot, L1StatePhase, SamplePolicy};
use triad_foundation::id::ClusterId;
use tracing::debug;
use uuid::Uuid;

/// Relatório de um step — o "evento pequeno" que viaja para fora.
#[derive(Debug, Clone)]
pub struct L1StepReport {
    pub step: u64,
    pub population: usize,
    pub active_executed: usize,
    pub deaths: usize,
    pub divisions: usize,
    pub merges: usize,
    pub dormancy_count: usize,
    pub repair_count: usize,
    pub emergency_active: bool,
    pub mean_energy: f64,
    pub post_physical: L1Snapshot,
    pub pre_cognitive: L1Snapshot,
    pub readout: Option<ReadoutResult>,
    pub ledger_version: u64,
}

/// Runner do substrato L1.
pub struct L1Runner {
    pub seed: u64,
    pub rng: SmallRng,
    pub step: u64,
    /// Configuração vigente (importada de `config/default.toml [l1.*]`).
    pub config: cfg::L1Config,
    pub clusters: Vec<ClusterBio>,
    pub survival: Vec<SurvivalState>,
    pub graph: LocalGraph,
    pub matrix: ClusterStateMatrix,
    pub reservoir: HotmReservoir,
    pub energy: EnergyBudget,
    pub emergency: CollectiveEmergency,
    pub ledger: L1Ledger,
    pub metrics: L1Metrics,
    pub deaths_total: u64,
    pub divisions_total: u64,
    pub merges_total: u64,
    last_mean_energy: f64,
}

impl L1Runner {
    /// Gênese com defaults congelados (consts históricas) — os testes
    /// A/A bit-idênticos dependem disto.
    pub fn new(seed: u64, initial_population: usize) -> Self {
        Self::new_with_config(seed, initial_population, cfg::L1Config::default())
    }

    /// Gênese com a configuração central injetada de
    /// `config/default.toml [l1.*]` (energia de nascimento, setpoint O1,
    /// banda/ganho da homeostase, limiar de divisão, fusão, tetos).
    pub fn new_with_config(
        seed: u64,
        initial_population: usize,
        config: cfg::L1Config,
    ) -> Self {
        let mut rng = SmallRng::seed_from_u64(seed);
        let n = initial_population.min(config.morphogenesis.max_population);
        let mut clusters = Vec::with_capacity(n);
        for _ in 0..n {
            let id = ClusterId::from_uuid(Uuid::from_u64_pair(rng.gen(), rng.gen()));
            let mut c = ClusterBio::new(id, 0, &mut rng);
            c.profile = ClusterProfile::from_config(&config);
            clusters.push(c);
        }
        let survival = (0..n).map(|_| SurvivalState::default()).collect();
        let mut graph = LocalGraph::new(n);
        graph.rebuild(&clusters);
        let mut matrix = ClusterStateMatrix::new(n);
        let alive: Vec<usize> = (0..n).collect();
        matrix.rebuild(&clusters, &alive);
        let mut ledger = L1Ledger::new();
        // Publicação da gênese (step 0): o substrato nasce com contrato.
        let mean = matrix.mean_state.clone();
        let ids: Vec<ClusterId> = clusters.iter().map(|c| c.id).collect();
        ledger.publish(
            0,
            L1StatePhase::PostPhysical,
            n as u32,
            n as u32,
            Some(&mean),
            &ids,
            SamplePolicy::All,
        );
        let mean_energy = if n == 0 {
            0.0
        } else {
            clusters.iter().map(|c| c.energy).sum::<f64>() / n as f64
        };
        // Orçamento de energia com a homeostase injetada ([l1.homeostasis]).
        let energy = EnergyBudget::new_with_config(
            config.energy.target_level,
            (config.homeostasis.band[0], config.homeostasis.band[1]),
            config.homeostasis.adaptation_gain,
        );
        Self {
            seed,
            rng,
            step: 0,
            config,
            clusters,
            survival,
            graph,
            matrix,
            reservoir: HotmReservoir::new(&mut SmallRng::seed_from_u64(seed ^ 0x5eed)),
            energy,
            emergency: CollectiveEmergency::default(),
            ledger,
            metrics: L1Metrics::default(),
            deaths_total: 0,
            divisions_total: 0,
            merges_total: 0,
            last_mean_energy: mean_energy,
        }
    }

    fn alive_indices(&self) -> Vec<usize> {
        (0..self.clusters.len())
            .filter(|&i| {
                self.clusters[i].is_alive()
                    && self.clusters[i].lifecycle.state != LifecycleState::Archived
                    && self.clusters[i].lifecycle.state != LifecycleState::Merged
            })
            .collect()
    }

    fn mean_energy_of(&self) -> f64 {
        let alive = self.alive_indices();
        if alive.is_empty() {
            return 0.0;
        }
        alive.iter().map(|&i| self.clusters[i].energy).sum::<f64>() / alive.len() as f64
    }

    /// Um passo completo do substrato.
    pub fn step(&mut self) -> L1StepReport {
        self.step += 1;
        let s = self.step;
        let mut deaths = 0usize;
        let mut divisions = 0usize;
        let mut merges = 0usize;

        // ---- 1. Física dos devidos (banda tau ∨ dirty), não dormentes ----
        let due: Vec<usize> = (0..self.clusters.len())
            .filter(|&i| {
                let c = &self.clusters[i];
                c.is_alive()
                    && !matches!(
                        c.lifecycle.state,
                        LifecycleState::Archived | LifecycleState::Merged
                    )
                    && c.lifecycle.state != LifecycleState::Dormant
                    && c.lifecycle.state != LifecycleState::Repairing
                    && c.due_this_step(s)
            })
            .collect();

        // Pré-coleta vizinhos + física (17.6): PARALELA por due com rng
        // DERIVADO de (seed, id, step) — determinismo POR CONSTRUÇÃO:
        // cada cluster tem seu próprio stream, então o resultado não
        // depende da ordem de execução ⇒ A/A bit-exato entre runs E
        // entre níveis de paralelismo do rayon (ver teste
        // `fisica_paralela_deterministica_entre_threads`). Dois está-
        // gios SEM clones de ClusterBio: (a) snapshot PARALELO e puro
        // dos vizinhos de cada due (buffer plano: 1 alloc por due);
        // (b) física IN-PLACE em `par_iter_mut` — cada due muta só o
        // seu cluster e lê apenas o snapshot já materializado.
        let seed = self.seed;
        // Streams separados por PROPÓSITO (tag): a amostragem de vi-
        // zinhos e a mutação de estado não compartilham draws (sem
        // correlação artificial), cada uma 100% determinística por
        // (seed, id, step, tag).
        let stream_for = |id: ClusterId, step: u64, tag: u64| -> u64 {
            use std::hash::{Hash, Hasher};
            let mut h = std::hash::DefaultHasher::new();
            seed.hash(&mut h);
            id.hash(&mut h);
            step.hash(&mut h);
            tag.hash(&mut h);
            h.finish()
        };
        let d = cfg::DIMENSIONALITY;
        // (a) snapshots (puros): `due` vem em ordem canônica — o vec
        // final alinha posição-t com due[t].
        let snapshots: Vec<(Vec<usize>, Vec<f64>)> = due
            .par_iter()
            .map(|&i| {
                let id = self.clusters[i].id;
                let mut rng = SmallRng::seed_from_u64(stream_for(id, s, 0xA5));
                let nbs = self.graph.sample_neighbors(i, &mut rng);
                let k = nbs.len();
                let mut flat: Vec<f64> = vec![0.0; k * d];
                for (t, &j) in nbs.iter().enumerate() {
                    flat[t * d..(t + 1) * d].copy_from_slice(&self.clusters[j].state);
                }
                (nbs, flat)
            })
            .collect();
        let due_set: std::collections::HashSet<usize> = due.iter().copied().collect();
        // (b) física in-place: due_set é o mapa posição→snapshot.
        let snaps: Vec<&(Vec<usize>, Vec<f64>)> = snapshots.iter().collect();
        self.clusters
            .par_iter_mut()
            .enumerate()
            .filter(|(i, _)| due_set.contains(i))
            .for_each(|(i, c)| {
                let t = due
                    .binary_search(&i)
                    .expect("due contém todo índice filtrado");
                let flat: &[f64] = &snaps[t].1;
                let k = flat.len() / d;
                let refs: Vec<&[f64]> = (0..k).map(|u| &flat[u * d..(u + 1) * d]).collect();
                let mut rng = SmallRng::seed_from_u64(stream_for(c.id, s, 0x5A));
                c.update_state(s, &mut rng, &refs, 0.0, None);
                c.update_position(&mut rng);
            });
        let moved: Vec<usize> = due;
        if !moved.is_empty() {
            self.graph.update_positions(&self.clusters, &moved);
        }
        let active_executed = moved.len();

        // ---- 2. Energia: custo metabólico + intake O1 ----
        let emergency_bonus = self.emergency.energy_bonus();
        for i in 0..self.clusters.len() {
            let c = &self.clusters[i];
            if !c.is_alive() {
                continue;
            }
            let dormant = c.lifecycle.state == LifecycleState::Dormant;
            let cost = EnergyBudget::metabolic_cost(c.profile.metabolic_cost, dormant);
            let intake =
                self.energy
                    .per_cluster_intake(self.last_mean_energy, dormant, emergency_bonus);
            let c = &mut self.clusters[i];
            c.energy = (c.energy - cost + intake).clamp(0.0, cfg::ENERGY_MAX);
        }
        let mean_energy = self.mean_energy_of();

        // ---- 3. Emergência coletiva (quorum com histerese) ----
        let low_count = self
            .clusters
            .iter()
            .filter(|c| c.is_alive() && c.energy < cfg::SURVIVAL_EMERGENCY_ENERGY_MIN)
            .count();
        let pop_now = self.alive_indices().len();
        self.emergency.update(s, low_count, pop_now, 10);

        // ---- 4. Survival por cluster (histerese) + lifecycle ----
        for i in 0..self.clusters.len() {
            if !self.clusters[i].is_alive() {
                continue;
            }
            let c = &self.clusters[i];
            let dormant = c.lifecycle.state == LifecycleState::Dormant;
            let cost = EnergyBudget::metabolic_cost(c.profile.metabolic_cost, dormant);
            let e_now = c.energy;
            let e_pred = (e_now - cost + cfg::ENERGY_INTAKE_BASE).clamp(0.0, cfg::ENERGY_MAX);
            let age = c.age;
            let decision = self.survival[i].step(s, e_now, e_pred, age, &mut self.clusters[i].energy);
            match decision {
                SurvivalDecision::EnterDormancy => {
                    self.clusters[i].lifecycle.mark_dormant(s, "survival_low_energy");
                }
                SurvivalDecision::ExitDormancy => {
                    self.clusters[i].lifecycle.mark_active(s, "survival_recovered");
                }
                SurvivalDecision::EnterRepair
                | SurvivalDecision::StayRepairing => {
                    if self.clusters[i].lifecycle.state != LifecycleState::Repairing {
                        self.clusters[i].lifecycle.mark_repairing(s, "damage_repair");
                    }
                }
                SurvivalDecision::ExitRepair => {
                    self.clusters[i].lifecycle.mark_active(s, "damage_repaired");
                }
                _ => {}
            }
        }

        // ---- 5. Mortes ----
        for i in 0..self.clusters.len() {
            let c = &self.clusters[i];
            if c.is_alive() {
                continue;
            }
            if c.lifecycle.state != LifecycleState::Archived {
                let reason = if c.lifecycle.state == LifecycleState::Merged {
                    "merged"
                } else {
                    "death_threshold"
                };
                self.clusters[i].lifecycle.mark_archived(s, reason);
                deaths += 1;
            }
        }
        self.deaths_total += deaths as u64;

        // ---- 6. Divisão sob demanda (cap + teto) ----
        let alive_now = self.alive_indices();
        if alive_now.len() < self.config.morphogenesis.max_population {
            // Candidatos: energia acima do threshold, em ordem de índice.
            let candidates: Vec<usize> = alive_now
                .iter()
                .copied()
                .filter(|&i| self.clusters[i].can_divide())
                .take(self.config.morphogenesis.max_divisions_per_step)
                .collect();
            for i in candidates {
                if self.clusters.len() >= self.config.morphogenesis.max_population {
                    break;
                }
                let child_id =
                    ClusterId::from_uuid(Uuid::from_u64_pair(self.rng.gen(), self.rng.gen()));
                let child = self.clusters[i].divide(child_id, s, &mut self.rng);
                let idx = self.clusters.len();
                self.clusters.push(child);
                self.survival.push(SurvivalState::default());
                self.matrix.push_cluster(idx, &self.clusters[idx]);
                self.graph.add_cluster(idx, &self.clusters);
                divisions += 1;
            }
        }
        self.divisions_total += divisions as u64;

        // ---- 7. Fusão de fracos em fortes (vizinhos próximos) ----
        let mut merged_away: Vec<usize> = Vec::new();
        let alive = self.alive_indices();
        for &i in &alive {
            if merges >= cfg::MAX_MERGES_PER_STEP {
                break;
            }
            let ci = &self.clusters[i];
            if ci.energy > cfg::MERGE_ENERGY_MAX
                || ci.lifecycle.state == LifecycleState::Merged
                || merged_away.contains(&i)
            {
                continue;
            }
            let nbs: Vec<usize> = self
                .graph
                .neighbors_of(i)
                .iter()
                .copied()
                .filter(|&j| {
                    j < self.clusters.len()
                        && self.clusters[j].is_alive()
                        && self.clusters[j].energy > self.clusters[i].energy
                        && !merged_away.contains(&j)
                })
                .collect();
            for j in nbs {
                let mut dist = 0.0;
                for k in 0..cfg::DIMENSIONALITY {
                    dist += (self.clusters[i].state[k] - self.clusters[j].state[k]).abs();
                }
                dist /= cfg::DIMENSIONALITY as f64;
                if dist <= self.config.morphogenesis.fusion_threshold {
                    // Fusão: o forte absorve (média ponderada por energia).
                    let wi = self.clusters[i].energy;
                    let wj = self.clusters[j].energy;
                    let total = wi + wj;
                    for k in 0..cfg::DIMENSIONALITY {
                        let v = (self.clusters[i].state[k] * wi + self.clusters[j].state[k] * wj)
                            / total.max(1e-12);
                        self.clusters[j].state[k] = v.clamp(-1.0, 1.0);
                    }
                    self.clusters[j].energy = (wi + wj) * 0.5; // 50% perdido na fusão
                    self.clusters[i].lifecycle.mark_merged(s, "merged_into_stronger");
                    merged_away.push(i);
                    merges += 1;
                    break;
                }
            }
        }
        self.merges_total += merges as u64;
        if deaths + divisions + merges > 0 {
            debug!(mortes = deaths, divisoes = divisions, fusoes = merges, "l1.runner eventos vitais");
        }

        // ---- 8. Compactação (só se houve morte/fusão) e sync ----
        if deaths + merges > 0 {
            // keep[i] alinha clusters e survival pelo MESMO índice.
            let keep: Vec<bool> = (0..self.clusters.len())
                .map(|i| {
                    self.clusters[i].is_alive()
                        && !matches!(
                            self.clusters[i].lifecycle.state,
                            LifecycleState::Archived | LifecycleState::Merged
                        )
                })
                .collect();
            self.clusters = self
                .clusters
                .iter()
                .enumerate()
                .filter(|(i, _)| keep[*i])
                .map(|(_, c)| c.clone())
                .collect();
            self.survival = self
                .survival
                .iter()
                .enumerate()
                .filter(|(i, _)| keep[*i])
                .map(|(_, sv)| sv.clone())
                .collect();
            self.graph.rebuild(&self.clusters);
            let alive: Vec<usize> = (0..self.clusters.len()).collect();
            self.matrix.rebuild(&self.clusters, &alive);
        } else {
            for &i in &moved {
                self.matrix.sync_cluster(i, &self.clusters[i]);
            }
            // Divisões: push já atualizou a matriz.
        }
        self.matrix.finalize();

        // ---- 9. Reservoir (a cada intervalo) ----
        let readout = if s % cfg::RESERVOIR_INTERVAL as u64 == 0 {
            let order: Vec<usize> = self.matrix.order.clone();
            Some(self.reservoir.update(&self.clusters, &order, &mut self.rng))
        } else {
            None
        };

        // ---- 10. Publicações (POST_PHYSICAL + PRE_COGNITIVE) ----
        let pop = self.matrix.order.len() as u32;
        let mean_all = self.matrix.mean_state.clone();
        let ids_all: Vec<ClusterId> =
            self.matrix.order.iter().map(|&i| self.clusters[i].id).collect();
        let post = self.ledger.publish(
            s,
            L1StatePhase::PostPhysical,
            pop,
            pop,
            if pop > 0 { Some(&mean_all) } else { None },
            &ids_all,
            SamplePolicy::All,
        );
        let (sample_idxs, sample_mean) = self.matrix.stratified_sample(cfg::RESERVOIR_SAMPLE_SIZE);
        let sample_ids: Vec<ClusterId> = sample_idxs.iter().map(|&i| self.clusters[i].id).collect();
        let sample_n = sample_idxs.len() as u32;
        let pre = self.ledger.publish(
            s,
            L1StatePhase::PreCognitive,
            pop,
            sample_n,
            sample_mean.as_deref(),
            &sample_ids,
            SamplePolicy::Stratified,
        );

        // ---- 11. Métricas ----
        let dormancy_count = self
            .clusters
            .iter()
            .filter(|c| c.lifecycle.state == LifecycleState::Dormant)
            .count();
        let repair_count = self
            .clusters
            .iter()
            .filter(|c| c.lifecycle.state == LifecycleState::Repairing)
            .count();
        let mean_entropy = if self.clusters.is_empty() {
            0.0
        } else {
            self.clusters.iter().map(|c| c.entropy_cache).sum::<f64>()
                / self.clusters.len() as f64
        };
        let mean_firing = if self.clusters.is_empty() {
            0.0
        } else {
            self.clusters.iter().map(|c| c.firing_rate).sum::<f64>()
                / self.clusters.len() as f64
        };
        let pop_for_metrics = self.clusters.len();
        let active_fraction = if pop_for_metrics > 0 {
            active_executed as f64 / pop_for_metrics as f64
        } else {
            0.0
        };
        self.metrics.push_step(
            pop_for_metrics,
            mean_energy,
            if pop_for_metrics > 0 { deaths as f64 } else { 0.0 },
            divisions as f64,
            merges as f64,
            if pop_for_metrics > 0 {
                dormancy_count as f64 / pop_for_metrics as f64
            } else {
                0.0
            },
            if pop_for_metrics > 0 {
                repair_count as f64 / pop_for_metrics as f64
            } else {
                0.0
            },
            mean_entropy,
            mean_firing,
            active_fraction,
        );
        self.last_mean_energy = mean_energy;
        debug!(passo = s, populacao = pop_for_metrics, energia_media = mean_energy, "l1.runner passo concluído");

        L1StepReport {
            step: s,
            population: pop_for_metrics,
            active_executed,
            deaths,
            divisions,
            merges,
            dormancy_count,
            repair_count,
            emergency_active: self.emergency.active,
            mean_energy,
            post_physical: post,
            pre_cognitive: pre,
            readout,
            ledger_version: self.ledger.version(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use triad_foundation::status::Status;

    /// Smoke determinístico: 200 steps saudáveis — população viva,
    /// versão monotônica, snapshots Value, energia em [0,1], sem NaN.
    #[test]
    fn smoke_200_steps_saudavel() {
        let mut r = L1Runner::new(42, 64);
        let mut last_version = 0u64;
        for k in 1..=200 {
            let rep = r.step();
            assert_eq!(rep.step, k);
            assert!(
                rep.population > 0,
                "sem extinção com homeostase (step {k}: pop {})",
                rep.population
            );
            assert!(rep.mean_energy >= 0.0 && rep.mean_energy <= 1.0);
            assert!(rep.mean_energy.is_finite());
            assert!(rep.ledger_version > last_version, "versão monotônica");
            last_version = rep.ledger_version;
            assert_eq!(rep.post_physical.provider_status, Status::Value);
            assert!(rep.post_physical.mean_state_hash.is_some());
            assert_eq!(rep.pre_cognitive.provider_status, Status::Value);
            assert!(rep.pre_cognitive.sample_coverage.unwrap_or(0.0) <= 1.0);
        }
        // Hebbian provado em 200 steps (40 ciclos de reservoir).
        assert!(r.reservoir.norm_delta_total > 0.0, "Hebbian real");
        assert!(matches!(
            r.reservoir.plasticity,
            crate::reservoir::PlasticityStatus::Hebbian { .. }
        ));
        // A homeostase sustentou a banda na janela 100 (não-fragilidade).
        let s = r.metrics.mean_energy.stats(100).expect("200 steps");
        assert!(s.mean > 0.0 && s.mean <= 1.0);
    }

    /// A/A bit-idêntica: mesma seed → mesma sequência de hashes (o
    /// protocolo científico da doc exige A/A antes de qualquer claim).
    #[test]
    fn aa_bit_identical_mesma_seed() {
        let mut a = L1Runner::new(42, 32);
        let mut b = L1Runner::new(42, 32);
        for _ in 0..50 {
            let ra = a.step();
            let rb = b.step();
            assert_eq!(ra.step, rb.step);
            assert_eq!(ra.population, rb.population);
            assert_eq!(
                ra.post_physical.mean_state_hash, rb.post_physical.mean_state_hash,
                "mean_state_hash tem que ser bit-idêntico"
            );
            assert_eq!(
                ra.post_physical.cluster_order_hash, rb.post_physical.cluster_order_hash,
                "cluster_order_hash tem que ser bit-idêntico"
            );
            assert_eq!(ra.deaths, rb.deaths);
            assert_eq!(ra.divisions, rb.divisions);
            assert_eq!(ra.merges, rb.merges);
        }
        assert_eq!(a.matrix.state_version, b.matrix.state_version);
    }

    /// Seeds diferentes → organismos diferentes (a seed não é decorativa).
    #[test]
    fn seeds_diferentes_estados_diferentes() {
        let mut a = L1Runner::new(1, 32);
        let mut b = L1Runner::new(2, 32);
        let ra = a.step();
        let rb = b.step();
        assert_ne!(ra.post_physical.mean_state_hash, rb.post_physical.mean_state_hash);
    }

    /// Arquitetura: no_stale_as_value — população vazia publica NoData
    /// COM RAZÃO, nunca zeros ou Value falso; e a leitura registra ausência.
    #[test]
    fn populacao_vazia_publica_no_data_com_razao() {
        let mut r = L1Runner::new(7, 0);
        let rep = r.step();
        assert_eq!(rep.population, 0);
        assert_eq!(rep.post_physical.provider_status, Status::NoData);
        assert_eq!(rep.post_physical.no_data_reason.as_deref(), Some("state_absent"));
        assert_eq!(rep.pre_cognitive.provider_status, Status::NoData);
        // Recibo de leitura registra a ausência (E4 não falsificado).
        let receipt = r.ledger.read(L1StatePhase::PostPhysical, "l3_attention");
        assert_eq!(receipt.provider_status, Status::NoData);
        assert!(receipt.fallback_used);
        assert_eq!(r.ledger.consumed_absence, 1);
        assert_eq!(r.ledger.consumed_value, 0);
    }

    /// Divisão respeita o teto e o cap por step (nunca explosiva).
    #[test]
    fn divisao_nunca_explosiva() {
        let mut r = L1Runner::new(11, 8);
        let mut max_pop_seen = 0usize;
        for _ in 0..300 {
            let rep = r.step();
            assert!(rep.divisions <= cfg::MAX_DIVISIONS_PER_STEP);
            max_pop_seen = max_pop_seen.max(rep.population);
        }
        assert!(max_pop_seen <= cfg::MAX_POPULATION, "teto populacional");
    }

    /// O(active): com bandas tau, a fração executada fica abaixo de 1
    /// após os clusters envelhecerem (custo real, não declarado).
    #[test]
    fn bandas_tau_reduzem_custo_ativo() {
        let mut r = L1Runner::new(13, 64);
        // Primeiro step: todos jovens → todos executam.
        let rep0 = r.step();
        assert_eq!(rep0.active_executed, 64);
        for _ in 0..80 {
            r.step();
        }
        // Após envelhecer (tau_age ≥ 20 → banda 2), metade dos steps
        // executa ~metade. Coletamos a média da fração ativa.
        let mut fractions = Vec::new();
        for _ in 0..40 {
            let rep = r.step();
            fractions.push(rep.active_executed as f64 / rep.population as f64);
        }
        let mean_frac = fractions.iter().sum::<f64>() / fractions.len() as f64;
        assert!(
            mean_frac < 1.0,
            "bandas tau poupam execução (fração média {mean_frac:.3})"
        );
    }
}
