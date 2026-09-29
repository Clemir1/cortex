//! Ponte da L3 para o runtime cognitivo — versão REAL.
//!
//! O L3 consome a visão tecidual do L2 **pelo módulo do tecido** (o
//! scheduler cria um contexto por módulo, então o caminho instrumentado
//! é o `TissueModule`, não o contexto): cada tick lê o snapshot L2 com
//! recibo E4 registrado no L2Ledger, alimenta a atenção com a saliência
//! REAL dos tecidos (coesão/especialização/integração), gera predição a
//! partir de evidência viva (não mais constante) e, com cadência
//! própria, **propõe** adaptação estrutural L3→L2 (`AdaptationRequest`).
//! Propor, nunca escrever: o gate do L2 decide com histerese.
//!
//! Sem tecido válido: focos vazios e predição ausente — nada fabricado.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use triad_contracts as tc;
use triad_foundation as tf;
use triad_l2_tissue as l2;
use triad_runtime as rt;
use tracing::debug;

use crate::attention::{AttentionCfg, AttentionField, AttentionFoci};
use crate::prediction::{Prediction, PredictionCfg, Predictor};

/// Cadência mínima entre propostas de adaptação (ticks).
pub const L3_PROPOSAL_INTERVAL: u64 = 30;
/// Coerência média abaixo disso autoriza propor tecidos mais coesos.
pub const L3_COHERENCE_FLOOR: f64 = 0.30;
/// Queda de coerência desde a última observação que configura
/// deterioração (tendência — a memória cognitiva do L3).
pub const L3_COHERENCE_DROP: f64 = 0.05;
/// Cobertura abaixo disso autoriza propor inclusão de clusters.
pub const L3_COVERAGE_FLOOR: f64 = 0.80;
/// Salto proposto por pedido (o gate do L2 clampa ao teto real de 0.1).
pub const L3_PROPOSAL_DELTA: f64 = 0.2;

/// Política L3→L2 — importada de `config/default.toml [l3.adaptation]`
/// (configuração centralizada; as consts acima são os defaults
/// congelados, herdados quando a seção não cita a chave).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct L3Policy {
    /// Cadência mínima entre propostas (ticks).
    pub proposal_interval_steps: u64,
    /// Piso absoluto de coerência média.
    pub coherence_floor: f64,
    /// Queda desde o pico da janela que configura deterioração.
    pub coherence_drop: f64,
    /// Piso de cobertura organizacional.
    pub coverage_floor: f64,
    /// Salto proposto por pedido (o gate do L2 decide de verdade).
    pub proposal_delta: f64,
}

impl Default for L3Policy {
    fn default() -> Self {
        Self {
            proposal_interval_steps: L3_PROPOSAL_INTERVAL,
            coherence_floor: L3_COHERENCE_FLOOR,
            coherence_drop: L3_COHERENCE_DROP,
            coverage_floor: L3_COVERAGE_FLOOR,
            proposal_delta: L3_PROPOSAL_DELTA,
        }
    }
}

/// Seção `[l3]` completa — porta de entrada da configuração L3.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct L3Config {
    /// `[l3.attention]` — limiar, capacidade e ganho do campo.
    pub attention: AttentionCfg,
    /// `[l3.prediction]` — horizonte, confiança mínima, tolerância.
    pub prediction: PredictionCfg,
    /// `[l3.adaptation]` — política de propostas L3→L2.
    pub adaptation: L3Policy,
}

/// Índice determinístico tecido→conceito (o campo de atenção fala em
/// `ConceptId`; a ordem de primeira aparição gera ids estáveis).
#[derive(Debug, Default)]
struct TissueConceptIndex {
    map: HashMap<tf::id::TissueId, tf::id::ConceptId>,
    seq: u64,
}

impl TissueConceptIndex {
    fn concept_of(&mut self, tissue: tf::id::TissueId) -> tf::id::ConceptId {
        if let Some(c) = self.map.get(&tissue) {
            return *c;
        }
        self.seq += 1;
        let c = tf::id::ConceptId::from_uuid(uuid::Uuid::from_u64_pair(0xC0FF_EE, self.seq));
        self.map.insert(tissue, c);
        c
    }
}

/// Módulo L3: cognição local REAL sobre o tecido.
pub struct L3Module {
    descriptor: tc::ModuleDescriptor,
    /// Módulo do tecido L2 (ponte instrumentada; None ⇒ sem fonte).
    l2: Option<Arc<l2::TissueModule>>,
    /// Política de propostas vigente (importada de `config/`).
    policy: L3Policy,
    /// Política do campo de atenção (`[l3.attention]`).
    attention_cfg: AttentionCfg,
    attention: Mutex<AttentionField>,
    predictor: Mutex<Predictor>,
    concepts: Mutex<TissueConceptIndex>,
    /// Contagem de ticks entre propostas (cadência própria).
    proposal_countdown: Mutex<u64>,
    /// Total de propostas submetidas (telemetria honesta).
    proposals_total: Mutex<u64>,
    /// Pico de coerência desde a última proposta (linha de base da
    /// tendência — a memória cognitiva do L3).
    best_coherence: Mutex<Option<f64>>,
    /// Último recibo da leitura L2 (auditoria da ponte).
    last_receipt: Mutex<Option<tc::l2::L2ReadReceipt>>,
    state: Mutex<rt::ModuleState>,
}

/// PONTE L3→L4: fotografia corrente para a cognição global.
///
/// Leitura READ-ONLY do dono L3 (observação nunca muta — f5 achado 3):
/// focos com pesos de saliência, predição corrente e sinais
/// estruturais do tecido (coerência/cobertura). O L4 nunca escreve.
#[derive(Debug, Clone)]
pub struct L3Snapshot {
    /// Focos correntes com saliência, do mais ao menos saliente.
    pub foci: Vec<(tf::id::ConceptId, f32)>,
    /// Última predição emitida (com confiança); None = ausência.
    pub prediction: Option<Prediction>,
    /// Coerência média dos tecidos (None = sem fonte viva).
    pub coherence_mean: Option<f64>,
    /// Cobertura organizacional (None = sem fonte viva).
    pub coverage: Option<f32>,
    /// Tick da fotografia (proveniência da leitura).
    pub step: u64,
}

impl L3Module {
    /// Cria o módulo L3 sem tecido (modo honesto: sem fonte, publica
    /// ausência, nunca fabrica focos).
    pub fn new() -> Self {
        Self::with_l2(None)
    }

    /// Cria o módulo L3 REAL sobre o tecido do L2 (política default).
    pub fn new_with_substrate(l2: Arc<l2::TissueModule>) -> Self {
        Self::with_l2(Some(l2))
    }

    /// Cria o módulo L3 REAL com a política de propostas injetada
    /// (atenção e predição com defaults congelados).
    pub fn new_with_policy(l2: Arc<l2::TissueModule>, policy: L3Policy) -> Self {
        let mut m = Self::with_l2(Some(l2));
        m.policy = policy;
        m
    }

    /// Cria o módulo L3 REAL com a configuração completa injetada de
    /// `config/default.toml [l3.*]` (atenção, predição e adaptação).
    pub fn new_with_config(l2: Arc<l2::TissueModule>, config: L3Config) -> Self {
        let mut m = Self::with_l2(Some(l2));
        m.policy = config.adaptation;
        m.attention_cfg = config.attention;
        *m.predictor.lock().unwrap_or_else(|p| p.into_inner()) =
            Predictor::with_config(config.prediction);
        m
    }

    fn with_l2(l2: Option<Arc<l2::TissueModule>>) -> Self {
        Self {
            descriptor: tc::ModuleDescriptor {
                module_id: tf::id::ModuleId::new(),
                name: "l3.local".into(),
                layer: tc::Layer::L3,
                domain: "local-cognition".into(),
            },
            l2,
            policy: L3Policy::default(),
            attention_cfg: AttentionCfg::default(),
            attention: Mutex::new(AttentionField::new(0)),
            predictor: Mutex::new(Predictor::new()),
            concepts: Mutex::new(TissueConceptIndex::default()),
            proposal_countdown: Mutex::new(0),
            proposals_total: Mutex::new(0),
            best_coherence: Mutex::new(None),
            last_receipt: Mutex::new(None),
            state: Mutex::new(rt::ModuleState::Active),
        }
    }

    /// Fotografia corrente dos focos de atenção.
    pub fn attention_foci(&self) -> AttentionFoci {
        self.attention
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .snapshot()
    }

    /// Última predição emitida (None = ausência declarada).
    pub fn last_prediction(&self) -> Option<Prediction> {
        self.predictor
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .latest_qualified(self.descriptor.module_id, tf::id::StepId::new())
            .as_ref_value()
            .cloned()
    }

    /// Total de propostas de adaptação submetidas ao L2.
    pub fn proposals_total(&self) -> u64 {
        *self
            .proposals_total
            .lock()
            .unwrap_or_else(|p| p.into_inner())
    }

    /// Último recibo da leitura L2→L3 (auditoria da ponte).
    pub fn last_receipt(&self) -> Option<tc::l2::L2ReadReceipt> {
        self.last_receipt
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    /// Lê o snapshot L2 com recibo E4 (ou ausência com razão).
    fn read_l2(&self) -> (Option<tc::l2::TissueSnapshot>, Option<tc::l2::L2ReadReceipt>) {
        match &self.l2 {
            None => (None, None),
            Some(l2) => {
                let (snapshot, receipt) = l2.read_for_l3("l3.cognition");
                *self.last_receipt.lock().unwrap_or_else(|p| p.into_inner()) =
                    Some(receipt.clone());
                (snapshot, Some(receipt))
            }
        }
    }

    /// PONTE L3→L4: fotografia corrente para o workspace global.
    ///
    /// Somente leitura (a observação nunca muta o dono): focos com
    /// pesos da ÚLTIMA fotografia, predição corrente e sinais
    /// estruturais. Sem fonte L2 os campos ficam vazios — ausência
    /// nunca vira valor fabricado.
    pub fn read_for_l4(&self) -> L3Snapshot {
        // Focos com pesos: reprocessa as views correntes com a mesma
        // saliência do tick (fotografia é derivada, não mutada).
        let mut foci: Vec<(tf::id::ConceptId, f32)> = Vec::new();
        let (coherence_mean, coverage) = match &self.l2 {
            Some(l2) => {
                let (snapshot, _receipt) = l2.read_for_l3("l4.cognition");
                let views = l2.tissue_views();
                let mut concepts = self.concepts.lock().unwrap_or_else(|p| p.into_inner());
                for v in &views {
                    let concept = concepts.concept_of(v.tissue_id);
                    foci.push((concept, Self::salience_of(v)));
                }
                drop(concepts);
                match snapshot {
                    Some(s) if s.provider_status == tf::status::Status::Value => {
                        (s.coherence_mean, s.assignment_coverage)
                    }
                    _ => (None, None),
                }
            }
            None => (None, None),
        };
        let prediction = self
            .predictor
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .last();
        let step = self
            .attention
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .snapshot()
            .step;
        L3Snapshot {
            foci,
            prediction,
            coherence_mean,
            coverage,
            step,
        }
    }

    /// Saliência de um tecido: média das três qualidades reais.
    fn salience_of(v: &tc::TissueState) -> f32 {
        (v.cohesion.value() + v.specialization.value() + v.integration.value()) / 3.0
    }

    /// Política de adaptação L3→L2 com cadência: deterioração ⇒ proposta.
    /// Queda de coerência desde o PICO da janela (tendência — memória
    /// cognitiva do L3; sondas reais mostram declínio natural 0.98→0.85)
    /// ⇒ tecidos mais coesos (threshold maior); coerência abaixo do piso
    /// absoluto ⇒ idem; cobertura abaixo do piso ⇒ inclusão (threshold
    /// menor). Propor; o gate do L2 decide. A linha de base só reseta
    /// quando uma proposta é feita.
    fn maybe_propose(&self, snapshot: &tc::l2::TissueSnapshot) {
        let Some(l2) = &self.l2 else { return };
        // Coerência média ausente (sem tecidos) ⇒ nada a propor.
        let Some(coherence) = snapshot.coherence_mean else {
            return;
        };
        // Memória de tendência: observa TODOS os ticks (mesmo sem
        // propor) — a cadência limita propostas, nunca a observação.
        let mut best_c = self.best_coherence.lock().unwrap_or_else(|p| p.into_inner());
        let best = best_c.unwrap_or(coherence);
        let drop_from_best = (best - coherence).max(0.0);
        if drop_from_best < L3_COHERENCE_DROP {
            *best_c = Some(best.max(coherence));
        }
        drop(best_c);
        let mut countdown = self
            .proposal_countdown
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        if *countdown > 0 {
            *countdown -= 1;
            return;
        }
        *countdown = self.policy.proposal_interval_steps;
        let (parameter, proposal, reset_baseline) = if drop_from_best >= self.policy.coherence_drop {
            // Deterioração em curso: reforça a coesão dos tecidos.
            (
                "l2.affinity.threshold",
                0.25 + self.policy.proposal_delta,
                true,
            )
        } else if coherence < self.policy.coherence_floor {
            (
                "l2.affinity.threshold",
                0.25 + self.policy.proposal_delta,
                true,
            )
        } else if (snapshot.assignment_coverage.unwrap_or(0.0) as f64)
            < self.policy.coverage_floor
        {
            (
                "l2.affinity.threshold",
                (0.25 - self.policy.proposal_delta).max(0.05),
                false,
            )
        } else {
            return; // dentro dos pisos: não propõe
        };
        let req = tc::AdaptationRequest {
            requester: self.descriptor.module_id,
            target: tf::id::ModuleId::new(),
            parameter: parameter.to_string(),
            current: String::new(), // o gate lê o estado REAL do L2
            proposed: format!("{proposal:.3}"),
        };
        l2.submit_adaptation(req);
        *self.proposals_total.lock().unwrap_or_else(|p| p.into_inner()) += 1;
        // Proposta feita ⇒ nova linha de base para a próxima janela.
        if reset_baseline {
            *self.best_coherence.lock().unwrap_or_else(|p| p.into_inner()) = Some(coherence);
        }
        debug!(
            coerencia = coherence,
            cobertura = snapshot.assignment_coverage.unwrap_or(0.0),
            parametro = parameter,
            "l3 propôe adaptação estrutural ao l2"
        );
    }
}

impl rt::CognitiveModule for L3Module {
    /// Descritor imutável do módulo.
    fn descriptor(&self) -> &tc::ModuleDescriptor {
        &self.descriptor
    }

    /// Estado atual do módulo, mesmo se o lock estiver envenenado.
    fn state(&self) -> rt::ModuleState {
        *self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Um tick: lê o tecido com recibo, fotografa focos REAIS, prediz
    /// com evidência viva e (com cadência) propõe adaptação ao L2.
    fn tick(
        &self,
        ctx: &rt::TypedContext,
        out: &mut Vec<tc::EventEnvelope>,
    ) -> tf::TriadResult<()> {
        let tick = ctx.clock().tick;

        // (1) PONTE L2→L3: leitura instrumentada com recibo E4 no
        // L2Ledger — ausência também viaja com razão.
        let (snapshot, receipt) = self.read_l2();
        let valid = snapshot
            .as_ref()
            .map(|s| s.provider_status == tf::status::Status::Value)
            .unwrap_or(false);

        // (2) Atenção: saliência REAL dos tecidos vivos. Campo novo a
        // cada tick (fotografia corrente, sem acúmulo de fantasma) com
        // a política de `[l3.attention]` injetada.
        let mut field = AttentionField::with_config(tick, self.attention_cfg.clone());
        if valid {
            let views = match &self.l2 {
                Some(l2) => l2.tissue_views(),
                None => Vec::new(),
            };
            let mut concepts = self.concepts.lock().unwrap_or_else(|p| p.into_inner());
            for v in &views {
                let concept = concepts.concept_of(v.tissue_id);
                field.salient(concept, Self::salience_of(v));
            }
        }
        let foci = field.snapshot();
        *self.attention.lock().unwrap_or_else(|p| p.into_inner()) = field;

        // (3) Predição: evidência VIVA do snapshot (não mais constante).
        let evidence: Vec<(String, f32)> = match &snapshot {
            Some(s) if valid => vec![
                (
                    "coesao_tecidual".into(),
                    s.coherence_mean.unwrap_or(0.0) as f32,
                ),
                (
                    "cobertura_organizacional".into(),
                    s.assignment_coverage.unwrap_or(0.0),
                ),
                (
                    "energia_organizacional".into(),
                    s.mean_energy.unwrap_or(0.0) as f32,
                ),
            ],
            _ => Vec::new(), // sem fonte ⇒ sem predição (ausência)
        };
        self.predictor
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .predict(&evidence, tick);

        // Trilha de auditoria do tick L3: focos e fonte da leitura.
        debug!(
            focos = foci.foci.len(),
            fonte_l2 = valid,
            recibo = receipt.is_some(),
            "l3.local tick publicado"
        );

        // (4) Publica focos, predição e recibo como valores qualificados
        // (padrão dos slots L3: tipos em `Qualified` no contexto).
        ctx.set(
            "l3.attention.foci",
            tc::Qualified::value(foci, self.descriptor.module_id.clone(), ctx.clock().step),
        );
        let prediction = {
            let predictor = self.predictor.lock().unwrap_or_else(|p| p.into_inner());
            predictor.latest_qualified(self.descriptor.module_id.clone(), ctx.clock().step)
        };
        ctx.set("l3.prediction.latest", prediction);
        if let Some(receipt) = receipt {
            ctx.set("l3.l2.receipt", receipt);
        }

        // (5) PONTE L3→L2: propõe adaptação com cadência (o gate decide).
        if let Some(s) = &snapshot {
            self.maybe_propose(s);
        }

        // (6) Evento pequeno do tick L3.
        out.push(rt::envelope(
            "l3.local",
            tick,
            tc::EventType::Cognitive,
            tc::Priority::Normal,
        ));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use triad_l1_substrate as l1;
    use triad_runtime::CognitiveModule as _;
    use triad_runtime::TypedContext;

    /// Um ciclo real L1→L2→L3 por tick (a ordem do scheduler).
    struct Ciclo {
        l1: Arc<l1::ClusterModule>,
        l2: Arc<l2::TissueModule>,
        l3: Arc<L3Module>,
    }

    impl Ciclo {
        fn new(seed: u64, population: usize) -> Self {
            let l1 = Arc::new(l1::ClusterModule::new(seed, population));
            let l2 = Arc::new(l2::TissueModule::new(l1.shared_runner(), seed));
            let l3 = Arc::new(L3Module::new_with_substrate(Arc::clone(&l2)));
            Self { l1, l2, l3 }
        }

        fn tick(&self) {
            let ctx = TypedContext::new(tf::LogicalClock::new());
            let mut out = Vec::new();
            // L1 primeiro (física), L2 (tecido), L3 (cognição local).
            self.l1.tick(&ctx, &mut out).expect("tick l1");
            self.l2.tick(&ctx, &mut out).expect("tick l2");
            self.l3.tick(&ctx, &mut out).expect("tick l3");
        }
    }

    #[test]
    fn ciclo_real_registra_recibo_e4_do_l3_no_l2_ledger() {
        let ciclo = Ciclo::new(42, 24);
        for _ in 0..5 {
            ciclo.tick();
        }
        // Uma leitura E4 por tick L3 no ledger do L2.
        let l2 = ciclo.l2.l2();
        let (cv, _ca) = l2.ledger.consumed_counts();
        assert_eq!(cv, 5, "L3 consumiu o snapshot L2 a cada tick");
        assert!(l2.ledger.receipts().iter().any(|r| r.key == "l3.cognition"));
        // O recibo também fica guardado no módulo L3 (auditoria).
        let receipt = ciclo.l3.last_receipt().expect("recibo guardado");
        assert_eq!(receipt.key, "l3.cognition");
        assert_eq!(receipt.state_phase, Some(tc::l1::L1StatePhase::PostTissue));
    }

    #[test]
    fn atencao_e_predicao_alimentadas_por_dados_reais() {
        let ciclo = Ciclo::new(11, 24);
        let mut teve_focos = false;
        let mut teve_predicao = false;
        for _ in 0..20 {
            ciclo.tick();
            if ciclo.l3.attention_foci().foci.len() > 0 {
                teve_focos = true;
            }
            if ciclo.l3.last_prediction().is_some() {
                teve_predicao = true;
            }
        }
        assert!(teve_focos, "saliência real dos tecidos gerou focos");
        assert!(
            teve_predicao,
            "evidência viva do L2 alimentou a predição"
        );
    }

    #[test]
    fn l3_propoe_adaptacao_com_cadencia_e_gate_decide_com_razao() {
        let ciclo = Ciclo::new(33, 24);
        for _ in 0..61 {
            ciclo.tick();
        }
        assert!(
            ciclo.l3.proposals_total() >= 1,
            "L3 propôs ao menos uma adaptação (cadência 30)"
        );
        // A proposta foi drenada pelo tick L2 seguinte e contada pelo
        // gate — o L3 propôs, o gate DECIDIU (aplicou ou adiou, com razão).
        let l2 = ciclo.l2.l2();
        assert!(l2.gate.received >= 1, "gate recebeu a proposta");
        let decididos = l2.gate.applied
            + l2.gate.deferred_band
            + l2.gate.deferred_interval
            + l2.gate.deferred_range
            + l2.gate.deferred_unknown
            + l2.gate.deferred_unparseable
            + l2.gate.deferred_invalid;
        assert!(
            decididos >= 1,
            "gate decidiu sobre a proposta (contadores nunca mudos)"
        );
    }

    #[test]
    fn sem_l2_o_l3_publica_ausencia_e_nao_fabrica() {
        let l3 = L3Module::new();
        let ctx = TypedContext::new(tf::LogicalClock::new());
        let mut out = Vec::new();
        l3.tick(&ctx, &mut out).expect("tick nunca entra em pânico");
        // Sem fonte: sem recibo, sem focos, sem predição, sem proposta.
        assert!(l3.last_receipt().is_none());
        assert_eq!(l3.attention_foci().foci.len(), 0);
        assert!(l3.last_prediction().is_none());
        assert_eq!(l3.proposals_total(), 0);
    }

    #[test]
    fn ciclo_aa_deterministico_focos_e_conceitos_estaveis() {
        let a = Ciclo::new(42, 24);
        let b = Ciclo::new(42, 24);
        let mut foci_a: Option<Vec<tf::id::ConceptId>> = None;
        let mut foci_b: Option<Vec<tf::id::ConceptId>> = None;
        for _ in 0..25 {
            a.tick();
            let f = a.l3.attention_foci();
            if !f.foci.is_empty() {
                foci_a = Some(f.foci);
            }
        }
        for _ in 0..25 {
            b.tick();
            let f = b.l3.attention_foci();
            if !f.foci.is_empty() {
                foci_b = Some(f.foci);
            }
        }
        assert_eq!(
            foci_a, foci_b,
            "mesma seed ⇒ tecidos, saliência e conceitos idênticos"
        );
    }
}
