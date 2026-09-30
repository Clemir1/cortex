//! # triad-l1-substrate — substrato computacional L1 do Triad_AEE
//!
//! L1 é o substrato biológico-computacional: clusters vivos, energia
//! homeostática (O1), atividade, estado 97D, lifecycle com transições
//! contadas, survival com histerese, plasticidade Hebbian REAL (ou
//! `Bypassed` com razão — nunca silêncio), grafo local incremental,
//! HOTM/Reservoir com readout, divisão/fusão/morte sob demanda, matriz
//! de estados versionada com hashes, ledger de 4 fases com recibos e
//! métricas por janelas 1/5/20/50/100. Custo por step: O(active).
//!
//! L1 **não possui** linguagem, identidade, World Model, metacognição,
//! tecidos (L2), ecologia semântica, federação ou leis.
//!
//! Módulos:
//! - [`config`] — constantes L1 (legado + aprimoramentos A1–A8);
//! - [`math`] — normal/entropia/delta determinísticos;
//! - [`cluster`] — `ClusterBio` 97D, lifecycle, bandas tau, firing;
//! - [`survival`] — dormancy/wake/repair com histerese contada (A3);
//! - [`energy`] — O1 fechado: setpoint, banda, atuação medida (A1);
//! - [`graph`] — spatial hash incremental (O(movidos));
//! - [`state_matrix`] — soma incremental, hashes, ordem canônica;
//! - [`reservoir`] — HOTM + 8 features + Hebbian mensurável;
//! - [`metrics`] — janelas 1/5/20/50/100 (telemetria pura);
//! - [`contract`] — ledger de 4 fases, versão monotônica, recibos E3→E4;
//! - [`runner`] — o step completo, determinístico (A/A bit-idêntico).

pub mod chladni_signal;
pub mod cluster;
pub mod config;
pub mod contract;
pub mod dynamics;
pub mod energy;
pub mod graph;
pub mod math;
pub mod metrics;
pub mod module;
pub mod reservoir;
pub mod runner;
pub mod soa;
pub mod state_matrix;
pub mod survival;

pub use chladni_signal::ChladniSignal;
pub use cluster::{ClusterBio, ClusterProfile, Lifecycle, LifecycleState};
pub use contract::L1Ledger;
pub use dynamics::{
    ErrGate, HarmonicDynamics, HotmDynamics, InhibitionReport, LocalDynamicsEngine,
    LocalDynamicsReport, LocalInhibition, LocalPredictionError, LpeReport, RecurrentDynamics,
};
pub use energy::EnergyBudget;
pub use graph::LocalGraph;
pub use metrics::L1Metrics;
pub use module::{ClusterModule, SubstrateSummary};
pub use reservoir::{HotmReservoir, PlasticityStatus, ReadoutResult};
pub use runner::{L1Runner, L1StepReport};
pub use soa::SampleSoA;
pub use state_matrix::ClusterStateMatrix;
pub use survival::{CollectiveEmergency, SurvivalConfig, SurvivalDecision, SurvivalState};
