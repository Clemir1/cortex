//! Espaço de trabalho global: competição e broadcast com uma vitória por rodada.
//!
//! REAL (P1 da análise 13-0): a capacidade vem de `[l4.workspace]
//! capacity` (clássico 7±2 do workspace, não mais 128 hard-coded);
//! as entradas são submetidas pelo `L4Module` a partir de dados L3 —
//! este módulo nunca fabrica candidatos.

use crate::config::WorkspaceCfg;
use crate::{StageLosses, WorkspaceStage};
use tracing::trace;

/// Conteúdo competindo no espaço de trabalho global.
#[derive(Debug, Clone)]
pub struct WorkspaceEntry {
    /// Estágio em que o conteúdo entrou.
    pub stage: WorkspaceStage,
    /// Descrição do conteúdo (com proveniência da fonte).
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
    /// Política vigente de `[l4.workspace]`.
    cfg: WorkspaceCfg,
}

impl GlobalWorkspace {
    /// Espaço de trabalho vazio com política default (7 entradas).
    pub fn new() -> Self {
        Self::with_config(WorkspaceCfg::default())
    }

    /// Espaço de trabalho com a política injetada de `[l4.workspace]`.
    pub fn with_config(cfg: WorkspaceCfg) -> Self {
        Self {
            entries: Vec::new(),
            losses: Vec::new(),
            cfg,
        }
    }

    /// Submete conteúdo; acima da capacidade, o de menor saliência é
    /// descartado com perda TIPADA (por estágio, com denominador).
    pub fn submit(&mut self, stage: WorkspaceStage, content: &str, salience: f32) {
        self.entries.push(WorkspaceEntry {
            stage,
            content: content.to_string(),
            salience: salience.clamp(0.0, 1.0),
        });
        let cap = self.cfg.capacity.max(1);
        if self.entries.len() > cap {
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
                capacidade = cap,
                "entrada descartada por excesso no workspace"
            );
            self.losses.push(StageLosses {
                stage,
                dropped: 1,
                total: self.entries.len() + 1,
            });
        }
    }

    /// Broadcast: devolve o vencedor (maior saliência; empate fica
    /// com o mais antigo — determinismo) e limpa a competição.
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
        // Perdas tipadas da fronteira do broadcast (contrato L4 interno:
        // candidate→…→broadcast).
        self.losses.push(StageLosses {
            stage: WorkspaceStage::Broadcast,
            dropped,
            total,
        });
        self.entries.clear();
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

    /// Política vigente (auditoria da configuração).
    pub fn config(&self) -> &WorkspaceCfg {
        &self.cfg
    }
}

impl Default for GlobalWorkspace {
    /// Estado inicial: espaço vazio com política default.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn competicao_deterministica_e_capacidade_da_config() {
        let cfg = WorkspaceCfg {
            capacity: 3,
            ..WorkspaceCfg::default()
        };
        let mut ws = GlobalWorkspace::with_config(cfg);
        ws.submit(WorkspaceStage::Local, "forte", 0.9);
        ws.submit(WorkspaceStage::Local, "fraco", 0.1);
        ws.submit(WorkspaceStage::Local, "medio", 0.5);
        ws.submit(WorkspaceStage::Sensory, "novo-fraco", 0.05); // estoura cap 3
        assert_eq!(ws.len(), 3, "capacidade respeitada");
        let winner = ws.broadcast().expect("vencedor");
        assert_eq!(winner.content, "forte");
        assert!(ws.broadcast().is_none(), "competição limpa após broadcast");
        // Perda tipada registrada com denominador.
        assert!(ws.losses().iter().any(|l| l.dropped == 1 && l.total == 4));
    }

    #[test]
    fn empate_de_saliencia_fica_com_o_mais_antigo() {
        let mut ws = GlobalWorkspace::new();
        ws.submit(WorkspaceStage::Local, "primeiro", 0.7);
        ws.submit(WorkspaceStage::Local, "segundo", 0.7);
        let winner = ws.broadcast().expect("empate resolvido");
        assert_eq!(winner.content, "primeiro");
    }
}
