//! Escalonador de módulos cognitivos por passo lógico.
//!
//! 17.4 — CONCORRÊNCIA INTERNA DE MÓDULOS (diretriz do dono: módulos
//! em paralelo na mesma máquina): o passo ticka GRUPOS de módulos
//! independentes em paralelo (rayon scoped, barreira implícita ao fim
//! do grupo) e os grupos executam em ORDEM. A publicação de eventos é
//! sempre na ORDEM CANÔNICA (índice ascendente de registro) — quem
//! termina primeiro não adianta a fila. Default: grupos unitários =
//! bit-idêntico ao sequencial (zero rayon, zero overhead); a cadeia
//! L1→L5 segue sequencial por DEPENDÊNCIA REAL de leitura no mesmo
//! passo; substratos independentes (regiões) agrupam-se livremente.

use std::collections::HashSet;
use std::sync::Arc;
use triad_contracts as tc;
use triad_foundation as tf;

/// Orçamento máximo de eventos publicados por passo.
pub struct StepBudget {
    pub max_events: usize,
}

impl StepBudget {
    /// Cria um orçamento com limite de eventos por passo.
    pub fn new(max_events: usize) -> Self {
        Self { max_events }
    }
}

/// Relatório consolidado da execução de um passo.
pub struct StepReport {
    pub step: tf::id::StepId,
    pub tick: u64,
    pub executed: usize,
    pub skipped: usize,
    pub events_published: usize,
    pub budget_exceeded: bool,
    pub degraded: bool,
    /// 17.6 — latência por módulo (nome, µs) na ordem canônica:
    /// o perfil de escala do organismo (telemetria profunda).
    pub module_latency: Vec<(String, u64)>,
    /// 18.7 — hash OBSERVACIONAL do estado por módulo (nome,
    /// Option<u64>), mesma ordem canônica: A/A bit-exato por camada
    /// (mesma seed ⇒ mesmo hash; a latência varia, o hash NÃO).
    /// `None` = camada ainda sem estado canônico instrumentado
    /// (ausência ≠ zero — débito por camada no checklist).
    pub state_hashes: Vec<(String, Option<u64>)>,
    /// 19.5 (sob diretriz da dona) — budget POR MÓDULO com throttle:
    /// módulos PULADOS por política de carga neste passo (nome,
    /// fator corrente). Determinístico: função de (tick, fator) —
    /// A/A bit-exato (mesma política ⇒ mesmos saltos). Sem política
    /// configurada ⇒ lista VAZIA (ausência ≠ zero: comportamento
    /// bit-idêntico ao histórico).
    pub throttled: Vec<(String, f64)>,
    /// 20.6a (SEÇÃO 20) — BUDGET POR CAMADA (herança do legado
    /// cognitive_budget.py): gasto (µs) DEBITADO NA CAMADA DONA,
    /// cotas por GRUPO L1-L2 25% / L3-L4 35% / L5 25% / overhead
    /// (T/P/B) 15%. OBSERVACIONAL: registra o perfil de gasto,
    /// NUNCA altera execução (throttle do budget é SEMPRE 1.0 em
    /// modo determinístico — zero acoplamento wall-clock→cognição).
    /// `share`/`group_share` = Some(x/y) com DENOMINADOR (Lei 1);
    /// total 0 ⇒ None (ausência ≠ zero — Lei 2). Sempre 8 linhas
    /// na ordem canônica L1..L5,T,P,B (determinístico).
    pub layer_budget: Vec<LayerBudgetLine>,
}

/// Linha do budget por camada (SEÇÃO 20.6a): gasto da camada +
/// veredito do grupo (cota compartilhada). `over_cap` é REGISTRO
/// honesto, sem efeito (o budget não desliga nada — Lei 4).
#[derive(Debug, Clone, PartialEq)]
pub struct LayerBudgetLine {
    /// Camada dona do gasto ("L1".."L5","T","P","B" — as_str).
    pub layer: &'static str,
    /// µs gastos por módulos DESTA camada neste passo.
    pub spent_us: u64,
    /// spent/total do passo (0 < share ≤ 1.0); None se total 0.
    pub share: Option<f64>,
    /// Grupo de cota: "L1-L2" | "L3-L4" | "L5" | "overhead".
    pub group: &'static str,
    /// Cota do grupo (0.25/0.35/0.25/0.15 — legado).
    pub group_cap: f64,
    /// µs gastos pelo GRUPO inteiro neste passo.
    pub group_spent_us: u64,
    /// group_spent/total; None se total 0 (ausência ≠ zero).
    pub group_share: Option<f64>,
    /// Grupo excedeu a cota neste passo? REGISTRO sem efeito.
    pub over_cap: bool,
}

/// Escalonador que executa módulos cognitivos e publica eventos no barramento.
pub struct Scheduler {
    pub modules: Vec<Arc<dyn crate::module::CognitiveModule>>,
    pub budget: StepBudget,
    pub bus: crate::event_bus::EventBus,
    clock: tf::LogicalClock,
    degraded_overrides: HashSet<tf::id::ModuleId>,
    /// 17.4 — grupos de concorrência por índice: dentro do grupo os
    /// módulos tickam em PARALELO (independentes entre si); entre
    /// grupos a ordem é a declarada. Default: `[[0],[1],..]`.
    concurrency: Vec<Vec<usize>>,
    /// 19.5 (sob diretriz da dona) — throttle POR MÓDULO (por NOME
    /// canônico do descritor): 1.0 = sempre executa; 0.5 = cada 2
    /// passos; 0.25 = cada 4. PERÍODO determinístico
    /// round(1/fator) — herança do cognitive_budget do legado
    /// (system.py:9540-9579: a carga modulada nunca desativa
    /// sistemas, apenas reduz cadência — Lei 4).
    throttles: std::collections::BTreeMap<String, f64>,
}

/// Resultado do tick de um módulo do grupo (fase paralela → merge).
#[derive(Clone)]
enum TickOutcome {
    Skipped,
    Ok(Vec<tc::EventEnvelope>),
    Err,
}

impl Scheduler {
    /// Cria escalonador com módulos, orçamento, barramento e relógio novo.
    /// Grupos de concorrência DEFAULT (unitários): execução sequencial
    /// canônica — bit-idêntica ao comportamento histórico.
    pub fn new(
        modules: Vec<Arc<dyn crate::module::CognitiveModule>>,
        budget: StepBudget,
        bus: crate::event_bus::EventBus,
    ) -> Self {
        let n = modules.len();
        Self {
            modules,
            budget,
            bus,
            clock: tf::LogicalClock::new(),
            degraded_overrides: HashSet::new(),
            concurrency: (0..n).map(|i| vec![i]).collect(),
            throttles: std::collections::BTreeMap::new(),
        }
    }

    /// Cria escalonador com GRUPOS DE CONCORRÊNCIA explícitos: cada
    /// grupo é uma lista de índices de módulos INDEPENDENTES entre si
    /// (tickam em paralelo); os grupos executam na ordem declarada.
    /// Validação na construção: cobertura exata (cada índice uma vez).
    pub fn with_concurrency(
        modules: Vec<Arc<dyn crate::module::CognitiveModule>>,
        budget: StepBudget,
        bus: crate::event_bus::EventBus,
        groups: Vec<Vec<usize>>,
    ) -> Self {
        let mut scheduler = Self::new(modules, budget, bus);
        let n = scheduler.modules.len();
        let mut vistos: HashSet<usize> = HashSet::new();
        for grupo in &groups {
            for &i in grupo {
                assert!(
                    vistos.insert(i),
                    "grupo de concorrência repete o índice {i}"
                );
                assert!(i < n, "grupo referencia índice {i} inexistente ({n} módulos)");
            }
        }
        assert_eq!(
            vistos.len(),
            n,
            "grupos de concorrência devem cobrir todos os {n} módulos exatamente uma vez"
        );
        scheduler.concurrency = groups;
        scheduler
    }

    /// Acesso imutável ao relógio lógico corrente.
    pub fn clock(&self) -> &tf::LogicalClock {
        &self.clock
    }

    /// 19.5 (sob diretriz da dona) — define o throttle de um módulo
    /// por NOME canônico. Clamp [0.01, 1.0]: nunca zero (módulo
    /// NUNCA desliga — Lei 4; Shed completo não existe, existe
    /// cadência mínima). Fatores fora da faixa são clampados com a
    /// política aplicada (telemetria honesta via `module_throttle`).
    pub fn set_module_throttle(&mut self, module: &str, factor: f64) {
        let f = factor.clamp(0.01, 1.0);
        self.throttles.insert(module.to_string(), f);
    }

    /// 19.5 — remove a política de carga do módulo (volta ao
    /// comportamento bit-idêntico ao histórico: executa sempre).
    pub fn clear_module_throttle(&mut self, module: &str) {
        self.throttles.remove(module);
    }

    /// 19.5 — fator corrente do módulo (None = sem política:
    /// ausência ≠ zero, executa sempre).
    pub fn module_throttle(&self, module: &str) -> Option<f64> {
        self.throttles.get(module).copied()
    }

    /// 19.5 — decisão determinística de execução: período
    /// round(1/fator) em passos; executa quando tick % período == 0.
    /// A/A: mesma política + mesma sequência ⇒ mesmos saltos.
    fn throttle_allows(factor: f64, tick: u64) -> bool {
        if factor >= 1.0 {
            return true;
        }
        let period = ((1.0 / factor).round() as u64).max(1);
        tick % period == 0
    }

    /// Executa um passo: tica módulos ativos, publica eventos e avança
    /// o relógio. Grupos unitários executam inline (sem rayon);
    /// grupos com 2+ módulos tickam em paralelo com barreira ao fim.
    pub fn step(&mut self) -> StepReport {
        let (mut executed, mut skipped, mut published) = (0usize, 0usize, 0usize);
        let (mut budget_exceeded, mut degraded) = (false, false);
        // 17.6 — telemetria de latência POR MÓDULO (µs), sempre na
        // ordem canônica de registro: o perfil de escala do dono.
        let mut latencias: Vec<(String, u64)> = Vec::new();
        // 18.7 — hash observacional por módulo, coletado APÓS o tick na
        // MESMA ordem canônica do merge (determinístico: função do
        // estado pós-tick, nunca do wall-clock).
        let mut hashes: Vec<(String, Option<u64>)> = Vec::new();
        // 19.5 — telemetria da política de carga aplicada no passo.
        let mut throttled_now: Vec<(String, f64)> = Vec::new();
        for grupo in self.concurrency.clone() {
            // FASE 1 (serial): decide quem está ativo ANTES de tickar
            // — módulo inativo nunca roda (a semântica do skip é a
            // mesma do escalonador sequencial histórico).
            let clock_snap = self.clock;
            let mut outcomes: Vec<TickOutcome> = vec![TickOutcome::Skipped; self.modules.len()];
            let mut micros: Vec<u64> = vec![0; self.modules.len()];
            let ativos: Vec<usize> = grupo
                .iter()
                .copied()
                .filter(|&i| {
                    let id = self.modules[i].descriptor().module_id;
                    if !self.degraded_overrides.contains(&id) && self.modules[i].state().can_tick()
                    {
                        // 19.5 — política de carga POR MÓDULO: o
                        // fator da cadência decide se o módulo roda
                        // NESTE tick (determinístico, A/A).
                        match self
                            .throttles
                            .get(&self.modules[i].descriptor().name)
                            .copied()
                        {
                            Some(f) => Self::throttle_allows(f, clock_snap.tick),
                            None => true,
                        }
                    } else {
                        false
                    }
                })
                .collect();
            // FASE 2 (paralela por grupo): cada ativo com contexto
            // próprio do MESMO passo e fila própria de eventos.
            if ativos.len() <= 1 {
                for &i in &ativos {
                    let (o, us) = Self::tick_one(&self.modules[i], clock_snap);
                    outcomes[i] = o;
                    micros[i] = us;
                }
            } else {
                use rayon::prelude::*;
                let resultados: Vec<(usize, TickOutcome, u64)> = ativos
                    .par_iter()
                    .map(|&i| {
                        let (o, us) = Self::tick_one(&self.modules[i], clock_snap);
                        (i, o, us)
                    })
                    .collect();
                for (i, o, us) in resultados {
                    outcomes[i] = o;
                    micros[i] = us;
                }
            }
            // FASE 3 (merge determinístico): ordem canônica — índice
            // ascendente — publica, degrada e conta. Quem termina
            // primeiro NÃO adianta a fila (barreira + ordem canônica).
            let mut ordenado = grupo;
            ordenado.sort_unstable();
            for i in ordenado {
                let id = self.modules[i].descriptor().module_id;
                let outcome = std::mem::replace(&mut outcomes[i], TickOutcome::Skipped);
                match outcome {
                    TickOutcome::Skipped => {
                        skipped += 1;
                        // 19.5 — se a política de carga pulou ESTE
                        // módulo neste tick, telemetria com fator.
                        if let Some(f) = self
                            .throttles
                            .get(&self.modules[i].descriptor().name)
                            .copied()
                        {
                            if !Self::throttle_allows(f, clock_snap.tick) {
                                throttled_now.push((
                                    self.modules[i].descriptor().name.clone(),
                                    f,
                                ));
                            }
                        }
                    }
                    TickOutcome::Ok(out) => {
                        self.degraded_overrides.remove(&id);
                        executed += 1;
                        latencias
                            .push((self.modules[i].descriptor().name.clone(), micros[i]));
                        hashes.push((
                            self.modules[i].descriptor().name.clone(),
                            self.modules[i].state_hash(),
                        ));
                        for ev in out {
                            if published >= self.budget.max_events {
                                budget_exceeded = true;
                                break;
                            }
                            if self.bus.publish(ev).is_ok() {
                                published += 1;
                            } else {
                                budget_exceeded = true;
                            }
                        }
                    }
                    TickOutcome::Err => {
                        self.degraded_overrides.insert(id);
                        degraded = true;
                        skipped += 1;
                        latencias
                            .push((self.modules[i].descriptor().name.clone(), micros[i]));
                        hashes.push((
                            self.modules[i].descriptor().name.clone(),
                            self.modules[i].state_hash(),
                        ));
                        eprintln!("módulo {:?} degradado por erro no tick", id);
                    }
                }
            }
        }
        self.clock.advance();
        StepReport {
            step: self.clock.step,
            tick: self.clock.tick,
            executed,
            skipped,
            events_published: published,
            budget_exceeded,
            degraded,
            module_latency: latencias,
            state_hashes: hashes,
            throttled: throttled_now,
        }
    }

    /// Tick de UM módulo com contexto do passo corrente (usado tanto
    /// no caminho serial quanto no paralelo — mesmo código, mesmo
    /// resultado bit-exato) + latência medida (µs) para o perfil 17.6.
    fn tick_one(
        m: &Arc<dyn crate::module::CognitiveModule>,
        clock: tf::LogicalClock,
    ) -> (TickOutcome, u64) {
        let t0 = std::time::Instant::now();
        let ctx = crate::context::TypedContext::new(clock);
        let mut out = Vec::new();
        let outcome = match m.tick(&ctx, &mut out) {
            Ok(()) => TickOutcome::Ok(out),
            Err(_) => TickOutcome::Err,
        };
        (outcome, t0.elapsed().as_micros() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Módulo de teste DETERMINÍSTICO: publica 2 eventos por tick
    /// com (entity_id, entity_version) derivados do próprio tick.
    struct MockModule {
        descriptor: tc::ModuleDescriptor,
        ticks: AtomicU64,
    }

    impl MockModule {
        fn new(nome: &str) -> Self {
            Self {
                descriptor: tc::ModuleDescriptor::new(
                    tf::id::ModuleId::new(),
                    nome,
                    tc::Layer::L1,
                    "teste",
                ),
                ticks: AtomicU64::new(0),
            }
        }
    }

    impl crate::module::CognitiveModule for MockModule {
        fn descriptor(&self) -> &tc::ModuleDescriptor {
            &self.descriptor
        }
        fn state(&self) -> crate::module_state::ModuleState {
            crate::module_state::ModuleState::Active
        }
        fn tick(
            &self,
            ctx: &crate::context::TypedContext,
            out: &mut Vec<tc::EventEnvelope>,
        ) -> tf::TriadResult<()> {
            let tick = ctx.clock().tick;
            let n = self.ticks.fetch_add(1, Ordering::SeqCst);
            out.push(crate::envelope(
                &format!("{}#{n}a", self.descriptor.name),
                tick,
                tc::EventType::Cognitive,
                tc::Priority::Normal,
            ));
            out.push(crate::envelope(
                &format!("{}#{n}b", self.descriptor.name),
                tick,
                tc::EventType::Cognitive,
                tc::Priority::Normal,
            ));
            Ok(())
        }
    }

    /// Assinatura comparável do barramento: (entity_id, entity_version)
    /// de CADA evento na ordem de publicação — sem ids aleatórios.
    fn assinatura(scheduler: &Scheduler) -> Vec<(String, u64)> {
        scheduler
            .bus
            .try_receive(1024)
            .into_iter()
            .map(|ev| (ev.entity_id, ev.entity_version))
            .collect()
    }

    fn mocks(n: usize) -> Vec<Arc<dyn crate::module::CognitiveModule>> {
        (0..n)
            .map(|i| {
                let m: Arc<MockModule> = Arc::new(MockModule::new(&format!("mock{i}")));
                m as Arc<dyn crate::module::CognitiveModule>
            })
            .collect()
    }

    /// 17.4 — A/A BIT-IDÊNTICO: 4 módulos INDEPENDENTES, 10 passos —
    /// grupos unitários (sequencial) vs. um grupo [0,1,2,3] (rayon):
    /// a sequência de eventos publicados é IDÊNTICA na ordem canônica.
    #[test]
    fn grupo_paralelo_e_bit_identico_ao_sequencial() {
        // Run A: default (grupos unitários, sequencial).
        let mut a = Scheduler::new(mocks(4), StepBudget::new(1024), crate::event_bus::EventBus::new(1024));
        // Run B: TODOS em um grupo só (rayon paralelo com barreira).
        let mut b = Scheduler::with_concurrency(
            mocks(4),
            StepBudget::new(1024),
            crate::event_bus::EventBus::new(1024),
            vec![vec![0, 1, 2, 3]],
        );
        for _ in 0..10 {
            let ra = a.step();
            let rb = b.step();
            assert_eq!((ra.executed, ra.skipped, ra.events_published), (rb.executed, rb.skipped, rb.events_published));
            assert_eq!(assinatura(&a), assinatura(&b), "ordem canônica bit-idêntica por passo");
        }
        // Grupos mistos (2 grupos de 2): a ordem ENTRE grupos é a
        // DECLARADA (0,3 depois 1,2) — determinística, verificada
        // explicitamente passo a passo; contagens idênticas ao default.
        let mut c = Scheduler::with_concurrency(
            mocks(4),
            StepBudget::new(1024),
            crate::event_bus::EventBus::new(1024),
            vec![vec![0, 3], vec![1, 2]],
        );
        let mut d = Scheduler::new(mocks(4), StepBudget::new(1024), crate::event_bus::EventBus::new(1024));
        for passo in 0..10u64 {
            let rc = c.step();
            let rd = d.step();
            assert_eq!(
                (rc.executed, rc.skipped, rc.events_published),
                (rd.executed, rd.skipped, rd.events_published)
            );
            // Ordem DECLARADA dos grupos: [0,3] então [1,2]; dentro do
            // grupo, ordem canônica (índice ascendente).
            let esperado: Vec<(String, u64)> = [0usize, 3, 1, 2]
                .iter()
                .flat_map(|&i| {
                    vec![
                        (format!("mock{i}#{passo}a"), passo),
                        (format!("mock{i}#{passo}b"), passo),
                    ]
                })
                .collect();
            assert_eq!(assinatura(&c), esperado, "ordem declarada determinística");
        }
    }

    /// Validação dos grupos na construção: cobertura exata, sem duplo
    /// registro (panic = bug do chamador, não do escalonador).
    #[test]
    #[should_panic(expected = "exatamente uma vez")]
    fn grupos_devem_cobrir_todos_os_modulos_uma_vez() {
        Scheduler::with_concurrency(
            mocks(3),
            StepBudget::new(16),
            crate::event_bus::EventBus::new(16),
            vec![vec![0], vec![1]], // falta o 2
        );
    }

    /// 19.5 — SEM política ⇒ comportamento bit-idêntico ao
    /// histórico: nada throttled (ausência ≠ zero).
    #[test]
    fn sem_throttle_nada_e_pulado() {
        let mut s = Scheduler::new(mocks(2), StepBudget::new(16), crate::event_bus::EventBus::new(16));
        for _ in 0..4 {
            let r = s.step();
            assert!(r.throttled.is_empty(), "sem política ⇒ lista vazia");
        }
    }

    /// 19.5 — throttle 0.5 no "mock0": período 2, executa nos ticks
    /// pares; o outro módulo roda sempre; telemetria com fator.
    #[test]
    fn throttle_por_modulo_periodo_deterministico() {
        let mut s = Scheduler::new(mocks(2), StepBudget::new(16), crate::event_bus::EventBus::new(16));
        s.set_module_throttle("mock0", 0.5);
        // Clamp: fator zero vira 0.01 (nunca desliga — Lei 4).
        s.set_module_throttle("mock1", 0.0);
        assert_eq!(s.module_throttle("mock1"), Some(0.01));
        let mut mock0_exec = 0usize;
        for _ in 0..4 {
            let r = s.step();
            let lat: Vec<&str> = r.module_latency.iter().map(|(n, _)| n.as_str()).collect();
            if lat.contains(&"mock0") {
                mock0_exec += 1;
            }
        }
        assert_eq!(mock0_exec, 2, "0.5 ⇒ executa metade dos passos (período 2)");
        assert!(s.module_throttle("mock0").is_some());
        // clear restaura o comportamento histórico.
        s.clear_module_throttle("mock0");
        let r = s.step();
        assert!(r.throttled.is_empty() || r.throttled.iter().all(|(n, _)| n != "mock0"));
    }

    /// 19.5 — A/A: duas instâncias com a MESMA política saltam os
    /// mesmos módulos nos mesmos passos (bit-exato).
    #[test]
    fn throttle_e_deterministico_aa_bit_exato() {
        let run = || {
            let mut s = Scheduler::new(
                mocks(3),
                StepBudget::new(16),
                crate::event_bus::EventBus::new(16),
            );
            s.set_module_throttle("mock0", 0.5);
            s.set_module_throttle("mock2", 0.25);
            (0..8)
                .map(|_| {
                    let r = s.step();
                    (
                        r.executed,
                        r.skipped,
                        r.throttled.clone(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run(), "mesma política ⇒ mesmos saltos (A/A)");
    }
}
