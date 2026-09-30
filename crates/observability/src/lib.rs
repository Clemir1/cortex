//! # triad-observability — T-observability (transversal)
//!
//! Telemetria profunda e reporting (diretriz do dono): a cada teste e
//! validação o organismo registra a interface `var/system.log`
//! (linhas legíveis) e a telemetria estruturada `var/system.json`
//! (um objeto JSON por evento) — os herdeiros do system.log /
//! system.json do legado.
//!
//! 17.10 — TraceEngine: tracing distribuído em MEMÓRIA com
//! trace_id/span_id/parent_span (cascatas reconstruíveis — proveniência
//! causal de tick→módulo→evento) e DeltaTelemetry (performance COM
//! denominador). COMPLEMENTA o SystemJournal sem duplicá-lo.
//!
//! Regras da casa aplicadas:
//! - ausência ≠ zero: quem registra só escreve o que mediu;
//! - taxas com denominador: a razão viaja junto com o total;
//! - append por linha: múltiplos binários de teste validam em
//!   paralelo sem corromper os arquivos.

pub mod journal;
pub mod trace;

pub use journal::SystemJournal;
pub use trace::{delta, DeltaRow, SpanRecord, TraceEngine, TraceError};

/// FNV-1a 64-bit (determinístico e portátil — independente do hasher
/// da stdlib) usado para derivar trace_id/span_id estáveis.
pub(crate) fn fnv_span(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}
