//! Observação Chladni — o sinal publicado pelos donos de estado no
//! TypedContext e consumido por qualquer camada.
//!
//! Harmonia Cluster/HOTM/Chladni: o L1 (dono do estado 97D e da energia)
//! publica `Observation` na chave `l1.substrate.chladni`; consumidores
//! (atenção L3, development) leem o sinal QUALIFICADO sem tocar o estado
//! canônico — ausência ≠ zero, denominador sempre visível.

use crate::CognitiveBand;
use triad_foundation as tf;

/// Sinal de ressonância de um tick — aditivo, nunca muda comportamento.
#[derive(Debug, Clone)]
pub struct Observation {
    pub step: tf::StepId,
    /// Ressonância média da amostra — VALUE ou ausência explícita
    /// (`no_data`), nunca zero fantasma.
    pub resonance: tf::Qualified<f32>,
    /// Banda cognitiva da frequência da energia média da amostra.
    pub band: Option<CognitiveBand>,
    /// Tamanho da amostra efetiva (denominador sempre visível).
    pub sample_size: u64,
}
