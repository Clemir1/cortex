//! Governança: leis duras, leis suaves, arbitragem, federação e ecologia.

pub mod arbitration;
pub mod ecology;
pub mod federation;
// 17.11 (CAMADA.txt:1549) — as LEIS da casa em laws/ (hard laws
// como dado, LawEngine observacional, livro de soft laws).
pub mod laws {
    pub mod hard_law;
    pub mod law_engine;
    pub mod soft_law;
}

pub use arbitration::{
    Arbiter, ArbitrationEngine, ArbitrationRecord, ArbitrationStats, Contender, LossReason, Tie,
};
pub use ecology::{EcologyEngine, NicheHealth};
pub use ecology::{
    CrossFeedRecord, EcoCensus, EcologyMotor, EcologyPolicy, PopulationSource, Species,
    SpeciesObs, TransferStatus,
};
pub use federation::{
    CnpPolicy, CnpReject, ControlKind, ControlSignal, EffectSink, Federation, FederationEngine,
    FederationMember, FederationPhase, FederationState, FederationTransport, InProcessTransport,
    PendingOutcome, Resource, RevertReason, TradeRecord, TradeStatus,
};
pub use laws::hard_law::{HardLawSet, LawEntry, Verdict, Violation};
// 17.11 — LawEngine (sessão 7): leis da casa como DADO, enforcement
// observacional pós-tick com denominadores, soft law de orçamento.
pub use laws::law_engine::{LawAudit, LawEngine, LawObservation, LawStats};
pub use laws::soft_law::{SoftLaw, SoftLawBook};
