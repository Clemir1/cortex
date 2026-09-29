//! Embriogênese: plano inicial determinístico do organismo (boot).

use serde::{Deserialize, Serialize};
use tracing::debug;
use triad_foundation as tf;

/// Plano embrionário: clusters a criar e ordem de ativação.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmbryoPlan {
    /// Clusters previstos no boot (IDs opacos; o plano é a parte determinística).
    pub clusters: Vec<tf::ClusterId>,
    /// Ordem de ativação dos clusters — função pura do seed.
    pub order: Vec<String>,
}

/// Passo xorshift64 do gerador interno (determinístico, sem dependências externas).
fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// Boot determinístico — a forma inicial não é loteria.
///
/// Mesmo seed → mesmo plano (quantidades e ordem); `initial_clusters` 0 produz plano vazio válido.
pub fn genesis(seed: u64, initial_clusters: usize) -> EmbryoPlan {
    // seed | 1 garante estado nunca-nulo para o xorshift.
    let mut state = seed | 1;

    // Nomes canônicos dos clusters; a permutação abaixo define a ordem de ativação.
    let mut order: Vec<String> = (0..initial_clusters).map(|i| format!("cluster-{i}")).collect();

    // Fisher-Yates movido só pelo PRNG do seed: a ordem é função pura do seed.
    for i in (1..initial_clusters).rev() {
        let j = (next(&mut state) % (i as u64 + 1)) as usize;
        order.swap(i, j);
    }

    // IDs são opacos (ClusterId::new); o PLANO — ordem e quantidades — é o que carrega a determinação.
    let clusters = (0..initial_clusters).map(|_| tf::ClusterId::new()).collect();

    debug!(seed, initial_clusters, "gênese concluída: plano inicial gerado");

    EmbryoPlan { clusters, order }
}
