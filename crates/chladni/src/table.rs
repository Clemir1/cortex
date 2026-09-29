//! Tabela PIRT — frequências reais medidas em placas Chladni.
//!
//! Fonte: Physics Instructional Resource Team (legado `REAL_FREQUENCIES`).
//! O nome do padrão escolhido seleciona a fórmula de síntese trigonométrica
//! em `pattern.rs`. A coluna complexidade/descrição do legado é DESCARTADA
//! (a complexidade é estimada por fórmula em `pattern.rs`, não lida da
//! tabela). A placa "violin" do legado é descartada (duplicava os valores
//! da square — legado/histórico).

/// Placa física de referência.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlateType {
    Circular,
    Square,
}

impl PlateType {
    /// Nome canônico (igual às chaves do legado).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Circular => "circular",
            Self::Square => "square",
        }
    }
}

/// Entrada PIRT: frequência medida (Hz) + nome canônico do padrão.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PirtEntry {
    pub frequency: f64,
    pub name: &'static str,
}

/// 20 padrões medidos em placa CIRCULAR (legado, na ordem do dicionário).
pub const CIRCULAR: [PirtEntry; 20] = [
    PirtEntry { frequency: 180.7, name: "single_inner_ring" },
    PirtEntry { frequency: 186.0, name: "circular_islands" },
    PirtEntry { frequency: 246.0, name: "single_outer_ring" },
    PirtEntry { frequency: 322.0, name: "outer_ring_inner_island" },
    PirtEntry { frequency: 440.0, name: "double_ring" },
    PirtEntry { frequency: 540.0, name: "double_ring_outer_double" },
    PirtEntry { frequency: 761.7, name: "sunburst" },
    PirtEntry { frequency: 924.1, name: "triple_ring" },
    PirtEntry { frequency: 991.2, name: "plus_symbol" },
    PirtEntry { frequency: 1010.0, name: "triple_ring_curvy" },
    PirtEntry { frequency: 1135.0, name: "radiation_baseball" },
    PirtEntry { frequency: 1646.7, name: "asterisk" },
    PirtEntry { frequency: 1742.4, name: "quadruple_ring" },
    PirtEntry { frequency: 1901.1, name: "sundial" },
    PirtEntry { frequency: 2063.6, name: "crosshairs" },
    PirtEntry { frequency: 2794.4, name: "quintuple_ring" },
    PirtEntry { frequency: 3261.6, name: "crosshairs_extra" },
    PirtEntry { frequency: 3914.2, name: "triple_ring_closed" },
    PirtEntry { frequency: 4540.6, name: "spider_web" },
    PirtEntry { frequency: 5959.2, name: "sixtuple_ring" },
];

/// 20 padrões medidos em placa QUADRADA (legado, na ordem do dicionário).
pub const SQUARE: [PirtEntry; 20] = [
    PirtEntry { frequency: 116.1, name: "simple_line" },
    PirtEntry { frequency: 439.6, name: "grid_basic" },
    PirtEntry { frequency: 463.9, name: "diagonal_pattern" },
    PirtEntry { frequency: 510.6, name: "complex_grid" },
    PirtEntry { frequency: 590.24, name: "double_pattern" },
    PirtEntry { frequency: 612.3, name: "star_pattern" },
    PirtEntry { frequency: 627.2, name: "symmetric_complex" },
    PirtEntry { frequency: 1002.7, name: "fractal_basic" },
    PirtEntry { frequency: 1255.2, name: "fractal_complex" },
    PirtEntry { frequency: 1655.5, name: "neural_basic" },
    PirtEntry { frequency: 1729.8, name: "neural_complex" },
    PirtEntry { frequency: 2032.0, name: "hyper_symmetric" },
    PirtEntry { frequency: 2120.5, name: "multi_center" },
    PirtEntry { frequency: 2157.6, name: "complex_web" },
    PirtEntry { frequency: 2208.6, name: "dense_network" },
    PirtEntry { frequency: 2282.8, name: "ultra_complex_1" },
    PirtEntry { frequency: 2478.8, name: "ultra_complex_2" },
    PirtEntry { frequency: 2494.4, name: "fractal_ultra" },
    PirtEntry { frequency: 2662.5, name: "max_complexity" },
    PirtEntry { frequency: 2761.5, name: "hypersymmetric" },
];

/// Entrada de MENOR |Δf| na placa (empate: primeira na ordem da tabela,
/// como na iteração do legado). Tabelas são não-vazias; o "unknown" só
/// existe como defesa (fórmula default em `pattern.rs`).
pub fn nearest(plate: PlateType, frequency: f64) -> PirtEntry {
    let table: &[PirtEntry] = match plate {
        PlateType::Circular => &CIRCULAR,
        PlateType::Square => &SQUARE,
    };
    let mut best = PirtEntry { frequency: 0.0, name: "unknown" };
    let mut best_delta = f64::INFINITY;
    for entry in table {
        let delta = (entry.frequency - frequency).abs();
        if delta < best_delta {
            best_delta = delta;
            best = *entry;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seleciona_padrao_pela_menor_delta() {
        assert_eq!(nearest(PlateType::Circular, 440.0).name, "double_ring");
        // 200 está mais perto de 186.0 (Δ14) do que de 180.7 (Δ19.3).
        assert_eq!(nearest(PlateType::Circular, 200.0).name, "circular_islands");
        assert_eq!(nearest(PlateType::Square, 439.6).name, "grid_basic");
        assert_eq!(nearest(PlateType::Circular, 5000.0).name, "spider_web");
    }

    #[test]
    fn tabelas_tem_20_entradas_com_nome() {
        assert_eq!(CIRCULAR.len(), 20);
        assert_eq!(SQUARE.len(), 20);
        for e in CIRCULAR.iter().chain(SQUARE.iter()) {
            assert!(!e.name.is_empty());
            assert!(e.frequency.is_finite() && e.frequency > 0.0);
        }
    }

    #[test]
    fn placa_tem_nome_canonico() {
        assert_eq!(PlateType::Circular.as_str(), "circular");
        assert_eq!(PlateType::Square.as_str(), "square");
    }
}
