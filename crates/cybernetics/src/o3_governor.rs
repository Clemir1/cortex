//! O3: governador de recursos — modula atividade por estresse e energia.
use tracing::{debug, warn};

/// Ação de governança decidida pelo O3 sobre a carga de trabalho.
#[derive(Debug, Clone, PartialEq)]
pub enum GoverningAction {
    /// Operação normal.
    Normal,
    /// Redução proporcional de carga COM razão canônica (trilha
    /// de auditoria da decisão — herança policy_arbitrator.py:406).
    Throttle {
        /// Fator multiplicador aplicado à carga [0.25, 1.0].
        factor: f32,
        /// Razão legível da redução.
        reason: String,
    },
    /// Carga derrubada: energia crítica.
    Shed {
        /// Motivo legível do desligamento da carga.
        reason: String,
    },
}

/// Governador O3: modula atividade conforme estresse e energia.
/// `stress` é CONTÍNUO [0..1] (19.8-a: a urgência do O1 deixa de
/// ser binária — herança homeostasis_network.py:71, urgência por
/// |erro|; o O1 entrega |corrective| clampado com razão).
#[derive(Debug, Clone)]
pub struct O3Governor {
    /// Limiar de estresse aceitável (0.0..=1.0).
    stress_threshold: f32,
    /// Piso de reserva de energia (0.0..=1.0).
    energy_floor: f32,
}

impl O3Governor {
    /// Cria o governador; `None` se limiar ou piso saírem de 0.0..=1.0.
    pub fn new(stress_threshold: f32, energy_floor: f32) -> Option<Self> {
        if !(0.0..=1.0).contains(&stress_threshold) || !(0.0..=1.0).contains(&energy_floor) {
            return None;
        }
        Some(Self {
            stress_threshold,
            energy_floor,
        })
    }

    /// Decide a ação de governança a partir de estresse contínuo e
    /// reserva de energia. Fator PROPORCIONAL ao estresse (herança
    /// CONTEXT_ADJUST por regime, policy_arbitrator.py:82-87: crise
    /// muda POLÍTICA de precedência, nunca desativa sistemas).
    pub fn assess(&self, stress: f32, energy_reserve: f32) -> GoverningAction {
        let stress = stress.clamp(0.0, 1.0);
        if energy_reserve < self.energy_floor / 2.0 {
            warn!(
                floor = self.energy_floor,
                reserva = energy_reserve,
                "energia critica: carga derrubada"
            );
            return GoverningAction::Shed {
                reason: "energia critica".to_string(),
            };
        }
        if energy_reserve < self.energy_floor {
            debug!(
                stress,
                reserva = energy_reserve,
                "reserva abaixo do piso: carga reduzida pela metade"
            );
            return GoverningAction::Throttle {
                factor: 0.5,
                reason: "reserva de energia abaixo do piso".to_string(),
            };
        }
        if stress > self.stress_threshold {
            // Fator contínuo: threshold ⇒ ~0.5; stress máximo ⇒ 0.25.
            let factor = (1.0 - 0.75 * stress).clamp(0.25, 1.0);
            debug!(
                stress,
                fator = factor,
                "estresse acima do limiar: carga reduzida proporcionalmente"
            );
            return GoverningAction::Throttle {
                factor,
                reason: "estresse acima do limiar".to_string(),
            };
        }
        GoverningAction::Normal
    }

    /// Concede orçamento de recursos sem nunca exceder o disponível.
    pub fn budget(&self, requested: u64, available: u64, stress: f32) -> u64 {
        let granted = requested.min(available);
        if stress > self.stress_threshold {
            let reduced = granted / 2;
            debug!(reduzido_para = reduced, "orçamento reduzido por estresse");
            return reduced;
        }
        granted
    }
}
