//! Config de cybernetics injetada de `[cybernetics.*]` — ordens O1–O5
//! como protocolos SOBRE componentes existentes (intervalos por ordem).
//! Lei 7 (O5 offline): `genome_in_main_loop` é LIDO e RESPEITADO —
//! false congela o genome fora do loop principal; o módulo só coleta
//! amostras de horizonte.

use serde::{Deserialize, Serialize};

/// Uma ordem cibernética com cadência própria.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OrderCfg {
    pub enabled: bool,
    /// Intervalo em ticks entre execuções da ordem (0 = todo tick;
    /// mínimo aplicado 1).
    pub interval_steps: u64,
}

impl Default for OrderCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_steps: 10,
        }
    }
}

/// `[cybernetics.o5_evolution]` — LENTO, entre episódios.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct O5Cfg {
    pub enabled: bool,
    /// Cadência declarada ("between_episodes" da casa).
    pub cadence: String,
    pub min_interval_steps: u64,
    /// Lei 7: genome FORA do loop principal (false congela).
    pub genome_in_main_loop: bool,
}

impl Default for O5Cfg {
    fn default() -> Self {
        Self {
            enabled: true,
            cadence: "between_episodes".to_string(),
            min_interval_steps: 10_000,
            genome_in_main_loop: false,
        }
    }
}

/// `[cybernetics]` — política completa do transversal de controle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CyberneticsCfg {
    pub o1_control: OrderCfg,
    pub o2_adaptation: OrderCfg,
    pub o3_coordination: OrderCfg,
    pub o4_self_observation: OrderCfg,
    pub o5_evolution: O5Cfg,
}

impl Default for CyberneticsCfg {
    fn default() -> Self {
        // Intervalos da casa (default.toml): O1=10, O2=25, O3=50,
        // O4=100; O5 entre episódios (Lei 7).
        Self {
            o1_control: OrderCfg {
                enabled: true,
                interval_steps: 10,
            },
            o2_adaptation: OrderCfg {
                enabled: true,
                interval_steps: 25,
            },
            o3_coordination: OrderCfg {
                enabled: true,
                interval_steps: 50,
            },
            o4_self_observation: OrderCfg {
                enabled: true,
                interval_steps: 100,
            },
            o5_evolution: O5Cfg::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_da_casa_parseia_cybernetics_1_1() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../config/default.toml"
        ))
        .expect("default.toml presente");
        let raw: toml::Value = src.parse().expect("TOML válido");
        let t = raw.get("cybernetics").expect("[cybernetics]");
        assert!(t.get("o1_control").is_some());
        assert!(t.get("o2_adaptation").is_some());
        assert!(t.get("o3_coordination").is_some());
        assert!(t.get("o4_self_observation").is_some());
        let o5 = t.get("o5_evolution").expect("o5");
        assert_eq!(
            o5.get("genome_in_main_loop"),
            Some(&toml::Value::Boolean(false)),
            "Lei 7: genome fora do loop principal"
        );
        assert!(o5.get("min_interval_steps").is_some());
    }

    #[test]
    fn defaults_congelados() {
        let c = CyberneticsCfg::default();
        assert!(c.o1_control.enabled);
        assert_eq!(c.o1_control.interval_steps, 10);
        assert_eq!(c.o2_adaptation.interval_steps, 25);
        assert_eq!(c.o3_coordination.interval_steps, 50);
        assert_eq!(c.o4_self_observation.interval_steps, 100);
        assert!(!c.o5_evolution.genome_in_main_loop, "Lei 7");
    }
}
