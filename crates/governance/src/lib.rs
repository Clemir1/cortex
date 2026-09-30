//! Governança: leis duras, leis suaves, arbitragem, federação e ecologia.

pub mod arbitration;
pub mod ecology;
pub mod federation;
pub mod hard_law;
pub mod soft_law;

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
pub use hard_law::{HardLawSet, LawCheck, Verdict};
pub use soft_law::{SoftLaw, SoftLawBook};
