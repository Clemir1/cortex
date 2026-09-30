//! Ponte L1→runtime: o L1Runner vestido de módulo cognitivo.

use crate::cluster::LifecycleState;
use crate::runner::L1Runner;
use std::sync::{Arc, Mutex};
use tracing::debug;
use triad_contracts as tc;
use triad_foundation as tf;
use triad_runtime as rt;

/// Resumo público do substrato — evento pequeno (ADR-0004).
#[derive(Debug, Clone)]
pub struct SubstrateSummary {
    pub step: tf::StepId,
    pub population: usize,
    pub active: usize,
    pub dormant: usize,
    pub mean_energy: f64,
    pub emergency_active: bool,
}

/// Módulo L1: um tick = um passo do runner; publica o resumo no contexto.
pub struct ClusterModule {
    descriptor: tc::ModuleDescriptor,
    runner: Arc<Mutex<L1Runner>>,
    /// Sinal Chladni da harmonia (13.4) — motor próprio, observação
    /// ADITIVA: não altera o passo físico nem os eventos publicados.
    chladni: Mutex<crate::chladni_signal::ChladniSignal>,
    /// Última observação Chladni (ponte read-only p/ L3/development —
    /// ADR-0007: o scheduler cria UM CONTEXTO POR MÓDULO, então o sinal
    /// trafega por handle, padrão das pontes L1→L2→L3→L4).
    last_chladni: Mutex<triad_chladni::Observation>,
    state: Mutex<rt::ModuleState>,
}

impl ClusterModule {
    pub fn new(seed: u64, initial_population: usize) -> Self {
        Self::new_with_config(seed, initial_population, crate::config::L1Config::default())
    }

    /// Módulo com a configuração central injetada de
    /// `config/default.toml [l1.*]` (energia, homeostase, morfogênese).
    pub fn new_with_config(
        seed: u64,
        initial_population: usize,
        config: crate::config::L1Config,
    ) -> Self {
        Self {
            // Descritor canônico (CAMADA.txt, 17.1): L1 controla a
            // própria dinâmica e adapta por plasticidade (O1+O2);
            // substrato é proteção MÁXIMA em crise (Lei 4); serve o
            // sinal Chladni por leitura read-only (ADR-0007).
            descriptor: tc::ModuleDescriptor::new(
                tf::ModuleId::new(),
                "l1.substrate",
                tc::Layer::L1,
                "substrate",
            )
            .with_orders(&[
                tc::CyberneticOrder::O1Control,
                tc::CyberneticOrder::O2Adaptation,
            ])
            .with_state_owner("l1.substrate")
            .with_inputs(&["runtime.tick"])
            .with_outputs(&["l1.substrate", "l1.chladni"])
            .with_backend(tc::ExecutionBackend::CpuSeq)
            .with_criticality(tc::Criticality::Critical)
            .with_evidence(tf::evidence::EvidenceLevel::E3Productive)
            .with_recovery(tc::RecoveryPolicy::RestartModule),
            runner: Arc::new(Mutex::new(L1Runner::new_with_config(
                seed,
                initial_population,
                config,
            ))),
            // Harmonia: motor Chladni com config default; o loader central
            // injeta a seção [chladni] via with_chladni_config (builder).
            chladni: Mutex::new(crate::chladni_signal::ChladniSignal::new(
                triad_chladni::ChladniConfig::default(),
            )),
            // Antes do primeiro tick: ausência EXPLÍCITA (nunca zero).
            last_chladni: Mutex::new(triad_chladni::Observation {
                step: tf::StepId::new(),
                resonance: tf::Qualified::no_data(
                    "nenhum tick ainda",
                    tf::ModuleId::new(),
                    tf::StepId::new(),
                ),
                band: None,
                sample_size: 0,
            }),
            // Nasce ativo: Boot/Embryo não podem tickar no runtime.
            state: Mutex::new(rt::ModuleState::Active),
        }
    }

    /// Injeta a config `[chladni]` do default.toml (builder, aditivo —
    /// não muda nenhuma assinatura existente).
    pub fn with_chladni_config(mut self, chladni: triad_chladni::ChladniConfig) -> Self {
        self.chladni = Mutex::new(crate::chladni_signal::ChladniSignal::new(chladni));
        self
    }

    /// Última observação Chladni do substrato (ponte read-only —
    /// consumida pela atenção L3 e pelo development; observação
    /// nunca muta o dono).
    pub fn last_chladni(&self) -> triad_chladni::Observation {
        self.last_chladni
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    /// Acesso compartilhado ao runner do substrato — para o L2 real
    /// (formação de tecidos) ler os clusters POR REFERÊNCIA no mesmo
    /// processo, com recibo E4 no ledger (o estado fica no dono).
    pub fn shared_runner(&self) -> Arc<Mutex<L1Runner>> {
        Arc::clone(&self.runner)
    }
}

impl rt::CognitiveModule for ClusterModule {
    fn descriptor(&self) -> &tc::ModuleDescriptor {
        &self.descriptor
    }

    fn state(&self) -> rt::ModuleState {
        *self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// 18.7 — hash observacional da camada L1: combina o hash da
    /// média de estado 97D com o hash da ordem dos clusters (os dois
    /// canônicos da matriz versionada) — função pura do estado, A/A
    /// bit-exato entre runs (mesma seed ⇒ mesmo hash; o tick avança o
    /// estado ⇒ hash muda). Observação, nunca causa.
    fn state_hash(&self) -> Option<u64> {
        let runner = self.runner.lock().unwrap_or_else(|p| p.into_inner());
        let mean = runner.matrix.mean_state_hash?;
        let order = runner.matrix.cluster_order_hash?;
        let mut h = std::hash::DefaultHasher::new();
        use std::hash::{Hash, Hasher};
        mean.hash(&mut h);
        order.hash(&mut h);
        Some(h.finish())
    }

    fn tick(
        &self,
        ctx: &rt::TypedContext,
        out: &mut Vec<tc::EventEnvelope>,
    ) -> tf::TriadResult<()> {
        // Passo físico completo; um único lock cobre runner inteiro.
        let step_id = ctx.clock().step;
        let mut runner = self.runner.lock().unwrap_or_else(|p| p.into_inner());
        let report = runner.step();

        // Contagem viva (não estimada) por estado de lifecycle.
        let mut active = 0usize;
        let mut dormant = 0usize;
        for c in &runner.clusters {
            match c.lifecycle.state {
                LifecycleState::Active => active += 1,
                LifecycleState::Dormant | LifecycleState::Repairing => dormant += 1,
                _ => {}
            }
        }

        let summary = SubstrateSummary {
            step: step_id,
            population: report.population,
            active,
            dormant,
            mean_energy: report.mean_energy,
            emergency_active: report.emergency_active,
        };
        // Trilha de auditoria do tick L1: resumo antes de publicar.
        debug!(
            populacao = summary.population,
            ativos = summary.active,
            dormentes = summary.dormant,
            energia_media = summary.mean_energy,
            emergencia = summary.emergency_active,
            "l1.substrate tick publicado"
        );
        ctx.set("l1.substrate.summary", summary);

        // Harmonia (13.4): sinal Chladni ADITIVO — observa os clusters
        // vivos e publica chave NOVA no contexto. Não altera o passo
        // físico, os eventos nem os testes A/A do substrato.
        let mut chladni = self.chladni.lock().unwrap_or_else(|p| p.into_inner());
        let obs = chladni.observe(&*runner, self.descriptor.module_id, step_id);
        // Ponte read-only (ADR-0007): guarda a observação para os
        // consumidores por handle (atenção L3, development).
        *self.last_chladni.lock().unwrap_or_else(|p| p.into_inner()) = obs.clone();
        ctx.set("l1.substrate.chladni", obs);
        drop(chladni);

        // Evento pequeno: só escalares + versão do ledger (ADR-0004).
        out.push(rt::envelope(
            "l1.substrate",
            report.ledger_version,
            tc::EventType::Cognitive,
            tc::Priority::Normal,
        ));

        // Primeiro tick vivo ⇒ sai de Boot.
        let mut st = self.state.lock().unwrap_or_else(|p| p.into_inner());
        *st = rt::ModuleState::Active;
        Ok(())
    }
}
