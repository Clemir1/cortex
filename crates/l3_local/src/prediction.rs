//! Cognição local L3: predição de passos futuros a partir de evidências.
//!
//! Horizonte e confiança mínima vêm de `config/default.toml
//! [l3.prediction]` (`PredictionCfg`); as consts são os defaults
//! congelados. `error_tolerance` é declarada no TOML para o avaliador
//! futuro de acerto/erro (o consumidor compara |observado − previsto|).

use triad_contracts as tc;
use triad_foundation as tf;
use tf::id::{ModuleId, StepId};
use tracing::{debug, trace};

/// Horizonte fixo, em passos, de toda predição emitida — congelado.
pub const HORIZON: u32 = 5;

/// Confiança mínima para emitir predição; abaixo disso, declara ausência.
pub const MIN_CONFIDENCE: f32 = 0.2;

/// Tolerância de erro declarada (avaliador de acerto futuro) — congelada.
pub const ERROR_TOLERANCE: f32 = 0.1;

/// `[l3.prediction]` — política do preditor.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PredictionCfg {
    /// Horizonte, em passos, do evento previsto (`horizon_steps`).
    pub horizon_steps: u32,
    /// Confiança mínima para emitir (`min_confidence`).
    pub min_confidence: f32,
    /// Tolerância do avaliador de acerto (`error_tolerance`).
    pub error_tolerance: f32,
}

impl Default for PredictionCfg {
    fn default() -> Self {
        Self {
            horizon_steps: HORIZON,
            min_confidence: MIN_CONFIDENCE,
            error_tolerance: ERROR_TOLERANCE,
        }
    }
}

/// Predição de um evento futuro, com confiança e passo de emissão.
#[derive(Debug, Clone)]
pub struct Prediction {
    /// Horizonte, em passos, do evento previsto.
    pub horizon: u32,
    /// Evento esperado.
    pub expected: String,
    /// Confiança atribuída à predição.
    pub confidence: f32,
    /// Passo em que a predição foi emitida.
    pub step: u64,
}

/// Emite predições qualificadas a partir da melhor evidência disponível.
pub struct Predictor {
    /// Última predição emitida; None declara ausência.
    last: Option<Prediction>,
    /// Política vigente (horizonte, confiança mínima) do TOML central.
    cfg: PredictionCfg,
}

impl Predictor {
    /// Cria um predictor sem predição anterior (política default).
    pub fn new() -> Self {
        Self::with_config(PredictionCfg::default())
    }

    /// Cria o predictor com a política injetada de `[l3.prediction]`.
    pub fn with_config(cfg: PredictionCfg) -> Self {
        Self { last: None, cfg }
    }

    /// Recalcula a predição com a melhor evidência; sem força mínima, vira ausência.
    pub fn predict(&mut self, evidence: &[(String, f32)], step: u64) {
        let mut best: Option<&(String, f32)> = None;
        for pair in evidence {
            let stronger = match best {
                None => true,
                Some(current) => pair.1 > current.1,
            };
            if stronger {
                best = Some(pair);
            }
        }
        match best {
            Some((expected, confidence)) if *confidence >= self.cfg.min_confidence => {
                trace!(confianca = *confidence, "predição calculada");
                self.last = Some(Prediction {
                    horizon: self.cfg.horizon_steps,
                    expected: expected.clone(),
                    confidence: *confidence,
                    step,
                });
            }
            _ => self.last = None,
        }
    }

    /// Última predição como Qualified; ausência vira NO_DATA explícito.
    pub fn latest_qualified(&self, source: ModuleId, step_id: StepId) -> tc::Qualified<Prediction> {
        match &self.last {
            Some(p) => tc::Qualified::value(p.clone(), source, step_id),
            None => {
                debug!("sem predição válida");
                tc::Qualified::no_data("sem predição", source, step_id)
            }
        }
    }

    /// Horizonte vigente de predição, em passos (política injetada).
    pub fn horizon(&self) -> u32 {
        self.cfg.horizon_steps
    }

    /// Política vigente (auditoria da configuração).
    pub fn config(&self) -> &PredictionCfg {
        &self.cfg
    }

    /// Última predição emitida (clone), se existir — ponte L3→L4.
    /// Ausência permanece ausência (None), nunca valor fantasma.
    pub fn last(&self) -> Option<Prediction> {
        self.last.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizonte_e_confianca_da_config_sao_respeitados() {
        let cfg = PredictionCfg {
            horizon_steps: 9,
            min_confidence: 0.9,
            error_tolerance: 0.1,
        };
        let mut p = Predictor::with_config(cfg);
        p.predict(&[("fraco".into(), 0.5)], 1); // 0.5 < 0.9 ⇒ ausência
        assert!(p.latest_qualified(ModuleId::new(), StepId::new())
            .as_ref_value()
            .is_none());
        p.predict(&[("forte".into(), 0.95)], 2);
        let last = p.latest_qualified(ModuleId::new(), StepId::new());
        let pred = last.as_ref_value().expect("emitida acima do mínimo");
        assert_eq!(pred.horizon, 9, "horizonte da config");
    }

    #[test]
    fn default_congelado_bate_com_as_consts() {
        let d = PredictionCfg::default();
        assert_eq!(d.horizon_steps, 5);
        assert!((d.min_confidence - 0.2).abs() < 1e-6);
        assert!((d.error_tolerance - 0.1).abs() < 1e-6);
    }
}
