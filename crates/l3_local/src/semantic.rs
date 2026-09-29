//! Campo semântico local da camada L3.

use triad_foundation as tf;
use tracing::{debug, trace};

use tf::id::ConceptId;

/// Força mínima para um conceito se ligar ao campo semântico.
pub const BINDING_THRESHOLD: f32 = 0.4;

/// Capacidade máxima de conceitos no campo semântico.
pub const MAX_CONCEPTS: usize = 4096;

/// Conceito individual ligado ao campo semântico.
#[derive(Debug, Clone)]
pub struct Concept {
    /// Identificador único do conceito.
    pub id: ConceptId,
    /// Rótulo textual do conceito.
    pub label: String,
    /// Força de ligação no campo, no intervalo 0.0..=1.0.
    pub binding: f32,
    /// Contador de reforços (usos) do conceito.
    pub usage: u64,
}

/// Campo semântico que concentra os conceitos da cognição local.
pub struct SemanticField {
    concepts: Vec<Concept>,
}

impl SemanticField {
    /// Cria um campo semântico vazio.
    pub fn new() -> Self {
        Self { concepts: Vec::new() }
    }

    /// Liga um novo conceito; None se fraco demais ou campo cheio (ausência ≠ zero).
    pub fn bind(&mut self, label: &str, strength: f32) -> Option<ConceptId> {
        if strength < BINDING_THRESHOLD {
            return None;
        }
        if self.concepts.len() >= MAX_CONCEPTS {
            debug!(total = self.concepts.len(), "teto de conceitos atingido, bind descartado");
            return None;
        }
        let id = ConceptId::new();
        self.concepts.push(Concept {
            id,
            label: label.to_string(),
            binding: strength.clamp(0.0, 1.0),
            usage: 0,
        });
        trace!(label = label, strength = strength, "conceito ligado ao campo");
        Some(id)
    }

    /// Reforca um conceito por id (usage += 1, binding somado e clampado); ausente não faz nada.
    pub fn reinforce(&mut self, id: ConceptId, amount: f32) {
        if let Some(concept) = self.concepts.iter_mut().find(|c| c.id == id) {
            concept.usage += 1;
            concept.binding = (concept.binding + amount).clamp(0.0, 1.0);
            trace!(usage = concept.usage, binding = concept.binding, "conceito reforcado");
        }
    }

    /// Busca o conceito pelo rótulo; None se não existir.
    pub fn by_label(&self, label: &str) -> Option<&Concept> {
        self.concepts.iter().find(|c| c.label == label)
    }

    /// Número de conceitos atualmente ligados ao campo.
    pub fn len(&self) -> usize {
        self.concepts.len()
    }
}
