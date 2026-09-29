//! Cognição local L3: campo de atenção com limiar e capacidade limitada.
//!
//! Limiar, capacidade e ganho vêm de `config/default.toml [l3.attention]`
//! (`AttentionCfg`); as consts são os defaults congelados.

use std::cmp::Ordering;
use triad_foundation as tf;
use tf::id::ConceptId;
use tracing::trace;

/// Peso mínimo para um conceito ser considerado saliente — default
/// congelado; o valor real vem do TOML central.
pub const ATTENTION_THRESHOLD: f32 = 0.35;

/// Capacidade máxima de focos simultâneos — default congelado.
pub const MAX_FOCI: usize = 128;

/// `[l3.attention]` — política do campo de atenção.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AttentionCfg {
    /// Peso mínimo de saliência (`salience_threshold`).
    pub salience_threshold: f32,
    /// Capacidade máxima de focos simultâneos (`max_candidates`).
    pub max_candidates: usize,
    /// Ganho aplicado ao peso ao registrar (`gain`).
    pub gain: f32,
}

impl Default for AttentionCfg {
    fn default() -> Self {
        Self {
            salience_threshold: ATTENTION_THRESHOLD,
            max_candidates: MAX_FOCI,
            gain: 1.0,
        }
    }
}

/// Fotografia imutável dos focos de atenção em um passo.
#[derive(Debug, Clone)]
pub struct AttentionFoci {
    /// Identificadores dos conceitos em foco, do mais ao menos saliente.
    pub foci: Vec<ConceptId>,
    /// Passo em que a fotografia foi produzida.
    pub step: u64,
}

/// Campo de atenção dinâmico: focos ponderados por saliência.
#[derive(Debug, Clone)]
pub struct AttentionField {
    /// Focos ativos com seus pesos de saliência.
    foci: Vec<(ConceptId, f32)>,
    /// Passo atual do campo de atenção.
    step: u64,
    /// Política vigente (limiar, capacidade, ganho) do TOML central.
    cfg: AttentionCfg,
}

impl AttentionField {
    /// Cria um campo de atenção vazio no passo dado (política default).
    pub fn new(step: u64) -> Self {
        Self::with_config(step, AttentionCfg::default())
    }

    /// Cria o campo com a política injetada de `[l3.attention]`.
    pub fn with_config(step: u64, cfg: AttentionCfg) -> Self {
        Self {
            foci: Vec::new(),
            step,
            cfg,
        }
    }

    /// Registra conceito acima do limiar, reordena por peso e avança o passo.
    pub fn salient(&mut self, id: ConceptId, weight: f32) {
        let weighted = weight * self.cfg.gain;
        if weighted >= self.cfg.salience_threshold {
            trace!(peso = weighted, "conceito cruza o limiar de atenção");
            self.foci.push((id, weighted));
        }
        self.foci
            .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        self.foci.truncate(self.cfg.max_candidates);
        self.step += 1;
    }

    /// Produz uma fotografia com apenas os identificadores dos focos.
    pub fn snapshot(&self) -> AttentionFoci {
        trace!(focos = self.foci.len(), "snapshot de focos");
        AttentionFoci {
            foci: self.foci.iter().map(|(id, _)| *id).collect(),
            step: self.step,
        }
    }

    /// Número de focos ativos no campo.
    pub fn len(&self) -> usize {
        self.foci.len()
    }

    /// Política vigente (auditoria da configuração).
    pub fn config(&self) -> &AttentionCfg {
        &self.cfg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limiar_e_capacidade_da_config_sao_respeitados() {
        let cfg = AttentionCfg {
            salience_threshold: 0.5,
            max_candidates: 2,
            gain: 1.0,
        };
        let mut field = AttentionField::with_config(0, cfg);
        field.salient(ConceptId::new(), 0.4); // abaixo do limiar 0.5
        field.salient(ConceptId::new(), 0.6);
        field.salient(ConceptId::new(), 0.9);
        field.salient(ConceptId::new(), 0.7); // acima da capacidade 2
        let foci = field.snapshot();
        assert_eq!(foci.foci.len(), 2, "capacidade 2 respeitada");
        assert_eq!(field.config().max_candidates, 2);
    }

    #[test]
    fn default_congelado_bate_com_as_consts_e_o_toml() {
        let d = AttentionCfg::default();
        assert!((d.salience_threshold - 0.35).abs() < 1e-6);
        assert_eq!(d.max_candidates, 128);
        assert!((d.gain - 1.0).abs() < 1e-6);
        // [l3.attention] do default.toml carrega por Deserialize.
        let text = "salience_threshold = 0.35\nmax_candidates = 128\ngain = 1.0\n";
        let from_toml: AttentionCfg = toml::from_str(text).expect("[l3.attention]");
        assert_eq!(from_toml.max_candidates, 128);
    }
}
