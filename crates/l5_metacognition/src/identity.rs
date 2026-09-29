//! Arco de identidade: valores nucleares, continuidade e observação L5.

use std::sync::{Mutex, MutexGuard};
use triad_contracts as tc;
use triad_foundation as tf;

/// Valores nucleares observados (crescimento, estabilidade, exploração, integridade).
#[derive(Debug, Clone)]
pub struct IdentityValues {
    /// Peso do crescimento.
    pub growth: f32,
    /// Peso da estabilidade.
    pub stability: f32,
    /// Peso da exploração.
    pub exploration: f32,
    /// Peso da integridade.
    pub integrity: f32,
}

/// Arco de identidade corrente em um passo do organismo.
#[derive(Clone)]
pub struct IdentityArc {
    /// Valores nucleares.
    pub values: IdentityValues,
    /// Continuidade percebida da identidade (0.0..1.0).
    pub continuity: f32,
    /// Passo da última observação.
    pub step: tf::StepId,
}

/// Armazenamento thread-safe do arco de identidade.
pub struct IdentityStore {
    inner: Mutex<IdentityArc>,
}

impl IdentityStore {
    /// Cria o armazenamento com um arco inicial.
    pub fn new(initial: IdentityArc) -> Self {
        Self { inner: Mutex::new(initial) }
    }

    /// Guarda o mutex mesmo envenenado: identidade nunca fica inacessível.
    fn locked(&self) -> MutexGuard<'_, IdentityArc> {
        match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// Snapshot (clone) do arco corrente.
    pub fn current(&self) -> IdentityArc {
        self.locked().clone()
    }

    /// Observa valores externos; só atualiza continuidade (promoção é manual).
    pub fn observe(&self, observed_values: IdentityValues, step: tf::StepId) -> IdentityArc {
        let mut arc = self.locked();
        let avg_delta = ((arc.values.growth - observed_values.growth).abs()
            + (arc.values.stability - observed_values.stability).abs()
            + (arc.values.exploration - observed_values.exploration).abs()
            + (arc.values.integrity - observed_values.integrity).abs())
            / 4.0;
        let compat = 1.0 - avg_delta;
        let mut continuity = arc.continuity * 0.9 + compat * 0.1;
        if avg_delta > 0.8 {
            continuity -= 0.5;
        }
        arc.continuity = continuity.clamp(0.0, 1.0);
        arc.step = step;
        arc.clone()
    }

    /// Contexto de identidade sempre presente; ausência não existe aqui.
    pub fn identity_context(&self, source: tf::id::ModuleId) -> tc::IdentityContext {
        let arc = self.locked();
        tc::IdentityContext {
            // Valores nucleares como rótulos estáveis do contrato (Vec<String>).
            values: vec![
                "growth".to_string(),
                "stability".to_string(),
                "exploration".to_string(),
                "integrity".to_string(),
            ],
            continuity: tf::Confidence::construct(arc.continuity.clamp(0.0, 1.0))
                .unwrap_or_else(|| tf::Confidence::construct(0.0).unwrap()),
            source_module: source,
        }
    }

    /// Promoção MANUAL de valores (E0-E5: nunca automática); registra log.
    pub fn promote_identity(&mut self, new_values: IdentityValues) {
        let mut arc = self.locked();
        eprintln!("[L5] promocao MANUAL de identidade aplicada");
        arc.values = new_values;
    }
}
