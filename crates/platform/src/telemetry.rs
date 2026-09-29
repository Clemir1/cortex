//! Contadores de telemetria com taxas qualificadas.

use triad_contracts as tc;
use triad_foundation as tf;

/// Contadores de eventos por nome, thread-safe.
pub struct Counters {
    /// Mapa nome -> contagem.
    inner: std::sync::Mutex<std::collections::HashMap<String, u64>>,
}

impl Counters {
    /// Cria contadores vazios.
    pub fn new() -> Self {
        Self {
            inner: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Incrementa em 1 o contador `name`.
    pub fn record(&self, name: &str) {
        let mut map = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        *map.entry(name.to_string()).or_insert(0) += 1;
    }

    /// Valor atual do contador, se existir (ausência não é zero).
    pub fn get(&self, name: &str) -> Option<u64> {
        let map = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        map.get(name).copied()
    }

    /// Taxa do contador por tick, sempre qualificada.
    /// Ausência de contador ou ticks == 0 gera NO_DATA; valor inválido gera INVALID.
    pub fn rate(
        &self,
        name: &str,
        ticks: u64,
        source: tf::id::ModuleId,
        step: tf::id::StepId,
    ) -> tc::Qualified<tf::Rate> {
        let count = match self.get(name) {
            Some(c) => c,
            // Ausência de contador NÃO é zero.
            None => return tc::Qualified::no_data("contador ausente", source, step),
        };
        if ticks == 0 {
            return tc::Qualified::no_data("sem ticks", source, step);
        }
        match tf::Rate::construct(count as f32 / ticks as f32) {
            Some(rate) => tc::Qualified::value(rate, source, step),
            None => tc::Qualified::invalid("taxa fora do intervalo válido", source, step),
        }
    }
}
