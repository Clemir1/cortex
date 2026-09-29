//! Corticalização: migração cortical com critério de parada minimal-viable.

use tracing::{debug, warn};

/// Critério que encerrou a migração cortical.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StopCriterion {
    /// Energia sustentável alcançada.
    EnergySustain,
    /// Estabilidade entrou em platô.
    StabilityPlateau,
    /// Limite de capacidade atingido.
    CapacityLimit,
}

impl StopCriterion {
    /// Nome curto e estável do critério.
    pub fn as_str(&self) -> &'static str {
        match self {
            StopCriterion::EnergySustain => "energy_sustain",
            StopCriterion::StabilityPlateau => "stability_plateau",
            StopCriterion::CapacityLimit => "capacity_limit",
        }
    }
}

/// Controla a migração cortical: para quando sustentável, não quando perfeita.
#[derive(Debug)]
pub struct Corticalization {
    /// Iterações de migração executadas.
    iterations: u64,
    /// Critério atingido, se a migração já parou.
    criterion: Option<StopCriterion>,
    /// Parada já alcançada (true = parar).
    done: bool,
    /// Última estabilidade observada (base do platô).
    last_stability: f64,
    /// Chamadas seguidas com estabilidade em platô.
    plateau_steps: u64,
}

impl Corticalization {
    /// Nova migração cortical zerada.
    pub fn new() -> Self {
        Self {
            iterations: 0,
            criterion: None,
            done: false,
            last_stability: 0.0,
            plateau_steps: 0,
        }
    }

    /// Uma iteração de migração por chamada; retorna true quando é hora de parar.
    ///
    /// Critérios minimal-viable: energia ≥ 0.8, platô de estabilidade (3 chamadas seguidas) ou capacidade ≥ 0.95.
    pub fn step(&mut self, energy: f64, stability: f64, capacity: f64) -> bool {
        // Parada é final: chamadas após done apenas confirmam, sem reavaliar critério.
        if self.done {
            return true;
        }
        self.iterations += 1;
        if energy >= 0.8 {
            debug!(energy, iterations = self.iterations, "energia sustentável: migração concluída");
            self.criterion = Some(StopCriterion::EnergySustain);
            self.done = true;
        } else if (stability - self.last_stability).abs() < 0.01 {
            self.plateau_steps += 1;
            if self.plateau_steps >= 3 {
                warn!(plateau_steps = self.plateau_steps, stability, "estagnação: estabilidade em platô");
                self.criterion = Some(StopCriterion::StabilityPlateau);
                self.done = true;
            }
        } else {
            self.plateau_steps = 0;
        }
        if !self.done && capacity >= 0.95 {
            warn!(capacity, iterations = self.iterations, "capacidade no limite: migração encerrada");
            self.criterion = Some(StopCriterion::CapacityLimit);
            self.done = true;
        }
        self.last_stability = stability;
        self.done
    }

    /// Iterações de migração executadas até agora.
    pub fn iterations(&self) -> u64 {
        self.iterations
    }

    /// Critério de parada atingido, se houver.
    pub fn criterion(&self) -> Option<StopCriterion> {
        self.criterion
    }
}
