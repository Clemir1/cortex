//! Espaço de trabalho global: competição e broadcast com uma vitória por rodada.

use crate::{StageLosses, WorkspaceStage};
use tracing::trace;

/// Conteúdo competindo no espaço de trabalho global.
#[derive(Debug, Clone)]
pub struct WorkspaceEntry {
    /// Estágio em que o conteúdo entrou.
    pub stage: WorkspaceStage,
    /// Descrição do conteúdo.
    pub content: String,
    /// Saliência corrente do conteúdo.
    pub salience: f32,
}

/// Espaço de trabalho global: competição e broadcast com uma vitória por rodada.
pub struct GlobalWorkspace {
    /// Entradas correntes na competição.
    entries: Vec<WorkspaceEntry>,
    /// Histórico de perdas por estágio.
    losses: Vec<StageLosses>,
}

impl GlobalWorkspace {
    /// Espaço de trabalho vazio.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            losses: Vec::new(),
        }
    }

    /// Submete conteúdo; acima de 128, o de menor saliência é descartado.
    pub fn submit(&mut self, stage: WorkspaceStage, content: &str, salience: f32) {
        self.entries.push(WorkspaceEntry {
            stage,
            content: content.to_string(),
            salience: salience.clamp(0.0, 1.0),
        });
        if self.entries.len() > 128 {
            // Descarta o de menor saliência; empate fica com o mais antigo.
            let mut weakest = 0usize;
            let mut min = f32::INFINITY;
            for (i, e) in self.entries.iter().enumerate() {
                if e.salience < min {
                    min = e.salience;
                    weakest = i;
                }
            }
            self.entries.remove(weakest);
            trace!(
                estagio = stage.name(),
                "entrada descartada por excesso no workspace"
            );
            self.losses.push(StageLosses {
                stage,
                dropped: 1,
                total: 129,
            });
        }
    }

    /// Broadcast: devolve o vencedor (maior saliência) e o remove da disputa.
    pub fn broadcast(&mut self) -> Option<WorkspaceEntry> {
        if self.entries.is_empty() {
            return None;
        }
        let mut best = 0usize;
        for (i, e) in self.entries.iter().enumerate() {
            if e.salience > self.entries[best].salience {
                best = i;
            }
        }
        let total = self.entries.len();
        let winner = self.entries.remove(best);
        let dropped = self.entries.len();
        self.losses.push(StageLosses {
            stage: WorkspaceStage::Broadcast,
            dropped,
            total,
        });
        Some(winner)
    }

    /// Entradas correntes na competição.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Histórico de perdas por estágio.
    pub fn losses(&self) -> &[StageLosses] {
        &self.losses
    }
}

impl Default for GlobalWorkspace {
    /// Estado inicial: espaço vazio.
    fn default() -> Self {
        Self::new()
    }
}
