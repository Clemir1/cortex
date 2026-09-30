//! Ponte da L5 para o runtime: metacognição REAL sobre o ciclo L4.
//!
//! REAL (seção 16.1): o L5 OBSERVA o L4 (stats/taxas COM DENOMINADOR)
//! e o L1 (energia média dos clusters) por pontes read-only — observação
//! nunca muta o dono. Identidade deriva do comportamento REAL
//! (crescimento=explorar/ações, estabilidade=confirmação com sham,
//! exploração=vencedores distintos, integridade=ciclos fechados);
//! límbico com histerese e razões tipadas; metacontrolador propõe com
//! motivo, monitora a métrica-alvo e classifica Kept/Reverted por TTL
//! (Lei 6) — NUNCA aplica política diretamente. Sem fonte L4: publica
//! ausência contada, nunca fabrica. Promoção de identidade: sempre
//! MANUAL (E0–E5).

use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};
use triad_contracts as tc;
use triad_foundation as tf;
use triad_l1_substrate as l1;
use triad_l4_global as l4;
use triad_runtime as rt;

use crate::config::L5Config;
use crate::governor::{BandReason, LimbicBand, LimbicGovernor};
use crate::identity::{IdentityArc, IdentityStore, IdentityValues};
use crate::meta_controller::{InterventionOutcome, MetaController};
use crate::resource_governor::{
    CognitiveGenomeOffline, DevelopmentGovernor, HistoricalMetaLearner, ResourceGovernor,
};
use crate::wake::{should_wake, WakeReason};

/// Telemetria honesta da L5 — taxas sempre com denominador.
#[derive(Debug, Default, Clone)]
pub struct L5Stats {
    /// Refreshes do self-model com dados reais do L4.
    pub self_model_updates: u64,
    /// Updates límbicos com sinais reais (L4/L1).
    pub limbic_updates: u64,
    /// Transições de banda límbica (histerese) com razão tipada.
    pub limbic_band_changes: u64,
    /// Propostas do metacontrolador registradas (com motivo).
    pub meta_proposals: u64,
    /// Recusadas por teto de concorrência (razão tipada).
    pub meta_rejected_full: u64,
    /// Intervenções mantidas: efeito observado no TTL.
    pub meta_kept: u64,
    /// Intervenções revertidas: TTL sem efeito (Lei 6).
    pub meta_reverted: u64,
    /// Intervenções ativas correntes.
    pub meta_active: u64,
    /// Despertares por LowActivity/HighStress/DormantTooLong/Scheduled.
    pub wake_low_activity: u64,
    pub wake_high_stress: u64,
    pub wake_dormant_too_long: u64,
    pub wake_scheduled: u64,
    /// Ticks sem fonte L4 (ausência contada, nunca zero fantasma).
    pub no_source_ticks: u64,
    /// Propostas de política submetidas ao inbox do L4 (16.7).
    pub policy_submitted: u64,
    /// Recusas tipadas do inbox do L4 (16.7; a razão vai no log).
    pub policy_rejected: u64,
    /// Eventos l5.* publicados no tick (por ocorrência).
    pub events: u64,
    /// Continuidade corrente do arco de identidade (0..1).
    pub identity_continuity: f32,
    /// Valores nucleares correntes derivados do L4 (congelados sem fonte).
    pub identity_values: IdentityValues,
    /// Stress corrente (0..1).
    pub stress: f32,
    /// Reserva de energia corrente (0..1).
    pub energy_reserve: f32,
    /// Banda límbica corrente (histerese).
    pub band: Option<LimbicBand>,
    /// 17.7: auditorias do ciclo de governo autorizadas (Grant).
    pub governor_audits: u64,
    /// 17.7: auditorias PULSADAS com a razão tipada da negativa.
    pub governor_denied: u64,
    /// 17.7: trials do meta-learner (denominador de commits+rollbacks).
    pub learner_trials: u64,
    /// 17.7: trials commitados (reward > baseline).
    pub learner_commits: u64,
    /// 17.7: trials revertidos (Lei 6 aplicada ao parâmetro).
    pub learner_rollbacks: u64,
    /// 17.7: despertares de morfogênese por causa declarada.
    pub dev_wakes: u64,
    /// 17.7: standbys do desenvolvimento (causa ausente/fora).
    pub dev_standbys: u64,
}

/// Ponte da L5 para o runtime: metacognição real do organismo.
pub struct L5Module {
    /// Descritor do módulo registrado no runtime.
    descriptor: tc::ModuleDescriptor,
    /// Ponte L4→L5 read-only (stats/taxas do ciclo real).
    l4: Option<Arc<l4::L4Module>>,
    /// Ponte L1→L5 read-only (energia média dos clusters).
    l1: Option<Arc<l1::ClusterModule>>,
    /// Política injetada de `config/default.toml [l5.*]`.
    config: L5Config,
    /// Arco de identidade observado.
    identity: IdentityStore,
    /// Governador límbico com histerese (tick é &self: interior mutável).
    governor: Mutex<LimbicGovernor>,
    /// Controlador metacognitivo (propor→validar→keep/revert).
    meta: Mutex<MetaController>,
    /// 17.7: governadores completos (budgets tipados, morfogênese
    /// por causa, bandit com rollback, genome offline Lei 7).
    resource_governor: Mutex<ResourceGovernor>,
    development_governor: Mutex<DevelopmentGovernor>,
    meta_learner: Mutex<HistoricalMetaLearner>,
    genome_offline: Mutex<CognitiveGenomeOffline>,
    /// 17.7: (último tick auditado, confirmed L4 vistos lá) —
    /// janela do ciclo de governo a cada audit_interval_steps.
    audit_state: Mutex<(u64, u64)>,
    /// Passos desde o último despertar (dormência relativa).
    dormant: Mutex<u64>,
    /// Telemetria corrente.
    stats: Mutex<L5Stats>,
    /// Estado corrente do módulo no runtime.
    state: Mutex<rt::ModuleState>,
}

impl L5Module {
    /// Construtor de compatibilidade: componentes prontos, SEM fonte —
    /// publica ausência contada, nunca fabrica dados.
    pub fn new(
        descriptor: tc::ModuleDescriptor,
        identity: IdentityStore,
        governor: LimbicGovernor,
        meta: MetaController,
    ) -> Self {
        Self::new_with_sources_and_config(None, None, descriptor, identity, governor, meta, L5Config::default())
    }

    /// L5 REAL com política central e pontes read-only L4/L1.
    pub fn new_with_l4_and_config(
        l4: Option<Arc<l4::L4Module>>,
        l1: Option<Arc<l1::ClusterModule>>,
        config: L5Config,
    ) -> Self {
        let identity = IdentityStore::new(IdentityArc {
            values: IdentityValues::default(),
            continuity: 1.0,
            step: tf::StepId::new(),
        });
        let governor = LimbicGovernor::with_config(config.limbic.clone());
        let meta = MetaController::with_config(config.meta_controller.clone());
        Self::new_with_sources_and_config(
            l4,
            l1,
            // Descritor canônico (CAMADA.txt, 17.1): L5 adapta,
            // arbitra e observa (O2+O3+O4); metacontrolador exige
            // efeito OBSERVADO com baseline (E3, Lei 6).
            tc::ModuleDescriptor::new(
                tf::id::ModuleId::new(),
                "l5.meta",
                tc::Layer::L5,
                "metacognition",
            )
            .with_orders(&[
                tc::CyberneticOrder::O2Adaptation,
                tc::CyberneticOrder::O3Coordination,
                tc::CyberneticOrder::O4Observation,
            ])
            .with_state_owner("l5.meta")
            .with_inputs(&["runtime.tick", "l4.global", "l1.substrate"])
            .with_outputs(&["l5.identity", "l5.meta", "l5.intervention"])
            .with_backend(tc::ExecutionBackend::CpuSeq)
            .with_criticality(tc::Criticality::Normal)
            .with_dependencies(&["l1.substrate", "l4.global"])
            .with_evidence(tf::evidence::EvidenceLevel::E3Productive)
            .with_recovery(tc::RecoveryPolicy::RestartModule),
            identity,
            governor,
            meta,
            config,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_with_sources_and_config(
        l4: Option<Arc<l4::L4Module>>,
        l1: Option<Arc<l1::ClusterModule>>,
        descriptor: tc::ModuleDescriptor,
        identity: IdentityStore,
        governor: LimbicGovernor,
        meta: MetaController,
        config: L5Config,
    ) -> Self {
        // 17.7: governadores completos derivados da CONFIG antes do
        // move (morfogênese só por causa declarada).
        let dev_governor = DevelopmentGovernor::new(
            config.development_governor.wake_conditions.clone(),
            config.development_governor.morphogenesis_default.clone(),
        );
        Self {
            descriptor,
            l4,
            l1,
            config,
            identity,
            governor: Mutex::new(governor),
            meta: Mutex::new(meta),
            dormant: Mutex::new(0),
            stats: Mutex::new(L5Stats::default()),
            // 17.7: governadores completos (audit 17-4 §5) — budgets
            // com transições tipadas; morfogênese só por causa.
            resource_governor: Mutex::new(ResourceGovernor::new()),
            development_governor: Mutex::new(dev_governor),
            meta_learner: Mutex::new(HistoricalMetaLearner::default()),
            // Lei 7: genome SEMPRE offline — fitness coletado no
            // run; seleção/aplicação só em evaluate_offline.
            genome_offline: Mutex::new(CognitiveGenomeOffline::default()),
            audit_state: Mutex::new((0, 0)),
            // Nasce ativo: Boot/Embryo não podem tickar no runtime.
            state: Mutex::new(rt::ModuleState::Active),
        }
    }

    /// Telemetria corrente (auditoria).
    pub fn stats(&self) -> L5Stats {
        self.stats.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    /// 17.7: governadores — acesso público tipado (decisões de
    /// alocação com Lei 3; rewards por contribuição confirmada).
    pub fn resource_governor(&self) -> std::sync::MutexGuard<'_, ResourceGovernor> {
        self.resource_governor
            .lock()
            .unwrap_or_else(|p| p.into_inner())
    }

    /// 17.7: governador de desenvolvimento (morfogênese por causa).
    pub fn development_governor(
        &self,
    ) -> std::sync::MutexGuard<'_, DevelopmentGovernor> {
        self.development_governor
            .lock()
            .unwrap_or_else(|p| p.into_inner())
    }

    /// 17.7: meta-learner canônico (bandit com rollback).
    pub fn meta_learner(
        &self,
    ) -> std::sync::MutexGuard<'_, HistoricalMetaLearner> {
        self.meta_learner.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// 17.7 (Lei 7): genome offline — fitness COLETADO no tick
    /// (observação permitida); seleção nunca aqui.
    pub fn genome_offline(&self) -> std::sync::MutexGuard<'_, CognitiveGenomeOffline> {
        self.genome_offline
            .lock()
            .unwrap_or_else(|p| p.into_inner())
    }

    /// Contexto de identidade corrente (contrato: sempre presente).
    pub fn identity_context(&self) -> tc::IdentityContext {
        self.identity.identity_context(self.descriptor.module_id)
    }

    /// Energia média REAL dos clusters ativos do L1 (read-only pela
    /// ponte do runner compartilhado; sem L1 = None).
    fn mean_energy(&self) -> Option<f32> {
        let l1 = self.l1.as_ref()?;
        let runner_arc = l1.shared_runner();
        let runner = runner_arc.lock().unwrap_or_else(|p| p.into_inner());
        let mut sum = 0.0f64;
        let mut n = 0u64;
        for bio in &runner.clusters {
            if bio.lifecycle.state == l1::LifecycleState::Active {
                sum += bio.energy;
                n += 1;
            }
        }
        if n == 0 {
            return Some(1.0); // sem clusters vivos: reserva intacta (ausência ≠ zero)
        }
        let mean = sum / n as f64;
        if !(0.0..=1.0).contains(&mean) {
            warn!(
                media = mean,
                vivos = n,
                "energia media fora de 0..1 — clamp com razao registrada"
            );
        }
        Some(mean.clamp(0.0, 1.0) as f32)
    }

    /// Valores nucleares DERIVADOS das taxas reais do L4 (com
    /// denominador; sem amostras: None — ausência ≠ zero).
    fn identity_values_from_l4(&self) -> Option<IdentityValues> {
        let l4 = self.l4.as_ref()?;
        let s = l4.stats();
        if s.broadcasts == 0 || s.actions == 0 {
            return None;
        }
        let growth = tf::Rate::from_ratio(s.explorations, s.actions);
        let exploration = tf::Rate::from_ratio(s.distinct_broadcasts, s.broadcasts);
        let stability = l4.confirm_rate();
        let integrity = l4.closed_rate();
        if growth.is_none() || exploration.is_none() || stability.is_none() || integrity.is_none() {
            return None;
        }
        Some(IdentityValues {
            growth: growth.expect("checado").value(),
            stability: stability.expect("checado").value(),
            exploration: exploration.expect("checado").value(),
            integrity: integrity.expect("checado").value(),
        })
    }

    /// PROTOCOLO 16.7 (Lei 6 executável): propõe ao L4 — DONO do
    /// limiar — um ALÍVIO de política do commit_threshold (crise
    /// muda POLÍTICA, nunca suspende — Lei 4). A proposta entra
    /// na FILA do dono com TTL da config; sem efeito observado no
    /// prazo o L4 REVERTE exato (Lei 6). Passo/piso de
    /// `[l5.meta_controller]`; TTL clamp à faixa aceita pelo L4.
    pub fn propose_commit_threshold_relief(
        &self,
        intervention_id: tf::ModuleId,
        reason: &str,
        tick: u64,
    ) -> Result<(), l4::policy::PolicyReject> {
        let Some(l4src) = self.l4.as_ref() else {
            // Sem ponte: NÃO é erro de política — ausência contada
            // no caller; aqui apenas recusa sem fonte (nada fabricado).
            return Ok(());
        };
        let current = l4src.effective_commit_threshold();
        let step = self.config.meta_controller.commit_threshold_step;
        let floor = self.config.meta_controller.commit_threshold_floor;
        let proposed = (current - step).max(floor);
        let ttl = self
            .config
            .meta_controller
            .intervention_ttl_steps
            .clamp(*l4::policy::TTL_RANGE.start(), *l4::policy::TTL_RANGE.end());
        let proposal = l4::policy::PolicyProposal {
            intervention_id,
            target: l4::policy::PolicyTarget::CommitThreshold,
            current,
            proposed,
            reason: reason.to_string(),
            ttl_ticks: ttl,
            issued_tick: tick,
        };
        let res = l4src.submit_policy(proposal);
        let mut stats = self.stats.lock().unwrap_or_else(|p| p.into_inner());
        match &res {
            Ok(()) => stats.policy_submitted += 1,
            Err(r) => {
                stats.policy_rejected += 1;
                warn!(
                    razao = r.as_str(),
                    "proposta de politica recusada pelo inbox do L4 (16.7)"
                );
            }
        }
        res
    }

    /// Limbico REAL: pressão = não-confirmação; aversão = falhas
    /// normalizadas; energia = média real dos clusters (com L1).
    fn limbic_inputs(&self) -> (f32, f32, f32) {
        let (stress, aversion) = match self.l4.as_ref() {
            Some(l4) => {
                let s = l4.stats();
                let verified = s.confirmed + s.content_gone + s.degraded_beyond_tolerance;
                let confirm = l4.confirm_rate();
                let aversion =
                    tf::Rate::from_ratio(s.content_gone + s.degraded_beyond_tolerance, verified);
                (
                    confirm.map(|r| 1.0 - r.value()).unwrap_or(0.0),
                    aversion.map(|r| r.value()).unwrap_or(0.0),
                )
            }
            None => (0.0, 0.0),
        };
        let energy = self.mean_energy().unwrap_or(1.0);
        (stress, energy, aversion)
    }
}

impl rt::CognitiveModule for L5Module {
    /// Descritor do módulo registrado no runtime.
    fn descriptor(&self) -> &tc::ModuleDescriptor {
        &self.descriptor
    }

    /// Estado corrente; mutex envenenado não bloqueia a leitura.
    fn state(&self) -> rt::ModuleState {
        *self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// 18.7 (débito fechado) — hash observacional REAL da camada L5:
    /// função pura dos contadores de governo/metacognição (L5Stats),
    /// floats em bits canônicos (identidade/stress/reserva), banda
    /// límbica corrente, janela de auditoria e dormência. NUNCA
    /// wall-clock; NÃO inclui ModuleId (A/A bit-exato entre gêmeos
    /// com a mesma trajetória). Sem fontes L4/L1 cada tick conta
    /// ausência (`no_source_ticks`/`events`) ⇒ o hash RESPONDE ao
    /// estado mesmo em regime sem fonte.
    fn state_hash(&self) -> Option<u64> {
        use std::hash::{Hash, Hasher};
        let s = self.stats.lock().unwrap_or_else(|p| p.into_inner()).clone();
        let mut h = std::hash::DefaultHasher::new();
        s.self_model_updates.hash(&mut h);
        s.limbic_updates.hash(&mut h);
        s.limbic_band_changes.hash(&mut h);
        s.meta_proposals.hash(&mut h);
        s.meta_rejected_full.hash(&mut h);
        s.meta_kept.hash(&mut h);
        s.meta_reverted.hash(&mut h);
        s.meta_active.hash(&mut h);
        s.wake_low_activity.hash(&mut h);
        s.wake_high_stress.hash(&mut h);
        s.wake_dormant_too_long.hash(&mut h);
        s.wake_scheduled.hash(&mut h);
        s.no_source_ticks.hash(&mut h);
        s.policy_submitted.hash(&mut h);
        s.policy_rejected.hash(&mut h);
        s.events.hash(&mut h);
        s.identity_continuity.to_bits().hash(&mut h);
        s.stress.to_bits().hash(&mut h);
        s.energy_reserve.to_bits().hash(&mut h);
        s.band.map(|b| b as u8).hash(&mut h);
        s.governor_audits.hash(&mut h);
        s.governor_denied.hash(&mut h);
        let audit = *self.audit_state.lock().unwrap_or_else(|p| p.into_inner());
        audit.0.hash(&mut h);
        audit.1.hash(&mut h);
        (*self.dormant.lock().unwrap_or_else(|p| p.into_inner())).hash(&mut h);
        Some(h.finish())
    }

    /// Um tick L5: acorda por causa, observa L4/L1 (read-only), atualiza
    /// límbico com histerese, refresca a identidade por intervalo, propõe
    /// e valida intervenções (Lei 6) e publica o evento meta.
    fn tick(&self, ctx: &rt::TypedContext, out: &mut Vec<tc::EventEnvelope>) -> tf::TriadResult<()> {
        let module_id = self.descriptor.module_id;
        let step = ctx.clock().step;
        let tick = ctx.clock().tick;
        let mut stats = self.stats.lock().unwrap_or_else(|p| p.into_inner());
        let mut events = 0u64;

        // (0) Sem fonte L4: ausência contada — nada é fabricado.
        if self.l4.is_none() {
            stats.no_source_ticks += 1;
        }

        // (0b) 17.7 — GOVERNADORES no tick: budgets com decay/regen
        // tipados no ledger + emergência por energia baixa; fitness
        // do genome COLETADO (Lei 7: seleção só em evaluate_offline,
        // NUNCA no loop). Fitness observado = taxa de confirmação do
        // ciclo L4 (com denominador; sem ciclo: não coleta).
        {
            self.resource_governor.lock().unwrap_or_else(|p| p.into_inner()).tick(tick);
            if let Some(l4) = &self.l4 {
                if let Some(rate) = l4.confirm_rate() {
                    let f = rate.value() as f32;
                    self.genome_offline
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .collect_fitness(f);
                }
            }
        }

        // (0c) 17.7 — CICLO DE GOVERNO a cada audit_interval_steps:
        // (i) o governador AUTORIZA a introspecção (pedido de
        // attention tipado — Grant faz, Deny/Throttle pula COM
        // RAZÃO registrada: Lei 3, ausência ≠ zero);
        // (ii) contribuição REAL da janela (Δ ciclos confirmados
        // L4) vira reward de energia (audit cognitive_economy:254);
        // (iii) meta-learner: trial do gasto de introspecção com
        // reward = taxa de confirmação da janela — commit ou
        // ROLLBACK tipado (Lei 6);
        // (iv) development governor: causas OBSERVADAS (stress alto
        // = persistent_overload) decidem wake ou standby tipado.
        {
            let interval = self.config.resource_governor.audit_interval_steps.max(1);
            let mut audit = self.audit_state.lock().unwrap_or_else(|p| p.into_inner());
            if tick >= audit.0 + interval {
                audit.0 = tick;
                let mut rg = self
                    .resource_governor
                    .lock()
                    .unwrap_or_else(|p| p.into_inner());
                let (stress, _, _) = self.limbic_inputs();
                let scarcity = rg
                    .budget(crate::resource_governor::Resource::Attention)
                    .map(|b| b.scarcity())
                    .unwrap_or(0.0);
                let bucket = crate::resource_governor::HistoricalMetaLearner::bucket_key(
                    stress.clamp(0.0, 1.0),
                    scarcity,
                );
                let mut ml = self.meta_learner.lock().unwrap_or_else(|p| p.into_inner());
                let (pedido, explorou) = ml.suggest(&bucket, "introspection_attention", 0.02);
                let decisao = rg.request(
                    crate::resource_governor::Resource::Attention,
                    pedido.max(0.01),
                    tick,
                );
                let autorizado = matches!(
                    decisao,
                    crate::resource_governor::AllocationDecision::Grant { .. }
                );
                if autorizado {
                    stats.governor_audits += 1;
                } else {
                    stats.governor_denied += 1;
                }
                // Contribuição real da janela: Δ confirmed do L4.
                let confirmed_agora = self
                    .l4
                    .as_ref()
                    .map(|l4| l4.stats().confirmed)
                    .unwrap_or(0);
                let contrib = confirmed_agora.saturating_sub(audit.1) as f32;
                audit.1 = confirmed_agora;
                let _ganho = rg.reward_contribution(contrib, tick);
                // Trial: reward = taxa de confirmação da janela (com
                // denominador broadcasts; sem fonte: reward 0 →
                // rollback honesto do gasto).
                let l4s = self.l4.as_ref().map(|l4| l4.stats());
                let reward = match (&l4s, autorizado) {
                    (Some(s), true) if s.broadcasts > 0 => {
                        s.confirmed as f32 / s.broadcasts as f32
                    }
                    _ => 0.0,
                };
                let verdict = ml.trial(&bucket, "introspection_attention", 0.02, pedido, reward);
                stats.learner_trials += 1;
                match verdict {
                    crate::resource_governor::TrialVerdict::Commit { .. } => {
                        stats.learner_commits += 1;
                    }
                    crate::resource_governor::TrialVerdict::Rollback { .. } => {
                        stats.learner_rollbacks += 1;
                    }
                }
                let _ = explorou;
                drop(ml);
                drop(rg);
                // Development: causas OBSERVADAS reais.
                let mut dg = self
                    .development_governor
                    .lock()
                    .unwrap_or_else(|p| p.into_inner());
                let mut causas: Vec<String> = Vec::new();
                if stress > 0.8 {
                    causas.push("persistent_overload".to_string());
                }
                if let Some(s) = &l4s {
                    if s.broadcasts > 0 && s.actions == 0 {
                        causas.push("capacity_shortage".to_string());
                    }
                }
                match dg.should_wake(&causas) {
                    crate::resource_governor::WakeVerdict::Wake { .. } => {
                        stats.dev_wakes += 1;
                    }
                    crate::resource_governor::WakeVerdict::Standby { .. } => {
                        stats.dev_standbys += 1;
                    }
                }
            }
        }

        // (1) WAKE: causa declarada para o despertar (dormência relativa).
        {
            let mut dormant = self.dormant.lock().unwrap_or_else(|p| p.into_inner());
            let (stress, _, _) = self.limbic_inputs();
            let activity = self
                .l4
                .as_ref()
                .map(|l4| if l4.stats().broadcasts > 0 { 1.0 } else { 0.0 })
                .unwrap_or(0.0);
            if let Some(reason) = should_wake(*dormant, stress, activity) {
                match reason {
                    WakeReason::LowActivity => stats.wake_low_activity += 1,
                    WakeReason::HighStress => stats.wake_high_stress += 1,
                    WakeReason::DormantTooLong(_) => stats.wake_dormant_too_long += 1,
                    WakeReason::Scheduled => stats.wake_scheduled += 1,
                }
                *dormant = 0;
            } else {
                *dormant += 1;
            }
        }

        // (2) LIMBICO: sinais reais com HISTERESE e razão tipada.
        let (stress, energy, aversion) = self.limbic_inputs();
        let mut governor = self.governor.lock().unwrap_or_else(|p| p.into_inner());
        let reason = governor.update(stress, energy, aversion);
        stats.limbic_updates += 1;
        stats.stress = stress;
        stats.energy_reserve = energy;
        stats.band = Some(governor.band());
        if reason != BandReason::Unchanged {
            stats.limbic_band_changes += 1;
            info!(
                banda = ?governor.band(),
                razao = reason.as_str(),
                stress,
                "transicao limbica (histerese da config [l5.limbic])"
            );
        }
        drop(governor);

        // (3) SELF-MODEL por intervalo: identidade DERIVADA do L4 real.
        if self.config.self_model.enabled && tick % self.config.self_model.update_interval_steps.max(1) == 0
        {
            match self.identity_values_from_l4() {
                Some(values) => {
                    let arc = self.identity.observe(values.clone(), tf::StepId::new());
                    stats.identity_continuity = arc.continuity;
                    stats.identity_values = values;
                    stats.self_model_updates += 1;
                    // Trabalho meta real: a dormência reinicia.
                    *self.dormant.lock().unwrap_or_else(|p| p.into_inner()) = 0;
                    let ctx_identity = self.identity.identity_context(module_id);
                    ctx.set(
                        "l5.identity.context",
                        tc::Qualified::value(ctx_identity, module_id, step),
                    );
                    out.push(rt::envelope(
                        "l5.identity",
                        tick,
                        tc::EventType::Cognitive,
                        tc::Priority::Normal,
                    ));
                    events += 1;
                    debug!(
                        growth = stats.identity_values.growth,
                        stability = stats.identity_values.stability,
                        exploration = stats.identity_values.exploration,
                        integrity = stats.identity_values.integrity,
                        continuidade = arc.continuity,
                        "self-model atualizado com taxas reais do L4"
                    );
                }
                None => {
                    // Ausência de amostras no L4 (sem broadcasts/ações):
                    // identidade congelada, ausência contada.
                    debug!("self-model sem amostras suficientes no L4 (ausência ≠ zero)");
                }
            }
        }

        // (4) META por intervalo: observe→propose→validate→keep/revert.
        if self.config.meta_controller.enabled
            && tick % self.config.meta_controller.proposal_interval_steps.max(1) == 0
        {
            let mut meta = self.meta.lock().unwrap_or_else(|p| p.into_inner());
            // PROPOSE: condições sobre métricas reais, com motivo exigido.
            if let Some(l4) = &self.l4 {
                let s = l4.stats();
                let active_targets: Vec<String> =
                    meta.active_reports().iter().map(|r| r.target.clone()).collect();
                if let Some(confirm) = l4.confirm_rate() {
                    if confirm.value() < 0.5
                        && !active_targets.iter().any(|t| t == "l4.verificacao")
                    {
                        let motivo =
                            format!("confirmacao {:.2} abaixo de 0.5", confirm.value());
                        match meta.intervene("l4.verificacao", &motivo, confirm.value()) {
                            Ok(report) => {
                                stats.meta_proposals += 1;
                                // 16.7: a intervenção vira PROPOSTA REAL de
                                // política no inbox do dono (Lei 6 executável;
                                // recusas tipadas contadas dentro do método).
                                let _ = self.propose_commit_threshold_relief(
                                    report.intervention_id,
                                    &motivo,
                                    tick,
                                );
                            }
                            Err(InterventionOutcome::RejectedFull) => {
                                stats.meta_rejected_full += 1
                            }
                            Err(_) => {}
                        }
                    }
                }
                if s.envelopes_expired > 0
                    && !active_targets.iter().any(|t| t == "l4.action.timeout")
                {
                    match meta.intervene(
                        "l4.action.timeout",
                        &format!("{} ciclos expiraram sem outcome", s.envelopes_expired),
                        s.envelopes_expired as f32,
                    ) {
                        Ok(_) => stats.meta_proposals += 1,
                        Err(InterventionOutcome::RejectedFull) => stats.meta_rejected_full += 1,
                        Err(_) => {}
                    }
                }
                // VALIDATE: a métrica melhorou desde o baseline?
                for report in meta.active_reports() {
                    match report.target.as_str() {
                        "l4.verificacao" => {
                            if let Some(confirm) = l4.confirm_rate() {
                                if confirm.value() > report.baseline {
                                    meta.observe_effect(report.intervention_id, confirm.value());
                                }
                            }
                        }
                        "l4.action.timeout" => {
                            let now_expired = l4.stats().envelopes_expired;
                            if (now_expired as f32) < report.baseline {
                                meta.observe_effect(report.intervention_id, now_expired as f32);
                            }
                        }
                        _ => {}
                    }
                }
            }
            // KEEP/REVERT por TTL individual (Lei 6: reversão obrigatória).
            for (report, outcome) in meta.tick_validate() {
                match outcome {
                    InterventionOutcome::Kept => {
                        stats.meta_kept += 1;
                        info!(alvo = %report.target, "intervencao mantida: efeito observado no TTL");
                    }
                    InterventionOutcome::Reverted => {
                        stats.meta_reverted += 1;
                        info!(alvo = %report.target, "intervencao revertida por TTL (Lei 6)");
                    }
                    _ => {}
                }
                out.push(rt::envelope(
                    "l5.intervention",
                    tick,
                    tc::EventType::Decision,
                    tc::Priority::Normal,
                ));
                events += 1;
            }
            stats.meta_active = meta.active_reports().len() as u64;
        }

        // (5) Evento meta por ocorrência, com tick REAL (nunca 1 fixo).
        out.push(rt::envelope(
            "l5.meta",
            tick,
            tc::EventType::Cognitive,
            tc::Priority::Normal,
        ));
        events += 1;
        stats.events += events;
        drop(stats);
        *self.state.lock().unwrap_or_else(|p| p.into_inner()) = rt::ModuleState::Active;
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

    /// Ciclo completo com TODAS as camadas (usa o construtor real).
    fn organismo_full(seed: u64) -> (
        Arc<l1::ClusterModule>,
        Arc<l2::TissueModule>,
        Arc<l3::L3Module>,
        Arc<l4::L4Module>,
        Arc<L5Module>,
    ) {
        let l1 = Arc::new(l1::ClusterModule::new(seed, 24));
        let l2 = Arc::new(l2::TissueModule::new(l1.shared_runner(), seed));
        let l3 = Arc::new(l3::L3Module::new_with_substrate(Arc::clone(&l2)));
        let l4 = Arc::new(l4::L4Module::new_with_l3(Arc::clone(&l3)));
        let l5 = Arc::new(L5Module::new_with_l4_and_config(
            Some(Arc::clone(&l4)),
            Some(Arc::clone(&l1)),
            L5Config::default(),
        ));
        (l1, l2, l3, l4, l5)
    }

    fn tick_all(
        clock: &mut tf::LogicalClock,
        l1: &Arc<l1::ClusterModule>,
        l2: &Arc<l2::TissueModule>,
        l3: &Arc<l3::L3Module>,
        l4: &Arc<l4::L4Module>,
        l5: &Arc<L5Module>,
    ) -> usize {
        clock.advance();
        let ctx = TypedContext::new(*clock);
        let mut out = Vec::new();
        l1.tick(&ctx, &mut out).expect("l1");
        l2.tick(&ctx, &mut out).expect("l2");
        l3.tick(&ctx, &mut out).expect("l3");
        l4.tick(&ctx, &mut out).expect("l4");
        l5.tick(&ctx, &mut out).expect("l5");
        out.len()
    }

    #[test]
    fn ciclo_real_l1_a_l5_identidade_do_comportamento() {
        let (l1, l2, l3, l4, l5) = organismo_full(42);
        let mut clock = tf::LogicalClock::new();
        let mut total_eventos = 0usize;
        for _ in 0..101 {
            total_eventos += tick_all(&mut clock, &l1, &l2, &l3, &l4, &l5);
        }
        let stats = l5.stats();
        // Self-model atualizou com dados REAIS (intervalo 100 ⇒ 1 refresh).
        assert!(stats.self_model_updates >= 1, "identidade derivada do L4");
        assert!(stats.identity_continuity > 0.0, "continuidade observada");
        // Valores derivados: estabilidade = confirmação (1.0 no app).
        assert!(
            stats.identity_values.stability > 0.5,
            "estabilidade segue a confirmacao real do L4"
        );
        // Limbico com sinais reais todo tick; banda estável no repouso.
        assert!(stats.limbic_updates >= 100);
        assert_eq!(stats.band, Some(LimbicBand::Stable));
        assert!(stats.energy_reserve > 0.0, "energia real dos clusters");
        // Eventos por ocorrência: >= 1 l5.meta por tick.
        assert!(stats.events >= 101);
        assert!(total_eventos >= 101);
    }

    #[test]
    fn a_a_determinismo_l5() {
        let (l1a, l2a, l3a, l4a, l5a) = organismo_full(7);
        let (l1b, l2b, l3b, l4b, l5b) = organismo_full(7);
        let mut clock = tf::LogicalClock::new();
        for _ in 0..12 {
            tick_all(&mut clock, &l1a, &l2a, &l3a, &l4a, &l5a);
            tick_all(&mut clock, &l1b, &l2b, &l3b, &l4b, &l5b);
        }
        let sa = l5a.stats();
        let sb = l5b.stats();
        assert_eq!(sa.events, sb.events);
        assert!((sa.identity_continuity - sb.identity_continuity).abs() < 1e-9);
        assert!((sa.stress - sb.stress).abs() < 1e-9);
        assert!((sa.energy_reserve - sb.energy_reserve).abs() < 1e-9);
    }

    #[test]
    fn sem_fonte_l4_ausencia_contada_nunca_fabricada() {
        let l5 = L5Module::new_with_l4_and_config(None, None, L5Config::default());
        let mut clock = tf::LogicalClock::new();
        let mut eventos = 0usize;
        for _ in 0..5 {
            clock.advance();
            let ctx = TypedContext::new(clock);
            let mut out = Vec::new();
            l5.tick(&ctx, &mut out).expect("tick");
            eventos += out.len();
        }
        let stats = l5.stats();
        assert_eq!(stats.no_source_ticks, 5, "ausência contada por tick");
        assert_eq!(stats.self_model_updates, 0, "sem fonte não deriva identidade");
        assert_eq!(stats.identity_values, IdentityValues::default(), "valores congelados");
        assert!(eventos >= 5, "o módulo segue ativo e publicando");
        // Contexto de identidade SEMPRE presente (contrato).
        let _ = l5.identity_context();
    }

    #[test]
    fn lei_6_executavel_proposta_real_aplica_e_reverte_exato() {
        // 16.7 INTEGRADA: L5 propõe alívio de política no inbox do
        // L4; o DONO aplica (previous guardado) e REVERTE exato
        // quando o TTL esgota (intervenção sem observed_effect).
        let l1 = Arc::new(l1::ClusterModule::new(42, 24));
        let l2 = Arc::new(l2::TissueModule::new(l1.shared_runner(), 42));
        let l3 = Arc::new(l3::L3Module::new_with_substrate(Arc::clone(&l2)));
        let l4 = Arc::new(l4::L4Module::new_with_l3(Arc::clone(&l3)));
        // TTL CURTO injetado: a reversão chega em poucos ticks.
        let mut cfg = L5Config::default();
        cfg.meta_controller.intervention_ttl_steps = 3;
        let l5 = Arc::new(L5Module::new_with_l4_and_config(
            Some(Arc::clone(&l4)),
            Some(Arc::clone(&l1)),
            cfg,
        ));
        let base = l4.effective_commit_threshold();
        let id = tf::ModuleId::new();
        // (a) proposta aceita no inbox do dono.
        l5.propose_commit_threshold_relief(id, "confirmacao 0.30 abaixo de 0.5", 1)
            .expect("primeira proposta aceita");
        assert_eq!(l5.stats().policy_submitted, 1);
        // (b) duplicada no mesmo alvo: recusa TIPADA.
        assert!(l5
            .propose_commit_threshold_relief(id, "repetida", 1)
            .is_err());
        assert_eq!(l5.stats().policy_rejected, 1);
        // (c) o dono APLICA no próximo tick: limiar efetivo baixa
        // (passo 0.10 com piso 0.25), previous = base guardado.
        let mut clock = tf::LogicalClock::new();
        let mut out = Vec::new();
        clock.advance();
        let ctx = TypedContext::new(clock);
        l1.tick(&ctx, &mut out).expect("l1");
        l4.tick(&ctx, &mut out).expect("l4");
        let efetivo = l4.effective_commit_threshold();
        assert!(
            efetivo < base,
            "override ativo baixa o limiar: {efetivo} < {base}"
        );
        assert_eq!(l4.stats().policy_applied, 1);
        assert_eq!(l4.stats().policy_active, 1);
        // (d) Lei 6: TTL esgota sem efeito ⇒ REVERSÃO EXATA.
        for _ in 0..3 {
            clock.advance();
            let ctx = TypedContext::new(clock);
            let mut out2 = Vec::new();
            l4.tick(&ctx, &mut out2).expect("l4");
        }
        assert_eq!(l4.stats().policy_reverted, 1, "revertida no TTL (Lei 6)");
        assert_eq!(l4.effective_commit_threshold(), base, "restaura o previous EXATO");
        assert_eq!(l4.stats().policy_active, 0);
        assert_eq!(l5.stats().policy_submitted, 1);
    }

    #[test]
    fn a_a_determinismo_do_protocolo_de_politica() {
        // Mesma sequência de propostas ⇒ mesmo estado do inbox e do
        // limiar em CADA tick (A/A bit-idêntico do protocolo 16.7).
        let run = || {
            let (_l1, _l2, _l3, l4, l5) = organismo_full(42);
            let mut hist: Vec<f32> = Vec::new();
            let mut clock = tf::LogicalClock::new();
            for t in 0..6 {
                if t == 1 {
                    let _ = l5.propose_commit_threshold_relief(
                        tf::ModuleId::new(),
                        "confirmacao baixa",
                        t as u64,
                    );
                }
                clock.advance();
                let ctx = TypedContext::new(clock);
                let mut out = Vec::new();
                l4.tick(&ctx, &mut out).expect("l4");
                hist.push(l4.effective_commit_threshold());
            }
            hist
        };
        assert_eq!(run(), run(), "mesma sequência ⇒ mesmo limiar por tick");
    }
}
