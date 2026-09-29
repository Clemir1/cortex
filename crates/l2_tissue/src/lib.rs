//! L2: tecido — ponte do substrato (L1) para cognição (L3), adaptação
//! coordenada, roteamento de feedback e topologia de tecidos.

pub mod adaptation;
pub mod bridge;
pub mod config;
pub mod coordination;
pub mod feedback;
pub mod formation;
pub mod l2_ledger;
pub mod runner;
pub mod topology;
pub mod tissue;
pub mod tissue_module;

pub use adaptation::{AdaptParam, AdaptationGate, GateDecision};
pub use bridge::TissueView;
pub use config::{AdaptationCfg, AffinityCfg, L2Config, TissuesCfg};
pub use coordination::AdaptationCoordinator;
pub use feedback::{FeedbackRouter, FeedbackSignal};
pub use formation::{FormationParams, TissueChange, TissueFormation};
pub use l2_ledger::L2Ledger;
pub use runner::{L2Metrics, L2Runner, L2StepReport, L2Windows};
pub use topology::TopologyManager;
pub use tissue::{StateRegion, Tissue};
pub use tissue_module::TissueModule;
