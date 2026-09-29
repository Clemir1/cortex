//! Grafo local do substrato — vizinhança espacial com spatial hash e
//! atualização **incremental**.
//!
//! O legado recomputava vizinhos por força bruta O(n²) a cada mudança de
//! posição. Aqui: hash espacial por células de lado `NEIGHBOR_RADIUS` —
//! vizinhança = células adjacentes (27) filtradas por distância real;
//! clusters que não mudaram de célula não são reprocessados (custo
//! proporcional ao que se moveu — mesmo espírito O(active) das bandas tau).
//!
//! Determinismo: células em `HashMap` servem APENAS para lookup — a ordem
//! canônica de vizinhos sai sempre ordenada por índice (`Vec` ordenado),
//! então iteração de estado nunca depende da ordem interna do HashMap.

use crate::cluster::ClusterBio;
use crate::config as cfg;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use tracing::trace;

type CellKey = [i64; 3];

/// Spatial hash: célula → índices dos clusters nela.
#[derive(Debug, Default)]
pub struct SpatialHash {
    cell_size: f64,
    cells: HashMap<CellKey, Vec<usize>>,
}

impl SpatialHash {
    pub fn new(cell_size: f64) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
        }
    }

    fn key_of(&self, pos: [f64; 3]) -> CellKey {
        [
            (pos[0] / self.cell_size).floor() as i64,
            (pos[1] / self.cell_size).floor() as i64,
            (pos[2] / self.cell_size).floor() as i64,
        ]
    }

    fn insert(&mut self, idx: usize, pos: [f64; 3]) {
        self.cells.entry(self.key_of(pos)).or_default().push(idx);
    }

    /// Índices candidatos: clusters nas 27 células adjacentes (exclui `idx`).
    fn candidates(&self, idx: usize, pos: [f64; 3]) -> Vec<usize> {
        let [cx, cy, cz] = self.key_of(pos);
        let mut out = Vec::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if let Some(v) = self.cells.get(&[cx + dx, cy + dy, cz + dz]) {
                        out.extend(v.iter().copied().filter(|&i| i != idx));
                    }
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// Grafo local: adjacência espacial cacheada, incremental por cluster.
pub struct LocalGraph {
    pub spatial: SpatialHash,
    /// Adjacência por índice de cluster (ordenada — determinismo).
    pub neighbors: Vec<Vec<usize>>,
    pub radius: f64,
    pub rebuild_count: u64,
    pub incremental_updates: u64,
    pub last_mean_degree: f64,
}

impl LocalGraph {
    pub fn new(population_hint: usize) -> Self {
        Self {
            spatial: SpatialHash::new(cfg::NEIGHBOR_RADIUS),
            neighbors: Vec::with_capacity(population_hint),
            radius: cfg::NEIGHBOR_RADIUS,
            rebuild_count: 0,
            incremental_updates: 0,
            last_mean_degree: 0.0,
        }
    }

    fn recompute_neighbors_of(&mut self, idx: usize, clusters: &[ClusterBio]) {
        let pos = clusters[idx].position;
        let cands = self.spatial.candidates(idx, pos);
        let mut nb: Vec<usize> = cands
            .into_iter()
            .filter(|&j| dist2(pos, clusters[j].position) <= self.radius * self.radius)
            .collect();
        nb.sort_unstable();
        nb.dedup();
        self.neighbors[idx] = nb;
    }

    /// Reconstrução completa (gênese e compactação de população).
    pub fn rebuild(&mut self, clusters: &[ClusterBio]) {
        self.spatial = SpatialHash::new(self.radius);
        self.neighbors = vec![Vec::new(); clusters.len()];
        for (i, c) in clusters.iter().enumerate() {
            self.spatial.insert(i, c.position);
        }
        for i in 0..clusters.len() {
            self.recompute_neighbors_of(i, clusters);
        }
        self.rebuild_count += 1;
        trace!(clusters = clusters.len(), "grafo local reconstruído");
        self.refresh_mean_degree();
    }

    /// Atualização incremental: apenas clusters que se moveram (e seus
    /// vínculos antigos/novos) são reprocessados. Simetria garantida:
    /// recomputamos o movido, seus antigos vizinhos E os candidatos da
    /// nova célula — cada um consulta o hash espacial já atualizado.
    pub fn update_positions(&mut self, clusters: &[ClusterBio], moved: &[usize]) {
        // 1. Antigos vizinhos de cada movido (vínculos a invalidar).
        let mut affected: Vec<usize> = Vec::new();
        for &i in moved {
            affected.extend(self.neighbors[i].iter().copied());
        }
        // 2. Re-hash dos movidos (para a célula nova).
        for &i in moved {
            self.spatial.remove_stale(i);
            self.spatial.insert(i, clusters[i].position);
        }
        // 3. Candidatos da nova posição também são afetados.
        for &i in moved {
            affected.extend(self.spatial.candidates(i, clusters[i].position));
        }
        affected.extend(moved.iter().copied());
        affected.sort_unstable();
        affected.dedup();
        // 4. Recomputa a adjacência de todos os afetados.
        for &i in &affected {
            self.recompute_neighbors_of(i, clusters);
        }
        self.incremental_updates += moved.len() as u64;
        trace!(movidos = moved.len(), "atualização incremental do grafo");
        self.refresh_mean_degree();
    }

    pub fn neighbors_of(&self, idx: usize) -> &[usize] {
        &self.neighbors[idx]
    }

    /// Cluster nasceu: slot novo no fim + recompute local (o novo e os
    /// candidatos da célula onde caiu).
    pub fn add_cluster(&mut self, idx: usize, clusters: &[ClusterBio]) {
        debug_assert_eq!(self.neighbors.len(), idx, "add_cluster só no fim");
        self.neighbors.push(Vec::new());
        self.spatial.insert(idx, clusters[idx].position);
        let mut affected = vec![idx];
        affected.extend(self.spatial.candidates(idx, clusters[idx].position));
        affected.sort_unstable();
        affected.dedup();
        for &i in &affected {
            self.recompute_neighbors_of(i, clusters);
        }
        trace!(idx = idx, "cluster adicionado ao grafo local");
        self.refresh_mean_degree();
    }

    /// Amostra até `k` vizinhos para o update_state (legado: min 1, ratio 4).
    pub fn sample_neighbors(&self, idx: usize, rng: &mut StdRng) -> Vec<usize> {
        let nbs = self.neighbors_of(idx);
        if nbs.is_empty() {
            return Vec::new();
        }
        let k = (nbs.len() / cfg::STATE_NEIGHBOR_SAMPLE_RATIO).max(cfg::STATE_NEIGHBOR_SAMPLE_MIN);
        if k >= nbs.len() {
            nbs.to_vec()
        } else {
            let mut sample: Vec<usize> = nbs.to_vec();
            sample.partial_shuffle(rng, k).0.to_vec()
        }
    }

    fn refresh_mean_degree(&mut self) {
        let n = self.neighbors.len();
        self.last_mean_degree = if n == 0 {
            0.0
        } else {
            self.neighbors.iter().map(|v| v.len()).sum::<usize>() as f64 / n as f64
        };
    }
}

impl SpatialHash {
    /// Remove entradas duplicadas/stale de `idx` (varre células — chamado
    /// só para clusters movidos; células são pequenas).
    fn remove_stale(&mut self, idx: usize) {
        for v in self.cells.values_mut() {
            if let Some(p) = v.iter().position(|&i| i == idx) {
                v.swap_remove(p);
            }
        }
    }
}

fn dist2(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cluster::ClusterProfile;
    use rand::{Rng, SeedableRng};
    use triad_foundation::id::ClusterId;

    fn clusters_at(positions: &[[f64; 3]]) -> Vec<ClusterBio> {
        let mut rng = StdRng::seed_from_u64(7);
        positions
            .iter()
            .map(|&p| {
                let mut c = ClusterBio::new(ClusterId::new(), 0, &mut rng);
                c.position = p;
                c.profile = ClusterProfile::default();
                c
            })
            .collect()
    }

    #[test]
    fn vizinhos_dentro_do_raio_fora_nao() {
        let cs = clusters_at(&[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],  // dist 1 → vizinho
            [0.5, 0.5, 0.0],  // dist ~0.707 → vizinho
            [10.0, 0.0, 0.0], // dist 10 → NÃO
        ]);
        let mut g = LocalGraph::new(4);
        g.rebuild(&cs);
        assert_eq!(g.neighbors_of(0), &[1, 2]);
        assert!(g.neighbors_of(3).is_empty());
        assert!((g.last_mean_degree - 1.5).abs() < 1e-9);
    }

    #[test]
    fn incremental_igual_rebuild_total() {
        // Propriedade: update incremental == rebuild completo.
        let mut rng = StdRng::seed_from_u64(99);
        let mut cs = clusters_at(&[]);
        for _ in 0..40 {
            let mut c = ClusterBio::new(ClusterId::new(), 0, &mut rng);
            c.position = [
                rng.gen::<f64>() * 20.0,
                rng.gen::<f64>() * 20.0,
                rng.gen::<f64>() * 20.0,
            ];
            cs.push(c);
        }
        let mut g_inc = LocalGraph::new(cs.len());
        g_inc.rebuild(&cs);
        let mut g_full = LocalGraph::new(cs.len());
        g_full.rebuild(&cs);

        // Move 5 clusters de célula (longe da original).
        let moved: Vec<usize> = (0..5).collect();
        for &i in &moved {
            cs[i].position = [
                rng.gen::<f64>() * 20.0,
                rng.gen::<f64>() * 20.0,
                rng.gen::<f64>() * 20.0,
            ];
        }
        g_inc.update_positions(&cs, &moved);
        g_full.rebuild(&cs);
        assert_eq!(g_inc.neighbors, g_full.neighbors, "incremental == total");
    }

    #[test]
    fn amostra_de_vizinhos_respeita_ratio() {
        let positions: Vec<[f64; 3]> = (0..8)
            .map(|i| [i as f64 * 0.5, 0.0, 0.0])
            .collect();
        let cs = clusters_at(&positions); // todos próximos
        let mut g = LocalGraph::new(cs.len());
        g.rebuild(&cs);
        let mut rng = StdRng::seed_from_u64(1);
        let n_total = g.neighbors_of(4).len();
        assert!(n_total >= 2);
        let sample = g.sample_neighbors(4, &mut rng);
        let expected = (n_total / cfg::STATE_NEIGHBOR_SAMPLE_RATIO).max(cfg::STATE_NEIGHBOR_SAMPLE_MIN);
        assert_eq!(sample.len(), expected.min(n_total));
    }
}
