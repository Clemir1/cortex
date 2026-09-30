//! DiagnÃ³stico de escala 17.6 (nÃ£o Ã© teste de validaÃ§Ã£o â€” Ã© PERFIL:
//! imprime tempos por passo para achar o gargalo @30K; mantido como
//! ferramenta de auditoria de performance).

use triad_foundation as tf;
use triad_l1_substrate::ClusterModule;
use triad_runtime::{CognitiveModule as _, TypedContext};

fn perfil(nome: &str, max_pop: usize, passos: u64) {
    let mut cfg = triad_l1_substrate::config::L1Config::default();
    cfg.morphogenesis.max_population = max_pop;
    let module = ClusterModule::new_with_config(42, 30_000, cfg);
    let mut clock = tf::LogicalClock::new();
    let mut out = Vec::new();
    for s in 1..=passos {
        clock.advance();
        let ctx = TypedContext::new(clock);
        let t0 = std::time::Instant::now();
        module.tick(&ctx, &mut out).expect("tick");
        let dt = t0.elapsed();
        println!("{nome} step {s}: {dt:?}");
        out.clear();
    }
}

/// Transiente de juventude (banda tau A7: novos tickam a cada passo
/// atÃ© amadurecer) + steady state: @30K os primeiros ~10 passos sÃ£o
/// fÃ­sica COMPLETA O(n); depois a banda fraciona (1/2, 1/5, 1/10).
#[test]
fn diagnostico_transiente_e_steady_30k() {
    perfil("t", 32_000, 26);
}
