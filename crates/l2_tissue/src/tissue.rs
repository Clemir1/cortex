//! Tecido real: grupo de clusters identificado por `ClusterId` (estável
//! através de compactações do L1). O centroide é mantido INCREMENTAL nos
//! adds (fresco dentro do step de formação) e recalculado do zero nos
//! tecidos tocados por remoções (morte/migração) — contribuição de morto
//! nunca persiste. A especialização emerge como DADO: a partição com
//! maior massa média do centroide.

use std::collections::HashMap;

use triad_foundation::id::{ClusterId, TissueId};

use triad_l1_substrate::cluster::ClusterBio;
use triad_l1_substrate::config::slice;

/// Partição funcional do estado 97D que domina o tecido.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateRegion {
    Sensory,
    ShortMemory,
    EnergyRegion,
    Communication,
    Structural,
    Adaptive,
}

impl StateRegion {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sensory => "sensory",
            Self::ShortMemory => "short_memory",
            Self::EnergyRegion => "energy_region",
            Self::Communication => "communication",
            Self::Structural => "structural",
            Self::Adaptive => "adaptive",
        }
    }

    /// Região da dimensão `d` (partições do ClusterBio: 16/16/16/16/16/17).
    pub const fn of_dimension(d: usize) -> Self {
        if d < slice::SENSORY.end {
            Self::Sensory
        } else if d < slice::SHORT_MEMORY.end {
            Self::ShortMemory
        } else if d < slice::ENERGY_REGION.end {
            Self::EnergyRegion
        } else if d < slice::COMMUNICATION.end {
            Self::Communication
        } else if d < slice::STRUCTURAL.end {
            Self::Structural
        } else {
            Self::Adaptive
        }
    }
}

/// Um tecido: membros por identidade, qualidade derivada e centroide.
#[derive(Debug, Clone)]
pub struct Tissue {
    /// Identidade determinística do tecido.
    pub id: TissueId,
    /// Membros por identidade, na ordem de entrada (estável).
    pub members: Vec<ClusterId>,
    /// Energia média dos membros vivos.
    pub energy: f64,
    /// Coesão: 1 − distância média membro↔centroide (0..=1).
    pub coherence: f64,
    /// Região dominante do centroide (especialização como dado).
    pub dominant_region: StateRegion,
    /// Step de formação.
    pub formation_step: u64,
    /// Último step com mudança de membros.
    pub last_change_step: u64,
    /// Soma incremental dos estados dos membros (por dimensão).
    centroid_sum: Vec<f64>,
    /// Centroide atual = centroid_sum / membros.
    centroid: Vec<f64>,
}

impl Tissue {
    /// Nasce com um membro semente — o centroide JÁ É o estado da semente
    /// (senão um centroide zero aceitaria qualquer candidato).
    pub fn new(id: TissueId, seed: ClusterId, state: &[f64], step: u64) -> Self {
        debug_assert_eq!(state.len(), triad_l1_substrate::config::DIMENSIONALITY);
        Self {
            id,
            members: vec![seed],
            energy: 0.0,
            coherence: 0.0,
            dominant_region: StateRegion::Sensory,
            formation_step: step,
            last_change_step: step,
            centroid_sum: state.to_vec(),
            centroid: state.to_vec(),
        }
    }

    /// Tamanho atual.
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// Tecido sem membros.
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Capacidade ocupada: membros / máximo.
    /// Ocupação relativa à capacidade configurada (`[l2.tissues]
    /// max_members`) — sinal de falta de capacidade, não enforce.
    pub fn capacity(&self, max_members: usize) -> f64 {
        self.members.len() as f64 / max_members as f64
    }

    /// Centroide atual (97D).
    pub fn centroid(&self) -> &[f64] {
        &self.centroid
    }

    /// Contém o membro?
    pub fn contains(&self, cid: &ClusterId) -> bool {
        self.members.contains(cid)
    }

    /// Afinidade de um estado com este tecido:
    /// `1 − média|estado − centroide| / 2` (dist média por dim ∈ [0,2]
    /// ⇒ afinidade ∈ [0,1]).
    pub fn affinity(&self, state: &[f64]) -> f64 {
        debug_assert_eq!(state.len(), self.centroid.len());
        let mut acc = 0.0_f64;
        for (a, b) in state.iter().zip(self.centroid.iter()) {
            acc += (a - b).abs();
        }
        1.0 - acc / (state.len() as f64 * 2.0)
    }

    /// Adiciona membro com estado (soma incremental — o centroide fica
    /// fresco para os próximos candidatos DO MESMO step).
    pub fn add_member(&mut self, cid: ClusterId, state: &[f64], step: u64) {
        debug_assert_eq!(state.len(), self.centroid_sum.len());
        self.members.push(cid);
        for (s, x) in self.centroid_sum.iter_mut().zip(state.iter()) {
            *s += x;
        }
        self.last_change_step = step;
        self.refresh_centroid();
    }

    /// Remove membro por identidade; `true` se existia. O centroide fica
    /// stale até o `refresh` (o caller marca o tecido como tocado).
    pub fn remove_member(&mut self, cid: &ClusterId, step: u64) -> bool {
        if let Some(pos) = self.members.iter().position(|m| m == cid) {
            self.members.remove(pos);
            self.last_change_step = step;
            true
        } else {
            false
        }
    }

    /// Centroide = soma / contagem (O(97)).
    fn refresh_centroid(&mut self) {
        let n = self.members.len();
        if n == 0 {
            return;
        }
        for (c, s) in self.centroid.iter_mut().zip(self.centroid_sum.iter()) {
            *c = *s / n as f64;
        }
    }

    /// Recalcula TUDO do zero a partir dos membros VIVOS (centroid,
    /// soma, energia, coesão, região dominante). Chamado nos tecidos
    /// tocados por remoção no step — sem contribuição fantasma de mortos.
    pub fn refresh(&mut self, clusters: &[ClusterBio], id_index: &HashMap<ClusterId, usize>) {
        let dim = self.centroid.len();
        let mut sum = vec![0.0_f64; dim];
        let mut energy_acc = 0.0_f64;
        let mut dist_acc = 0.0_f64;
        let mut count = 0usize;
        for cid in &self.members {
            let Some(&i) = id_index.get(cid) else {
                debug_assert!(false, "refresh com membro ausente do id_index");
                continue;
            };
            let c = &clusters[i];
            energy_acc += c.energy;
            for (d, x) in c.state.iter().enumerate() {
                sum[d] += x;
            }
            count += 1;
        }
        if count == 0 {
            self.energy = 0.0;
            self.coherence = 0.0;
            return;
        }
        self.centroid_sum = sum.clone();
        for (cd, s) in self.centroid.iter_mut().zip(sum.iter()) {
            *cd = *s / count as f64;
        }
        for cid in &self.members {
            let Some(&i) = id_index.get(cid) else {
                continue;
            };
            let c = &clusters[i];
            for (d, x) in c.state.iter().enumerate() {
                dist_acc += (x - self.centroid[d]).abs();
            }
        }
        self.energy = energy_acc / count as f64;
        self.coherence = 1.0 - (dist_acc / (count as f64 * dim as f64)) / 2.0;
        self.dominant_region = dominant_region_of(&self.centroid);
    }
}

/// Região dominante: dimensão com maior |centroide| (empate: primeira —
/// determinístico).
fn dominant_region_of(centroid: &[f64]) -> StateRegion {
    let mut best = StateRegion::Sensory;
    let mut best_mass = f64::MIN;
    for (d, x) in centroid.iter().enumerate() {
        if x.abs() > best_mass {
            best_mass = x.abs();
            best = StateRegion::of_dimension(d);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn tissue_id(seq: u64) -> TissueId {
        TissueId::from_uuid(uuid::Uuid::from_u64_pair(7, seq))
    }

    /// Cluster de teste com estado controlado.
    fn cluster_with(id: ClusterId, energy: f64, state: Vec<f64>) -> ClusterBio {
        let mut rng = rand::rngs::StdRng::seed_from_u64(9);
        let mut c = ClusterBio::new(id, 0, &mut rng);
        c.state = state;
        c.energy = energy;
        c
    }

    #[test]
    fn semente_define_centroide_imediatamente() {
        let state = vec![0.9_f64; 97];
        let t = Tissue::new(tissue_id(1), ClusterId::new(), &state, 0);
        // Antagonico com centroid 0.9: affinity 0.1 (não o 0.55 de um
        // centroide zero).
        let opp = vec![-0.9_f64; 97];
        assert!((t.affinity(&opp) - 0.1).abs() < 1e-12);
        assert!((t.affinity(&state) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn add_e_incremental_e_centroide_fica_fresco_no_step() {
        let s0 = vec![0.25_f64; 97];
        let mut t = Tissue::new(tissue_id(2), ClusterId::new(), &s0, 0);
        t.add_member(ClusterId::new(), &vec![0.75_f64; 97], 1);
        for x in t.centroid() {
            assert!((x - 0.5).abs() < 1e-12);
        }
        assert_eq!(t.len(), 2);
        assert!((t.capacity(64) - 2.0 / 64.0).abs() < 1e-12);
    }

    #[test]
    fn refresh_dos_vivos_sem_fantasma_dos_mortos() {
        let id_a = ClusterId::new();
        let id_b = ClusterId::new();
        let a = cluster_with(id_a, 1.0, vec![0.25; 97]);
        let b = cluster_with(id_b, 0.5, vec![0.75; 97]);
        let clusters = vec![a, b];
        let id_index = HashMap::from([(id_a, 0usize), (id_b, 1usize)]);
        let mut t = Tissue::new(tissue_id(3), id_a, &vec![0.25_f64; 97], 0);
        t.add_member(id_b, &vec![0.75_f64; 97], 1);
        t.refresh(&clusters, &id_index);
        for x in t.centroid() {
            assert!((x - 0.5).abs() < 1e-12);
        }
        assert!((t.energy - 0.75).abs() < 1e-12);
        // Morte de B: refresh só com vivos — sem contribuição fantasma.
        let mut id_sem_b = id_index.clone();
        id_sem_b.remove(&id_b);
        t.remove_member(&id_b, 2);
        t.refresh(&clusters, &id_sem_b);
        for x in t.centroid() {
            assert!((x - 0.25).abs() < 1e-12);
        }
        assert!((t.energy - 1.0).abs() < 1e-12);
        // Coerência de um tecido unânime é máxima.
        assert!(t.coherence > 0.99);
    }

    #[test]
    fn regiao_dominante_e_dado_do_centroide() {
        let mut state = vec![0.0_f64; 97];
        state[80] = 0.9; // partição adaptativa
        let id = ClusterId::new();
        let clusters = vec![cluster_with(id, 0.8, state)];
        let id_index = HashMap::from([(id, 0usize)]);
        let mut t = Tissue::new(tissue_id(4), id, &vec![0.0_f64; 97], 0);
        t.members.clear();
        t.members.push(id);
        t.refresh(&clusters, &id_index);
        assert_eq!(t.dominant_region, StateRegion::Adaptive);
        assert_eq!(StateRegion::of_dimension(0), StateRegion::Sensory);
        assert_eq!(StateRegion::of_dimension(96), StateRegion::Adaptive);
        assert_eq!(t.dominant_region.as_str(), "adaptive");
    }
}
