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
use std::time::Duration;
use triad_foundation as tf;
use triad_l1_substrate as l1;
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

criterion_group!(benches, step_l1_por_carga);
criterion_main!(benches);
