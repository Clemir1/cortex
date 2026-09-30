//! Matemática local de L1 — determinística, sem dependências além de `rand`
//! (Box-Muller próprio; o legado usava `rand_distr` implícito do numpy).

use rand::rngs::SmallRng;
use rand::Rng;

/// Normal(0,1) via Box-Muller — determinístico dado o `SmallRng` semeado.
pub fn normal(rng: &mut SmallRng) -> f64 {
    let mut u1 = rng.gen::<f64>();
    while u1 <= f64::EPSILON {
        u1 = rng.gen::<f64>();
    }
    let u2 = rng.gen::<f64>();
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

/// Entropia de Shannon da distribuição |x|/Σ|x| (legado:
/// `ClusterBio.calculate_entropy`). 0.0 para entrada vazia.
pub fn entropy_abs(xs: &[f64]) -> f64 {
    let sum: f64 = xs.iter().map(|x| x.abs()).sum();
    if sum <= f64::EPSILON {
        return 0.0;
    }
    xs.iter()
        .map(|x| {
            let p = x.abs() / sum;
            if p > 0.0 {
                -p * p.ln()
            } else {
                0.0
            }
        })
        .sum()
}

/// Média de |a - b| por dimensão (delta de estado usado por tau/firing).
pub fn mean_abs_delta(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len());
    if n == 0 {
        return 0.0;
    }
    let s: f64 = (0..n).map(|i| (a[i] - b[i]).abs()).sum();
    s / n as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn normal_e_reprodutivel_com_mesma_seed() {
        let mut a = SmallRng::seed_from_u64(42);
        let mut b = SmallRng::seed_from_u64(42);
        for _ in 0..100 {
            assert_eq!(normal(&mut a), normal(&mut b));
        }
    }

    #[test]
    fn entropia_de_zero_e_vazio() {
        assert_eq!(entropy_abs(&[]), 0.0);
        assert_eq!(entropy_abs(&[0.0; 8]), 0.0);
        // Uniforme em N dims: H = ln N.
        let h = entropy_abs(&[1.0; 4]);
        assert!((h - 4.0_f64.ln()).abs() < 1e-12);
    }

    #[test]
    fn mean_abs_delta_simples() {
        assert_eq!(mean_abs_delta(&[0.0, 0.0], &[1.0, -1.0]), 1.0);
        assert_eq!(mean_abs_delta(&[0.0], &[0.25]), 0.25);
    }
}
