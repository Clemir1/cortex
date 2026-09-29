//! Regeneração: fila de danos com orçamento de cura por passo.

use std::collections::VecDeque;
use triad_foundation as tf;
use tracing::{debug, warn};

/// Capacidade máxima da fila; além disso descarta — nunca cresce sem limite.
pub const QUEUE_CAP: usize = 1024;

/// Natureza do dano reportado em um módulo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DamageKind {
    /// Módulo desconectado do tecido.
    Disconnection,
    /// Módulo sem recursos ou energia.
    Starvation,
    /// Estado interno corrompido.
    Corruption,
    /// Módulo sobrecarregado.
    Overload,
}

impl DamageKind {
    /// Nome curto e estável do dano.
    pub fn as_str(&self) -> &'static str {
        match self {
            DamageKind::Disconnection => "disconnection",
            DamageKind::Starvation => "starvation",
            DamageKind::Corruption => "corruption",
            DamageKind::Overload => "overload",
        }
    }
}

/// Fila de regeneração com descarte contabilizado.
#[derive(Debug)]
pub struct Regeneration {
    /// Danos aguardando cura (limitada a QUEUE_CAP).
    queue: VecDeque<(DamageKind, tf::ModuleId)>,
    /// Danos descartados por fila cheia.
    dropped: u64,
}

impl Regeneration {
    /// Nova fila de regeneração vazia.
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            dropped: 0,
        }
    }

    /// Reporta um dano; fila cheia descarta e conta — o crescimento tem teto.
    pub fn report(&mut self, kind: DamageKind, module: tf::ModuleId) {
        if self.queue.len() >= QUEUE_CAP {
            self.dropped += 1;
            warn!(dano = kind.as_str(), modulo = ?module, "fila de regeneração cheia: dano descartado");
            return;
        }
        debug!(dano = kind.as_str(), modulo = ?module, "dano reportado à regeneração");
        self.queue.push_back((kind, module));
    }

    /// Cura até `max_per_step` danos da fila — regeneração tem orçamento por passo.
    pub fn heal(&mut self, max_per_step: usize) -> usize {
        let mut healed = 0usize;
        while healed < max_per_step {
            if self.queue.pop_front().is_none() {
                break;
            }
            healed += 1;
        }
        debug!(curados = healed, "regeneração concluída");
        healed
    }

    /// Danos atualmente na fila.
    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    /// Danos descartados por capacidade estourada.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
}
