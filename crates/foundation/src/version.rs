//! Versionamento de schema.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Versão de schema — monotônica, nunca reutilizada nem rebaixada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SchemaVersion(pub u32);

impl SchemaVersion {
    /// Versão de schema vigente no núcleo mínimo.
    pub const fn current() -> Self {
        Self(1)
    }
}

impl fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
