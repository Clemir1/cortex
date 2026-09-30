//! 17.1 — BENCHMARK BASELINE do passo L1 (tick e2e) nas cargas do
//! dono: boot 1.200, teto 30K. Check-list seção 17; hardware de
//! referência do dono (desktop com RX580).
//!
//! THRESHOLDS — DECLARADOS ANTES DA PRIMEIRA MEDIÇÃO (não se move o
//! golde depois; a violação do BASELINE é ESPERADA nas cargas altas
//! sem SoA/paralelismo — as fases 17.2/17.3 existem para cumprir o
//! que faltar; a meta FINAL da 17.6 é o valor de "operacao"):
//!
//!   carga       baseline aceitável   meta operação (17.6)
//!   1.200       10 ms/tick           10 ms/tick  (ciclo cognitivo ~100 Hz)
//!   5.000       40 ms/tick           20 ms/tick
//!   30.000      250 ms/tick           50 ms/tick  (teto do dono utilizável)
//!
//! Baseline salvo em `var/bench/` (não versionado); os números
//! medidos são citados no CHECKLIST_IMPLEMENTACAO.txt (17.1).
//!
//! A determinismo: o tick é o MESMO do organismo (seed fixa 42, config
//! default de código — sem injetar default.toml; o objetivo é a
//! CURVA de custo por carga, não o estado final).

use criterion::{criterion_group, criterion_main, Criterion};
use rand::SeedableRng;
use serde::Deserialize;
use std::time::Duration;
use triad_foundation as tf;
use triad_foundation::id::ClusterId;
use triad_l1_substrate as l1;
use triad_l1_substrate::soa::SampleSoA;
use triad_runtime::{CognitiveModule as _, TypedContext};

const CARGAS: [usize; 3] = [1_200, 5_000, 30_000];

fn step_l1_por_carga(c: &mut Criterion) {
    for pop in CARGAS {
        let mut group = c.benchmark_group(format!("step_l1/pop={pop}"));
        group.warm_up_time(Duration::from_secs(1));
        group.measurement_time(Duration::from_secs(8));
        // Setup fora da medição: organismo + relógio próprios do bench.
        let module = l1::ClusterModule::new(42, pop);
        let mut clock = tf::LogicalClock::new();
        let mut out = Vec::new();
        group.bench_function("tick_e2e", |b| {
            b.iter(|| {
                clock.advance();
                let ctx = TypedContext::new(clock);
                module.tick(&ctx, &mut out).expect("tick l1 no bench");
                out.clear();
            })
        });
        group.finish();
    }
}

/// 17.3 — kernels colunares da amostra em 3 caminhos: serial f64
/// (canônico), rayon por coluna (bit-idêntico), f32 (candidato de
/// custo — diretriz f32/f64 dinâmica do dono). Amostra GRANDE
/// (n=4096) para medir o crossover onde o paralelismo compensa.
fn kernel_dim_sums_17_3(c: &mut Criterion) {
    let n = 4096usize;
    let mut group = c.benchmark_group("kernel_dim_sums/n=4096");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(6));
    let mut rng = rand::rngs::SmallRng::seed_from_u64(21);
    let clusters: Vec<l1::ClusterBio> =
        (0..n).map(|_| l1::ClusterBio::new(ClusterId::new(), 0, &mut rng)).collect();
    let sample: Vec<usize> = (0..n).collect();
    let soa = SampleSoA::pack(&clusters, &sample);
    let soa32 = soa.pack_f32();
    group.bench_function("serial_f64", |b| b.iter(|| soa.dim_sums()));
    group.bench_function("rayon_f64", |b| b.iter(|| soa.dim_sums_par()));
    group.bench_function("serial_f32", |b| b.iter(|| soa32.dim_sums()));
    group.bench_function("rayon_f32", |b| b.iter(|| soa32.dim_sums_par()));
    group.finish();
}

/// 17.6 — ISOLAMENTO DO GARGALO: o app @30K mediu ~5,2 s/tick com a
/// config do default.toml, mas o bench `new()` (config default de
/// CÓDIGO) mede 3,67 ms/tick. Bissecção por seção do L1Config: qual
/// sub-config do TOML explode o custo do tick @30K.
fn step_l1_config_toml(c: &mut Criterion) {
    let pop = 30_000usize;
    let raw = std::fs::read_to_string("../../config/default.toml").expect("config/default.toml");
    let v: toml::Value = toml::from_str(&raw).expect("toml válido");
    let l1cfg = l1::config::L1Config::deserialize(v["l1"].clone()).expect("seção [l1]");
    let mut group = c.benchmark_group(format!("step_l1_config_toml/pop={pop}"));
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(6));
    // (a) TOML completo — reproduz o custo do app?
    let module = l1::ClusterModule::new_with_config(42, pop, l1cfg.clone());
    let mut clock = tf::LogicalClock::new();
    let mut out = Vec::new();
    group.bench_function("toml_full", |b| {
        b.iter(|| {
            clock.advance();
            let ctx = TypedContext::new(clock);
            module.tick(&ctx, &mut out).expect("tick l1 (toml full)");
            out.clear();
        })
    });
    // (b) TOML com MORPHOGENESIS do código (default histórico).
    let mut b_cfg = l1cfg.clone();
    b_cfg.morphogenesis = l1::config::MorphogenesisCfg::default();
    let module_b = l1::ClusterModule::new_with_config(42, pop, b_cfg);
    let mut clock_b = tf::LogicalClock::new();
    group.bench_function("toml_morpho_codigo", |b| {
        b.iter(|| {
            clock_b.advance();
            let ctx = TypedContext::new(clock_b);
            module_b.tick(&ctx, &mut out).expect("tick l1 (morpho código)");
            out.clear();
        })
    });
    // (c) TOML com ENERGY do código (default histórico).
    let mut c_cfg = l1cfg.clone();
    c_cfg.energy = l1::config::EnergyCfg::default();
    let module_c = l1::ClusterModule::new_with_config(42, pop, c_cfg);
    let mut clock_c = tf::LogicalClock::new();
    group.bench_function("toml_energy_codigo", |b| {
        b.iter(|| {
            clock_c.advance();
            let ctx = TypedContext::new(clock_c);
            module_c.tick(&ctx, &mut out).expect("tick l1 (energy código)");
            out.clear();
        })
    });
    // Bissecção por CAMPO do morpho: config de CÓDIGO + UM campo do
    // TOML por variante — a lenta aponta o campo culpado.
    let base = l1::config::L1Config::default();
    let variantes: Vec<(&str, l1::config::L1Config)> = vec![
        ("so_maxpop_32000", {
            let mut c = base.clone();
            c.morphogenesis.max_population = 32_000;
            c
        }),
        ("so_division_085", {
            let mut c = base.clone();
            c.morphogenesis.division_threshold = 0.85;
            c
        }),
        ("so_maxdiv_1", {
            let mut c = base.clone();
            c.morphogenesis.max_divisions_per_step = 1;
            c
        }),
    ];
    for (nome, cfg_v) in variantes {
        let module_v = l1::ClusterModule::new_with_config(42, pop, cfg_v);
        let mut clock_v = tf::LogicalClock::new();
        group.bench_function(nome, |b| {
            b.iter(|| {
                clock_v.advance();
                let ctx = TypedContext::new(clock_v);
                module_v.tick(&ctx, &mut out).expect("tick l1 (variante)");
                out.clear();
            })
        });
    }
    group.finish();
}

criterion_group!(benches, step_l1_por_carga, kernel_dim_sums_17_3, step_l1_config_toml);
criterion_main!(benches);
