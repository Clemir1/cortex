//! Bandas cognitivas Chladni — classificação por frequência e por entropia.
//!
//! Legado: `COGNITIVE_BANDS`/`_entropy_to_band` de `chladni_frequencies.py`.
//! Quatro bandas em Hz, limites inclusivos: stability 80–250, memory
//! 250–700, creativity 700–1500, meta_organization 1500–4000.

/// Banda cognitiva de uma frequência (ou de um estado, via entropia).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CognitiveBand {
    Stability,
    Memory,
    Creativity,
    MetaOrganization,
}

impl CognitiveBand {
    /// Nome canônico (igual às chaves do legado).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stability => "stability",
            Self::Memory => "memory",
            Self::Creativity => "creativity",
            Self::MetaOrganization => "meta_organization",
        }
    }

    /// Intervalo da banda em Hz — `(fmin, fmax)`, limites inclusivos.
    pub const fn range(self) -> (f64, f64) {
        match self {
            Self::Stability => (80.0, 250.0),
            Self::Memory => (250.0, 700.0),
            Self::Creativity => (700.0, 1500.0),
            Self::MetaOrganization => (1500.0, 4000.0),
        }
    }
}

/// Classifica por frequência. Regras do legado: bordas compartilhadas ficam
/// com a PRIMEIRA banda na ordem de inserção (250→stability, 700→memory,
/// 1500→creativity); fora de [80, 4000] (inclui NaN) → fallback stability.
pub fn classify(frequency: f64) -> CognitiveBand {
    if !(80.0..=4000.0).contains(&frequency) {
        return CognitiveBand::Stability;
    }
    if frequency <= 250.0 {
        CognitiveBand::Stability
    } else if frequency <= 700.0 {
        CognitiveBand::Memory
    } else if frequency <= 1500.0 {
        CognitiveBand::Creativity
    } else {
        CognitiveBand::MetaOrganization
    }
}

/// Banda por entropia `h` (legado `_entropy_to_band`):
/// h<0.25→stability; h<0.5→memory; h<0.75→creativity; senão meta.
pub fn entropy_to_band(h: f64) -> CognitiveBand {
    if h < 0.25 {
        CognitiveBand::Stability
    } else if h < 0.5 {
        CognitiveBand::Memory
    } else if h < 0.75 {
        CognitiveBand::Creativity
    } else {
        CognitiveBand::MetaOrganization
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bordas_compartilhadas_ficam_com_a_primeira_banda() {
        assert_eq!(classify(250.0), CognitiveBand::Stability);
        assert_eq!(classify(700.0), CognitiveBand::Memory);
        assert_eq!(classify(1500.0), CognitiveBand::Creativity);
    }

    #[test]
    fn classifica_faixas_interiores() {
        assert_eq!(classify(80.0), CognitiveBand::Stability);
        assert_eq!(classify(250.1), CognitiveBand::Memory);
        assert_eq!(classify(700.1), CognitiveBand::Creativity);
        assert_eq!(classify(1500.1), CognitiveBand::MetaOrganization);
        assert_eq!(classify(4000.0), CognitiveBand::MetaOrganization);
    }

    #[test]
    fn fora_do_dominio_fallback_stability() {
        assert_eq!(classify(79.9), CognitiveBand::Stability);
        assert_eq!(classify(4000.1), CognitiveBand::Stability);
        assert_eq!(classify(-1.0), CognitiveBand::Stability);
        assert_eq!(classify(f64::NAN), CognitiveBand::Stability);
    }

    #[test]
    fn entropia_mapeia_banda() {
        assert_eq!(entropy_to_band(0.0), CognitiveBand::Stability);
        assert_eq!(entropy_to_band(0.24), CognitiveBand::Stability);
        assert_eq!(entropy_to_band(0.25), CognitiveBand::Memory);
        assert_eq!(entropy_to_band(0.49), CognitiveBand::Memory);
        assert_eq!(entropy_to_band(0.5), CognitiveBand::Creativity);
        assert_eq!(entropy_to_band(0.74), CognitiveBand::Creativity);
        assert_eq!(entropy_to_band(0.75), CognitiveBand::MetaOrganization);
        assert_eq!(entropy_to_band(1.0), CognitiveBand::MetaOrganization);
    }

    #[test]
    fn intervalos_de_banda() {
        assert_eq!(CognitiveBand::Stability.range(), (80.0, 250.0));
        assert_eq!(CognitiveBand::Memory.range(), (250.0, 700.0));
        assert_eq!(CognitiveBand::Creativity.range(), (700.0, 1500.0));
        assert_eq!(CognitiveBand::MetaOrganization.range(), (1500.0, 4000.0));
    }
}
