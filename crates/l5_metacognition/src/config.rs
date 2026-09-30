//! Configuração injetável da L5 — espelha `config/default.toml [l5.*]`.
//! Defaults congelados = comportamento A/A documentado.

use serde::{Deserialize, Serialize};

/// `[l5.self_model]` — arco de identidade observado.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SelfModelCfg {
    pub enabled: bool,
    /// Um refresh de identidade a cada N ticks.
    pub update_interval_steps: u64,
    /// Contexto de identidade exigido em toda decisão (contrato).
    pub identity_required: bool,
}

impl Default for SelfModelCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            update_interval_steps: 100,
            identity_required: true,
        }
    }
}

/// `[l5.meta_controller]` — ciclo observe→propose→validate→keep/revert.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MetaControllerCfg {
    pub enabled: bool,
    /// Uma janela de proposta a cada N ticks.
    pub proposal_interval_steps: u64,
    /// Sem `observed_effect` dentro do TTL ⇒ reversão (Lei 6).
    pub intervention_ttl_steps: u32,
    /// Teto de intervenções simultâneas ativas.
    pub max_concurrent_interventions: usize,
    /// Proposta sem razão é rejeitada.
    pub require_reason: bool,
    /// 16.7: passo de alívio do limiar de commit proposto ao L4
    /// (crise muda POLÍTICA, nunca suspende — Lei 4).
    pub commit_threshold_step: f32,
    /// 16.7: piso do limiar — a proposta nunca desce além disso.
    pub commit_threshold_floor: f32,
}

impl Default for MetaControllerCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            proposal_interval_steps: 50,
            intervention_ttl_steps: 50,
            max_concurrent_interventions: 4,
            require_reason: true,
            commit_threshold_step: 0.10,
            commit_threshold_floor: 0.25,
        }
    }
}

/// `[l5.limbic]` — histerese do estado límbico (política da casa:
/// crises mudam POLÍTICA, nunca suspendem — Lei 4).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LimbicCfg {
    /// Entra em ALERT acima deste nível de stress.
    pub stress_enter: f32,
    /// Sai do ALERT abaixo deste nível (histerese: nunca igual ao enter).
    pub stress_exit: f32,
    /// Fração da reserva liberada como orçamento (metade é sobrevivência).
    pub reserve_fraction: f32,
}

impl Default for LimbicCfg {
    fn default() -> Self {
        Self {
            stress_enter: 0.75,
            stress_exit: 0.60,
            reserve_fraction: 0.5,
        }
    }
}

/// `[l5.resource_governor]` — auditoria de orçamento.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ResourceGovernorCfg {
    pub enabled: bool,
    pub audit_interval_steps: u64,
}

impl Default for ResourceGovernorCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            audit_interval_steps: 25,
        }
    }
}

/// `[l5.development_governor]` — despertar por causa declarada.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DevelopmentGovernorCfg {
    pub enabled: bool,
    pub wake_conditions: Vec<String>,
    /// Acordar exige causa declarada; default é aguardar (STANDBY).
    pub morphogenesis_default: String,
}

impl Default for DevelopmentGovernorCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            wake_conditions: vec![
                "damage".into(),
                "capacity_shortage".into(),
                "fragmentation".into(),
                "persistent_overload".into(),
            ],
            morphogenesis_default: "STANDBY".into(),
        }
    }
}

/// `[l5.*]` — política completa da metacognição.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct L5Config {
    pub self_model: SelfModelCfg,
    pub meta_controller: MetaControllerCfg,
    pub limbic: LimbicCfg,
    pub resource_governor: ResourceGovernorCfg,
    pub development_governor: DevelopmentGovernorCfg,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_da_casa_parseia_1_1() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../config/default.toml"
        ))
        .expect("default.toml presente");
        let raw: toml::Value = src.parse().expect("TOML válido");
        let table = raw.get("l5").expect("seção [l5.*] existe");
        // Espelho 1:1 das chaves da casa.
        assert!(table.get("self_model").is_some());
        assert!(table.get("meta_controller").is_some());
        assert!(table.get("limbic").is_some());
        assert!(table.get("resource_governor").is_some());
        assert!(table.get("development_governor").is_some());
    }

    #[test]
    fn defaults_congelados_espelham_o_toml() {
        // Histerese: exit < enter sempre (senão a banda nunca alterna).
        let c = L5Config::default();
        assert!(c.limbic.stress_exit < c.limbic.stress_enter);
        assert!(c.meta_controller.max_concurrent_interventions >= 1);
        assert_eq!(c.development_governor.morphogenesis_default, "STANDBY");
    }
}
