//! Módulo transversal de development: liga os subsistemas morfológicos ao runtime.

use std::collections::HashSet;
use std::sync::Mutex;
use triad_contracts as tc;
use triad_foundation as tf;
use triad_runtime as rt;
use tracing::{debug, trace};

use crate::corticalization::Corticalization;
use crate::maintenance::Maintenance;
use crate::morphogenesis::Morphogenesis;
use crate::regeneration::Regeneration;

/// Fotografia do estado de development publicada no contexto a cada tick.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DevelopmentStatus {
    /// Morfogênese em standby?
    pub standby: bool,
    /// Razão do despertar, se acordada.
    pub wake_reason: Option<String>,
    /// Total de reparos realizados.
    pub repairs: u64,
    /// Danos aguardando regeneração.
    pub damage_queue_len: usize,
    /// Descartes acumulados (regeneração + morfogênese).
    pub dropped: u64,
    /// Iterações de corticalização executadas.
    pub cortical_iterations: u64,
}

/// Estado interno protegido por lock — tick nunca entra em pânico.
struct Inner {
    maintenance: Maintenance,
    regeneration: Regeneration,
    morphogenesis: Morphogenesis,
    corticalization: Corticalization,
}

impl Inner {
    /// Subsistemas morfológicos recém-criados.
    fn new() -> Self {
        Self {
            maintenance: Maintenance::new(),
            regeneration: Regeneration::new(),
            morphogenesis: Morphogenesis::new(),
            corticalization: Corticalization::new(),
        }
    }
}

/// Módulo transversal de development (forma e manutenção do organismo).
pub struct DevelopmentModule {
    inner: Mutex<Inner>,
    descriptor: tc::ModuleDescriptor,
}

impl DevelopmentModule {
    /// Cria o módulo de development (transversal, domínio "development").
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner::new()),
            descriptor: tc::ModuleDescriptor {
                module_id: tf::ModuleId::new(),
                name: "development".to_string(),
                layer: tc::Layer::Transversal,
                domain: "development".to_string(),
            },
        }
    }
}

impl rt::CognitiveModule for DevelopmentModule {
    /// Descritor estático do módulo.
    fn descriptor(&self) -> &tc::ModuleDescriptor {
        &self.descriptor
    }

    /// Sempre ativo: transversal não dorme (crise muda política, nunca desliga sistema).
    fn state(&self) -> rt::ModuleState {
        rt::ModuleState::Active
    }

    /// Um passo de manutenção do organismo: amostral, orçado e sem pânico.
    fn tick(
        &self,
        ctx: &rt::TypedContext,
        out: &mut Vec<tc::EventEnvelope>,
    ) -> tf::TriadResult<()> {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(_) => {
                // Lock envenenado: contrato violado — nunca pânico.
                return Err(tf::TriadError::ContractViolation {
                    detail: "mutex de development envenenado".to_string(),
                });
            }
        };

        // Manutenção O(1): no scaffold os candidatos internos são vazios e nada consta danificado.
        let damaged: HashSet<tf::ModuleId> = HashSet::new();
        let _ = inner.maintenance.check(&[], &damaged);

        // Regeneração com orçamento: até 4 curas por passo.
        let _ = inner.regeneration.heal(4);

        // Morfogênese: sem sinal externo neste scaffold permanece em standby — nada acorda sem razão.

        // Corticalização: proxies fixas do scaffold; itera a migração até o critério de parada.
        while !inner.corticalization.step(0.5, 0.5, 0.1) {}

        let (standby, reason) = inner.morphogenesis.status();
        let status = DevelopmentStatus {
            standby,
            wake_reason: reason.map(|r| r.as_str().to_string()),
            repairs: inner.maintenance.repaired(),
            damage_queue_len: inner.regeneration.queue_len(),
            dropped: inner.regeneration.dropped() + inner.morphogenesis.dropped(),
            cortical_iterations: inner.corticalization.iterations(),
        };

        debug!(
            fila = status.damage_queue_len,
            descartados = status.dropped,
            reparos = status.repairs,
            iteracoes = status.cortical_iterations,
            "development tick publicado"
        );
        ctx.set("development.status", status);

        // Harmonia (13.4): consome o sinal Chladni do substrato — leitura
        // qualificada, ADITIVA. Ausência ≠ zero: sem sinal, trace com
        // motivo; uso em POLÍTICA (peso de atenção) só via ADR, com
        // necessidade demonstrada e consumidor real (regra de promoção).
        let chladni = ctx.get_qualified::<triad_chladni::Observation>("l1.substrate.chladni");
        if let Some(o) = chladni.as_ref_value() {
            trace!(
                banda = ?o.band,
                amostra = o.sample_size,
                ressonancia = ?o.resonance,
                "development consumiu o sinal chladni do substrato"
            );
        } else {
            trace!("development: sinal chladni ausente (ausência ≠ zero)");
        }
        out.push(rt::envelope(
            "development",
            ctx.clock().tick,
            tc::EventType::Development,
            tc::Priority::Normal,
        ));
        Ok(())
    }
}
