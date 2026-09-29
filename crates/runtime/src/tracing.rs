//! Inicialização de tracing do organismo (idempotente).

use std::str::FromStr;
use std::sync::Once;

static INIT: Once = Once::new();

/// Inicializa o subscriber global; `false` se já estava inicializado.
pub fn tracing_init(level: &str) -> bool {
    let mut first = false;
    INIT.call_once(|| {
        let lv = tracing::Level::from_str(level).unwrap_or(tracing::Level::INFO);
        let _ = tracing_subscriber::fmt().with_max_level(lv).try_init();
        first = true;
    });
    first
}
