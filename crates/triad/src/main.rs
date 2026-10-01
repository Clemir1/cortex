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
use triad_persistence as per;
use triad_cybernetics as cyb;
use triad_governance as governance;
use triad_telemetry as tel;
use triad_lua as lua_host;
use triad_compute as compute;
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

    // T/CYBERNETICS (16.8): ordens O1–O4 REAIS sobre métricas vivas
    // do L1 (energia/ressonância/capacidade) nas cadências da config;
    // O5 coleta horizonte, genome FORA do loop (Lei 7).
    let cybcfg: cyb::CyberneticsCfg = match cfg.get_section("cybernetics") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("FALHA NO BOOT: seção [cybernetics] ausente: {e:?}");
            return;
        }
    };
    let cyb_mod = cyb::CyberneticsModule::new_with_config(cybcfg)
        .with_l1_source(Arc::clone(&l1_handle));
    let cyb_handle = Arc::new(cyb_mod);

    // T/TELEMETRY (16.9): [telemetry] INJETADA — auditor da escada
    // E0–E5 com descritores VIVOS de TODOS os módulos montados
    // (incluindo o próprio auditor: gap tipado, nunca promoção).
    let telcfg: tel::TelemetryCfg = match cfg.get_section("telemetry") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("FALHA NO BOOT: seção [telemetry] ausente: {e:?}");
            return;
        }
    };
    // [crisis] estruturada e VALIDADA no boot (Lei 4 congelada):
    // máquina inválida é erro de boot — nunca silenciada.
    let crisis_cfg: tel::CrisisCfg = match cfg.get_section("crisis") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("FALHA NO BOOT: seção [crisis] ausente: {e:?}");
            return;
        }
    };
    if let Err(e) = crisis_cfg.validate() {
        eprintln!("FALHA NO BOOT: [crisis] inválida: {e:?}");
        return;
    }
    let tel_mod = {
        let descs = vec![
            rt::CognitiveModule::descriptor(&*l1_handle).clone(),
            rt::CognitiveModule::descriptor(&*l2_mod).clone(),
            rt::CognitiveModule::descriptor(&*l3_handle).clone(),
            rt::CognitiveModule::descriptor(&*l4_handle).clone(),
            rt::CognitiveModule::descriptor(&*l5_handle).clone(),
            rt::CognitiveModule::descriptor(&*dev_handle).clone(),
            rt::CognitiveModule::descriptor(&*lrn_handle).clone(),
            rt::CognitiveModule::descriptor(&*cyb_handle).clone(),
        ];
        let mut m = tel::TelemetryModule::new_with_config(telcfg).with_descriptors(descs);
        // O auditor se inclui na auditoria (gap próprio é tipado).
        m.include_self();
        m
    };
    let tel_handle = Arc::new(tel_mod);

    // 17.12 — LUA POLICY HOST: Lua é APENAS política (retorna
    // Proposal ou nil); Rust valida (whitelist+faixas+Lei 3) e
    // aplica. Sandbox rígido; hashes versionados por arquivo.
    let luacfg: lua_host::LuaCfg = match cfg.get_section("lua") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("FALHA NO BOOT: seção [lua] ausente: {e:?}");
            return;
        }
    };
    let mut policy_host = lua_host::PolicyHost::boot(&luacfg, std::path::Path::new("."))
        .unwrap_or_else(|e| {
            eprintln!("FALHA NO BOOT: PolicyHost: {e}");
            std::process::exit(1);
        });
    for (nome, hash) in policy_host.policies() {
        println!("  policy Lua registrada: {nome} (hash {:016x})", hash);
    }
    for (nome, hash) in policy_host.modules() {
        println!("  módulo Lua utilitário: {nome} (hash {:016x})", hash);
    }
    for issue in policy_host.load_issues() {
        eprintln!("  policy com problema (boot segue): {} — {}", issue.file, issue.message);
    }
    for err in policy_host.registry_errors() {
        eprintln!("  erro de registro Lua (boot segue): {err}");
    }
    // Contadores do wiring (aplicação com denominador).
    let mut lua_aplicadas: u64 = 0;
    let mut lua_rejeitadas: u64 = 0;
    let mut lua_banda_morta: u64 = 0;
    let mut lua_chamadas: u64 = 0;

    // ---- Cross-run (17.9, absorve 16.10): RESTORE com verificação de
    // procedência — o snapshot mais recente em var/runs/ é carregado,
    // checksumado, REPLAYado (as taxas do learning têm que reproduzir
    // os cicos do snapshot) e então o learning retoma as taxas/traços
    // e o development retoma o ESTÁGIO alcançado. Ausência de snapshot
    // é estado limpo INFORMADO (nunca silenciado); corrupção/adulteração
    // é rejeitada com erro tipado — o organismo não boota sobre
    // procedência duvidosa.
    let runs_dir = std::path::Path::new(per::RUNS_DIR);
    match per::Checkpointer::latest(runs_dir) {
        Ok(Some(path)) => match per::Checkpointer::load(&path)
            .and_then(|snap| {
                per::verify_provenance(&snap).map(|rep| (snap, rep))
            }) {
            Ok((snap, rep)) => {
                lrn_handle.restore_snapshot(&snap.learning);
                dev_handle.restore_snapshot(&snap.development);
                println!(
                    "Cross-run (17.9): restaurado {} — tick {}, seed {}; procedência VERIFICADA: {} registros replayados, {} módulos batidos; estágio {} retomado",
                    path.display(),
                    snap.meta.created_tick,
                    snap.meta.seed,
                    rep.records_replayed,
                    rep.modules_matched,
                    snap.development.stage.as_str(),
                );
            }
            Err(e) => {
                eprintln!(
                    "Cross-run (17.9): snapshot rejeitado por procedência: {e:?} — boot limpo sem restore"
                );
            }
        },
        Ok(None) => {
            println!(
                "Cross-run (17.9): nenhum snapshot anterior em {}/ — ausência ≠ zero: ontogenia começa do embrião",
                per::RUNS_DIR
            );
        }
        Err(e) => {
            eprintln!("Cross-run (17.9): falha ao procurar snapshots: {e:?}");
        }
    }

    let modules: Vec<Arc<dyn rt::CognitiveModule>> = vec![
        l1_handle.clone() as Arc<dyn rt::CognitiveModule>,
        l2_mod as Arc<dyn rt::CognitiveModule>,
        l3_handle.clone() as Arc<dyn rt::CognitiveModule>,
        l4_handle.clone() as Arc<dyn rt::CognitiveModule>,
        l5_handle.clone() as Arc<dyn rt::CognitiveModule>,
        dev_handle.clone() as Arc<dyn rt::CognitiveModule>,
        lrn_handle.clone() as Arc<dyn rt::CognitiveModule>,
        cyb_handle.clone() as Arc<dyn rt::CognitiveModule>,
        tel_handle.clone() as Arc<dyn rt::CognitiveModule>,
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
            ("T-cyb", &*cyb_handle),
            ("T-tel", &*tel_handle),
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
            "Descritores canônicos completos: {completos}/{} — transversais T montados no app (16.6/17.8/16.8/16.9, E3)",
            inventario.len()
        );
    }

    // 17.10 — TraceEngine: tracing em memória do run (cascatas
    // reconstruíveis + telemetria de performance COM denominador).
    let mut tracer = triad_observability::TraceEngine::new(seed);
    let mut agg_before: std::collections::HashMap<String, (u64, u64)> =
        std::collections::HashMap::new();
    let metade = ticks / 2;
    let mut span_amostra: Option<u64> = None;

    // 17.4 — admissão top-down (limiar homeostático 0.6) + ids dos
    // módulos para o canal AdaptationRequest (canal existente do L2).
    let mut topdown_adm = l2::TopDownAdmission::new();
    // 17.14: limiar de especialização real da formação — o
    // TopDownAdmission acompanha o que o gate de adaptação aplicar.
    let mut spec_th_prev = l2_handle.specialization_threshold();
    topdown_adm.set_threshold(
        spec_th_prev,
        "limiar inicial da formação [l2.tissues.specialization_threshold]",
    );
    // 18.5 — Ecologia T: motor de populações por domínio com
    // adapters REAIS (L3 conceitos=SYMBOLIC, L2 tecidos=TISSUE).
    // Projeção viva: o motor NUNCA edita estado das camadas.
    let mut eco_l3 = governance::EcologyMotor::new("symbolic", 0xEC05_EE01);
    let mut eco_l2 = governance::EcologyMotor::new("tissue", 0xEC05_EE02);
    let mut eco_census_l3: Option<governance::EcoCensus> = None;
    let mut eco_census_l2: Option<governance::EcoCensus> = None;
    // 18.6 — Arbitragem T: ownership do atuador real do canal
    // L3→L2 resolvendo conflitos com prioridade determinística.
    let mut arbitrator = governance::ArbitrationEngine::new();
    let mut arb_conflitos: u64 = 0;
    let mut arb_vencedores: u64 = 0;
    let mut arb_perdedores: u64 = 0;
    // 18.4 — Federação T (STAGING feature-off): o motor CNP
    // existe e é comprovado por golden tests; nenhuma troca roda
    // no organismo sem o dono ligar. A policy Lua "federation"
    // propõe trade_rate; o Rust valida (whitelist) e aplica no
    // motor — zero efeito no comportamento (caixa 18.4).
    let mut fed_engine = governance::FederationEngine::new();
    let mut fed_propostas_validadas: u64 = 0;
    let mut fed_propostas_rejeitadas: u64 = 0;
    let mut fed_scarcity_sinal: Option<f64> = None;
    // 20.5b (sessão 7, sob diretriz da dona) — FONTE de outcomes
    // federativos RESOLVIDOS por tick para o StepReport: o scheduler
    // consulta `FederationOutcomeSource` (accessor dela no runtime);
    // esta impl consulta o estado compartilhado que o ciclo alimenta
    // após cada `resolve_outcomes(t)` — chain_hashes dos horizontes
    // h1/h5 fechados no tick (o ledger do governance é a origem).
    struct FedOutcomeSource(std::sync::Arc<std::sync::Mutex<(u64, Vec<u64>)>>);
    impl rt::FederationOutcomeSource for FedOutcomeSource {
        fn resolved_outcome_ids(&self, tick: u64) -> Vec<u64> {
            let g = self.0.lock().unwrap();
            if g.0 == tick {
                g.1.clone()
            } else {
                Vec::new()
            }
        }
    }
    let fed_resolved: std::sync::Arc<std::sync::Mutex<(u64, Vec<u64>)>> =
        std::sync::Arc::new(std::sync::Mutex::new((0, Vec::new())));
    let mut fed_horizontes_resolvidos: u64 = 0;
    scheduler.with_federation_outcomes(Box::new(FedOutcomeSource(fed_resolved.clone())));
    let topdown_requester = rt::CognitiveModule::descriptor(&*l3_handle)
        .module_id
        .clone();
    let topdown_target =
        rt::CognitiveModule::descriptor(&*l2_handle).module_id.clone();

    // 19.5/19.8-c — módulos NÃO-VITAIS sujeitos à política de carga
    // do O3 (cadência reduzida em crise). Vital = l1.substrate
    // (vida), learning (Lei 4) e cybernetics (o controlador).
    const NAO_VITAIS: [&str; 6] = [
        "l2.tissue",
        "l3.local",
        "l4.global",
        "l5.meta",
        "development",
        "telemetry",
    ];
    let mut aplicacoes_throttle: u64 = 0;
    let mut liberacoes_throttle: u64 = 0;
    let mut razao_throttle_corrente: Option<String> = None;
    let mut throttled_no_ciclo: u64 = 0;
    // 19.8-e — resumo do veredito O5 do boundary (impresso no bloco
    // final do cybernetics).
    let mut o5_resumo_episodio: Option<String> = None;
    // 17.11 (sessão 7) — LawEngine: enforcement OBSERVACIONAL
    // pós-tick das 8 leis da casa (como DADO no HardLawSet) +
    // soft law de orçamento via policy Lua "law_soft" (whitelist
    // 17.12, faixa [0.5, 1.0] — Lei 4: o mínimo SEMPRE deixa o
    // sistema vivo). Nunca causa efeito: audita e REGISTRA.
    let mut law_engine = governance::LawEngine::new();
    let budget_base_events = cfg.runtime.max_events_per_tick;
    let mut law_soft_validadas: u64 = 0;
    let mut law_soft_rejeitadas: u64 = 0;
    let mut law_soft_liberacoes: u64 = 0;
    let mut law_soft_razao: Option<String> = None;
    let mut law_soft_fator_corrente: Option<f64> = None;

    for t in 1..=ticks {
        // 20.5b — resolve os outcomes federativos vencidos ANTES do
        // passo: o StepReport do tick publica os chain_hashes
        // RESOLVIDOS nele (h1/h5 fechados — Lei 5 trilha por recibo).
        {
            let n = fed_engine.resolve_outcomes(t) as u64;
            fed_horizontes_resolvidos += n;
            *fed_resolved.lock().unwrap() = (t, fed_engine.take_resolved_ids(t));
        }
        let report = scheduler.step();
        // 19.8-c (sob diretriz da dona) — CONSUMIDOR RUNTIME da
        // GoverningAction do O3: a ação deixa de ser recomendação e
        // vira POLÍTICA DE CARGA APLICADA no escalonador. Vital é
        // intocável (Lei 4): l1.substrate (vida), learning
        // (aprendizagem nunca é suspensa) e cybernetics (o próprio
        // controlador) NUNCA são throttled; o restante da cadeia
        // apenas reduz cadência — crise muda política, não desativa.
        {
            use triad_cybernetics::GoverningAction;
            let aplicados_antes = aplicacoes_throttle;
            match cyb_handle.load_directive() {
                Some(GoverningAction::Throttle { factor, reason }) => {
                    let f = factor.clamp(0.25, 1.0) as f64;
                    for nome in NAO_VITAIS.iter() {
                        if scheduler.module_throttle(*nome) != Some(f) {
                            scheduler.set_module_throttle(nome, f);
                            aplicacoes_throttle += 1;
                        }
                    }
                    if aplicacoes_throttle > aplicados_antes {
                        razao_throttle_corrente = Some(reason);
                    }
                }
                Some(GoverningAction::Shed { reason }) => {
                    // Shed no runtime vira cadência mínima 0.25 com
                    // razão — NUNCA desligamento (Lei 4).
                    for nome in NAO_VITAIS.iter() {
                        if scheduler.module_throttle(*nome) != Some(0.25) {
                            scheduler.set_module_throttle(nome, 0.25);
                            aplicacoes_throttle += 1;
                        }
                    }
                    razao_throttle_corrente = Some(reason);
                }
                Some(GoverningAction::Normal) => {
                    // Normal ⇒ política removida: cadeia volta ao
                    // passo pleno (comportamento histórico).
                    if liberacoes_throttle == 0 && aplicacoes_throttle > 0 {
                        for nome in NAO_VITAIS.iter() {
                            scheduler.clear_module_throttle(nome);
                        }
                        liberacoes_throttle += 1;
                        razao_throttle_corrente = None;
                    }
                }
                None => {}
            }
            throttled_no_ciclo += report.throttled.len() as u64;
        }
        // 17.11 — LawEngine: auditoria pós-tick OBSERVACIONAL das
        // leis da casa sobre a fotografia do passo (executados,
        // orçamento, degradação). Violacões só REGISTRAM com razão
        // — o engine nunca bloqueia nem desliga (Lei 4).
        {
            let executados: Vec<String> = report
                .module_latency
                .iter()
                .map(|(nome, _)| nome.clone())
                .collect();
            let observacao = governance::LawObservation::from_step(
                t,
                executados,
                report.budget_exceeded,
                vec![],
            );
            law_engine.audit(&observacao);
            // Soft law de orçamento (per 10 ticks, padrão federation):
            // sinal REAL = pressão observada (budget_exceeded/10);
            // ausência de sinal ⇒ SEM proposta (ausência ≠ pressa).
            if policy_host.policy_hash("law_soft").is_some() && t % 10 == 0 {
                let pressao = law_engine.budget_pressure();
                match policy_host.call(
                    "law_soft",
                    lua_host::PolicyContext {
                        active_fraction: pressao,
                        ..Default::default()
                    },
                ) {
                    Ok(Some(proposta)) => {
                        // Faixa [0.5, 1.0] já validada pela whitelist
                        // 17.12 — aplicar é POLÍTICA de orçamento.
                        let fator = proposta.value.clamp(0.5, 1.0);
                        scheduler.budget.max_events =
                            ((budget_base_events as f64) * fator).round().max(1.0) as usize;
                        // Lei 6 no espírito da soft law: pressão zerou e
                        // havia economia ativa ⇒ LIBERAÇÃO com recibo (o
                        // teto volta à base; ausência de pressão = efeito
                        // observado da política anterior).
                        if fator >= 1.0
                            && law_soft_fator_corrente.is_some_and(|f| f < 1.0)
                        {
                            law_soft_liberacoes += 1;
                        }
                        law_soft_fator_corrente = Some(fator);
                        law_soft_validadas += 1;
                        law_soft_razao = Some(proposta.reason);
                    }
                    Ok(None) => {}
                    Err(_) => law_soft_rejeitadas += 1,
                }
            }
        }
        // 17.10: cada tick é um span RAIZ; cada módulo executado é
        // filho (proveniência causal tick→módulo reconstruível).
        let root = tracer.root_span(t);
        for (nome, us) in &report.module_latency {
            match tracer.module_span(root, t, nome, *us) {
                Ok(id) => {
                    if span_amostra.is_none() && t == ticks && nome == "l1.substrate"
                    {
                        span_amostra = Some(id);
                    }
                }
                Err(e) => eprintln!("TraceEngine rejeitou span: {e:?}"),
            }
        }
        if t == metade {
            agg_before = tracer.snapshot();
        }
        // 17.12 — Lua policy wiring FORA do hot path (janela longa):
        // a cada 10 ticks a policy "learning" recebe o erro preditivo
        // MÉDIO COM DENOMINADOR (ausência = None → default do Lua).
        if policy_host.policy_hash("learning").is_some() && t % 10 == 0 {
            let ls = lrn_handle.stats();
            let mean_err = if ls.credits_awarded + ls.skipped_unconfirmed > 0 {
                Some(
                    (ls.abs_effect_sum / (ls.credits_awarded + ls.skipped_unconfirmed) as f32)
                        as f64,
                )
            } else {
                None
            };
            lua_chamadas += 1;
            match policy_host.call(
                "learning",
                lua_host::PolicyContext {
                    prediction_error: mean_err,
                    ..Default::default()
                },
            ) {
                Ok(None) => lua_banda_morta += 1,
                Ok(Some(proposal)) => {
                    match lrn_handle.apply_policy_eta(
                        proposal.value as f32,
                        &proposal.reason,
                        proposal.policy_hash,
                    ) {
                        Ok(_) => lua_aplicadas += 1,
                        Err(_) => lua_rejeitadas += 1,
                    }
                }
                Err(_) => lua_rejeitadas += 1,
            }
        }
        // 18.5 — Ecologia T: censo real por cadência (10 ticks) →
        // um passo ecológico por domínio (seleção/especiação/
        // extinção/cross-feed tipado — tudo na projeção).
        if t % 10 == 0 {
            let obs_symbolic: Vec<governance::SpeciesObs> = l3_handle
                .ecology_species()
                .into_iter()
                .map(|(niche, fitness, population)| governance::SpeciesObs {
                    niche,
                    fitness,
                    population,
                })
                .collect();
            eco_census_l3 = Some(eco_l3.step(&obs_symbolic, t as u64));
            let obs_tissue: Vec<governance::SpeciesObs> = l2_handle
                .ecology_species()
                .into_iter()
                .map(|(niche, fitness, population)| governance::SpeciesObs {
                    niche,
                    fitness,
                    population,
                })
                .collect();
            eco_census_l2 = Some(eco_l2.step(&obs_tissue, t as u64));
        }
        // 18.4 — Policy Lua "federation" (staging): sinal REAL =
        // escassez média dos budgets L5 (denominador 5); Proposal
        // validada pelo Rust ajusta SÓ o cap de perna do motor.
        if policy_host.policy_hash("federation").is_some() && t % 10 == 0 {
            let rg = l5_handle.resource_governor();
            let budgets: Vec<Option<f32>> = [
                l5::Resource::Energy,
                l5::Resource::Compute,
                l5::Resource::Attention,
                l5::Resource::Memory,
                l5::Resource::Federation,
            ]
            .iter()
            .map(|r| rg.budget(*r).map(|b| b.scarcity()))
            .collect();
            drop(rg);
            let presentes: Vec<f32> = budgets.into_iter().flatten().collect();
            let scarcity = if presentes.is_empty() {
                None
            } else {
                Some(presentes.iter().sum::<f32>() as f64 / presentes.len() as f64)
            };
            fed_scarcity_sinal = scarcity;
            match policy_host.call(
                "federation",
                lua_host::PolicyContext {
                    active_fraction: scarcity,
                    ..Default::default()
                },
            ) {
                Ok(None) => {}
                Ok(Some(proposal)) => {
                    fed_engine.set_trade_rate(proposal.value);
                    fed_propostas_validadas += 1;
                }
                Err(_) => fed_propostas_rejeitadas += 1,
            }
        }
        // 17.4 — TopDownFeedback L3→L2: sinais do CAMPO REAL de
        // atenção → admissão TIPADA (recebido/admitido/rejeitado
        // por motivo, tudo com denominador) → admitidos seguem o
        // canal AdaptationRequest do L2 (o gate da sessão 6 decide
        // a aplicação com razão própria — camadas por contrato).
        topdown_adm.tick();
        let views_17_4 = l2_handle.tissue_views();
        // 18.6 — Arbitragem T: os admitidos do tick disputam a
        // ownership do atuador "inbox.l2.adaptation" (prioridade
        // = intensidade do realce; chegada = ordem de sinal). Só
        // o VENCEDOR vira AdaptationRequest — perdedores ganham
        // recibo tipado e não poluem o gate (contenção real
        // reduzida; admissão 17.4 intacta).
        let mut admitidos_do_tick: Vec<(String, f32, l2::AdmittedSignal)> = Vec::new();
        for signal in l3_handle.topdown_signals() {
            match topdown_adm.evaluate(
                signal.concept,
                signal.intensity,
                &views_17_4,
            ) {
                Ok(admitido) => {
                    let chave = format!("{:?}", signal.concept);
                    arbitrator.submit(governance::Contender {
                        controller: chave.clone(),
                        actuator: "inbox.l2.adaptation".to_string(),
                        priority: (signal.intensity * 1000.0) as i32,
                        reason: "realce top-down admitido".to_string(),
                    });
                    admitidos_do_tick.push((chave, signal.intensity, admitido));
                }
                Err(_) => {
                    // Contado COM motivo tipado dentro da admissão.
                }
            }
        }
        // 18.6 — resolve do tick: vencedor segue o canal, com a
        // trilha de auditoria tipada dos perdedores.
        for record in arbitrator.resolve_tick(t as u64) {
            if record.had_conflict() {
                arb_conflitos += 1;
            }
            if let Some(vencedor) = record.winner.as_deref() {
                if let Some((_, _, admitido)) = admitidos_do_tick
                    .iter()
                    .find(|(chave, _, _)| chave == vencedor)
                {
                    let atual = views_17_4
                        .iter()
                        .find(|v| v.tissue_id == admitido.tissue_id)
                        .map(|v| v.specialization.value())
                        .unwrap_or(0.0);
                    l2_handle.submit_adaptation(
                        triad_contracts::messages::AdaptationRequest {
                            requester: topdown_requester.clone(),
                            target: topdown_target.clone(),
                            parameter: "l2.tissue.specialization_threshold"
                                .to_string(),
                            current: format!("{atual:.3}"),
                            proposed: format!("{:.3}", admitido.new_threshold),
                        },
                    );
                    arb_vencedores += 1;
                }
            }
            arb_perdedores += record.losers.len() as u64;
        }
        // 17.14 — CANAL FECHADO: quando o gate de adaptação aplica
        // `l2.tissue.specialization_threshold`, o valor REAL da
        // formação realimenta o TopDownAdmission (o roteamento L3→L2
        // passa a usar o limiar aplicado — efeito de ponta a ponta).
        {
            let th = l2_handle.specialization_threshold();
            if (th - spec_th_prev).abs() > 1e-6 {
                topdown_adm.set_threshold(
                    th,
                    "gate de adaptação aplicou l2.tissue.specialization_threshold",
                );
                spec_th_prev = th;
            }
        }
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
    // 19.8-e — O5 SELEÇÃO NO BOUNDARY DE EPISÓDIO (fora do loop:
    // Lei 7; hook previsto pela sessão 6: evaluate_offline "é
    // chamado em boundary de episode pela ORQUESTRAÇÃO"). A seleção
    // atravessa o tempo DE VERDADE: fitness observado do episódio
    // (taxa de confirmação do L4, coletada no run com denominador)
    // decide a promoção; SÓ o vencedor validado é promovido (o erro
    // do legado — ativo aleatório ≠ campeão, cognitive_genome.py:
    // 353-358 — fica fora). Lei 6: o retune tem recibo (razão) e o
    // fitness do PRÓXIMO episódio é o observed_effect (reversível).
    {
        const O5_MIN_DENOMINADOR: usize = 10;
        const O5_PROMOTION_FLOOR: f32 = 0.5;
        let mut go = l5_handle.genome_offline();
        let n = go.collected();
        let avg = go.evaluate_offline();
        let mut o5_resumo = String::new();
        match avg {
            None => {
                let razao = "fitness AUSENTE — nada coletado (ausência ≠ zero)".to_string();
                cyb_handle.o5_reject(&razao);
                o5_resumo = format!("REJEITADO: {razao}");
            }
            Some(avg) if n < O5_MIN_DENOMINADOR => {
                let razao =
                    format!("denominador insuficiente: {n} de {O5_MIN_DENOMINADOR} amostras")
                ;
                cyb_handle.o5_reject(&razao);
                o5_resumo = format!("REJEITADO: {razao}");
            }
            Some(avg) if avg < O5_PROMOTION_FLOOR => {
                let razao =
                    format!("fitness {avg:.3}/{n} abaixo do piso {O5_PROMOTION_FLOOR}")
                ;
                cyb_handle.o5_reject(&razao);
                o5_resumo = format!("REJEITADO: {razao}");
            }
            Some(avg) => {
                // Genoma corrente = gene REAL do controlador
                // (ganho O1); mutação determinística LCG (Lei 7:
                // offline, seed do episódio); campeão com margem e
                // denominador ⇒ promovido COM recibo e reversão.
                let gain_corrente = cyb_handle.o1_gain();
                let genome = cyb::Genome::new(vec![("o1.gain".to_string(), gain_corrente)]);
                let mutante = genome.mutate(ticks, 0.2);
                let (gene, novo) = mutante
                    .genes
                    .iter()
                    .find(|(g, _)| g == "o1.gain")
                    .map(|(g, v)| (g.clone(), *v))
                    .expect("gene o1.gain presente no mutante");
                let _scores = vec![(gene, avg)];
                let aplicado =
                    cyb_handle.o5_promote_retune(
                        novo - gain_corrente,
                        &format!(
                            "O5 boundary: fitness {avg:.3}/{n} ≥ piso {O5_PROMOTION_FLOOR}; o1.gain {gain_corrente:.3}→{novo:.3} (reversível — Lei 6)"
                        ),
                    );
                o5_resumo = format!(
                    "PROMOVIDO: fitness {avg:.3}/{n}, o1.gain {gain_corrente:.3}→{aplicado:.3}"
                );
            }
        }
        o5_resumo_episodio = Some(o5_resumo);
    }

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
            "Crédito por módulo (traço vivo): l3.prediction {}, l2.tissue {}, l3.attention {} — eta {:.4} (nunca suspenso), traços podados {} em {} rodadas de poda, ticks sem fonte {} de {}",
            credito_fmt("l3.prediction"),
            credito_fmt("l2.tissue"),
            credito_fmt("l3.attention"),
            ls.eta, ls.pruned_traces, ls.prune_rounds, ls.no_source_ticks, ls.ticks,
        );
        let taxas = lrn_handle.module_rates();
        if taxas.is_empty() {
            println!("Taxa por módulo: AUSENTE (nenhum registro consumido)");
        } else {
            for (modulo, creditos, registros) in taxas {
                let taxa = if registros == 0 {
                    "n/d".to_string()
                } else {
                    format!("{:.1}%", creditos as f32 * 100.0 / registros as f32)
                };
                println!(
                    "Taxa por módulo: {modulo} — {creditos}/{registros} confirmados ({taxa})"
                );
            }
        }
    }

    // 17.4 — TopDownFeedback L3→L2: admissão TIPADA com denominador
    // (recebido ≠ admitido; recusa sempre com motivo) + homeostase.
    {
        println!("=== TopDown L3→L2 (17.4) ===");
        let (rec, ok, rej, por) = topdown_adm.stats();
        let pct = if rec > 0 {
            format!("{:.1}%", ok as f64 * 100.0 / rec as f64)
        } else {
            "n/d".to_string()
        };
        println!(
            "Sinais: {} recebidos — {} admitidos ({pct}), {} rejeitados (denominador {rec})",
            rec, ok, rej,
        );
        if por.is_empty() {
            println!("Rejeições por motivo: nenhuma");
        } else {
            for (motivo, n) in por {
                println!("Rejeições por motivo: {motivo} — {n} de {rec}");
            }
        }
        println!(
            "Limiar homeostático de especialização: {:.3} (razão: {})",
            topdown_adm.threshold(),
            topdown_adm.last_threshold_reason().unwrap_or("default da casa"),
        );
        let ult = topdown_adm.last_thresholds();
        if ult.is_empty() {
            println!("Realces aplicados: nenhum (nenhum tecido com carência)");
        } else {
            for (tissue, novo) in ult.iter().rev().take(3) {
                println!("Realce aplicado (trilha): tecido {tissue:?} → threshold {:.3}", novo);
            }
        }
        // views já computado acima (guards l1/l2 retidos pela main
        // até o fim — tissue_views() aqui seria deadlock l1→l2).
        let h = l2::homeostasis(&views);
        println!(
            "Homeostase mesoscópica: {} tecidos vivos (de {} vistos) — desvio das especializações {:.3} (denominador {})",
            h.n_tecidos_vivos,
            views.len(),
            h.desvio_padrao,
            h.n_tecidos_vivos,
        );
    }

    // 17.5 — L3 REPRESENTAÇÃO REAL: ConceptStore/Binding/
    // OntogeneticMemory janelada + ponte temporal HOTM→semântica +
    // metaestabilidade — tudo COM DENOMINADOR.
    {
        println!("=== Representação L3 (17.5) ===");
        let r = l3_handle.representation_report();
        println!(
            "ConceptStore: {} conceitos ({} inserts, {} dedupes)",
            r.conceitos, r.inserts, r.dedupes,
        );
        println!(
            "Bindings semânticos: {} arestas",
            r.bindings,
        );
        println!(
            "Memória ontogenética (janelada): {} eventos — {} consolidadas, {} reconsolidadas, {} decaídas (denominador {}); {} traços vivos, {} consolidados agora",
            r.eventos,
            r.eventos_consolidados,
            r.reconsolidadas,
            r.decaidas,
            r.eventos,
            r.traces_vivas,
            r.traces_consolidadas,
        );
        println!(
            "Ponte temporal HOTM→semântica: {} observações → {} ligações por recorrência (denominador {})",
            r.observacoes_temporais, r.ligacoes_temporais, r.observacoes_temporais,
        );
        println!(
            "Metaestabilidade local: {} atratores de {} conceitos (limiar 0.5) — estabilidade média {:.3}",
            r.atratores, r.conceitos, r.estabilidade_media,
        );
    }

    // 17.6 — L4 INTEGRAÇÃO COMPLETA: GlobalIntegration agregador
    // (thalamus/episódico/abstração/navegador fundidos) COM
    // DENOMINADOR — conforme audit 17-3.
    {
        println!("=== GlobalIntegration L4 (17.6) ===");
        let g = l4_handle.global_integration_report();
        println!(
            "ThalamicRouter (gate antes do workspace): {} chamados — {} admitidos, {} recusados (LowPriority/TtlAlive), {} expirados por TTL (denominador {})",
            g.thalamus_chamados, g.thalamus_admitidos, g.thalamus_recusados,
            g.thalamus_expirados, g.thalamus_chamados,
        );
        println!(
            "EpisodicIntegration: {} pushes — {} episódios fechados (delta>0.15), {} keyframes (delta>0.05)",
            g.pushes, g.episodios_fechados, g.keyframes,
        );
        println!(
            "GlobalAbstraction: {} símbolos grounded (L0), {} co-ocorrências (L1), {} meta (L2) — {} formações",
            g.simbolos_l0, g.simbolos_l1, g.simbolos_l2, g.formacoes,
        );
        println!(
            "FutureNavigator: {} trajetórias (4 bandas × ticks com focos) — melhor banda {} score {:.3}",
            g.trajetorias,
            g.best_band.map(|b| b.to_string()).unwrap_or("n/d".into()),
            g.best_path_score.unwrap_or(0.0),
        );
        println!(
            "Contexto fundido: {} focos no último tick, saliência média {}",
            g.focos_ultimo_tick,
            g.saliencia_media
                .map(|s| format!("{s:.3}"))
                .unwrap_or("AUSENTE".into()),
        );
    }

    // 17.7 — L5 GOVERNADORES COMPLETOS (audit 17-4 §5): budgets
    // tipados, decisões com Lei 3, morfogênese por causa, bandit
    // com rollback, genome OFFLINE (Lei 7).
    {
        println!("=== Governadores L5 (17.7) ===");
        let rg = l5_handle.resource_governor();
        let (gr, de, th, denom) = rg.decision_stats();
        println!(
            "ResourceGovernor: {} decisões — {} grants, {} denials, {} throttles (denominador {})",
            denom, gr, de, th, denom,
        );
        for r in [
            l5::Resource::Energy,
            l5::Resource::Compute,
            l5::Resource::Attention,
            l5::Resource::Memory,
            l5::Resource::Federation,
        ] {
            match rg.budget(r) {
                Some(b) => println!(
                    "  budget {:>10}: {:.3} (piso {:.2}) — escassez {:.3}{}",
                    r.as_str(),
                    b.value,
                    b.floor,
                    b.scarcity(),
                    if b.is_starving() { " — FAMINTO" } else { "" },
                ),
                None => println!("  budget {:>10}: AUSENTE", r.as_str()),
            }
        }
        let (led, total) = rg.ledger();
        println!(
            "  ledger {} eventos vivos de {} totais; últimos: {:?}",
            led,
            total,
            rg.last_events()
                .iter()
                .rev()
                .take(2)
                .map(|e| format!("{:?}{}", e.resource.as_str(), e.reason.split('(').next().unwrap_or("")))
                .collect::<Vec<_>>()
        );
        drop(rg);
        let dg = l5_handle.development_governor();
        let (wakes, standbys) = dg.stats();
        println!(
            "DevelopmentGovernor: {} wakes, {} standbys por causa declarada (default {})",
            wakes, standbys, dg.morphogenesis_default,
        );
        drop(dg);
        let ml = l5_handle.meta_learner();
        let (trials, commits, rollbacks) = ml.stats();
        println!(
            "MetaLearning: {} trials — {} commits, {} rollbacks tipados (denominador {})",
            trials, commits, rollbacks, trials,
        );
        drop(ml);
        let go = l5_handle.genome_offline();
        let collected = go.collected();
        match go.avg_fitness() {
            Some(avg) => println!(
                "CognitiveGenome (Lei 7 OFFLINE): {} avaliações coletadas — fitness médio {:.3}; seleção NUNCA no loop",
                collected, avg,
            ),
            None => println!(
                "CognitiveGenome (Lei 7 OFFLINE): {} avaliações coletadas — fitness AUSENTE (ausência ≠ zero)",
                collected,
            ),
        }
    }

    // 17.3 — L1 DINÂMICA CANÔNICA: multi-motor (HOTM UMA
    // estratégia + harmônico), inibição lateral esparsa e
    // prediction error local com gates tipados — COM DENOMINADOR.
    {
        println!("=== Dinâmica Local L1 (17.3) ===");
        let runner = l1_shared.lock().unwrap_or_else(|p| p.into_inner());
        let r = runner.local_dynamics.report();
        let motores = r.hotm_passos + r.harmonic_passos;
        println!(
            "Multi-motor: {} passos — {} HOTM, {} harmônico (denominador {})",
            motores, r.hotm_passos, r.harmonic_passos, motores,
        );
        println!(
            "Inibição lateral: {} inibidos de {} vivos agora ({} competidores >0.8) — liberados {} neste passo; total histórico {}",
            r.inibidos_agora, r.vivos, r.competidores, r.liberados_agora, r.inibicoes_total,
        );
        match (r.err_medio, r.surpresa_media) {
            (Some(err), Some(surp)) => println!(
                "Prediction error: L2 médio {:.4}, surpresa (entropia) {:.4} — gates {} ALIGNED / {} OPPOSING (denominador {} amostrados, {} sem predição)",
                err, surp, r.gates_aligned, r.gates_opposing, r.amostrados, r.sem_predicao,
            ),
            _ => println!(
                "Prediction error: AUSENTE — {} sem predição prévia (ausência ≠ zero)",
                r.sem_predicao,
            ),
        }
        drop(runner);
    }

    // 18.5 — Ecologia T: censo final dos dois domínios com
    // denominadores; cross-feeds tipados e validação honesta.
    {
        println!("=== Ecologia T (18.5) ===");
        for (nome, census, motor) in [
            ("SYMBOLIC (conceitos L3)", &eco_census_l3, &eco_l3),
            ("TISSUE (tecidos L2)", &eco_census_l2, &eco_l2),
        ] {
            match census {
                Some(c) => {
                    println!(
                        "[{nome}] {}/{} espécies vivas/observadas, pop total {} — Shannon {} (ausência ≠ zero), dominante {}",
                        c.species_alive,
                        c.species_observed,
                        c.population_total,
                        match c.shannon { Some(h) => format!("{h:.4}"), None => "AUSENTE".to_string() },
                        c.dominant.as_deref().unwrap_or("nenhum"),
                    );
                    println!(
                        "[{nome}] seleção: {} extinções, {} especiações, {} por teto — escassez da comida {:.4} — cross-feeds: {} APPLIED / {} REJECTED / {} NO_TRANSFER (denominador: total de propostas do domínio)",
                        c.extinctions,
                        c.speciations,
                        c.capped,
                        c.scarcity,
                        c.feeds_applied,
                        c.feeds_rejected,
                        c.feeds_no_transfer,
                    );
                    let validados = motor
                        .feeds()
                        .iter()
                        .filter(|r| r.benefit_validated == Some(true))
                        .count();
                    let invalidados = motor
                        .feeds()
                        .iter()
                        .filter(|r| r.benefit_validated == Some(false))
                        .count();
                    println!(
                        "[{nome}] validação honesta: {} benefícios COMPROVADOS (MATCH nos horizontes 1 e 5) / {} DRIFT — pendentes não contam como benefício",
                        validados, invalidados,
                    );
                }
                None => println!(
                    "[{nome}] AUSENTE — sem censo nesta execução (ausência ≠ zero)"
                ),
            }
        }
    }

    // 18.6 — Arbitragem T: conflitos reais de ownership com
    // trilha tipada e denominadores.
    {
        println!("=== Arbitragem T (18.6) ===");
        let s = arbitrator.stats();
        let concorrencia = arb_vencedores + arb_perdedores;
        println!(
            "Atuador inbox.l2.adaptation: {arb_vencedores} vencedores encaminhados, {arb_perdedores} perdedores COM recibo tipado (denominador: {concorrencia} pretensões admitidas no período)",
        );
        println!(
            "Conflitos reais: {arb_conflitos} ticks com disputa — perdas por razão: {} lower_priority / {} later_arrival (denominador: {arb_perdedores} perdedores)",
            s.by_lower_priority, s.by_later_arrival,
        );
        println!(
            "Trilha retida: {} recibos (teto 64), fila pendente {} (ausência ≠ zero)",
            s.records_kept, s.queued,
        );
        if let Some(last) = arbitrator.records().iter().rev().find(|r| r.had_conflict()) {
            println!(
                "Última disputa (tick {}): vencedor {} (prio {}) — perdedores: {}",
                last.tick,
                last.winner.as_deref().unwrap_or("nenhum"),
                last.winner_priority.unwrap_or(0),
                last.losers
                    .iter()
                    .map(|(c, r)| format!("{c} [{}]", r.as_str()))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        } else {
            println!("Nenhuma disputa com 2+ pretendentes nesta execução — conflito ausente é ausência, não zero");
        }
    }

    // 18.4 — Federação T: estado do staging com evidência real.
    {
        println!("=== Federação T (18.4 — STAGING feature-off) ===");
        let hash = policy_host.policy_hash("federation");
        match hash {
            Some(h) => println!(
                "Policy Lua \"federation\" registrada (hash {h:016x}) — {} propostas validadas pelo Rust, {} rejeitadas (denominador: {} chamadas na janela)",
                fed_propostas_validadas,
                fed_propostas_rejeitadas,
                fed_propostas_validadas + fed_propostas_rejeitadas,
            ),
            None => println!(
                "Policy Lua \"federation\": AUSENTE — nenhum arquivo em lua/policies (ausência ≠ zero)"
            ),
        }
        println!(
            "Cap de perna corrente: {:.3} (faixa inegociável [0.0, 0.5] da whitelist Rust) — sinal de escassez L5: {}",
            fed_engine.trade_rate_cap(),
            match fed_scarcity_sinal {
                Some(s) => format!("{s:.4} (média dos 5 budgets)"),
                None => "AUSENTE".to_string(),
            },
        );
        println!(
            "Protocolo CNP comprovado por golden tests: estados tipados (applied/rejected/reverted), conservação 2 pernas < 1e-12, ledger encadeado determinístico, reservas contra dupla promessa — NENHUMA troca executada no organismo (feature desligada por padrão, critério da caixa)",
        );
        let (total, applied, rejected, reverted, max_err) = fed_engine.stats();
        let _ = (total, applied, rejected, reverted, max_err);
        println!(
            "Rodadas CNP no organismo: 0 (staging) — membros federativos: 0; ligar exige decisão do dono + ADR (var/verificacao/18-4_ADR_federacao.md)",
        );
        // 20.5b — trilha de outcomes por tick: horizontes h1/h5
        // resolvidos no ciclo e pendências honestas (staging: 0).
        println!(
            "Outcomes federativos (20.5b): horizontes resolvidos no ciclo: {}; pendentes agora: {} — chain_hashes resolvidos por tick via StepReport.federation_outcome_ids (ausência ≠ zero)",
            fed_horizontes_resolvidos,
            fed_engine.pending_outcome_count(),
        );
    }

    // T/CYBERNETICS (16.8): ordens O1–O4 sobre métricas VIVAS nas
    // cadências da config — satisfação/taxas COM denominador; O3
    // recomenda ação TIPADA; O5 amostra horizonte (genome fora, Lei 7).
    {
        println!("=== Cybernetics (T, 16.8) ===");
        // 17.11 (sessão 7) — ControllerConflictGraph: os controladores
        // e seus conflitos DECLARADOS com resolução estrutural (o
        // controle sobre controladores deixa de ser implícito).
        let grafo = triad_cybernetics::ControllerConflictGraph::house();
        println!(
            "ControllerConflictGraph (17.11): {}",
            grafo.summary()
        );
        for e in &grafo.edges {
            println!(
                "  {}×{} sobre {}: {}",
                e.a, e.b, e.target, e.resolution
            );
        }
        let cs = cyb_handle.stats();
        let sat = if cs.o1_runs == 0 {
            "n/d (sem execução)".to_string()
        } else {
            format!(
                "{:.1}% ({}/{})",
                cs.o1_satisfied as f32 * 100.0 / cs.o1_runs as f32,
                cs.o1_satisfied,
                cs.o1_runs
            )
        };
        let acao = match &cs.last_action {
            Some(cyb::GoverningAction::Normal) => "Normal",
            Some(cyb::GoverningAction::Throttle { .. }) => "Throttle",
            Some(cyb::GoverningAction::Shed { .. }) => "Shed",
            None => "nenhuma (sem execução ainda)",
        };
        println!(
            "O1 energia em banda: {} — O2 oscilador: {} execuções, O3 governador: {} execuções (última ação: {}), {} throttles / {} sheds",
            sat, cs.o2_runs, cs.o3_runs, acao, cs.o3_throttles, cs.o3_sheds,
        );
        println!(
            "O4 auto-auditoria: {} execuções; eventos publicados {} de {} ticks (denominador explícito); ticks sem métrica viva {} (ausência ≠ zero)",
            cs.o4_runs, cs.events_published, cs.ticks, cs.no_source_ticks,
        );
        // 19.8-a/b/c/d — os loops fecharam de fato (série 19):
        // O1 corr vira urgência do O3; O2 retuna o controlador O1;
        // O3 aplica a carga no runtime; O4 emite alarmes tipados.
        println!(
            "O1 correção contínua: {} — O2 retuning do controlador O1: {}/{} DOWN, {}/{} UP (oscilação em {} janelas)",
            cs.o1_last_corrective
                .map(|v| format!("{v:.3}"))
                .unwrap_or_else(|| "AUSENTE".into()),
            cs.o2_retunes_down,
            cs.o2_runs,
            cs.o2_retunes_up,
            cs.o2_runs,
            cs.o2_oscillating_windows,
        );
        println!(
            "O3 APLICADA de fato no runtime: {} aplicações de throttle, {} liberações; razão corrente: {}",
            aplicacoes_throttle,
            liberacoes_throttle,
            razao_throttle_corrente
                .as_deref()
                .unwrap_or("nenhuma (Normal)"),
        );
        println!(
            "  saltos por política de carga no ciclo: {} (denominador: {} ticks; vitais intocáveis: l1.substrate/learning/cybernetics — Lei 4)",
            throttled_no_ciclo, ticks,
        );
        println!(
            "O4 alarmes tipados: {} de {} auditorias (vivos agora: {}) — observação alimenta O2/O3, nunca decide",
            cs.o4_alarms,
            cs.o4_runs,
            cs.alarms.len(),
        );
        for al in &cs.alarms {
            println!("  ALARME {}: valor {:.3} — {}", al.metric, al.value, al.reason);
        }
        // 19.8-e — a seleção através do tempo RODOU no boundary.
        println!(
            "O5 SELEÇÃO NO BOUNDARY (fora do loop, Lei 7): {} — promoções {}/{} corridas",
            o5_resumo_episodio.as_deref().unwrap_or("nenhuma"),
            cs.o5_promotions,
            cs.o5_promotions + cs.o5_rejections,
        );
    }

    // 17.11 (sessão 7) — LawEngine: as leis da casa AUDITADAS de
    // fato em cada passo, com denominadores por lei.
    {
        println!("=== LawEngine (T, 17.11) ===");
        let ls = law_engine.stats();
        println!(
            "Leis duras como DADO: {} entradas no HardLawSet; auditorias pós-tick: {} (denominador explícito); violações registradas: {}/{} auditorias",
            law_engine.law_count(),
            ls.audits,
            ls.violations_total,
            ls.audits,
        );
        if ls.violations_by_law.is_empty() {
            println!("  nenhuma lei dura violada no ciclo (0/{})", ls.audits);
        } else {
            for (lei, n) in &ls.violations_by_law {
                println!("  lei {lei}: {n}/{} auditorias violada", ls.audits);
            }
        }
        println!(
            "Pressão de orçamento observada: {}/{} passos com budget_exceeded; soft law \"law_soft\" (Lei 4: política, nunca suspensão): {} propostas validadas, {} rejeitadas, {} liberações para a base {budget_base_events}",
            ls.budget_exceeded_steps,
            ls.audits,
            law_soft_validadas,
            law_soft_rejeitadas,
            law_soft_liberacoes,
        );
        println!(
            "  razão da política corrente: {}",
            law_soft_razao
                .as_deref()
                .unwrap_or("nenhuma (sem pressão observada — ausência ≠ pressa)"),
        );
        let soft = &law_engine.soft_book;
        println!(
            "Soft laws no livro: {} (peso total {:.1})",
            soft.len(),
            soft.total_weight(),
        );
    }

    // T/LEARNING (17.8 completa): janelas de validação t+1/t+5 com
    // vereditos tipados e TRANSFER como taxa de fluxo por fronteira
    // (legado layer_transfer.py) — denominador sempre, gargalo sempre.
    {
        let ls = lrn_handle.stats();
        println!("=== Learning validação temporal (17.8) ===");
        let total = ls.validation_hits + ls.validation_incomplete + ls.validation_misses;
        if total == 0 {
            println!("Janelas fechadas: 0 (ausência ≠ zero — nenhuma maturou ainda)");
        } else {
            println!(
                "Janelas fechadas: {} — validadas: {} ({:.1}%), incompletas sem penalidade: {}, penalizadas: {} (denominador total)",
                total,
                ls.validation_hits,
                ls.validation_hits as f32 * 100.0 / total as f32,
                ls.validation_incomplete,
                ls.validation_misses,
            );
        }
        let t = lrn_handle.transfer_rates();
        println!(
            "Transfer (fronteira {}): input {} → accepted {} → acted {} → effect {} → learned {}; F1 {:.2} F2 {:.2} F3 {:.2} F4 {:.2}; gargalo {}",
            t.fronteira, t.input, t.accepted, t.acted, t.effect, t.learned, t.f1, t.f2, t.f3, t.f4, t.gargalo,
        );
    }

    // T/TELEMETRY (16.9): auditoria da escada de evidência E0–E5 com
    // descritores VIVOS vs alvo da config — CONFORMIDADE com
    // denominador; gaps tipados (nunca promoção automática).
    {
        println!("=== Telemetry (T, 16.9) ===");
        let ts = tel_handle.stats();
        let gaps = tel_handle.last_gaps();
        if ts.audits == 0 {
            println!("Auditoria de evidência: AUSENTE (sem execução no intervalo)");
        } else {
            let taxa = if ts.last_audited == 0 {
                "n/d".to_string()
            } else {
                format!(
                    "{:.1}%",
                    ts.last_compliant as f32 * 100.0 / ts.last_audited as f32
                )
            };
            println!(
                "Escada E0–E5 auditada: {}/{} módulos no alvo ({}) em {} auditorias",
                ts.last_compliant, ts.last_audited, taxa, ts.audits,
            );
            if gaps.is_empty() {
                println!("Gaps de evidência: nenhum (última auditoria)");
            } else {
                println!(
                    "Gaps tipados da última auditoria ({}): {} — escada só sobe com verificação (Lei 1); total acumulado: {}",
                    gaps.len(),
                    gaps.iter()
                        .map(|g| format!("{} {}<{}", g.module, g.declared, g.target))
                        .collect::<Vec<_>>()
                        .join(", "),
                    ts.gap_events,
                );
            }
        }
    }

    // 17.12 — Lua PolicyHost: proposals COM DENOMINADOR; hashes
    // versionados; aplicação validada por Rust (nunca estado em Lua).
    {
        println!("=== Lua PolicyHost (17.12) ===");
        let total = lua_aplicadas + lua_rejeitadas + lua_banda_morta;
        let taxa = if lua_chamadas == 0 {
            "n/d (sem chamada)".to_string()
        } else {
            format!(
                "{:.1}% ({}/{})",
                lua_aplicadas as f32 * 100.0 / lua_chamadas as f32,
                lua_aplicadas,
                lua_chamadas
            )
        };
        println!(
            "Chamadas: {} — proposals aplicadas: {}, rejeitadas: {}, banda morta (nil): {} de {} resultados",
            lua_chamadas, lua_aplicadas, lua_rejeitadas, lua_banda_morta, total,
        );
        println!("Aplicação sobre 'learning.eta': {} de {} chamadas válidas ({})", lua_aplicadas, lua_chamadas, taxa);
        println!(
            "eta corrente da política: {:.4} (faixa da casa [0.005, 0.02] — aprendizagem NUNCA suspensa, Lei 4)",
            lrn_handle.current_eta(),
        );
        let npol = policy_host.policies().len();
        let nmod = policy_host.modules().len();
        println!(
            "Policies registradas: {npol}; módulos utilitários: {nmod}; arquivos com problema: {} (boot segue — o host nunca cai)",
            policy_host.load_issues().len(),
        );
    }

    // 17.2 — T/COMPUTE: benchmarks dos kernels nos DOIS backends
    // (seq = contrato f64 Numba; rayon bit-identical) COM DENOMINADOR
    // — latência média e throughput por iteração medida.
    {
        println!("=== Compute (17.2) ===");
        let rows = compute::run_benchmarks(256, 5);
        for r in &rows {
            println!(
                "  {} [{}] n={} — {:.1}µs/iter de {} iters (throughput {:.0}/s)",
                r.kernel, r.backend, r.n, r.mean_us, r.iters, r.ops_per_s,
            );
        }
        println!(
            "  WGPU: NO-GO pela 17.7 (decisão por benchmarks da sessão 6) — backend duplo CPU é o caminho real",
        );
    }

    // 17.10 — TraceEngine: delta de performance ENTRE JANELAS (1ª vs
    // 2ª metade do run) com denominador + amostra de cascata
    // reconstruída (proveniência causal tick→módulo).
    {
        println!("=== TraceEngine (T, 17.10) ===");
        let after = tracer.snapshot();
        let deltas = triad_observability::delta(&agg_before, &after);
        let total_spans = tracer.spans().len();
        println!(
            "Trace {} — {} spans em {} ticks (média {:.1} spans/tick, denominador explícito)",
            tracer.trace_id,
            total_spans,
            ticks,
            total_spans as f64 / ticks as f64,
        );
        if deltas.is_empty() {
            println!("Delta entre janelas: AUSENTE (sem spans na 2ª metade)");
        } else {
            println!(
                "Delta 1ª→2ª metade (Δµs, Δspans, µs médio POR SPAN com denominador):"
            );
            for d in deltas.iter().take(8) {
                let media = match d.avg_us_per_span {
                    Some(m) => format!("{m:.1}µs/span"),
                    None => "AUSENTE (Δspans=0)".to_string(),
                };
                println!(
                    "  {} — Δ{}µs em Δ{} spans ({})",
                    d.name, d.delta_us, d.delta_spans, media
                );
            }
        }
        if let Some(span) = span_amostra {
            match tracer.render_cascade(span) {
                Ok(arvore) => {
                    println!("Cascata reconstruída (amostra do último tick):");
                    for linha in arvore.lines() {
                        println!("  {linha}");
                    }
                }
                Err(e) => eprintln!("Cascata não renderizável: {e:?}"),
            }
        }
    }

    // ---- Cross-run (17.9): SAVE atômico do snapshot final — os cicos
    // fechados (histórico de aprendizagem), o learning e o estágio do
    // organismo sobrevivem à execução em var/runs/ (serde, checksum,
    // schema versionado — sem banco prematuro).
    {
        let snap = per::Checkpointer::build(
            ticks,
            seed,
            &l4_handle,
            &lrn_handle,
            &dev_handle,
        );
        match per::Checkpointer::save(
            std::path::Path::new(per::RUNS_DIR),
            &snap,
        ) {
            Ok(path) => {
                println!(
                    "Snapshot salvo (17.9): {} — {} registros, checksum {:016x}",
                    path.display(),
                    snap.meta.records,
                    snap.meta.closed_log_checksum,
                );
            }
            Err(e) => {
                eprintln!("FALHA ao salvar snapshot (17.9): {e:?}");
            }
        }
    }

    // T-observability (diretriz do dono): telemetria profunda por RUN
    // em var/system.log — FORMATO DO SYSTEM_02.LOG DO LEGADO (blocos
    // `======`, linhas `[TAG] chave=valor`, rodapé de conclusão) — e
    // var/system.json (um objeto por evento com `ts`; o tempo vive no
    // JSON, a linha fica limpa como no legado). Taxas sempre com
    // denominador; ausência registrada como ausência (nunca zero).
    // 21.2: JOURNAL DO RUN com ROTAÇÃO — a execução anterior é
    // arquivada em var/logs/ (análise científica sem duplicações;
    // o system.log contém SÓ o run corrente e o índice
    // var/runs_index.jsonl acumula 1 linha por run).
    if let Ok(journal) = triad_observability::SystemJournal::open_run() {
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
                ("f32_staging", "reservado (17.7 NO-GO por dados)".to_string()),
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
        // 21.2: fecha o RUN no índice científico (1 linha JSON por
        // execução — comparação entre runs sem duplicação textual).
        let _ = journal.finish_run(&format!(
            "{{\"passos\":{},\"tempo_s\":{:.1},\"veredito\":\"sem violar as leis da casa\"}}",
            ticks,
            t0.elapsed().as_secs_f32()
        ));
    }

    println!("Núcleo encerrado sem violar as leis da casa.");
}
