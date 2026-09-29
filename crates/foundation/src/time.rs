//! Tempo do organismo: relógio lógico (passos) e carimbos unix.

use crate::id::StepId;
use serde::{Deserialize, Serialize};

/// Relógio lógico do organismo: passo atual + tique monótono.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LogicalClock {
    /// Passo lógico atual.
    pub step: StepId,
    /// Contador monótono de avanços.
    pub tick: u64,
}

impl LogicalClock {
    /// Novo relógio: passo inicial, tique 0.
    pub fn new() -> Self {
        Self {
            step: StepId::new(),
            tick: 0,
        }
    }

    /// Avança um tique e entra em novo passo lógico.
    pub fn advance(&mut self) {
        self.tick += 1;
        self.step = StepId::new();
    }
}

/// Carimbo temporal em milissegundos unix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp(pub u64);

impl Timestamp {
    /// Carimbo a partir de milissegundos unix.
    pub const fn from_millis(ms: u64) -> Self {
        Self(ms)
    }

    /// Valor em milissegundos unix.
    pub const fn as_millis(self) -> u64 {
        self.0
    }
}
