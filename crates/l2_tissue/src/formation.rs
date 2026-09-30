//! FormaÃ§Ã£o de tecidos por afinidade â€” determinÃ­stica (ordem canÃ´nica da
//! matriz do L1, first-fit por ordem de formaÃ§Ã£o, ids de tecido derivados
//! de seed+sequÃªncia). Sem rng: a organizaÃ§Ã£o Ã© funÃ§Ã£o pura do estado.
//!
//! Ciclo por step: (1) mortos saem; (2) re-vinculaÃ§Ã£o com histerese
//! (cadÃªncia `REBIND_INTERVAL_STEPS`, margem `REBIND_MARGIN`); (3) vivos
//! sem tecido entram no primeiro compatÃ­vel ou fundam tecido novo (atÃ©
//! `L2Config::default().tissues.max_count`); (4) dissoluÃ§Ã£o de tecidos abaixo do mÃ­nimo apÃ³s a
//! carÃªncia de gÃªnese; (5) refresh de qualidade dos tecidos tocados.

use std::collections::{BTreeSet, HashMap};

use triad_foundation::id::{ClusterId, TissueId};

use triad_l1_substrate::cluster::ClusterBio;

use crate::config::L2Config;
use crate::tissue::Tissue;

/// ParÃ¢metros mutÃ¡veis em runtime â€” sÃ³ mudam via gate de adaptaÃ§Ã£o
/// ([`crate::adaptation`]), nunca por escrita direta de outra camada.
#[derive(Debug, Clone, PartialEq)]
pub struct FormationParams {
    /// Afinidade mÃ­nima para membership.
    pub affinity_threshold: f64,
    /// Abaixo disso o tecido se dissolve (apÃ³s carÃªncia).
    pub min_members: usize,
    /// Capacidade por tecido.
    pub max_members: usize,
    /// Alvo de coesÃ£o (referÃªncia de qualidade; dado, nÃ£o enforce).
    pub coherence_target: f64,
    /// Arestas inter-tecido mÃ­nimas para decretar bridge.
    pub bridge_min_edges: usize,
    /// 17.14: limiar de especialização do feedback top-down
    /// (L3→L2) — mutável SÓ pelo gate (AdaptParam
    /// `l2.tissue.specialization_threshold`).
    pub specialization_threshold: f64,
}

impl Default for FormationParams {
    fn default() -> Self {
        Self::from_config(&L2Config::default())
    }
}

impl FormationParams {
    /// Deriva os parÃ¢metros mutÃ¡veis da seÃ§Ã£o `[l2]` do TOML central.
    pub fn from_config(cfg: &L2Config) -> Self {
        Self {
            affinity_threshold: cfg.affinity.threshold,
            min_members: cfg.tissues.min_members,
            max_members: cfg.tissues.max_members,
            coherence_target: cfg.tissues.coherence_target,
            bridge_min_edges: cfg.affinity.bridge_min_edges,
            specialization_threshold: cfg.tissues.specialization_threshold,
        }
    }

    /// MantÃ©m os parÃ¢metros em faixas estruturais sÃ£s (o gate clampa
    /// por parÃ¢metro antes; isto Ã© a rede de seguranÃ§a final).
    pub fn sanitize(&mut self) {
        self.affinity_threshold = self.affinity_threshold.clamp(0.05, 0.95);
        self.min_members = self.min_members.clamp(2, 16);
        self.max_members = self.max_members.clamp(8, 256);
        self.coherence_target = self.coherence_target.clamp(0.1, 0.9);
        self.bridge_min_edges = self.bridge_min_edges.clamp(1, 8);
        self.specialization_threshold = self.specialization_threshold.clamp(0.05, 0.95);
    }
}

/// MudanÃ§a estrutural de um step â€” o runner converte em `TissueEvent`.
#[derive(Debug, Clone, PartialEq)]
pub enum TissueChange {
    /// Tecido fundado.
    Formed(TissueId),
    /// Membro entrou em tecido existente.
    Joined(TissueId),
    /// Membro saiu (morte ou migraÃ§Ã£o).
    Left(TissueId),
    /// Membro migrou de um tecido para outro (histerese satisfeita).
    Rebound { from: TissueId, to: TissueId },
    /// Tecido dissolvido (abaixo do mÃ­nimo, apÃ³s carÃªncia).
    Dissolved(TissueId),
}

/// Motor de formaÃ§Ã£o de tecidos.
pub struct TissueFormation {
    /// Semente determinÃ­stica dos ids de tecido.
    pub seed: u64,
    /// Contador de tecidos jÃ¡ criados.
    pub seq: u64,
    /// ConfiguraÃ§Ã£o vigente (importada de `config/default.toml [l2.*]`).
    pub config: L2Config,
    /// ParÃ¢metros vigentes (subset mutÃ¡vel via gate).
    pub params: FormationParams,
    /// Tecidos vivos, na ordem de formaÃ§Ã£o (estÃ¡vel).
    pub tissues: Vec<Tissue>,
    /// Cluster â†’ Ã­ndice no vetor de tecidos (reconstruÃ­do apÃ³s dissoluÃ§Ã£o).
    pub member_of: HashMap<ClusterId, usize>,
    /// Cluster â†’ Ãºltimo step de vÃ­nculo (histerese de re-vinculaÃ§Ã£o).
    pub last_bind: HashMap<ClusterId, u64>,
    // Contadores (a telemetria nunca mente).
    pub formed_total: u64,
    pub dissolved_total: u64,
    pub rebinds_total: u64,
    pub joins_total: u64,
    pub unassigned_now: usize,
}

impl TissueFormation {
    pub fn new(seed: u64) -> Self {
        Self::new_with_config(seed, L2Config::default())
    }

    /// Cria o motor com a configuraÃ§Ã£o injetada do TOML central.
    pub fn new_with_config(seed: u64, config: L2Config) -> Self {
        let params = FormationParams::from_config(&config);
        Self {
            seed,
            seq: 0,
            config,
            params,
            tissues: Vec::new(),
            member_of: HashMap::new(),
            last_bind: HashMap::new(),
            formed_total: 0,
            dissolved_total: 0,
            rebinds_total: 0,
            joins_total: 0,
            unassigned_now: 0,
        }
    }

    /// Id de tecido determinÃ­stico: par (seed, sequÃªncia).
    fn next_tissue_id(&mut self) -> TissueId {
        self.seq += 1;
        TissueId::from_uuid(uuid::Uuid::from_u64_pair(self.seed, self.seq))
    }

    /// Um step de organizaÃ§Ã£o sobre o substrato.
    ///
    /// `order`: ordem canÃ´nica de iteraÃ§Ã£o (a da matriz L1) â€” Ã© ela que
    /// garante determinismo A/A.
    /// `id_index`: cluster vivo â†’ Ã­ndice em `clusters` (reconstruÃ­do pelo
    /// runner a cada step; compactaÃ§Ã£o do L1 reindexa, ids nÃ£o mudam).
    pub fn update(
        &mut self,
        clusters: &[ClusterBio],
        id_index: &HashMap<ClusterId, usize>,
        order: &[usize],
        step: u64,
    ) -> Vec<TissueChange> {
        let mut changes: Vec<TissueChange> = Vec::new();
        let mut touched: BTreeSet<usize> = BTreeSet::new();

        // (1) Mortos/arquivados/fundidos saem dos tecidos.
        for ti in 0..self.tissues.len() {
            let dead: Vec<ClusterId> = self.tissues[ti]
                .members
                .iter()
                .filter(|cid| !id_index.contains_key(*cid))
                .copied()
                .collect();
            for cid in dead {
                let tid = self.tissues[ti].id;
                if self.tissues[ti].remove_member(&cid, step) {
                    self.member_of.remove(&cid);
                    changes.push(TissueChange::Left(tid));
                    touched.insert(ti);
                }
            }
        }

        // (2) Re-vinculaÃ§Ã£o com histerese (cadÃªncia + margem).
        for &i in order {
            let cid = clusters[i].id;
            let Some(&current) = self.member_of.get(&cid) else {
                continue;
            };
            let last = self.last_bind.get(&cid).copied().unwrap_or(0);
            if step < last.saturating_add(self.config.affinity.rebind_interval_steps()) {
                continue;
            }
            let current_aff = self.tissues[current].affinity(&clusters[i].state);
            let mut best: Option<(usize, f64)> = None;
            for (tj, t) in self.tissues.iter().enumerate() {
                if tj == current || t.len() >= self.params.max_members {
                    continue;
                }
                let a = t.affinity(&clusters[i].state);
                if a < self.params.affinity_threshold {
                    continue;
                }
                if best.map(|(_, b)| a > b).unwrap_or(true) {
                    best = Some((tj, a));
                }
            }
            if let Some((tj, a)) = best {
                if a > current_aff + self.config.affinity.rebind_margin {
                    let from = self.tissues[current].id;
                    let to = self.tissues[tj].id;
                    self.tissues[current].remove_member(&cid, step);
                    self.tissues[tj].add_member(cid, &clusters[i].state, step);
                    self.member_of.insert(cid, tj);
                    self.last_bind.insert(cid, step);
                    self.rebinds_total += 1;
                    changes.push(TissueChange::Rebound { from, to });
                    touched.insert(current);
                    touched.insert(tj);
                }
            }
        }

        // (3) Vivos sem tecido: first-fit determinÃ­stico ou fundaÃ§Ã£o.
        for &i in order {
            let cid = clusters[i].id;
            if self.member_of.contains_key(&cid) {
                continue;
            }
            let mut placed = false;
            for ti in 0..self.tissues.len() {
                let fits = self.tissues[ti].len() < self.params.max_members;
                if fits
                    && self.tissues[ti].affinity(&clusters[i].state)
                        >= self.params.affinity_threshold
                {
                    let tid = self.tissues[ti].id;
                    self.tissues[ti].add_member(cid, &clusters[i].state, step);
                    self.member_of.insert(cid, ti);
                    self.last_bind.insert(cid, step);
                    self.joins_total += 1;
                    changes.push(TissueChange::Joined(tid));
                    touched.insert(ti);
                    placed = true;
                    break;
                }
            }
            if !placed && self.tissues.len() < self.config.tissues.max_count {
                let id = self.next_tissue_id();
                let t = Tissue::new(id, cid, &clusters[i].state, step);
                self.tissues.push(t);
                self.member_of.insert(cid, self.tissues.len() - 1);
                self.last_bind.insert(cid, step);
                self.formed_total += 1;
                changes.push(TissueChange::Formed(id));
                touched.insert(self.tissues.len() - 1);
            }
        }

        // (5) Refresh de qualidade dos tecidos tocados (centroide e
        // mÃ©tricas frescos, sem contribuiÃ§Ã£o de mortos) â€” ANTES da
        // dissoluÃ§Ã£o, que decide com qualidade do step corrente.
        for ti in touched {
            if ti < self.tissues.len() {
                self.tissues[ti].refresh(clusters, id_index);
            }
        }

        // (4) DissoluÃ§Ã£o: abaixo do mÃ­nimo apÃ³s a carÃªncia de gÃªnese.
        let mut dissolved_ids: Vec<TissueId> = Vec::new();
        self.tissues.retain(|t| {
            let abaixo = t.len() < self.params.min_members;
            let maduro = step - t.formation_step >= self.config.affinity.dissolve_grace_steps;
            if abaixo && maduro {
                dissolved_ids.push(t.id);
                false
            } else {
                true
            }
        });
        if !dissolved_ids.is_empty() {
            self.dissolved_total += dissolved_ids.len() as u64;
            for id in &dissolved_ids {
                changes.push(TissueChange::Dissolved(*id));
            }
            // Ãndices no vetor mudaram: reconstrÃ³i o mapa por inteiro.
            self.member_of.clear();
            for (ti, t) in self.tissues.iter().enumerate() {
                for cid in &t.members {
                    self.member_of.insert(*cid, ti);
                }
            }
        }

        // (6) Contagem viva de nÃ£o-vinculados.
        let assigned: usize = self.tissues.iter().map(Tissue::len).sum();
        self.unassigned_now = order.len().saturating_sub(assigned);

        changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn cid(seq: u64) -> ClusterId {
        ClusterId::from_uuid(uuid::Uuid::from_u64_pair(11, seq))
    }

    fn substrato(n: usize, estado: f64) -> (Vec<ClusterBio>, HashMap<ClusterId, usize>) {
        let mut rng = rand::rngs::SmallRng::seed_from_u64(3);
        let mut clusters = Vec::with_capacity(n);
        let mut index = HashMap::new();
        for k in 0..n {
            let id = cid(k as u64);
            let mut c = ClusterBio::new(id, 0, &mut rng);
            c.state = vec![estado; 97];
            c.energy = 0.8;
            index.insert(id, k);
            clusters.push(c);
        }
        (clusters, index)
    }

    #[test]
    fn estados_iguais_formam_um_so_tecido() {
        let (clusters, index) = substrato(10, 0.5);
        let order: Vec<usize> = (0..clusters.len()).collect();
        let mut f = TissueFormation::new(99);
        let changes = f.update(&clusters, &index, &order, 0);
        // Primeiro cluster funda; os demais entram por afinidade 1.0.
        assert_eq!(f.tissues.len(), 1);
        assert_eq!(f.tissues[0].len(), 10);
        assert!(matches!(changes[0], TissueChange::Formed(_)));
        assert_eq!(f.unassigned_now, 0);
        assert_eq!(f.formed_total, 1);
    }

    #[test]
    fn estados_opostos_formam_tecidos_distintos() {
        let mut rng = rand::rngs::SmallRng::seed_from_u64(5);
        let mut clusters = Vec::new();
        let mut index = HashMap::new();
        for k in 0..6 {
            let id = cid(k as u64);
            let mut c = ClusterBio::new(id, 0, &mut rng);
            c.state = vec![if k % 2 == 0 { 0.9 } else { -0.9 }; 97];
            c.energy = 0.8;
            index.insert(id, k);
            clusters.push(c);
        }
        let order: Vec<usize> = (0..clusters.len()).collect();
        let mut f = TissueFormation::new(99);
        f.update(&clusters, &index, &order, 0);
        // Dois grupos antagÃ´nicos, afinidade cruzada = 1 âˆ’ 0.9 = 0.1 < 0.25.
        assert_eq!(f.tissues.len(), 2);
        assert_eq!(f.tissues.iter().map(Tissue::len).sum::<usize>(), 6);
        assert_eq!(f.unassigned_now, 0);
    }

    #[test]
    fn mortos_saem_e_tecido_abaixo_do_minimo_dissolve() {
        let (mut clusters, mut index) = substrato(4, 0.5);
        let order: Vec<usize> = (0..clusters.len()).collect();
        let mut f = TissueFormation::new(99);
        f.update(&clusters, &index, &order, 0);
        assert_eq!(f.tissues.len(), 1);
        // Mata os clusters 0 e 1 (compactaÃ§Ã£o estilo L1: vivos
        // sobrevivem, Ã­ndices remapeados, identidades estÃ¡veis).
        let mortos: Vec<ClusterId> = clusters[0..2].iter().map(|c| c.id).collect();
        for id in &mortos {
            index.remove(id);
        }
        clusters.retain(|c| index.contains_key(&c.id));
        index.clear();
        for (k, c) in clusters.iter().enumerate() {
            index.insert(c.id, k);
        }
        let order: Vec<usize> = (0..clusters.len()).collect();
        let changes = f.update(&clusters, &index, &order, 10);
        // 2 Left + 1 Dissolved (2 membros < min 3, maduro).
        assert_eq!(f.tissues.len(), 0);
        assert_eq!(f.dissolved_total, 1);
        assert!(changes
            .iter()
            .any(|c| matches!(c, TissueChange::Dissolved(_))));
        // Vivos sem tecido = estado honesto, nÃ£o zero fabricado.
        assert_eq!(f.unassigned_now, 2);
        // No step seguinte re-formam um tecido (dentro do grace novo).
        f.update(&clusters, &index, &order, 11);
        assert_eq!(f.tissues.len(), 1);
        assert_eq!(f.tissues[0].len(), 2);
        assert_eq!(f.unassigned_now, 0);
    }

    #[test]
    fn tecido_recem_formado_tem_carencia_antes_de_dissolver() {
        let (clusters, index) = substrato(2, 0.5);
        let order: Vec<usize> = (0..clusters.len()).collect();
        let mut f = TissueFormation::new(99);
        f.update(&clusters, &index, &order, 0);
        // 2 membros < min 3, mas dentro do grace (5 steps): nÃ£o dissolve.
        assert_eq!(f.tissues.len(), 1);
        let changes = f.update(&clusters, &index, &order, 3);
        assert_eq!(f.tissues.len(), 1);
        assert!(!changes
            .iter()
            .any(|c| matches!(c, TissueChange::Dissolved(_))));
    }

    #[test]
    fn ids_de_tecidos_sao_deterministicos() {
        let (clusters, index) = substrato(4, 0.9);
        let order: Vec<usize> = (0..clusters.len()).collect();
        let mut f1 = TissueFormation::new(42);
        let mut f2 = TissueFormation::new(42);
        f1.update(&clusters, &index, &order, 0);
        f2.update(&clusters, &index, &order, 0);
        let ids1: Vec<TissueId> = f1.tissues.iter().map(|t| t.id).collect();
        let ids2: Vec<TissueId> = f2.tissues.iter().map(|t| t.id).collect();
        assert_eq!(ids1, ids2);
        let mut f3 = TissueFormation::new(43);
        f3.update(&clusters, &index, &order, 0);
        assert_ne!(f3.tissues[0].id, ids1[0]);
    }

    #[test]
    fn teto_de_tecidos_e_respeitado() {
        let mut f = TissueFormation::new(99);
        // 40 clusters iguais, mas capacidade 1 por tecido: pede 40
        // tecidos; o teto estrutural Ã© 32 â€” o excedente fica
        // nÃ£o-vinculado (estado honesto, nÃ£o erro fabricado).
        f.params.max_members = 1;
        let mut rng = rand::rngs::SmallRng::seed_from_u64(7);
        let mut clusters = Vec::new();
        let mut index = HashMap::new();
        for k in 0..40u64 {
            let id = cid(k);
            let mut c = ClusterBio::new(id, 0, &mut rng);
            c.state = vec![0.5_f64; 97];
            c.energy = 0.8;
            index.insert(id, k as usize);
            clusters.push(c);
        }
        let order: Vec<usize> = (0..clusters.len()).collect();
        f.update(&clusters, &index, &order, 0);
        assert_eq!(f.tissues.len(), L2Config::default().tissues.max_count);
        assert_eq!(f.unassigned_now, 8);
        assert_eq!(f.formed_total, L2Config::default().tissues.max_count as u64);
    }
}
