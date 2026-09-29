//! Síntese de padrões Chladni — matemática pura portada do legado
//! `core/chladni_frequencies.py` (geração, features, complexidade,
//! hashes) e da integração de atenção `attention_system.py`
//! (`state_features`/`compare_features`). Sem estado, sem cache: quem
//! orquestra é `system.rs`.
//!
//! Nota honesta: o legado NÃO usa modos físicos (m,n) de placa — são
//! sínteses trigonométricas escolhidas pelo NOME do padrão PIRT mais
//! próximo da frequência. Este módulo replica essa mecânica.

use crate::bands::{classify, CognitiveBand};
use crate::table::{nearest, PlateType};

/// Features nomeadas — de um padrão (quadrantes da grade 2D) ou de um
/// estado 1D (quartos contíguos). A forma é a mesma; a semântica muda
/// (o legado compara as duas diretamente, sem alinhamento dimensional).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PatternFeatures {
    pub radial: f64,
    pub angular: f64,
    pub diagonal: f64,
    pub fractal: f64,
}

/// Padrão Chladni gerado — grade N×N flat (row-major) + descritor.
#[derive(Debug, Clone)]
pub struct Pattern {
    pub grid: Vec<f64>,
    pub size: usize,
    pub frequency: f64,
    pub plate: PlateType,
    pub band: CognitiveBand,
    pub pattern_name: &'static str,
    /// Complexidade estimada, truncada para inteiro (legado `int()`).
    pub complexity: f64,
    pub features: PatternFeatures,
    /// FNV-1a 64 sobre os bytes da grade (row-major, `f64::to_ne_bytes`).
    pub content_hash: u64,
    /// FNV-1a 64 sobre a string canônica do descritor (versionada).
    pub descriptor_hash: u64,
}

/// Gera o padrão da frequência na placa — ver módulo doc para a origem
/// das fórmulas. `grid_size` < 2 vira 2.
pub fn generate(frequency: f64, plate: PlateType, grid_size: usize, descriptor_version: u32) -> Pattern {
    let n = grid_size.max(2);
    // linspace(−1, 1, N) com extremos (passo 2/(N−1)).
    let step = 2.0 / (n - 1) as f64;

    let entry = nearest(plate, frequency);
    let k = frequency / 100.0;

    let mut grid = Vec::with_capacity(n * n);
    for i in 0..n {
        let y = -1.0 + i as f64 * step;
        for j in 0..n {
            let x = -1.0 + j as f64 * step;
            let r = (x * x + y * y).sqrt();
            let theta = y.atan2(x);
            let p = synthesize(entry.name, k, x, y, r, theta, plate);
            grid.push(p);
        }
    }

    let band = classify(frequency);
    let features = extract_features(&grid, n);
    let complexity = estimate_complexity(frequency, band);

    // Hash de conteúdo: bytes das células em ordem row-major.
    let mut bytes = Vec::with_capacity(grid.len() * 8);
    for cell in &grid {
        bytes.extend_from_slice(&cell.to_ne_bytes());
    }
    let content_hash = fnv1a64(&bytes);

    // Descritor versionado (reimplementação Rust-native; determinística,
    // não byte-compatível com o hash de evidência do legado).
    let descriptor = format!(
        "chladni_frequencies|{}:{:.12}|{}|{}|radial={:.12}|angular={:.12}|diagonal={:.12}|fractal={:.12}|content={}|descriptor_version={}",
        plate.as_str(),
        frequency,
        n,
        band.as_str(),
        features.radial,
        features.angular,
        features.diagonal,
        features.fractal,
        content_hash,
        descriptor_version
    );
    let descriptor_hash = fnv1a64(descriptor.as_bytes());

    Pattern {
        grid,
        size: n,
        frequency,
        plate,
        band,
        pattern_name: entry.name,
        complexity,
        features,
        content_hash,
        descriptor_hash,
    }
}

/// Fórmula por substring do nome do padrão PIRT — ordem de checagem do
/// legado (para no primeiro match). `k = f/100`; R/Θ/X/Y da célula.
fn synthesize(name: &str, k: f64, x: f64, y: f64, r: f64, theta: f64, plate: PlateType) -> f64 {
    let p = if name.contains("ring") {
        let n = extract_ring_number(name) as f64;
        (n * k * r).sin() * (k * r).cos()
    } else if name.contains("sunburst") {
        (k * r + 6.0 * theta).sin().abs()
    } else if name.contains("crosshairs") {
        ((k * x).sin() * (k * y).sin()).abs()
    } else if name.contains("asterisk") {
        ((k * r).sin() * (4.0 * theta).sin()).abs()
    } else if name.contains("plus") || name.contains("sundial") {
        ((k * x).sin() + (k * y).sin()).abs()
    } else if name.contains("spider_web") {
        ((k * r).sin() + (8.0 * theta).sin()).abs()
    } else if name.contains("radiation") || name.contains("baseball") {
        ((k * r).sin() * (3.0 * theta).cos()).abs()
    } else if name.contains("island") {
        (k * r).sin() * (0.5 * k * r).cos()
    } else if name.contains("grid") || name.contains("network") {
        (k * x).sin() * (k * y).sin()
    } else {
        (2.0 * k * r).sin() * (k * theta).cos()
    };
    // Janela radial: apenas a placa quadrada (legado).
    match plate {
        PlateType::Square => p * (1.0 - r * r),
        PlateType::Circular => p,
    }
}

/// Número do anel a partir do nome (legado `_extract_number`): primeiro
/// run de dígitos; sem dígitos, palavras-chave; quirk legado mantido
/// ("quadruple_ring" → 1, não há dígito nem palavra-chave).
pub fn extract_ring_number(name: &str) -> u32 {
    let bytes = name.as_bytes();
    let mut i = 0;
    while i < bytes.len() && !bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i < bytes.len() {
        let mut j = i;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        // Infallível: run só tem dígitos ASCII.
        return name[i..j].parse().unwrap_or(1);
    }
    if name.contains("single") {
        1
    } else if name.contains("double") {
        2
    } else if name.contains("triple") {
        3
    } else {
        1
    }
}

/// Complexidade estimada (legado `_estimate_complexity`): escala contígua
/// 1–10 em [80, 4000]; f<80 pode dar 0 (comportamento legado, mantido).
/// NaN/Inf → 1.0. Trunca para inteiro (legado `int()`).
pub fn estimate_complexity(frequency: f64, band: CognitiveBand) -> f64 {
    if !frequency.is_finite() {
        return 1.0;
    }
    let f = frequency.clamp(10.0, 4000.0);
    let raw = match band {
        CognitiveBand::Stability => 1.0 + (f - 80.0) / 170.0 * 3.0,
        CognitiveBand::Memory => 4.0 + (f - 250.0) / 450.0 * 3.0,
        CognitiveBand::Creativity => 7.0 + (f - 700.0) / 800.0 * 2.0,
        CognitiveBand::MetaOrganization => 9.0 + (f - 1500.0) / 2500.0,
    };
    raw.trunc()
}

/// Features da grade N×N: média |·| por quadrante (divisão inteira).
/// Nomes históricos do legado: radial=Q1 (sup. esq.), angular=Q2 (sup.
/// dir.), diagonal=Q3 (inf. esq.), fractal=Q4 (inf. dir.).
pub fn extract_features(grid: &[f64], size: usize) -> PatternFeatures {
    let half = size / 2;
    let mut sums = [0.0f64; 4];
    let mut counts = [0u64; 4];
    for i in 0..size {
        for j in 0..size {
            let v = grid[i * size + j].abs();
            let q = if i < half {
                if j < half { 0 } else { 1 }
            } else if j < half {
                2
            } else {
                3
            };
            sums[q] += v;
            counts[q] += 1;
        }
    }
    let mean = |q: usize| {
        if counts[q] == 0 {
            0.0
        } else {
            sums[q] / counts[q] as f64
        }
    };
    PatternFeatures {
        radial: mean(0),
        angular: mean(1),
        diagonal: mean(2),
        fractal: mean(3),
    }
}

/// Features de um ESTADO 1D (legado attention `_extract_pattern_features`):
/// quartos CONTÍGUOS por divisão inteira; o último absorve o resto
/// (ex. 97D → 24/24/24/25). Entrada JÁ L2-normalizada pelo chamador.
/// `len < 4` → 4 features 0.0.
pub fn state_features(state: &[f64]) -> PatternFeatures {
    if state.len() < 4 {
        return PatternFeatures {
            radial: 0.0,
            angular: 0.0,
            diagonal: 0.0,
            fractal: 0.0,
        };
    }
    let q = state.len() / 4;
    let mean_abs = |slice: &[f64]| {
        if slice.is_empty() {
            0.0
        } else {
            slice.iter().map(|v| v.abs()).sum::<f64>() / slice.len() as f64
        }
    };
    PatternFeatures {
        radial: mean_abs(&state[0..q]),
        angular: mean_abs(&state[q..2 * q]),
        diagonal: mean_abs(&state[2 * q..3 * q]),
        fractal: mean_abs(&state[3 * q..]),
    }
}

/// Comparação do legado (`_compare_patterns`): média de (1 − |Δ| com Δ
/// clampado a 1) sobre as 4 chaves fixas. Denominador FIXO 4.0 — campo
/// ausente contribui 0 mas conta no denominador (aqui a struct sempre
/// tem os 4, então o clamp é o único guard). Iguais → 1.0; diffs ≥ 1 →
/// 0.0; saída sempre [0,1]. O clamp existe porque features de PADRÃO
/// podem exceder 1 (fórmulas com soma ≤ 2); as de estado ficam ≤ 1.
pub fn compare_features(a: &PatternFeatures, b: &PatternFeatures) -> f64 {
    let term = |x: f64, y: f64| 1.0 - (x - y).abs().min(1.0);
    (term(a.radial, b.radial)
        + term(a.angular, b.angular)
        + term(a.diagonal, b.diagonal)
        + term(a.fractal, b.fractal))
        / 4.0
}

/// Redimensionamento bilinear (ordem 1) de uma grade `from×from` para
/// `to×to` (legado usava zoom bilinear para comparar padrões de grades
/// distintas). `from == to` → clone; `from == 1` → replica o pixel.
pub fn resize_bilinear(grid: &[f64], from: usize, to: usize) -> Vec<f64> {
    if grid.is_empty() || to == 0 {
        return vec![0.0; to * to];
    }
    if from == to {
        return grid.to_vec();
    }
    if from == 1 {
        return vec![grid[0]; to * to];
    }
    let mut out = Vec::with_capacity(to * to);
    let scale = (from - 1) as f64 / (to - 1) as f64;
    for r in 0..to {
        let sy = r as f64 * scale;
        let y0 = sy.floor() as usize;
        let y1 = (y0 + 1).min(from - 1);
        let ty = sy - y0 as f64;
        for c in 0..to {
            let sx = c as f64 * scale;
            let x0 = sx.floor() as usize;
            let x1 = (x0 + 1).min(from - 1);
            let tx = sx - x0 as f64;
            let top = grid[y0 * from + x0] * (1.0 - tx) + grid[y0 * from + x1] * tx;
            let bottom = grid[y1 * from + x0] * (1.0 - tx) + grid[y1 * from + x1] * tx;
            out.push(top * (1.0 - ty) + bottom * ty);
        }
    }
    out
}

/// Similaridade por cosseno com clamps do legado: norma ~zero → 0.0;
/// correlação negativa clampada a 0. Entradas com mesmo comprimento
/// (o chamador garante — use `resize_bilinear` antes se precisar).
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let n1: f64 = a.iter().map(|v| v * v).sum();
    let n2: f64 = b.iter().map(|v| v * v).sum();
    if n1 < 1e-10 || n2 < 1e-10 {
        return 0.0;
    }
    let dot: f64 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    (dot / (n1 * n2).sqrt()).clamp(0.0, 1.0)
}

/// FNV-1a 64 — hash determinístico, sem deps.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_e_deterministico() {
        let a = generate(440.0, PlateType::Circular, 16, 1);
        let b = generate(440.0, PlateType::Circular, 16, 1);
        assert_eq!(a.content_hash, b.content_hash);
        assert_eq!(a.descriptor_hash, b.descriptor_hash);
        assert_eq!(a.size, 16);
        assert_eq!(a.grid.len(), 16 * 16);
        assert!(a.grid.iter().all(|v| v.is_finite()));
        assert_eq!(a.pattern_name, "double_ring");
        assert_eq!(a.band, CognitiveBand::Memory);
    }

    #[test]
    fn placas_diferentes_hash_diferente() {
        let c = generate(116.1, PlateType::Circular, 16, 1);
        let s = generate(116.1, PlateType::Square, 16, 1);
        assert_ne!(c.content_hash, s.content_hash);
    }

    #[test]
    fn grade_minima_e_dois() {
        let p = generate(250.0, PlateType::Circular, 0, 1);
        assert_eq!(p.size, 2);
    }

    #[test]
    fn numero_de_anel_do_nome() {
        assert_eq!(extract_ring_number("single_inner_ring"), 1);
        assert_eq!(extract_ring_number("double_ring"), 2);
        assert_eq!(extract_ring_number("triple_ring_curvy"), 3);
        assert_eq!(extract_ring_number("quadruple_ring"), 1); // quirk legado
        assert_eq!(extract_ring_number("ultra_complex_1"), 1);
        assert_eq!(extract_ring_number("spider_web"), 1);
    }

    #[test]
    fn complexidade_por_banda() {
        assert_eq!(estimate_complexity(f64::NAN, CognitiveBand::Stability), 1.0);
        assert_eq!(estimate_complexity(80.0, CognitiveBand::Stability), 1.0);
        assert_eq!(estimate_complexity(250.0, CognitiveBand::Stability), 4.0);
        assert_eq!(estimate_complexity(250.0, CognitiveBand::Memory), 4.0);
        assert_eq!(estimate_complexity(1500.0, CognitiveBand::MetaOrganization), 9.0);
        assert_eq!(estimate_complexity(4000.0, CognitiveBand::MetaOrganization), 10.0);
        assert_eq!(estimate_complexity(10.0, CognitiveBand::Stability), 0.0);
    }

    #[test]
    fn features_por_quadrante() {
        let f = extract_features(&[1.0, 2.0, 3.0, 4.0], 2);
        assert_eq!(f.radial, 1.0);
        assert_eq!(f.angular, 2.0);
        assert_eq!(f.diagonal, 3.0);
        assert_eq!(f.fractal, 4.0);
    }

    #[test]
    fn features_de_estado_por_quartos() {
        let z = state_features(&[0.0; 97]);
        assert_eq!(z, PatternFeatures { radial: 0.0, angular: 0.0, diagonal: 0.0, fractal: 0.0 });

        let v = state_features(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
        assert_eq!(v.radial, 1.5);
        assert_eq!(v.angular, 3.5);
        assert_eq!(v.diagonal, 5.5);
        assert_eq!(v.fractal, 7.5);

        let curto = state_features(&[-1.0, -1.0]);
        assert_eq!(curto, PatternFeatures { radial: 0.0, angular: 0.0, diagonal: 0.0, fractal: 0.0 });

        // q=1: o último quarto absorve o resto (2 elementos).
        let assim = state_features(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(assim.radial, 1.0);
        assert_eq!(assim.angular, 2.0);
        assert_eq!(assim.diagonal, 3.0);
        assert_eq!(assim.fractal, 4.5);
    }

    #[test]
    fn comparacao_de_features_legado() {
        let z = PatternFeatures { radial: 0.0, angular: 0.0, diagonal: 0.0, fractal: 0.0 };
        let um = PatternFeatures { radial: 1.0, angular: 1.0, diagonal: 1.0, fractal: 1.0 };
        let meio = PatternFeatures { radial: 0.5, angular: 0.5, diagonal: 0.5, fractal: 0.5 };

        assert_eq!(compare_features(&z, &z), 1.0);
        assert_eq!(compare_features(&z, &um), 0.0);
        assert!((compare_features(&z, &meio) - 0.5).abs() < 1e-12);

        // Campo com diff ≥ 1 (clamp) zera o termo mas mantém denominador 4.
        let quase = PatternFeatures { radial: 0.0, angular: 0.0, diagonal: 0.0, fractal: 2.0 };
        assert!((compare_features(&z, &quase) - 0.75).abs() < 1e-12);
    }

    #[test]
    fn resize_bilinear_2x2_para_3x3() {
        let out = resize_bilinear(&[0.0, 1.0, 2.0, 3.0], 2, 3);
        let esperado = [0.0, 0.5, 1.0, 1.0, 1.5, 2.0, 2.0, 2.5, 3.0];
        for (o, e) in out.iter().zip(esperado) {
            assert!((o - e).abs() < 1e-12, "{o} vs {e}");
        }
    }

    #[test]
    fn cosseno_com_clamps_legado() {
        assert!((cosine_similarity(&[1.0, 2.0], &[1.0, 2.0]) - 1.0).abs() < 1e-12);
        assert_eq!(cosine_similarity(&[1.0, 0.0, 0.0, 1.0], &[0.0, 1.0, 1.0, 0.0]), 0.0);
        assert_eq!(cosine_similarity(&[1.0, 1.0], &[-1.0, -1.0]), 0.0);
        assert_eq!(cosine_similarity(&[0.0, 0.0], &[1.0, 1.0]), 0.0);
    }

    #[test]
    fn fnv_conhecido() {
        // Vetor de teste padrão do FNV-1a 64: hash de "a" e de "foobar".
        assert_eq!(fnv1a64(b"a"), 0xaf63dc4c8601ec8c);
        assert_eq!(fnv1a64(b"foobar"), 0x85944171f73967e8);
    }
}
