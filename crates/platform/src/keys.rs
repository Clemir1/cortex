//! Chaves de contexto canônicas do sistema.
//! Toda camada referencia estes nomes; nada de strings mágicas espalhadas.

/// Resumo do substrato (L1).
pub const L1_SUMMARY: &'static str = "l1.substrate.summary";
/// Visão do tecido (L2).
pub const TISSUE_VIEW: &'static str = "l2.tissue.view";
/// Focos de atenção (L3).
pub const ATTENTION_FOCI: &'static str = "l3.attention.foci";
/// Predição mais recente (L3).
pub const PREDICTION_LATEST: &'static str = "l3.prediction.latest";
/// Decisão pendente (L4).
pub const PENDING_DECISION: &'static str = "l4.decision.pending";
/// Ação executada (L4).
pub const ACTION_EXECUTED: &'static str = "l4.action.executed";
/// Contexto de identidade (L5).
pub const IDENTITY_CONTEXT: &'static str = "l5.identity.context";
/// Intervenções de metacognição (L5).
pub const L5_INTERVENTIONS: &'static str = "l5.interventions";
/// Métricas de cybernetics (transversal).
pub const CYBERNETICS_METRICS: &'static str = "cybernetics.metrics";
/// Envelope de aprendizagem (transversal).
pub const LEARNING_ENVELOPE: &'static str = "learning.envelope";
/// Status de desenvolvimento (transversal).
pub const DEVELOPMENT_STATUS: &'static str = "development.status";
/// Hardlaws de governança (transversal).
pub const HARDLAWS: &'static str = "governance.hardlaws";

/// Todas as chaves de contexto do sistema.
pub const ALL: &[&str] = &[
    L1_SUMMARY,
    TISSUE_VIEW,
    ATTENTION_FOCI,
    PREDICTION_LATEST,
    PENDING_DECISION,
    ACTION_EXECUTED,
    IDENTITY_CONTEXT,
    L5_INTERVENTIONS,
    CYBERNETICS_METRICS,
    LEARNING_ENVELOPE,
    DEVELOPMENT_STATUS,
    HARDLAWS,
];
