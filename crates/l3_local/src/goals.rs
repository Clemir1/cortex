//! Metas locais perseguidas pela cognição local L3.

use triad_foundation as tf;
use tracing::{debug, trace};

/// Meta local: alvo perseguido pela cognição local.
#[derive(Debug, Clone)]
pub struct Goal {
    pub id: tf::id::ConceptId,
    pub label: String,
    pub priority: f32,
    pub step: u64,
}

/// Pilha de metas locais, ordenada por prioridade (maior no topo).
pub struct GoalStack {
    goals: Vec<Goal>,
}

impl GoalStack {
    /// Cria uma pilha de metas vazia.
    pub fn new() -> Self {
        Self { goals: Vec::new() }
    }

    /// Insere uma meta em ordem DESC de prioridade; acima de 16, descarta a de menor prioridade.
    pub fn push(&mut self, label: &str, priority: f32, step: u64) {
        let goal = Goal {
            id: tf::id::ConceptId::new(),
            label: label.to_string(),
            priority: priority.clamp(0.0, 1.0),
            step,
        };
        self.goals.push(goal);
        trace!(tamanho = self.goals.len(), "meta aceita na pilha");
        // Ordena DESC por prioridade com sort estável.
        self.goals.sort_by(|a, b| b.priority.total_cmp(&a.priority));
        // Limite de 16: remove a última (menor prioridade).
        if self.goals.len() > 16 {
            debug!(tamanho = self.goals.len(), "pilha cheia: meta descartada");
            self.goals.pop();
        }
    }

    /// Retorna a meta do topo (maior prioridade), se houver.
    pub fn top(&self) -> Option<&Goal> {
        self.goals.first()
    }

    /// Remove e retorna a meta do topo (maior prioridade), se houver.
    pub fn pop(&mut self) -> Option<Goal> {
        if self.goals.is_empty() {
            return None;
        }
        trace!(tamanho = self.goals.len(), "meta removida da pilha");
        Some(self.goals.remove(0))
    }

    /// Retorna quantas metas estão na pilha.
    pub fn len(&self) -> usize {
        self.goals.len()
    }
}
