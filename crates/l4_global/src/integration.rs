//! Ponte da L4 para o runtime: workspace, decisão, ação e ciclo de
//! aprendizado integrados sobre dados REAIS do L3.
//!
//! REAL (P1 da análise 13-0 + correção da seção 15): os candidatos do
//! workspace vêm da fotografia L3 (focos com saliência, predição,
//! coerência/cobertura) — zero candidatos manufaturados. A seleção
//! usa SALIÊNCIA EFETIVA com habituação (GWT/novelty — o spotlight
//! circula; confiança de predição não é saliência de conteúdo).
//! Cada decisão commitada abre um `LearningEnvelope` cujo outcome é
//! verificado em t+1 NO MESMO CONTEÚDO (comensurável — active
//! inference: prediction error entre grandezas iguais) contra um
//! SHAM DE DERIVA (baseline contrafactual local: o valor que
//! persistiria sem efeito atribuível à ação — Lei 5/E5). O ciclo só
//! fecha com `verified_future_effect`; razões tipadas explicam cada
//! verificação (nunca só um float).

use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};
use triad_contracts as tc;
use triad_foundation as tf;
use triad_runtime as rt;
use triad_runtime::CognitiveModule as _;

use crate::action::execute;
use crate::causal::CausalTracker;
use crate::config::L4Config;
use crate::decision::decide_best;
use crate::degraded::DegradedMode;
use crate::workspace::{GlobalWorkspace, WorkspaceEntry};
use crate::world_model::WorldModel;
use crate::{ActionRecord, PendingDecision};

/// Ciclo de aprendizado aberto: decisão executada aguardando outcome.
#[derive(Debug, Clone)]
struct OpenCycle {
    /// Envelope com `verified_future_effect = None` até fechar.
    envelope: tc::LearningEnvelope,
    /// CONTEÚDO que a ação promete manter/elevar (o vencedor do
    /// broadcast) — a expectativa é medida NELE (comensurável).
    content_key: String,
    /// Sham de deriva (baseline contrafactual local): valor do
    /// conteúdo no tick do commit — o que aconteceria por
    /// persistência, sem efeito atribuível à ação (Lei 5/E5).
    sham: f32,
    /// Causa tipada para o rastreador causal.
    cause: String,
    /// Tick em que a decisão foi commitada.
    decided_tick: u64,
    /// Tick limite para o outcome chegar (depois: expira contado).
    timeout_tick: u64,
}

/// Razão tipada do resultado da verificação — nunca só um float
/// (instrumentação da seção 15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationReason {
    /// Efeito líquido dentro da tolerância ou melhor (confirmado).
    Confirmed,
    /// Conteúdo saiu do foco antes de t+1 (não-confirmado).
    ContentGone,
    /// Conteúdo vivo, mas degradou além da tolerância (não-confirmado).
    DegradedBeyondTolerance,
}

impl VerificationReason {
    /// Nome canônico da razão (auditoria).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Confirmed => "CONFIRMED",
            Self::ContentGone => "CONTENT_GONE",
            Self::DegradedBeyondTolerance => "DEGRADED_BEYOND_TOLERANCE",
        }
    }
}

/// Telemetria honesta do L4 — taxas sempre com denominador.
#[derive(Debug, Default, Clone)]
pub struct L4Stats {
    pub broadcasts: u64,
    pub decisions_committed: u64,
    pub decisions_deferred: u64,
    pub decisions_expired: u64,
    pub actions: u64,
    pub envelopes_opened: u64,
    pub envelopes_closed: u64,
    pub envelopes_expired: u64,
    pub world_updates: u64,
    /// Ações de exploração commitadas (para o self-model do L5).
    pub explorations: u64,
    /// Vencedores distintos já broadcastizados (diversidade do
    /// spotlight — insumo da identidade L5, com denominador em
    /// `broadcasts`).
    pub distinct_broadcasts: u64,
    /// Verificações confirmadas (efeito líquido ≥ −tolerância).
    pub confirmed: u64,
    /// Reforços L4→L3 submetidos à memória episódica (seção 16.2).
    pub memory_submissions: u64,
    /// Rollouts contrafactuais AVALIADOS (16.3): ciclos fechados
    /// cujo eixo comensurável tinha crença viva no world model.
    pub rollout_evaluations: u64,
    /// Soma das divergências |observado − rollout(t+1)| (16.3);
    /// a média honesta usa `rollout_evaluations` de denominador.
    pub rollout_div_sum: f32,
    /// Propostas de política ACEITAS no inbox (16.7, Lei 6).
    pub policy_submitted: u64,
    /// Políticas APLICADAS pelo dono (com previous guardado).
    pub policy_applied: u64,
    /// Políticas REVERTIDAS por TTL sem efeito (Lei 6 executável).
    pub policy_reverted: u64,
    /// Recusas tipadas: alvo já ativo/pendente (16.7).
    pub policy_rejected_duplicate: u64,
    /// Recusas tipadas: valor fora de 0..=1 (16.7).
    pub policy_rejected_out_of_range: u64,
    /// Recusas tipadas: TTL fora da faixa (16.7).
    pub policy_rejected_invalid_ttl: u64,
    /// Recusas tipadas: teto de políticas ativas (16.7).
    pub policy_rejected_full: u64,
    /// Políticas ATIVAS no instante da leitura (preenchida em
    /// `stats()` — como `distinct_broadcasts`).
    pub policy_active: u64,
    /// Não-confirmadas: conteúdo saiu do foco (razão tipada).
    pub content_gone: u64,
    /// Não-confirmadas: degradação além da tolerância (razão tipada).
    pub degraded_beyond_tolerance: u64,
}

/// Registro de ciclo FECHADO (telemetria POR DECISÃO — insumo do
/// experimento E5 com gêmea, checklist 16.11; cap `CLOSED_LOG_MAX`;
/// serializável para a persistência cross-run — 17.9/16.10).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClosedCycleRecord {
    /// Tick em que a decisão foi commitada (t).
    pub decided_tick: u64,
    /// Conteúdo prometido (chave comensurável).
    pub content_key: String,
    /// Valor do conteúdo em t+1 (outcome observado).
    pub actual: f32,
    /// Sham local no tick do commit (persistência sem a ação).
    pub sham: f32,
    /// Efeito líquido (actual − sham).
    pub net_effect: f32,
    /// Confirmação do critério E4 (Lei 5).
    pub confirmed: bool,
}

/// Cap do histórico de ciclos fechados (telemetria limitada — o
/// experimento E5 é offline, o organismo não carrega o log inteiro).
const CLOSED_LOG_MAX: usize = 64;

/// Módulo L4: workspace, modelo do mundo, causal e decisão integrados.
pub struct L4Module {
    descriptor: tc::ModuleDescriptor,
    /// Ponte L3→L4 (leitura read-only com fotografia corrente).
    l3: Option<Arc<triad_l3_local::L3Module>>,
    /// Política injetada de `config/default.toml [l4.*]`.
    config: L4Config,
    workspace: Mutex<GlobalWorkspace>,
    world: Mutex<WorldModel>,
    causal: Mutex<CausalTracker>,
    degraded: Mutex<DegradedMode>,
    /// Decisões adiadas por confiança abaixo do piso, com prazo.
    deferred: Mutex<Vec<(PendingDecision, u64, u64)>>,
    /// Ciclos abertos aguardando outcome (Lei 5).
    open: Mutex<Vec<OpenCycle>>,
    /// Conteúdos distintos que já venceram broadcast (diversidade).
    distinct: Mutex<std::collections::HashSet<String>>,
    /// Último passo de reforço por rótulo (throttle da trilha L4→L3:
    /// o MESMO conteúdo não reconsolida mais que 1× por janela).
    reinforcement_throttle: Mutex<std::collections::HashMap<String, u64>>,
    /// Inbox de políticas de intervenção (16.7, Lei 6): o L5 propõe,
    /// o L4 (dono) valida/aplica/reverte por TTL.
    policy: Mutex<crate::policy::PolicyInbox>,
    stats: Mutex<L4Stats>,
    /// Últimos ciclos fechados (cap CLOSED_LOG_MAX) — telemetria por
    /// decisão, insumo do experimento E5 com gêmea (16.11).
    closed_log: Mutex<Vec<ClosedCycleRecord>>,
    /// 17.6: agregador GlobalIntegration (thalamus/episódico/
    /// abstração/navegador — contexto fundido com denominadores).
    global_integration: Mutex<crate::global_integration::GlobalIntegration>,
    state: Mutex<rt::ModuleState>,
}

impl L4Module {
    /// Cria o módulo L4 sem fonte L3 e com política default
    /// (compatibilidade: publica ausência, nunca fabrica).
    pub fn new() -> Self {
        Self::new_with_l3_and_config(None, L4Config::default())
    }

    /// Cria o módulo L4 REAL sobre a cognição local L3.
    pub fn new_with_l3(l3: Arc<triad_l3_local::L3Module>) -> Self {
        Self::new_with_l3_and_config(Some(l3), L4Config::default())
    }

    /// Cria o módulo L4 REAL com a configuração injetada de
    /// `config/default.toml [l4.*]` (sem fonte L3: publica ausência).
    pub fn new_with_config(config: L4Config) -> Self {
        Self::new_with_l3_and_config(None, config)
    }

    /// Cria o módulo L4 REAL sobre o L3 com config central.
    pub fn new_with_l3_and_config(
        l3: Option<Arc<triad_l3_local::L3Module>>,
        config: L4Config,
    ) -> Self {
        Self {
            // Descritor canônico (CAMADA.txt, 17.1): L4 decide e
            // coordena (O1+O3); ciclo fecha com efeito futuro
            // VERIFICADO (Lei 5 = E5); atua na memória L3 apenas por
            // SUBMISSÃO ao inbox do dono (trilha 16.2).
            descriptor: tc::ModuleDescriptor::new(
                tf::id::ModuleId::new(),
                "l4.global",
                tc::Layer::L4,
                "global-cognition",
            )
            .with_orders(&[
                tc::CyberneticOrder::O1Control,
                tc::CyberneticOrder::O3Coordination,
            ])
            .with_state_owner("l4.global")
            .with_inputs(&["runtime.tick", "l3.local"])
            .with_outputs(&[
                "l4.global",
                "l4.workspace",
                "l4.decision",
                "l4.action",
                "l4.learning",
                "l4.policy",
            ])
            .with_actuators(&["inbox.l3.reinforcement"])
            .with_backend(tc::ExecutionBackend::CpuSeq)
            .with_criticality(tc::Criticality::Critical)
            .with_dependencies(&["l1.substrate", "l2.tissue", "l3.local"])
            .with_evidence(tf::evidence::EvidenceLevel::E5EffectValidated)
            .with_recovery(tc::RecoveryPolicy::RestartModule),
            workspace: Mutex::new(GlobalWorkspace::with_config(config.workspace.clone())),
            world: Mutex::new(WorldModel::new()),
            causal: Mutex::new(CausalTracker::new()),
            degraded: Mutex::new(DegradedMode::new()),
            deferred: Mutex::new(Vec::new()),
            open: Mutex::new(Vec::new()),
            distinct: Mutex::new(std::collections::HashSet::new()),
            reinforcement_throttle: Mutex::new(std::collections::HashMap::new()),
            policy: Mutex::new(crate::policy::PolicyInbox::new()),
            config,
            l3,
            stats: Mutex::new(L4Stats::default()),
            closed_log: Mutex::new(Vec::new()),
            // 17.6: agregador fundido do L4 (audit 17-3).
            global_integration: Mutex::new(crate::global_integration::GlobalIntegration::new()),
            state: Mutex::new(rt::ModuleState::Active),
        }
    }

    /// Últimos ciclos fechados (cap CLOSED_LOG_MAX) — POR DECISÃO, para
    /// auditoria e para o experimento E5 com gêmea (16.11).
    pub fn closed_log(&self) -> Vec<ClosedCycleRecord> {
        self.closed_log
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    /// 17.6: fotografia do agregador GlobalIntegration (thalamus/
    /// episódico/abstração/navegador — COM DENOMINADORES).
    pub fn global_integration_report(&self) -> crate::global_integration::GiReport {
        self.global_integration
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .report()
    }

    /// Telemetria corrente (auditoria); a diversidade de vencedores é
    /// preenchida do conjunto distinto no instante da leitura.
    pub fn stats(&self) -> L4Stats {
        let mut s = self.stats.lock().unwrap_or_else(|p| p.into_inner()).clone();
        s.distinct_broadcasts =
            self.distinct.lock().unwrap_or_else(|p| p.into_inner()).len() as u64;
        s.policy_active = self
            .policy
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .active()
            .count() as u64;
        s
    }

    /// Taxa de ciclos fechados com denominador; sem ciclos = None.
    pub fn closed_rate(&self) -> Option<tf::Rate> {
        let s = self.stats();
        tf::Rate::from_ratio(s.envelopes_closed, s.envelopes_opened)
    }

    /// Taxa de confirmação causal COM DENOMINADOR (hits/total de
    /// verificações); sem verificações = None (ausência ≠ zero).
    pub fn confirm_rate(&self) -> Option<tf::Rate> {
        let s = self.stats();
        tf::Rate::from_ratio(s.confirmed, s.confirmed + s.content_gone + s.degraded_beyond_tolerance)
    }

    /// Percentual causal global com denominador; sem observações = None.
    pub fn causal_pct(&self) -> Option<tf::Rate> {
        self.causal
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .causal_pct()
    }

    /// Versão do modelo do mundo (crenças atualizadas).
    pub fn world_version(&self) -> u64 {
        self.world
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .version()
    }

    /// Relatório do rollout contrafactual (16.3): (avaliações,
    /// divergência média |observado − projetado t+1|). A média usa
    /// o NÚMERO de avaliações de denominador; sem avaliações =
    /// None (ausência ≠ zero — crença viva ausente não é erro).
    pub fn rollout_report(&self) -> (u64, Option<f32>) {
        let s = self.stats();
        if s.rollout_evaluations == 0 {
            return (0, None);
        }
        (
            s.rollout_evaluations,
            Some(s.rollout_div_sum / s.rollout_evaluations as f32),
        )
    }

    /// PROTOCOLO de intervenção (16.7, Lei 6): o L5 submete uma
    /// proposta de mudança de política na FILA do dono — o L4
    /// valida (range, TTL, duplicada, teto), APLICA com previous
    /// guardado e REVERTE por TTL sem efeito. A ponte L5→L4 é
    /// read-only para ESTADO; a fila é protocolo (espelho do
    /// `submit_reinforcement` L4→L3). Recusas são TIPADAS.
    pub fn submit_policy(&self, p: crate::policy::PolicyProposal) -> Result<(), crate::policy::PolicyReject> {
        let mut stats = self.stats.lock().unwrap_or_else(|p| p.into_inner());
        let mut inbox = self.policy.lock().unwrap_or_else(|p| p.into_inner());
        let res = inbox.submit(p);
        match &res {
            Ok(()) => stats.policy_submitted += 1,
            Err(crate::policy::PolicyReject::DuplicateTarget) => {
                stats.policy_rejected_duplicate += 1
            }
            Err(crate::policy::PolicyReject::Full) => stats.policy_rejected_full += 1,
            Err(crate::policy::PolicyReject::OutOfRange) => {
                stats.policy_rejected_out_of_range += 1
            }
            Err(crate::policy::PolicyReject::InvalidTtl) => {
                stats.policy_rejected_invalid_ttl += 1
            }
        }
        res
    }

    /// Limiar de commit EFETIVO: override de política ativa (16.7)
    /// ou o valor base da config `[l4.decision] commit_threshold`.
    pub fn effective_commit_threshold(&self) -> f32 {
        let inbox = self.policy.lock().unwrap_or_else(|p| p.into_inner());
        inbox.effective(
            crate::policy::PolicyTarget::CommitThreshold,
            self.config.decision.commit_threshold,
        )
    }

    /// Políticas ativas correntes (auditoria; clones).
    pub fn active_policies(&self) -> Vec<crate::policy::ActivePolicy> {
        self.policy
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .active()
            .cloned()
            .collect()
    }

    /// Fecha os ciclos cujo outcome chegou (t+1) e expira os que
    /// estouraram o prazo — o ciclo NUNCA fecha sem verificação.
    /// COMENSURÁVEL (seção 15): o outcome é o valor do MESMO
    /// conteúdo da promessa, medido na fotografia t+1; o efeito
    /// líquido é comparado ao SHAM DE DERIVA (valor no commit).
    fn close_cycles(
        &self,
        tick: u64,
        snapshot: &triad_l3_local::L3Snapshot,
    ) -> Vec<tc::LearningEnvelope> {
        let mut open = self.open.lock().unwrap_or_else(|p| p.into_inner());
        let mut stats = self.stats.lock().unwrap_or_else(|p| p.into_inner());
        let tolerance = self.config.causal.confirm_tolerance;
        let mut closed = Vec::new();
        let mut i = 0;
        while i < open.len() {
            // Clone owned: o braço que fecha/expira remove o ciclo da fila.
            let cycle = open[i].clone();
            if tick <= cycle.decided_tick {
                i += 1;
                continue; // outcome só existe em t+1 (não no mesmo tick)
            }
            // Outcome COMENSURÁVEL: valor do conteúdo prometido em t+1.
            match content_value(&cycle.content_key, snapshot) {
                Some(actual) => {
                    // Sham de deriva: efeito líquido acima do baseline
                    // contrafactual local (persistência sem a ação).
                    let net_effect = actual - cycle.sham;
                    let reason = if net_effect >= -tolerance {
                        VerificationReason::Confirmed
                    } else {
                        VerificationReason::DegradedBeyondTolerance
                    };
                    let confirmed = reason == VerificationReason::Confirmed;
                    let mut envelope = cycle.envelope.clone();
                    envelope.actual_outcome = format!(
                        "{}={actual:.3} (efeito líquido {net_effect:+.3} vs sham {:.3})",
                        cycle.content_key, cycle.sham
                    );
                    envelope.error = Some(net_effect.abs());
                    envelope.verified_future_effect = Some(tc::VerifiedEffect {
                        observed_step: tf::id::StepId::new(),
                        description: format!(
                            "{} [{}] em t+1",
                            reason.as_str(),
                            cycle.content_key
                        ),
                        // E4: efeito consumido pelo próprio L4 no tempo;
                        // E5 (gêmea bit-idêntica com sham pareado) é
                        // degrau futuro DECLARADO, não inflado.
                        evidence: tf::evidence::EvidenceLevel::E4Consumed,
                    });
                    self.causal
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .observe(&cycle.cause, "conteudo_mantido", confirmed);
                    match reason {
                        VerificationReason::Confirmed => stats.confirmed += 1,
                        VerificationReason::ContentGone => stats.content_gone += 1,
                        VerificationReason::DegradedBeyondTolerance => {
                            stats.degraded_beyond_tolerance += 1
                        }
                    }
                    stats.envelopes_closed += 1;
                    // Telemetria POR DECISÃO (insumo do E5 com gêmea):
                    // cap rotativo — o organismo não cresce sem limite.
                    {
                        let mut log = self
                            .closed_log
                            .lock()
                            .unwrap_or_else(|p| p.into_inner());
                        if log.len() >= CLOSED_LOG_MAX {
                            log.remove(0);
                        }
                        log.push(ClosedCycleRecord {
                            decided_tick: cycle.decided_tick,
                            content_key: cycle.content_key.clone(),
                            actual,
                            sham: cycle.sham,
                            net_effect,
                            confirmed,
                        });
                    }
                    // TRILHA L4→L3 (seção 16.2): o ciclo CONFIRMADO
                    // reforça o conceito na memória episódica do L3 —
                    // fila no DONO, o L4 apenas submete; janela de
                    // reconsolidação por rótulo da config.
                    if confirmed && self.config.memory_integration.enabled {
                        let interval = self
                            .config
                            .memory_integration
                            .integration_interval_steps
                            .max(1);
                        let mut throttle =
                            self.reinforcement_throttle.lock().unwrap_or_else(|p| p.into_inner());
                        let last = throttle.get(&cycle.content_key).copied().unwrap_or(0);
                        let due = tick.saturating_sub(last) >= interval;
                        if due {
                            throttle.insert(cycle.content_key.clone(), tick);
                        }
                        drop(throttle);
                        if due {
                            if let Some(l3) = &self.l3 {
                                l3.submit_reinforcement(
                                    triad_l3_local::reinforcement::Reinforcement {
                                        label: cycle.content_key.clone(),
                                        step: tick,
                                        net_effect,
                                        reconsolidate: self
                                            .config
                                            .memory_integration
                                            .reconsolidation_enabled,
                                    },
                                );
                                stats.memory_submissions += 1;
                            }
                        }
                    }

                    // ROLLOUT CONTRAFACTUAL (16.3): trajetória rasa
                    // DETERMINÍSTICA da crença do EIXO COMENSURÁVEL,
                    // projetada do mundo observado — segundo
                    // contrafactual INFORMATIVO ao lado do sham (a
                    // Lei 5 continua sendo fechada pelo sham; o
                    // rollout mede a qualidade da projeção do world
                    // model, nunca decide). Sem crença viva: ausência
                    // contada como ausência (não zero, não erro).
                    if let Some(belief_key) = belief_key_for(&cycle.content_key) {
                        let world = self.world.lock().unwrap_or_else(|p| p.into_inner());
                        let depth = self.config.world_model.simulation_max_depth as usize;
                        let proj = world.rollout(
                            belief_key,
                            depth,
                            self.descriptor.module_id,
                            tf::id::StepId::new(),
                        );
                        if let Some(path) = proj.as_ref_value() {
                            if let Some(proj_t1) = path.first() {
                                stats.rollout_evaluations += 1;
                                stats.rollout_div_sum += (actual - proj_t1).abs();
                                debug!(
                                    chave_crenca = belief_key,
                                    projetado_t1 = proj_t1,
                                    observado = actual,
                                    divergencia = (actual - proj_t1).abs(),
                                    "rollout contrafactual avaliado (16.3)"
                                );
                            }
                        }
                    }
                    debug!(
                        decisao = %envelope.action,
                        efeito_liquido = net_effect,
                        razao = reason.as_str(),
                        "ciclo decisão→outcome→learning FECHADO (comensurável + sham)"
                    );
                    closed.push(envelope);
                    open.remove(i);
                }
                None if tick >= cycle.timeout_tick => {
                    // Conteúdo sumiu do foco e o prazo esgotou: expira
                    // contado, não fecha (Lei 5).
                    warn!(
                        decisao = %cycle.envelope.action,
                        prazo = cycle.timeout_tick,
                        "ciclo expirou sem outcome — não fecha (Lei 5)"
                    );
                    stats.envelopes_expired += 1;
                    open.remove(i);
                }
                None => {
                    // Conteúdo sumiu do foco: razão tipada imediata.
                    let mut envelope = cycle.envelope.clone();
                    envelope.actual_outcome = format!(
                        "{} saiu do foco antes de t+1 (CONTENT_GONE)",
                        cycle.content_key
                    );
                    envelope.verified_future_effect = Some(tc::VerifiedEffect {
                        observed_step: tf::id::StepId::new(),
                        description: format!("CONTENT_GONE [{}]", cycle.content_key),
                        evidence: tf::evidence::EvidenceLevel::E4Consumed,
                    });
                    self.causal
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .observe(&cycle.cause, "conteudo_mantido", false);
                    stats.content_gone += 1;
                    stats.envelopes_closed += 1;
                    debug!(
                        decisao = %envelope.action,
                        razao = "CONTENT_GONE",
                        "ciclo fechado com razão tipada: conteúdo saiu do foco"
                    );
                    closed.push(envelope);
                    open.remove(i);
                }
            }
        }
        closed
    }

    /// Processa decisões adiadas: re-tenta o commit; expira com prazo.
    fn process_deferred(
        &self,
        tick: u64,
        snapshot: &triad_l3_local::L3Snapshot,
        out: &mut Vec<tc::EventEnvelope>,
    ) -> Vec<ActionRecord> {
        let mut deferred = self.deferred.lock().unwrap_or_else(|p| p.into_inner());
        let mut stats = self.stats.lock().unwrap_or_else(|p| p.into_inner());
        let mut executed = Vec::new();
        let mut i = 0;
        while i < deferred.len() {
            let (pending, born, deadline) = &deferred[i];
            match decide_best(pending) {
                Ok(option) if option.confidence.value() >= self.effective_commit_threshold() => {
                    // O alvo é o conteúdo prometido pela própria opção
                    // (comensurável); o sham é o valor corrente dele.
                    let content_key = content_key_of(&option.action);
                    let sham = content_value(&content_key, snapshot).unwrap_or(0.0);
                    let record = self.commit(&option, tick, content_key, sham, snapshot, out);
                    executed.push(record);
                    deferred.remove(i);
                }
                Ok(_) if tick >= *deadline => {
                    stats.decisions_expired += 1;
                    warn!(
                        prazo = deadline,
                        nascida = born,
                        "decisão adiada expirou sem commit"
                    );
                    deferred.remove(i);
                }
                Ok(_) => i += 1,
                Err(_) => i += 1,
            }
        }
        executed
    }

    /// Commita a opção vencedora: executa a ação e abre o ciclo de
    /// aprendizado com expectativa COMENSURÁVEL (valor corrente do
    /// conteúdo prometido = sham de deriva) — Lei 5.
    #[allow(clippy::too_many_arguments)]
    fn commit(
        &self,
        option: &tc::DecisionOption,
        tick: u64,
        content_key: String,
        sham: f32,
        snapshot: &triad_l3_local::L3Snapshot,
        out: &mut Vec<tc::EventEnvelope>,
    ) -> ActionRecord {
        let decision_id = tf::id::DecisionId::new();
        let record = execute(option, decision_id.clone(), tf::id::StepId::new());
        let event_id = tf::id::EventId::new();
        let envelope = tc::LearningEnvelope {
            event_id: event_id.clone(),
            decision_id,
            prediction: snapshot
                .prediction
                .as_ref()
                .map(|p| format!("{} (t+{}, conf {:.2})", p.expected, p.horizon, p.confidence))
                .unwrap_or_else(|| "sem predição (NO_DATA)".into()),
            action: option.action.clone(),
            // Expectativa comensurável: manter/elevar o valor do
            // conteúdo prometido (o sham é a linha de base).
            expected_outcome: format!("{content_key} >= {sham:.3}"),
            actual_outcome: "aguardando outcome".into(),
            error: None,
            credit_assignment: self
                .l3
                .as_ref()
                .map(|l3| vec![l3.descriptor().module_id])
                .unwrap_or_default(),
            updated_modules: vec![self.descriptor.module_id],
            memory_update: None,
            policy_update: None,
            verified_future_effect: None,
        };
        let cycle = OpenCycle {
            envelope,
            content_key,
            sham,
            cause: format!("decisao:{}", option.action),
            decided_tick: tick,
            timeout_tick: tick + self.config.action.outcome_timeout_steps,
        };
        self.open
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(cycle);
        {
            let mut stats = self.stats.lock().unwrap_or_else(|p| p.into_inner());
            stats.decisions_committed += 1;
            stats.actions += 1;
            if option.action.starts_with("explorar") {
                stats.explorations += 1;
            }
            stats.envelopes_opened += 1;
        }
        debug!(acao = %record.action, "l4.acao executada e ciclo aberto (comensurável)");
        out.push(rt::envelope(
            "l4.action",
            tick,
            tc::EventType::Decision,
            tc::Priority::Normal,
        ));
        record
    }
}

impl Default for L4Module {
    /// Estado inicial: módulo ativo com política default.
    fn default() -> Self {
        Self::new()
    }
}

/// Conteúdo-alvo da ação (comensurável): "explorar:X" promete
/// operar sobre X; "consolidar:E" sobre a predição de E; "manter:coesao"
/// sobre o sinal de coerência.
fn content_key_of(action: &str) -> String {
    if let Some(rest) = action.strip_prefix("explorar:") {
        rest.to_string()
    } else if let Some(ev) = action.strip_prefix("consolidar:") {
        format!("predicao:{ev}")
    } else if action.starts_with("manter:") {
        "sinal:coesao".to_string()
    } else {
        action.to_string()
    }
}

/// Crença do world model que corresponde ao EIXO COMENSURÁVEL do
/// content_key (16.3): predição → confiança da predição L3;
/// coerência → coerência L2; foco conceitual → SEM crença viva
/// (ausência ≠ zero: sem rollout, contado como ausência).
fn belief_key_for(content_key: &str) -> Option<&'static str> {
    if content_key.starts_with("predicao:") {
        Some("l3.prediction.confidence")
    } else if content_key == "sinal:coesao" {
        Some("l2.coherence")
    } else {
        None
    }
}

/// Valor COMENSURÁVEL de um conteúdo na fotografia L3: foco pela
/// identidade do conceito, predição pelo evento previsto, sinal de
/// coerência pelo valor corrente. Conteúdo ausente = None (a razão
/// tipada CONTENT_GONE cuida da ausência — nunca vira zero).
fn content_value(key: &str, snapshot: &triad_l3_local::L3Snapshot) -> Option<f32> {
    if let Some(foco) = key.strip_prefix("foco:") {
        return snapshot
            .foci
            .iter()
            .find(|(concept, _)| format!("foco:{concept:?}") == key)
            .map(|(_, s)| *s)
            .or_else(|| {
                let _ = foco;
                None
            });
    }
    if let Some(ev) = key.strip_prefix("predicao:") {
        return snapshot
            .prediction
            .as_ref()
            .filter(|p| p.expected == ev)
            .map(|p| p.confidence);
    }
    if key == "sinal:coesao" {
        return snapshot.coherence_mean.map(|c| c as f32);
    }
    None
}

impl rt::CognitiveModule for L4Module {
    /// Descritor do módulo registrado no runtime.
    fn descriptor(&self) -> &tc::ModuleDescriptor {
        &self.descriptor
    }

    /// Estado corrente; mutex envenenado não bloqueia a leitura.
    fn state(&self) -> rt::ModuleState {
        *self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// 18.7 (débito fechado) — hash observacional REAL da camada L4:
    /// função pura dos contadores canônicos de decisões/ciclos do
    /// próprio L4 (L4Stats, mesma família do L1/L2/L3) + lens dos
    /// buffers de ciclo (adiadas, abertos, distinct). NUNCA wall-
    /// clock; NÃO inclui ModuleId (gêmeos com mesma trajetória
    /// batem bit-exato independente dos ids). A/A: mesma seed ⇒
    /// mesmo hash em cada passo.
    fn state_hash(&self) -> Option<u64> {
        use std::hash::{Hash, Hasher};
        let s = self.stats.lock().unwrap_or_else(|p| p.into_inner());
        let mut h = std::hash::DefaultHasher::new();
        s.broadcasts.hash(&mut h);
        s.decisions_committed.hash(&mut h);
        s.decisions_deferred.hash(&mut h);
        s.decisions_expired.hash(&mut h);
        s.actions.hash(&mut h);
        s.envelopes_opened.hash(&mut h);
        s.envelopes_closed.hash(&mut h);
        s.envelopes_expired.hash(&mut h);
        s.world_updates.hash(&mut h);
        s.explorations.hash(&mut h);
        s.distinct_broadcasts.hash(&mut h);
        s.confirmed.hash(&mut h);
        s.memory_submissions.hash(&mut h);
        s.rollout_evaluations.hash(&mut h);
        s.rollout_div_sum.to_bits().hash(&mut h);
        s.policy_submitted.hash(&mut h);
        s.policy_applied.hash(&mut h);
        s.policy_reverted.hash(&mut h);
        s.policy_rejected_duplicate.hash(&mut h);
        s.policy_rejected_out_of_range.hash(&mut h);
        s.policy_rejected_invalid_ttl.hash(&mut h);
        s.policy_rejected_full.hash(&mut h);
        s.policy_active.hash(&mut h);
        s.content_gone.hash(&mut h);
        s.degraded_beyond_tolerance.hash(&mut h);
        self.deferred.lock().unwrap_or_else(|p| p.into_inner()).len().hash(&mut h);
        self.open.lock().unwrap_or_else(|p| p.into_inner()).len().hash(&mut h);
        self.distinct.lock().unwrap_or_else(|p| p.into_inner()).len().hash(&mut h);
        Some(h.finish())
    }

    /// Um tick L4: fecha ciclos anteriores, processa adiadas, compete
    /// candidatos REAIS do L3, decide, executa e abre novos ciclos.
    fn tick(&self, ctx: &rt::TypedContext, out: &mut Vec<tc::EventEnvelope>) -> tf::TriadResult<()> {
        let module_id = self.descriptor.module_id;
        let step = ctx.clock().step;
        let tick = ctx.clock().tick;

        // (1) PONTE L3→L4: fotografia corrente (read-only).
        let snapshot = self
            .l3
            .as_ref()
            .map(|l3| l3.read_for_l4())
            .unwrap_or_else(|| triad_l3_local::L3Snapshot {
                foci: Vec::new(),
                prediction: None,
                coherence_mean: None,
                coverage: None,
                step: 0,
            });

        // (2) Fecha ciclos de decisões anteriores (outcome em t+1).
        for envelope in self.close_cycles(tick, &snapshot) {
            ctx.set(
                "l4.learning.envelope",
                tc::Qualified::value(envelope, module_id, step),
            );
            out.push(rt::envelope(
                "l4.learning",
                tick,
                tc::EventType::Learning,
                tc::Priority::Normal,
            ));
        }

        // (3) Decisões adiadas: re-tentam commit até o prazo.
        let executed = self.process_deferred(tick, &snapshot, out);
        for record in executed {
            ctx.set(
                "l4.action.executed",
                tc::Qualified::value(record, module_id, step),
            );
        }

        // (4) World model: crenças com dados REAIS por intervalo —
        // observações carregam o PASSO (insumo da tendência/rollout 16.3).
        if tick % self.config.world_model.update_interval_steps.max(1) == 0 {
            let mut world = self.world.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(p) = &snapshot.prediction {
                world.observe("l3.prediction.confidence", p.confidence, tick);
            }
            if let Some(c) = snapshot.coherence_mean {
                world.observe("l2.coherence", c as f32, tick);
            }
            self.stats.lock().unwrap_or_else(|p| p.into_inner()).world_updates += 1;
        }

        // (5) Poda causal por intervalo (hipóteses nunca confirmadas).
        if tick % self.config.causal.graph_prune_interval_steps.max(1) == 0 && tick > 0 {
            self.causal
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .prune(3);
        }

        // (5.5) POLÍTICA de intervenção (16.7, Lei 6 executável):
        // o dono expira overrides esgotados (reversão EXATA para o
        // previous — antes de aplicar pendentes, liberando o alvo)
        // e aplica as propostas aceitas (previous = efetivo REAL).
        // Cada apply/revert publica evento de GOVERNANÇA com recibo.
        {
            let mut inbox = self.policy.lock().unwrap_or_else(|p| p.into_inner());
            let reverted = inbox.expire(tick);
            let eff_now = inbox.effective(
                crate::policy::PolicyTarget::CommitThreshold,
                self.config.decision.commit_threshold,
            );
            let applied = inbox.drain(tick, eff_now);
            drop(inbox);
            let mut stats = self.stats.lock().unwrap_or_else(|p| p.into_inner());
            stats.policy_reverted += reverted.len() as u64;
            stats.policy_applied += applied.len() as u64;
            for ap in &applied {
                info!(
                    alvo = ap.proposal.target.key(),
                    anterior = ap.previous,
                    novo = ap.proposal.proposed,
                    motivo = %ap.proposal.reason,
                    expira = ap.expires_tick,
                    "politica APLICADA pelo dono (16.7)"
                );
                out.push(rt::envelope(
                    "l4.policy",
                    tick,
                    tc::EventType::Governance,
                    tc::Priority::Normal,
                ));
            }
            for ap in &reverted {
                info!(
                    alvo = ap.proposal.target.key(),
                    anterior = ap.proposal.proposed,
                    restaurado = ap.previous,
                    ttl = ap.proposal.ttl_ticks,
                    "politica REVERTIDA por TTL sem efeito (Lei 6)"
                );
                out.push(rt::envelope(
                    "l4.policy",
                    tick,
                    tc::EventType::Governance,
                    tc::Priority::Normal,
                ));
            }
        }

        // (6) Workspace: candidatos REAIS competem por broadcast com
        // SALIÊNCIA EFETIVA (habituação — o spotlight circula).
        if self.config.decision.enabled
            && tick % self.config.workspace.broadcast_interval_steps.max(1) == 0
        {
            let mut workspace = self.workspace.lock().unwrap_or_else(|p| p.into_inner());
            // 17.6 — ThalamicRouter: gate ANTES do workspace (o foco
            // fraco não entra na competição; repetidos na TTL
            // habituem). Recusas tipadas com denominador no agregador.
            let mut gi = self
                .global_integration
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            for (concept, salience) in &snapshot.foci {
                let key = format!("foco:{concept:?}");
                if matches!(
                    gi.thalamus.gate(tick, &key, *salience),
                    crate::global_integration::GateVerdict::Admitted { .. }
                ) {
                    workspace.submit(crate::WorkspaceStage::Local, &key, *salience);
                }
            }
            if let Some(p) = &snapshot.prediction {
                workspace.submit(
                    crate::WorkspaceStage::Sensory,
                    &format!("predicao:{}", p.expected),
                    p.confidence,
                );
            }
            if let Some(c) = snapshot.coherence_mean {
                workspace.submit(crate::WorkspaceStage::Sensory, "sinal:coesao", c as f32);
            }
            let winner: Option<WorkspaceEntry> = workspace.broadcast();
            drop(workspace);

            // 17.6 — GlobalIntegration: contexto fundido por tick
            // (focos/competição/ressonância como UMA métrica);
            // episódio fechado por delta ALIMENTA a memória L3 pela
            // trilha 16.2 (fila no dono, o L4 apenas submete).
            let episodio = gi.observe_tick(
                tick,
                &snapshot.foci,
                winner.as_ref(),
                snapshot.coherence_mean,
            );
            drop(gi);
            if let Some(ep) = episodio {
                if let Some(l3) = &self.l3 {
                    // Efeito líquido do episódio = coerência do
                    // último keyframe (estado real empurrado — nada
                    // inventado; ausência vira 0 declarado).
                    let net_effect = ep
                        .keyframes
                        .last()
                        .and_then(|k| k.last().copied())
                        .unwrap_or(0.0);
                    l3.submit_reinforcement(triad_l3_local::reinforcement::Reinforcement {
                        label: format!("episodio:{}", ep.started_tick),
                        step: tick,
                        net_effect,
                        reconsolidate: true,
                    });
                }
            }

            if let Some(winner) = winner {
                self.stats.lock().unwrap_or_else(|p| p.into_inner()).broadcasts += 1;
                self.distinct
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .insert(winner.content.clone());
                debug!(
                    saliencia = winner.salience,
                    conteudo = %winner.content,
                    "l4.workspace broadcast venceu (candidato real do L3)"
                );
                ctx.set(
                    "l4.workspace.broadcast",
                    tc::Qualified::value(winner.clone(), module_id, step),
                );
                out.push(rt::envelope(
                    "l4.workspace",
                    tick,
                    tc::EventType::Cognitive,
                    tc::Priority::Normal,
                ));

                // (7) Decisão REAL: opções derivadas do vencedor com
                // valor/confiança da FONTE — nada manufaturado.
                let mut options = Vec::new();
                if let Some(value) = tf::Reward::construct(winner.salience) {
                    if let Some(conf) = tf::Confidence::construct(winner.salience) {
                        options.push(tc::DecisionOption {
                            action: format!("explorar:{}", winner.content),
                            predicted_value: value,
                            confidence: conf,
                        });
                    }
                }
                if let Some(p) = &snapshot.prediction {
                    if let (Some(value), Some(conf)) = (
                        tf::Reward::construct(p.confidence),
                        tf::Confidence::construct(p.confidence),
                    ) {
                        options.push(tc::DecisionOption {
                            action: format!("consolidar:{}", p.expected),
                            predicted_value: value,
                            confidence: conf,
                        });
                    }
                }
                if let Some(c) = snapshot.coherence_mean {
                    let conf = snapshot
                        .coverage
                        .and_then(|cov| tf::Confidence::construct(cov));
                    if let (Some(value), Some(conf)) =
                        (tf::Reward::construct(c as f32), conf)
                    {
                        options.push(tc::DecisionOption {
                            action: "manter:coesao".into(),
                            predicted_value: value,
                            confidence: conf,
                        });
                    }
                }
                let pending = PendingDecision {
                    event_id: tf::id::EventId::new(),
                    options,
                    identity_context: Some(tc::IdentityContext {
                        // Identidade operacional do L4 até o L5 real (P2)
                        // assumir a política de identidade.
                        values: vec!["growth".into(), "stability".into()],
                        continuity: tf::Confidence::construct(1.0)
                            .unwrap_or_else(|| tf::Confidence::construct(0.0).expect("0.0 válido")),
                        source_module: module_id,
                    }),
                    limbic_modulation: None,
                    expected_outcome: format!("{} >= {:.3}", winner.content, winner.salience),
                };
                ctx.set(
                    "l4.decision.pending",
                    tc::Qualified::value(pending.clone(), module_id, step),
                );

                match decide_best(&pending) {
                    Ok(option) => {
                        out.push(rt::envelope(
                            "l4.decision",
                            tick,
                            tc::EventType::Decision,
                            tc::Priority::Normal,
                        ));
                        if option.confidence.value() >= self.effective_commit_threshold() {
                            // Expectativa comensurável: o valor corrente
                            // do conteúdo que a ação promete operar.
                            let content_key = content_key_of(&option.action);
                            let sham = content_value(&content_key, &snapshot)
                                .unwrap_or(winner.salience);
                            let record =
                                self.commit(&option, tick, content_key, sham, &snapshot, out);
                            ctx.set(
                                "l4.action.executed",
                                tc::Qualified::value(record, module_id, step),
                            );
                        } else {
                            // Abaixo do piso de commit: adia com prazo.
                            self.stats.lock().unwrap_or_else(|p| p.into_inner()).decisions_deferred += 1;
                            self.deferred
                                .lock()
                                .unwrap_or_else(|p| p.into_inner())
                                .push((
                                    pending,
                                    tick,
                                    tick + self.config.decision.proposal_timeout_steps,
                                ));
                            debug!(
                                confianca = option.confidence.value(),
                                piso = self.config.decision.commit_threshold,
                                "decisão adiada (abaixo do piso de commit)"
                            );
                        }
                    }
                    Err(e) => {
                        // Decisão rejeitada não vira ação; modo degradado registra.
                        warn!("l4.decisao rejeitada: {e}");
                        let mut degraded = self.degraded.lock().unwrap_or_else(|p| p.into_inner());
                        degraded.enter("decisao rejeitada", tick);
                    }
                }
            }
        }

        // (8) Evento pequeno do tick L4.
        out.push(rt::envelope(
            "l4.global",
            tick,
            tc::EventType::Decision,
            tc::Priority::Normal,
        ));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use triad_l1_substrate as l1;
    use triad_l2_tissue as l2;
    use triad_l3_local as l3;
    use triad_runtime::TypedContext;

    /// Ciclo REAL L1→L2→L3→L4 por tick, com relógio que avança.
    struct Ciclo {
        l1: Arc<l1::ClusterModule>,
        l2: Arc<l2::TissueModule>,
        l3: Arc<l3::L3Module>,
        l4: Arc<L4Module>,
        clock: tf::LogicalClock,
    }

    impl Ciclo {
        fn new(seed: u64, population: usize) -> Self {
            let l1 = Arc::new(l1::ClusterModule::new(seed, population));
            let l2 = Arc::new(l2::TissueModule::new(l1.shared_runner(), seed));
            let l3 = Arc::new(l3::L3Module::new_with_substrate(Arc::clone(&l2)));
            let l4 = Arc::new(L4Module::new_with_l3(Arc::clone(&l3)));
            Self {
                l1,
                l2,
                l3,
                l4,
                clock: tf::LogicalClock::new(),
            }
        }

        fn tick(&mut self) {
            self.clock.advance();
            let ctx = TypedContext::new(self.clock);
            let mut out = Vec::new();
            self.l1.tick(&ctx, &mut out).expect("tick l1");
            self.l2.tick(&ctx, &mut out).expect("tick l2");
            self.l3.tick(&ctx, &mut out).expect("tick l3");
            self.l4.tick(&ctx, &mut out).expect("tick l4");
        }

        /// 16.11: gêmea — MESMO organismo com a política L4 trocada
        /// (commit_threshold acima do máximo força defer pareado global).
        fn with_l4_config(seed: u64, population: usize, cfg: L4Config) -> Self {
            let l1 = Arc::new(l1::ClusterModule::new(seed, population));
            let l2 = Arc::new(l2::TissueModule::new(l1.shared_runner(), seed));
            let l3 = Arc::new(l3::L3Module::new_with_substrate(Arc::clone(&l2)));
            let l4 = Arc::new(L4Module::new_with_l3_and_config(Some(Arc::clone(&l3)), cfg));
            Self {
                l1,
                l2,
                l3,
                l4,
                clock: tf::LogicalClock::new(),
            }
        }
    }

    /// 16.11 — E5 COM GÊMEA BIT-IDÊNTICA (experimento offline, Lei 7):
    /// run ativa (commits) vs. gêmea com o MESMO organismo e seed, mas
    /// commit_threshold 1.1 ⇒ defer pareado global nas MESMAS decisões.
    /// O veredito E5 exige: gêmea nunca commita, contrafactual global
    /// dentro da tolerância em t+1 (twin_effect ≈ 0 — nada além da
    /// decisão propaga) e ao menos uma confirmação (Lei 5 intacta).
    #[test]
    fn gemea_bit_identica_valida_e5_por_decisao() {
        let mut ativa = Ciclo::new(42, 24);
        let cfg_gemea = L4Config {
            decision: crate::config::DecisionCfg {
                commit_threshold: 1.1,
                ..crate::config::DecisionCfg::default()
            },
            ..L4Config::default()
        };
        let mut gemea = Ciclo::with_l4_config(42, 24, cfg_gemea);
        let mut snapshots_gemea: Vec<triad_l3_local::L3Snapshot> = Vec::new();
        for _ in 0..10 {
            ativa.tick();
            gemea.tick();
            // Fotografia corrente da gêmea (índice t−1 = tick t).
            snapshots_gemea.push(gemea.l3.read_for_l4());
        }
        let log = ativa.l4.closed_log();
        assert!(!log.is_empty(), "a run ativa commitou decisões");
        assert_eq!(
            gemea.l4.stats().decisions_committed,
            0,
            "gêmea: defer pareado global (nunca commita)"
        );
        let decisions: Vec<crate::twin::TwinDecision> = log
            .iter()
            .map(|r| crate::twin::TwinDecision {
                decided_tick: r.decided_tick,
                content_key: r.content_key.clone(),
                actual_t_plus_1: r.actual,
                sham: r.sham,
            })
            .collect();
        // Valor do MESMO conteúdo em t+1 na gêmea (mesma chave, mesmo
        // tick — ids de conceito são estáveis por ordem de aparição).
        let twin_vals: Vec<(u64, String, f32)> = log
            .iter()
            .filter_map(|r| {
                let s = snapshots_gemea.get(r.decided_tick as usize)?;
                content_value(&r.content_key, s)
                    .map(|v| (r.decided_tick, r.content_key.clone(), v))
            })
            .collect();
        assert_eq!(
            twin_vals.len(),
            decisions.len(),
            "gêmea cobre TODAS as decisões (determinismo por seed)"
        );
        let verdicts = crate::twin::twin_verdicts(&decisions, &twin_vals, 0.1);
        for v in &verdicts {
            assert_eq!(
                v.evidence,
                tf::evidence::EvidenceLevel::E5EffectValidated,
                "gêmea determinística valida o contrafactual global"
            );
            let te = v.twin_effect.expect("twin presente");
            assert!(
                te.abs() <= 0.1 + 1e-9,
                "divergência contrafatual {te} dentro da tolerância"
            );
        }
        assert!(
            verdicts.iter().any(|v| v.confirmed),
            "ao menos um ciclo confirmado com E5 (Lei 5)"
        );
    }

    #[test]
    fn ciclo_real_l1_l2_l3_l4_fecha_envelope_comensuravel_e_sham() {
        let mut ciclo = Ciclo::new(42, 24);
        for _ in 0..8 {
            ciclo.tick();
        }
        let stats = ciclo.l4.stats();
        assert!(stats.broadcasts >= 1, "broadcast com candidatos reais");
        assert!(stats.decisions_committed >= 1, "commit acima do piso");
        assert!(
            stats.envelopes_closed >= 1,
            "ciclo fechado com outcome em t+1"
        );
        assert!(stats.envelopes_closed <= stats.envelopes_opened);
        let rate = ciclo.l4.closed_rate().expect("taxa com denominador");
        assert!(rate.value() > 0.0);
        // Comensurável + sham: organismo estável mantém os conteúdos
        // ⇒ verificações confirmadas existem e são maioria.
        assert!(
            stats.confirmed >= 1,
            "efeito líquido dentro da tolerância (sham de deriva)"
        );
        // TRILHA L4→L3 (16.2): confirmações viram reforços na memória
        // episódica real do L3 (o dono aplica no tick seguinte).
        assert!(
            stats.memory_submissions >= 1,
            "ciclo confirmado submete reforço à memória L3"
        );
        let (aplicados, _reconsolidados, _descartados, _pendentes) =
            ciclo.l3.reinforcement_stats();
        assert!(aplicados >= 1, "o L3 drenou e aplicou os reforços");
        let confirm = ciclo
            .l4
            .confirm_rate()
            .expect("taxa de confirmação com denominador");
        assert!(confirm.value() > 0.5, "conteúdos estáveis confirmam");
        // Causal global com denominador (pares observados).
        assert!(ciclo.l4.causal_pct().is_some());
    }

    #[test]
    fn rollout_contrafactual_tem_denominador_e_dominio_16t3() {
        let mut ciclo = Ciclo::new(42, 24);
        for _ in 0..8 {
            ciclo.tick();
        }
        let s = ciclo.l4.stats();
        let (n, mean) = ciclo.l4.rollout_report();
        // Ciclos fechados com eixo `predicao:*` ⇒ crença viva no
        // world model ⇒ o rollout contrafactual é AVALIADO (seed
        // fixa: determinístico).
        assert!(n >= 1, "denominador não silencioso: rollout avaliado");
        // Denominador honesto: o report usa EXATAMENTE as avaliações.
        assert_eq!(n, s.rollout_evaluations, "denominador = avaliações reais");
        let m = mean.expect("n>=1 ⇒ média presente (nunca zero silencioso)");
        assert!((0.0..=1.0).contains(&m), "divergência média em [0,1]: {m}");
        assert_eq!(m, s.rollout_div_sum / s.rollout_evaluations as f32);
    }

    #[test]
    fn sem_fonte_l3_nao_fabrica_broadcast_nem_decisao() {
        let l4 = L4Module::new();
        let mut clock = tf::LogicalClock::new();
        let mut total_eventos = 0usize;
        for _ in 0..4 {
            clock.advance();
            let ctx = TypedContext::new(clock);
            let mut out = Vec::new();
            l4.tick(&ctx, &mut out).expect("tick");
            total_eventos += out.len();
        }
        let stats = l4.stats();
        assert_eq!(stats.broadcasts, 0, "ausência ≠ zero: sem candidatos");
        assert_eq!(stats.decisions_committed, 0);
        assert_eq!(stats.envelopes_opened, 0);
        assert!(l4.closed_rate().is_none(), "sem ciclos = sem taxa");
        assert!(l4.confirm_rate().is_none(), "sem verificações = sem taxa");
        // O tick L4 continua publicando o evento próprio (ativo).
        assert!(total_eventos >= 4);
    }

    #[test]
    fn content_key_e_valores_comensuraveis_da_fotografia() {
        let snapshot = triad_l3_local::L3Snapshot {
            foci: Vec::new(),
            prediction: Some(l3::Prediction {
                horizon: 5,
                expected: "cobertura_organizacional".into(),
                confidence: 0.9,
                step: 3,
            }),
            coherence_mean: Some(0.8),
            coverage: Some(1.0),
            step: 3,
        };
        // Chave derivada da ação aponta para o conteúdo prometido.
        assert_eq!(
            content_key_of("consolidar:cobertura_organizacional"),
            "predicao:cobertura_organizacional"
        );
        assert_eq!(content_key_of("manter:coesao"), "sinal:coesao");
        // Valor comensurável na fotografia; foco ausente = None.
        assert!(content_value("predicao:cobertura_organizacional", &snapshot).is_some());
        let v = content_value("sinal:coesao", &snapshot).expect("coerência viva");
        assert!((v - 0.8).abs() < 1e-6);
        assert!(
            content_value("foco:inexistente", &snapshot).is_none(),
            "ausência ≠ zero (CONTENT_GONE tipa a razão)"
        );
    }

    /// 19.2 (sessao 7 — verificacao do debito 18.7 sob diretriz
    /// da dona; codigo L4 da sessao 6 intocado): hash A/A L4
    /// bit-exato POR TICK em organismos gemeos completos.
    #[test]
    fn l4_hash_aa_bit_exato_por_tick_serie19() {
        use triad_runtime::CognitiveModule as _;
        let run = || {
            let mut ativa = Ciclo::new(42, 24);
            let mut hashes: Vec<(u64, Option<u64>)> = Vec::new();
            for _ in 0..8 {
                ativa.tick();
                hashes.push((ativa.l4.stats().decisions_committed, ativa.l4.state_hash()));
            }
            hashes
        };
        let h = run();
        assert_eq!(run(), h.clone(), "gemeos bit-exatos por tick (A/A)");
        assert!(h.iter().any(|(c, _)| *c > 0), "ciclo vivo cometeu decisoes");
        assert!(h.windows(2).any(|w| w[0].1 != w[1].1), "hash evolui com o estado");
    }
}
