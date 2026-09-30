//! `ClusterBio` Rust — entidade dinâmica do substrato L1.
//!
//! Referência: `core/cluster.py` legado (1.128 linhas, grau 295 no
//! Graphify). Fiel ao legado em: estado 97D com as 6 partições; regra de
//! atualização `state' = clip(state + local + vizinhos + memória + mutação
//! + gradiente, -1, 1)`; EMA de memória (0.7/0.3); entropia |x|; tau_age;
//! divisão por threshold de energia; morte por `DEATH_THRESHOLD`.
//!
//! Aprimoramentos desta reescrita (defeitos provados nas auditorias):
//! - **A5** `tau_stability` por EMA do delta (`state_delta_ema`) em vez de
//!   janela de 20 estados completos por cluster (20×97×8B cada): mesma
//!   semântica, memória O(1) — o legado pagava MBs por cluster.
//! - **A6** `firing_state` com limiares constantes (0..4) e EMA contado.
//! - **A7** banda tau (`step_interval`) nativa: cluster antigo executa a
//!   cada 2/5/10 steps — custo O(active), requisito do L1AL5 §3.
//! - **A8** lifecycle com transições contadas e razão tipada desde o
//!   nascimento (o legado adicionou isso depois, por instrumentação).
//!
//! FORA (não portado — estruturas desnecessárias sem prova no núcleo L1):
//! EML state (L3), tissue_affinity/dominant_tissue (L2), cognitive_value/
//! contribution (L3), genome/expressão de genes (O5/extensão), 7
//! reservatórios de energia (substituídos por O1 escalar), development
//! stages embrionários (development/).

use crate::config as cfg;
use crate::math;
use rand::rngs::SmallRng;
use rand::Rng;
use tracing::debug;
use triad_foundation::id::ClusterId;

/// Fativas do estado 97D (config::slice).
pub use crate::config::slice as state_slice;

/// Estado do ciclo de vida — transições contadas (auditoria L1 §12:
/// lifecycle com razão tipada).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Active,
    Dormant,
    Repairing,
    /// Fundido em outro cluster; aguarda remoção física.
    Merged,
    /// Morto/arquivado; aguarda remoção física.
    Archived,
}

impl LifecycleState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "ACTIVE",
            Self::Dormant => "DORMANT",
            Self::Repairing => "REPAIRING",
            Self::Merged => "MERGED",
            Self::Archived => "ARCHIVED",
        }
    }
}

/// Lifecycle com razão e step de entrada.
#[derive(Debug, Clone, Copy)]
pub struct Lifecycle {
    pub state: LifecycleState,
    pub reason: &'static str,
    pub entered_step: u64,
    pub transitions: u32,
}

impl Default for Lifecycle {
    fn default() -> Self {
        Self {
            state: LifecycleState::Active,
            reason: "birth",
            entered_step: 0,
            transitions: 0,
        }
    }
}

impl Lifecycle {
    fn mark(&mut self, state: LifecycleState, reason: &'static str, step: u64) {
        if self.state != state {
            self.transitions += 1;
            self.state = state;
            self.reason = reason;
            self.entered_step = step;
            debug!(state = state.as_str(), reason, step, "transição de lifecycle");
        }
    }

    pub fn mark_dormant(&mut self, step: u64, reason: &'static str) {
        self.mark(LifecycleState::Dormant, reason, step);
    }
    pub fn mark_repairing(&mut self, step: u64, reason: &'static str) {
        self.mark(LifecycleState::Repairing, reason, step);
    }
    pub fn mark_active(&mut self, step: u64, reason: &'static str) {
        self.mark(LifecycleState::Active, reason, step);
    }
    pub fn mark_merged(&mut self, step: u64, reason: &'static str) {
        self.mark(LifecycleState::Merged, reason, step);
    }
    pub fn mark_archived(&mut self, step: u64, reason: &'static str) {
        // Merged/Archived são terminais: contam transição mesmo repetindo.
        if self.state != LifecycleState::Archived {
            self.transitions += 1;
            debug!(reason, step, "cluster morreu");
        }
        self.state = LifecycleState::Archived;
        self.reason = reason;
        self.entered_step = step;
    }
}

/// Perfil de nascimento — os 4 parâmetros do genoma que o update de L1
/// de fato consome (metabolic_cost, mutation_rate, division_threshold,
/// initial_energy). O sistema de genoma completo fica fora do core
/// (NUCLEO_MINIMO §32: genoma fora do loop principal).
#[derive(Debug, Clone, Copy)]
pub struct ClusterProfile {
    pub metabolic_cost: f64,
    pub mutation_rate: f64,
    pub division_threshold: f64,
    pub initial_energy: f64,
}

impl Default for ClusterProfile {
    fn default() -> Self {
        Self {
            metabolic_cost: cfg::METABOLIC_BASE_COST,
            mutation_rate: cfg::DEFAULT_MUTATION_RATE,
            division_threshold: cfg::DEFAULT_DIVISION_THRESHOLD,
            initial_energy: cfg::INITIAL_ENERGY,
        }
    }
}

impl ClusterProfile {
    /// Perfil de nascimento derivado da config central injetada
    /// (`[l1.energy]` e `[l1.morphogenesis] division_threshold`); custo
    /// metabólico e mutação são mecânica (consts).
    pub fn from_config(config: &cfg::L1Config) -> Self {
        Self {
            metabolic_cost: cfg::METABOLIC_BASE_COST,
            mutation_rate: cfg::DEFAULT_MUTATION_RATE,
            division_threshold: config.morphogenesis.division_threshold,
            initial_energy: config.energy.initial_level,
        }
    }
}

/// Entidade dinâmica do substrato.
#[derive(Clone)]
pub struct ClusterBio {
    pub id: ClusterId,
    pub birth_step: u64,
    /// Estado 97D, clipado em [-1, 1].
    pub state: Vec<f64>,
    /// Traço de memória (RESERVOIR_MEMORY_DIM) — EMA do short_memory.
    pub memory_trace: Vec<f64>,
    /// Energia escalar [0, ENERGY_MAX] — O1 fecha o loop em energy.rs.
    pub energy: f64,
    /// Idade em steps (dt = 1.0).
    pub age: f64,
    /// EMA de |Δstate| — taxa de disparo.
    pub firing_rate: f64,
    /// 0=silent, 1=low, 2=medium, 3=high, 4=spiking (A6).
    pub firing_state: u8,
    /// Entropia cacheada do estado atual.
    pub entropy_cache: f64,
    pub tau_age: f64,
    pub tau_stability: f64,
    /// A5: EMA do delta de estado — substrato de tau_stability O(1).
    pub state_delta_ema: f64,
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    /// Semente de fase da oscilação por idade (herança do legado `id % 7`,
    /// agora sem depender da representação do UUID — determinística via
    /// RNG semeado da run).
    pub osc_seed: u64,
    pub profile: ClusterProfile,
    pub lifecycle: Lifecycle,
    /// Sujo = precisa de atualização física neste step (A7/O(active)).
    pub dirty: bool,
    pub last_updated_step: u64,
}

impl ClusterBio {
    /// Nasce com estado de baixa amplitude, energia inicial e posição
    /// dentro dos bounds (determinístico dado o rng).
    pub fn new(id: ClusterId, step: u64, rng: &mut SmallRng) -> Self {
        let mut state = Vec::with_capacity(cfg::DIMENSIONALITY);
        for _ in 0..cfg::DIMENSIONALITY {
            state.push(0.02 * math::normal(rng));
        }
        let position = [
            rng.gen::<f64>() * cfg::POSITION_BOUNDS[0],
            rng.gen::<f64>() * cfg::POSITION_BOUNDS[1],
            rng.gen::<f64>() * cfg::POSITION_BOUNDS[2],
        ];
        let profile = ClusterProfile::default();
        let mut c = Self {
            id,
            birth_step: step,
            state,
            memory_trace: vec![0.001; cfg::RESERVOIR_MEMORY_DIM],
            energy: profile.initial_energy,
            age: 0.0,
            firing_rate: 0.0,
            firing_state: 0,
            entropy_cache: 0.0,
            tau_age: 0.0,
            tau_stability: 0.5,
            state_delta_ema: 0.0,
            position,
            velocity: [0.0; 3],
            osc_seed: rng.gen::<u64>(),
            profile,
            lifecycle: Lifecycle::default(),
            dirty: true,
            last_updated_step: step,
        };
        c.entropy_cache = math::entropy_abs(&c.state);
        debug!(cluster_id = ?c.id, step, "cluster nasceu");
        c
    }

    // --- Fativas nomeadas (partições 97D) -----------------------------------

    pub fn sensory(&self) -> &[f64] {
        &self.state[state_slice::SENSORY]
    }
    pub fn short_memory(&self) -> &[f64] {
        &self.state[state_slice::SHORT_MEMORY]
    }
    pub fn energy_region(&self) -> &[f64] {
        &self.state[state_slice::ENERGY_REGION]
    }
    pub fn communication(&self) -> &[f64] {
        &self.state[state_slice::COMMUNICATION]
    }
    pub fn structural(&self) -> &[f64] {
        &self.state[state_slice::STRUCTURAL]
    }
    pub fn adaptive(&self) -> &[f64] {
        &self.state[state_slice::ADAPTIVE]
    }

    // --- Dinâmica ------------------------------------------------------------

    /// Regra de atualização completa (legado `update_state`):
    ///
    /// `state' = clip(state + local + vizinhos + memória + mutação + gradiente)`
    ///
    /// `neighbor_states`: estados dos vizinhos amostrados (o runner coleta
    /// antes de mutar — sem aliasing de borrow).
    #[allow(clippy::too_many_arguments)]
    pub fn update_state(
        &mut self,
        step: u64,
        rng: &mut SmallRng,
        neighbor_states: &[&[f64]],
        gradient: f64,
        external: Option<&[f64]>,
    ) {
        let d = cfg::DIMENSIONALITY;
        debug_assert_eq!(self.state.len(), d);

        // Fase determinística por cluster (legado: id % 7).
        let phase = (self.osc_seed % 7) as f64 / 7.0 * std::f64::consts::TAU;

        let prev_delta = if step == self.last_updated_step {
            None
        } else {
            Some(self.state.clone())
        };

        for i in 0..d {
            // Dinâmica local escalada pela energia (tanh).
            let mut delta = cfg::STATE_LOCAL_DYNAMICS_SCALE * math::normal(rng) * self.energy.tanh();
            // Oscilação por idade com fase própria.
            if self.age > 0.0 {
                delta += cfg::STATE_AGE_OSCILLATION_AMPLITUDE
                    * (std::f64::consts::TAU * self.age / cfg::STATE_AGE_OSCILLATION_PERIOD
                        + phase)
                        .sin()
                    * rng.gen::<f64>();
            }
            // Entrada dos vizinhos: média das diferenças.
            if !neighbor_states.is_empty() {
                let mut sum = 0.0;
                for ns in neighbor_states {
                    sum += ns[i] - self.state[i];
                }
                delta += sum / neighbor_states.len() as f64;
                delta += cfg::STATE_NEIGHBOR_INPUT_NOISE * math::normal(rng);
            }
            // Viés de memória no short_memory.
            let m = state_slice::SHORT_MEMORY;
            if i >= m.start && i < m.end {
                let k = i - m.start;
                if k < self.memory_trace.len() {
                    delta += self.memory_trace[k];
                }
            }
            // Mutação.
            delta += math::normal(rng) * self.profile.mutation_rate * cfg::STATE_MUTATION_SCALE;
            // Gradiente aplicado na região adaptativa.
            let a = state_slice::ADAPTIVE;
            if i >= a.start && i < a.end {
                delta += self.state[i] * gradient * 0.01;
            }
            self.state[i] = (self.state[i] + delta).clamp(-1.0, 1.0);
        }

        // Entrada externa (sensory) — chega do runner, clipada.
        if let Some(ext) = external {
            for (i, &v) in ext.iter().enumerate().take(d) {
                self.state[i] = (self.state[i] + v).clamp(-1.0, 1.0);
            }
        }

        self.age += 1.0;

        // EMA do traço de memória sobre o short_memory atual.
        let m = state_slice::SHORT_MEMORY;
        for k in 0..self.memory_trace.len().min(m.len()) {
            let v = cfg::STATE_MEMORY_EMA_RETENTION * self.memory_trace[k]
                + cfg::STATE_MEMORY_EMA_ALPHA * self.state[m.start + k];
            self.memory_trace[k] = v.max(cfg::MEMORY_TRACE_FLOOR);
        }

        // Delta → firing (A6) e tau (A5).
        if let Some(prev) = prev_delta {
            let delta = math::mean_abs_delta(&prev, &self.state);
            self.state_delta_ema = 0.9 * self.state_delta_ema + 0.1 * delta;
            self.firing_rate =
                (1.0 - cfg::FIRING_EMA_ALPHA) * self.firing_rate + cfg::FIRING_EMA_ALPHA * delta;
        }
        self.update_firing_state();
        self.entropy_cache = math::entropy_abs(&self.state);
        self.update_tau();
        self.last_updated_step = step;
        self.dirty = false;
    }

    /// Atualiza tau (A5: estabilidade = exp(-EMA_delta × janela)).
    pub fn update_tau(&mut self) {
        if !cfg::TAU_ENABLED {
            return;
        }
        self.tau_age += 1.0;
        self.tau_stability =
            (-self.state_delta_ema * cfg::TAU_CLUSTER_STABILITY_WINDOW).exp();
    }

    fn update_firing_state(&mut self) {
        let f = self.firing_rate;
        self.firing_state = if f < cfg::FIRING_STATE_THRESHOLDS[0] {
            0
        } else if f < cfg::FIRING_STATE_THRESHOLDS[1] {
            1
        } else if f < cfg::FIRING_STATE_THRESHOLDS[2] {
            2
        } else if f < 0.5 {
            3
        } else {
            4
        };
    }

    // --- Fisiologia ------------------------------------------------------------

    pub fn is_alive(&self) -> bool {
        self.energy > cfg::DEATH_THRESHOLD
    }

    pub fn can_divide(&self) -> bool {
        self.energy > self.profile.division_threshold
    }

    /// Custo metabólico por step (chamado pelo energy budget).
    pub fn apply_metabolic_cost(&mut self) {
        self.energy = (self.energy - self.profile.metabolic_cost).clamp(0.0, cfg::ENERGY_MAX);
    }

    /// Recebe energia (intake do budget), clampada.
    pub fn receive_energy(&mut self, amount: f64) {
        self.energy = (self.energy + amount).clamp(0.0, cfg::ENERGY_MAX);
    }

    /// Conectividade 0..1 (legado `calculate_connectivity`).
    pub fn connectivity_score(&self, connection_count: usize, neighbor_count: usize) -> f64 {
        let effective = connection_count as f64
            + cfg::CONNECTIVITY_NEIGHBOR_WEIGHT * neighbor_count as f64;
        (effective / cfg::CONNECTIVITY_DIVISOR).min(cfg::CONNECTIVITY_MAX_DISPLAY)
    }

    /// Movimento com damping (legado `update_position`).
    pub fn update_position(&mut self, rng: &mut SmallRng) {
        for i in 0..3 {
            self.position[i] = (self.position[i] + self.velocity[i]) % cfg::POSITION_BOUNDS[i];
            self.velocity[i] *= cfg::VELOCITY_DAMPING;
        }
        // Inércia mínima para o grafo não congelar (determinístico).
        if self.velocity.iter().all(|v| v.abs() < 1e-4) {
            for v in &mut self.velocity {
                *v += 0.001 * math::normal(rng);
            }
        }
    }

    // --- Bandas tau (A7: custo O(active)) ------------------------------------

    /// A cada quantos steps este cluster precisa executar física completa?
    pub fn step_interval(&self) -> u32 {
        if !cfg::TAU_ENABLED {
            return 1;
        }
        if self.tau_age < cfg::TAU_BAND_YOUNG_MAX {
            1
        } else if self.tau_age < cfg::TAU_BAND_MATURE_MAX {
            2
        } else if self.tau_age < cfg::TAU_BAND_STABLE_MAX {
            5
        } else {
            cfg::TAU_BAND_ANCIENT_INTERVAL
        }
    }

    /// Se este step toca a banda do cluster.
    pub fn due_this_step(&self, step: u64) -> bool {
        let interval = self.step_interval() as u64;
        step % interval == 0 || self.dirty
    }

    /// Divide o cluster: filho herda metade da energia e estado com ruído.
    pub fn divide(&mut self, child_id: ClusterId, step: u64, rng: &mut SmallRng) -> ClusterBio {
        debug_assert!(self.can_divide());
        let half = self.energy / 2.0;
        self.energy = half;
        let mut child = ClusterBio::new(child_id, step, rng);
        child.profile = self.profile;
        child.state = self
            .state
            .iter()
            .map(|&v| (v + 0.05 * math::normal(rng)).clamp(-1.0, 1.0))
            .collect();
        child.memory_trace = self.memory_trace.clone();
        child.energy = half;
        child.tau_age = 0.0;
        child.position = [
            self.position[0] + 0.1 * math::normal(rng),
            self.position[1] + 0.1 * math::normal(rng),
            self.position[2] + 0.1 * math::normal(rng),
        ];
        for (i, p) in child.position.iter_mut().enumerate() {
            *p = p.rem_euclid(cfg::POSITION_BOUNDS[i]);
        }
        self.dirty = true;
        child.dirty = true;
        debug!(parent_id = ?self.id, child_id = ?child.id, step, "cluster dividiu");
        child
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn seeded() -> SmallRng {
        SmallRng::seed_from_u64(1234)
    }

    #[test]
    fn nascimento_e_fativas() {
        let mut rng = seeded();
        let c = ClusterBio::new(ClusterId::new(), 0, &mut rng);
        assert_eq!(c.state.len(), cfg::DIMENSIONALITY);
        assert_eq!(c.sensory().len(), 16);
        assert_eq!(c.short_memory().len(), 16);
        assert_eq!(c.adaptive().len(), 17);
        assert_eq!(c.energy, cfg::INITIAL_ENERGY);
        assert_eq!(c.lifecycle.state, LifecycleState::Active);
    }

    #[test]
    fn update_state_clipa_e_envelhece() {
        let mut rng = seeded();
        let mut c = ClusterBio::new(ClusterId::new(), 0, &mut rng);
        let neighbors: Vec<Vec<f64>> = vec![vec![0.5; cfg::DIMENSIONALITY]];
        let refs: Vec<&[f64]> = neighbors.iter().map(|v| v.as_slice()).collect();
        c.update_state(1, &mut rng, &refs, 0.0, None);
        assert_eq!(c.age, 1.0);
        assert!(c.state.iter().all(|v| (-1.0..=1.0).contains(v)));
        assert!(c.firing_rate > 0.0);
        assert!(c.memory_trace.iter().all(|&m| m >= cfg::MEMORY_TRACE_FLOOR));
    }

    #[test]
    fn divisao_transfere_metade_da_energia() {
        let mut rng = seeded();
        let mut c = ClusterBio::new(ClusterId::new(), 0, &mut rng);
        c.energy = 0.9;
        assert!(c.can_divide());
        let child = c.divide(ClusterId::new(), 1, &mut rng);
        assert!((c.energy - 0.45).abs() < 1e-12);
        assert!((child.energy - 0.45).abs() < 1e-12);
        assert!(child.dirty && c.dirty);
    }

    #[test]
    fn lifecycle_conta_transicoes_com_razao() {
        let mut lc = Lifecycle::default();
        lc.mark_dormant(3, "low_energy");
        assert_eq!(lc.state, LifecycleState::Dormant);
        assert_eq!(lc.transitions, 1);
        assert_eq!(lc.reason, "low_energy");
        lc.mark_dormant(4, "low_energy"); // mesma transição não conta de novo
        assert_eq!(lc.transitions, 1);
        lc.mark_active(9, "restored");
        assert_eq!(lc.transitions, 2);
        lc.mark_archived(10, "death_threshold");
        assert_eq!(lc.state, LifecycleState::Archived);
        assert_eq!(lc.transitions, 3);
    }

    #[test]
    fn bandas_tau_reduzem_frequencia_com_idade() {
        let mut rng = seeded();
        let mut c = ClusterBio::new(ClusterId::new(), 0, &mut rng);
        assert_eq!(c.step_interval(), 1);
        c.tau_age = 20.0;
        assert_eq!(c.step_interval(), 2);
        c.tau_age = 75.0;
        assert_eq!(c.step_interval(), 5);
        c.tau_age = 150.0;
        assert_eq!(c.step_interval(), cfg::TAU_BAND_ANCIENT_INTERVAL);
        // cluster sujo executa sempre
        c.dirty = true;
        assert!(c.due_this_step(1));
    }
}
