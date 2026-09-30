//! Config de learning injetada de `config/default.toml [learning]`.
//! Defaults congelados = comportamento A/A documentado. LEI 4:
//! `standby_allowed` é SEMPRE false (o campo DOCUMENTA a lei; o
//! módulo é sempre ativo).

use serde::{Deserialize, Serialize};

/// `[learning]` — política completa do transversal de aprendizagem.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LearningCfg {
    pub enabled: bool,
    /// Lei 4 documentada: learning NUNCA entra em standby.
    pub standby_allowed: bool,
    /// Peso do erro de predição no crédito atribuído.
    pub prediction_error_weight: f32,
    /// Horizonte de crédito em passos.
    pub credit_horizon_steps: u64,
    /// Horizontes de validação futura (t+1 hoje; t+5 aguarda
    /// rastreamento multi-passo — chave nunca apagada).
    pub validation_horizons: Vec<u64>,
    /// Lei 5: E5 só após consequência observada.
    pub e5_requires_observed_consequence: bool,
    /// Sem verified_future_effect o ciclo NÃO conta como fechado.
    pub require_verified_future_effect: bool,
    /// Decaimento exponencial de traço por step.
    pub forgetting_rate: f32,
    /// Limite de atualização por step (estabilidade: nenhum evento
    /// isolado move mais que isso na avaliação de um módulo).
    pub max_update_rate: f32,
}

impl Default for LearningCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            // Lei 4: nunca standby — valor congela a lei.
            standby_allowed: false,
            prediction_error_weight: 1.0,
            credit_horizon_steps: 5,
            validation_horizons: vec![1, 5],
            e5_requires_observed_consequence: true,
            require_verified_future_effect: true,
            forgetting_rate: 0.0005,
            max_update_rate: 0.1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_da_casa_parseia_learning_1_1() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../config/default.toml"
        ))
        .expect("default.toml presente");
        let raw: toml::Value = src.parse().expect("TOML válido");
        let table = raw.get("learning").expect("[learning]");
        assert_eq!(table.get("standby_allowed"), Some(&toml::Value::Boolean(false)));
        assert_eq!(
            table.get("require_verified_future_effect"),
            Some(&toml::Value::Boolean(true))
        );
        assert!(table.get("max_update_rate").is_some());
        assert!(table.get("validation_horizons").is_some());
    }

    #[test]
    fn defaults_congelados_e_leis() {
        let c = LearningCfg::default();
        assert!(!c.standby_allowed, "Lei 4: learning nunca em standby");
        assert!(c.require_verified_future_effect, "Lei 5");
        assert!(c.e5_requires_observed_consequence);
        assert_eq!(c.validation_horizons, vec![1, 5]);
        assert!(c.forgetting_rate > 0.0 && c.forgetting_rate < 0.01);
        assert!(c.max_update_rate > 0.0 && c.max_update_rate <= 1.0);
    }
}
