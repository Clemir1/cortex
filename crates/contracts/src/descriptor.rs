//! Descritores de módulos e camadas do Cortex.

use triad_foundation as tf;

use serde::{Deserialize, Serialize};

/// Camada em que um módulo reside.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Layer {
    L1,
    L2,
    L3,
    L4,
    L5,
    Transversal,
    Platform,
    Binding,
}

/// Cartão de identidade de um módulo: id, nome, camada e domínio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleDescriptor {
    pub module_id: tf::id::ModuleId,
    pub name: String,
    pub layer: Layer,
    pub domain: String,
}
