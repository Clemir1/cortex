//! Critérios de despertar metacognitivo a partir da dormência.

/// Motivo pelo qual a metacognição deve despertar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeReason {
    /// Atividade baixa por tempo prolongado.
    LowActivity,
    /// Stress alto exige regulação.
    HighStress,
    /// Dormência excessiva, com a contagem de passos.
    DormantTooLong(u64),
    /// Despertar agendado.
    Scheduled,
}

/// Avalia se a metacognição deve despertar; dormência extrema vence.
pub fn should_wake(dormant_steps: u64, stress: f32, _activity_rate: f32) -> Option<WakeReason> {
    if dormant_steps > 100 {
        Some(WakeReason::DormantTooLong(dormant_steps))
    } else if stress > 0.8 {
        Some(WakeReason::HighStress)
    } else if dormant_steps > 20 {
        Some(WakeReason::LowActivity)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precedencia_dormencia_extrema_e_limiares() {
        // Dormência extrema vence mesmo com stress alto.
        assert_eq!(
            should_wake(101, 0.99, 1.0),
            Some(WakeReason::DormantTooLong(101))
        );
        assert_eq!(should_wake(0, 0.9, 1.0), Some(WakeReason::HighStress));
        assert_eq!(should_wake(21, 0.1, 1.0), Some(WakeReason::LowActivity));
        assert_eq!(should_wake(10, 0.1, 0.0), None);
    }
}
