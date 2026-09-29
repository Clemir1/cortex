//! 17.2 — SoA n×97 da AMOSTRA (layout de COMPUTE colunar; checklist
//! seção 17). O estado dos clusters continua canônico (AoS por
//! cluster, janela versionada); o que vira colunar é a AMOSTRA
//! estratificada que os kernels do reservoir consomem — os mesmos
//! números, na MESMA ordem de acumulação, agora prontos para o rayon
//! por coluna (17.3) com redução em ordem canônica.
//!
//! A/A BIT-IDÊNTICO por construção: cada kernel soma exatamente na
//! ordem do loop AoS original (i externo para soma por amostra, k
//! interno para por dimensão) — nenhum reagrupamento de floats, os
//! 8 features do readout saem bit-idênticos. Provado em teste com
//! `assert_eq!` de f64 (sem tolerância: bit-exato).

use crate::cluster::ClusterBio;

/// Estado da amostra em Struct-of-Arrays: `cols[k][i]` é a dimensão
/// `k` (0..97) da i-ésima amostra (ordem da amostra estratificada).
#[derive(Debug, Clone)]
pub struct SampleSoA {
    d: usize,
    n: usize,
    cols: Vec<Vec<f64>>,
}

impl SampleSoA {
    /// Empacota a amostra (índices em `clusters`) em colunas.
    pub fn pack(clusters: &[ClusterBio], sample: &[usize]) -> Self {
        let d = clusters.first().map(|c| c.state.len()).unwrap_or(0);
        let n = sample.len();
        let mut cols: Vec<Vec<f64>> = vec![Vec::with_capacity(n); d];
        for &i in sample {
            let st = &clusters[i].state;
            for k in 0..d {
                cols[k].push(st[k]);
            }
        }
        Self { d, n, cols }
    }

    /// Número de amostras empacotadas.
    pub fn len(&self) -> usize {
        self.n
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Σᵢ cols[k][i] por dimensão, na ordem canônica i — bit-idêntico
    /// ao loop AoS `dim_mean[k] += st[k]` na ordem da amostra.
    pub fn dim_sums(&self) -> Vec<f64> {
        (0..self.d)
            .map(|k| self.cols[k].iter().fold(0.0_f64, |acc, &v| acc + v))
            .collect()
    }

    /// Σᵢ ‖estadoᵢ‖ com o produto interno na ordem k — bit-idêntico
    /// ao loop AoS (i externo, k interno).
    pub fn norms_sum(&self) -> f64 {
        let mut acc = 0.0_f64;
        for i in 0..self.n {
            let mut n2 = 0.0_f64;
            for k in 0..self.d {
                let v = self.cols[k][i];
                n2 += v * v;
            }
            acc += n2.sqrt();
        }
        acc
    }

    /// Σᵢ cols[start+k][i] da projeção short_memory — bit-idêntico ao
    /// loop AoS `short_proj[k] += st[sm.start + k]`.
    pub fn proj_sums(&self, start: usize, dim: usize) -> Vec<f64> {
        (0..dim)
            .map(|k| self.cols[start + k].iter().fold(0.0_f64, |acc, &v| acc + v))
            .collect()
    }

    /// Separação: média |Δ| dos pares espaçados por `pair_stride`
    /// (determinístico, cap de custo) — bit-idêntico ao loop AoS de
    /// pares (p externo, k interno). Retorna (acumulador, nº pares).
    pub fn separation_sum(&self, pair_stride: usize, d: usize) -> (f64, usize) {
        let mut acc = 0.0_f64;
        let mut pairs = 0usize;
        let mut p = 0usize;
        while p + pair_stride < self.n {
            let mut s = 0.0_f64;
            for k in 0..d {
                s += (self.cols[k][p] - self.cols[k][p + pair_stride]).abs();
            }
            acc += s / d as f64;
            pairs += 1;
            p += pair_stride;
        }
        (acc, pairs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use triad_foundation::id::ClusterId;

    fn clusters(n: usize) -> Vec<ClusterBio> {
        let mut rng = rand::rngs::StdRng::seed_from_u64(21);
        (0..n).map(|_| ClusterBio::new(ClusterId::new(), 0, &mut rng)).collect()
    }

    /// A/A bit-exato: TODOS os kernels SoA contra os loops AoS
    /// originais (mesma ordem, `assert_eq!` sem tolerância).
    #[test]
    fn kernels_soa_sao_bit_identicos_aos_loops_aos() {
        let clusters = clusters(37);
        let sample: Vec<usize> = (0..clusters.len()).step_by(3).collect();
        let d = clusters[0].state.len();
        let sm_start = 8;
        let dim = 12;
        let pair_stride = (sample.len() / 64).max(1);

        // Referência AoS: os loops ORIGINAIS do reservoir (ordem i).
        let mut ref_dim_mean = vec![0.0_f64; d];
        let mut ref_norm = 0.0_f64;
        let mut ref_proj = vec![0.0_f64; dim];
        let mut ref_sep = 0.0_f64;
        let mut ref_pairs = 0usize;
        for &ci in &sample {
            let st = &clusters[ci].state;
            let mut n2 = 0.0_f64;
            for k in 0..d {
                ref_dim_mean[k] += st[k];
                n2 += st[k] * st[k];
            }
            ref_norm += n2.sqrt();
            for k in 0..dim {
                ref_proj[k] += st[sm_start + k];
            }
        }
        let mut p = 0usize;
        while p + pair_stride < sample.len() {
            let a = &clusters[sample[p]].state;
            let b = &clusters[sample[p + pair_stride]].state;
            let mut s = 0.0_f64;
            for k in 0..d {
                s += (a[k] - b[k]).abs();
            }
            ref_sep += s / d as f64;
            ref_pairs += 1;
            p += pair_stride;
        }

        // SoA: pack + kernels.
        let soa = SampleSoA::pack(&clusters, &sample);
        assert_eq!(soa.len(), sample.len());
        assert_eq!(soa.dim_sums(), ref_dim_mean, "dim_sums bit-exato");
        assert_eq!(soa.norms_sum(), ref_norm, "norms_sum bit-exato");
        assert_eq!(soa.proj_sums(sm_start, dim), ref_proj, "proj bit-exato");
        let (sep, pairs) = soa.separation_sum(pair_stride, d);
        assert_eq!(sep, ref_sep, "separation bit-exata");
        assert_eq!(pairs, ref_pairs);
    }

    /// Round-trip: cols[k][i] == clusters[sample[i]].state[k].
    #[test]
    fn pack_preserva_exatamente_o_estado_da_amostra() {
        let clusters = clusters(11);
        let sample: Vec<usize> = vec![3, 7, 10];
        let soa = SampleSoA::pack(&clusters, &sample);
        let d = clusters[0].state.len();
        for (i, &ci) in sample.iter().enumerate() {
            for k in 0..d {
                assert_eq!(soa.cols[k][i], clusters[ci].state[k]);
            }
        }
    }
}
