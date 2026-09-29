//! triad-chladni — motor de ressonância Chladni do neocortex consolidado.
//!
//! Origem: legado Python `core/chladni_frequencies.py`, usado APENAS como
//! referência de mecânica (classificação completa em AUDITORIA.md, seção
//! "Legado Python"). A árvore nova tem ZERO arquivos Python — este crate é
//! a reescrita em Rust.
//!
//! Harmonia Cluster/HOTM/Chladni: o L1 já tem clusters (ClusterBio) e
//! HOTM/reservoir; este motor converte energia→frequência→padrão de onda
//! estacionária e mede ressonância entre padrões. Consumidores: atenção L3
//! (bônus ponderado por `attention_weight`), memória L3 (similaridade por
//! ressonância), development (banda cognitiva por estágio). Biblioteca
//! pura: não publica eventos; os consumidores publicam via EventBus e
//! qualificam via TypedContext.
//!
//! Leis da casa aplicadas (DIFERENTE do legado):
//! - entrada inválida (NaN/Inf/fora de domínio) = `TriadError::Invalid`
//!   explícito — o legado substituía por 0.0 em silêncio; ausência ≠ zero;
//! - taxas de cache SEMPRE com denominador (`tf::Rate::from_ratio`);
//! - todo padrão carrega hashes de conteúdo e evidência (descritor
//!   versionado), rastreável até o consumidor.

pub mod bands;
pub mod observation;
pub mod pattern;
pub mod system;
pub mod table;

pub use bands::CognitiveBand;
pub use observation::Observation;
pub use pattern::{Pattern, PatternFeatures};
pub use system::{ChladniFrequencySystem, HistoryEntry, StateMapping, Statistics};
pub use table::{PirtEntry, PlateType};

use serde::Deserialize;

// --- Constantes default da seção [chladni] ----------------------------------
// Política vive em config/default.toml; estes são os defaults congelados.
// Os do legado config.py não estavam expostos na extração — escolhas
// documentadas aqui, A/A onde o valor era visível no código.

/// Chladni ligado por default (legado: CHLADNI_ENABLED default True).
pub const ENABLED: bool = true;
/// Grade N×N dos padrões (legado: CHLADNI_DEFAULT_GRID_SIZE; mínimo 2).
pub const DEFAULT_GRID_SIZE: usize = 32;
/// Teto do cache de padrões (FIFO); 0 = sem limite.
pub const PATTERN_CACHE_MAX: usize = 64;
/// Teto do cache de ressonância (LRU); 0 = sem limite.
pub const RESONANCE_CACHE_MAX: usize = 128;
/// Últimos N pedidos guardados no histórico.
pub const HISTORY_MAX: usize = 50;
/// Versão do descritor de padrão (muda ⇒ hashes mudam).
pub const DESCRIPTOR_VERSION: u32 = 1;
/// Versão do algoritmo de hash de evidência.
pub const EVIDENCE_HASH_VERSION: u32 = 1;
/// Peso do bônus de ressonância na energia de atenção (legado: 0.15).
pub const ATTENTION_WEIGHT: f64 = 0.15;

/// Config tipada da seção `[chladni]` (config/default.toml) — mesmo padrão
/// do piloto `[l1.survival]`: desserializada pelo loader do triad-platform
/// (`get_section::<ChladniConfig>("chladni")`) e injetada via construtor.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct ChladniConfig {
    /// Motor ligado (crise muda política de USO, nunca desativa mecânica).
    pub enabled: bool,
    /// Grade N×N dos padrões (mínimo 2).
    pub grid_size: usize,
    /// Teto do cache de padrões FIFO (0 = sem limite).
    pub pattern_cache_max: usize,
    /// Teto do cache de ressonância LRU (0 = sem limite).
    pub resonance_cache_max: usize,
    /// Tamanho do histórico de pedidos.
    pub history_max: usize,
    /// Versão do descritor.
    pub descriptor_version: u32,
    /// Versão do hash de evidência.
    pub evidence_hash_version: u32,
    /// Peso do bônus na atenção (consumidor L3).
    pub attention_weight: f64,
}

impl Default for ChladniConfig {
    fn default() -> Self {
        Self {
            enabled: ENABLED,
            grid_size: DEFAULT_GRID_SIZE,
            pattern_cache_max: PATTERN_CACHE_MAX,
            resonance_cache_max: RESONANCE_CACHE_MAX,
            history_max: HISTORY_MAX,
            descriptor_version: DESCRIPTOR_VERSION,
            evidence_hash_version: EVIDENCE_HASH_VERSION,
            attention_weight: ATTENTION_WEIGHT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_default_espelha_constantes() {
        let c = ChladniConfig::default();
        assert!(c.enabled);
        assert_eq!(c.grid_size, DEFAULT_GRID_SIZE);
        assert_eq!(c.pattern_cache_max, PATTERN_CACHE_MAX);
        assert_eq!(c.resonance_cache_max, RESONANCE_CACHE_MAX);
        assert_eq!(c.history_max, HISTORY_MAX);
        assert_eq!(c.descriptor_version, DESCRIPTOR_VERSION);
        assert_eq!(c.evidence_hash_version, EVIDENCE_HASH_VERSION);
        assert!((c.attention_weight - ATTENTION_WEIGHT).abs() < 1e-12);
    }
}
