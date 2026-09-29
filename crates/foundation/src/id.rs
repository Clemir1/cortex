//! Identidades opacas do organismo.
//!
//! Cada newtype envolve um `uuid::Uuid` v7 (ordenável pelo tempo). A
//! opacidade impede misturar identidades de entidades diferentes:
//! um `StepId` nunca é aceito onde se pede um `ClusterId`.

use serde::{Deserialize, Serialize};

macro_rules! opaque_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(uuid::Uuid);

        impl $name {
            /// Gera um novo identificador (UUID v7).
            pub fn new() -> Self {
                Self(uuid::Uuid::now_v7())
            }

            /// Constrói a partir de um UUID existente.
            pub fn from_uuid(id: uuid::Uuid) -> Self {
                Self(id)
            }
        }
    };
}

opaque_id!(
    /// Identidade de uma execução (run) do organismo.
    RunId
);
opaque_id!(
    /// Identidade de um passo lógico.
    StepId
);
opaque_id!(
    /// Identidade de um cluster (L1).
    ClusterId
);
opaque_id!(
    /// Identidade de um tecido (L2).
    TissueId
);
opaque_id!(
    /// Identidade de um conceito (L3).
    ConceptId
);
opaque_id!(
    /// Identidade de uma decisão (L4).
    DecisionId
);
opaque_id!(
    /// Identidade de um episódio de memória.
    EpisodeId
);
opaque_id!(
    /// Identidade de um evento.
    EventId
);
opaque_id!(
    /// Identidade de um módulo.
    ModuleId
);
