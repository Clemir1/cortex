//! Ponte do resumo L1 para a visão de tecido L2.

use triad_foundation as tf;
use triad_l1_substrate as l1;
use triad_runtime as rt;

/// Visão de tecido derivada do resumo do substrato L1.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TissueView {
    pub tissue_id: tf::TissueId,
    pub population: usize,
    pub mean_energy: f64,
    pub active_fraction: f64,
    pub source_step: u64,
}

/// Lê `l1.substrate.summary` do contexto e projeta em `TissueView`.
/// Lei: ausência ≠ zero — propaga NO_DATA, nunca fabrica visão zerada.
pub fn view(ctx: &rt::TypedContext, tissue_id: tf::TissueId) -> tf::Qualified<TissueView> {
    let q = ctx.get_qualified::<l1::SubstrateSummary>("l1.substrate.summary");
    if q.is_value() {
        let summary = q.as_ref_value().unwrap().clone();
        tf::Qualified::value(
            TissueView {
                tissue_id,
                population: summary.population,
                mean_energy: summary.mean_energy,
                active_fraction: if summary.population == 0 {
                    0.0
                } else {
                    summary.active as f64 / summary.population as f64
                },
                source_step: ctx.clock().tick,
            },
            tf::ModuleId::new(),
            q.step,
        )
    } else {
        tf::Qualified::no_data("l1.substrate.summary ausente", tf::ModuleId::new(), q.step)
    }
}
