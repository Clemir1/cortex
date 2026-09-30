//! T/telemetry (16.9): [telemetry] INJETADA com consumidor real —
//! auditoria da escada de evidência E0–E5 com recibos denominados
//! e gaps tipados (nunca promoção automática). Inclui as STRUCTS de
//! `[crisis]` VALIDADAS (Lei 4 congelada: preserve inegociável).

pub mod config;
pub mod crisis_cfg;
pub mod ledger;
pub mod module;

pub use config::{target_for, HeavyCfg, ScientificCfg, TelemetryCfg};
pub use crisis_cfg::{
    CrisisCfg, CrisisCfgError, CrisisEffectsCfg, CrisisMachineCfg,
    CrisisThresholdsCfg, CRISIS_STATES,
};
pub use ledger::{EvidenceLedger, EvidenceReceipt};
pub use module::{TelemetryModule, TelemetryStats, TelemetryStatus};
