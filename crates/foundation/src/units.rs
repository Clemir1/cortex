//! Unidades do organismo: grandezas explícitas com faixa validada.
//!
//! Unidades explícitas evitam comparações acidentais entre grandezas:
//! `Energy` e `Stress` são ambos [0,1], mas não são intercambiáveis.

use serde::{Deserialize, Serialize};
use serde::de::Error as _;

macro_rules! bounded_unit {
    ($(#[$doc:meta])* $name:ident, $min:expr, $max:expr) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Serialize)]
        pub struct $name(f32);

        impl $name {
            /// Constrói a unidade. `None` se NaN ou fora da faixa.
            pub fn construct(value: f32) -> Option<Self> {
                if value.is_nan() || value < $min || value > $max {
                    None
                } else {
                    Some(Self(value))
                }
            }

            /// Valor bruto em f32 (já validado).
            pub const fn value(self) -> f32 {
                self.0
            }
        }

        // Deserialização também valida: a faixa é parte do tipo.
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let raw = f32::deserialize(deserializer)?;
                Self::construct(raw)
                    .ok_or_else(|| D::Error::custom(concat!(stringify!($name), ": NaN ou fora da faixa")))
            }
        }
    };
}

bounded_unit!(
    /// Energia de cluster/tecido. Faixa [0, 1].
    Energy, 0.0, 1.0
);
bounded_unit!(
    /// Estresse do sistema. Faixa [0, 1].
    Stress, 0.0, 1.0
);
bounded_unit!(
    /// Saliência de um conteúdo. Faixa [0, 1].
    Salience, 0.0, 1.0
);
bounded_unit!(
    /// Confiança em um valor ou decisão. Faixa [0, 1].
    Confidence, 0.0, 1.0
);
bounded_unit!(
    /// Recompensa de um resultado. Faixa [-1, 1].
    Reward, -1.0, 1.0
);
bounded_unit!(
    /// Aversão a um resultado. Faixa [0, 1].
    Aversion, 0.0, 1.0
);

/// Taxa: nasce sempre de uma divisão com denominador conhecido (E1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Rate(f32);

impl Rate {
    /// Constrói a partir de uma razão já calculada. `None` se NaN ou negativa.
    pub fn construct(value: f32) -> Option<Self> {
        if value.is_nan() || value < 0.0 {
            None
        } else {
            Some(Self(value))
        }
    }

    /// Razão explícita com denominador. Denominador 0 → `None` (ausência ≠ zero).
    pub fn from_ratio(numerator: u64, denominator: u64) -> Option<Self> {
        if denominator == 0 {
            None
        } else {
            Self::construct(numerator as f32 / denominator as f32)
        }
    }

    /// Valor bruto em f32 (já validado).
    pub const fn value(self) -> f32 {
        self.0
    }
}

// Deserialização também valida: taxa negativa ou NaN não existe.
impl<'de> Deserialize<'de> for Rate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = f32::deserialize(deserializer)?;
        Rate::construct(raw).ok_or_else(|| D::Error::custom("Rate: NaN ou negativa"))
    }
}

#[cfg(test)]
mod tests {
    use triad_foundation::units::{Aversion, Confidence, Energy, Reward, Salience, Stress};

    #[test]
    fn aceita_limites_e_miolo() {
        assert_eq!(Energy::construct(0.0).unwrap().value(), 0.0);
        assert!(Energy::construct(1.0).is_some());
        assert!(Energy::construct(0.5).is_some());
        assert!(Stress::construct(0.9).is_some());
        assert!(Salience::construct(0.25).is_some());
        assert!(Confidence::construct(1.0).is_some());
        assert!(Reward::construct(-1.0).is_some());
        assert!(Reward::construct(0.0).is_some());
        assert!(Reward::construct(1.0).is_some());
        assert!(Aversion::construct(0.0).is_some());
    }

    #[test]
    fn rejeita_nan_e_fora_da_faixa() {
        assert!(Energy::construct(f32::NAN).is_none());
        assert!(Energy::construct(-0.01).is_none());
        assert!(Energy::construct(1.01).is_none());
        assert!(Reward::construct(1.5).is_none());
        assert!(Reward::construct(-1.5).is_none());
        assert!(Stress::construct(f32::NAN).is_none());
        assert!(Aversion::construct(2.0).is_none());
    }

    #[test]
    fn deserialize_valida_faixa() {
        let ok: Energy = serde_json::from_str("0.5").unwrap();
        assert_eq!(ok.value(), 0.5);
        assert!(serde_json::from_str::<Energy>("2.0").is_err());
        assert!(serde_json::from_str::<Reward>("\"alto\"").is_err());
    }
}
