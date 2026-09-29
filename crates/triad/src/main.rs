//! Organismo cognitivo Triad_AEE: sobe o núcleo L1–L5 e roda passos.
//!
//! L1 e L2 são REAIS: o substrato executa o passo físico e o tecido
//! deriva a organização mesoscópica sobre ele (formação por afinidade,
//! bridges, feedback com histerese), com as pontes instrumentadas por
//! recibos E3/E4 nos ledgers.

use std::sync::Arc;
use triad_contracts as tc;
use triad_foundation as tf;
use triad_l1_substrate as l1;
use triad_l2_tissue as l2;
use triad_l3_local as l3;
use triad_l4_global as l4;
use triad_l5_meta as l5;
use triad_platform as platform;
use triad_runtime as rt;

fn main() {
    // Nível debug: mostra a trilha de auditoria das cadeias L1/L2.
    rt::tracing_init("debug");

    // ---- Configuração centralizada (diretriz do dono) ----
    // O app carrega `config/default.toml` UMA vez e injeta nas camadas;
    // arquivo ausente é erro de boot — o organismo não inventa config.
    let cfg = match platform::PlatformConfig::load_default() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("FALHA NO BOOT: configuração central ausente/ilegível: {e:?}");
            return;
        }
    };
    println!(
        "=== Config: {} ===",
        cfg.source()
    );
    for nota in cfg.notes() {
        println!("[config] {nota}");
    }
    let seed = cfg.organism.seed;
    // Configs das camadas — extraídas do TOML central por seção.
    let l1cfg: l1::config::L1Config = match cfg.get_section("l1") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("FALHA NO BOOT: seção [l1] ausente: {e:?}");
            return;
        }
    };
    let l2cfg: l2::L2Config = match cfg.get_section("l2") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("FALHA NO BOOT: seção [l2] ausente: {e:?}");
            return;
        }
    };
    let l3cfg: l3::L3Config = match cfg.get_section("l3") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("FALHA NO BOOT: seção [l3] ausente: {e:?}");
            return;
        }
    };
    let l4cfg: l4::L4Config = match cfg.get_section("l4") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("FALHA NO BOOT: seção [l4] ausente: {e:?}");
            return;
        }
    };

    let bus = rt::EventBus::new(4096);
    // Auditor de eventos: assinante canônico do runtime (orientação a
    // eventos — cada camada publica, o auditor conta por fonte).
    let auditor = rt::EventAuditor::new(&bus, 65_536);

    // L1: substrato de clusters energéticos (semente do [organism]);
    // energia, homeostase e morfogênese vêm de `config/default.toml
    // [l1.*]` — o crescimento segue a política do TOML.
    let l1_mod = l1::ClusterModule::new_with_config(seed, 24, l1cfg);
    // Handle do substrato: o L2 real compartilha o MESMO runner (o
    // estado fica no dono; o tecido lê por referência com recibo E4).
    let l1_shared = l1_mod.shared_runner();

    // L2: tecido REAL — formação por afinidade sobre o passo físico de
    // cada tick, publicação POST_TISSUE versionada e visões para L3.
    // Config injetada de `config/default.toml [l2.*]`.
    let l2_mod = Arc::new(l2::TissueModule::new_with_config(
        l1_mod.shared_runner(),
        seed,
        l2cfg,
    ));
    let l2_handle = Arc::clone(&l2_mod);

    // L3: cognição local REAL — consome os tecidos do L2 com recibo E4,
    // prediz com evidência viva e propõe adaptação estrutural ao L2.
    // Config completa injetada de `config/default.toml [l3.*]`.
    let l3_mod = l3::L3Module::new_with_config(Arc::clone(&l2_handle), l3cfg);
    let l3_handle = Arc::new(l3_mod);
    // L4: cognição global REAL — workspace com candidatos do L3
    // (focos/predição/sinais), decisão com commit/defer e ciclo
    // decisão→outcome→learning FECHADO (Lei 5). Config injetada de
    // `config/default.toml [l4.*]`.
    let l4_mod = l4::L4Module::new_with_l3_and_config(Some(Arc::clone(&l3_handle)), l4cfg);
    let l4_handle = Arc::new(l4_mod);
    // L5: metacognição (identidade, límbico, metacontrolador).
    let l5_mod = l5::L5Module::new(
        tc::ModuleDescriptor {
            module_id: tf::id::ModuleId::new(),
            name: "l5.meta".into(),
            layer: tc::Layer::L5,
            domain: "metacognition".into(),
        },
        l5::IdentityStore::new(l5::IdentityArc {
            values: l5::identity::IdentityValues {
                growth: 0.8,
                stability: 0.6,
                exploration: 0.5,
                integrity: 0.9,
            },
            continuity: 1.0,
            step: tf::id::StepId::new(),
        }),
        l5::LimbicGovernor::new(),
        l5::MetaController::new(),
    );

    let modules: Vec<Arc<dyn rt::CognitiveModule>> = vec![
        Arc::new(l1_mod),
        l2_mod as Arc<dyn rt::CognitiveModule>,
        l3_handle.clone() as Arc<dyn rt::CognitiveModule>,
        l4_handle.clone() as Arc<dyn rt::CognitiveModule>,
        Arc::new(l5_mod),
    ];
    let mut scheduler = rt::Scheduler::new(
        modules,
        rt::StepBudget::new(cfg.runtime.max_events_per_tick),
        bus,
    );

    // 65 ticks: tempo para tecidos emergirem, migrarem, consolidarem e o
    // ciclo L3→L2 ter janelas de cadência completas (slots 31 e 61).
    let ticks = 65u64;
    println!("=== Triad_AEE: núcleo L1-L5 online ===");
    for t in 1..=ticks {
        let report = scheduler.step();
        println!(
            "[{t}/{ticks}] tick={} executados={} saltados={} eventos={} budget_excedido={} degradado={}",
            report.tick,
            report.executed,
            report.skipped,
            report.events_published,
            report.budget_exceeded,
            report.degraded
        );
    }

    // Drena eventos remanescentes no barramento.
    let drenados = scheduler.bus.try_receive(1000).len();
    println!("Eventos drenados do barramento: {drenados}");

    // ---- Auditor de eventos (orientação a eventos instrumentada) ----
    // O assinante canônico do runtime prova o fluxo: cada publicação
    // chegou a um consumidor independente sem puxar o estado.
    auditor.record();
    println!(
        "Auditor de eventos: {} contados; por fonte: {:?}; descartes de fila cheia: {}",
        auditor.total(),
        auditor.report(),
        scheduler.bus.subscriber_drops()
    );

    // ---- Pontes instrumentadas: a auditoria das fronteiras ----
    let l1_guard = l1_shared.lock().unwrap_or_else(|p| p.into_inner());
    println!("=== Pontes instrumentadas ===");
    println!(
        "L1→L2 (E4): {} leituras com valor, {} ausências, registradas no L1Ledger",
        l1_guard.ledger.consumed_value, l1_guard.ledger.consumed_absence
    );
    let l2 = l2_handle.l2();
    if let Some(s) = l2.ledger.latest() {
        println!(
            "L2 (E3): v{} — {} tecidos, {} clusters vinculados, {} soltos, cobertura {:.2}",
            s.state_version,
            s.tissue_count,
            s.assigned_members,
            s.unassigned_members,
            s.assignment_coverage.unwrap_or(0.0),
        );
    }
    let (pv, pa) = l2.ledger.published_counts();
    let (cv, ca) = l2.ledger.consumed_counts();
    println!(
        "L2Ledger: publicações valor/ausência {pv}/{pa}; consumos valor/ausência {cv}/{ca}"
    );
    let views = l2.views(&l1_guard);
    println!(
        "L2→L3: {} visões de tecido prontas para consumo (coerência/especialização/integração)",
        views.len()
    );
    println!(
        "L3→L2: {} propostas de adaptação submetidas (o gate L2 decide com histerese); atenção com {} focos reais",
        l3_handle.proposals_total(),
        l3_handle.attention_foci().foci.len()
    );
    println!(
        "Gate L2: {} pedidos L3 recebidos; {} aplicados (com clamp), {} adiados (histerese/cadência)",
        l2.gate.received,
        l2.gate.applied,
        l2.gate.deferred_band + l2.gate.deferred_interval
    );
    if let Some(rep) = l2_handle.last_report() {
        println!(
            "Último step L2: {} mudanças estruturais, {} bridges ativos, adaptações aplicadas/adiadas {}/{}",
            rep.events.len(),
            rep.bridges,
            rep.adaptations_applied,
            rep.adaptations_deferred
        );
    }
    // ---- L4 REAL: o ciclo global medido com denominador ----
    let l4_stats = l4_handle.stats();
    println!("=== Cognição global L4 (real) ===");
    println!(
        "Workspace: {} broadcasts (candidatos do L3); decisões commitidas/adiadas/expiradas {}/{}/{}",
        l4_stats.broadcasts,
        l4_stats.decisions_committed,
        l4_stats.decisions_deferred,
        l4_stats.decisions_expired,
    );
    println!(
        "Ciclo decisão→outcome→learning: fechados {} de {} abertos (taxa {:?}); expirados sem outcome: {}",
        l4_stats.envelopes_closed,
        l4_stats.envelopes_opened,
        l4_handle.closed_rate().map(|r| r.value()),
        l4_stats.envelopes_expired,
    );
    println!(
        "Causal: {:?} de confirmação (hits/total); modelo do mundo v{} ({} atualizações com dados reais)",
        l4_handle.causal_pct().map(|r| r.value()),
        l4_handle.world_version(),
        l4_stats.world_updates,
    );
    drop(l1_guard);

    println!("Núcleo encerrado sem violar as leis da casa.");
}
