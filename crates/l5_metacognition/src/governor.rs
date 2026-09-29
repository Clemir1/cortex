//! Governador límbico: regula stress, reserva de energia e aversão.

use triad_contracts as tc;
use triad_foundation as tf;

/// Estado límbico regulado do organismo.
pub struct LimbicGovernor {
    /// Nível de stress corrente (0.0..1.0).
    pub stress: f32,
    /// Reserva de energia disponível (0.0..1.0).
    pub energy_reserve: f32,
    /// Sinal de aversão corrente (0.0..1.0).
    pub aversion: f32,
}

impl LimbicGovernor {
    /// Novo governador em repouso saudável.
    pub fn new() -> Self {
        Self { stress: 0.1, energy_reserve: 1.0, aversion: 0.1 }
    }

    /// Absorve sinais externos com clamp 0.0..1.0 em cada eixo.
    pub fn update(&mut self, external_stress: f32, external_energy: f32, aversion_signal: f32) {
        self.stress = external_stress.clamp(0.0, 1.0);
        self.energy_reserve = external_energy.clamp(0.0, 1.0);
        self.aversion = aversion_signal.clamp(0.0, 1.0);
    }

    /// Projeta o estado interno como modulação límbica.
    pub fn modulation(&self) -> tc::LimbicModulation {
        // construct(0.0) é sempre Some (0.0 dentro da faixa): fallback estrutural.
        tc::LimbicModulation {
            stress: tf::Stress::construct(self.stress.clamp(0.0, 1.0))
                .unwrap_or_else(|| tf::Stress::construct(0.0).unwrap()),
            energy: tf::Energy::construct(self.energy_reserve.clamp(0.0, 1.0))
                .unwrap_or_else(|| tf::Energy::construct(0.0).unwrap()),
            aversion: Some(
                tf::Aversion::construct(self.aversion.clamp(0.0, 1.0))
                    .unwrap_or_else(|| tf::Aversion::construct(0.0).unwrap()),
            ),
        }
    }

    /// Orçamento liberado da reserva: metade é reserva de sobrevivência.
    pub fn reserve_budget(&self, requested: f32) -> f32 {
        requested.min(self.energy_reserve * 0.5)
    }
}
