//! Envelope de aprendizagem: decisão, resultado e efeito futuro verificado.

use triad_contracts as tc;
use triad_foundation as tf;
use tracing::debug;

/// Envelope de um ciclo decisão→resultado→aprendizagem.
pub struct LearningEnvelope {
    pub decision_id: tf::DecisionId,
    pub action: String,
    pub predicted_reward: f32,
    pub observed_reward: Option<f32>,
    pub verified_future_effect: Option<f32>,
    pub step: tf::StepId,
    pub weight: f32,
}

impl LearningEnvelope {
    /// Cria envelope aberto com a predição, antes de qualquer resultado.
    pub fn new(decision_id: tf::DecisionId, action: &str, predicted: f32, step: tf::StepId) -> Self {
        Self {
            decision_id,
            action: action.to_string(),
            predicted_reward: predicted,
            observed_reward: None,
            verified_future_effect: None,
            step,
            weight: 1.0,
        }
    }

    /// Registra a recompensa observada da decisão.
    pub fn record_result(&mut self, observed_reward: f32) {
        self.observed_reward = Some(observed_reward);
    }

    /// Registra o efeito futuro verificado; é isto que fecha o ciclo.
    pub fn verify_future_effect(&mut self, effect: f32) {
        self.verified_future_effect = Some(effect);
        debug!(effect = effect, "envelope fechado com efeito futuro verificado");
    }

    /// True somente com recompensa observada E efeito futuro verificado.
    pub fn is_closed(&self) -> bool {
        self.observed_reward.is_some() && self.verified_future_effect.is_some()
    }

    /// Erro de predição; `no_data` quando o resultado nunca foi observado.
    pub fn prediction_error(
        &self,
        source: tf::id::ModuleId,
        step: tf::id::StepId,
    ) -> tc::Qualified<f32> {
        match self.observed_reward {
            Some(observed) => tc::Qualified::value(observed - self.predicted_reward, source, step),
            None => {
                debug!("prediction_error sem dado: ausência, não zero");
                tc::Qualified::no_data("resultado não observado", source, step)
            }
        }
    }
}
