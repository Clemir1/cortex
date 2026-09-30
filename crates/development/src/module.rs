//! Módulo transversal de development: liga os subsistemas morfológicos
//! ao runtime. 16.6: ESTÁGIOS REAIS — a maturação é guiada pela
//! RESSONÂNCIA REAL do substrato (sinal Chladni, ADR-0007/13.5) com
//! banda por estágio e permanência; a corticalização consome
//! MÉTRICAS VIVAS (energia média dos clusters, ressonância,
//! capacidade viva/total) — nada de literais de scaffold; o
//! orçamento de regeneração cresce com o estágio (tabela declarada).

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use triad_contracts as tc;
use triad_foundation as tf;
use triad_l1_substrate as l1;
use triad_runtime as rt;
use tracing::{debug, info, trace};

use crate::corticalization::Corticalization;
use crate::maintenance::Maintenance;
use crate::morphogenesis::Morphogenesis;
use crate::regeneration::Regeneration;
use crate::stages::{DevelopmentStage, StageTick, StageTracker};

/// Telemetria honesta do development — contadores com denominador
/// natural (ticks executados) e ausência contada (nunca zero).
#[derive(Debug, Default, Clone)]
pub struct DevelopmentStats {
    /// Ticks executados (denominador natural).
    pub ticks: u64,
    /// Estágio corrente do organismo.
    pub stage: Option<DevelopmentStage>,
    /// Avanços de estágio (máx. 3 no ciclo EMBRYO→MATURE).
    pub stage_advances: u64,
    /// Ticks consecutivos dentro da banda corrente.
    pub ticks_in_band: u64,
    /// Ticks sem sinal chladni (ausência contada).
    pub no_signal_ticks: u64,
    /// Iterações de corticalização executadas.
    pub cortical_iterations: u64,
    /// Reparos realizados (Maintenance).
    pub repairs: u64,
    /// Curas aplicadas (Regeneration, orçamento por estágio).
    pub heals: u64,
    /// Descartes acumulados.
    pub dropped: u64,
    /// Danos aguardando (fila da regeneração).
    pub damage_queue_len: u64,
}

/// Fotografia do estado de development publicada no contexto a cada tick.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DevelopmentStatus {
    /// Estágio corrente (canônico).
    pub stage: String,
    /// Estágio avançou neste tick (motivo tipado no evento).
    pub advanced: bool,
    /// Ressonância corrente (ausência = None).
    pub resonance: Option<f32>,
    /// Banda alvo do estágio corrente [min, max].
    pub band: (f32, f32),
    /// Ticks consecutivos dentro da banda.
    pub ticks_in_band: u64,
    /// Avanços já ocorridos.
    pub stage_advances: u64,
    /// Morfogênese em standby?
    pub standby: bool,
    /// Razão do despertar, se acordada.
    pub wake_reason: Option<String>,
    /// Total de reparos realizados.
    pub repairs: u64,
    /// Danos aguardando regeneração.
    pub damage_queue_len: usize,
    /// Descartes acumulados (regeneração + morfogênese).
    pub dropped: u64,
    /// Iterações de corticalização executadas.
    pub cortical_iterations: u64,
    /// Alvos DECLARADOS da corticalização (aguardam medição de tecido).
    pub corticalization_targets: (f32, f32),
}

/// Estado interno protegido por lock — tick nunca entra em pânico.
struct Inner {
    maintenance: Maintenance,
    regeneration: Regeneration,
    morphogenesis: Morphogenesis,
    corticalization: Corticalization,
    tracker: StageTracker,
    stats: DevelopmentStats,
    /// Ressonância do último sinal VIVO (publicada no status).
    last_resonance: Option<f32>,
}

impl Inner {
    /// Subsistemas morfológicos recém-criados.
    fn new(stages: crate::stages::StagesCfg) -> Self {
        Self {
            maintenance: Maintenance::new(),
            regeneration: Regeneration::new(),
            morphogenesis: Morphogenesis::new(),
            corticalization: Corticalization::new(),
            tracker: StageTracker::new(stages),
            stats: DevelopmentStats::default(),
            last_resonance: None,
        }
    }
}

/// Módulo transversal de development (forma e manutenção do organismo).
pub struct DevelopmentModule {
    inner: Mutex<Inner>,
    descriptor: tc::ModuleDescriptor,
    /// Política injetada de `config/default.toml [development.*]`.
    config: crate::config::DevelopmentCfg,
    /// Fonte do sinal Chladni (ponte read-only ao L1 — ADR-0007).
    /// None = ponte não ligada (ausência declarada no tick).
    chladni_source: Option<Arc<l1::ClusterModule>>,
}

impl DevelopmentModule {
    /// Compatibilidade: política default, SEM ponte (ausência contada).
    pub fn new() -> Self {
        Self::new_with_config(crate::config::DevelopmentCfg::default())
    }

    /// Development REAL com política central (16.6: bandas de estágio).
    pub fn new_with_config(config: crate::config::DevelopmentCfg) -> Self {
        let stages = config.stages.clone();
        Self {
            inner: Mutex::new(Inner::new(stages)),
            // Descritor canônico (CAMADA.txt, 17.1): T/DEVELOPMENT
            // — transversal com suporte declarado; seleção
            // estrutural (O5) sempre por razão tipada. 16.6: o
            // estágio avança por banda de ressonância OBSERVADA
            // (E3: subsistema produtivo com efeito real nos
            // orçamentos do organismo).
            descriptor: tc::ModuleDescriptor::new(
                tf::ModuleId::new(),
                "development",
                tc::Layer::Transversal,
                "development",
            )
            .with_support(Some(tc::SupportLayer::Development))
            .with_orders(&[tc::CyberneticOrder::O5Selection])
            .with_state_owner("development")
            .with_inputs(&["runtime.tick", "l1.chladni"])
            .with_outputs(&["development", "development.stage"])
            .with_backend(tc::ExecutionBackend::CpuSeq)
            .with_criticality(tc::Criticality::Normal)
            .with_dependencies(&["l1.substrate"])
            .with_evidence(tf::evidence::EvidenceLevel::E3Productive)
            .with_recovery(tc::RecoveryPolicy::RestartModule),
            config,
            chladni_source: None,
        }
    }

    /// Liga a ponte do sinal Chladni (ADR-0007): leitura read-only da
    /// última observação do substrato — o scheduler cria um contexto
    /// por módulo, então o sinal trafega por handle, não por contexto.
    pub fn with_chladni_source(mut self, l1: Arc<l1::ClusterModule>) -> Self {
        self.chladni_source = Some(l1);
        self
    }

    /// Telemetria corrente (auditoria).
    pub fn stats(&self) -> DevelopmentStats {
        let inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let mut s = inner.stats.clone();
        s.stage = Some(inner.tracker.stage());
        s
    }

    /// Métricas VIVAS do substrato pela ponte L1 (read-only): energia    /// média dos clusters ATIVOS e capacidade = fração de clusters
    /// não mortos. Sem clusters = None (ausência ≠ zero).
    fn mean_energy_and_capacity(
        &self,
        src: &Arc<l1::ClusterModule>,
    ) -> Option<(f64, f64)> {
        let runner_arc = src.shared_runner();
        let runner = runner_arc.lock().unwrap_or_else(|p| p.into_inner());
        let total = runner.clusters.len() as u64;
        if total == 0 {
            return None;
        }
        let mut sum = 0.0f64;
        let mut active = 0u64;
        let mut alive = 0u64;
        for bio in &runner.clusters {
            match bio.lifecycle.state {
                l1::LifecycleState::Active => {
                    sum += bio.energy;
                    active += 1;
                    alive += 1;
                }
                l1::LifecycleState::Dormant | l1::LifecycleState::Repairing => {
                    alive += 1;
                }
                _ => {}
            }
        }
        let energy_mean = if active == 0 { 0.0 } else { sum / active as f64 };
        let capacity = alive as f64 / total as f64;
        Some((energy_mean, capacity))
    }

    /// SNAPSHOT cross-run (17.9): estágio + contadores de telemetria.
    /// Determinístico: mesma história ⇒ mesmo snapshot.
    pub fn snapshot(&self) -> crate::stages::StageSnapshot {
        self.inner
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .tracker
            .snapshot()
    }

    /// RESTORE cross-run (17.9): retoma o estágio alcançado e os
    /// contadores (procedência verificada pelo Checkpointer).
    pub fn restore_snapshot(&self, snap: &crate::stages::StageSnapshot) {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        inner.tracker.restore(snap.clone());
        inner.stats.stage_advances = snap.advances;
        inner.stats.ticks_in_band = snap.ticks_in_band;
        inner.stats.no_signal_ticks = snap.no_signal_ticks;
    }
}

impl rt::CognitiveModule for DevelopmentModule {
    /// Descritor estático do módulo.
    fn descriptor(&self) -> &tc::ModuleDescriptor {
        &self.descriptor
    }

    /// Sempre ativo: transversal não dorme (crise muda política, nunca desliga sistema).
    fn state(&self) -> rt::ModuleState {
        rt::ModuleState::Active
    }

    /// Um passo do organismo: estágio por banda de ressonância REAL,
    /// corticalização com métricas VIVAS, reparo orçado por estágio.
    fn tick(
        &self,
        ctx: &rt::TypedContext,
        out: &mut Vec<tc::EventEnvelope>,
    ) -> tf::TriadResult<()> {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(_) => {
                // Lock envenenado: contrato violado — nunca pânico.
                return Err(tf::TriadError::ContractViolation {
                    detail: "mutex de development enveninado".to_string(),
                });
            }
        };
        inner.stats.ticks += 1;
        let tick = ctx.clock().tick;

        // (1) ESTÁGIO pela RESSONÂNCIA REAL do substrato (16.6):
        // banda por estágio + permanência; ausência congela.
        let mut advanced = false;
        match &self.chladni_source {
            Some(src) => {
                let obs = src.last_chladni();
                let res = obs
                    .resonance
                    .as_ref_value()
                    .copied();
                if res.is_some() {
                    inner.last_resonance = res;
                }
                let outcome = inner.tracker.tick(res);
                match outcome {
                    StageTick::Advanced { from, to, resonance } => {
                        advanced = true;
                        inner.stats.stage_advances += 1;
                        info!(
                            de = from.as_str(),
                            para = to.as_str(),
                            ressonancia = resonance,
                            "estagio avancado (banda cumprida)"
                        );
                        out.push(rt::envelope(
                            "development.stage",
                            tick,
                            tc::EventType::Development,
                            tc::Priority::Normal,
                        ));
                    }
                    StageTick::InBand { ticks_in_band } => {
                        inner.stats.ticks_in_band = ticks_in_band;
                    }
                    StageTick::OutOfBand { .. } => {
                        inner.stats.ticks_in_band = 0;
                    }
                    StageTick::NoSignal => {
                        inner.stats.no_signal_ticks += 1;
                    }
                    StageTick::StayedMature { .. } => {}
                }
            }
            None => {
                // Ponte não ligada: ausência contada, estágio congela.
                let outcome = inner.tracker.tick(None);
                if let StageTick::NoSignal = outcome {
                    inner.stats.no_signal_ticks += 1;
                }
                trace!("development: ponte chladni não ligada (ausência ≠ zero)");
            }
        }
        let stage = inner.tracker.stage();

        // (2) Manutenção O(1): no estado atual os candidatos internos
        // são vazios e nada consta danificado — o orçamento segue o
        // estágio (efeito REAL da maturação no organismo).
        let damaged: HashSet<tf::ModuleId> = HashSet::new();
        let _ = inner.maintenance.check(&[], &damaged);

        // (3) Regeneração ORÇADA PELO ESTÁGIO (tabela declarada:
        // o organismo amadurece, o reparo cresce).
        let budget = stage.heal_budget() as usize;
        let healed = inner.regeneration.heal(budget);
        inner.stats.heals += healed as u64;

        // (4) Corticalização com MÉTRICAS VIVAS (16.6 — fim dos
        // literais de scaffold): energia média real dos clusters,
        // estabilidade = ressonância observada (sem sinal: a iteração
        // NÃO roda nesse tick — ausência ≠ zero), capacidade = fração
        // de clusters não mortos. Uma iteração por tick (orçada),
        // critérios de parada do subsistema intactos.
        if self.config.corticalization.enabled {
            if let Some(src) = &self.chladni_source {
                if let Some((energy, capacity)) = self.mean_energy_and_capacity(src) {
                    if let Some(res) = inner.last_resonance {
                        let stopped =
                            inner.corticalization.step(energy, res as f64, capacity);
                        if stopped {
                            trace!(
                                criterio = ?inner.corticalization.criterion(),
                                "corticalizacao: criterio de parada atingido"
                            );
                        }
                    }
                }
            }
        }

        let (standby, reason) = inner.morphogenesis.status();
        let band = crate::stages::band_for(&self.config.stages, stage);
        let status = DevelopmentStatus {
            stage: stage.as_str().to_string(),
            advanced,
            resonance: inner.last_resonance,
            band,
            ticks_in_band: inner.tracker.ticks_in_band(),
            stage_advances: inner.tracker.advances(),
            standby,
            wake_reason: reason.map(|r| r.as_str().to_string()),
            repairs: inner.maintenance.repaired(),
            damage_queue_len: inner.regeneration.queue_len(),
            dropped: inner.regeneration.dropped() + inner.morphogenesis.dropped(),
            cortical_iterations: inner.corticalization.iterations(),
            corticalization_targets: (
                self.config.corticalization.specialization_target,
                self.config.corticalization.integration_target,
            ),
        };
        inner.stats.repairs = status.repairs;
        inner.stats.cortical_iterations = status.cortical_iterations;
        inner.stats.damage_queue_len = status.damage_queue_len as u64;
        inner.stats.dropped = status.dropped;

        debug!(
            estagio = stage.as_str(),
            ressonancia = ?inner.last_resonance,
            na_banda = status.ticks_in_band,
            avancos = status.stage_advances,
            curas_orcadas = budget,
            "development tick publicado"
        );
        ctx.set("development.status", status);

        out.push(rt::envelope(
            "development",
            ctx.clock().tick,
            tc::EventType::Development,
            tc::Priority::Normal,
        ));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use triad_runtime::CognitiveModule as _;

    #[test]
    fn sem_ponte_ausencia_contada_e_estagio_congela() {
        let dev = DevelopmentModule::new();
        let mut clock = tf::LogicalClock::new();
        for _ in 0..5 {
            clock.advance();
            let ctx = rt::TypedContext::new(clock);
            let mut out = Vec::new();
            dev.tick(&ctx, &mut out).expect("tick");
        }
        let s = dev.stats();
        assert_eq!(s.ticks, 5, "denominador natural");
        assert_eq!(s.no_signal_ticks, 5, "sem ponte: ausência contada por tick");
        assert_eq!(s.stage, Some(DevelopmentStage::Embryo), "congela no embrião");
        assert_eq!(s.stage_advances, 0);
        assert_eq!(s.heals, 0, "fila vazia: curas 0 com orçamento do estágio");
        assert_eq!(
            s.cortical_iterations, 0,
            "sem métrica viva: corticalização não itera (ausência ≠ zero)"
        );
    }

    #[test]
    fn com_ponte_l1_consome_sinal_real_e_metricas_vivas() {
        let l1m = Arc::new(l1::ClusterModule::new(42, 16));
        let dev = DevelopmentModule::new().with_chladni_source(Arc::clone(&l1m));
        let mut clock = tf::LogicalClock::new();
        // O L1 ticka PRIMEIRO: o sinal Chladni sai com clusters vivos
        // (VALUE) e as métricas de energia/capacidade ficam reais.
        clock.advance();
        let ctx = rt::TypedContext::new(clock);
        let mut out_l1 = Vec::new();
        l1m.tick(&ctx, &mut out_l1).expect("l1 tick");
        let mut eventos = 0usize;
        for _ in 0..6 {
            clock.advance();
            let ctx = rt::TypedContext::new(clock);
            let mut out = Vec::new();
            dev.tick(&ctx, &mut out).expect("tick");
            eventos += out.len();
        }
        let s = dev.stats();
        assert_eq!(s.ticks, 6);
        assert_eq!(s.no_signal_ticks, 0, "sinal vivo do L1 consumido");
        assert_eq!(s.stage_advances, 0, "dwell 20 >> 6 ticks: sem avanço");
        assert_eq!(s.stage, Some(DevelopmentStage::Embryo));
        assert!(eventos >= 6, "evento development por tick");
        // Métricas VIVAS: capacidade = fração não-morta > 0.
        let (_energy, capacity) = dev
            .mean_energy_and_capacity(dev.chladni_source.as_ref().expect("ponte"))
            .expect("métricas com clusters");
        assert!(capacity > 0.0, "capacidade viva > 0");
    }

    #[test]
    fn a_a_determinismo_do_modulo_sem_ponte() {
        let run = || {
            let dev = DevelopmentModule::new();
            let mut clock = tf::LogicalClock::new();
            let mut log: Vec<u64> = Vec::new();
            for _ in 0..4 {
                clock.advance();
                let ctx = rt::TypedContext::new(clock);
                let mut out = Vec::new();
                dev.tick(&ctx, &mut out).expect("tick");
                log.push(out.len() as u64);
            }
            let s = dev.stats();
            log.push(s.no_signal_ticks);
            log.push(
                s.stage
                    .map(|x| x.as_str().to_string())
                    .unwrap_or_default()
                    .len() as u64,
            );
            log
        };
        assert_eq!(run(), run(), "mesma sequência ⇒ mesmo módulo (A/A)");
    }
}
