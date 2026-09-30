//! Runner L2: um step de organização tecidual sobre o substrato L1, com
//! as pontes INSTRUMENTADAS:
//! - **L1→L2**: leitura de `POST_PHYSICAL` com recibo E4 registrado no
//!   `L1Ledger` (quem leu, qual versão, qual classificação). Se a fonte
//!   é ausente, o L2 NÃO deriva visão — publica `NoData` com a razão da
//!   fonte (ausência ≠ zero).
//! - **L2→L3**: publicação `POST_TISSUE` no `L2Ledger` (E3, versão
//!   monotônica, hash da ordem de tecidos) + visões consumíveis
//!   ([`L2Runner::views`]) cuja leitura pelo L3 gera recibo E4.
//!
//! Determinismo: formação em ordem canônica da matriz, ids de tecido
//! derivados de seed+sequência, zero rng no L2. Os `event_id` são UUID
//! v7 (unicidade de auditoria) e não entram em nenhum hash.

use std::collections::{HashMap, HashSet};

use triad_contracts as tc;
use triad_foundation as tf;
use triad_l1_substrate as l1;

use tc::l1::{L1ReadReceipt, L1StatePhase};
use tc::l2::TissueSnapshot;
use tc::{AdaptationRequest, TissueEvent, TissueKind, TissueState};
use tf::id::{ClusterId, EventId, TissueId};
use tf::status::Status;
use tf::units::Confidence;

use l1::metrics::{MetricWindow, WindowStats};
use l1::L1Runner;

use crate::adaptation::{AdaptParam, AdaptationGate, GateDecision};
use crate::formation::{FormationParams, TissueChange, TissueFormation};
use crate::l2_ledger::L2Ledger;
use crate::topology::TopologyManager;
use crate::tissue::{StateRegion, Tissue};

/// Tamanhos de janela (telemetria por janelas 1/5/20/50/100).
pub const L2_WINDOW_SIZES: [usize; 5] = [1, 5, 20, 50, 100];

/// Janelas 1/5/20/50/100 de uma série L2.
#[derive(Debug, Clone)]
pub struct L2Windows {
    windows: Vec<MetricWindow>,
}

impl L2Windows {
    pub fn new() -> Self {
        Self {
            windows: L2_WINDOW_SIZES
                .iter()
                .map(|&c| MetricWindow::new(c))
                .collect(),
        }
    }

    pub fn push(&mut self, v: f64) {
        for w in &mut self.windows {
            w.push(v);
        }
    }

    /// Estatísticas da janela `idx` (0..=4 → 1/5/20/50/100).
    pub fn stats(&self, idx: usize) -> Option<WindowStats> {
        self.windows.get(idx).and_then(|w| w.stats())
    }

    /// Estatísticas da janela 100.
    pub fn stats_w100(&self) -> Option<WindowStats> {
        self.stats(4)
    }
}

/// Métricas L2 por janelas (telemetria PURA: nunca decide, nunca hash).
///
/// Convenção: em `NoData` nada é empurrado (ausência não é observação);
/// `coherence` sem tecidos empurra 0.0 — interprete junto com
/// `tissue_count`.
#[derive(Debug, Clone)]
pub struct L2Metrics {
    pub tissue_count: L2Windows,
    pub coverage: L2Windows,
    pub coherence: L2Windows,
    pub unassigned: L2Windows,
    pub bridges: L2Windows,
}

impl L2Metrics {
    pub fn new() -> Self {
        Self {
            tissue_count: L2Windows::new(),
            coverage: L2Windows::new(),
            coherence: L2Windows::new(),
            unassigned: L2Windows::new(),
            bridges: L2Windows::new(),
        }
    }

    pub fn push_step(
        &mut self,
        tissue_count: f64,
        coverage: f64,
        coherence: f64,
        unassigned: f64,
        bridges: f64,
    ) {
        self.tissue_count.push(tissue_count);
        self.coverage.push(coverage);
        self.coherence.push(coherence);
        self.unassigned.push(unassigned);
        self.bridges.push(bridges);
    }
}

/// Report de um step L2 — a ponte visível para o orquestrador.
#[derive(Debug, Clone)]
pub struct L2StepReport {
    pub step: u64,
    pub population_total: u32,
    pub tissue_count: u32,
    pub assigned: u32,
    pub unassigned: u32,
    pub coverage: Option<f32>,
    pub mean_energy: Option<f64>,
    pub coherence_mean: Option<f64>,
    pub bridges: usize,
    pub changes: Vec<TissueChange>,
    pub events: Vec<TissueEvent>,
    pub snapshot: TissueSnapshot,
    pub l1_receipt: L1ReadReceipt,
    pub adaptations_applied: u64,
    pub adaptations_deferred: u64,
}

/// O runner L2: formação por afinidade + topologia com bridges + gate
/// de feedback com histerese + ledger POST_TISSUE + métricas.
pub struct L2Runner {
    /// Semente determinística (ids de tecido = seed ^ 0x7155 + sequência).
    pub seed: u64,
    /// Relógio lógico próprio (um step L2 por chamada).
    pub step: u64,
    pub formation: TissueFormation,
    pub topology: TopologyManager,
    pub gate: AdaptationGate,
    pub ledger: L2Ledger,
    pub metrics: L2Metrics,
    /// Tecidos registrados na topologia (para remover dissolvidos).
    registered: HashSet<TissueId>,
    /// Região dominante conhecida por tecido (emite `Specialized`).
    prev_regions: HashMap<TissueId, StateRegion>,
}

impl L2Runner {
    pub fn new(seed: u64) -> Self {
        Self::new_with_config(seed, crate::config::L2Config::default())
    }

    /// Cria o runner com a configuração injetada de `config/default.toml
    /// [l2.*]` (formação, bridges e gate de adaptação).
    pub fn new_with_config(seed: u64, config: crate::config::L2Config) -> Self {
        let gate = AdaptationGate::new_with_config(&config.adaptation);
        Self {
            seed,
            step: 0,
            formation: TissueFormation::new_with_config(seed ^ 0x7155, config),
            topology: TopologyManager::new(),
            gate,
            ledger: L2Ledger::new(),
            metrics: L2Metrics::new(),
            registered: HashSet::new(),
            prev_regions: HashMap::new(),
        }
    }

    /// Um step L2. Fluxo: recibo L1 → formação → topologia → feedback →
    /// eventos → publicação → telemetria.
    pub fn step(&mut self, l1: &mut L1Runner, feedback: &[AdaptationRequest]) -> L2StepReport {
        self.step += 1;
        let s = self.step;

        // (1) PONTE L1→L2: leitura instrumentada POST_PHYSICAL (E4).
        let l1_receipt = l1.ledger.read(L1StatePhase::PostPhysical, "l2.tissue");
        if l1_receipt.provider_status != Status::Value {
            // Fonte ausente: não deriva visão; publica a ausência com a
            // razão da FONTE (nunca fabrica tecidos de nada).
            let reason = format!(
                "l1_source_absent:{}",
                l1_receipt
                    .fallback_reason
                    .clone()
                    .unwrap_or_else(|| "sem_razao".into())
            );
            let snapshot = self.ledger.publish_no_data(s, &reason);
            return L2StepReport {
                step: s,
                population_total: 0,
                tissue_count: 0,
                assigned: 0,
                unassigned: 0,
                coverage: None,
                mean_energy: None,
                coherence_mean: None,
                bridges: 0,
                changes: Vec::new(),
                events: Vec::new(),
                snapshot,
                l1_receipt,
                adaptations_applied: 0,
                adaptations_deferred: 0,
            };
        }

        let population_total = l1
            .ledger
            .latest(L1StatePhase::PostPhysical)
            .map(|sn| sn.population_total)
            .unwrap_or(0);

        // (2) Índice vivo por identidade (compactação do L1 reindexa; a
        // identidade é estável). O( população ), barato contra o risco
        // de desalinhamento de índices.
        let id_index = build_id_index(l1);
        let order: Vec<usize> = l1.matrix.order.clone();

        // (3) Organização tecidual sobre o substrato (ordem canônica).
        let changes = self.formation.update(&l1.clusters, &id_index, &order, s);

        // (4) Topologia: nós vivos + bridges inter-tecido.
        let bridges = self.sync_topology(l1, &id_index);

        // (5) Feedback L3→L2 pelo gate com histerese.
        let (adaptations_applied, adaptations_deferred) = self.apply_feedback(feedback, s);

        // (6) Eventos pequenos (ADR-0004): transições + especialização.
        let mut events = change_events(&changes);
        events.extend(self.specialized_events());

        // (7) Publicação POST_TISSUE (E3) — versão monotônica no L2Ledger.
        let n_t = self.formation.tissues.len();
        let assigned: usize = self.formation.tissues.iter().map(Tissue::len).sum();
        let mean_energy = if n_t > 0 {
            Some(self.formation.tissues.iter().map(|t| t.energy).sum::<f64>() / n_t as f64)
        } else {
            None
        };
        let coherence_mean = if n_t > 0 {
            Some(
                self.formation
                    .tissues
                    .iter()
                    .map(|t| t.coherence)
                    .sum::<f64>()
                    / n_t as f64,
            )
        } else {
            None
        };
        let ids: Vec<TissueId> = self.formation.tissues.iter().map(|t| t.id).collect();
        let snapshot = self.ledger.publish(
            s,
            population_total,
            n_t as u32,
            assigned as u32,
            mean_energy,
            coherence_mean,
            &ids,
        );

        // (8) Telemetria (só em Value: ausência não é observação).
        let unassigned = order.len().saturating_sub(assigned);
        self.metrics.push_step(
            n_t as f64,
            snapshot.assignment_coverage.unwrap_or(0.0) as f64,
            coherence_mean.unwrap_or(0.0),
            unassigned as f64,
            bridges as f64,
        );

        L2StepReport {
            step: s,
            population_total,
            tissue_count: n_t as u32,
            assigned: assigned as u32,
            unassigned: unassigned as u32,
            coverage: snapshot.assignment_coverage,
            mean_energy,
            coherence_mean,
            bridges,
            changes,
            events,
            snapshot,
            l1_receipt,
            adaptations_applied,
            adaptations_deferred,
        }
    }

    /// PONTE L2→L3: um `TissueState` por tecido — coerência,
    /// especialização (share do centroide na região dominante),
    /// integração (densidade de arestas internas no grafo L1) e os
    /// membros por identidade. Escalares + ids; nada de estado 97D
    /// cruza a fronteira (ADR-0004).
    pub fn views(&self, l1: &L1Runner) -> Vec<TissueState> {
        let id_index = build_id_index(l1);
        self.formation.tissues.iter().map(|t| {
            let n = t.len();
            // Especialização: share da massa |centroide| na região dominante.
            let total: f64 = t.centroid().iter().map(|x| x.abs()).sum();
            let dom: f64 = t
                .centroid()
                .iter()
                .enumerate()
                .filter(|(d, _)| StateRegion::of_dimension(*d) == t.dominant_region)
                .map(|(_, x)| x.abs())
                .sum();
            let specialization = if total > 0.0 { dom / total } else { 0.0 };
            // Integração: densidade de arestas internas no grafo local.
            let possible = n.saturating_sub(1) * n / 2;
            let mut internal = 0usize;
            if n >= 2 {
                for (wi, &ma) in t.members.iter().enumerate() {
                    let Some(&ia) = id_index.get(&ma) else { continue };
                    for &mb in t.members.iter().skip(wi + 1) {
                        let Some(&ib) = id_index.get(&mb) else { continue };
                        if l1.graph.neighbors_of(ia).contains(&ib) {
                            internal += 1;
                        }
                    }
                }
            }
            let integration = if possible > 0 {
                internal as f64 / possible as f64
            } else {
                0.0
            };
            TissueState {
                tissue_id: t.id,
                cohesion: conf(t.coherence),
                specialization: conf(specialization),
                integration: conf(integration),
                member_clusters: t.members.clone(),
            }
        }).collect()
    }

    /// Sincroniza a topologia com os tecidos vivos e decretas bridges
    /// (pares com ≥ `bridge_min_edges` arestas no grafo L1; cap
    /// `bridge_max_per_tissue` por tecido, em ordem determinística).
    fn sync_topology(&mut self, l1: &L1Runner, id_index: &HashMap<ClusterId, usize>) -> usize {
        let live: Vec<TissueId> = self.formation.tissues.iter().map(|t| t.id).collect();
        for id in self.registered.iter() {
            if !live.contains(id) {
                self.topology.remove_tissue(*id);
                self.prev_regions.remove(id);
            }
        }
        for id in &live {
            if !self.registered.contains(id) {
                self.topology.add_tissue(*id);
            }
        }
        self.registered = live.iter().copied().collect();

        let mut links = 0usize;
        let (bridge_enabled, bridge_max) = (
            self.formation.config.affinity.bridge_enabled,
            self.formation.config.affinity.bridge_max_per_tissue,
        );
        if bridge_enabled && self.formation.tissues.len() >= 2 {
            let min_edges = self.formation.params.bridge_min_edges;
            let n = self.formation.tissues.len();
            let mut per_tissue = vec![0usize; n];
            for ti in 0..n {
                if per_tissue[ti] >= bridge_max {
                    continue;
                }
                for tj in (ti + 1)..n {
                    if per_tissue[ti] >= bridge_max {
                        break;
                    }
                    let count = self.inter_edges(l1, id_index, ti, tj);
                    let a = self.formation.tissues[ti].id;
                    let b = self.formation.tissues[tj].id;
                    if count >= min_edges {
                        self.topology.link(a, b);
                        per_tissue[ti] += 1;
                        links += 1;
                    } else {
                        self.topology.unlink(a, b);
                    }
                }
            }
        }
        links
    }

    /// Arestas do grafo L1 entre membros de `ti` e de `tj`.
    fn inter_edges(
        &self,
        l1: &L1Runner,
        id_index: &HashMap<ClusterId, usize>,
        ti: usize,
        tj: usize,
    ) -> usize {
        let member_of = &self.formation.member_of;
        let mut count = 0usize;
        for &m in &self.formation.tissues[ti].members {
            let Some(&im) = id_index.get(&m) else { continue };
            for &nb in l1.graph.neighbors_of(im) {
                let cid = l1.clusters[nb].id;
                if member_of.get(&cid) == Some(&tj) {
                    count += 1;
                }
            }
        }
        count
    }

    /// Aplica os pedidos L3→L2 pelo gate (current = estado REAL do L2).
    fn apply_feedback(
        &mut self,
        feedback: &[AdaptationRequest],
        step: u64,
    ) -> (u64, u64) {
        let mut applied = 0u64;
        let mut deferred = 0u64;
        for req in feedback {
            let current = AdaptParam::from_key(&req.parameter)
                .map(|p| current_value(&self.formation.params, p))
                .unwrap_or(0.0);
            let decision =
                self.gate
                    .request_str(&req.parameter, current, &req.proposed, step);
            match decision {
                GateDecision::Applied { .. } => {
                    AdaptationGate::apply_to_params(&decision, &mut self.formation.params);
                    applied += 1;
                }
                GateDecision::Deferred { .. } => {
                    deferred += 1;
                }
            }
        }
        (applied, deferred)
    }

    /// Eventos `Specialized`: mudança da região dominante de um tecido.
    fn specialized_events(&mut self) -> Vec<TissueEvent> {
        let mut out = Vec::new();
        for t in &self.formation.tissues {
            let r = t.dominant_region;
            if let Some(old) = self.prev_regions.insert(t.id, r) {
                if old != r {
                    out.push(tissue_event(t.id, TissueKind::Specialized));
                }
            }
        }
        out
    }
}

/// Índice vivo: cluster vivo → índice atual em `clusters`.
fn build_id_index(l1: &L1Runner) -> HashMap<ClusterId, usize> {
    let mut index = HashMap::with_capacity(l1.clusters.len());
    for (i, c) in l1.clusters.iter().enumerate() {
        if c.is_alive() {
            index.insert(c.id, i);
        }
    }
    index
}

/// Transições → eventos pequenos (Formed/Integrated/Fragmented).
fn change_events(changes: &[TissueChange]) -> Vec<TissueEvent> {
    changes
        .iter()
        .flat_map(|c| match c {
            TissueChange::Formed(id) => vec![tissue_event(*id, TissueKind::Formed)],
            TissueChange::Joined(id) => vec![tissue_event(*id, TissueKind::Integrated)],
            TissueChange::Left(id) => vec![tissue_event(*id, TissueKind::Fragmented)],
            TissueChange::Rebound { from, to } => vec![
                tissue_event(*from, TissueKind::Fragmented),
                tissue_event(*to, TissueKind::Integrated),
            ],
            TissueChange::Dissolved(id) => vec![tissue_event(*id, TissueKind::Fragmented)],
        })
        .collect()
}

/// Valor atual de um parâmetro (estado real, nunca o declarado pelo L3).
fn current_value(params: &FormationParams, p: AdaptParam) -> f64 {
    match p {
        AdaptParam::AffinityThreshold => params.affinity_threshold,
        AdaptParam::MinMembers => params.min_members as f64,
        AdaptParam::MaxMembers => params.max_members as f64,
        AdaptParam::CoherenceTarget => params.coherence_target,
        AdaptParam::BridgeMinEdges => params.bridge_min_edges as f64,
        AdaptParam::SpecializationThreshold => params.specialization_threshold,
    }
}

/// `Confidence` clampada (construct é Option; clamp garante Some).
fn conf(x: f64) -> Confidence {
    Confidence::construct(x.clamp(0.0, 1.0) as f32).expect("clamp ∈ [0,1] e finito")
}

/// Evento pequeno de tecido.
fn tissue_event(tissue_id: TissueId, kind: TissueKind) -> TissueEvent {
    TissueEvent {
        event_id: EventId::new(),
        tissue_id,
        kind,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tf::id::ModuleId;

    /// Requisição de adaptação de teste.
    fn adapt_request(parameter: &str, proposed: &str) -> AdaptationRequest {
        AdaptationRequest {
            requester: ModuleId::new(),
            target: ModuleId::new(),
            parameter: parameter.to_string(),
            current: String::new(),
            proposed: proposed.to_string(),
        }
    }


    /// Roda `steps` de um organismo L1+L2 com a seed dada e devolve os
    /// snapshots L2 (A/A: bit-idênticos por seed).
    fn run_organism(seed: u64, steps: u64) -> (L1Runner, L2Runner, Vec<TissueSnapshot>) {
        let mut l1 = L1Runner::new(seed, 24);
        let mut l2 = L2Runner::new(seed);
        let mut snaps = Vec::new();
        for _ in 0..steps {
            l1.step();
            let rep = l2.step(&mut l1, &[]);
            snaps.push(rep.snapshot);
        }
        (l1, l2, snaps)
    }

    #[test]
    fn ponte_l1_l2_registra_recibo_e4_no_l1_ledger() {
        let mut l1 = L1Runner::new(42, 24);
        let mut l2 = L2Runner::new(42);
        l1.step();
        let consumed_before = l1.ledger.consumed_value;
        let rep = l2.step(&mut l1, &[]);
        // O recibo da PONTE viaja no report (quem leu, versão, fase).
        assert_eq!(rep.l1_receipt.provider_status, Status::Value);
        assert_eq!(rep.l1_receipt.key, "l2.tissue");
        assert_eq!(rep.l1_receipt.state_phase, Some(L1StatePhase::PostPhysical));
        assert!(rep.l1_receipt.state_hash.is_some());
        // E o E4 ficou registrado no L1Ledger (consumo contado).
        assert_eq!(l1.ledger.consumed_value, consumed_before + 1);
        assert!(l1
            .ledger
            .read_receipts
            .iter()
            .any(|r| r.key == "l2.tissue" && r.provider_status == Status::Value));
    }

    #[test]
    fn aa_bit_identico_mesma_seed_diverge_entre_seeds() {
        let (_, _, a) = run_organism(42, 30);
        let (_, _, b) = run_organism(42, 30);
        assert_eq!(a, b, "mesma seed ⇒ snapshots bit-idênticos");
        let (_, _, c) = run_organism(43, 30);
        assert_ne!(
            a.last().unwrap().tissue_order_hash,
            c.last().unwrap().tissue_order_hash,
            "seeds diferentes ⇒ organizações diferentes"
        );
    }

    #[test]
    fn fonte_ausente_propaga_no_data_com_razao_da_fonte() {
        // L1 sem população: publica NoData; L2 não fabrica tecidos.
        let mut l1 = L1Runner::new(42, 0);
        let mut l2 = L2Runner::new(42);
        l1.step();
        let rep = l2.step(&mut l1, &[]);
        assert_eq!(rep.snapshot.provider_status, Status::NoData);
        let reason = rep.snapshot.no_data_reason.unwrap();
        assert!(
            reason.starts_with("l1_source_absent:"),
            "razão da FONTE viaja: {reason}"
        );
        assert_eq!(rep.tissue_count, 0);
        assert_eq!(l1.ledger.consumed_absence, 1, "ausência consumida conta");
        assert_eq!(l2.ledger.version(), 1, "ausência também versiona");
    }

    #[test]
    fn tecidos_emergem_e_cobrem_a_populacao_viva() {
        let (l1, l2, snaps) = run_organism(7, 40);
        let last = snaps.last().unwrap();
        assert!(last.tissue_count >= 1, "tecidos emergem da afinidade");
        assert_eq!(last.provider_status, Status::Value);
        assert!(last.assignment_coverage.unwrap_or(0.0) > 0.0);
        // Atribuídos + não-atribuídos = população viva (nunca estoura).
        assert_eq!(
            last.assigned_members + last.unassigned_members,
            last.population_total
        );
        if last.mean_energy.is_some() {
            let e = last.mean_energy.unwrap();
            assert!((0.0..=1.0).contains(&e));
        }
        // Janelas de telemetria registradas.
        assert!(l2.metrics.tissue_count.stats_w100().is_some());
        assert!(l2.metrics.coverage.stats_w100().is_some());
        let pop = l1.clusters.iter().filter(|c| c.is_alive()).count() as u32;
        assert_eq!(last.population_total, pop);
    }

    #[test]
    fn feedback_l3_l2_aplica_com_clamp_e_histerese_contada() {
        let mut l1 = L1Runner::new(42, 24);
        let mut l2 = L2Runner::new(42);
        l1.step();
        l2.step(&mut l1, &[]);
        // Pedido 0.25 → 0.5: delta 0.25 > teto 0.1 ⇒ aplica clampado 0.35.
        let rep = l2.step(&mut l1, &[adapt_request("l2.affinity.threshold", "0.5")]);
        assert_eq!(rep.adaptations_applied, 1);
        assert!((l2.formation.params.affinity_threshold - 0.35).abs() < 1e-9);
        assert_eq!(l2.gate.applied, 1);
        assert_eq!(l2.gate.applied_clamped, 1);
        // Imediato de novo: cadência mínima — adiado com razão.
        let rep2 = l2.step(&mut l1, &[adapt_request("l2.affinity.threshold", "0.5")]);
        assert_eq!(rep2.adaptations_deferred, 1);
        assert_eq!(l2.gate.deferred_interval, 1);
        // Parâmetro desconhecido: adiado com razão, contado.
        let rep3 = l2.step(&mut l1, &[adapt_request("l2.nada.disso", "1.0")]);
        assert_eq!(rep3.adaptations_deferred, 1);
        assert_eq!(l2.gate.deferred_unknown, 1);
        // Valor não-parseável: adiado com razão.
        let rep4 = l2.step(&mut l1, &[adapt_request("l2.affinity.threshold", "banana")]);
        assert_eq!(rep4.adaptations_deferred, 1);
        assert_eq!(l2.gate.deferred_unparseable, 1);
        // Após o intervalo (20 steps), micro-ajuste DENTRO da banda é
        // adiado por insignificância — histerese de dois gumes.
        for _ in 0..19 {
            l1.step();
            l2.step(&mut l1, &[]);
        }
        let rep5 = l2.step(&mut l1, &[adapt_request("l2.affinity.threshold", "0.40")]);
        // current 0.35 → 0.40: |Δ| = 0.05 ≤ banda 0.1 ⇒ adiado.
        assert_eq!(rep5.adaptations_deferred, 1);
        assert_eq!(l2.gate.deferred_band, 1);
        assert_eq!(l2.gate.applied, 1, "nenhuma aplicação a mais");
    }

    #[test]
    fn views_publicam_tissue_state_e_leitura_l3_gera_recibo_e4() {
        let (l1, mut l2, _) = run_organism(11, 30);
        // PONTE L2→L3: visões consumíveis (escalares + identidades).
        let views = l2.views(&l1);
        assert!(!views.is_empty());
        for v in &views {
            assert!((0.0..=1.0).contains(&v.cohesion.value()));
            assert!((0.0..=1.0).contains(&v.specialization.value()));
            assert!((0.0..=1.0).contains(&v.integration.value()));
            assert!(!v.member_clusters.is_empty());
        }
        // O L3 lê com recibo (consumo contado no L2Ledger).
        let consumed_before = l2.ledger.consumed_counts().0;
        let receipt = l2.ledger.read("l3.cognition");
        assert_eq!(receipt.provider_status, Status::Value);
        assert_eq!(receipt.state_phase, Some(L1StatePhase::PostTissue));
        assert_eq!(l2.ledger.consumed_counts().0, consumed_before + 1);
        assert!(l2.ledger.receipts().iter().any(|r| r.key == "l3.cognition"));
    }

    #[test]
    fn eventos_sao_pequenos_e_tipados() {
        let (l1, l2, _) = run_organism(5, 12);
        let views = l2.views(&l1);
        let _ = views;
        // Nos primeiros steps há Fundação/Integração com certeza.
        let mut l1 = L1Runner::new(5, 24);
        let mut l2 = L2Runner::new(5);
        l1.step();
        let rep = l2.step(&mut l1, &[]);
        assert!(rep
            .events
            .iter()
            .any(|e| e.kind == TissueKind::Formed));
        // Evento pequeno: id do tecido + tipo; sem payload 97D.
        for e in &rep.events {
            let json = serde_json::to_string(e).expect("serializável");
            assert!(!json.contains("state"), "sem estado cru no evento");
        }
    }
}
