//! Espaço de trabalho global: competição e broadcast com uma vitória por rodada.
//!
//! REAL (P1 da análise 13-0 + correção da seção 15): a capacidade vem
//! de `[l4.workspace] capacity` (clássico 7±2 do workspace); a
//! **habituação** (GWT/novelty — Baars: o workspace valoriza o NOVO)
//! faz o vencedor repetido perder força exponencial, circulando o
//! spotlight (Dehaene: seleção por amplificação, não por mérito
//! estático); as entradas vêm do `L4Module` com dados L3 — este
//! módulo nunca fabrica candidatos.

use std::collections::HashMap;

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
    /// Vitórias consecutivas por conteúdo (habituação — GWT/novelty).
    win_streaks: HashMap<String, u32>,
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
            win_streaks: HashMap::new(),
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

    /// Broadcast: devolve o vencedor pela SALIÊNCIA EFETIVA (com
    /// habituação — vencedor repetido perde força exponencial;
    /// empate fica com o mais antigo: determinismo) e limpa a
    /// competição. O streak do vencedor sobe; o dos demais zera
    /// (novelty: quem não venceu volta com força total).
    pub fn broadcast(&mut self) -> Option<WorkspaceEntry> {
        if self.entries.is_empty() {
            return None;
        }
        let rate = self.cfg.habituation_rate.clamp(0.0, 1.0);
        let effective = |content: &str, salience: f32| -> f32 {
            let streak = self.win_streaks.get(content).copied().unwrap_or(0);
            salience * rate.powi(streak as i32)
        };
        let mut best = 0usize;
        let mut best_eff = f32::NEG_INFINITY;
        for (i, e) in self.entries.iter().enumerate() {
            let eff = effective(&e.content, e.salience);
            if eff > best_eff {
                best_eff = eff;
                best = i;
            }
        }
        let total = self.entries.len();
        let winner = self.entries.remove(best);
        let dropped = self.entries.len();
        // Habituação: o vencedor acumula streak; os demais resetam.
        for e in &self.entries {
            self.win_streaks.remove(&e.content);
        }
        *self.win_streaks.entry(winner.content.clone()).or_insert(0) += 1;
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

    /// Sequência de vitórias corrente de um conteúdo (auditoria da
    /// habituação — GWT/novelty).
    pub fn win_streak(&self, content: &str) -> u32 {
        self.win_streaks.get(content).copied().unwrap_or(0)
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

    #[test]
    fn habituacao_circula_o_spotlight_entre_conteudos() {
        // GWT/novelty: sem habituação, "forte" venceria para sempre.
        let mut ws = GlobalWorkspace::new();
        ws.submit(WorkspaceStage::Local, "forte", 0.9);
        ws.submit(WorkspaceStage::Local, "medio", 0.5);
        assert_eq!(ws.broadcast().expect("1º").content, "forte");
        // Reaparecem com as mesmas saliências: forte agora 0.9×0.5=0.45.
        ws.submit(WorkspaceStage::Local, "forte", 0.9);
        ws.submit(WorkspaceStage::Local, "medio", 0.5);
        assert_eq!(ws.broadcast().expect("2º").content, "medio", "streak halvera o forte");
        assert_eq!(ws.win_streak("medio"), 1);
        // Terceira rodada: medio 0.5×0.5=0.25 vs forte resetado 0.9.
        ws.submit(WorkspaceStage::Local, "forte", 0.9);
        ws.submit(WorkspaceStage::Local, "medio", 0.5);
        let winner = ws.broadcast().expect("3º");
        assert_eq!(winner.content, "forte", "perdedor reseta o streak");
        assert_eq!(ws.win_streak("medio"), 0, "medio perdeu ⇒ novelty zerada");
    }

    #[test]
    fn habituacao_rate_um_desliga_a_politica() {
        // rate = 1.0: 1^streak = 1 ⇒ sem habituação, o bruto vence sempre.
        let cfg = WorkspaceCfg {
            habituation_rate: 1.0,
            ..WorkspaceCfg::default()
        };
        let mut ws = GlobalWorkspace::with_config(cfg);
        for _ in 0..3 {
            ws.submit(WorkspaceStage::Local, "forte", 0.9);
            ws.submit(WorkspaceStage::Local, "medio", 0.5);
            assert_eq!(ws.broadcast().expect("sem habituação").content, "forte");
        }
    }

    #[test]
    fn habituacao_rate_zero_zera_o_vencedor_anterior() {
        // rate = 0.0: uma vitória zera o conteúdo até ele perder.
        let cfg = WorkspaceCfg {
            habituation_rate: 0.0,
            ..WorkspaceCfg::default()
        };
        let mut ws = GlobalWorkspace::with_config(cfg);
        ws.submit(WorkspaceStage::Local, "forte", 0.9);
        ws.submit(WorkspaceStage::Local, "medio", 0.5);
        assert_eq!(ws.broadcast().expect("1º").content, "forte");
        ws.submit(WorkspaceStage::Local, "forte", 0.9);
        ws.submit(WorkspaceStage::Local, "medio", 0.5);
        assert_eq!(
            ws.broadcast().expect("2º").content,
            "medio",
            "forte com streak zera (0.9×0)"
        );
        ws.submit(WorkspaceStage::Local, "forte", 0.9);
        ws.submit(WorkspaceStage::Local, "medio", 0.5);
        assert_eq!(ws.broadcast().expect("3º").content, "forte", "streaks resetam");
    }
}
