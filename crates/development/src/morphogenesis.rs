//! Morfogênese: mudança de forma é cara — standby é o estado padrão.

use tracing::debug;

/// Padrão: nasce em standby (dorme até existir razão tipada para acordar).
pub const DEFAULT_STANDBY: bool = true;

/// Razão tipada para despertar a morfogênese.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WakeReason {
    /// Dano estrutural detectado.
    Damage,
    /// Capacidade insuficiente para a carga.
    CapacityShortage,
    /// Tecido fragmentado demais.
    Fragmentation,
    /// Sobrecarga persistente.
    PersistentOverload,
}

impl WakeReason {
    /// Nome curto e estável da razão.
    pub fn as_str(&self) -> &'static str {
        match self {
            WakeReason::Damage => "damage",
            WakeReason::CapacityShortage => "capacity_shortage",
            WakeReason::Fragmentation => "fragmentation",
            WakeReason::PersistentOverload => "persistent_overload",
        }
    }
}

/// Morfogênese dormente por padrão; só aceita planos quando acordada.
#[derive(Debug)]
pub struct Morphogenesis {
    /// Em standby quando true.
    standby: bool,
    /// Razão tipada do último despertar.
    reason: Option<WakeReason>,
    /// Planos morfogenéticos aceitos e pendentes.
    pending: Vec<serde_json::Value>,
    /// Planos descartados por chegarem em standby.
    dropped: u64,
}

impl Morphogenesis {
    /// Nova morfogênese em standby (o padrão é dormir).
    pub fn new() -> Self {
        Self {
            standby: DEFAULT_STANDBY,
            reason: None,
            pending: Vec::new(),
            dropped: 0,
        }
    }

    /// Acorda sempre com razão tipada — LEI: nunca acorda sem motivo.
    pub fn wake(&mut self, reason: WakeReason) {
        self.standby = false;
        self.reason = Some(reason);
        debug!(razao = reason.as_str(), "morfogênese acordada");
    }

    /// Volta ao standby; dormir é o estado natural (mudança cara fica reservada).
    pub fn sleep(&mut self) {
        self.standby = true;
        self.reason = None;
        debug!("morfogênese voltou ao standby");
    }

    /// Aceita plano apenas acordada; em standby descarta e conta o descarte.
    pub fn enqueue(&mut self, plan: serde_json::Value) {
        if self.standby {
            self.dropped += 1;
            return;
        }
        self.pending.push(plan);
        debug!(pendentes = self.pending.len(), "plano morfogenético enfileirado");
    }

    /// Estado atual: (standby, razão do despertar).
    pub fn status(&self) -> (bool, Option<WakeReason>) {
        (self.standby, self.reason)
    }

    /// Planos morfogenéticos pendentes.
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// Planos descartados por chegarem em standby.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
}
