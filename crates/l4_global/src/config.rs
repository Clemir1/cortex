//! Configuração da L4 — seções `[l4.*]` de `config/default.toml`.
//!
//! Cada crate é dono da sua struct `Deserialize` (inversão limpa:
//! a platform não conhece as camadas). Defaults congelados = valores
//! históricos; o app injeta o TOML central via `get_section("l4")`.

use serde::{Deserialize, Serialize};

/// `[l4.workspace]` — competição e broadcast.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceCfg {
    /// Modo corrente (`FULL`/`MINIMAL` — piso da degradação).
    pub mode: String,
    /// Conteúdo simultâneo clássico do workspace (Miller 7±2).
    pub capacity: usize,
    /// Rodadas de competição por broadcast.
    pub competition_rounds: u32,
    /// Um broadcast a cada N ticks.
    pub broadcast_interval_steps: u64,
    /// Perdas tipadas por fronteira (candidate→…→broadcast).
    pub loss_typing_required: bool,
    /// Habituação (GWT/novelty): vencedor repetido perde força
    /// exponencial — `saliência_efetiva = saliência × rate^streak`.
    /// 1.0 = sem habituação; 0.5 = cada vitória halvera a próxima;
    /// 0.0 = uma vitória zera o conteúdo até ele perder e resetar.
    pub habituation_rate: f32,
}

impl Default for WorkspaceCfg {
    fn default() -> Self {
        Self {
            mode: "FULL".into(),
            capacity: 7,
            competition_rounds: 2,
            broadcast_interval_steps: 1,
            loss_typing_required: true,
            habituation_rate: 0.5,
        }
    }
}

/// `[l4.world_model]` — crenças versionadas.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorldModelCfg {
    /// Modo corrente (`FULL`/`DEGRADED` — piso da degradação).
    pub mode: String,
    /// Atualização de crenças a cada N ticks.
    pub update_interval_steps: u64,
    /// Profundidade máxima de rollouts (mecanismo de simulação
    /// aguardando consumidor real — declarado, não teatral).
    pub simulation_max_depth: u32,
}

impl Default for WorldModelCfg {
    fn default() -> Self {
        Self {
            mode: "FULL".into(),
            update_interval_steps: 1,
            simulation_max_depth: 3,
        }
    }
}

/// `[l4.causal]` — hipóteses causa→efeito com denominador.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CausalCfg {
    /// Rastreamento causal ativo.
    pub enabled: bool,
    /// Poda do grafo a cada N ticks.
    pub graph_prune_interval_steps: u64,
    /// Tolerância de |observado − esperado| que confirma um efeito.
    pub confirm_tolerance: f32,
}

impl Default for CausalCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            graph_prune_interval_steps: 1000,
            confirm_tolerance: 0.2,
        }
    }
}

/// `[l4.memory_integration]` — integração de memórias (aguardando
/// consumidor real no `memory.rs`; declarada com razão no README).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryIntegrationCfg {
    pub enabled: bool,
    pub integration_interval_steps: u64,
    pub reconsolidation_enabled: bool,
}

impl Default for MemoryIntegrationCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            // Espelha o TOML calibrado (seção 16.2): cada confirmação
            // reforça imediatamente; a janela limita a RECONSOLIDAÇÃO
            // do mesmo rótulo, não a primeira gravação.
            integration_interval_steps: 1,
            reconsolidation_enabled: true,
        }
    }
}

/// `[l4.decision]` — proposal→competition→selection→commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DecisionCfg {
    pub enabled: bool,
    /// Prazo (ticks) de uma proposta antes de expirar contada.
    pub proposal_timeout_steps: u64,
    /// Confiança mínima para commit; abaixo, defer com prazo.
    pub commit_threshold: f32,
}

impl Default for DecisionCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            proposal_timeout_steps: 3,
            commit_threshold: 0.5,
        }
    }
}

/// `[l4.action]` — execução e observação de outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ActionCfg {
    pub enabled: bool,
    /// Prazo (ticks) para o outcome chegar; sem ele o ciclo NÃO fecha.
    pub outcome_timeout_steps: u64,
}

impl Default for ActionCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            outcome_timeout_steps: 10,
        }
    }
}

/// `[l4.degradation]` — modo degradado OBRIGATÓRIO (contrato §7):
/// crise muda política, nunca desliga sistemas.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DegradationCfg {
    pub policy: String,
    pub min_workspace_mode: String,
    pub min_world_model_mode: String,
    /// `allow_suspension = true` é VIOLAÇÃO DE LEI — o boot rejeita.
    pub allow_suspension: bool,
}

impl Default for DegradationCfg {
    fn default() -> Self {
        Self {
            policy: "MANDATORY".into(),
            min_workspace_mode: "MINIMAL".into(),
            min_world_model_mode: "DEGRADED".into(),
            allow_suspension: false,
        }
    }
}

/// Seção `[l4]` completa — porta de entrada da configuração L4.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct L4Config {
    pub workspace: WorkspaceCfg,
    pub world_model: WorldModelCfg,
    pub causal: CausalCfg,
    pub memory_integration: MemoryIntegrationCfg,
    pub decision: DecisionCfg,
    pub action: ActionCfg,
    pub degradation: DegradationCfg,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_congelados_batem_com_o_toml_central() {
        let d = L4Config::default();
        assert_eq!(d.workspace.capacity, 7, "capacidade clássica do workspace");
        assert_eq!(d.workspace.competition_rounds, 2);
        assert_eq!(d.decision.commit_threshold, 0.5);
        assert_eq!(d.action.outcome_timeout_steps, 10);
        assert!(!d.degradation.allow_suspension, "suspensão de L4 é violação");
    }

    #[test]
    fn secao_l4_carrega_do_toml() {
        let text = r#"
[workspace]
capacity = 9
broadcast_interval_steps = 2

[decision]
commit_threshold = 0.7
"#;
        let cfg: L4Config = toml::from_str(text).expect("[l4] parcial");
        assert_eq!(cfg.workspace.capacity, 9);
        assert_eq!(cfg.workspace.broadcast_interval_steps, 2);
        assert!((cfg.decision.commit_threshold - 0.7).abs() < 1e-6);
        // Chave ausente em seção presente = default do campo.
        assert_eq!(cfg.action.outcome_timeout_steps, 10);
    }
}
