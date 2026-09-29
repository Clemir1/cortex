//! Modo degradado da L4: reduz orçamento, nunca desativa decisão.

use tracing::{debug, warn};

/// Máximo de entradas do espaço de trabalho em modo degradado.
pub const DEGRADED_MAX_ENTRIES: usize = 32;

/// Estado de degradação da cognição global.
#[derive(Debug, Clone)]
pub struct DegradedMode {
    reason: String,
    since_tick: u64,
    active: bool,
}

impl DegradedMode {
    /// Novo estado: operação normal.
    pub fn new() -> Self {
        Self {
            reason: String::new(),
            since_tick: 0,
            active: false,
        }
    }

    /// Entra em modo degradado com motivo e tique de origem.
    pub fn enter(&mut self, reason: &str, since_tick: u64) {
        self.reason = reason.to_string();
        self.since_tick = since_tick;
        self.active = true;
        warn!(motivo = reason, tique = since_tick, "modo degradado ativado");
    }

    /// Sai do modo degradado.
    pub fn exit(&mut self) {
        self.active = false;
        debug!(motivo = %self.reason, "saída do modo degradado");
        self.reason.clear();
    }

    /// `true` se degradado agora.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Motivo da degradação (vazio se normal).
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// Tique em que a degradação começou.
    pub fn since_tick(&self) -> u64 {
        self.since_tick
    }

    /// Orçamento de entradas vigente: reduzido quando degradado.
    pub fn budget_entries(&self, normal: usize) -> usize {
        if self.active {
            normal.min(DEGRADED_MAX_ENTRIES)
        } else {
            normal
        }
    }
}

impl Default for DegradedMode {
    /// Estado inicial: operação normal.
    fn default() -> Self {
        Self::new()
    }
}
