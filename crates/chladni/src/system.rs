//! Orquestrador do motor Chladni — caches, contadores, validação e
//! mapeamento estado→padrão. Portado do legado `chladni_frequencies.py`
//! (`ChladniFrequencySystem`), com as leis da casa aplicadas: entrada
//! inválida = erro EXPLÍCITO (o legado usava 0.0 silencioso); taxas
//! SEMPRE com denominador (`Option<tf::Rate>`; ausência ≠ zero).

use crate::bands::{self, CognitiveBand};
use crate::pattern::{self, Pattern};
use crate::table::PlateType;
use crate::ChladniConfig;
use std::collections::{HashMap, VecDeque};
use triad_foundation as tf;
use tracing::{debug, trace, warn};

/// Chave do cache de padrões: (bits da frequência, placa, grade).
/// f64 não é `Hash`; bits servem (frequências validadas são finitas).
type PatternKey = (u64, PlateType, usize);

/// Chave canônica SIMÉTRICA do cache de ressonância: par
/// ((tamanho, hash de conteúdo)) ordenado — a vs b == b vs a.
type ResonanceKey = ((usize, u64), (usize, u64));

/// Pedido registrado no histórico (limitado por `history_max`).
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub frequency: f64,
    pub plate: PlateType,
    pub grid_size: usize,
    pub band: CognitiveBand,
    pub consumer: String,
}

/// Mapeamento estado→padrão (legado `create_state_to_pattern_mapping`).
#[derive(Debug, Clone)]
pub struct StateMapping {
    pub frequency: f64,
    pub band: CognitiveBand,
    pub pattern: Pattern,
    pub energy: f64,
    pub entropy: f64,
    pub complexity: f64,
}

/// Contadores do motor — as taxas derivadas SEMPRE têm denominador.
#[derive(Debug, Clone, Default)]
pub struct Statistics {
    pub pattern_requests: u64,
    pub pattern_cache_hits: u64,
    pub pattern_cache_misses: u64,
    pub pattern_generations: u64,
    pub pattern_cache_evictions: u64,
    pub resonance_comparisons: u64,
    pub resonance_cache_hits: u64,
    pub resonance_cache_misses: u64,
    pub resonance_cache_evictions: u64,
    pub mapping_calls: u64,
    pub invalid_inputs: u64,
}

impl Statistics {
    /// Taxa de acerto do cache de padrões — `None` sem pedidos
    /// (ausência ≠ zero, nunca 0/0 fabricado).
    pub fn pattern_cache_hit_rate(&self) -> Option<tf::Rate> {
        let total = self.pattern_cache_hits + self.pattern_cache_misses;
        tf::Rate::from_ratio(self.pattern_cache_hits, total)
    }

    /// Taxa de acerto do cache de ressonância — idem.
    pub fn resonance_cache_hit_rate(&self) -> Option<tf::Rate> {
        let total = self.resonance_cache_hits + self.resonance_cache_misses;
        tf::Rate::from_ratio(self.resonance_cache_hits, total)
    }
}

/// Motor de ressonância Chladni — biblioteca pura; os consumidores
/// (L1 sinal, atenção L3, development) injetam via construtor.
pub struct ChladniFrequencySystem {
    config: ChladniConfig,
    pattern_cache: HashMap<PatternKey, Pattern>,
    /// Ordem FIFO de inserção (evicção remove a mais antiga).
    pattern_order: VecDeque<PatternKey>,
    resonance_cache: HashMap<ResonanceKey, f64>,
    /// Ordem LRU (hit move pro fim; evicção remove a mais antiga).
    resonance_order: VecDeque<ResonanceKey>,
    stats: Statistics,
    history: VecDeque<HistoryEntry>,
}

impl ChladniFrequencySystem {
    /// Motor com a config default (seção `[chladni]`).
    pub fn new() -> Self {
        Self::with_config(ChladniConfig::default())
    }

    /// Motor com config injetada (padrão do piloto config central).
    pub fn with_config(config: ChladniConfig) -> Self {
        Self {
            config,
            pattern_cache: HashMap::new(),
            pattern_order: VecDeque::new(),
            resonance_cache: HashMap::new(),
            resonance_order: VecDeque::new(),
            stats: Statistics::default(),
            history: VecDeque::new(),
        }
    }

    pub fn config(&self) -> &ChladniConfig {
        &self.config
    }

    /// Energia [0,1] → frequência (Hz) da banda cognitiva. INVERSÃO do
    /// legado: energia alta → banda BAIXA (banda por `entropy_to_band(1−e)`):
    /// e>0.75→stability; 0.5<e≤0.75→memory; 0.25<e≤0.5→creativity;
    /// e≤0.25→meta. f = fmin + e·(fmax−fmin) da banda.
    pub fn energy_to_frequency(&self, energy: f64) -> Result<f64, tf::TriadError> {
        if !energy.is_finite() || !(0.0..=1.0).contains(&energy) {
            warn!(energia = energy, "energia fora do domínio [0,1]");
            return Err(tf::TriadError::Invalid {
                about: "energia fora do domínio [0,1]".to_string(),
                reason: format!("valor recebido: {energy}"),
            });
        }
        let band = bands::entropy_to_band(1.0 - energy);
        let (fmin, fmax) = band.range();
        Ok(fmin + energy * (fmax - fmin))
    }

    /// Padrão da frequência na placa, com cache FIFO por
    /// (frequência, placa, grade). `grid_size` `None` → default da config
    /// (mínimo 2). Entrada inválida = erro explícito (legado: 0.0).
    pub fn get_pattern_from_frequency(
        &mut self,
        frequency: f64,
        plate: PlateType,
        grid_size: Option<usize>,
        consumer: &str,
    ) -> Result<Pattern, tf::TriadError> {
        if !frequency.is_finite() || frequency <= 0.0 {
            self.stats.invalid_inputs += 1;
            warn!(freq = frequency, "frequência inválida");
            return Err(tf::TriadError::Invalid {
                about: "frequência inválida para geração de padrão".to_string(),
                reason: format!("valor recebido: {frequency}"),
            });
        }
        self.stats.pattern_requests += 1;
        let n = grid_size.unwrap_or(self.config.grid_size).max(2);
        let key = (frequency.to_bits(), plate, n);

        if let Some(p) = self.pattern_cache.get(&key) {
            self.stats.pattern_cache_hits += 1;
            trace!(freq = frequency, "cache de padrão hit");
            return Ok(p.clone());
        }
        self.stats.pattern_cache_misses += 1;
        self.stats.pattern_generations += 1;
        debug!(freq = frequency, grade = n, "gerando padrão");
        let p = pattern::generate(frequency, plate, n, self.config.descriptor_version);

        if self.config.pattern_cache_max > 0
            && self.pattern_cache.len() >= self.config.pattern_cache_max
        {
            // FIFO: remove a inserção mais antiga ANTES de inserir.
            if let Some(oldest) = self.pattern_order.pop_front() {
                self.pattern_cache.remove(&oldest);
                self.stats.pattern_cache_evictions += 1;
                debug!("evicção FIFO do cache de padrões");
            }
        }
        self.pattern_cache.insert(key, p.clone());
        self.pattern_order.push_back(key);

        self.history.push_back(HistoryEntry {
            frequency,
            plate,
            grid_size: n,
            band: p.band,
            consumer: consumer.to_string(),
        });
        if self.config.history_max > 0 && self.history.len() > self.config.history_max {
            self.history.pop_front();
        }
        Ok(p)
    }

    /// Atalho do legado: sempre placa circular + grade default.
    pub fn get_pattern_for_frequency(
        &mut self,
        frequency: f64,
        consumer: &str,
    ) -> Result<Pattern, tf::TriadError> {
        self.get_pattern_from_frequency(frequency, PlateType::Circular, None, consumer)
    }

    /// Banda cognitiva da frequência (fallback stability fora do domínio
    /// — comportamento do legado, sem validação).
    pub fn get_cognitive_band(&self, frequency: f64) -> CognitiveBand {
        bands::classify(frequency)
    }

    /// Ressonância entre dois padrões (similaridade cosseno, [0,1]) com
    /// cache LRU de chave SIMÉTRICA — a×b == b×a. Grades distintas são
    /// redimensionadas (bilinear) para a grade do primeiro.
    pub fn compute_resonance_between_patterns(
        &mut self,
        a: &Pattern,
        b: &Pattern,
        _consumer: &str,
    ) -> Result<f64, tf::TriadError> {
        let p1 = (a.size, a.content_hash);
        let p2 = (b.size, b.content_hash);
        let key = if p1 <= p2 { (p1, p2) } else { (p2, p1) };

        if let Some(v) = self.resonance_cache.get(&key) {
            self.stats.resonance_cache_hits += 1;
            // LRU: hit volta pro fim da fila.
            if let Some(pos) = self
                .resonance_order
                .iter()
                .position(|k| k == &key)
            {
                self.resonance_order.remove(pos);
            }
            self.resonance_order.push_back(key);
            trace!("cache de ressonância hit");
            return Ok(*v);
        }
        self.stats.resonance_cache_misses += 1;
        self.stats.resonance_comparisons += 1;
        let grid_b = if a.size == b.size {
            b.grid.clone()
        } else {
            pattern::resize_bilinear(&b.grid, b.size, a.size)
        };
        let value = pattern::cosine_similarity(&a.grid, &grid_b);
        debug!(valor = value, "ressonância calculada");

        if self.config.resonance_cache_max > 0
            && self.resonance_cache.len() >= self.config.resonance_cache_max
        {
            if let Some(oldest) = self.resonance_order.pop_front() {
                self.resonance_cache.remove(&oldest);
                self.stats.resonance_cache_evictions += 1;
                debug!("evicção LRU do cache de ressonância");
            }
        }
        self.resonance_cache.insert(key, value);
        self.resonance_order.push_back(key);
        Ok(value)
    }

    /// Estado (ex. 97D) → energia/entropia → banda → frequência → padrão
    /// (legado `create_state_to_pattern_mapping`). Estado vazio = erro.
    pub fn create_state_to_pattern_mapping(
        &mut self,
        state: &[f64],
        plate: PlateType,
        consumer: &str,
    ) -> Result<StateMapping, tf::TriadError> {
        if state.is_empty() {
            self.stats.invalid_inputs += 1;
            warn!("estado vazio no mapeamento chladni");
            return Err(tf::TriadError::Invalid {
                about: "estado vazio não mapeia para padrão".to_string(),
                reason: "ausência de estado é erro explícito, não zero".to_string(),
            });
        }
        let energy: f64 = state.iter().map(|v| v * v).sum::<f64>().sqrt();
        let entropy = -state
            .iter()
            .map(|v| v * (v.abs() + 1e-10).ln())
            .sum::<f64>();
        let e_norm = (energy / 10.0).clamp(0.0, 1.0);
        let h_norm = (entropy / 5.0).clamp(0.0, 1.0);
        let band = bands::entropy_to_band(h_norm);
        let (fmin, fmax) = band.range();
        let frequency = fmin + e_norm * (fmax - fmin);
        let p = self.get_pattern_from_frequency(frequency, plate, None, consumer)?;
        self.stats.mapping_calls += 1;
        Ok(StateMapping {
            frequency,
            band,
            complexity: p.complexity,
            pattern: p,
            energy,
            entropy,
        })
    }

    pub fn statistics(&self) -> &Statistics {
        &self.stats
    }

    pub fn history(&self) -> &VecDeque<HistoryEntry> {
        &self.history
    }
}

impl Default for ChladniFrequencySystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn energia_para_frequencia_por_banda() {
        let s = ChladniFrequencySystem::new();
        assert!((s.energy_to_frequency(0.9).unwrap() - 233.0).abs() < 1e-9);
        assert!((s.energy_to_frequency(0.6).unwrap() - 520.0).abs() < 1e-9);
        assert!((s.energy_to_frequency(0.3).unwrap() - 940.0).abs() < 1e-9);
        assert!((s.energy_to_frequency(0.1).unwrap() - 1750.0).abs() < 1e-9);
    }

    #[test]
    fn energia_invalida_e_erro_explícito() {
        let s = ChladniFrequencySystem::new();
        assert!(s.energy_to_frequency(f64::NAN).is_err());
        assert!(s.energy_to_frequency(-0.1).is_err());
        assert!(s.energy_to_frequency(1.1).is_err());
    }

    #[test]
    fn frequencia_invalida_e_erro_explícito() {
        let mut s = ChladniFrequencySystem::new();
        assert!(s.get_pattern_for_frequency(f64::NAN, "t").is_err());
        assert!(s.get_pattern_for_frequency(0.0, "t").is_err());
        assert!(s.get_pattern_for_frequency(-5.0, "t").is_err());
        assert_eq!(s.statistics().invalid_inputs, 3);
        assert_eq!(s.statistics().pattern_requests, 0, "inválido não conta pedido");
    }

    #[test]
    fn padrao_cacheado_na_segunda_chamada() {
        let mut s = ChladniFrequencySystem::new();
        let a = s.get_pattern_for_frequency(440.0, "t").unwrap();
        let b = s.get_pattern_for_frequency(440.0, "t").unwrap();
        assert_eq!(a.content_hash, b.content_hash);
        let st = s.statistics();
        assert_eq!(st.pattern_requests, 2);
        assert_eq!(st.pattern_cache_hits, 1);
        assert_eq!(st.pattern_cache_misses, 1);
        assert_eq!(st.pattern_generations, 1);
    }

    #[test]
    fn cache_de_padroes_e_fifo() {
        let mut s = ChladniFrequencySystem::with_config(ChladniConfig {
            pattern_cache_max: 2,
            ..ChladniConfig::default()
        });
        s.get_pattern_for_frequency(200.0, "t").unwrap();
        s.get_pattern_for_frequency(300.0, "t").unwrap();
        s.get_pattern_for_frequency(400.0, "t").unwrap();
        assert_eq!(s.statistics().pattern_cache_evictions, 1);
        // A mais antiga (200) foi evictada: re-pedir é miss de novo.
        s.get_pattern_for_frequency(200.0, "t").unwrap();
        assert_eq!(s.statistics().pattern_cache_misses, 4);
        assert_eq!(s.statistics().pattern_cache_hits, 0);
    }

    #[test]
    fn ressonancia_com_cache_simetrico() {
        let mut s = ChladniFrequencySystem::new();
        let a = s.get_pattern_for_frequency(200.0, "t").unwrap();
        let b = s.get_pattern_for_frequency(400.0, "t").unwrap();
        let c = s.get_pattern_for_frequency(600.0, "t").unwrap();

        let self_r = s.compute_resonance_between_patterns(&a, &a, "t").unwrap();
        assert!((self_r - 1.0).abs() < 1e-9, "padrão vs ele mesmo = 1");

        let r_ab = s.compute_resonance_between_patterns(&a, &b, "t").unwrap();
        assert!((0.0..=1.0).contains(&r_ab));

        // Ordem trocada = MESMA chave canônica → hit.
        s.compute_resonance_between_patterns(&b, &a, "t").unwrap();
        assert_eq!(s.statistics().resonance_cache_hits, 1);

        let _ = c;
    }

    #[test]
    fn ressonancia_lru_evicta_o_mais_antigo() {
        let mut s = ChladniFrequencySystem::with_config(ChladniConfig {
            resonance_cache_max: 1,
            ..ChladniConfig::default()
        });
        let a = s.get_pattern_for_frequency(200.0, "t").unwrap();
        let b = s.get_pattern_for_frequency(400.0, "t").unwrap();
        let c = s.get_pattern_for_frequency(600.0, "t").unwrap();
        s.compute_resonance_between_patterns(&a, &b, "t").unwrap();
        s.compute_resonance_between_patterns(&a, &c, "t").unwrap();
        assert_eq!(s.statistics().resonance_cache_evictions, 1);
        // (a,b) saiu: re-pedir é miss.
        s.compute_resonance_between_patterns(&a, &b, "t").unwrap();
        assert_eq!(s.statistics().resonance_cache_misses, 3);
    }

    #[test]
    fn mapeamento_estado_zero() {
        let mut s = ChladniFrequencySystem::new();
        let m = s.create_state_to_pattern_mapping(&[0.0; 97], PlateType::Circular, "t").unwrap();
        assert!((m.frequency - 80.0).abs() < 1e-9);
        assert_eq!(m.band, CognitiveBand::Stability);
        assert_eq!(m.energy, 0.0);
        assert_eq!(s.statistics().mapping_calls, 1);
    }

    #[test]
    fn estado_vazio_e_erro() {
        let mut s = ChladniFrequencySystem::new();
        assert!(s.create_state_to_pattern_mapping(&[], PlateType::Circular, "t").is_err());
    }

    #[test]
    fn historico_truncado() {
        let mut s = ChladniFrequencySystem::with_config(ChladniConfig {
            history_max: 3,
            ..ChladniConfig::default()
        });
        for f in [200.0, 300.0, 400.0, 500.0, 600.0] {
            s.get_pattern_for_frequency(f, "t").unwrap();
        }
        assert_eq!(s.history().len(), 3);
        assert!((s.history()[0].frequency - 400.0).abs() < 1e-9, "mantém os últimos");
    }

    #[test]
    fn taxas_com_denominador_ou_none() {
        let mut s = ChladniFrequencySystem::new();
        assert!(s.statistics().pattern_cache_hit_rate().is_none());
        assert!(s.statistics().resonance_cache_hit_rate().is_none());
        s.get_pattern_for_frequency(440.0, "t").unwrap();
        s.get_pattern_for_frequency(440.0, "t").unwrap();
        assert!(s.statistics().pattern_cache_hit_rate().is_some());
    }
}
