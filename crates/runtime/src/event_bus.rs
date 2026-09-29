//! Barramento de eventos do runtime com canal limitado.
//!
//! Orientação a eventos (diretriz do dono): módulos publicam envelopes
//! pequenos; consumidores assinam com `subscribe` e recebem TUDO por
//! fan-out (cada assinante tem fila própria — consumo não rouba evento
//! de outro). Descarte por fila cheia é CONTADO, nunca silencioso.
//! O canal principal (drain do scheduler/app) segue intacto.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crossbeam_channel::{bounded, Receiver, Sender};
use triad_contracts as tc;
use triad_foundation as tf;

/// Barramento de eventos com canal limitado e descarte determinístico.
pub struct EventBus {
    tx: Sender<tc::EventEnvelope>,
    rx: Receiver<tc::EventEnvelope>,
    /// Assinantes por fan-out (cada um com fila própria).
    subscribers: Arc<Mutex<Vec<Sender<tc::EventEnvelope>>>>,
    /// Eventos descartados por fila de assinante cheia (contador vivo).
    subscriber_drops: AtomicU64,
}

impl EventBus {
    /// Cria um barramento com capacidade fixa de eventos.
    pub fn new(capacity: usize) -> Self {
        let (tx, rx) = bounded(capacity);
        Self {
            tx,
            rx,
            subscribers: Arc::new(Mutex::new(Vec::new())),
            subscriber_drops: AtomicU64::new(0),
        }
    }

    /// Assina o barramento: fila própria com a capacidade dada; o
    /// assinante recebe uma cópia de CADA evento publicado (fan-out).
    pub fn subscribe(&self, capacity: usize) -> EventSubscription {
        let (tx, rx) = bounded(capacity);
        self.subscribers
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(tx);
        EventSubscription { rx }
    }

    /// Eventos descartados por filas de assinante cheias (auditoria).
    pub fn subscriber_drops(&self) -> u64 {
        self.subscriber_drops.load(Ordering::Relaxed)
    }

    /// Publica um evento sem bloquear; falha quando cheio ou desconectado.
    /// Entrega também a todos os assinantes (descartes contados).
    pub fn publish(&self, ev: tc::EventEnvelope) -> tf::TriadResult<()> {
        let result = match self.tx.try_send(ev.clone()) {
            Ok(()) => Ok(()),
            Err(crossbeam_channel::TrySendError::Full(_)) => Err(tf::TriadError::Invalid {
                about: "event bus cheio".into(),
                reason: "descarte determinístico".into(),
            }),
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => Err(tf::TriadError::Invalid {
                about: "event bus desconectado".into(),
                reason: "canal fechado".into(),
            }),
        };
        // Fan-out para assinantes: descarte é contado, nunca silencioso.
        let subs = self.subscribers.lock().unwrap_or_else(|p| p.into_inner());
        for sub in subs.iter() {
            if sub.try_send(ev.clone()).is_err() {
                self.subscriber_drops.fetch_add(1, Ordering::Relaxed);
            }
        }
        result
    }

    /// Drena até `max` eventos pendentes, parando no primeiro erro.
    pub fn try_receive(&self, max: usize) -> Vec<tc::EventEnvelope> {
        let mut out = Vec::new();
        for _ in 0..max {
            match self.rx.try_recv() {
                Ok(ev) => out.push(ev),
                Err(_) => break,
            }
        }
        out
    }
}

/// Assinatura de eventos: fila própria do consumidor.
pub struct EventSubscription {
    rx: Receiver<tc::EventEnvelope>,
}

impl EventSubscription {
    /// Drena até `max` eventos da fila do assinante.
    pub fn try_receive(&self, max: usize) -> Vec<tc::EventEnvelope> {
        let mut out = Vec::new();
        for _ in 0..max {
            match self.rx.try_recv() {
                Ok(ev) => out.push(ev),
                Err(_) => break,
            }
        }
        out
    }

    /// Eventos pendentes na fila do assinante.
    pub fn pending(&self) -> usize {
        self.rx.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(id: u64) -> tc::EventEnvelope {
        crate::envelope("teste", id, tc::EventType::Tissue, tc::Priority::Normal)
    }

    #[test]
    fn assinante_recebe_copia_de_cada_evento_publicado() {
        let bus = EventBus::new(16);
        let sub = bus.subscribe(16);
        assert!(bus.publish(envelope(1)).is_ok());
        assert!(bus.publish(envelope(2)).is_ok());
        assert_eq!(sub.pending(), 2, "fan-out entrega cópias");
        let eventos = sub.try_receive(10);
        assert_eq!(eventos.len(), 2);
        // O canal principal também recebeu (comportamento intacto).
        assert_eq!(bus.try_receive(10).len(), 2);
    }

    #[test]
    fn dois_assinantes_recebem_o_mesmo_evento_sem_roubo() {
        let bus = EventBus::new(16);
        let a = bus.subscribe(16);
        let b = bus.subscribe(16);
        bus.publish(envelope(9)).expect("publica");
        assert_eq!(a.try_receive(4).len(), 1);
        assert_eq!(b.try_receive(4).len(), 1, "consumo de A não rouba de B");
    }

    #[test]
    fn fila_cheia_descarta_com_contador_nunca_silencioso() {
        let bus = EventBus::new(16);
        let sub = bus.subscribe(1);
        bus.publish(envelope(1)).expect("primeiro cabe");
        bus.publish(envelope(2)).expect("principal aceita");
        assert_eq!(
            bus.subscriber_drops(),
            1,
            "descarte do assinante contado"
        );
        assert_eq!(sub.try_receive(4).len(), 1);
        assert_eq!(bus.try_receive(4).len(), 2, "principal intacto");
    }
}
