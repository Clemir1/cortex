//! O3: governador de recursos — modula atividade por estresse e energia.
use tracing::{debug, warn};

/// Ação de governança decidida pelo O3 sobre a carga de trabalho.
#[derive(Debug, Clone, PartialEq)]
pub enum GoverningAction {
    /// Operação normal.
    Normal,
    /// Redução proporcional de carga.
    Throttle {
        /// Fator multiplicador aplicado à carga.
        factor: f32,
    },
    /// Carga derrubada: energia crítica.
    Shed {
        /// Motivo legível do desligamento da carga.
        reason: String,
    },
}

/// Governador O3: modula atividade conforme estresse e reserva de energia.
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

    /// Decide a ação de governança a partir de estresse e reserva de energia.
    pub fn assess(&self, stress: f32, energy_reserve: f32) -> GoverningAction {
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
        if energy_reserve < self.energy_floor || stress > self.stress_threshold {
            debug!(
                stress = stress,
                reserva = energy_reserve,
                "estresse ou energia baixa: carga reduzida pela metade"
            );
            return GoverningAction::Throttle { factor: 0.5 };
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
