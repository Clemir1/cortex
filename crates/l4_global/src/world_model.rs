//! Modelo do mundo: crenças versionadas por observação.

use std::collections::HashMap;

use triad_contracts as tc;
use triad_foundation as tf;
use tracing::trace;

/// Crença do mundo: confiança observada para uma chave.
#[derive(Debug, Clone)]
pub struct Belief {
    /// Chave da crença.
    pub key: String,
    /// Confiança observada (0.0..1.0).
    pub confidence: f32,
}

/// Modelo do mundo: crenças versionadas por observação.
pub struct WorldModel {
    /// Crenças correntes por chave.
    beliefs: HashMap<String, f32>,
    /// Versão monotônica do modelo.
    version: u64,
}

impl WorldModel {
    /// Modelo do mundo vazio.
    pub fn new() -> Self {
        Self {
            beliefs: HashMap::new(),
            version: 0,
        }
    }

    /// Registra observação: atualiza confiança e avança a versão.
    pub fn observe(&mut self, key: &str, confidence: f32) {
        self.beliefs
            .insert(key.to_string(), confidence.clamp(0.0, 1.0));
        self.version += 1;
        trace!(chave = %key, versao = self.version, "crenca observada");
    }

    /// Consulta uma crença qualificada; ausência nunca vira zero.
    pub fn belief(
        &self,
        key: &str,
        source: tf::id::ModuleId,
        step: tf::id::StepId,
    ) -> tc::Qualified<f32> {
        match self.beliefs.get(key) {
            Some(confidence) => tc::Qualified::value(*confidence, source, step),
            None => tc::Qualified::no_data("crença ausente", source, step),
        }
    }

    /// Versão monotônica do modelo.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Número de crenças correntes.
    pub fn len(&self) -> usize {
        self.beliefs.len()
    }
}

impl Default for WorldModel {
    /// Estado inicial: modelo vazio.
    fn default() -> Self {
        Self::new()
    }
}
