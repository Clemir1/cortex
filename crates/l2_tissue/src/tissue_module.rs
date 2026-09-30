//! Módulo L2 REAL: o [`L2Runner`] vestido de módulo cognitivo.
//!
//! A cada tick, o L1 já executou o passo físico (ordem do scheduler);
//! este módulo trava o runner do substrato, roda um step L2 completo
//! (formação por afinidade → topologia com bridges → feedback →
//! publicação POST_TISSUE) e publica no contexto:
//! - `l2.tissue.snapshot` — o `TissueSnapshot` versionado (E3);
//! - `l2.tissue.views` — visões consumíveis pelo L3 (escalares + ids).
//!
//! As pontes ficam instrumentadas: a leitura L1→L2 registra recibo E4
//! no `L1Ledger` do substrato; o L3 lê o snapshot pelo `L2Ledger` com
//! recibo próprio. Ausência ≠ zero: sem fonte válida, o módulo publica
//! a ausência com a razão e NÃO fabrica tecidos.

use std::sync::{Arc, Mutex};

use triad_contracts as tc;
use triad_foundation as tf;
use triad_l1_substrate as l1;
use triad_runtime as rt;

use crate::runner::{L2Runner, L2StepReport};

/// Módulo do tecido real: ponte L1↔L3 com organização mesoscópica.
pub struct TissueModule {
    descriptor: tc::ModuleDescriptor,
    /// Runner L2 (formação, gate, ledger, métricas) — mutável dentro do
    /// lock; a trait exige `tick(&self)`.
    l2: Mutex<L2Runner>,
    /// Acesso ao runner do substrato L1 (o estado fica no dono).
    l1: Arc<Mutex<l1::L1Runner>>,
    /// Inbox de pedidos de adaptação L3→L2 (o gate decide no próximo
    /// step L2 — propor, nunca escrever).
    inbox: Mutex<Vec<tc::AdaptationRequest>>,
    /// Último report (para inspeção do orquestrador/app).
    last_report: Mutex<Option<L2StepReport>>,
    state: Mutex<rt::ModuleState>,
}

impl TissueModule {
    /// Cria o módulo do tecido com defaults documentados (consts do
    /// crate). Os ids de tecidos são derivados da seed.
    pub fn new(l1_shared: Arc<Mutex<l1::L1Runner>>, seed: u64) -> Self {
        Self::new_with_config(l1_shared, seed, crate::config::L2Config::default())
    }

    /// Cria o módulo com a configuração importada do TOML central
    /// (`config/default.toml [l2.*]`).
    pub fn new_with_config(
        l1_shared: Arc<Mutex<l1::L1Runner>>,
        seed: u64,
        config: crate::config::L2Config,
    ) -> Self {
        Self {
            // Descritor canônico (CAMADA.txt, 17.1): L2 adapta a
            // organização mesoscópica (O2) com recibos no L2Ledger
            // (E3); drena as propostas L3 pelo inbox do DONO.
            descriptor: tc::ModuleDescriptor::new(
                tf::ModuleId::new(),
                "l2.tissue",
                tc::Layer::L2,
                "tissue",
            )
            .with_orders(&[tc::CyberneticOrder::O2Adaptation])
            .with_state_owner("l2.tissue")
            .with_inputs(&["runtime.tick", "l1.substrate", "inbox.l2.adaptation"])
            .with_outputs(&["l2.tissue", "l2.tissue.change"])
            .with_backend(tc::ExecutionBackend::CpuSeq)
            .with_criticality(tc::Criticality::High)
            .with_dependencies(&["l1.substrate"])
            .with_evidence(tf::evidence::EvidenceLevel::E3Productive)
            .with_recovery(tc::RecoveryPolicy::RestartModule),
            l2: Mutex::new(L2Runner::new_with_config(seed, config)),
            l1: l1_shared,
            inbox: Mutex::new(Vec::new()),
            last_report: Mutex::new(None),
            state: Mutex::new(rt::ModuleState::Active),
        }
    }

    /// Acesso ao runner L2 (ledger, formação, gate, métricas).
    pub fn l2(&self) -> std::sync::MutexGuard<'_, L2Runner> {
        self.l2.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Acesso compartilhado ao substrato (o estado fica no dono).
    pub fn shared_l1(&self) -> Arc<Mutex<l1::L1Runner>> {
        Arc::clone(&self.l1)
    }

    /// PONTE L2→L3 (consumo REAL): lê o snapshot mais recente com recibo
    /// E4 registrado no L2Ledger. Devolve o snapshot (se houver) e o
    /// recibo — a ausência também vem com razão.
    pub fn read_for_l3(
        &self,
        key: &'static str,
    ) -> (Option<tc::l2::TissueSnapshot>, tc::l2::L2ReadReceipt) {
        let mut l2 = self.l2.lock().unwrap_or_else(|p| p.into_inner());
        let receipt = l2.ledger.read(key);
        let snapshot = l2.ledger.latest().cloned();
        (snapshot, receipt)
    }

    /// Visões teciduais para o L3 (o mesmo lock ordenado l1→l2 do tick).
    pub fn tissue_views(&self) -> Vec<tc::TissueState> {
        let l1_guard = self.l1.lock().unwrap_or_else(|p| p.into_inner());
        let l2_guard = self.l2.lock().unwrap_or_else(|p| p.into_inner());
        l2_guard.views(&l1_guard)
    }

    /// PONTE L3→L2 (proposta, nunca escrita): o L3 submete um pedido de
    /// adaptação; o gate do L2 decide no próximo step (com histerese).
    pub fn submit_adaptation(&self, req: tc::AdaptationRequest) {
        self.inbox
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(req);
    }

    /// Pedidos de adaptação pendentes (drenados pelo tick do L2).
    pub fn pending_adaptations(&self) -> usize {
        self.inbox
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .len()
    }

    /// 17.14: limiar de especialização CORRENTE do L2 (lido do
    /// estado real da formação — só muda pelo gate de adaptação).
    /// O wiring 17.4 realimenta o TopDownAdmission com este valor:
    /// o canal L3→L2 fecha de ponta a ponta.
    pub fn specialization_threshold(&self) -> f32 {
        let l2 = self.l2.lock().unwrap_or_else(|p| p.into_inner());
        l2.formation.params.specialization_threshold as f32
    }

    /// 18.5: censo ecológico dos tecidos — adapter TISSUE do
    /// `EcologyMotor` (T/governance). Lock só do L2 (não toca L1).
    pub fn ecology_species(&self) -> Vec<(String, f32, u64)> {
        let l2 = self.l2.lock().unwrap_or_else(|p| p.into_inner());
        l2.formation.ecology_species()
    }

    /// Último report do step L2 (telemetria de ponte).
    pub fn last_report(&self) -> Option<L2StepReport> {
        self.last_report
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }
}

impl rt::CognitiveModule for TissueModule {
    fn descriptor(&self) -> &tc::ModuleDescriptor {
        &self.descriptor
    }

    fn state(&self) -> rt::ModuleState {
        *self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// 18.7 — hash observacional da camada L2: demografia tecidual
    /// canônica do último report (population_total, tissue_count,
    /// assigned, unassigned, bridges, adaptations aplicadas/deferred)
    /// — INTEIROS determinísticos, função pura do estado macro.
    /// Débito registrado no checklist: hash fino por tecido/membros.
    fn state_hash(&self) -> Option<u64> {
        let report = self.last_report.lock().unwrap_or_else(|p| p.into_inner()).clone()?;
        let mut h = std::hash::DefaultHasher::new();
        use std::hash::{Hash, Hasher};
        report.population_total.hash(&mut h);
        report.tissue_count.hash(&mut h);
        report.assigned.hash(&mut h);
        report.unassigned.hash(&mut h);
        report.bridges.hash(&mut h);
        report.adaptations_applied.hash(&mut h);
        report.adaptations_deferred.hash(&mut h);
        Some(h.finish())
    }

    fn tick(
        &self,
        ctx: &rt::TypedContext,
        out: &mut Vec<tc::EventEnvelope>,
    ) -> tf::TriadResult<()> {
        // Um step L2 sobre o passo físico que o L1 já executou neste
        // tick. Ordem de locks: l1 → l2 (sempre a mesma — sem deadlock;
        // scheduler é sequencial). Pedidos L3→L2 pendentes entram pelo
        // gate (com histerese) — o L3 propõe, o L2 decide.
        let inbox: Vec<tc::AdaptationRequest> =
            std::mem::take(&mut *self.inbox.lock().unwrap_or_else(|p| p.into_inner()));
        let mut l1_guard = self
            .l1
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut l2_guard = self
            .l2
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let report = l2_guard.step(&mut l1_guard, &inbox);
        // Visões L2→L3 no MESMO lock (o estado fica no dono).
        let views = l2_guard.views(&l1_guard);
        drop(l1_guard);
        drop(l2_guard);

        // Publica snapshot + visões no contexto (chaves canônicas).
        ctx.set("l2.tissue.snapshot", report.snapshot.clone());
        ctx.set("l2.tissue.views", views);

        // Trilha de auditoria do tick L2.
        let snap = &report.snapshot;
        tracing::debug!(
            step = report.step,
            tecidos = snap.tissue_count,
            atribuidos = snap.assigned_members,
            cobertura = snap
                .assignment_coverage
                .map(|c| c.to_string())
                .unwrap_or_else(|| "n/a".into()),
            status = %snap.provider_status,
            "l2.tissue tick publicado"
        );

        // Evento pequeno: versão do ledger L2 como entity_version.
        out.push(rt::envelope(
            "l2.tissue",
            report.snapshot.state_version,
            tc::EventType::Tissue,
            tc::Priority::Normal,
        ));

        // Orientação a eventos: cada mudança estrutural do step vira um
        // envelope pequeno no barramento (quem assina recebe a
        // sinalização fina sem puxar o estado — o ledger prova o resto).
        for _change in &report.events {
            out.push(rt::envelope(
                "l2.tissue.change",
                report.snapshot.state_version,
                tc::EventType::Tissue,
                tc::Priority::Normal,
            ));
        }

        *self
            .last_report
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = Some(report);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use triad_runtime::TypedContext;
    use triad_runtime::CognitiveModule as _;

    #[test]
    fn modulo_tick_publica_snapshot_e_views_no_contexto() {
        let l1_mod = l1::ClusterModule::new(42, 24);
        let shared = l1_mod.shared_runner();
        let module = TissueModule::new(Arc::clone(&shared), 42);

        // O L1 dá o passo físico primeiro (como o scheduler ordena).
        shared
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .step();

        let ctx = TypedContext::new(tf::LogicalClock::new());
        let mut out = Vec::new();
        module
            .tick(&ctx, &mut out)
            .expect("tick do tecido nunca entra em pânico");

        let snap = ctx.get_qualified::<tc::l2::TissueSnapshot>("l2.tissue.snapshot");
        assert!(snap.is_value(), "snapshot L2 publicado no contexto");
        let views = ctx.get_qualified::<Vec<tc::TissueState>>("l2.tissue.views");
        assert!(views.is_value(), "visões L2 publicadas no contexto");

        // O report de ponte fica acessível ao orquestrador.
        let report = module.last_report().expect("report guardado");
        assert_eq!(report.step, 1);
        assert_eq!(
            module.l2().ledger.version(),
            report.snapshot.state_version
        );
        // Orientação a eventos: 1 envelope do tick + 1 por mudança
        // estrutural (sinalização fina no barramento).
        assert_eq!(
            out.len(),
            1 + report.events.len(),
            "um evento por tick + um por mudança"
        );
    }
}
