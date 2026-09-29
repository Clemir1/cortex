//! Governança: leis duras, leis suaves, arbitragem, federação e ecologia.

pub mod arbitration;
pub mod ecology;
pub mod federation;
pub mod hard_law;
pub mod soft_law;

pub use arbitration::{Arbiter, Tie};
pub use ecology::{EcologyEngine, NicheHealth};
pub use federation::{Federation, FederationPhase, FederationState};
pub use hard_law::{HardLawSet, LawCheck, Verdict};
pub use soft_law::{SoftLaw, SoftLawBook};
