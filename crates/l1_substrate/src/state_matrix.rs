//! Matriz de estados versionada — o mapa do substrato.
//!
//! **Substitui as 48 colunas "mágicas" de metadata da matriz legada
//! (state_matrix.py: 97 estado + 48 metadata = 145):** os metadados
//! (energia, idade, tau, lifecycle, entropia, firing) vivem TIPADOS no
//! `ClusterBio` — nada de coluna 26 que às vezes é idade e às vezes é
//! outra coisa. A matriz carrega apenas o que viaja para fora: o estado
//! 97D de cada cluster, em ordem canônica.
//!
//! **O(active):** a média por dimensão é mantida por soma incremental —
//! cada cluster sujo atualiza `sum` com o DELTA da linha
//! (`sum += nova - antiga`), então o custo por step é proporcional aos
//! clusters que mudaram, não à população. `finalize` recalcula média e
//! hashes.
//!
//! **Identidade:** `order` (índices vivos na ordem de inserção) entra no
//! `cluster_order_hash` — reordenação silenciosa é detectável (o bug do
//! `_ctx` heterogêneo do legado).

use crate::cluster::ClusterBio;
use crate::config as cfg;
use std::collections::VecDeque;
use triad_contracts::hash::{hash_cluster_order, hash_f64_slice};
use triad_foundation::id::ClusterId;
use tracing::trace;

/// Matriz row-major `n × 97` com média incremental e hashes.
pub struct ClusterStateMatrix {
    pub rows: Vec<f64>,
    pub n_rows: usize,
    /// Versão monotônica — incrementada a cada `finalize` com mudança.
    pub state_version: u64,
    /// Média por dimensão da POPULAÇÃO viva.
    pub mean_state: Vec<f64>,
    pub mean_state_hash: Option<u64>,
    pub cluster_order_hash: Option<u64>,
    /// Ordem canônica (índices vivos, ordem de inserção).
    pub order: Vec<usize>,
    order_ids: Vec<ClusterId>,
    sum: Vec<f64>,
    /// Telemetria: linhas efetivamente reescritas no último sync.
    pub dirty_rows_last: usize,
}

impl ClusterStateMatrix {
    pub fn new(population_hint: usize) -> Self {
        Self {
            rows: Vec::with_capacity(population_hint * cfg::DIMENSIONALITY),
            n_rows: 0,
            state_version: 0,
            mean_state: vec![0.0; cfg::DIMENSIONALITY],
            mean_state_hash: None,
            cluster_order_hash: None,
            order: Vec::with_capacity(population_hint),
            order_ids: Vec::with_capacity(population_hint),
            sum: vec![0.0; cfg::DIMENSIONALITY],
            dirty_rows_last: 0,
        }
    }

    fn add_row(&mut self, idx: usize, state: &[f64]) {
        // Soma/copia SEM tocar n_rows — contabilidade de população é
        // responsabilidade de rebuild/push_cluster/remove.
        for d in 0..cfg::DIMENSIONALITY {
            self.sum[d] += state[d];
        }
        let base = idx * cfg::DIMENSIONALITY;
        self.rows[base..base + cfg::DIMENSIONALITY].copy_from_slice(state);
    }

    fn sub_row(&mut self, idx: usize) {
        for d in 0..cfg::DIMENSIONALITY {
            self.sum[d] -= self.rows[idx * cfg::DIMENSIONALITY + d];
        }
    }

    /// Rebuild total (gênese e compactação de população).
    pub fn rebuild(&mut self, clusters: &[ClusterBio], alive: &[usize]) {
        let d = cfg::DIMENSIONALITY;
        self.rows = vec![0.0; clusters.len() * d];
        self.n_rows = 0;
        self.sum = vec![0.0; d];
        self.order = Vec::with_capacity(alive.len());
        self.order_ids = Vec::with_capacity(alive.len());
        for &i in alive {
            self.add_row(i, &clusters[i].state);
            self.order.push(i);
            self.order_ids.push(clusters[i].id);
        }
        self.n_rows = alive.len();
        self.dirty_rows_last = alive.len();
        self.finalize();
    }

    /// Sync incremental de UM cluster (O(97)): subtrai a linha antiga,
    /// soma a nova. O runner chama para cada cluster ativo do step.
    pub fn sync_cluster(&mut self, idx: usize, cluster: &ClusterBio) {
        debug_assert!(idx < self.rows.len() / cfg::DIMENSIONALITY);
        self.sub_row(idx);
        self.add_row(idx, &cluster.state);
        self.dirty_rows_last += 1;
    }

    /// Cluster nasceu: ganha linha (o runner já alocou o slot).
    pub fn push_cluster(&mut self, idx: usize, cluster: &ClusterBio) {
        debug_assert_eq!(
            self.rows.len() / cfg::DIMENSIONALITY,
            idx,
            "push_cluster só no fim da matriz"
        );
        self.rows.extend_from_slice(&cluster.state);
        for d in 0..cfg::DIMENSIONALITY {
            self.sum[d] += cluster.state[d];
        }
        self.n_rows += 1;
        self.order.push(idx);
        self.order_ids.push(cluster.id);
        self.dirty_rows_last += 1;
    }

    /// Recalcula média e hashes e avança a versão.
    pub fn finalize(&mut self) {
        let d = cfg::DIMENSIONALITY;
        if self.n_rows == 0 {
            self.mean_state = vec![0.0; d];
            self.mean_state_hash = None;
            self.cluster_order_hash = None;
        } else {
            let n = self.n_rows as f64;
            for k in 0..d {
                self.mean_state[k] = self.sum[k] / n;
            }
            self.mean_state_hash = hash_f64_slice(&self.mean_state);
            self.cluster_order_hash = hash_cluster_order(&self.order_ids);
        }
        self.state_version = self.state_version.wrapping_add(1);
        trace!(
            version = self.state_version,
            dirty = self.dirty_rows_last,
            "versão da matriz de estados avançada"
        );
        // Fim do round: telemetria de sujeira zera (linhas sujas desde o
        // último finalize — rebuild também conta seu próprio round).
        self.dirty_rows_last = 0;
    }

    /// Amostra estratificada por stride sobre a ordem canônica
    /// (determinística — sem rng; política declarada pelo chamador).
    /// Retorna (índices, média da amostra por dimensão).
    pub fn stratified_sample(&self, k: usize) -> (Vec<usize>, Option<Vec<f64>>) {
        if self.order.is_empty() {
            return (Vec::new(), None);
        }
        let stride = (self.order.len() as f64 / k as f64).ceil() as usize;
        let idxs: Vec<usize> = (0..self.order.len())
            .step_by(stride.max(1))
            .map(|p| self.order[p])
            .collect();
        let d = cfg::DIMENSIONALITY;
        let mut mean = vec![0.0; d];
        for &i in &idxs {
            for k in 0..d {
                mean[k] += self.rows[i * d + k];
            }
        }
        for m in &mut mean {
            *m /= idxs.len() as f64;
        }
        (idxs, Some(mean))
    }

    /// Snapshot enxuto da amostra PRE_COGNITIVE (eventos pequenos —
    /// índices e média, nunca payloads 97D).
    pub fn sample_ids(&self) -> &[ClusterId] {
        &self.order_ids
    }
}

/// Janela de telemetria pura (não participa de estado/hash).
#[derive(Debug, Clone)]
pub struct TelemetryWindow {
    data: VecDeque<f64>,
    cap: usize,
}

impl TelemetryWindow {
    pub fn new(cap: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(cap.min(128)),
            cap,
        }
    }

    pub fn push(&mut self, v: f64) {
        if self.data.len() == self.cap {
            self.data.pop_front();
        }
        self.data.push_back(v);
    }

    pub fn mean(&self) -> Option<f64> {
        if self.data.is_empty() {
            None
        } else {
            Some(self.data.iter().sum::<f64>() / self.data.len() as f64)
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cluster::ClusterProfile;
    use rand::SeedableRng;
    use triad_foundation::id::ClusterId;

    fn mk_clusters(n: usize) -> Vec<ClusterBio> {
        let mut rng = rand::rngs::StdRng::seed_from_u64(5);
        (0..n)
            .map(|_| {
                let mut c = ClusterBio::new(ClusterId::new(), 0, &mut rng);
                c.profile = ClusterProfile::default();
                c
            })
            .collect()
    }

    #[test]
    fn media_incremental_igual_recomputada() {
        // Propriedade central: soma incremental == média recomputada.
        let clusters = mk_clusters(12);
        let alive: Vec<usize> = (0..12).collect();
        let mut m = ClusterStateMatrix::new(12);
        m.rebuild(&clusters, &alive);
        // Muta 3 clusters e sincroniza incrementalmente.
        let mut clusters = clusters;
        for i in [2usize, 5, 9] {
            for d in 0..cfg::DIMENSIONALITY {
                clusters[i].state[d] = (clusters[i].state[d] + 0.1).clamp(-1.0, 1.0);
            }
            m.sync_cluster(i, &clusters[i]);
        }
        let dirty_this_round = m.dirty_rows_last; // antes do finalize
        m.finalize();
        // Recomputa do zero e compara.
        let mut m2 = ClusterStateMatrix::new(12);
        m2.rebuild(&clusters, &alive);
        for d in 0..cfg::DIMENSIONALITY {
            assert!(
                (m.mean_state[d] - m2.mean_state[d]).abs() < 1e-12,
                "dim {d}: {} vs {}",
                m.mean_state[d],
                m2.mean_state[d]
            );
        }
        assert_eq!(m.mean_state_hash, m2.mean_state_hash);
        assert_eq!(dirty_this_round, 3);
        assert_eq!(m.dirty_rows_last, 0, "finalize zera o round de sujeira");
    }

    #[test]
    fn versao_monotonica_e_hash_sensivel() {
        let clusters = mk_clusters(4);
        let alive: Vec<usize> = (0..4).collect();
        let mut m = ClusterStateMatrix::new(4);
        m.rebuild(&clusters, &alive);
        let v0 = m.state_version;
        let h0 = m.mean_state_hash;
        m.finalize();
        assert!(m.state_version > v0);
        assert_eq!(m.mean_state_hash, h0, "sem mudança de estado → mesmo hash");
        let mut clusters = clusters;
        clusters[0].state[0] += 0.5;
        m.sync_cluster(0, &clusters[0]);
        m.finalize();
        assert_ne!(m.mean_state_hash, h0, "mudança de estado → hash muda");
    }

    #[test]
    fn amostra_estratificada_e_cobertura() {
        let clusters = mk_clusters(100);
        let alive: Vec<usize> = (0..100).collect();
        let mut m = ClusterStateMatrix::new(100);
        m.rebuild(&clusters, &alive);
        let (idxs, mean) = m.stratified_sample(10);
        assert!(!idxs.is_empty() && idxs.len() <= 10);
        let mean = mean.expect("amostra com população tem média");
        assert_eq!(mean.len(), cfg::DIMENSIONALITY);
        // População vazia: ausência, não zeros.
        let mut m0 = ClusterStateMatrix::new(0);
        m0.rebuild(&[], &[]);
        assert!(m0.mean_state_hash.is_none());
        let (empty, mean_none) = m0.stratified_sample(10);
        assert!(empty.is_empty());
        assert!(mean_none.is_none());
    }
}
