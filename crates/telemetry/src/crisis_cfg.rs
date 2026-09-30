//! Structs de `[crisis]` (16.9) — máquina de estados transversal
//! (overview.md §7): NORMAL→PRESSURE→CRISIS→STABILIZATION→
//! RECOVERY→REINTEGRATION. Crise muda POLÍTICAS, NUNCA desliga
//! sistemas (Lei 4) — as validações CONGELAM a lei na estrutura.
//! O CONSUMIDOR (CrisisMachine executável) aguarda o mecanismo real;
//! as structs e a VALIDAÇÃO são a entrega desta caixa (a chave nunca
//! é apagada).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Estados canônicos da máquina de crise (overview.md §7).
pub const CRISIS_STATES: [&str; 6] = [
    "NORMAL",
    "PRESSURE",
    "CRISIS",
    "STABILIZATION",
    "RECOVERY",
    "REINTEGRATION",
];

/// Erros tipados de validação da config de crise — nunca booleano mudo.
#[derive(Debug, Clone, PartialEq)]
pub enum CrisisCfgError {
    /// Estado inicial fora da máquina declarada.
    UnknownInitialState { found: String },
    /// Estados fora dos canônicos da casa.
    UnknownState { found: String },
    /// Transições fora da sequência NORMAL→…→REINTEGRATION.
    IllegalStateOrder,
    /// Threshold fora de 0..=1.
    ThresholdOutOfRange { field: &'static str, value: f32 },
    /// Histerese quebrada: recuperação exige nível MAIOR que a
    /// entrada (anti-oscilação — overview.md §7).
    HysteresisViolated { lower: f32, upper: f32 },
    /// LEI 4 (inegociável): preservado em todos os estados não pode
    /// ser desligado (l1_homeostasis, learning, decisão, outcome).
    PreserveViolated { key: &'static str },
    /// Efeitos ausentes para algum estado declarado.
    MissingEffects { state: String },
    /// Fator de política negativo não existe.
    IllegalFactor { field: String, value: f32 },
}

/// `[crisis.machine]` — estados, inicial e histerese.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CrisisMachineCfg {
    pub states: Vec<String>,
    pub initial_state: String,
    /// Passos sustentados para confirmar transição (histerese).
    pub sustain_steps: u64,
}

impl Default for CrisisMachineCfg {
    fn default() -> Self {
        Self {
            states: CRISIS_STATES.iter().map(|s| s.to_string()).collect(),
            initial_state: "NORMAL".to_string(),
            sustain_steps: 50,
        }
    }
}

/// `[crisis.thresholds]` — frações de energia das transições.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CrisisThresholdsCfg {
    pub pressure_enter: f32,
    pub crisis_enter: f32,
    pub stabilization_enter: f32,
    pub recovery_enter: f32,
    pub reintegration_enter: f32,
}

impl Default for CrisisThresholdsCfg {
    fn default() -> Self {
        Self {
            pressure_enter: 0.40,
            crisis_enter: 0.20,
            stabilization_enter: 0.35,
            recovery_enter: 0.55,
            reintegration_enter: 0.75,
        }
    }
}

/// `[crisis.effects.<ESTADO>]` — fatores de POLÍTICA por estado.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CrisisEffectsCfg {
    pub morphogenesis_factor: f32,
    pub division_factor: f32,
    pub telemetry_heavy: bool,
    pub persistence_heavy: bool,
    pub o5_enabled: bool,
    pub recharge_factor: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attention_gain: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lua_window_ms: Option<u64>,
}

impl Default for CrisisEffectsCfg {
    fn default() -> Self {
        // NORMAL: todos nominais (fatores 1.0, tudo ON).
        Self {
            morphogenesis_factor: 1.0,
            division_factor: 1.0,
            telemetry_heavy: true,
            persistence_heavy: true,
            o5_enabled: true,
            recharge_factor: 1.0,
            attention_gain: Some(1.0),
            lua_window_ms: Some(5),
        }
    }
}

/// `[crisis]` — política completa da máquina de crise.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CrisisCfg {
    pub machine: CrisisMachineCfg,
    pub thresholds: CrisisThresholdsCfg,
    /// Preservado em TODOS os estados (lei de degradação).
    pub preserve: BTreeMap<String, bool>,
    /// Fatores de política por estado.
    pub effects: BTreeMap<String, CrisisEffectsCfg>,
}

impl CrisisCfg {
    /// VALIDAÇÃO da casa — Lei 4 e coerências estruturais. Erros
    /// tipados; nada é "adaptado" silenciosamente.
    pub fn validate(&self) -> Result<(), CrisisCfgError> {
        let m = &self.machine;
        // Estados: exatamente os canônicos, na ordem da casa.
        if m.states.len() != CRISIS_STATES.len()
            || m.states
                .iter()
                .zip(CRISIS_STATES.iter())
                .any(|(a, b)| a != b)
        {
            return Err(CrisisCfgError::IllegalStateOrder);
        }
        for s in &m.states {
            if !CRISIS_STATES.contains(&s.as_str()) {
                return Err(CrisisCfgError::UnknownState { found: s.clone() });
            }
        }
        if !m.states.contains(&m.initial_state) {
            return Err(CrisisCfgError::UnknownInitialState {
                found: m.initial_state.clone(),
            });
        }
        let t = &self.thresholds;
        for (field, v) in [
            ("pressure_enter", t.pressure_enter),
            ("crisis_enter", t.crisis_enter),
            ("stabilization_enter", t.stabilization_enter),
            ("recovery_enter", t.recovery_enter),
            ("reintegration_enter", t.reintegration_enter),
        ] {
            if !(0.0..=1.0).contains(&v) {
                return Err(CrisisCfgError::ThresholdOutOfRange { field, value: v });
            }
        }
        // Histerese: sair da crise exige MAIS energia que entrar.
        if t.recovery_enter <= t.crisis_enter {
            return Err(CrisisCfgError::HysteresisViolated {
                lower: t.recovery_enter,
                upper: t.crisis_enter,
            });
        }
        if t.stabilization_enter <= t.crisis_enter {
            return Err(CrisisCfgError::HysteresisViolated {
                lower: t.stabilization_enter,
                upper: t.crisis_enter,
            });
        }
        if t.reintegration_enter <= t.stabilization_enter {
            return Err(CrisisCfgError::HysteresisViolated {
                lower: t.reintegration_enter,
                upper: t.stabilization_enter,
            });
        }
        // LEI 4 (inegociável): preserve dessas chaves NUNCA desliga.
        for key in ["l1_homeostasis", "learning", "l4_decision", "l4_outcome"] {
            if self.preserve.get(key) != Some(&true) {
                return Err(CrisisCfgError::PreserveViolated { key });
            }
        }
        // Efeitos: um bloco por estado EXCETO NORMAL (os efeitos
        // nominais são o default da estrutura — a casa não repete);
        // subseções do TOML em minúsculas, estados canônicos em
        // MAIÚSCULOS: lookup normalizado. Fatores não-negativos.
        for state in &m.states {
            if state == "NORMAL" {
                continue;
            }
            let Some(e) = self.effects.get(&state.to_lowercase()) else {
                return Err(CrisisCfgError::MissingEffects {
                    state: state.clone(),
                });
            };
            for (field, v) in [
                ("morphogenesis_factor", e.morphogenesis_factor),
                ("division_factor", e.division_factor),
                ("recharge_factor", e.recharge_factor),
            ] {
                if v < 0.0 {
                    return Err(CrisisCfgError::IllegalFactor {
                        field: field.to_string(),
                        value: v,
                    });
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn casa() -> CrisisCfg {
        // default == casa (clone do default.toml).
        CrisisCfg {
            machine: CrisisMachineCfg::default(),
            thresholds: CrisisThresholdsCfg::default(),
            preserve: [
                "l1_homeostasis",
                "l2_tissues",
                "l3_memory_semantic",
                "l4_workspace_minimal",
                "l4_world_model_degraded",
                "l4_decision",
                "l4_outcome",
                "learning",
            ]
            .iter()
            .map(|k| (k.to_string(), true))
            .collect(),
            effects: CRISIS_STATES
                .iter()
                .map(|s| (s.to_string(), CrisisEffectsCfg::default()))
                .collect(),
        }
    }

    #[test]
    fn toml_da_casa_parseia_crisis_1_1_e_valida() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../config/default.toml"
        ))
        .expect("default.toml presente");
        let raw: toml::Value = src.parse().expect("TOML válido");
        let crisis = raw.get("crisis").expect("[crisis]");
        let cfg: CrisisCfg = crisis.clone().try_into().expect("parse 1:1");
        assert_eq!(cfg.machine.initial_state, "NORMAL");
        assert_eq!(cfg.machine.sustain_steps, 50);
        assert_eq!(cfg.thresholds.crisis_enter, 0.20);
        assert_eq!(cfg.preserve.get("learning"), Some(&true), "Lei 4");
        // NORMAL é o default nominal (sem bloco próprio): 5 blocos.
        assert_eq!(cfg.effects.len(), 5, "um bloco por estado não-normal");
        cfg.validate().expect("config da casa é válida");
    }

    #[test]
    fn lei_4_preserve_de_learning_e_inegociavel() {
        let mut c = casa();
        c.preserve.insert("learning".to_string(), false);
        assert_eq!(
            c.validate(),
            Err(CrisisCfgError::PreserveViolated { key: "learning" }),
            "crise muda política, NUNCA desliga aprendizagem"
        );
        c.preserve.insert("learning".to_string(), true);
        c.preserve.insert("l1_homeostasis".to_string(), false);
        assert_eq!(
            c.validate(),
            Err(CrisisCfgError::PreserveViolated {
                key: "l1_homeostasis"
            })
        );
    }

    #[test]
    fn histerese_e_thresholds_validados_tipados() {
        let mut c = casa();
        c.thresholds.recovery_enter = 0.10;
        assert_eq!(
            c.validate(),
            Err(CrisisCfgError::HysteresisViolated {
                lower: 0.10,
                upper: 0.20
            })
        );
        c.thresholds.recovery_enter = 1.5;
        assert_eq!(
            c.validate(),
            Err(CrisisCfgError::ThresholdOutOfRange {
                field: "recovery_enter",
                value: 1.5
            })
        );
    }

    #[test]
    fn estado_inicial_e_estados_sao_canonicos() {
        let mut c = casa();
        c.machine.initial_state = "PANICO".to_string();
        assert_eq!(
            c.validate(),
            Err(CrisisCfgError::UnknownInitialState {
                found: "PANICO".to_string()
            })
        );
        c.machine.initial_state = "NORMAL".to_string();
        c.machine.states[0] = "CAOS".to_string();
        assert!(matches!(
            c.validate(),
            Err(CrisisCfgError::UnknownState { .. } | CrisisCfgError::IllegalStateOrder)
        ));
    }
}
