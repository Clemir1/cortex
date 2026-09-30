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
    let mut tempos_ms: Vec<f64> = Vec::new();
    for s in 1..=passos {
        clock.advance();
        let ctx = TypedContext::new(clock);
        let t0 = std::time::Instant::now();
        module.tick(&ctx, &mut out).expect("tick");
        let dt = t0.elapsed();
        println!("{nome} step {s}: {dt:?}");
        tempos_ms.push(dt.as_secs_f64() * 1000.0);
        out.clear();
    }
    // Diretriz do dono: evidência da validação no var/system.log
    // (append; falha de I/O nunca reprova o perfil).
    let media = tempos_ms.iter().sum::<f64>() / tempos_ms.len() as f64;
    let pico = tempos_ms.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if let Ok(j) = triad_observability::SystemJournal::open_workspace() {
        let _ = j.section(&format!("DIAGNÓSTICO DE ESCALA 17.6 — {nome}"));
        let _ = j.section_line(&[
            ("População máxima", max_pop.to_string()),
            ("Passos", passos.to_string()),
            ("Média ms/tick", format!("{media:.1}")),
            ("Pico ms", format!("{pico:.1}")),
        ]);
    }
}

/// Transiente de juventude (banda tau A7: novos tickam a cada passo
/// atÃ© amadurecer) + steady state: @30K os primeiros ~10 passos sÃ£o
/// fÃ­sica COMPLETA O(n); depois a banda fraciona (1/2, 1/5, 1/10).
#[test]
fn diagnostico_transiente_e_steady_30k() {
    perfil("t", 32_000, 26);
}
