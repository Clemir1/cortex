//! Coordenação de adaptação entre módulos.

use std::sync::Mutex;

use triad_contracts as tc;
use triad_foundation as tf;

/// Coordenador de adaptação: acumula pedidos e os converte em ajustes.
/// Propõe ajustes; nunca escreve o estado do módulo alvo.
pub struct AdaptationCoordinator {
    pending: Mutex<Vec<tc::AdaptationRequest>>,
}

impl AdaptationCoordinator {
    /// Cria coordenador sem pedidos pendentes.
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(Vec::new()),
        }
    }

    /// Registra um pedido de adaptação para processamento posterior.
    pub fn submit(&self, req: tc::AdaptationRequest) {
        self.lock().push(req);
    }

    /// Converte os pedidos pendentes em ajustes com ttl = step limite.
    pub fn drain(&self, limit_step: tf::StepId) -> Vec<tc::ParameterAdjustment> {
        let pending = std::mem::take(&mut *self.lock());
        let mut adjustments = Vec::with_capacity(pending.len());
        for req in pending {
            adjustments.push(tc::ParameterAdjustment {
                target_module: req.target,
                parameter: req.parameter.clone(),
                old_value: req.current.clone(),
                new_value: req.proposed.clone(),
                ttl: limit_step,
            });
        }
        adjustments
    }

    /// Trava o mutex; recupera dados mesmo envenenado (nunca entra em pânico).
    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<tc::AdaptationRequest>> {
        self.pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
