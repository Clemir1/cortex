//! Módulo transversal de cybernetics (16.8): ordens O1–O4 REAIS
//! sobre MÉTRICAS VIVAS do organismo (ressonância/energia/capacidade
//! do L1 pela ponte read-only — protocolo SOBRE componentes, sem
//! cópias), cada uma na cadência declarada em `[cybernetics.*]`.
//! O5 coleta AMOSTRAS de horizonte mas o genome fica FORA do loop
//! (Lei 7: `genome_in_main_loop=false` lido e respeitado).
//!
//! Efeitos PUBLICADOS (o cybernetics não muta estado de outras
//! camadas): status em `cybernetics.status` + evento
//! `cybernetics.metrics` quando alguma ordem executa; a ação do O3
//! (`GoverningAction` tipada: Normal/Throttle{factor}/Shed{reason})
//! é RECOMENDAÇÃO publicada — os donos consomem.

use std::sync::{Arc, Mutex};

use triad_contracts as tc;
use triad_foundation as tf;
use triad_l1_substrate as l1;
use triad_runtime as rt;
use tracing::{debug, info, warn};

use crate::config::CyberneticsCfg;
use crate::homeostat::Homeostat;
use crate::o3_governor::{GoverningAction, O3Governor};
use crate::o4_metrics::audit_rate;
use crate::o5_horizon::O5Horizon;
use crate::stability::OscillationMonitor;

/// Alvo/banda do homeostato O1 sobre a ENERGIA MÉDIA real dos
/// clusters (literais DECLARADOS: banda de operação da casa —
/// calibráveis quando houver telemetria de limite físico).
const O1_TARGET: f32 = 0.5;
const O1_TOLERANCE: f32 = 0.3;
/// Janela de oscilação do O2 (amostras de ressonância).
const O2_WINDOW_HINT: usize = 32;
/// Limiar/piso do governador O3 (estresse e reserva de energia).
const O3_STRESS_THRESHOLD: f32 = 0.8;
const O3_ENERGY_FLOOR: f32 = 0.2;
/// Janela do horizonte O5 (amostras retidas para tendência offline).
const O5_WINDOW: usize = 128;
/// 19.8-d — piso de eventos/tick do audit O4: abaixo disso o
/// organismo está operando abaixo do esperado e O2/O3 recebem o
/// alarme como sinal (observa, nunca decide).
const O4_EVENTS_PER_TICK_FLOOR: f32 = 0.5;

/// Telemetria honesta — denominadores sempre visíveis.
#[derive(Debug, Default, Clone)]
pub struct CyberneticsStats {
    /// Ticks executados (denominador natural de tudo).
    pub ticks: u64,
    /// Execuções por ordem (denominador da cadência declarada).
    pub o1_runs: u64,
    pub o2_runs: u64,
    pub o3_runs: u64,
    pub o4_runs: u64,
    /// Leituras do O1 por desfecho (denominador o1_runs).
    pub o1_satisfied: u64,
    /// Recomendações Throttle/Shed emitidas pelo O3 (denom o3_runs).
    pub o3_throttles: u64,
    pub o3_sheds: u64,
    /// Eventos publicados (denominador do audit O4).
    pub events_published: u64,
    /// Ticks sem ponte L1 (ausência contada — nunca zero).
    pub no_source_ticks: u64,
    /// Última ação recomendada pelo O3 (legível).
    pub last_action: Option<GoverningAction>,
    /// 19.8-a — última correção O1 calculada (urgência contínua
    /// entregue ao O3; None = O1 ainda não executou, ausência ≠ 0).
    pub o1_last_corrective: Option<f32>,
    /// 19.8-b — retunings do ganho do O1 por desfecho (denom o2_runs).
    pub o2_retunes_down: u64,
    pub o2_retunes_up: u64,
    /// Janelas de cadência O2 com oscilação DETECTADA (denom o2_runs).
    pub o2_oscillating_windows: u64,
    /// 19.8-d — alarmes tipados emitidos pelo O4 (denom o4_runs).
    pub o4_alarms: u64,
    /// Alarmes VIVOS correntes (reavaliados a cada auditoria O4;
    /// consumidos como SINAL pelo O2/O3 — observa, nunca decide).
    pub alarms: Vec<MetricAlarm>,
    /// 19.8-e — seleção offline NO BOUNDARY (fora do loop, Lei 7):
    /// corridas do boundary (denominador), promoções e rejeições.
    pub o5_boundary_runs: u64,
    pub o5_promotions: u64,
    pub o5_rejections: u64,
    /// Último veredito O5 (trilha com razão).
    pub last_o5_verdict: Option<String>,
}

/// 19.8-d — alarme tipado da auto-observação O4: métrica, valor
/// medido COM denominador e razão canônica (herança
/// RuntimeObservability: estado+motivo como função pura dos
/// contadores; o alarme ALIMENTA decisões, não as toma).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricAlarm {
    /// Métrica auditada (nome canônico).
    pub metric: &'static str,
    /// Valor medido (com denominador no `reason`).
    pub value: f32,
    /// Razão legível (valor/denominador + piso violado).
    pub reason: String,
}

/// Estado interno protegido por lock.
struct Inner {
    /// O1: homeostato sobre a energia média REAL.
    energy: Homeostat,
    /// O2: oscilador sobre a ressonância REAL.
    osc: OscillationMonitor,
    /// O3: governador de carga.
    gov: O3Governor,
    /// O5: horizonte de amostras (genome OFF — Lei 7).
    horizon: O5Horizon,
    /// 19.8-b — histerese do retuning: janelas O2 ESTÁVEIS
    /// consecutivas (sobe o ganho só após 2 — herança
    /// edge_of_chaos.py:116-127, anti bang-bang).
    o2_stable_streak: u32,
    stats: CyberneticsStats,
}

impl Inner {
    fn new() -> Self {
        Self {
            energy: Homeostat::new(O1_TARGET, O1_TOLERANCE),
            osc: OscillationMonitor::with_window(O2_WINDOW_HINT),
            gov: O3Governor::new(O3_STRESS_THRESHOLD, O3_ENERGY_FLOOR)
                .expect("literais da casa em 0..=1"),
            horizon: O5Horizon::new(O5_WINDOW),
            o2_stable_streak: 0,
            stats: CyberneticsStats::default(),
        }
    }
}

/// Módulo transversal de cybernetics (ordens O1–O5 como protocolos).
pub struct CyberneticsModule {
    descriptor: tc::ModuleDescriptor,
    config: CyberneticsCfg,
    /// Ponte read-only ao L1 (métricas vivas). None = ausência contada.
    l1: Option<Arc<l1::ClusterModule>>,
    inner: Mutex<Inner>,
}

impl CyberneticsModule {
    /// Compatibilidade: política default, SEM ponte (ausência contada).
    pub fn new() -> Self {
        Self::new_with_config(CyberneticsCfg::default())
    }

    /// Cybernetics REAL com política central `[cybernetics.*]`.
    pub fn new_with_config(config: CyberneticsCfg) -> Self {
        Self {
            inner: Mutex::new(Inner::new()),
            // Descritor canônico (CAMADA.txt, 17.1): T/GOVERNANCE —
            // ordens O1–O4 executando protocolos SOBRE métricas
            // vivas; O5 declarado (genoma offline, Lei 7) ⇒ E3
            // produtivo (consome métricas reais e publica decisões
            // tipadas com efeito nos consumidores).
            descriptor: tc::ModuleDescriptor::new(
                tf::ModuleId::new(),
                "cybernetics",
                tc::Layer::Transversal,
                "cybernetics",
            )
            .with_support(Some(tc::SupportLayer::Governance))
            .with_orders(&[
                tc::CyberneticOrder::O1Control,
                tc::CyberneticOrder::O2Adaptation,
                tc::CyberneticOrder::O3Coordination,
                tc::CyberneticOrder::O4Observation,
                tc::CyberneticOrder::O5Selection,
            ])
            .with_state_owner("cybernetics")
            .with_inputs(&["runtime.tick", "l1.substrate"])
            .with_outputs(&["cybernetics.metrics", "cybernetics.status"])
            .with_backend(tc::ExecutionBackend::CpuSeq)
            .with_criticality(tc::Criticality::Normal)
            .with_dependencies(&["l1.substrate"])
            .with_evidence(tf::evidence::EvidenceLevel::E3Productive)
            .with_recovery(tc::RecoveryPolicy::RestartModule),
            config,
            l1: None,
        }
    }

    /// Liga a ponte read-only ao L1 (métricas vivas do substrato).
    pub fn with_l1_source(mut self, l1: Arc<l1::ClusterModule>) -> Self {
        self.l1 = Some(l1);
        self
    }

    /// Telemetria corrente (auditoria).
    pub fn stats(&self) -> CyberneticsStats {
        self.inner
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .stats
            .clone()
    }

    /// 19.8-c — a ação de governança CORRENTE do O3 para o runtime
    /// APLICAR de fato no escalonador (consumidor vivo; antes a ação
    /// era só recomendação sem consumidor — auditoria 19-0).
    pub fn load_directive(&self) -> Option<GoverningAction> {
        self.stats().last_action
    }

    /// 19.8-a — última correção O1 (urgência contínua legível).
    pub fn o1_last_corrective(&self) -> Option<f32> {
        self.stats().o1_last_corrective
    }

    /// 19.8-b/19.8-e — ganho corrente do controlador O1
    /// (observação A/A do retuning aplicado).
    pub fn o1_gain(&self) -> f32 {
        self.inner
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .energy
            .gain()
    }

    /// 19.8-e (O5 boundary) — promove UMA decisão de seleção offline
    /// sobre o controlador: retune com recibo (razão) e REVERSÃO
    /// futura pelo retune oposto (Lei 6: intervenção reverte sem
    /// efeito observado; o fitness do PRÓXIMO episódio é o
    /// observed_effect). Clamp [0.25, 1.0] idêntico ao O2 — a
    /// seleção NUNCA desliga o controle. Devolve o ganho aplicado.
    pub fn o5_promote_retune(&self, delta: f32, reason: &str) -> f32 {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        inner.stats.o5_promotions += 1;
        inner.energy.retune_gain(delta, reason)
    }

    /// 19.8-e (O5 boundary) — rejeita a seleção COM razão tipada
    /// (ausência ≠ zero: fitness ausente OU denominador insuficiente
    /// OU abaixo do piso declarado).
    pub fn o5_reject(&self, reason: &str) {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        inner.stats.o5_rejections += 1;
        inner.stats.last_o5_verdict = Some(reason.to_string());
    }

    /// 19.8-e — veredito corrente da seleção offline (trilha).
    pub fn o5_verdict(&self) -> Option<String> {
        self.stats().last_o5_verdict
    }
}

impl Default for CyberneticsModule {
    fn default() -> Self {
        Self::new()
    }
}

/// Métricas vivas do substrato (mesma leitura read-only do
/// development): energia média dos ATIVOS + capacidade não-morta.
/// Sem clusters = None (ausência ≠ zero).
fn live_metrics(src: &Arc<l1::ClusterModule>) -> Option<(f32, f32)> {
    let runner_arc = src.shared_runner();
    let runner = runner_arc.lock().unwrap_or_else(|p| p.into_inner());
    let total = runner.clusters.len();
    if total == 0 {
        return None;
    }
    let mut sum = 0.0f64;
    let mut active = 0usize;
    let mut alive = 0usize;
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
    let energy = if active == 0 {
        0.0
    } else {
        (sum / active as f64) as f32
    };
    let capacity = alive as f32 / total as f32;
    Some((energy, capacity))
}

impl rt::CognitiveModule for CyberneticsModule {
    fn descriptor(&self) -> &tc::ModuleDescriptor {
        &self.descriptor
    }

    /// LEI 4 (crise muda política, nunca desliga): sempre Active.
    fn state(&self) -> rt::ModuleState {
        rt::ModuleState::Active
    }

    fn tick(
        &self,
        ctx: &rt::TypedContext,
        out: &mut Vec<tc::EventEnvelope>,
    ) -> tf::TriadResult<()> {
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => {
                return Err(tf::TriadError::ContractViolation {
                    detail: "mutex de cybernetics enveninado".to_string(),
                })
            }
        };
        inner.stats.ticks += 1;
        let tick = ctx.clock().tick;
        let interval = |steps: u64| -> u64 { steps.max(1) };

        // Métricas vivas (ausência contada — nenhum protocolo roda
        // sobre dado inventado).
        let (energy, capacity, resonance) = match &self.l1 {
            Some(src) => {
                let res = src.last_chladni().resonance.as_ref_value().copied();
                match live_metrics(src) {
                    Some((e, c)) => (Some(e), Some(c), res),
                    None => (None, None, res),
                }
            }
            None => (None, None, None),
        };
        if energy.is_none() {
            inner.stats.no_source_ticks += 1;
        }

        let mut ran_any = false;

        // ---- O1 (controle): homeostato sobre a energia média REAL.
        if self.config.o1_control.enabled
            && tick % interval(self.config.o1_control.interval_steps) == 0
        {
            if let Some(e) = energy {
                inner.energy.observe(e);
                inner.stats.o1_runs += 1;
                if inner.energy.is_satisfied() {
                    inner.stats.o1_satisfied += 1;
                }
                // 19.8-a — a correção deixa de ser debug e vira
                // URGÊNCIA CONTÍNUA entregue ao O3 (budget()/
                // assess) e ao runtime (load): valor SIGNED
                // (positivo = déficit de energia), nunca descartada.
                let corr = inner.energy.corrective();
                inner.stats.o1_last_corrective = Some(corr);
                if !inner.energy.is_satisfied() {
                    debug!(correcao = corr, "O1: energia fora da banda");
                }
                ran_any = true;
            }
        }

        // ---- O2 (adaptação): oscilador sobre a ressonância REAL +
        // taxa de estabilidade COM DENOMINADOR + 19.8-b RETUNING DO
        // CONTROLADOR O1 (o loop O2→O1 que não existia — auditoria
        // 19-0: antes a oscilação era só warn).
        if self.config.o2_adaptation.enabled
            && tick % interval(self.config.o2_adaptation.interval_steps) == 0
        {
            if let Some(r) = resonance {
                inner.osc.observe(r);
                inner.stats.o2_runs += 1;
                if inner.osc.is_oscillating() {
                    inner.stats.o2_oscillating_windows += 1;
                    inner.o2_stable_streak = 0;
                    let g = inner.energy.retune_gain(
                        -0.05,
                        "O2: ressonancia oscilando — ganho do O1 reduzido",
                    );
                    inner.stats.o2_retunes_down += 1;
                    warn!(
                        ganho_resultante = g,
                        "O2: retuning DOWN do controlador O1 (histerese legada)"
                    );
                } else {
                    // Janela estável: histerese — só recupera o ganho
                    // após 2 janelas estáveis CONSECUTIVAS.
                    inner.o2_stable_streak += 1;
                    if inner.o2_stable_streak >= 2 {
                        let g = inner.energy.retune_gain(
                            0.02,
                            "O2: 2 janelas estaveis — ganho do O1 recuperado",
                        );
                        inner.stats.o2_retunes_up += 1;
                        inner.o2_stable_streak = 0;
                        debug!(
                            ganho_resultante = g,
                            "O2: retuning UP do controlador O1"
                        );
                    }
                }
                ran_any = true;
            }
        }

        // ---- O3 (coordenação): governador sobre estresse/energia
        // vivos; ação TIPADA publicada com razão e APLICADA pelo
        // runtime (19.8-c). Stress CONTÍNUO: urgência |corr| do O1
        // (19.8-a) + alarmes O4 como sinal adicional (19.8-d) —
        // crise muda POLÍTICA de carga, nunca desativa (Lei 4).
        if self.config.o3_coordination.enabled
            && tick % interval(self.config.o3_coordination.interval_steps) == 0
        {
            if let Some(e) = energy {
                inner.stats.o3_runs += 1;
                let mut stress = match inner.stats.o1_last_corrective {
                    // Déficit de energia (corr > 0) é o estresse
                    // contínuo real; excesso (corr < 0) não é stress.
                    Some(corr) => corr.max(0.0).clamp(0.0, 1.0),
                    None => {
                        if inner.energy.is_satisfied() {
                            0.0
                        } else {
                            1.0
                        }
                    }
                };
                if !inner.stats.alarms.is_empty() {
                    // 19.8-d — alarmes do O4 elevam o stress com
                    // razão registrada nos próprios alarmes (trilha).
                    stress = (stress + 0.2).clamp(0.0, 1.0);
                }
                let action = inner.gov.assess(stress, e);
                match &action {
                    GoverningAction::Throttle { .. } => {
                        inner.stats.o3_throttles += 1;
                    }
                    GoverningAction::Shed { .. } => {
                        inner.stats.o3_sheds += 1;
                    }
                    GoverningAction::Normal => {}
                }
                inner.stats.last_action = Some(action);
                ran_any = true;
            }
        }

        // ---- O4 (observação): audita os PRÓPRIOS eventos com
        // denominador (eventos/tick) + satisfação O1 com denominador
        // + 19.8-d ALARMES TIPADOS que ALIMENTAM O2/O3 (o feedback
        // O4→O2/O3 que não existia — auditoria 19-0: antes só debug).
        if self.config.o4_self_observation.enabled
            && tick % interval(self.config.o4_self_observation.interval_steps) == 0
        {
            inner.stats.o4_runs += 1;
            let rate = audit_rate(
                inner.stats.events_published + 1,
                inner.stats.ticks,
                tf::ModuleId::new(),
                tf::id::StepId::new(),
            );
            // Alarmes REAVALIADOS a cada auditoria: a lista VIVA é
            // função pura dos contadores (herança
            // runtime_observability.py:315 — estado+motivo).
            inner.stats.alarms.clear();
            match rate.as_ref_value() {
                Some(v) if v.value() < O4_EVENTS_PER_TICK_FLOOR => {
                    let (ev, tk) = (inner.stats.events_published + 1, inner.stats.ticks);
                    inner.stats.alarms.push(MetricAlarm {
                        metric: "cybernetics.events_per_tick",
                        value: v.value(),
                        reason: format!(
                            "audit {}/{} abaixo do piso {} (eventos/tick)",
                            ev, tk, O4_EVENTS_PER_TICK_FLOOR
                        ),
                    });
                    inner.stats.o4_alarms += 1;
                    warn!(
                        valor = v.value(),
                        piso = O4_EVENTS_PER_TICK_FLOOR,
                        "O4: alarme tipado — audit abaixo do piso"
                    );
                }
                _ => {}
            }
            debug!(
                eventos_por_tick = ?rate.as_ref_value().map(|r| r.value()),
                satisfeitas_o1 = inner.stats.o1_satisfied,
                execucoes_o1 = inner.stats.o1_runs,
                "O4: auto-auditoria com denominador"
            );
            ran_any = true;
        }

        // ---- O5 (horizonte): coleta AMOSTRA por tick para a
        // tendência offline; o GENOME NUNCA roda aqui (Lei 7:
        // genome_in_main_loop=false é lido e respeitado).
        if let Some(r) = resonance {
            inner.horizon.push(r);
        }

        let cap_fmt = capacity.map(|c| format!("{c:.3}")).unwrap_or_else(|| "AUSENTE".into());
        let status = CyberneticStatus {
            ticks: inner.stats.ticks,
            energy,
            capacity,
            o1_satisfied: inner.stats.o1_satisfied,
            o1_runs: inner.stats.o1_runs,
            o2_runs: inner.stats.o2_runs,
            o3_runs: inner.stats.o3_runs,
            o4_runs: inner.stats.o4_runs,
            last_action_label: match &inner.stats.last_action {
                Some(GoverningAction::Normal) => Some("Normal".to_string()),
                Some(GoverningAction::Throttle { .. }) => Some("Throttle".to_string()),
                Some(GoverningAction::Shed { .. }) => Some("Shed".to_string()),
                None => None,
            },
            o1_last_corrective: inner.stats.o1_last_corrective,
            o2_retunes_down: inner.stats.o2_retunes_down,
            o2_retunes_up: inner.stats.o2_retunes_up,
            o2_oscillating_windows: inner.stats.o2_oscillating_windows,
            o4_alarms: inner.stats.o4_alarms,
            alarms_alive: inner.stats.alarms.len() as u64,
        };
        debug!(capacidade = %cap_fmt, "cybernetics tick");

        ctx.set("cybernetics.status", status);
        if ran_any {
            inner.stats.events_published += 1;
            info!(
                o1 = inner.stats.o1_runs,
                o2 = inner.stats.o2_runs,
                o3 = inner.stats.o3_runs,
                o4 = inner.stats.o4_runs,
                "ordens ciberneticas executadas sobre metricas vivas"
            );
            out.push(rt::envelope(
                "cybernetics.metrics",
                tick,
                tc::EventType::Governance,
                tc::Priority::Normal,
            ));
        }
        Ok(())
    }
}

/// Fotografia publicada no contexto a cada tick.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CyberneticStatus {
    pub ticks: u64,
    pub energy: Option<f32>,
    pub capacity: Option<f32>,
    pub o1_satisfied: u64,
    pub o1_runs: u64,
    pub o2_runs: u64,
    pub o3_runs: u64,
    pub o4_runs: u64,
    pub last_action_label: Option<String>,
    /// 19.8-a — correção O1 corrente (urgência contínua).
    #[serde(default)]
    pub o1_last_corrective: Option<f32>,
    /// 19.8-b — retunings do ganho do O1 (denominador o2_runs).
    #[serde(default)]
    pub o2_retunes_down: u64,
    #[serde(default)]
    pub o2_retunes_up: u64,
    #[serde(default)]
    pub o2_oscillating_windows: u64,
    /// 19.8-d — alarmes vivos do O4 (denominador o4_runs).
    #[serde(default)]
    pub o4_alarms: u64,
    #[serde(default)]
    pub alarms_alive: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use triad_runtime::CognitiveModule as _;

    #[test]
    fn sem_ponte_ausencia_contada_e_nenhuma_ordem_roda_sobre_inventado() {
        let cyb = CyberneticsModule::new();
        let mut clock = tf::LogicalClock::new();
        for _ in 0..12 {
            clock.advance();
            let ctx = rt::TypedContext::new(clock);
            let mut out = Vec::new();
            cyb.tick(&ctx, &mut out).expect("tick");
        }
        let s = cyb.stats();
        assert_eq!(s.ticks, 12);
        assert_eq!(s.no_source_ticks, 12, "ausência contada por tick");
        assert_eq!(s.o1_runs, 0, "O1 nunca roda sem métrica viva");
        assert_eq!(s.o2_runs, 0);
        assert_eq!(s.o3_runs, 0);
    }

    #[test]
    fn estabilidade_tem_denominador_no_oscilador() {
        let mut m = OscillationMonitor::with_window(8);
        m.observe(0.1);
        let q = m.stability_rate(tf::ModuleId::new(), tf::id::StepId::new());
        assert!(!q.is_value(), "1 amostra ⇒ NO_DATA (ausência ≠ zero)");
        m.observe(0.9);
        m.observe(0.1);
        let q = m.stability_rate(tf::ModuleId::new(), tf::id::StepId::new());
        assert!(q.is_value(), "2 amostras ⇒ taxa existe");
        // 2 cruzamentos em 2 intervalos ⇒ estabilidade 1 - 1 = 0.
        assert_eq!(q.as_ref_value().expect("valor").value(), 0.0);
    }

    #[test]
    fn a_a_determinismo_sem_ponte() {
        let run = || {
            let cyb = CyberneticsModule::new();
            let mut clock = tf::LogicalClock::new();
            let mut log: Vec<u64> = Vec::new();
            for _ in 0..6 {
                clock.advance();
                let ctx = rt::TypedContext::new(clock);
                let mut out = Vec::new();
                cyb.tick(&ctx, &mut out).expect("tick");
                log.push(out.len() as u64);
            }
            log.push(cyb.stats().no_source_ticks);
            log
        };
        assert_eq!(run(), run(), "mesma sequência ⇒ mesmo cybernetics (A/A)");
    }

    #[test]
    fn genome_fica_fora_do_loop_principal_lei_7() {
        // A config da casa declara genome_in_main_loop=false: o tick
        // NÃO expõe nenhuma execução de genome — só amostras de
        // horizonte. Verificação estrutural: nenhum método de
        // Genome é chamado no tick (auditado por leitura de código)
        // e a config congelada é false.
        let cfg = CyberneticsCfg::default();
        assert!(!cfg.o5_evolution.genome_in_main_loop, "Lei 7");
        assert_eq!(cfg.o5_evolution.cadence, "between_episodes");
    }

    /// Integração REAL: com o L1 vivo, as ordens executam nas
    /// cadências declaradas sobre métricas medidas (denominadores).
    #[test]
    fn ordens_executam_sobre_metricas_vivas_do_l1() {
        let l1m = Arc::new(l1::ClusterModule::new(42, 16));
        let cyb = CyberneticsModule::new().with_l1_source(Arc::clone(&l1m));
        let mut clock = tf::LogicalClock::new();
        // L1 ticka primeiro (o sinal chladni sai VALUE com vivos).
        clock.advance();
        let ctx = rt::TypedContext::new(clock);
        let mut out_l1 = Vec::new();
        use triad_runtime::CognitiveModule as _;
        l1m.tick(&ctx, &mut out_l1).expect("l1");

        let mut eventos = 0usize;
        for t in 1..=101 {
            clock.advance();
            let ctx = rt::TypedContext::new(clock);
            let mut out = Vec::new();
            cyb.tick(&ctx, &mut out).expect("tick");
            eventos += out.len();
            let _ = t;
        }
        let s = cyb.stats();
        assert_eq!(s.ticks, 101);
        assert_eq!(s.no_source_ticks, 0, "L1 presente o tempo todo");
        // Cadências da casa: O1 a cada 10 ⇒ 10 execuções em 101 ticks.
        assert_eq!(s.o1_runs, 10, "O1 em 10/20/.../100");
        assert_eq!(s.o2_runs, 4, "O2 em 25/50/75/100");
        assert_eq!(s.o3_runs, 2, "O3 em 50/100");
        assert_eq!(s.o4_runs, 1, "O4 em 100");
        assert!(eventos > 0, "evento cybernetics.metrics por execução");
        assert!(s.o1_satisfied <= s.o1_runs, "denominador respeitado");
        // A ação O3 é tipada e legível.
        let label = cyb
            .stats()
            .last_action
            .as_ref()
            .map(|a| match a {
                GoverningAction::Normal => "Normal",
                GoverningAction::Throttle { .. } => "Throttle",
                GoverningAction::Shed { .. } => "Shed",
            })
            .unwrap_or("nenhuma");
        assert_ne!(label, "nenhuma", "O3 decidiu algo em 2 execuções");
    }

    /// 19.8-a/b/c/d — os QUATRO loops cibernéticos fecham de fato:
    /// O1 corr guardada (urgência contínua), O2 retuna o controlador
    /// O1, O3 decide com razão aplicável, O4 emite alarme tipado
    /// quando o piso é furado — tudo com denominador e A/A.
    #[test]
    fn loops_o1_o2_o3_o4_fecham_com_denominadores() {
        // (a) O1 corr é SIGNED e legível; déficit = urgência positiva.
        let mut h = Homeostat::new(0.5, 0.3);
        h.observe(0.1);
        let corr = h.corrective();
        assert!(corr > 0.0, "abaixo do alvo ⇒ correção positiva");
        // (b) O2 retuning: clamp [0.25, 1.0] do ganho (0.5×..2× do
        // nominal 0.5) — controle NUNCA desliga.
        let mut h2 = Homeostat::new(0.5, 0.3);
        for _ in 0..50 {
            h2.retune_gain(-0.05, "teste");
        }
        assert_eq!(h2.gain(), 0.25, "piso do retuning");
        for _ in 0..50 {
            h2.retune_gain(0.02, "teste");
        }
        assert_eq!(h2.gain(), 1.0, "teto do retuning");
        // (c/d) sem ponte L1, nenhuma ordem roda sobre inventado —
        // mas a auditoria O4 AINDA audita os PRÓPRIOS contadores e
        // emite alarme tipado quando o piso é furado (eventos 0).
        let cyb = CyberneticsModule::new();
        let mut clock = tf::LogicalClock::new();
        for _ in 0..101 {
            clock.advance();
            let ctx = rt::TypedContext::new(clock);
            let mut out = Vec::new();
            cyb.tick(&ctx, &mut out).expect("tick");
        }
        let s = cyb.stats();
        assert_eq!(s.o4_runs, 1, "O4 em 100");
        assert_eq!(s.o4_alarms, 1, "audit 1/101 < piso 0.5 ⇒ alarme");
        assert_eq!(s.alarms.len(), 1, "alarme VIVO com razão");
        assert!(s.alarms[0].reason.contains('/'), "razão com denominador");
        assert_eq!(s.o1_last_corrective, None, "sem métrica viva ⇒ ausência ≠ 0");
        // load_directive: None sem O3 executado (ausência ≠ zero).
        assert!(cyb.load_directive().is_none());
    }

    /// 19.8-a/c — com o L1 vivo o loop INTEIRO fecha: O1 corr vira
    /// stress contínuo do O3, a ação é legível e APLICÁVEL.
    #[test]
    fn load_directive_reflete_decisao_o3_sobre_urgencia_o1() {
        let l1m = Arc::new(l1::ClusterModule::new(42, 16));
        let cyb = CyberneticsModule::new().with_l1_source(Arc::clone(&l1m));
        let mut clock = tf::LogicalClock::new();
        clock.advance();
        let ctx = rt::TypedContext::new(clock);
        let mut out_l1 = Vec::new();
        use triad_runtime::CognitiveModule as _;
        l1m.tick(&ctx, &mut out_l1).expect("l1");
        for _ in 1..=101 {
            clock.advance();
            let ctx = rt::TypedContext::new(clock);
            let mut out = Vec::new();
            cyb.tick(&ctx, &mut out).expect("tick");
        }
        let s = cyb.stats();
        // O1 executou ⇒ corr guardada (urgência contínua no O3).
        assert!(s.o1_last_corrective.is_some(), "corr O1 legível");
        // O3 executou ⇒ ação tipada aplicável pelo runtime.
        let dir = cyb.load_directive();
        assert!(dir.is_some(), "load_directive viva (consumidor O3)");
        if let Some(GoverningAction::Throttle { factor, reason }) = &dir {
            assert!((0.25..=1.0).contains(factor), "fator clampado");
            assert!(!reason.is_empty(), "razão canônica presente");
        }
        // Invariantes de denominador: retunings nunca excedem runs.
        assert!(s.o2_retunes_down + s.o2_retunes_up <= s.o2_runs);
        assert!(s.o2_oscillating_windows <= s.o2_runs);
        assert!(s.o4_alarms <= s.o4_runs);
    }

    /// A/A bit-exato do cybernetics com os NOVOS loops: mesma seed
    /// ⇒ mesmas decisões (corr, retunes, alarms, ação).
    #[test]
    fn loops_novos_sao_deterministicos_aa() {
        let run = || {
            let l1m = Arc::new(l1::ClusterModule::new(42, 16));
            let cyb = CyberneticsModule::new().with_l1_source(Arc::clone(&l1m));
            let mut clock = tf::LogicalClock::new();
            clock.advance();
            let ctx = rt::TypedContext::new(clock);
            let mut out_l1 = Vec::new();
            use triad_runtime::CognitiveModule as _;
            l1m.tick(&ctx, &mut out_l1).expect("l1");
            for _ in 1..=121 {
                clock.advance();
                let ctx = rt::TypedContext::new(clock);
                let mut out = Vec::new();
                cyb.tick(&ctx, &mut out).expect("tick");
            }
            let s = cyb.stats();
            (
                s.o1_last_corrective.map(|v| v.to_bits()),
                s.o2_retunes_down,
                s.o2_retunes_up,
                s.o2_oscillating_windows,
                s.o4_alarms,
                s.o3_throttles,
                s.o3_sheds,
                s.last_action.is_some(),
            )
        };
        assert_eq!(run(), run(), "gêmeos bit-exatos com os 4 loops");
    }

    /// 19.8-e — seleção offline no boundary: Genome determinístico
    /// (LCG), promoção SÓ do vencedor com recibo, rejeição com
    /// razão tipada (ausência ≠ zero), reversão pelo retune oposto.
    #[test]
    fn selecao_o5_boundary_promove_so_vencedor_com_recibo() {
        use crate::genome::Genome;
        // Genoma: genes REAIS do controlador; mutação determinística.
        let mk = |v: f32| {
            Genome::new(vec![
                ("o1.gain".to_string(), v),
                ("o2.retune_down".to_string(), 0.05),
                ("o2.retune_up".to_string(), 0.02),
                ("o3.factor_min".to_string(), 0.25),
                ("o3.stress_delta".to_string(), 0.75),
                ("o4.floor".to_string(), 0.5),
                ("o4.alarm_stress".to_string(), 0.2),
                ("o5.rate".to_string(), 0.2),
            ])
        };
        let g = mk(0.5);
        let m1 = g.mutate(65, 0.2);
        let m2 = g.mutate(65, 0.2);
        assert_eq!(m1.genes, m2.genes, "mesma seed ⇒ mesmo mutante (A/A)");
        let m3 = g.mutate(66, 0.2);
        // Seed diferente muda o mutante (seleção não é identidade).
        let changed = m1
            .genes
            .iter()
            .zip(m3.genes.iter())
            .any(|((n1, v1), (n2, v2))| n1 == n2 && v1 != v2);
        assert!(changed, "seeds diferentes divergem");
        // Campeão com score: ranking por fitness; gene ausente ⇒ 0.
        let scores = vec![("o1.gain".to_string(), 0.9f32)];
        let cands = m1.evaluate_offline(&scores);
        assert_eq!(cands[0].id, "o1.gain");
        assert!((cands[0].fitness - 0.9).abs() < 1e-6, "score do campeão");
        // Módulo: promoção com recibo e clamp (nunca desliga).
        let cyb = CyberneticsModule::new();
        let antes = cyb.o1_gain();
        let aplicado = cyb.o5_promote_retune(0.02, "teste: fitness 0.9/10");
        assert!((aplicado - (antes + 0.02)).abs() < 1e-6, "promoção aplica");
        // Reversão (Lei 6): retune oposto restaura.
        let restaurado = cyb.o5_promote_retune(-0.02, "reversão sem efeito observado");
        assert!((restaurado - antes).abs() < 1e-6, "intervenção reversível");
        // Rejeição com razão tipada (ausência ≠ zero).
        cyb.o5_reject("fitness AUSENTE — nada coletado");
        let s = cyb.stats();
        assert_eq!(s.o5_promotions, 2);
        assert_eq!(s.o5_rejections, 1);
        assert!(s.o5_rejections <= s.o5_promotions + s.o5_rejections);
        assert_eq!(s.last_o5_verdict.as_deref(), Some("fitness AUSENTE — nada coletado"));
        // Clamp: promoção absurda não quebra os limites da casa.
        let cyb2 = CyberneticsModule::new();
        let fundo = cyb2.o5_promote_retune(-10.0, "teste clamp");
        assert_eq!(fundo, 0.25, "piso do ganho (controle nunca desliga)");
        let teto = cyb2.o5_promote_retune(10.0, "teste clamp");
        assert_eq!(teto, 1.0, "teto do ganho");
    }
}
