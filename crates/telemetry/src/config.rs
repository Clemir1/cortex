//! Config de telemetria injetada de `[telemetry.*]` — a escada de
//! evidência E0–E5 por módulo. Defaults congelados. `[telemetry.heavy]`
//! off é LIDO e RESPEITADO (nada de payload no hot path).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use triad_foundation::evidence::EvidenceLevel;

/// `[telemetry]` raiz.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TelemetryCfg {
    pub enabled: bool,
    /// Nível-alvo do núcleo (parse direto "E0".."E5").
    pub default_evidence_level: EvidenceLevel,
    /// Recibo producer→key→version→write_step→consumer→read_step.
    pub ledger_enabled: bool,
    /// Intervalo de emissão da auditoria (em ticks).
    pub emit_interval_steps: u64,
    /// `[telemetry.levels]` — rótulos canônicos da escada.
    pub levels: BTreeMap<String, String>,
    /// `[telemetry.modules]` — nível-alvo POR MÓDULO.
    pub modules: BTreeMap<String, EvidenceLevel>,
    /// `[telemetry.heavy]` — payloads completos/hash por step.
    pub heavy: HeavyCfg,
    /// `[telemetry.scientific]` — protocolo científico.
    pub scientific: ScientificCfg,
}

impl Default for TelemetryCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            default_evidence_level: EvidenceLevel::E5EffectValidated,
            ledger_enabled: true,
            emit_interval_steps: 10,
            levels: [
                ("E0", "declared"),
                ("E1", "initialized"),
                ("E2", "executed"),
                ("E3", "produced"),
                ("E4", "consumed"),
                ("E5", "effect_validated"),
            ]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
            modules: BTreeMap::new(),
            heavy: HeavyCfg::default(),
            scientific: ScientificCfg::default(),
        }
    }
}

/// `[telemetry.heavy]` — OFF no perfil base (não taxar o hot path).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HeavyCfg {
    pub enabled: bool,
    pub payload_capture: bool,
    pub per_step_state_hash: bool,
}

impl Default for HeavyCfg {
    fn default() -> Self {
        Self {
            enabled: false,
            payload_capture: false,
            per_step_state_hash: false,
        }
    }
}

/// `[telemetry.scientific]` — sham pareado, vereditos pré-registrados.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScientificCfg {
    pub paired_sham_arm: bool,
    /// "bit_identical" da casa (mudança display-only exige A/A).
    pub sham_strategy: String,
    pub pre_registered_verdicts: bool,
    /// Horizontes de checagem de efeito (t+1/t+5).
    pub effect_check_horizons: Vec<u64>,
}

impl Default for ScientificCfg {
    fn default() -> Self {
        Self {
            paired_sham_arm: false,
            sham_strategy: "bit_identical".to_string(),
            pre_registered_verdicts: false,
            effect_check_horizons: vec![1, 5],
        }
    }
}

/// Alvo de evidência de um módulo: nível do mapa `[telemetry.modules]`
/// quando declarado; senão o `default_evidence_level`.
pub fn target_for(cfg: &TelemetryCfg, module: &str) -> EvidenceLevel {
    cfg.modules
        .get(module)
        .copied()
        .unwrap_or(cfg.default_evidence_level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_da_casa_parseia_telemetry_1_1() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../config/default.toml"
        ))
        .expect("default.toml presente");
        let raw: toml::Value = src.parse().expect("TOML válido");
        let t = raw.get("telemetry").expect("[telemetry]");
        assert_eq!(
            t.get("default_evidence_level"),
            Some(&toml::Value::String("E5".into()))
        );
        let modules = t.get("modules").expect("[telemetry.modules]");
        assert!(modules.get("learning").is_some());
        assert!(modules.get("cybernetics").is_some());
        let heavy = t.get("heavy").expect("[telemetry.heavy]");
        assert_eq!(heavy.get("enabled"), Some(&toml::Value::Boolean(false)));
        let sci = t.get("scientific").expect("[telemetry.scientific]");
        assert_eq!(
            sci.get("sham_strategy"),
            Some(&toml::Value::String("bit_identical".into()))
        );
        assert_eq!(
            sci.get("effect_check_horizons"),
            Some(&toml::Value::Array(vec![
                toml::Value::Integer(1),
                toml::Value::Integer(5)
            ]))
        );
        // Parse direto para o tipo da escada:
        let cfg: TelemetryCfg = t.clone().try_into().expect("parse 1:1");
        assert_eq!(cfg.default_evidence_level, EvidenceLevel::E5EffectValidated);
        assert_eq!(
            cfg.modules.get("l1_substrate"),
            Some(&EvidenceLevel::E5EffectValidated)
        );
        assert!(!cfg.heavy.enabled, "heavy OFF no perfil base");
        assert_eq!(cfg.scientific.effect_check_horizons, vec![1, 5]);
    }

    #[test]
    fn defaults_congelados() {
        let c = TelemetryCfg::default();
        assert!(c.enabled);
        assert!(c.ledger_enabled);
        assert_eq!(c.emit_interval_steps, 10);
        assert!(!c.heavy.payload_capture, "sem payload no hot path");
        assert_eq!(c.levels.len(), 6, "E0..E5");
    }

    #[test]
    fn alvo_por_modulo_herdando_o_default() {
        let mut c = TelemetryCfg::default();
        c.modules
            .insert("learning".to_string(), EvidenceLevel::E4Consumed);
        assert_eq!(target_for(&c, "learning"), EvidenceLevel::E4Consumed);
        assert_eq!(
            target_for(&c, "desconhecido"),
            EvidenceLevel::E5EffectValidated,
            "herda o default"
        );
    }
}
