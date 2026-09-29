//! Construção de envelopes de eventos para o runtime.

use triad_contracts as tc;
use triad_foundation as tf;

/// Monta um envelope de evento raiz: sem pai, sem deadline, máscara vazia.
pub fn envelope(
    entity_id: &str,
    entity_version: u64,
    event_type: tc::EventType,
    priority: tc::Priority,
) -> tc::EventEnvelope {
    tc::EventEnvelope {
        event_id: tf::id::EventId::new(),
        parent_event_id: None,
        entity_id: entity_id.to_string(),
        entity_version,
        event_type,
        priority,
        deadline: None,
        dirty_mask: tc::DirtyMask::empty(),
    }
}
