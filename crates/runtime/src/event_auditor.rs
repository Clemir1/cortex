//! Auditor de eventos: assinante canônico do barramento.
//!
//! Orientação a eventos (diretriz do dono) com instrumentação de série:
//! o auditor assina o `EventBus` com fila própria (fan-out — consumir
//! não rouba de outros assinantes), conta cada evento por fonte
//! (`entity_id`) e expõe o relatório ordenado. Descartes por fila cheia
//! são observáveis pelo `subscriber_drops()` do barramento.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::event_bus::{EventBus, EventSubscription};

/// Auditor de eventos do barramento (assinante permanente).
pub struct EventAuditor {
    subscription: EventSubscription,
    counts: Mutex<HashMap<String, usize>>,
    total: Mutex<u64>,
}

impl EventAuditor {
    /// Assina o barramento com a capacidade de fila dada.
    pub fn new(bus: &EventBus, capacity: usize) -> Self {
        Self {
            subscription: bus.subscribe(capacity),
            counts: Mutex::new(HashMap::new()),
            total: Mutex::new(0),
        }
    }

    /// Drena a fila e conta cada evento recebido por fonte.
    /// Chame entre ciclos ou ao final — a fila é limitada.
    pub fn record(&self) -> usize {
        let eventos = self.subscription.try_receive(1_000_000);
        let n = eventos.len();
        if n == 0 {
            return 0;
        }
        let mut counts = self.counts.lock().unwrap_or_else(|p| p.into_inner());
        let mut total = self.total.lock().unwrap_or_else(|p| p.into_inner());
        for ev in eventos {
            *counts.entry(ev.entity_id.clone()).or_default() += 1;
            *total += 1;
        }
        n
    }

    /// Total de eventos contados desde a gênese do auditor.
    pub fn total(&self) -> u64 {
        *self.total.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Relatório ordenado por fonte (auditoria legível).
    pub fn report(&self) -> Vec<(String, usize)> {
        let counts = self.counts.lock().unwrap_or_else(|p| p.into_inner());
        let mut out: Vec<(String, usize)> =
            counts.iter().map(|(k, v)| (k.clone(), *v)).collect();
        out.sort();
        out
    }

    /// Eventos ainda pendentes na fila do assinante.
    pub fn pending(&self) -> usize {
        self.subscription.pending()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope;
    use triad_contracts as tc;

    #[test]
    fn auditor_conta_por_fonte_e_relata_ordenado() {
        let bus = EventBus::new(64);
        let auditor = EventAuditor::new(&bus, 64);
        for _ in 0..3 {
            bus.publish(envelope(
                "l2.tissue",
                1,
                tc::EventType::Tissue,
                tc::Priority::Normal,
            ))
            .expect("publica");
        }
        bus.publish(envelope(
            "l1.substrate",
            1,
            tc::EventType::Physical,
            tc::Priority::Normal,
        ))
        .expect("publica");
        assert_eq!(auditor.pending(), 4);
        let recebidos = auditor.record();
        assert_eq!(recebidos, 4);
        assert_eq!(auditor.total(), 4);
        assert_eq!(
            auditor.report(),
            vec![
                ("l1.substrate".to_string(), 1),
                ("l2.tissue".to_string(), 3),
            ],
            "ordenado por fonte"
        );
        assert_eq!(auditor.record(), 0, "fila drenada");
    }
}
