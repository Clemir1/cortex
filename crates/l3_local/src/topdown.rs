//! TopDownFeedback L3→L2 (17.4): sinais tipados emitidos PELO L3 a
//! partir do CAMPO DE ATENÇÃO REAL (focos ativos) — quem decide o
//! tecido-alvo é o ROUTER do L2 (o L3 não conhece a mesoestrutura;
//! camadas falam por contrato, não por intimidade).

use triad_foundation as tf;

/// Sinal top-down: um conceito em foco pede realce downstream.
#[derive(Debug, Clone, PartialEq)]
pub struct TopDownSignal {
    /// Conceito em foco (campo de atenção do L3).
    pub concept: tf::id::ConceptId,
    /// Intensidade do realce pedida (decay posicional do foco:
    /// 1/(posição+1) — determinístico).
    pub intensity: f32,
    /// Razão EXPLÍCITA (nunca sinal mudo — o legado morria em 16
    /// recebidos/0 admitidos sem reason).
    pub reason: String,
}

/// Decaimento posicional dos focos: foco n tem 1/(n+1).
pub fn focus_decay(position: usize) -> f32 {
    1.0 / (position as f32 + 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decay_posicional_e_deterministico() {
        assert!((focus_decay(0) - 1.0).abs() < 1e-9);
        assert!((focus_decay(1) - 0.5).abs() < 1e-9);
        assert!((focus_decay(3) - 0.25).abs() < 1e-9);
        assert_eq!(focus_decay(2), focus_decay(2), "A/A");
    }
}
