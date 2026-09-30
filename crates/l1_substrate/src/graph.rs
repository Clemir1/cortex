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
    /// 17.6 — índice reverso cluster→célula: `remove_stale` vira
    /// O(célula) em vez de varrer TODAS as células por movido (o
    /// caminho antigo era O(moved × células) ≈ 5 s @30K). A ordem
    /// interna das células não afeta a adjacência: `candidates`
    /// sempre ordena por índice (determinismo preservado).
    where_is: Vec<CellKey>,
}

impl SpatialHash {
    pub fn new(cell_size: f64) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
            where_is: Vec::new(),
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
        let key = self.key_of(pos);
        self.cells.entry(key).or_default().push(idx);
        if self.where_is.len() <= idx {
            self.where_is.resize(idx + 1, [i64::MIN; 3]);
        }
        self.where_is[idx] = key;
    }

    /// Remove `idx` da célula ONDE ELE ESTÁ (índice reverso — sem
    /// varrer o hash inteiro; 17.6).
    fn remove_stale(&mut self, idx: usize) {
        if idx >= self.where_is.len() {
            return;
        }
        let cell = self.where_is[idx];
        if let Some(v) = self.cells.get_mut(&cell) {
            if let Some(p) = v.iter().position(|&i| i == idx) {
                v.swap_remove(p);
            }
        }
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
    /// vínculos antigos/novos) são reprocessados. Simetria garantida
    /// por GEOMETRIA (17.6): o movido é recomputado primeiro — seus
    /// vizinhos REAIS na nova posição são exatamente os clusters no
    /// raio dele (dist² é simétrica), então estes (e apenas estes)
    /// ganham/perdem o movido; candidatos FORA do raio não mudam de
    /// adjacência — recomputá-los era O(célula) por movido (o caminho
    /// antigo coletava TODOS os candidatos: ~5 s @30K movidos).
    /// Resultado bit-idêntico (ver `incremental_igual_rebuild_total`).
    pub fn update_positions(&mut self, clusters: &[ClusterBio], moved: &[usize]) {
        // 1. Adjacência ANTIGA dos movidos (arestas a invalidar) —
        // capturada antes de qualquer recompute.
        let old_adj: Vec<Vec<usize>> = moved
            .iter()
            .map(|&m| self.neighbors[m].clone())
            .collect();
        // 2. Re-hash dos movidos (índice reverso — remoção O(célula)).
        for &i in moved {
            self.spatial.remove_stale(i);
            self.spatial.insert(i, clusters[i].position);
        }
        // 3. MOVIDOS primeiro: adjacência nova (vizinhos reais no raio).
        for &i in moved {
            self.recompute_neighbors_of(i, clusters);
        }
        // 4. Vínculos dos AFETADOS com os MOVED apenas (17.6): quem
        // não se moveu mantém posição — só as ARESTAS com os movidos
        // podem mudar. Remover o moved antigo e inserir o moved no
        // raio é bit-idêntico ao recompute completo (vizinhos de j =
        // não-movidos no raio [inalterados] + movidos no raio [re-
        // avaliados]; ordem canônica preservada por inserção/remoção
        // ordenada). Ver `incremental_igual_rebuild_total`.
        for (&m, olds) in moved.iter().zip(old_adj.iter()) {
            for &j in olds {
                let v = &mut self.neighbors[j];
                if let Ok(p) = v.binary_search(&m) {
                    v.remove(p);
                }
            }
        }
        for &m in moved {
            let news = self.neighbors[m].clone();
            for j in news {
                let v = &mut self.neighbors[j];
                if let Err(p) = v.binary_search(&m) {
                    v.insert(p, m);
                }
            }
        }
        self.incremental_updates += moved.len() as u64;
        trace!(movidos = moved.len(), "atualização incremental do grafo");
        self.refresh_mean_degree();
    }

    pub fn neighbors_of(&self, idx: usize) -> &[usize] {
        &self.neighbors[idx]
    }

    /// Cluster nasceu: slot novo no fim + adjacência LOCAL (17.6).
    ///
    /// Custo O(vizinhos_reais × log n) — NUNCA recomputa os afetados
    /// em cadeia: quem já existe ganha o novo como vizinho por
    /// INSERÇÃO ORDENADA (adjacência bit-idêntica ao rebuild completo
    /// — ver teste `add_cluster_igual_ao_rebuild`). O caminho antigo
    /// recomputava cada afetado re-escaneando candidatos: O(afetados
    /// × n) por divisão ≈ 3 s @30K.
    pub fn add_cluster(&mut self, idx: usize, clusters: &[ClusterBio]) {
        debug_assert_eq!(self.neighbors.len(), idx, "add_cluster só no fim");
        self.neighbors.push(Vec::new());
        self.spatial.insert(idx, clusters[idx].position);
        // Adjacência do novo: candidatos das 27 células filtrados por
        // distância real (mesma regra do recompute), ordem canônica.
        let mut nb: Vec<usize> = self
            .spatial
            .candidates(idx, clusters[idx].position)
            .into_iter()
            .filter(|&j| {
                dist2(clusters[idx].position, clusters[j].position) <= self.radius * self.radius
            })
            .collect();
        nb.sort_unstable();
        nb.dedup();
        // Simetria: cada vizinho real ganha `idx` por inserção ORDENADA
        // (determinística, sem recomputar nada de quem já existe).
        let nb_owned = std::mem::take(&mut nb);
        for &j in &nb_owned {
            if j == idx {
                continue;
            }
            let v = &mut self.neighbors[j];
            match v.binary_search(&idx) {
                Ok(_) => {}
                Err(pos) => v.insert(pos, idx),
            }
        }
        self.neighbors[idx] = nb_owned;
        self.incremental_updates += 1;
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

    /// 17.6 — o NASCIMENTO incremental (add_cluster com inserção
    /// ordenada nos vizinhos) é bit-idêntico ao rebuild completo.
    #[test]
    fn add_cluster_igual_ao_rebuild() {
        let mut rng = StdRng::seed_from_u64(31);
        let mut cs: Vec<ClusterBio> = (0..60)
            .map(|_| {
                let mut c = ClusterBio::new(ClusterId::new(), 0, &mut rng);
                c.position = [
                    rng.gen::<f64>() * 12.0,
                    rng.gen::<f64>() * 12.0,
                    rng.gen::<f64>() * 12.0,
                ];
                c
            })
            .collect();
        let mut filho = ClusterBio::new(ClusterId::new(), 0, &mut rng);
        filho.position = [
            rng.gen::<f64>() * 12.0,
            rng.gen::<f64>() * 12.0,
            rng.gen::<f64>() * 12.0,
        ];
        cs.push(filho);
        let n = cs.len() - 1;
        let mut g_inc = LocalGraph::new(cs.len());
        g_inc.rebuild(&cs[..n]);
        g_inc.add_cluster(n, &cs);
        let mut g_full = LocalGraph::new(cs.len());
        g_full.rebuild(&cs);
        assert_eq!(
            g_inc.neighbors, g_full.neighbors,
            "nascimento incremental == rebuild total"
        );
    }

    /// 17.6 — regressão do gargalo de escala: o caminho antigo
    /// recomputava TODOS os afetados em cadeia (O(afetados × n) ≈ 3 s
    /// @30K por divisão); o novo é local. Regime REAL do boot
    /// (ClusterBio::new, mesma semente do organismo). Margem CI
    /// generosa: 500 ms.
    #[test]
    fn add_cluster_30k_cabe_no_tick() {
        let mut rng = StdRng::seed_from_u64(42);
        let n = 30_000;
        let mut cs: Vec<ClusterBio> = (0..n)
            .map(|_| ClusterBio::new(ClusterId::new(), 0, &mut rng))
            .collect();
        let filho = ClusterBio::new(ClusterId::new(), 0, &mut rng);
        cs.push(filho);
        let mut g = LocalGraph::new(cs.len());
        g.rebuild(&cs[..n]);
        let t0 = std::time::Instant::now();
        g.add_cluster(n, &cs);
        let dt = t0.elapsed();
        assert!(
            dt.as_millis() < 500,
            "add_cluster @30K levou {dt:?} (o caminho antigo era ~3 s)"
        );
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
