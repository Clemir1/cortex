//! # triad-observability — T-observability (transversal)
//!
//! Telemetria profunda e reporting (diretriz do dono): a cada teste e
//! validação o organismo registra a interface `var/system.log`
//! (linhas legíveis) e a telemetria estruturada `var/system.json`
//! (um objeto JSON por evento) — os herdeiros do system.log /
//! system.json do legado.
//!
//! Regras da casa aplicadas:
//! - ausência ≠ zero: quem registra só escreve o que mediu;
//! - taxas com denominador: a razão viaja junto com o total;
//! - append por linha: múltiplos binários de teste validam em
//!   paralelo sem corromper os arquivos.

pub mod journal;

pub use journal::SystemJournal;
