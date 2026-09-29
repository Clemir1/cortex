use std::any::Any;
use std::collections::HashMap;
use std::sync::Mutex;

use triad_foundation as tf;
use triad_contracts as tc;

/// Contexto tipado compartilhado pelos módulos durante um tick.
pub struct TypedContext {
    /// Relógio lógico corrente do runtime.
    pub clock: tf::LogicalClock,
    /// Identidade do módulo hospedeiro que criou o contexto.
    host: tf::id::ModuleId,
    /// Slots de dados endereçados por chave estática.
    slots: Mutex<HashMap<&'static str, Box<dyn Any + Send + Sync>>>,
}

impl TypedContext {
    /// Cria um contexto com o relógio dado e um host novo.
    pub fn new(clock: tf::LogicalClock) -> Self {
        Self {
            clock,
            host: tf::id::ModuleId::new(),
            slots: Mutex::new(HashMap::new()),
        }
    }

    /// Acesso imutável ao relógio lógico.
    pub fn clock(&self) -> &tf::LogicalClock {
        &self.clock
    }

    /// Acesso mutável ao relógio lógico.
    pub fn clock_mut(&mut self) -> &mut tf::LogicalClock {
        &mut self.clock
    }

    /// Grava um valor clonável no slot indicado; nunca entra em pânico.
    pub fn set<T: Clone + Send + Sync + 'static>(&self, key: &'static str, value: T) {
        let mut guard = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        guard.insert(key, Box::new(value));
    }

    /// Lê o slot como `Qualified<T>`: NO_DATA, INVALID ou VALUE, sem fabricar valor.
    pub fn get_qualified<T: Clone + Send + Sync + 'static>(&self, key: &'static str) -> tc::Qualified<T> {
        let guard = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        match guard.get(key) {
            None => tc::Qualified::no_data(key, self.host, self.clock.step),
            Some(boxed) => match boxed.downcast_ref::<T>() {
                None => tc::Qualified::invalid("tipo divergente", self.host, self.clock.step),
                Some(v) => tc::Qualified::value(v.clone(), self.host, self.clock.step),
            },
        }
    }
}
