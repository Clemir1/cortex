//! Roteamento de feedback de decisões.

use std::sync::Mutex;

use triad_foundation as tf;

/// Sinal de feedback de uma decisão: resultado e entrega.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FeedbackSignal {
    pub decision_id: tf::DecisionId,
    pub success: bool,
    pub delivered: bool,
}

/// Roteador de feedback: acumula sinais entregues até a drenagem.
pub struct FeedbackRouter {
    outbox: Mutex<Vec<FeedbackSignal>>,
}

impl FeedbackRouter {
    /// Cria roteador com outbox vazia.
    pub fn new() -> Self {
        Self {
            outbox: Mutex::new(Vec::new()),
        }
    }

    /// Roteia o feedback de uma decisão e devolve o sinal entregue.
    pub fn route(&self, decision_id: tf::DecisionId, success: bool) -> FeedbackSignal {
        let signal = FeedbackSignal {
            decision_id,
            success,
            delivered: true,
        };
        self.lock().push(signal.clone());
        signal
    }

    /// Drena todos os sinais acumulados na outbox.
    pub fn drain(&self) -> Vec<FeedbackSignal> {
        std::mem::take(&mut *self.lock())
    }

    /// Trava o mutex; recupera dados mesmo envenenado (nunca entra em pânico).
    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<FeedbackSignal>> {
        self.outbox
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
