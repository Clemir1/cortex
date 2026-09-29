//! Ponte da L4 para o runtime: workspace, decisão, ação e ciclo de
//! aprendizado integrados sobre dados REAIS do L3.
//!
//! REAL (P1 da análise 13-0): os candidatos do workspace vêm da
//! fotografia L3 (focos com saliência, predição, coerência/cobertura)
//! — zero candidatos manufaturados. Cada decisão commitida abre um
//! `LearningEnvelope`; o outcome é observado em t+1 e o ciclo só
//! fecha com `verified_future_effect` (Lei 5). Envelope sem outcome
//! até o timeout NÃO fecha (contado, nunca escondido).

use std::sync::{Arc, Mutex};
use tracing::{debug, warn};
use triad_contracts as tc;
use triad_foundation as tf;
use triad_runtime as rt;
use triad_runtime::CognitiveModule as _;

use crate::action::execute;
use crate::causal::CausalTracker;
use crate::config::L4Config;
use crate::decision::decide_best;
use crate::degraded::DegradedMode;
use crate::workspace::GlobalWorkspace;
use crate::world_model::WorldModel;
use crate::{ActionRecord, PendingDecision};

/// Ciclo de aprendizado aberto: decisão executada aguardando outcome.
#[derive(Debug, Clone)]
struct OpenCycle {
    /// Envelope com `verified_future_effect = None` até fechar.
    envelope: tc::LearningEnvelope,
    /// Valor numérico esperado (saliência do vencedor no broadcast).
    expected_value: f32,
    /// Causa tipada para o rastreador causal.
    cause: String,
    /// Tick em que a decisão foi commitida.
    decided_tick: u64,
    /// Tick limite para o outcome chegar (depois: expira contado).
    timeout_tick: u64,
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
}

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
    stats: Mutex<L4Stats>,
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
            descriptor: tc::ModuleDescriptor {
                module_id: tf::id::ModuleId::new(),
                name: "l4.global".into(),
                layer: tc::Layer::L4,
                domain: "global-cognition".into(),
            },
            workspace: Mutex::new(GlobalWorkspace::with_config(config.workspace.clone())),
            world: Mutex::new(WorldModel::new()),
            causal: Mutex::new(CausalTracker::new()),
            degraded: Mutex::new(DegradedMode::new()),
            deferred: Mutex::new(Vec::new()),
            open: Mutex::new(Vec::new()),
            config,
            l3,
            stats: Mutex::new(L4Stats::default()),
            state: Mutex::new(rt::ModuleState::Active),
        }
    }

    /// Telemetria corrente (auditoria).
    pub fn stats(&self) -> L4Stats {
        self.stats.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    /// Taxa de ciclos fechados com denominador; sem ciclos = None.
    pub fn closed_rate(&self) -> Option<tf::Rate> {
        let s = self.stats();
        tf::Rate::from_ratio(s.envelopes_closed, s.envelopes_opened)
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

    /// Fecha os ciclos cujo outcome chegou (t+1) e expira os que
    /// estouraram o prazo — o ciclo NUNCA fecha sem verificação.
    fn close_cycles(&self, tick: u64, snapshot: &triad_l3_local::L3Snapshot) -> Vec<tc::LearningEnvelope> {
        let mut open = self.open.lock().unwrap_or_else(|p| p.into_inner());
        let mut stats = self.stats.lock().unwrap_or_else(|p| p.into_inner());
        let tolerance = self.config.causal.confirm_tolerance;
        let mut closed = Vec::new();
        let mut i = 0;
        while i < open.len() {
            // Clone owned: o braço que fecha/expira remove o ciclo da fila.
            let cycle = open[i].clone();
            // Outcome: saliência média corrente dos focos do L3.
            let actual = mean_salience(snapshot);
            match actual {
                Some(a) if tick > cycle.decided_tick => {
                    let error = (cycle.expected_value - a).abs();
                    let mut envelope = cycle.envelope.clone();
                    envelope.actual_outcome = format!("saliencia_media={a:.3}");
                    envelope.error = Some(error);
                    envelope.verified_future_effect = Some(tc::VerifiedEffect {
                        observed_step: tf::id::StepId::new(),
                        description: format!(
                            "saliencia observada {a:.3} vs esperada {:.3} em t+1",
                            cycle.expected_value
                        ),
                        // E4: efeito consumido pelo próprio L4 no tempo.
                        // E5 (validação com sham pareado) é degrau futuro.
                        evidence: tf::evidence::EvidenceLevel::E4Consumed,
                    });
                    let confirmed = error <= tolerance;
                    self.causal
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .observe(&cycle.cause, "saliencia_mantida", confirmed);
                    stats.envelopes_closed += 1;
                    debug!(
                        decisao = %envelope.action,
                        erro = error,
                        confirmado = confirmed,
                        "ciclo decisão→outcome→learning FECHADO"
                    );
                    closed.push(envelope);
                    open.remove(i);
                }
                None if tick >= cycle.timeout_tick => {
                    // Sem outcome até o prazo: expira contado, não fecha.
                    warn!(
                        decisao = %cycle.envelope.action,
                        prazo = cycle.timeout_tick,
                        "ciclo expirou sem outcome — não fecha (Lei 5)"
                    );
                    stats.envelopes_expired += 1;
                    open.remove(i);
                }
                _ => i += 1,
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
                Ok(option) if option.confidence.value() >= self.config.decision.commit_threshold => {
                    let record = self.commit(&option, tick, snapshot, out);
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
    /// aprendizado (envelope sem verificação ainda — Lei 5).
    fn commit(
        &self,
        option: &tc::DecisionOption,
        tick: u64,
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
            expected_outcome: format!("saliencia {}", option.predicted_value.value()),
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
            expected_value: option.predicted_value.value(),
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
            stats.envelopes_opened += 1;
        }
        debug!(acao = %record.action, "l4.acao executada e ciclo aberto");
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

/// Saliência média dos focos; sem focos = None (ausência ≠ zero).
fn mean_salience(snapshot: &triad_l3_local::L3Snapshot) -> Option<f32> {
    if snapshot.foci.is_empty() {
        return None;
    }
    Some(snapshot.foci.iter().map(|(_, s)| *s).sum::<f32>() / snapshot.foci.len() as f32)
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

        // (4) World model: crenças com dados REAIS por intervalo.
        if tick % self.config.world_model.update_interval_steps.max(1) == 0 {
            let mut world = self.world.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(p) = &snapshot.prediction {
                world.observe("l3.prediction.confidence", p.confidence);
            }
            if let Some(c) = snapshot.coherence_mean {
                world.observe("l2.coherence", c as f32);
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

        // (6) Workspace: candidatos REAIS competem por broadcast.
        if self.config.decision.enabled
            && tick % self.config.workspace.broadcast_interval_steps.max(1) == 0
        {
            let mut workspace = self.workspace.lock().unwrap_or_else(|p| p.into_inner());
            for (concept, salience) in &snapshot.foci {
                workspace.submit(crate::WorkspaceStage::Local, &format!("foco:{concept:?}"), *salience);
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
            let winner = workspace.broadcast();
            drop(workspace);

            if let Some(winner) = winner {
                self.stats.lock().unwrap_or_else(|p| p.into_inner()).broadcasts += 1;
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
                    expected_outcome: format!("saliencia {:.3}", winner.salience),
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
                        if option.confidence.value() >= self.config.decision.commit_threshold {
                            let record = self.commit(&option, tick, &snapshot, out);
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
    use triad_runtime::{CognitiveModule as _, TypedContext};

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
    }

    #[test]
    fn ciclo_real_l1_l2_l3_l4_fecha_envelope_com_evidencia() {
        let mut ciclo = Ciclo::new(42, 24);
        for _ in 0..6 {
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
        // Causal global com denominador (algum par observado).
        assert!(ciclo.l4.causal_pct().is_some());
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
        // O tick L4 continua publicando o evento próprio (ativo).
        assert!(total_eventos >= 4);
    }
}
