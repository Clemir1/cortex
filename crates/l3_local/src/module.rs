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
use triad_l1_substrate as l1;
use triad_l2_tissue as l2;
use triad_runtime as rt;
use tracing::{debug, trace};

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
    /// Fonte do sinal Chladni (ponte read-only ao L1 — ADR-0007):
    /// a atenção L3 realimenta a saliência com a harmonia do substrato.
    /// None = recursão desligada (comportamento A/A anterior).
    chladni_source: Option<Arc<l1::ClusterModule>>,
    /// Ticks com bônus de ressonância APLICADO (telemetria honesta).
    chladni_bonus_ticks: Mutex<u64>,
    /// Ticks sem bônus (sem fonte, sinal não-VALUE ou sem tecidos) —
    /// denominador sempre visível, nunca zero fantasma.
    chladni_absent_ticks: Mutex<u64>,
    /// 17.5: representação real agregada (store/binding/onto/temporal).
    representation: Mutex<crate::representation::L3Representation>,
    /// Memória episódica local REAL (seção 16.2): consumidora dos
    /// reforços confirmados do ciclo L4 — o L3 é o DONO do estado.
    memory: Mutex<crate::memory::LocalMemory>,
    /// Inbox de reforços L4→L3 (fila no dono; o L4 apenas submete).
    reinforce_inbox: Mutex<Vec<crate::reinforcement::Reinforcement>>,
    /// Último recibo de reforço aplicado (auditoria da trilha).
    reinforce_receipt: Mutex<Option<crate::reinforcement::ReinforcementReceipt>>,
    /// Reforços aplicados / reconsolidados / descartados por overflow
    /// (telemetria honesta com denominador).
    reinforce_applied: Mutex<u64>,
    reinforce_reconsolidated: Mutex<u64>,
    reinforce_dropped: Mutex<u64>,
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

    /// Liga a RECURSÃO Chladni (ADR-0007): a atenção L3 passa a
    /// consumir o sinal de harmonia do substrato por ponte read-only
    /// (o scheduler cria um contexto por módulo — o sinal trafega por
    /// handle, padrão das pontes L1→L2→L3→L4 do projeto).
    pub fn with_chladni_source(mut self, l1: Arc<l1::ClusterModule>) -> Self {
        self.chladni_source = Some(l1);
        self
    }

    /// Telemetria da recursão Chladni: (ticks com bônus, ticks sem).
    /// Ausência de fonte/sinal conta como "sem bônus" — denominador
    /// sempre visível (ausência ≠ zero).
    pub fn chladni_stats(&self) -> (u64, u64) {
        (
            *self
                .chladni_bonus_ticks
                .lock()
                .unwrap_or_else(|p| p.into_inner()),
            *self
                .chladni_absent_ticks
                .lock()
                .unwrap_or_else(|p| p.into_inner()),
        )
    }

    fn with_l2(l2: Option<Arc<l2::TissueModule>>) -> Self {
        Self {
            // Descritor canônico (CAMADA.txt, 17.1): L3 prediz e
            // observa localmente (O1+O4); consome o tecido L2 e o
            // sinal Chladni por pontes read-only; atua no L2 apenas
            // por SUBMISSÃO ao inbox de adaptação do dono; memória
            // episódica real aplicada com recibo (E4, trilha 16.2).
            descriptor: tc::ModuleDescriptor::new(
                tf::id::ModuleId::new(),
                "l3.local",
                tc::Layer::L3,
                "local-cognition",
            )
            .with_orders(&[
                tc::CyberneticOrder::O1Control,
                tc::CyberneticOrder::O4Observation,
            ])
            .with_state_owner("l3.local")
            .with_inputs(&["runtime.tick", "l2.tissue", "l1.chladni"])
            .with_outputs(&["l3.local", "l3.memory"])
            .with_actuators(&["inbox.l2.adaptation"])
            .with_backend(tc::ExecutionBackend::CpuSeq)
            .with_criticality(tc::Criticality::High)
            .with_dependencies(&["l1.substrate", "l2.tissue"])
            .with_evidence(tf::evidence::EvidenceLevel::E4Consumed)
            .with_recovery(tc::RecoveryPolicy::RestartModule),
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
            chladni_source: None,
            chladni_bonus_ticks: Mutex::new(0),
            chladni_absent_ticks: Mutex::new(0),
            // 17.5: representação real (store/binding/ontogenética/
            // temporal) — agregador único, contadores com denominador.
            representation: Mutex::new(crate::representation::L3Representation::new()),
            memory: Mutex::new(crate::memory::LocalMemory::new()),
            reinforce_inbox: Mutex::new(Vec::new()),
            reinforce_receipt: Mutex::new(None),
            reinforce_applied: Mutex::new(0),
            reinforce_reconsolidated: Mutex::new(0),
            reinforce_dropped: Mutex::new(0),
            state: Mutex::new(rt::ModuleState::Active),
        }
    }

    /// TRILHA L4→L3 (seção 16.2): submete um reforço confirmado do
    /// ciclo de aprendizado à fila do DONO (o L4 nunca escreve estado
    /// alheio). Overflow = descarte contado com razão, nunca silencioso.
    pub fn submit_reinforcement(&self, reinforcement: crate::reinforcement::Reinforcement) {
        let mut inbox = self.reinforce_inbox.lock().unwrap_or_else(|p| p.into_inner());
        if inbox.len() >= crate::reinforcement::INBOX_CAPACITY {
            *self.reinforce_dropped.lock().unwrap_or_else(|p| p.into_inner()) += 1;
            tracing::warn!(
                rotulo = %reinforcement.label,
                capacidade = crate::reinforcement::INBOX_CAPACITY,
                "reforco descartado por inbox cheio (contado, nao silencioso)"
            );
            return;
        }
        inbox.push(reinforcement);
    }

    /// Drena a fila de reforços NO TICK L3 (o dono aplica): cada reforço
    /// grava/reconsolida o episódio na memória episódica real, emite
    /// recibo e publica evento por ocorrência.
    fn drain_reinforcements(&self, tick: u64, out: &mut Vec<tc::EventEnvelope>) {
        let inbox: Vec<crate::reinforcement::Reinforcement> =
            std::mem::take(&mut *self.reinforce_inbox.lock().unwrap_or_else(|p| p.into_inner()));
        if inbox.is_empty() {
            return;
        }
        let mut memory = self.memory.lock().unwrap_or_else(|p| p.into_inner());
        let mut rep = self
            .representation
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        for r in inbox {
            let reconsolidated = memory.reinforce(&r.label, r.step, r.reconsolidate);
            // 17.5: o reforço confirmado (trilha 16.2) é EXPERIÊNCIA
            // ontogenética — entra na janela de consolidação com
            // veredito tipado (Consolidated/Reconsolidated/WindowOpen).
            rep.onto.experience(&r.label, tick);
            *self.reinforce_applied.lock().unwrap_or_else(|p| p.into_inner()) += 1;
            if reconsolidated {
                *self.reinforce_reconsolidated.lock().unwrap_or_else(|p| p.into_inner()) += 1;
            }
            let receipt = crate::reinforcement::ReinforcementReceipt {
                label: r.label.clone(),
                applied_step: tick,
                reconsolidated,
            };
            *self.reinforce_receipt.lock().unwrap_or_else(|p| p.into_inner()) = Some(receipt);
            out.push(rt::envelope(
                "l3.memory",
                tick,
                tc::EventType::Learning,
                tc::Priority::Normal,
            ));
        }
    }

    /// Recall qualificado da memória episódica (auditoria da trilha —
    /// ausência ≠ zero: sem episódio = NO_DATA com razão).
    pub fn memory_recall(&self, label: &str, current_step: u64) -> tc::Qualified<f32> {
        self.memory
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .recall(label, current_step, self.descriptor.module_id.clone(), tf::StepId::new())
    }

    /// Telemetria da trilha L4→L3: (aplicados, reconsolidados,
    /// descartados por overflow, fila pendente).
    pub fn reinforcement_stats(&self) -> (u64, u64, u64, usize) {
        (
            *self.reinforce_applied.lock().unwrap_or_else(|p| p.into_inner()),
            *self.reinforce_reconsolidated.lock().unwrap_or_else(|p| p.into_inner()),
            *self.reinforce_dropped.lock().unwrap_or_else(|p| p.into_inner()),
            self.reinforce_inbox.lock().unwrap_or_else(|p| p.into_inner()).len(),
        )
    }

    /// Fotografia corrente dos focos de atenção.
    pub fn attention_foci(&self) -> AttentionFoci {
        self.attention
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .snapshot()
    }

    /// 17.5: fotografia da representação real COM DENOMINADORES
    /// (conceitos/bindings/consolidações/temporal/atratores).
    pub fn representation_report(&self) -> crate::representation::RepReport {
        self.representation
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .report()
    }

    /// 17.4: sinais top-down tipados a partir do CAMPO REAL — um
    /// por foco ativo, intensidade pelo decay posicional e razão
    /// explícita. O tecido-alvo é decidido pelo ROUTER do L2.
    pub fn topdown_signals(&self) -> Vec<crate::topdown::TopDownSignal> {
        self.attention_foci()
            .foci
            .iter()
            .enumerate()
            .map(|(pos, concept)| crate::topdown::TopDownSignal {
                concept: *concept,
                intensity: crate::topdown::focus_decay(pos),
                reason: format!("foco {pos} do campo de atenção pede realce"),
            })
            .collect()
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
    /// Bônus de ressonância Chladni corrente (ADR-0007): a ressonância
    /// da última observação do substrato, se a ponte está ligada e o
    /// sinal é VALUE — ausência é `None`, nunca 0 fantasma.
    fn chladni_bonus(&self) -> Option<f32> {
        self.chladni_source
            .as_ref()?
            .last_chladni()
            .resonance
            .as_ref_value()
            .copied()
    }

    pub fn read_for_l4(&self) -> L3Snapshot {
        // Focos com pesos: reprocessa as views correntes com a mesma
        // saliência do tick (fotografia é derivada, não mutada) — e,
        // com a recursão Chladni LIGADA (ADR-0007), soma o bônus de
        // ressonância com o peso de `[l3.attention]`. Sem fonte
        // chladni o resultado é IDÊNTICO ao anterior (A/A preservado);
        // a harmonia só entra quando a ponte existe.
        let chladni_bonus = self.chladni_bonus();
        let mut foci: Vec<(tf::id::ConceptId, f32)> = Vec::new();
        let (coherence_mean, coverage) = match &self.l2 {
            Some(l2) => {
                let (snapshot, _receipt) = l2.read_for_l3("l4.cognition");
                let views = l2.tissue_views();
                let mut concepts = self.concepts.lock().unwrap_or_else(|p| p.into_inner());
                for v in &views {
                    let concept = concepts.concept_of(v.tissue_id);
                    let mut salience = Self::salience_of(v);
                    if let Some(bonus) = chladni_bonus {
                        salience += bonus * self.attention_cfg.chladni_bonus_weight;
                    }
                    foci.push((concept, salience));
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
        //
        // ADR-0007 — RECURSÃO CHLADNI: bônus de ressonância na
        // saliência (legado: attention += bonus * 0.15). O sinal vem
        // da ponte read-only ao L1 (última observação DESTE tick — o
        // L1 ticka antes do L3). Ausência (sem fonte ligada, sinal
        // não-VALUE ou sem tecidos) ou peso 0 = SEM efeito — contada
        // com denominador visível, nunca zero fantasma.
        let chladni_bonus = self.chladni_bonus();
        // 17.5 — PONTE TEMPORAL HOTM→semântica + tick da
        // representação: assinatura recorrente liga conceito
        // groundado; janelas ontogenéticas fecham; ativações
        // decaem (assintótico — ausência nunca vira zero de vez).
        {
            let mut rep = self
                .representation
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            if let Some(src) = self.chladni_source.as_ref() {
                let obs = src.last_chladni();
                if let Some(sig) = crate::memory::ResonanceSignature::from_observation(&obs) {
                    rep.observe_chladni(&sig, tick);
                }
            }
            rep.tick(tick);
        }
        let bonus_aplicado = chladni_bonus
            .filter(|_| valid && self.attention_cfg.chladni_bonus_weight > 0.0);
        if bonus_aplicado.is_some() {
            *self
                .chladni_bonus_ticks
                .lock()
                .unwrap_or_else(|p| p.into_inner()) += 1;
            trace!(bonus = ?bonus_aplicado, "atenção l3: bônus de ressonância chladni aplicado");
        } else {
            *self
                .chladni_absent_ticks
                .lock()
                .unwrap_or_else(|p| p.into_inner()) += 1;
        }
        let mut field = AttentionField::with_config(tick, self.attention_cfg.clone());
        if valid {
            let views = match &self.l2 {
                Some(l2) => l2.tissue_views(),
                None => Vec::new(),
            };
            let mut concepts = self.concepts.lock().unwrap_or_else(|p| p.into_inner());
            for v in &views {
                let concept = concepts.concept_of(v.tissue_id);
                let mut salience = Self::salience_of(v);
                if let Some(bonus) = bonus_aplicado {
                    salience += bonus * self.attention_cfg.chladni_bonus_weight;
                }
                field.salient(concept, salience);
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

        // (6) TRILHA L4→L3: drena reforços confirmados para a memória
        // episódica real (o dono aplica; recibo + evento por ocorrência).
        self.drain_reinforcements(tick, out);

        // (7) Evento pequeno do tick L3.
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

        /// Ciclo com a recursão Chladni LIGADA (ponte read-only ao L1).
        fn with_chladni(seed: u64, population: usize) -> Self {
            let l1 = Arc::new(l1::ClusterModule::new(seed, population));
            let l2 = Arc::new(l2::TissueModule::new(l1.shared_runner(), seed));
            let l3 = Arc::new(
                L3Module::new_with_substrate(Arc::clone(&l2))
                    .with_chladni_source(Arc::clone(&l1)),
            );
            Self { l1, l2, l3 }
        }

        /// Ciclo com a recursão Chladni LIGADA e config L3 injetada.
        fn with_l3_config(seed: u64, population: usize, cfg: L3Config) -> Self {
            let l1 = Arc::new(l1::ClusterModule::new(seed, population));
            let l2 = Arc::new(l2::TissueModule::new(l1.shared_runner(), seed));
            let l3 = Arc::new(
                L3Module::new_with_config(Arc::clone(&l2), cfg)
                    .with_chladni_source(Arc::clone(&l1)),
            );
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

        /// Focos COM PESOS por tick (via read_for_l4) — material do
        /// experimento A/A do bônus de ressonância.
        fn pesos_por_tick(
            &self,
            ticks: usize,
        ) -> Vec<Vec<(tf::id::ConceptId, f32)>> {
            let mut out = Vec::new();
            for _ in 0..ticks {
                self.tick();
                out.push(self.l3.read_for_l4().foci);
            }
            out
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

    #[test]
    fn bonus_chladni_e_reproduzivel_e_deterministico() {
        // ADR-0007 — experimento A/A do bônus: (1) com a recursão
        // ligada, o EFEITO é reproduzível (duas runs bit-idênticas);
        // (2) contra o L3 sem recursão, os pesos DIFEREM — o bônus
        // não é teatral; (3) telemetria com denominador visível.
        let com_a = Ciclo::with_chladni(42, 24);
        let com_b = Ciclo::with_chladni(42, 24);
        let sem = Ciclo::new(42, 24);
        let pa = com_a.pesos_por_tick(25);
        let pb = com_b.pesos_por_tick(25);
        let ps = sem.pesos_por_tick(25);
        assert_eq!(pa, pb, "mesma seed + recursão ligada ⇒ bit-idêntico");
        let difere = pa.iter().zip(ps.iter()).any(|(a, b)| a != b);
        assert!(difere, "o bônus de ressonância muda os pesos de saliência");
        // Os pesos COM bônus são maiores (bônus aditivo, nunca negativo).
        let (w_com, w_sem) = (pa[0][0].1, ps[0][0].1);
        assert!(
            w_com > w_sem,
            "saliência com bônus ({w_com}) > sem ({w_sem})"
        );
        let (bonus_ticks, _) = com_a.l3.chladni_stats();
        assert!(bonus_ticks >= 20, "bônus aplicado na maioria dos ticks: {bonus_ticks}");
    }

    #[test]
    fn peso_zero_mantem_aa_bit_identico_ao_sem_fonte() {
        // Política A/A do peso: chladni_bonus_weight = 0 com a fonte
        // ligada == L3 sem recursão (efeito zero é zero de verdade).
        let cfg0 = L3Config {
            attention: AttentionCfg {
                chladni_bonus_weight: 0.0,
                ..AttentionCfg::default()
            },
            ..L3Config::default()
        };
        let zero = Ciclo::with_l3_config(42, 24, cfg0);
        let sem = Ciclo::new(42, 24);
        let pz = zero.pesos_por_tick(25);
        let ps = sem.pesos_por_tick(25);
        assert_eq!(
            pz, ps,
            "peso 0 ⇒ idêntico ao L3 sem recursão (política desligável)"
        );
        let (bonus, _) = zero.l3.chladni_stats();
        assert_eq!(bonus, 0, "peso 0 nunca aplica bônus (contadores honestos)");
    }

    /// 17.5: ponte temporal HOTM→semântica com ciclo REAL (L1→L2→L3
    /// com fonte Chladni): observações contadas, conceitos só por
    /// ligação recorrente, decaimentos com denominador.
    #[test]
    fn representacao_temporal_integrada_com_denominadores() {
        let ciclo = Ciclo::with_chladni(42, 24);
        for _ in 0..3 {
            ciclo.tick();
        }
        let rep = ciclo.l3.representation_report();
        assert!(
            rep.observacoes_temporais >= 2,
            "ponte viva com sinal VALUE: {} observações",
            rep.observacoes_temporais,
        );
        assert_eq!(
            rep.conceitos, rep.ligacoes_temporais as usize,
            "todo conceito do store veio de ligação temporal (cenário puro)"
        );
        assert_eq!(rep.decaidas, 0, "janelas jovens não decaem em 3 ticks");
        assert_eq!(rep.eventos, 0, "memória ontogenética sem experiência ainda");
        let _ = rep.atratores; // denominador reportado, nunca taxa nua
    }
}
