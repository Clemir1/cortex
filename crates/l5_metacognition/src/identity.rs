//! Arco de identidade: valores nucleares, continuidade e observação L5.
//!
//! REAL (seção 16.1): os valores nucleares são OBSERVADOS do ciclo L4
//! real (taxas com denominador) — a identidade deriva do comportamento,
//! nunca de constantes decorativas; promoção de valores continua MANUAL
//! (E0–E5: nenhuma auto-promoção).

use std::sync::{Mutex, MutexGuard};
use tracing::warn;
use triad_contracts as tc;
use triad_foundation as tf;

/// Valores nucleares observados (crescimento, estabilidade, exploração, integridade).
#[derive(Debug, Clone, PartialEq)]
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

impl Default for IdentityValues {
    /// Nascimento neutro documentado: antes da primeira observação
    /// real do L4 o organismo parte do repouso (ausência ≠ zero: os
    /// valores são substituídos por medição, não lidos como dado).
    fn default() -> Self {
        Self {
            growth: 0.0,
            stability: 0.0,
            exploration: 0.0,
            integrity: 0.0,
        }
    }
}

/// Arco de identidade corrente em um passo do organismo.
#[derive(Debug, Clone)]
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

    /// Observa valores externos: atualiza o arco com o comportamento
    /// corrente (observação ≠ promoção — `promote_identity` é a
    /// intervenção MANUAL) e recalcula a continuidade pela
    /// compatibilidade com os valores ANTERIORES (suavização EMA).
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
        // O arco REFLETE o comportamento observado corrente (a promoção
        // de NOVOS valores nucleares é sempre manual — E0–E5).
        arc.values = observed_values;
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
        warn!(
            growth = new_values.growth,
            stability = new_values.stability,
            exploration = new_values.exploration,
            integrity = new_values.integrity,
            "promocao MANUAL de identidade aplicada (intervencao externa)"
        );
        arc.values = new_values;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step() -> tf::StepId {
        tf::StepId::new()
    }

    #[test]
    fn continuidade_cresce_com_observacao_estavel() {
        let store = IdentityStore::new(IdentityArc {
            values: IdentityValues::default(),
            continuity: 0.5,
            step: step(),
        });
        let v = IdentityValues {
            growth: 0.3,
            stability: 0.8,
            exploration: 0.2,
            integrity: 0.9,
        };
        // Observar valores coerentes duas vezes eleva a compatibilidade.
        store.observe(v.clone(), step());
        let arc = store.observe(v, step());
        assert!(arc.continuity > 0.5, "observacao estabiliza a identidade");
        assert!(arc.continuity <= 1.0);
    }

    #[test]
    fn ruptura_grande_pune_continuidade() {
        let store = IdentityStore::new(IdentityArc {
            values: IdentityValues {
                growth: 0.9,
                stability: 0.9,
                exploration: 0.9,
                integrity: 0.9,
            },
            continuity: 0.9,
            step: step(),
        });
        // Delta médio 0.8+ entre valores: ruptura.
        let arc = store.observe(IdentityValues::default(), step());
        assert!(arc.continuity < 0.9, "ruptura reduz continuidade");
    }
}
