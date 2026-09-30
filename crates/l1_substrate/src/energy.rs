//! Energia do substrato — O1 (homeostase) FECHADO e instrumentado.
//!
//! **A1 — correção do defeito de desenho do legado:** o legado tinha 7
//! reservatórios de energia "especiais" que nunca provaram independência
//! funcional (auditoria L1 §5) e nenhum loop fechado de regulação. Aqui:
//! energia escalar por cluster [0,1] + loop O1 canônico com os 5 elementos
//! explícitos:
//!
//! 1. **Variável controlada** — média da energia da população;
//! 2. **Referência** — `setpoint` (default.toml `[l1.energy] target_level`);
//! 3. **Sensor** — média calculada no runner a cada step;
//! 4. **Comparator/Atuador** — `regulate()` ajusta `intake_rate`
//!    proporcional ao erro (ganho lento, sem microgestão);
//! 5. **Feedback** — o erro do próximo step fecha o ciclo.
//!
//! Tudo contado (`corrections`, `out_of_band_steps`, `applied_last`) —
//! homeostase é medida, não declarada (E3 exige saída nova e válida).

use crate::config as cfg;
use tracing::{debug, warn};

/// Controlador O1 de energia da população.
#[derive(Debug, Clone)]
pub struct EnergyHomeostasis {
    /// Setpoint da média populacional (referência).
    pub setpoint: f64,
    /// Fator de intake (atuador) — multiplica o budget por step.
    pub intake_rate: f64,
    /// Banda aceitável: fora dela o corretor conta passo fora-de-banda.
    pub band: (f64, f64),
    /// Ganho proporcional do corretor (lento — anti-microgestão).
    pub gain: f64,
    /// Último erro medido (sensor→comparador).
    pub last_error: f64,
    /// Última atuação aplicada (para telemetria — nunca invisível).
    pub applied_last: f64,
    /// Correções aplicadas desde a gênese (contador de atuação).
    pub corrections: u64,
    /// Steps com média fora da banda (contador de saúde).
    pub out_of_band_steps: u64,
}

impl Default for EnergyHomeostasis {
    fn default() -> Self {
        Self::new(cfg::ENERGY_TARGET_LEVEL)
    }
}

impl EnergyHomeostasis {
    pub fn new(setpoint: f64) -> Self {
        // Banda derivada do setpoint — o padrão histórico (0.8 ⇒ [0.6, 0.9],
        // exatamente o `band` do default.toml) e o ganho clássico 0.05.
        Self::new_with(setpoint, None, 0.05)
    }

    /// Homeostase com banda e ganho injetados de
    /// `config/default.toml [l1.homeostasis]` (band = None ⇒ derivada).
    pub fn new_with(setpoint: f64, band: Option<(f64, f64)>, gain: f64) -> Self {
        let band = band.unwrap_or_else(|| {
            (
                (setpoint - 0.2).max(0.0),
                (setpoint + 0.1).min(cfg::ENERGY_MAX),
            )
        });
        Self {
            setpoint,
            intake_rate: 1.0,
            band,
            gain,
            last_error: 0.0,
            applied_last: 0.0,
            corrections: 0,
            out_of_band_steps: 0,
        }
    }

    /// **O loop O1:** mede o erro e atua sobre o intake.
    /// Retorna o `intake_rate` vigente (o runner aplica).
    pub fn regulate(&mut self, mean_energy: f64) -> f64 {
        self.last_error = self.setpoint - mean_energy;
        // Banda: dentro dela não mexe (histerese de atuação).
        let (lo, hi) = self.band;
        if mean_energy < lo || mean_energy > hi {
            self.out_of_band_steps += 1;
            self.intake_rate =
                (self.intake_rate + self.gain * self.last_error).clamp(cfg::INTAKE_RATE_MIN, cfg::INTAKE_RATE_MAX);
            self.corrections += 1;
            debug!(mean_energy, last_error = self.last_error, intake_rate = self.intake_rate, "orçamento de energia ajustado (fora da banda)");
        }
        self.applied_last = self.intake_rate;
        self.intake_rate
    }
}

/// Distribuição do orçamento de energia por step.
///
/// Intake base por cluster vivo × `intake_rate` do O1; dormentes recebem
/// 25% (metabolismo reduzido); emergência coletiva soma o bônus.
#[derive(Debug, Clone)]
pub struct EnergyBudget {
    /// Intake base por cluster vivo por step.
    pub base_per_cluster: f64,
    pub homeostasis: EnergyHomeostasis,
    /// Distribuído no último step (telemetria).
    pub distributed_last: f64,
}

impl Default for EnergyBudget {
    fn default() -> Self {
        Self::new(cfg::ENERGY_TARGET_LEVEL)
    }
}

impl EnergyBudget {
    pub fn new(setpoint: f64) -> Self {
        Self {
            base_per_cluster: cfg::ENERGY_INTAKE_BASE,
            homeostasis: EnergyHomeostasis::new(setpoint),
            distributed_last: 0.0,
        }
    }

    /// Orçamento com homeostase injetada de
    /// `config/default.toml [l1.homeostasis]` (banda e ganho explícitos).
    pub fn new_with_config(setpoint: f64, band: (f64, f64), gain: f64) -> Self {
        Self {
            base_per_cluster: cfg::ENERGY_INTAKE_BASE,
            homeostasis: EnergyHomeostasis::new_with(setpoint, Some(band), gain),
            distributed_last: 0.0,
        }
    }

    /// Regulação do O1 POR PASSO (17.6/Lei-1): chamada 1× por tick do
    /// runner — os contadores da homeostase (corrections, out_of_
    /// band_steps, applied_last) passam a ter denominador PASSO e não
    /// CHAMADA (antes o runner chamava `regulate` uma vez por CLUSTER:
    /// contadores inflados pela população — taxa sem denominador
    /// honesto). O intake de cada cluster vira função PURA do rate
    /// retornado (bit-idêntico ao estado anterior: `regulate` já era
    /// chamada N vezes com o MESMO mean_energy, produzindo o MESMO
    /// rate a cada chamada).
    pub fn regulate_step(&mut self, mean_energy: f64, emergency_bonus: f64) -> f64 {
        if emergency_bonus > 0.0 {
            warn!(emergency_bonus, mean_energy, "emergência energética ativada");
        }
        self.homeostasis.regulate(mean_energy)
    }

    /// Intake de um cluster neste step (fechamento com o O1 do passo
    /// anterior — o loop real: medir→atuar→medir). Mantido para os
    /// testes unitários do O1 (regulação por CHAMADA); o runner usa
    /// `regulate_step` + intake puro por cluster (17.6).
    pub fn per_cluster_intake(
        &mut self,
        mean_energy: f64,
        dormant: bool,
        emergency_bonus: f64,
    ) -> f64 {
        let rate = self.regulate_step(mean_energy, emergency_bonus);
        let base = if dormant {
            self.base_per_cluster * cfg::DORMANT_INTAKE_FACTOR
        } else {
            self.base_per_cluster
        };
        base * rate + emergency_bonus
    }

    /// Custo metabólico de um cluster (dormente paga reduzido — a economia
    /// da dormência é real, não cosmética).
    pub fn metabolic_cost(profile_cost: f64, dormant: bool) -> f64 {
        if dormant {
            profile_cost * cfg::DORMANT_COST_FACTOR
        } else {
            profile_cost
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o1_fecha_o_loop_subindo_e_descendo() {
        let mut b = EnergyBudget::new(0.8);
        // Média baixa: erro positivo → intake sobe.
        let r1 = b.per_cluster_intake(0.2, false, 0.0);
        assert!(r1 > 1.0 * cfg::ENERGY_INTAKE_BASE - 1e-12);
        assert!(b.homeostasis.intake_rate > 1.0);
        assert_eq!(b.homeostasis.corrections, 1);
        // Média na banda: não atua de novo.
        let before = b.homeostasis.intake_rate;
        b.per_cluster_intake(0.75, false, 0.0);
        assert_eq!(b.homeostasis.intake_rate, before);
        // Média acima da banda: erro negativo → intake desce.
        b.per_cluster_intake(1.0, false, 0.0);
        assert!(b.homeostasis.intake_rate < before);
        assert_eq!(b.homeostasis.out_of_band_steps, 2);
    }

    #[test]
    fn dormente_gasta_menos_e_recebe_menos() {
        assert!(EnergyBudget::metabolic_cost(0.005, true) < 0.005);
        let mut b = EnergyBudget::default();
        let awake = b.per_cluster_intake(0.8, false, 0.0);
        let mut b2 = EnergyBudget::default();
        let dormant = b2.per_cluster_intake(0.8, true, 0.0);
        assert!(dormant < awake, "intake dormente < intake ativo");
    }

    #[test]
    fn intake_tem_limites() {
        let mut b = EnergyBudget::new(0.8);
        for _ in 0..1000 {
            b.per_cluster_intake(0.0, false, 0.0); // pressão máxima
        }
        assert!(b.homeostasis.intake_rate <= cfg::INTAKE_RATE_MAX + 1e-12);
        for _ in 0..2000 {
            b.per_cluster_intake(1.0, false, 0.0); // pressão mínima
        }
        assert!(b.homeostasis.intake_rate >= cfg::INTAKE_RATE_MIN - 1e-12);
    }
}
