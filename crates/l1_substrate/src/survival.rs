//! Survival com histerese — dormancy→wake→repair encadeados.
//!
//! Referência: `core/survival.py` legado (325 linhas) — risco ponderado
//! (energia 0.5 / estresse 0.35 / dano 0.15), acumulação de dano por risco
//! sustentado, repair com ganho e custo, emergência com quorum.
//!
//! **A3 — a correção do defeito provado:** o legado explodiu repair para
//! 1.999 clusters no step 30 e dormancy para 939 (auditoria L1AL5 §2) porque
//! os campos de histerese existiam mas sem limites contados. Aqui:
//! - `MIN_SLEEP_STEPS` — dormência tem duração mínima antes de acordar;
//! - `MIN_AWAKE_STEPS` — vigília tem duração mínima antes de re-dormir;
//! - `WAKE_COOLDOWN_STEPS` — após acordar, não re-dorme no mesmo instante;
//! - `REPAIR_WAVE_GUARD_STEPS` — após sair de repair, guarda contra onda;
//! - todo ciclo é CONTADO (`dormancy_cycles`, `repair_cycles`) — métrica,
//!   não estado estético.

use crate::config as cfg;
use serde::Deserialize;
use tracing::{debug, warn};

/// Decisão do ciclo survival de um cluster — tipada para telemetria
/// (o legado logava strings soltas; aqui a razão é `&'static str`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurvivalDecision {
    StayActive,
    EnterDormancy,
    StayDormant,
    ExitDormancy,
    EnterRepair,
    /// Ainda repairing (dano acima do limiar).
    StayRepairing,
    ExitRepair,
    /// Energia crítica — sobe para o quorum global decidir.
    Emergency,
}

impl SurvivalDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StayActive => "STAY_ACTIVE",
            Self::EnterDormancy => "ENTER_DORMANCY",
            Self::StayDormant => "STAY_DORMANT",
            Self::ExitDormancy => "EXIT_DORMANCY",
            Self::EnterRepair => "ENTER_REPAIR",
            Self::StayRepairing => "STAY_REPAIRING",
            Self::ExitRepair => "EXIT_REPAIR",
            Self::Emergency => "EMERGENCY",
        }
    }
}

/// Config tipada da seção `[l1.survival]` (config/default.toml).
///
/// Piloto do padrão de config centralizada: o loader do triad-platform
/// (`PlatformConfig::get_section::<SurvivalConfig>("l1.survival")`)
/// desserializa esta struct, que é injetada via construtor
/// (`SurvivalState::with_config`). Default = comportamento atual do
/// código (A/A): cada campo espelha a constante equivalente em
/// `crate::config`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct SurvivalConfig {
    /// Energia por cluster que sinaliza emergência ao quorum.
    pub emergency_energy: f64,
    /// Fração de clusters em emergência que ativa o coletivo.
    pub collective_fraction: f64,
    /// Ticks mínimos dormindo antes de poder acordar.
    pub dormancy_min_sleep: u64,
    /// Ticks mínimos acordado antes de poder re-dormir.
    pub dormancy_min_awake: u64,
    /// Ticks de espera após acordar antes de re-dormir.
    pub dormancy_cooldown: u64,
    /// Ticks de guarda após sair de repair (anti-onda).
    pub repair_wave_guard: u64,
}

impl Default for SurvivalConfig {
    fn default() -> Self {
        Self {
            emergency_energy: cfg::SURVIVAL_EMERGENCY_ENERGY_MIN,
            collective_fraction: cfg::SURVIVAL_EMERGENCY_QUORUM_MIN,
            dormancy_min_sleep: cfg::DORMANCY_MIN_SLEEP_STEPS,
            dormancy_min_awake: cfg::DORMANCY_MIN_AWAKE_STEPS,
            dormancy_cooldown: cfg::DORMANCY_WAKE_COOLDOWN_STEPS,
            repair_wave_guard: cfg::REPAIR_WAVE_GUARD_STEPS,
        }
    }
}

/// Estado de survival por cluster (substitui as 48 colunas "mágicas" de
/// metadata da matriz legada por campos nomeados e tipados).
#[derive(Debug, Clone)]
pub struct SurvivalState {
    /// Config injetada da seção `[l1.survival]`.
    pub config: SurvivalConfig,
    pub stress: f64,
    pub damage: f64,
    pub resilience: f64,
    pub survival_risk: f64,
    pub emergency_triggered: bool,
    /// Ganho de repair acumulado (telemetria do legado: `repair_gain`).
    pub repair_gain: f64,
    // --- Histerese A3 (contada, nunca estética) ---
    pub sleep_start_step: Option<u64>,
    pub woke_at_step: Option<u64>,
    pub repair_wave_guard_until: u64,
    pub wake_cooldown_until: u64,
    pub dormancy_cycles: u32,
    pub repair_cycles: u32,
    pub last_decision: SurvivalDecision,
    pub dormant: bool,
    pub repairing: bool,
}

impl Default for SurvivalState {
    fn default() -> Self {
        Self {
            config: SurvivalConfig::default(),
            stress: 0.0,
            damage: 0.0,
            resilience: 0.5,
            survival_risk: 0.0,
            emergency_triggered: false,
            repair_gain: 0.0,
            sleep_start_step: None,
            woke_at_step: None,
            repair_wave_guard_until: 0,
            wake_cooldown_until: 0,
            dormancy_cycles: 0,
            repair_cycles: 0,
            last_decision: SurvivalDecision::StayActive,
            dormant: false,
            repairing: false,
        }
    }
}

impl SurvivalState {
    /// Construtor com config injetada (padrão: config central do organismo).
    pub fn with_config(config: SurvivalConfig) -> Self {
        Self {
            config,
            ..Self::default()
        }
    }

    /// Risco de morte ponderado (legado `calculate_survival_risk`):
    /// energia 0.5 + estresse 0.35 + dano 0.15, com fator de idade.
    pub fn compute_risk(&self, energy: f64, age: f64) -> f64 {
        let age_factor =
            (age / cfg::SURVIVAL_AGE_FACTOR_DIVISOR) * cfg::SURVIVAL_AGE_FACTOR_MULTIPLIER;
        (cfg::SURVIVAL_ENERGY_RISK_WEIGHT * (1.0 - energy)
            + cfg::SURVIVAL_STRESS_RISK_WEIGHT * self.stress
            + cfg::SURVIVAL_DAMAGE_RISK_WEIGHT * self.damage
            + age_factor)
            .clamp(0.0, 1.0)
    }

    /// Um passo do ciclo survival. Aplica histerese e devolve a decisão
    /// tipada. `energy_now`/`energy_predicted` vêm do runner (energia
    /// pós-custo e prevista para o próximo step).
    pub fn step(
        &mut self,
        step: u64,
        energy_now: f64,
        energy_predicted: f64,
        age: f64,
        cluster_energy: &mut f64,
    ) -> SurvivalDecision {
        // 1. Risco e estresse (EMA do risco — memória de pressão).
        self.survival_risk = self.compute_risk(energy_now, age);
        self.stress = 0.9 * self.stress + 0.1 * self.survival_risk;

        // 2. Dano acumula sob risco sustentado; resiliência reflete história.
        if self.survival_risk > cfg::SURVIVAL_REPAIR_RISK_THRESHOLD {
            self.damage = (self.damage + cfg::SURVIVAL_DAMAGE_ACCUMULATION_RATE).clamp(0.0, 1.0);
        }
        let mut resilience = 0.5;
        if self.dormancy_cycles > 0 {
            resilience += cfg::SURVIVAL_RESILIENCE_BONUS_SURVIVAL;
        }
        if self.survival_risk < cfg::SURVIVAL_PREEMPTIVE_RISK_THRESHOLD {
            resilience += cfg::SURVIVAL_RESILIENCE_BONUS;
        }
        resilience -= cfg::SURVIVAL_RESILIENCE_PENALTY_DAMAGE * self.damage;
        self.resilience = resilience.clamp(0.0, 1.0);

        // 3. Emergência por cluster (quorum global fica no runner).
        self.emergency_triggered = energy_now < self.config.emergency_energy;
        if self.emergency_triggered && !self.dormant && !self.repairing {
            self.last_decision = SurvivalDecision::Emergency;
            debug!(passo = step, energia = energy_now, "emergência sinalizada ao quorum");
            return SurvivalDecision::Emergency;
        }

        // 4. Repair em andamento — dano ainda acima do limiar?
        if self.repairing {
            if self.damage <= cfg::SURVIVAL_REPAIR_THRESHOLD {
                self.repairing = false;
                self.repair_wave_guard_until = step + self.config.repair_wave_guard;
                self.last_decision = SurvivalDecision::ExitRepair;
                return SurvivalDecision::ExitRepair;
            }
            let rate = if self.survival_risk < cfg::SURVIVAL_PREEMPTIVE_RISK_THRESHOLD {
                cfg::SURVIVAL_DAMAGE_REPAIR_RATE_HIGH
            } else {
                cfg::SURVIVAL_DAMAGE_REPAIR_RATE_LOW
            };
            let cost = if rate == cfg::SURVIVAL_DAMAGE_REPAIR_RATE_HIGH {
                cfg::SURVIVAL_DAMAGE_ENERGY_COST_HIGH
            } else {
                cfg::SURVIVAL_DAMAGE_ENERGY_COST_LOW
            };
            self.damage = (self.damage - rate).max(0.0);
            self.repair_gain += rate;
            *cluster_energy = (*cluster_energy - cost).max(0.0);
            self.last_decision = SurvivalDecision::StayRepairing;
            return SurvivalDecision::StayRepairing;
        }

        // 5. Dormência em andamento — histerese de saída.
        if self.dormant {
            let slept = step - self.sleep_start_step.unwrap_or(step);
            let recovered = energy_now >= cfg::SURVIVAL_DORMANCY_ENERGY_THRESHOLD;
            if slept >= self.config.dormancy_min_sleep && recovered {
                self.dormant = false;
                self.woke_at_step = Some(step);
                self.wake_cooldown_until = step + self.config.dormancy_cooldown;
                self.last_decision = SurvivalDecision::ExitDormancy;
                return SurvivalDecision::ExitDormancy;
            }
            self.last_decision = SurvivalDecision::StayDormant;
            return SurvivalDecision::StayDormant;
        }

        // 6. Entrar em repair (dano relevante, energia mínima, sem onda).
        let guard_ok = step >= self.repair_wave_guard_until;
        if self.damage > cfg::SURVIVAL_REPAIR_THRESHOLD
            && energy_now >= cfg::SURVIVAL_ENERGY_MIN_REPAIR
            && self.survival_risk < cfg::SURVIVAL_REPAIR_RISK_THRESHOLD
            && guard_ok
        {
            self.repairing = true;
            self.repair_cycles += 1;
            self.last_decision = SurvivalDecision::EnterRepair;
            debug!(passo = step, dano = self.damage, "entrando em repair");
            return SurvivalDecision::EnterRepair;
        }

        // 7. Entrar em dormência — preditivo (risco/estresse futuros) OU
        //    crítico; histerese: vigília mínima + cooldown de re-dormência.
        let awake_long_enough = match self.woke_at_step {
            None => true,
            Some(woke) => step - woke >= self.config.dormancy_min_awake,
        };
        let cooldown_ok = step >= self.wake_cooldown_until;
        let predicted_bad = energy_predicted < cfg::SURVIVAL_DORMANCY_PREDICTED_ENERGY
            && self.stress > cfg::SURVIVAL_DORMANCY_PREDICTED_STRESS;
        let critical = energy_now < self.config.emergency_energy;
        if awake_long_enough && cooldown_ok && (predicted_bad || critical) {
            self.dormant = true;
            self.sleep_start_step = Some(step);
            self.dormancy_cycles += 1;
            self.last_decision = SurvivalDecision::EnterDormancy;
            debug!(passo = step, ciclos = self.dormancy_cycles, "entrando em dormência");
            return SurvivalDecision::EnterDormancy;
        }

        self.last_decision = SurvivalDecision::StayActive;
        SurvivalDecision::StayActive
    }
}

/// Emergência coletiva — quorum da população (legado `should_emergency`):
/// ativa quando a fração de clusters em emergência ≥ `EMERGENCY_QUORUM_MIN`,
/// com ganho de energia e redução de quorum como feedback.
#[derive(Debug, Clone)]
pub struct CollectiveEmergency {
    pub active: bool,
    pub triggered_step: Option<u64>,
    pub quorum_fraction: f64,
    pub emergency_fraction: f64,
    pub steps_active: u64,
}

impl Default for CollectiveEmergency {
    fn default() -> Self {
        Self {
            active: false,
            triggered_step: None,
            quorum_fraction: SurvivalConfig::default().collective_fraction,
            emergency_fraction: 0.0,
            steps_active: 0,
        }
    }
}

impl CollectiveEmergency {
    /// Construtor com config injetada: quorum inicial da seção `[l1.survival]`.
    pub fn with_config(config: &SurvivalConfig) -> Self {
        Self {
            quorum_fraction: config.collective_fraction,
            ..Self::default()
        }
    }

    /// Avalia o quorum com histerese: sustentado `sustain` steps para
    /// ativar/desativar (anti-oscilação, mesma lógica da máquina de crise).
    pub fn update(&mut self, step: u64, emergency_count: usize, population: usize, sustain: u64) {
        self.emergency_fraction = if population == 0 {
            0.0
        } else {
            emergency_count as f64 / population as f64
        };
        if self.active {
            self.steps_active += 1;
            if self.emergency_fraction < self.quorum_fraction * 0.5 && self.steps_active >= sustain
            {
                self.active = false;
                self.triggered_step = None;
                self.steps_active = 0;
            }
        } else if self.emergency_fraction >= self.quorum_fraction {
            match self.triggered_step {
                None => self.triggered_step = Some(step),
                Some(t0) => {
                    if step - t0 >= sustain {
                        self.active = true;
                        warn!(passo = step, fracao = self.emergency_fraction, "emergência coletiva ativada");
                        // Feedback do legado: quorum cai um pouco quando a
                        // emergência se espalha (contenção preemptiva).
                        self.quorum_fraction =
                            (self.quorum_fraction - cfg::SURVIVAL_EMERGENCY_QUORUM_REDUCTION)
                                .max(0.1);
                    }
                }
            }
        } else {
            self.triggered_step = None;
        }
    }

    /// Ganho de energia por cluster quando ativo (legado `ENERGY_GAIN`).
    pub fn energy_bonus(&self) -> f64 {
        if self.active {
            cfg::SURVIVAL_EMERGENCY_ENERGY_GAIN
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risco_ponderado_e_idade() {
        let s = SurvivalState::default();
        let r = s.compute_risk(1.0, 0.0);
        assert!((r - 0.0).abs() < 1e-12 || r < 0.01, "energia cheia, sem risco: {r}");
        let r2 = s.compute_risk(0.0, 0.0);
        assert!(r2 >= cfg::SURVIVAL_ENERGY_RISK_WEIGHT - 1e-9);
        let r_old = s.compute_risk(1.0, 1000.0);
        assert!(r_old >= cfg::SURVIVAL_AGE_FACTOR_MULTIPLIER - 1e-9);
    }

    #[test]
    fn dormencia_tem_duracao_minima() {
        let mut s = SurvivalState::default();
        s.stress = 0.6; // pressão acumulada (o EMA não parte de zero aqui)
        let mut e = 0.4; // entre emergência (0.3) e a banda de dormência
        // Predição ruim + stress alto → dorme (não é emergência: e ≥ 0.3).
        let d = s.step(10, 0.4, 0.4, 5.0, &mut e);
        assert_eq!(d, SurvivalDecision::EnterDormancy);
        assert_eq!(s.dormancy_cycles, 1);
        assert!(!s.emergency_triggered);
        // Antes de MIN_SLEEP: fica dormente mesmo com energia recuperada.
        let mut e2 = 0.9;
        let d = s.step(11, 0.9, 0.9, 5.0, &mut e2);
        assert_eq!(d, SurvivalDecision::StayDormant);
        // Após MIN_SLEEP com energia: acorda.
        let min_sleep = SurvivalConfig::default().dormancy_min_sleep;
        let d = s.step(10 + min_sleep, 0.9, 0.9, 5.0, &mut e2);
        assert_eq!(d, SurvivalDecision::ExitDormancy);
        assert_eq!(s.woke_at_step, Some(10 + min_sleep));
    }

    #[test]
    fn redormencia_respeita_cooldown_e_min_awake() {
        let mut s = SurvivalState::default();
        s.stress = 0.6;
        let mut e = 0.4;
        let woke_step = 10 + SurvivalConfig::default().dormancy_min_sleep;
        assert_eq!(s.step(10, 0.4, 0.4, 0.0, &mut e), SurvivalDecision::EnterDormancy);
        assert_eq!(s.step(woke_step, 0.9, 0.9, 0.0, &mut e), SurvivalDecision::ExitDormancy);
        // No step seguinte (dentro de MIN_AWAKE e cooldown): não re-dorme,
        // mesmo com energia crítica (emergência é sinalizada, não dormência).
        let d = s.step(woke_step + 1, 0.1, 0.0, 0.0, &mut e);
        assert_ne!(d, SurvivalDecision::EnterDormancy);
        assert!(!s.dormant);
    }

    #[test]
    fn repair_reduz_dano_e_guarda_onda() {
        let mut s = SurvivalState::default();
        s.damage = 0.2;
        s.stress = 0.0;
        let mut e = 0.8;
        let d = s.step(100, 0.8, 0.8, 0.0, &mut e);
        assert_eq!(d, SurvivalDecision::EnterRepair);
        let d = s.step(101, 0.8, 0.8, 0.0, &mut e);
        assert_eq!(d, SurvivalDecision::StayRepairing);
        assert!(s.damage < 0.2, "repair reduz dano");
        assert!(s.repair_gain > 0.0, "ganho de repair é mensurável");
        assert!(e < 0.8, "repair custa energia");
        // Dano zerado → sai e instala wave guard.
        s.damage = 0.0;
        let d = s.step(102, 0.8, 0.8, 0.0, &mut e);
        assert_eq!(d, SurvivalDecision::ExitRepair);
        assert_eq!(
            s.repair_wave_guard_until,
            102 + SurvivalConfig::default().repair_wave_guard
        );
    }

    #[test]
    fn emergencia_coletiva_tem_histerese_de_quorum() {
        let mut c = CollectiveEmergency::default();
        c.update(1, 50, 100, 3); // 50% ≥ 30% — arma em t0=1
        assert!(!c.active, "precisa sustentar 3 steps");
        c.update(2, 50, 100, 3);
        assert!(!c.active);
        c.update(3, 50, 100, 3);
        assert!(!c.active);
        c.update(4, 50, 100, 3); // t0+sustain → ativa
        assert!(c.active);
        assert_eq!(c.energy_bonus(), cfg::SURVIVAL_EMERGENCY_ENERGY_GAIN);
        // Some a emergência: precisa sustantar baixa 3 steps para desativar.
        c.update(5, 0, 100, 3);
        c.update(6, 0, 100, 3);
        assert!(c.active);
        c.update(7, 0, 100, 3);
        assert!(!c.active);
    }

    #[test]
    fn config_default_espelha_constantes() {
        let c = SurvivalConfig::default();
        assert_eq!(c.emergency_energy, cfg::SURVIVAL_EMERGENCY_ENERGY_MIN);
        assert_eq!(c.collective_fraction, cfg::SURVIVAL_EMERGENCY_QUORUM_MIN);
        assert_eq!(c.dormancy_min_sleep, cfg::DORMANCY_MIN_SLEEP_STEPS);
        assert_eq!(c.dormancy_min_awake, cfg::DORMANCY_MIN_AWAKE_STEPS);
        assert_eq!(c.dormancy_cooldown, cfg::DORMANCY_WAKE_COOLDOWN_STEPS);
        assert_eq!(c.repair_wave_guard, cfg::REPAIR_WAVE_GUARD_STEPS);
    }

    #[test]
    fn injecao_de_config_muda_histerese() {
        // min_sleep = 0 → acorda no primeiro step com energia recuperada.
        let mut s = SurvivalState::with_config(SurvivalConfig {
            dormancy_min_sleep: 0,
            ..SurvivalConfig::default()
        });
        s.stress = 0.6;
        let mut e = 0.4;
        assert_eq!(s.step(10, 0.4, 0.4, 5.0, &mut e), SurvivalDecision::EnterDormancy);
        let mut e2 = 0.9;
        assert_eq!(s.step(11, 0.9, 0.9, 5.0, &mut e2), SurvivalDecision::ExitDormancy);
    }

    #[test]
    fn quorum_coletivo_via_config_injetada() {
        let c = CollectiveEmergency::with_config(&SurvivalConfig {
            collective_fraction: 0.5,
            ..SurvivalConfig::default()
        });
        assert_eq!(c.quorum_fraction, 0.5);
    }
}
