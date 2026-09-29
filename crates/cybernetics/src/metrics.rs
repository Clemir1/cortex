//! Snapshot de métricas cibernéticas publicado em `cybernetics.metrics`.

use triad_foundation as tf;

use tracing::trace;

use crate::homeostat::Homeostat;
use crate::stability::OscillationMonitor;

/// Métricas cibernéticas de um passo: homeostatos e osciladores auditados.
#[derive(Debug, Clone)]
pub struct CyberneticMetrics {
    pub step: tf::StepId,
    pub satisfied: Vec<String>,
    pub unsatisfied: Vec<String>,
    pub oscillating: Vec<String>,
}

/// Coleta o estado dos homeostatos e osciladores nomeados em um snapshot.
pub fn snapshot(
    step: tf::StepId,
    homeostats: &[(String, &Homeostat)],
    oscillators: &[(String, &OscillationMonitor)],
) -> CyberneticMetrics {
    let mut satisfied = Vec::new();
    let mut unsatisfied = Vec::new();
    for (name, homeostat) in homeostats {
        if homeostat.is_satisfied() {
            satisfied.push(name.clone());
        } else {
            unsatisfied.push(name.clone());
        }
    }
    let mut oscillating = Vec::new();
    for (name, monitor) in oscillators {
        if monitor.is_oscillating() {
            oscillating.push(name.clone());
        }
    }
    trace!(
        satisfeitos = satisfied.len(),
        insatisfeitos = unsatisfied.len(),
        oscilando = oscillating.len(),
        "snapshot coletado"
    );
    CyberneticMetrics {
        step,
        satisfied,
        unsatisfied,
        oscillating,
    }
}
