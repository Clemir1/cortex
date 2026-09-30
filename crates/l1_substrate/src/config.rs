//! Constantes L1 — centralizadas (o legado as mantém em `core/config.py`;
//! aqui o `cluster.rs` não carrega números mágicos).
//!
//! Valores herdados do legado (grep em `core/config.py`, conferidos um a
//! um). Aprimoramentos desta reescrita vêm marcados com `A1`, `A2`… e a
//! razão de cada um.

/// Dimensão do estado por cluster (legado: 97).
pub const DIMENSIONALITY: usize = 97;

// --- Partições do estado 97D (legado: config.py :1361-:1377) ----------------
// sensory [0,15) · short_memory [16,32) · energy_region [32,48)
// communication [48,64) · structural [64,80) · adaptive [80,97)
// (índices 15 e 31..32? não: boundaries [0,16,32,48,64,80,97] — o slot 15
// é pré-short_memory e fica fora de fatia nomeada, como no legado.)
pub const STATE_BOUNDARIES: [usize; 7] = [0, 16, 32, 48, 64, 80, 97];

/// Fativas nomeadas (Range em const é estável).
pub mod slice {
    use std::ops::Range;
    pub const SENSORY: Range<usize> = 0..16;
    pub const SHORT_MEMORY: Range<usize> = 16..32;
    pub const ENERGY_REGION: Range<usize> = 32..48;
    pub const COMMUNICATION: Range<usize> = 48..64;
    pub const STRUCTURAL: Range<usize> = 64..80;
    pub const ADAPTIVE: Range<usize> = 80..97;
}

// --- Dinâmica local (legado :1231-:1237, :1282-:1283) -----------------------
pub const STATE_LOCAL_DYNAMICS_SCALE: f64 = 0.05;
pub const STATE_MUTATION_SCALE: f64 = 0.01;
pub const STATE_MEMORY_EMA_ALPHA: f64 = 0.3;
pub const STATE_MEMORY_EMA_RETENTION: f64 = 0.7;
pub const STATE_NEIGHBOR_SAMPLE_MIN: usize = 1;
pub const STATE_NEIGHBOR_SAMPLE_RATIO: usize = 4;
pub const STATE_NEIGHBOR_INPUT_NOISE: f64 = 0.01;
pub const STATE_AGE_OSCILLATION_AMPLITUDE: f64 = 0.1;
pub const STATE_AGE_OSCILLATION_PERIOD: f64 = 20.0;
pub const MEMORY_TRACE_FLOOR: f64 = 0.001;

// --- Energia / metabolismo (legado :694, :1089, :1056, :1088) --------------
/// Abaixo disso o cluster morre (0.12 — corrigido no legado 08/09).
pub const DEATH_THRESHOLD: f64 = 0.12;
pub const METABOLIC_BASE_COST: f64 = 0.005;
pub const INITIAL_ENERGY: f64 = 0.8;
pub const DEFAULT_MUTATION_RATE: f64 = 0.10;
pub const DEFAULT_DIVISION_THRESHOLD: f64 = 0.3;
/// Energia máxima clampada (o legado usava pools até 1.5; aqui o loop O1
/// mantém [0,1] — A1: energia escalar com homeostase fechada em vez de
/// 7 reservatórios nunca provados; ver energy.rs).
pub const ENERGY_MAX: f64 = 1.0;
/// Setpoint da homeostase O1 (default.toml `[l1.energy] target_level`).
pub const ENERGY_TARGET_LEVEL: f64 = 0.8;
/// Intake base por cluster vivo por step (fechado pelo loop O1).
pub const ENERGY_INTAKE_BASE: f64 = 0.004;
pub const INTAKE_RATE_MIN: f64 = 0.5;
pub const INTAKE_RATE_MAX: f64 = 2.0;
/// Dormente recebe 25% do intake (metabolismo reduzido — economia real).
pub const DORMANT_INTAKE_FACTOR: f64 = 0.25;
/// Dormente paga 25% do custo metabólico.
pub const DORMANT_COST_FACTOR: f64 = 0.25;

// --- Morfogênese sob demanda (A3: teto explícito anti-explosão) -----------
/// População máxima do substrato (o legado explodiu para 1.999 clusters).
pub const MAX_POPULATION: usize = 1024;
/// Divisões por step (legado: 1 no morphogenesis; aqui teto por step).
pub const MAX_DIVISIONS_PER_STEP: usize = 4;
pub const MAX_MERGES_PER_STEP: usize = 4;
/// Distância média |Δstate| abaixo da qual fusão é permitida.
pub const FUSION_STATE_DIST: f64 = 0.15;
/// Energia abaixo da qual o cluster pode fundir-se ao vizinho.
pub const MERGE_ENERGY_MAX: f64 = 0.15;

// --- Movimento (legado :864) -----------------------------------------------
pub const VELOCITY_DAMPING: f64 = 0.99;
/// Limites do espaço de posições (legado recebia bounds externos;
/// fixado aqui — A2: determinismo do grafo espacial sem parâmetro oculto).
pub const POSITION_BOUNDS: [f64; 3] = [64.0, 64.0, 64.0];
/// Raio de vizinhança (legado: GENOME_DEFAULT_NEIGHBOR_RADIUS).
pub const NEIGHBOR_RADIUS: f64 = 3.0;

// --- Survival (legado :619, :792-:850, :832-:836, :1284) --------------------
pub const SURVIVAL_INTERVAL: u32 = 1;
pub const SURVIVAL_HISTORY_LIMIT: usize = 20;
pub const SURVIVAL_TREND_WINDOW: usize = 5;
pub const SURVIVAL_ENERGY_RISK_WEIGHT: f64 = 0.5;
pub const SURVIVAL_STRESS_RISK_WEIGHT: f64 = 0.35;
pub const SURVIVAL_DAMAGE_RISK_WEIGHT: f64 = 0.15;
pub const SURVIVAL_PREEMPTIVE_RISK_THRESHOLD: f64 = 0.45;
pub const SURVIVAL_PREEMPTIVE_STRESS_THRESHOLD: f64 = 0.55;
pub const SURVIVAL_PREDICTED_ENERGY_LOW: f64 = 0.35;
pub const SURVIVAL_DORMANCY_PREDICTED_ENERGY: f64 = 0.65;
pub const SURVIVAL_DORMANCY_PREDICTED_STRESS: f64 = 0.55;
pub const SURVIVAL_DORMANCY_ENERGY_THRESHOLD: f64 = 0.5;
pub const SURVIVAL_EMERGENCY_ENERGY_MIN: f64 = 0.3;
pub const SURVIVAL_EMERGENCY_RISK_MAX: f64 = 0.6;
pub const SURVIVAL_EMERGENCY_QUORUM_MIN: f64 = 0.3;
pub const SURVIVAL_EMERGENCY_ENERGY_GAIN: f64 = 0.05;
pub const SURVIVAL_EMERGENCY_QUORUM_REDUCTION: f64 = 0.05;
pub const SURVIVAL_REPAIR_RISK_THRESHOLD: f64 = 0.55;
pub const SURVIVAL_REPAIR_THRESHOLD: f64 = 0.01;
pub const SURVIVAL_ENERGY_MIN_REPAIR: f64 = 0.20;
pub const SURVIVAL_DAMAGE_ACCUMULATION_RATE: f64 = 0.02;
pub const SURVIVAL_DAMAGE_REPAIR_RATE_HIGH: f64 = 0.04;
pub const SURVIVAL_DAMAGE_REPAIR_RATE_LOW: f64 = 0.02;
pub const SURVIVAL_DAMAGE_ENERGY_COST_HIGH: f64 = 0.015;
pub const SURVIVAL_DAMAGE_ENERGY_COST_LOW: f64 = 0.01;
pub const SURVIVAL_RESILIENCE_BONUS: f64 = 0.1;
pub const SURVIVAL_RESILIENCE_PENALTY_DAMAGE: f64 = 0.25;
pub const SURVIVAL_RESILIENCE_BONUS_SURVIVAL: f64 = 0.2;
pub const SURVIVAL_AGE_FACTOR_DIVISOR: f64 = 1000.0;
pub const SURVIVAL_AGE_FACTOR_MULTIPLIER: f64 = 0.3;

/// Histerese dormancy↔wake↔repair — A3 (aprimoramento):
/// o legado registrou os campos de histerese (sleep_start_step,
/// woke_at_step, repair_wave_guard_until) mas o repair explodia para
/// 1.999 clusters no step 30 e dormancy para 939 (auditoria L1AL5 §2).
/// Aqui os guardas têm limites explícitos e contados.
pub const DORMANCY_MIN_SLEEP_STEPS: u64 = 5;
pub const DORMANCY_MIN_AWAKE_STEPS: u64 = 10;
pub const DORMANCY_WAKE_COOLDOWN_STEPS: u64 = 8;
pub const REPAIR_WAVE_GUARD_STEPS: u64 = 15;

// --- Tau / bandas de execução (legado :2595-:2642, :2675-:2676) ------------
pub const TAU_ENABLED: bool = true;
pub const TAU_CLUSTER_STABILITY_WINDOW: f64 = 20.0;
pub const TAU_CLUSTER_EMA_ALPHA: f64 = 0.05;
pub const TAU_BAND_YOUNG_MAX: f64 = 10.0; // intervalo 1
pub const TAU_BAND_MATURE_MAX: f64 = 50.0; // intervalo 2
pub const TAU_BAND_STABLE_MAX: f64 = 100.0; // intervalo 5
pub const TAU_BAND_ANCIENT_INTERVAL: u32 = 10;
pub const TAU_FUSION_SKIP_AGE: f64 = 25.0;
pub const TAU_FUSION_SKIP_STABILITY: f64 = 0.65;

// --- Reservoir / HOTM / Hebbian (legado :622, :1344-:1345, :1400-:1421) -----
pub const RESERVOIR_INTERVAL: u32 = 5;
pub const RESERVOIR_MEMORY_DIM: usize = 16;
pub const RESERVOIR_FEATURE_DIM: usize = 8;
pub const RESERVOIR_SAMPLE_SIZE: usize = 256;
pub const RESERVOIR_HEBBIAN_ETA: f64 = 0.01;
pub const RESERVOIR_HEBBIAN_SPARSE: f64 = 0.1;
pub const RESERVOIR_HEBBIAN_CLIP_MIN: f64 = -0.5;
pub const RESERVOIR_HEBBIAN_CLIP_MAX: f64 = 0.5;
pub const RESERVOIR_HEBBIAN_DECAY_PROB: f64 = 0.01;
pub const RESERVOIR_HEBBIAN_DECAY_FACTOR: f64 = 0.99;
pub const RESERVOIR_HISTORY_MAXLEN: usize = 50;

// --- Conectividade (legado :897-:899) --------------------------------------
pub const CONNECTIVITY_NEIGHBOR_WEIGHT: f64 = 0.1;
pub const CONNECTIVITY_DIVISOR: f64 = 10.0;
pub const CONNECTIVITY_MAX_DISPLAY: f64 = 1.0;

// --- Firing (legado state_matrix col 25/26) --------------------------------
/// Limiares do estado de disparo: 0=silent, 1=low, 2=medium, 3=high,
/// 4=spiking (A4: limiares explícitos; o legado os calculava sem
/// constante central).
pub const FIRING_STATE_THRESHOLDS: [f64; 3] = [0.05, 0.15, 0.30];
pub const FIRING_EMA_ALPHA: f64 = 0.1;

// ===========================================================================
// CONFIG INJETÁVEL (diretriz do dono: config centralizada em config/)
// ---------------------------------------------------------------------------
// As structs abaixo importam as seções `[l1.*]` do `config/default.toml`.
// Os defaults são os VALORES CONGELADOS das consts históricas acima — os
// testes e construtores antigos não mudam de comportamento. O app injeta
// o TOML central; a separação política×mecânica (e as chaves não
// aplicáveis, com razão) vive em `config/README.md`.
// ===========================================================================

/// `[l1.energy]` — o que o substrato REAL consome: nível inicial dos
/// clusters e setpoint do loop O1.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct EnergyCfg {
    /// Energia de nascimento de cada cluster (`initial_level`).
    pub initial_level: f64,
    /// Setpoint homeostático do O1 (`target_level`).
    pub target_level: f64,
}

impl Default for EnergyCfg {
    fn default() -> Self {
        Self {
            initial_level: INITIAL_ENERGY,
            target_level: ENERGY_TARGET_LEVEL,
        }
    }
}

/// `[l1.homeostasis]` — banda e ganho do corretor O1.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct HomeostasisCfg {
    /// Faixa aceitável da média (`band`).
    pub band: [f64; 2],
    /// Ganho proporcional do corretor (`adaptation_gain`).
    pub adaptation_gain: f64,
}

impl Default for HomeostasisCfg {
    fn default() -> Self {
        // Banda derivada do setpoint histórico (0.8): (0.6, 0.9) —
        // exatamente o `band = [0.6, 0.9]` do default.toml.
        Self {
            band: [
                (ENERGY_TARGET_LEVEL - 0.2).max(0.0),
                (ENERGY_TARGET_LEVEL + 0.1).min(1.0),
            ],
            adaptation_gain: 0.05,
        }
    }
}

/// `[l1.morphogenesis]` — crescimento sob demanda, nunca explosivo.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MorphogenesisCfg {
    /// Energia do cluster acima da qual a divisão é autorizada
    /// (`division_threshold`). NOTA DE SEMÂNTICA: no substrato real a
    /// chave é ENERGIA DO CLUSTER (cluster.rs), não ocupação — vale a
    /// semântica do código, documentada no default.toml.
    pub division_threshold: f64,
    /// Distância média |Δstate| abaixo da qual fusão é permitida
    /// (`fusion_threshold`).
    pub fusion_threshold: f64,
    /// Divisões por step (`max_divisions_per_step`) — teto anti-explosão.
    pub max_divisions_per_step: usize,
    /// Teto populacional do substrato (`max_population`) — A3.
    pub max_population: usize,
}

impl Default for MorphogenesisCfg {
    fn default() -> Self {
        Self {
            division_threshold: DEFAULT_DIVISION_THRESHOLD,
            fusion_threshold: FUSION_STATE_DIST,
            max_divisions_per_step: MAX_DIVISIONS_PER_STEP,
            max_population: MAX_POPULATION,
        }
    }
}

/// Seção `[l1]` consumida pelo substrato real.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct L1Config {
    /// `[l1].population` — população inicial (BOOT). Default do dono:
    /// iniciar com 1.200 clusters (diretriz de escala, checklist 17.6);
    /// o teto operacional (30K) fica em `[l1.morphogenesis].max_population`.
    pub population: usize,
    /// `[l1.energy]`.
    pub energy: EnergyCfg,
    /// `[l1.homeostasis]`.
    pub homeostasis: HomeostasisCfg,
    /// `[l1.morphogenesis]`.
    pub morphogenesis: MorphogenesisCfg,
}

impl Default for L1Config {
    fn default() -> Self {
        Self {
            population: 1_200,
            energy: EnergyCfg::default(),
            homeostasis: HomeostasisCfg::default(),
            morphogenesis: MorphogenesisCfg::default(),
        }
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn defaults_congelados_batem_com_as_consts_historicas() {
        let d = L1Config::default();
        assert_eq!(d.population, 1_200, "boot do dono (diretriz de escala 17.6)");
        assert!((d.energy.initial_level - 0.8).abs() < 1e-9);
        assert!((d.energy.target_level - 0.8).abs() < 1e-9);
        assert!((d.homeostasis.band[0] - 0.6).abs() < 1e-9, "banda do default.toml");
        assert!((d.homeostasis.band[1] - 0.9).abs() < 1e-9, "banda do default.toml");
        assert!((d.homeostasis.adaptation_gain - 0.05).abs() < 1e-9);
        assert!((d.morphogenesis.division_threshold - 0.3).abs() < 1e-9);
        assert!((d.morphogenesis.fusion_threshold - 0.15).abs() < 1e-9);
        assert_eq!(d.morphogenesis.max_divisions_per_step, 4);
        assert_eq!(d.morphogenesis.max_population, 1024);
    }

    #[test]
    fn seção_l1_do_toml_carrega_parcial_e_herda() {
        let text = r#"
            [energy]
            initial_level = 1.0
            target_level = 0.8

            [homeostasis]
            band = [0.6, 0.9]
            adaptation_gain = 0.05

            [morphogenesis]
            division_threshold = 0.85
            fusion_threshold = 0.15
            max_divisions_per_step = 1
            max_population = 1024
        "#;
        let cfg: L1Config = toml::from_str(text).expect("[l1] do default.toml");
        assert!((cfg.energy.initial_level - 1.0).abs() < 1e-9);
        assert!((cfg.morphogenesis.division_threshold - 0.85).abs() < 1e-9);
        assert_eq!(cfg.morphogenesis.max_divisions_per_step, 1);
    }
}
