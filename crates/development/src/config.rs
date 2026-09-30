//! Config de development injetada de `config/default.toml [development.*]`.

use serde::{Deserialize, Serialize};

use crate::stages::StagesCfg;

/// `[development.corticalization]` — alvos DECLARADOS da migração
/// cortical (especialização/integração do tecido). Os alvos são
/// publicados no status; viram critério de parada quando o tecido
/// real medir essas quantidades (nota honesta no README).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CorticalizationCfg {
    pub enabled: bool,
    /// Alvo de especialização dos tecidos.
    pub specialization_target: f32,
    /// Alvo de integração.
    pub integration_target: f32,
}

impl Default for CorticalizationCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            specialization_target: 0.6,
            integration_target: 0.5,
        }
    }
}

/// `[development.embryogenesis]` — gênese inicial.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EmbryogenesisCfg {
    pub enabled: bool,
    pub duration_steps: u64,
}

impl Default for EmbryogenesisCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            duration_steps: 1000,
        }
    }
}

/// `[development]` — política completa do transversal morfológico.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DevelopmentCfg {
    /// Estado da morfogênese (STANDBY por padrão — overview.md §3).
    pub morphogenesis_state: String,
    pub embryogenesis: EmbryogenesisCfg,
    pub corticalization: CorticalizationCfg,
    /// 16.6: bandas de estágio + permanência (maturação pela
    /// RESSONÂNCIA REAL do substrato).
    pub stages: StagesCfg,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_da_casa_parseia_development_1_1() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../config/default.toml"
        ))
        .expect("default.toml presente");
        let raw: toml::Value = src.parse().expect("TOML válido");
        let table = raw.get("development").expect("[development]");
        assert!(table.get("stages").is_some(), "16.6: [development.stages]");
        let stages = table.get("stages").expect("stages");
        assert!(stages.get("dwell_steps").is_some());
        assert!(stages.get("embryo_band").is_some());
        assert!(stages.get("mature_band").is_some());
    }

    #[test]
    fn defaults_congelados() {
        let c = DevelopmentCfg::default();
        assert_eq!(c.morphogenesis_state, "");
        assert_eq!(c.stages.dwell_steps, 20);
        assert_eq!(c.corticalization.specialization_target, 0.6);
    }
}
