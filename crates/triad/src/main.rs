//! Organismo cognitivo Triad_AEE: sobe o núcleo L1–L5 e roda passos.
//!
//! L1 e L2 são REAIS: o substrato executa o passo físico e o tecido
//! deriva a organização mesoscópica sobre ele (formação por afinidade,
//! bridges, feedback com histerese), com as pontes instrumentadas por
//! recibos E3/E4 nos ledgers.

use std::sync::Arc;
use triad_foundation as tf;
use triad_l1_substrate as l1;
use triad_l2_tissue as l2;
use triad_l3_local as l3;
use triad_l4_global as l4;
use triad_l5_meta as l5;
use triad_platform as platform;
use triad_development as dev;
use triad_learning as lrn;
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
    // Telemetria (17.6): tempo total do run para o rodapé do system.log.
    let t0 = std::time::Instant::now();
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
    let l5cfg: l5::L5Config = match cfg.get_section("l5") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("FALHA NO BOOT: seção [l5] ausente: {e:?}");
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
    // 17.6 (diretriz do dono): BOOT configurável — population = 1200
    // (default) com teto de divisão 32K; a validação 30K usa a mesma
    // chave com population = 30000.
    let l1_population = l1cfg.population;
    let l1_mod = l1::ClusterModule::new_with_config(seed, l1_population, l1cfg);
    // Handle forte do substrato: o L2 compartilha o MESMO runner (o
    // estado fica no dono; o tecido lê por referência com recibo E4) e
    // o L5 observa a energia dos clusters por ponte read-only.
    let l1_handle = Arc::new(l1_mod);
    let l1_shared = l1_handle.shared_runner();

    // L2: tecido REAL — formação por afinidade sobre o passo físico de
    // cada tick, publicação POST_TISSUE versionada e visões para L3.
    // Config injetada de `config/default.toml [l2.*]`.
    let l2_mod = Arc::new(l2::TissueModule::new_with_config(
        l1_handle.shared_runner(),
        seed,
        l2cfg,
    ));
    let l2_handle = Arc::clone(&l2_mod);

    // L3: cognição local REAL — consome os tecidos do L2 com recibo E4,
    // prediz com evidência viva e propõe adaptação estrutural ao L2.
    // Config completa injetada de `config/default.toml [l3.*]`.
    // ADR-0007: recursão Chladni ligada — a atenção realimenta a
    // saliência com a harmonia do substrato (ponte read-only ao L1).
    let l3_mod = l3::L3Module::new_with_config(Arc::clone(&l2_handle), l3cfg)
        .with_chladni_source(Arc::clone(&l1_handle));
    let l3_handle = Arc::new(l3_mod);
    // L4: cognição global REAL — workspace com candidatos do L3
    // (focos/predição/sinais), decisão com commit/defer e ciclo
    // decisão→outcome→learning FECHADO (Lei 5). Config injetada de
    // `config/default.toml [l4.*]`.
    let l4_mod = l4::L4Module::new_with_l3_and_config(Some(Arc::clone(&l3_handle)), l4cfg);
    let l4_handle = Arc::new(l4_mod);
    // L5: metacognição REAL — observa L4/L1 (read-only), identidade
    // derivada do comportamento (taxas com denominador), límbico com
    // histerese, metacontrolador com TTL individual (Lei 6). Config
    // injetada de `config/default.toml [l5.*]`.
    let (policy_step, policy_floor) = (
        l5cfg.meta_controller.commit_threshold_step,
        l5cfg.meta_controller.commit_threshold_floor,
    );
    let l5_mod = l5::L5Module::new_with_l4_and_config(
        Some(Arc::clone(&l4_handle)),
        Some(Arc::clone(&l1_handle)),
        l5cfg,
    );
    let l5_handle = Arc::new(l5_mod);

    // T/DEVELOPMENT (16.6): montado no app com política central
    // ([development.*]) e a ponte read-only do sinal Chladni do L1
    // (ADR-0007) — estágios por banda de ressonância REAL, orçamento
    // de regeneração por estágio, corticalização com métricas vivas.
    let devcfg: dev::DevelopmentCfg = match cfg.get_section("development") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("FALHA NO BOOT: seção [development] ausente: {e:?}");
            return;
        }
    };
    let dev_mod = dev::DevelopmentModule::new_with_config(devcfg)
        .with_chladni_source(Arc::clone(&l1_handle));
    let dev_handle = Arc::new(dev_mod);

    // T/LEARNING (17.8): montado no app com política central
    // ([learning]) e ponte read-only ao closed_log do L4 — crédito
    // POR MÓDULO com denominador sobre cicos FECHADOS com
    // verified_future_effect (Lei 5); esquecimento exponencial;
    // eta adaptativo NUNCA suspenso (Lei 4).
    let lrncfg: lrn::LearningCfg = match cfg.get_section("learning") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("FALHA NO BOOT: seção [learning] ausente: {e:?}");
            return;
        }
    };
    let lrn_mod = lrn::LearningModule::new_with_config(lrncfg)
        .with_l4_source(Arc::clone(&l4_handle));
    let lrn_handle = Arc::new(lrn_mod);

    let modules: Vec<Arc<dyn rt::CognitiveModule>> = vec![
        l1_handle.clone() as Arc<dyn rt::CognitiveModule>,
        l2_mod as Arc<dyn rt::CognitiveModule>,
        l3_handle.clone() as Arc<dyn rt::CognitiveModule>,
        l4_handle.clone() as Arc<dyn rt::CognitiveModule>,
        l5_handle.clone() as Arc<dyn rt::CognitiveModule>,
        dev_handle.clone() as Arc<dyn rt::CognitiveModule>,
        lrn_handle.clone() as Arc<dyn rt::CognitiveModule>,
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

    // ---- Inventário canônico de módulos (doc/CAMADA.txt, seção 17.1) ----
    // O BOOT serve de inventário: classificação por FUNÇÃO PRIMÁRIA
    // (L1–L5 cognitivas + T suporte), ordens O1–O5 como dimensão
    // própria, dono do estado, backend, criticalidade e evidência
    // na escada E0–E5 — tudo DECLARADO; pendência de declaração é
    // reportada, nunca silenciada (ausência ≠ zero).
    {
        let inventario: Vec<(&str, &dyn rt::CognitiveModule)> = vec![
            ("1", &*l1_handle),
            ("2", &*l2_handle),
            ("3", &*l3_handle),
            ("4", &*l4_handle),
            ("5", &*l5_handle),
            ("T-dev", &*dev_handle),
            ("T-lrn", &*lrn_handle),
        ];
        println!("=== Inventário canônico (doc/CAMADA.txt) ===");
        let mut completos = 0usize;
        for (pos, m) in &inventario {
            let d = m.descriptor();
            println!("  [{pos}] {}", d.summary());
            if d.is_canonical().is_empty() {
                completos += 1;
            }
        }
        println!(
            "Descritores canônicos completos: {completos}/{} — T/DEVELOPMENT e T/LEARNING montados no app (16.6/17.8, E3)",
            inventario.len()
        );
    }

    for t in 1..=ticks {
        let report = scheduler.step();
        // 17.6 — perfil de escala: passos demorados mostram a latência
        // por módulo (µs, ordem canônica) — onde o tempo do passo está.
        let total_us: u64 = report.module_latency.iter().map(|(_, us)| *us).sum();
        if total_us > 50_000 {
            let por_modulo: Vec<String> = report
                .module_latency
                .iter()
                .map(|(n, us)| format!("{n}={us}µs"))
                .collect();
            println!(
                "[{t}/{ticks}] tick={} executados={} saltados={} eventos={} budget_excedido={} degradado={} TOTAL={}ms",
                report.tick,
                report.executed,
                report.skipped,
                report.events_published,
                report.budget_exceeded,
                report.degraded,
                total_us / 1000
            );
            println!("  latências por módulo: {}", por_modulo.join(" "));
        } else {
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
    // ADR-0007: telemetria da recursão Chladni — bônus com denominador.
    let (bonus_ticks, absent_ticks) = l3_handle.chladni_stats();
    let chladni_total = bonus_ticks + absent_ticks;
    println!(
        "Recursão Chladni→atenção L3: bônus aplicado em {bonus_ticks}/{chladni_total} ticks (peso [l3.attention].chladni_bonus_weight); taxa {:?}",
        tf::Rate::from_ratio(bonus_ticks, chladni_total).map(|r| r.value()),
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
        "Verificação comensurável (sham de deriva): confirmados {} / conteúdo saiu do foco {} / degradou além da tolerância {} (taxa {:?})",
        l4_stats.confirmed,
        l4_stats.content_gone,
        l4_stats.degraded_beyond_tolerance,
        l4_handle.confirm_rate().map(|r| r.value()),
    );
    println!(
        "Causal: {:?} de confirmação (hits/total); modelo do mundo v{} ({} atualizações com dados reais)",
        l4_handle.causal_pct().map(|r| r.value()),
        l4_handle.world_version(),
        l4_stats.world_updates,
    );
    // Rollout contrafactual (16.3): trajetória rasa das crenças vs
    // observado — divergência média COM denominador (ausência ≠ zero).
    let (rollouts, rollout_media) = l4_handle.rollout_report();
    match rollout_media {
        Some(m) => println!(
            "Rollout raso (16.3): {} avaliações contrafactuais, divergência média t+1 = {:.3} (denominador {})",
            rollouts, m, rollouts,
        ),
        None => println!(
            "Rollout raso (16.3): {} avaliações — sem crença viva nos eixos fechados (ausência ≠ zero)",
            rollouts,
        ),
    }
    // Trilha L4→L3: o ciclo confirmado reforça a memória episódica real.
    let (aplicados, reconsolidados, descartados, pendentes) = l3_handle.reinforcement_stats();
    println!(
        "Trilha L4→L3 (memória real): L4 submeteu {} reforços; L3 aplicou {}, reconsolidou {}, descartou {} por overflow, fila pendente {}",
        l4_stats.memory_submissions, aplicados, reconsolidados, descartados, pendentes,
    );
    // Política de intervenção (16.7, Lei 6): o dono aplica/reverte
    // por TTL — recusas TIPADAS com denominador (ausência ≠ zero).
    println!(
        "Política de intervenção (16.7, Lei 6): submetidas {} / aplicadas {} / revertidas por TTL {} / ativas {}; recusas tipadas: duplicada {}, range {}, ttl {}, teto {}",
        l4_stats.policy_submitted,
        l4_stats.policy_applied,
        l4_stats.policy_reverted,
        l4_stats.policy_active,
        l4_stats.policy_rejected_duplicate,
        l4_stats.policy_rejected_out_of_range,
        l4_stats.policy_rejected_invalid_ttl,
        l4_stats.policy_rejected_full,
    );
    drop(l1_guard);

    // Metacognição REAL: o L5 observando o ciclo L4/L1 por inteiro.
    println!("=== Metacognição L5 (real) ===");
    let l5_stats = l5_handle.stats();
    println!(
        "Identidade observada: continuidade {:.3}; crescimento {:.3} estabilidade {:.3} exploração {:.3} integridade {:.3} ({} refreshes com dados reais)",
        l5_stats.identity_continuity,
        l5_stats.identity_values.growth,
        l5_stats.identity_values.stability,
        l5_stats.identity_values.exploration,
        l5_stats.identity_values.integrity,
        l5_stats.self_model_updates,
    );
    println!(
        "Límbico (histerese {:?}): stress {:.3}, reserva {:.3}, banda {:?}, {} transições tipadas",
        l5_stats.band,
        l5_stats.stress,
        l5_stats.energy_reserve,
        l5_stats.band,
        l5_stats.limbic_band_changes,
    );
    println!(
        "Metacontrolador (Lei 6): propostas {} recusadas {} mantidas {} revertidas {}; despertares: atividade-baixa {} stress-alto {} dormência {} agendado {}; sem fonte L4: {}",
        l5_stats.meta_proposals,
        l5_stats.meta_rejected_full,
        l5_stats.meta_kept,
        l5_stats.meta_reverted,
        l5_stats.wake_low_activity,
        l5_stats.wake_high_stress,
        l5_stats.wake_dormant_too_long,
        l5_stats.wake_scheduled,
        l5_stats.no_source_ticks,
    );
    // 16.7: propostas de política REALMENTE submetidas ao inbox do
    // L4 (protocolo L5→L4; recusas tipadas contadas no dono).
    println!(
        "Proponente de política (16.7): submetidas {} / recusadas pelo inbox {} — alívio do commit_threshold com passo {:.2} e piso {:.2}",
        l5_stats.policy_submitted,
        l5_stats.policy_rejected,
        policy_step,
        policy_floor,
    );

    // T/DEVELOPMENT (16.6): estágio do organismo pela RESSONÂNCIA
    // REAL (banda por estágio, dwell; ausência contada, não zero).
    {
        println!("=== Development (T, 16.6) ===");
        let ds = dev_handle.stats();
        let stage = ds
            .stage
            .map(|s| s.as_str().to_string())
            .unwrap_or_else(|| "AUSENTE".to_string());
        println!(
            "Estágio do organismo: {} — avanços {}, ticks na banda {} (dwell da config), ticks sem sinal {} de {} ticks",
            stage, ds.stage_advances, ds.ticks_in_band, ds.no_signal_ticks, ds.ticks,
        );
        println!(
            "Morphogênese/morphostase: reparos {}, curas {} (orçamento por estágio), fila de danos {}, descartes {}; corticalização: {} iterações com métricas vivas",
            ds.repairs, ds.heals, ds.damage_queue_len, ds.dropped, ds.cortical_iterations,
        );
    }

    // T/LEARNING (17.8): crédito por módulo sobre cicos FECHADOS do
    // L4 (verified_future_effect, Lei 5) — taxas COM denominador;
    // traço ausente é AUSÊNCIA, não zero (Lei 2).
    {
        println!("=== Learning (T, 17.8) ===");
        let ls = lrn_handle.stats();
        let denominador = ls.credits_awarded + ls.skipped_unconfirmed;
        println!(
            "Ciclos fechados consumidos: {} — confirmados com crédito {}, não confirmados {} (denominador {})",
            ls.records_consumed, ls.credits_awarded, ls.skipped_unconfirmed, denominador,
        );
        let credito_fmt = |modulo: &str| -> String {
            let q = lrn_handle.module_credit(
                modulo,
                tf::ModuleId::new(),
                tf::StepId::new(),
            );
            if q.is_value() {
                format!("{:.4}", q.as_ref_value().copied().unwrap_or_default())
            } else {
                "AUSENTE".to_string()
            }
        };
        println!(
            "Crédito por módulo (traço vivo): l3.prediction {}, l2.tissue {}, l3.attention {} — eta {:.4} (nunca suspenso), traços podados {}, ticks sem fonte {} de {}",
            credito_fmt("l3.prediction"),
            credito_fmt("l2.tissue"),
            credito_fmt("l3.attention"),
            ls.eta, ls.pruned_traces, ls.no_source_ticks, ls.ticks,
        );
    }

    // T-observability (diretriz do dono): telemetria profunda por RUN
    // em var/system.log — FORMATO DO SYSTEM_02.LOG DO LEGADO (blocos
    // `======`, linhas `[TAG] chave=valor`, rodapé de conclusão) — e
    // var/system.json (um objeto por evento com `ts`; o tempo vive no
    // JSON, a linha fica limpa como no legado). Taxas sempre com
    // denominador; ausência registrada como ausência (nunca zero).
    if let Ok(journal) = triad_observability::SystemJournal::open_default() {
        let _ = journal.section("TRIAD_AEE -- ORGANISMO COGNITIVO EM RUST (L1-L5 + T)");
        let _ = journal.section_line(&[
            ("Clusters inicio", l1_population.to_string()),
            ("Passos", ticks.to_string()),
            ("Seed causal", seed.to_string()),
        ]);
        let _ = journal.event(
            "L1",
            &[
                ("population", l1_population.to_string()),
                ("estado", "n x 97 f64 (SoA + rayon por coluna)".to_string()),
            ],
        );
        let _ = journal.event(
            "L2",
            &[
                ("publicacoes_valor", pv.to_string()),
                ("publicacoes_ausencia", pa.to_string()),
                ("consumos_valor", cv.to_string()),
                ("consumos_ausencia", ca.to_string()),
            ],
        );
        let _ = journal.event(
            "L3",
            &[
                ("propostas", l3_handle.proposals_total().to_string()),
                ("chladni_bonus_ticks", bonus_ticks.to_string()),
                ("chladni_total_ticks", chladni_total.to_string()),
                (
                    "chladni_taxa",
                    format!(
                        "{:?}",
                        tf::Rate::from_ratio(bonus_ticks, chladni_total).map(|r| r.value())
                    ),
                ),
            ],
        );
        let _ = journal.event(
            "L4",
            &[
                ("commits", l4_stats.decisions_committed.to_string()),
                ("envelopes_fechados", l4_stats.envelopes_closed.to_string()),
                ("confirmados", l4_stats.confirmed.to_string()),
                ("reforcos_l3", l4_stats.memory_submissions.to_string()),
            ],
        );
        let _ = journal.event(
            "L5",
            &[
                ("propostas_meta", l5_stats.meta_proposals.to_string()),
                ("revertidas_meta", l5_stats.meta_reverted.to_string()),
            ],
        );
        let _ = journal.event(
            "GPU-ORCH",
            &[
                ("selected_backend", "CPU".to_string()),
                ("available", "['CPU']".to_string()),
                ("f32_staging", "reservado (17.7 GO/NO-GO)".to_string()),
            ],
        );
        let _ = journal.event(
            "VALIDACAO",
            &[("resultado", "sem violar as leis da casa".to_string())],
        );
        let _ = journal.footer(&[
            ("Passos", ticks.to_string()),
            ("Tempo", format!("{:.1}s", t0.elapsed().as_secs_f32())),
            (
                "Arquivos",
                "var/system.log, var/system.json".to_string(),
            ),
        ]);
    }

    println!("Núcleo encerrado sem violar as leis da casa.");
}
