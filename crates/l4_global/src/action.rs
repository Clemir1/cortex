//! Execução de ações escolhidas pela decisão global.

use triad_contracts as tc;
use triad_foundation as tf;
use tracing::debug;

/// Executa a opção escolhida e devolve o registro de ação correspondente.
pub fn execute(
    option: &tc::DecisionOption,
    decision_id: tf::DecisionId,
    step: tf::StepId,
) -> crate::ActionRecord {
    debug!(action = ?option.action, decision_id = ?decision_id, "ação executada");
    crate::ActionRecord {
        action: option.action.clone(),
        decision_id,
        step,
        predicted_reward: option.predicted_value.clone(),
        status: crate::ActionStatus::Executed,
    }
}
