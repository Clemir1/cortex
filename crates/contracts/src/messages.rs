//! Mensagens transversais: propostas, metas, restrições, adaptações e ajustes.

use triad_foundation as tf;

use serde::{Deserialize, Serialize};

use crate::event_envelope::Priority;

/// Proposta de política endereçada ao dono do estado.
///
/// L5→L4 e transversais→dono do estado: propor, nunca escrever.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyProposal {
    pub proposal_id: tf::id::EventId,
    pub governance_id: Option<tf::id::EventId>,
    pub target_layer: u8,
    pub rationale: String,
}

/// Restrição global (hard ou soft) imposta sobre todo o sistema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalConstraint {
    pub name: String,
    pub hard: bool,
    pub value: String,
}

/// Meta local de um módulo com prioridade de execução.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalGoal {
    pub goal_id: tf::id::EventId,
    pub description: String,
    pub priority: Priority,
}

/// Pedido de adaptação de parâmetro de um módulo a outro.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdaptationRequest {
    pub requester: tf::id::ModuleId,
    pub target: tf::id::ModuleId,
    pub parameter: String,
    pub current: String,
    pub proposed: String,
}

/// Fatia de energia alocada a um módulo e o motivo da alocação.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceAllocation {
    pub module_id: tf::id::ModuleId,
    pub energy_share: f32,
    pub reason: String,
}

/// Ajuste de parâmetro com validade limitada (TTL).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterAdjustment {
    pub target_module: tf::id::ModuleId,
    pub parameter: String,
    pub old_value: String,
    pub new_value: String,
    pub ttl: tf::id::StepId,
}
