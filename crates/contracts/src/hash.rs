//! Hashes estáveis de estado (o legado usava `hashlib.sha256[:16]`;
//! aqui FNV-1a 64-bit — rápido, sem dependências, determinístico entre
//! plataformas para os mesmos bytes little-endian).
//!
//! Invariantes:
//! - `None` para entrada vazia ou não finita (ausência ≠ zero: hash de
//!   ausência não existe);
//! - `-0.0` é normalizado para `0.0` (senão estados equivalentes hasheiam
//!   diferente conforme o caminho aritmético);
//! - a ORDE de iteração entra no hash: é isso que faz o
//!   `cluster_order_hash` detectar reordenações silenciosas de dicionário
//!   (o bug clássico do `_ctx` legado).

use triad_foundation::id::ClusterId;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a 64-bit sobre bytes.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h = FNV_OFFSET;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// Hash de fatia de f64 (bits little-endian). `None` se vazia ou com NaN/inf.
///
/// **Quantização (1e-9):** o valor é arredondado a 9 casas antes de hashear.
/// Soma incremental e soma direta diferem no último bit (ordem de
/// operações) — sem quantização, o mesmo estado lógico hashearia diferente
/// conforme o caminho aritmético, e consumidores veriam mudanças
/// fantasma. O legado já quantizava implicitamente (`"%.6f"`).
pub fn hash_f64_slice(xs: &[f64]) -> Option<u64> {
    if xs.is_empty() || xs.iter().any(|x| !x.is_finite()) {
        return None;
    }
    let mut h = FNV_OFFSET;
    for &x in xs {
        let v: f64 = if x == 0.0 { 0.0 } else { x }; // -0.0 → 0.0
        let q = (v * 1e9).round() / 1e9; // tolerante a ruído de soma
        h = (h ^ q.to_bits()).wrapping_mul(FNV_PRIME);
    }
    Some(h)
}

/// Hash da ordem de iteração de uma lista de ids serializáveis (a
/// representação JSON transparente do UUID é estável entre versões).
/// `None` para lista vazia (ausência não tem hash).
pub fn hash_id_order<T: serde::Serialize>(ids: &[T]) -> Option<u64> {
    if ids.is_empty() {
        return None;
    }
    let bytes = serde_json::to_vec(ids).ok()?;
    Some(fnv1a64(&bytes))
}

/// Hash da ordem de iteração dos clusters (por identidade serde — a
/// representação JSON transparente do UUID é estável entre versões).
pub fn hash_cluster_order(ids: &[ClusterId]) -> Option<u64> {
    hash_id_order(ids)
}

/// Hex fixo de 16 caracteres (espelha o `[:16]` do sha256 legado).
pub fn to_hex16(h: u64) -> String {
    format!("{h:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fatia_vazia_ou_nao_finita_nao_tem_hash() {
        assert!(hash_f64_slice(&[]).is_none());
        assert!(hash_f64_slice(&[0.0, f64::NAN]).is_none());
        assert!(hash_f64_slice(&[f64::INFINITY]).is_none());
    }

    #[test]
    fn menos_zero_normaliza() {
        assert_eq!(
            hash_f64_slice(&[0.0, 1.5]).unwrap(),
            hash_f64_slice(&[-0.0, 1.5]).unwrap()
        );
    }

    #[test]
    fn deterministico_e_sensivel_a_ordem() {
        let a = hash_f64_slice(&[1.0, 2.0, 3.0]).unwrap();
        let b = hash_f64_slice(&[1.0, 2.0, 3.0]).unwrap();
        let c = hash_f64_slice(&[3.0, 2.0, 1.0]).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
