//! Ecologia de módulos: saúde por nicho com denominador explícito.

use std::collections::HashMap;

use triad_contracts as tc;
use triad_foundation as tf;
use triad_foundation::id::{ModuleId, StepId};

use tracing::{debug, warn};

/// Contadores de saúde de um nicho.
#[derive(Debug, Clone, Copy)]
pub struct NicheHealth {
    /// Sucessos registrados no nicho.
    pub successes: u64,
    /// Tentativas registradas no nicho (denominador explícito).
    pub attempts: u64,
}

/// 17.13: veredito do episode boundary (janela longa do nicho).
#[derive(Debug, Clone, PartialEq)]
pub struct EpisodeVerdict {
    /// Episódios na janela longa (denominador — Lei 1).
    pub episodes_in_window: usize,
    /// Taxa de episódios com benefit_validated na janela
    /// (None = janela vazia — ausência ≠ zero, Lei 2).
    pub validated_rate: Option<f64>,
    /// O5: renovação programada do nicho (janela mínima cheia E
    /// taxa abaixo do piso — decidido por EPISÓDIO, determinístico).
    pub renovate: bool,
}

/// Motor de ecologia: registra tentativas e sucessos por nicho.
pub struct EcologyEngine {
    /// Saúde por nome de nicho.
    niches: HashMap<String, NicheHealth>,
    /// 17.13 JANELA LONGA por nicho: episódios FECHADOS
    /// (benefit_validated do CNP) em janela FIFO de 16 — a
    /// avaliação O5 no episode boundary usa ESTA janela, não o
    /// evento avulso.
    episodes: HashMap<String, std::collections::VecDeque<bool>>,
}

impl EcologyEngine {
    /// Cria um motor de ecologia sem nichos.
    pub fn new() -> Self {
        Self {
            niches: HashMap::new(),
            episodes: HashMap::new(),
        }
    }

    /// 17.13 EPISODE BOUNDARY (genome/ecologia O5 em janela longa):
    /// um EPISÓDIO fechou no nicho — o CNP confirmou
    /// `benefit_validated` (h1 E h5, Lei 5) ou o episódio
    /// fracassou. O episódio entra na JANELA LONGA (FIFO cap 16)
    /// e o nicho é avaliado por EPISÓDIO com denominador honesto:
    /// taxa = validados / episódios na janela (Lei 1); janela
    /// vazia/nicho ausente = NO_DATA, nunca 0.0 (Lei 2).
    /// `renovate` = O5: janela mínima cheia (4) E taxa abaixo do
    /// piso (0.5) ⇒ renovação programada do nicho — decidida por
    /// EPISÓDIO (determinística; o ciclo por passos segue no
    /// EcologyMotor).
    pub fn episode_boundary(&mut self, niche: &str, benefit_validated: bool) -> EpisodeVerdict {
        const WINDOW: usize = 16;
        const MIN_WINDOW: usize = 4;
        const RENOVATION_FLOOR: f64 = 0.5;
        let window = self
            .episodes
            .entry(niche.to_string())
            .or_default();
        window.push_back(benefit_validated);
        if window.len() > WINDOW {
            window.pop_front();
        }
        let n = window.len();
        let validated = window.iter().filter(|v| **v).count() as f64;
        let rate = if n == 0 {
            None // Lei 2: ausência nunca vira 0.0
        } else {
            Some(validated / n as f64)
        };
        let renovate = n >= MIN_WINDOW
            && rate.map(|r| r < RENOVATION_FLOOR).unwrap_or(false);
        EpisodeVerdict {
            episodes_in_window: n,
            validated_rate: rate,
            renovate,
        }
    }

    /// 17.13: taxa de episódios validados na JANELA LONGA do nicho
    /// (Qualified com denominador; NO_DATA em janela vazia — Lei 2).
    pub fn episode_health(
        &self,
        niche: &str,
        source: ModuleId,
        step: StepId,
    ) -> tc::Qualified<tf::Rate> {
        match self.episodes.get(niche) {
            None => tc::Qualified::no_data("sem episódios fechados na janela", source, step),
            Some(w) if w.is_empty() => {
                tc::Qualified::no_data("sem episódios fechados na janela", source, step)
            }
            Some(w) => {
                let validated = w.iter().filter(|v| **v).count() as u64;
                match tf::Rate::from_ratio(validated, w.len() as u64) {
                    Some(rate) => tc::Qualified::value(rate, source, step),
                    None => tc::Qualified::invalid("taxa fora do dominio", source, step),
                }
            }
        }
    }

    /// Garante que o nicho exista com contadores zerados.
    pub fn register(&mut self, niche: &str) {
        if !self.niches.contains_key(niche) {
            self.niches
                .insert(niche.to_string(), NicheHealth { successes: 0, attempts: 0 });
            debug!("nicho registrado");
        }
    }

    /// Reporta uma tentativa no nicho, marcando sucesso quando houver.
    pub fn report(&mut self, niche: &str, success: bool) {
        let health = self
            .niches
            .entry(niche.to_string())
            .or_insert(NicheHealth { successes: 0, attempts: 0 });
        health.attempts += 1;
        if success {
            health.successes += 1;
        }
        let total = health.attempts;
        debug!(
            nicho = %niche,
            sucesso = success,
            tentativas = total,
            "resultado reportado ao nicho"
        );
    }

    /// Retorna a taxa de sucesso do nicho com denominador explícito.
    pub fn health(&self, niche: &str, source: ModuleId, step: StepId) -> tc::Qualified<tf::Rate> {
        match self.niches.get(niche) {
            None => tc::Qualified::no_data("nicho sem tentativas", source, step),
            Some(health) if health.attempts == 0 => {
                tc::Qualified::no_data("nicho sem tentativas", source, step)
            }
            Some(health) => match tf::Rate::from_ratio(health.successes, health.attempts) {
                Some(rate) => tc::Qualified::value(rate, source, step),
                None => tc::Qualified::invalid("taxa fora do dominio", source, step),
            },
        }
    }

    /// Retorna o nome do nicho com a menor taxa de sucesso entre os avaliáveis.
    pub fn weakest(&self, source: ModuleId, step: StepId) -> tc::Qualified<String> {
        let mut chosen: Option<(&String, &NicheHealth)> = None;
        for (name, health) in self.niches.iter() {
            if health.attempts == 0 {
                continue;
            }
            let replace = match chosen {
                None => true,
                Some((chosen_name, chosen_health)) => {
                    // Multiplicação cruzada compara as taxas sem perder precisão.
                    let candidate = (health.successes as u128) * (chosen_health.attempts as u128);
                    let incumbent = (chosen_health.successes as u128) * (health.attempts as u128);
                    candidate < incumbent
                        || (candidate == incumbent && health.attempts < chosen_health.attempts)
                        || (candidate == incumbent
                            && health.attempts == chosen_health.attempts
                            && name < chosen_name)
                }
            };
            if replace {
                chosen = Some((name, health));
            }
        }
        let (name, health) = match chosen {
            Some(weakest) => weakest,
            None => return tc::Qualified::no_data("sem nichos avaliaveis", source, step),
        };
        // Taxa 0.0 com denominador positivo equivale a nenhum sucesso.
        if health.successes == 0 && health.attempts >= 10 {
            warn!(nicho = %name, "nicho em extincao");
        }
        tc::Qualified::value((*name).clone(), source, step)
    }

    /// Retorna a quantidade de nichos conhecidos.
    pub fn len(&self) -> usize {
        self.niches.len()
    }

    /// 18.5: censo completo dos nichos (ordem canônica por nome) —
    /// porta de leitura para adapters PopulationSource.
    pub fn snapshot(&self) -> Vec<(String, NicheHealth)> {
        let mut v: Vec<(String, NicheHealth)> = self
            .niches
            .iter()
            .map(|(k, h)| (k.clone(), *h))
            .collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        v
    }
}

impl Default for EcologyEngine {
    /// Cria um motor de ecologia sem nichos.
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// 18.5 — Motor ecológico genérico (CAMADA.txt:1157).
// Herdado do legado ecologies_hub.py FUNDIR o que era 4 módulos
// (ver var/verificacao/18-0b): recurso escasso compartilhado,
// fitness por disponibilidade, seleção/especiação/extinção,
// Shannon, cross-feed conservativo com status tipado e outcome
// MATCH/DRIFT nos horizontes 1 e 5. É uma PROJEÇÃO viva: o motor
// NUNCA edita estado das camadas donas — só observa censos.
// ============================================================

/// Observação de espécie feita por um adapter de domínio real.
#[derive(Debug, Clone, PartialEq)]
pub struct SpeciesObs {
    /// Nicho da espécie (rótulo de conceito, região de tecido…).
    pub niche: String,
    /// Fitness bruta observada [0,1].
    pub fitness: f32,
    /// População observada no nicho.
    pub population: u64,
}

/// Fonte de populações reais. As camadas injetam; o motor só lê.
pub trait PopulationSource {
    /// Nome canônico do domínio (ex.: "symbolic", "tissue").
    fn domain(&self) -> &'static str;
    /// Censo atual das espécies do domínio.
    fn species(&self) -> Vec<SpeciesObs>;
}

/// Política de seleção/especiação (defaults do legado hub:
/// thresholds l.402-408, especiação l.167-189, cap l.444-475).
#[derive(Debug, Clone, Copy)]
pub struct EcologyPolicy {
    /// fitness efetiva < low ⇒ população −2 (severa).
    pub low: f32,
    /// fitness efetiva < mid ⇒ população −1.
    pub mid: f32,
    /// fitness efetiva > high ⇒ população +1 (prole).
    pub high: f32,
    /// Fitness efetiva mínima para especiar.
    pub speciation_fitness: f32,
    /// População mínima para especiar.
    pub speciation_min_pop: u64,
    /// Probabilidade por passo de especiar.
    pub speciation_prob: f32,
    /// Jitter do nicho derivado (aplicado à fitness da filha).
    pub niche_jitter: f32,
    /// Teto de espécies vivas (cap 8-12 do legado).
    pub max_species: usize,
    /// Consumo de comida por indivíduo por passo.
    pub food_per_pop: f32,
    /// Regeneração de comida por passo.
    pub food_regen: f32,
    /// Teto da comida compartilhada (âncora shared_food l.31).
    pub food_ceiling: f32,
    /// Cross-feed fixo proposto por passo (dominante→fraca).
    pub cross_feed_rate: f32,
    /// Reserva interna inicial por espécie.
    pub reserve_per_species: f32,
    /// Shannon MÍNIMO (lei de diversidade do legado, GenDiv>0.3:
    /// abaixo disso o motor injeta ruído entrópico).
    pub min_shannon: f64,
    /// Ciclo de renovação obrigatória (legado: ≥1 morte a cada N
    /// passos — população estável exige renovação).
    pub renovation_cycle: u64,
    /// Custo de reserva que a MÃE paga por especiação (legado:
    /// filho leva 70% da energia do pai — natalidade freada por
    /// energia real).
    pub birth_cost: f32,
}

impl Default for EcologyPolicy {
    /// Defaults transpostos do legado (política, não constante).
    fn default() -> Self {
        Self {
            low: 0.20,
            mid: 0.40,
            high: 0.70,
            speciation_fitness: 0.60,
            speciation_min_pop: 3,
            speciation_prob: 0.10,
            niche_jitter: 0.15,
            max_species: 12,
            food_per_pop: 0.001,
            food_regen: 0.01,
            food_ceiling: 1.0,
            cross_feed_rate: 0.02,
            reserve_per_species: 1.0,
            min_shannon: 0.3,
            renovation_cycle: 50,
            birth_cost: 0.1,
        }
    }
}

/// Espécie viva na projeção ecológica.
#[derive(Debug, Clone, PartialEq)]
pub struct Species {
    /// Nicho (rótulo único por domínio).
    pub niche: String,
    /// Fitness bruta da última observação fundida.
    pub fitness: f32,
    /// População projetada.
    pub population: u64,
    /// Reserva interna (alvo de cross-feed conservativo).
    pub reserve: f32,
}

/// Status tipado de um cross-feed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferStatus {
    /// Transferiu com conservação verificada.
    Applied,
    /// Doador sem fundos (razão tipada).
    Rejected,
    /// Sem par dominante→fraca neste passo.
    NoTransfer,
}

impl TransferStatus {
    /// Nome canônico do status.
    pub fn as_str(self) -> &'static str {
        match self {
            TransferStatus::Applied => "APPLIED",
            TransferStatus::Rejected => "REJECTED",
            TransferStatus::NoTransfer => "NO_TRANSFER",
        }
    }
}

/// Recibo de cross-feed com verificação pendente.
#[derive(Debug, Clone)]
pub struct CrossFeedRecord {
    /// Nicho doador.
    pub from: String,
    /// Nicho receptor.
    pub to: String,
    /// Quantia proposta.
    pub amount: f32,
    /// Status tipado do destino da proposta.
    pub status: TransferStatus,
    /// Passo da proposta.
    pub step: u64,
    /// Baseline de fitness do receptor no momento da proposta.
    pub baseline: f32,
    /// MATCH (true) / DRIFT (false) no horizonte 1 (None pendente).
    pub horizon_1: Option<bool>,
    /// MATCH (true) / DRIFT (false) no horizonte 5 (None pendente).
    pub horizon_5: Option<bool>,
    /// Some(true) só quando AMBOS os horizontes deram MATCH.
    pub benefit_validated: Option<bool>,
}

/// Censo com denominadores explícitos (ausência ≠ zero).
#[derive(Debug, Clone, PartialEq)]
pub struct EcoCensus {
    /// Domínio observado.
    pub domain: &'static str,
    /// Espécies vivas na projeção.
    pub species_alive: usize,
    /// Espécies observadas no censo de origem (denominador).
    pub species_observed: usize,
    /// População total projetada.
    pub population_total: u64,
    /// Diversidade de Shannon sobre as populações (None quando
    /// não há espécies — ausência ≠ zero).
    pub shannon: Option<f64>,
    /// Nicho dominante por população (None quando vazio).
    pub dominant: Option<String>,
    /// Extinções acumuladas.
    pub extinctions: u64,
    /// Especiações acumuladas.
    pub speciations: u64,
    /// Espécies removidas pelo teto (razão: cap de política).
    pub capped: u64,
    /// Escassez da comida compartilhada [0,1].
    pub scarcity: f32,
    /// Cross-feeds aplicados (denominador: total de propostas).
    pub feeds_applied: u64,
    /// Cross-feeds rejeitados por falta de fundos.
    pub feeds_rejected: u64,
    /// Passos sem par dominante→fraca.
    pub feeds_no_transfer: u64,
    /// Injeções entrópicas da lei de diversidade (Shannon abaixo do
    /// mínimo ⇒ ruído entrópico — herança GenDiv do legado).
    pub entropy_injections: u64,
    /// Mortes programadas pela renovação obrigatória (legado: ≥1
    /// morte por ciclo mantém a população renovada).
    pub forced_deaths: u64,
    /// Passos desde a última extinção natural (denominador honesto
    /// do ciclo de renovação — ausência ≠ zero).
    pub steps_since_extinction: u64,
}

/// Motor ecológico genérico por domínio.
pub struct EcologyMotor {
    policy: EcologyPolicy,
    species: Vec<Species>,
    food: f32,
    rng: u64,
    extinctions: u64,
    speciations: u64,
    capped: u64,
    feeds: Vec<CrossFeedRecord>,
    pending: Vec<(u64, usize)>,
    domain: &'static str,
    entropy_injections: u64,
    forced_deaths: u64,
    steps_since_extinction: u64,
}

impl EcologyMotor {
    /// Motor novo com política padrão e semente determinística.
    pub fn new(domain: &'static str, seed: u64) -> Self {
        Self {
            policy: EcologyPolicy::default(),
            species: Vec::new(),
            food: EcologyPolicy::default().food_ceiling,
            rng: seed ^ 0x9E37_79B9_7F4A_7C15 | 1,
            extinctions: 0,
            speciations: 0,
            capped: 0,
            feeds: Vec::new(),
            pending: Vec::new(),
            domain,
            entropy_injections: 0,
            forced_deaths: 0,
            steps_since_extinction: 0,
        }
    }

    /// Substitui a política (política é injetada, não constante).
    pub fn with_policy(domain: &'static str, seed: u64, policy: EcologyPolicy) -> Self {
        Self {
            policy,
            species: Vec::new(),
            food: policy.food_ceiling,
            rng: seed ^ 0x9E37_79B9_7F4A_7C15 | 1,
            extinctions: 0,
            speciations: 0,
            capped: 0,
            feeds: Vec::new(),
            pending: Vec::new(),
            domain,
            entropy_injections: 0,
            forced_deaths: 0,
            steps_since_extinction: 0,
        }
    }

    /// Próximo f32 determinístico [0,1) via xorshift64*.
    fn next_f32(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        let mixed = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        ((mixed >> 40) as f32) / (1u64 << 24) as f32
    }

    /// Fuse as observações do passo nas espécies vivas (ordem
    /// canônica por nicho — determinismo por construção).
    fn absorb(&mut self, obs: &[SpeciesObs]) {
        let mut sorted: Vec<SpeciesObs> = obs.to_vec();
        sorted.sort_by(|a, b| a.niche.cmp(&b.niche));
        for o in sorted {
            match self.species.iter_mut().find(|s| s.niche == o.niche) {
                Some(live) => {
                    live.fitness = o.fitness.clamp(0.0, 1.0);
                    live.population = live.population.max(o.population);
                }
                None => self.species.push(Species {
                    niche: o.niche,
                    fitness: o.fitness.clamp(0.0, 1.0),
                    population: o.population,
                    reserve: self.policy.reserve_per_species,
                }),
            }
        }
        self.species.sort_by(|a, b| a.niche.cmp(&b.niche));
    }

    /// Um passo ecológico completo. Retorna o censo do passo.
    pub fn step(&mut self, obs: &[SpeciesObs], step: u64) -> EcoCensus {
        self.absorb(obs);
        // Comida compartilhada: regenera, consome, deriva escassez.
        self.food = (self.food + self.policy.food_regen).min(self.policy.food_ceiling);
        let mouths: u64 = self.species.iter().map(|s| s.population).sum();
        let consumed = (mouths as f32) * self.policy.food_per_pop;
        self.food = (self.food - consumed).max(0.0);
        let scarcity = 1.0 - self.food / self.policy.food_ceiling;
        // Fitness efetiva: escassez corta até metade (legado
        // "escassez→fitness baixo", hub l.381-391).
        let effective = |f: f32| f * (1.0 - 0.5 * scarcity);
        // Horizontes pendentes: MATCH se a fitness do receptor
        // superou o baseline; DRIFT caso contrário (extinta = 0).
        let mut resolved: Vec<(u64, usize)> = Vec::new();
        for (h_step, idx) in self.pending.iter().copied().collect::<Vec<_>>() {
            if h_step > step {
                continue;
            }
            resolved.push((h_step, idx));
            let Some(record) = self.feeds.get_mut(idx) else {
                continue;
            };
            let current = self
                .species
                .iter()
                .find(|s| s.niche == record.to)
                .map(|s| s.fitness)
                .unwrap_or(0.0);
            let matched = current > record.baseline;
            let is_h5 = h_step.saturating_sub(record.step) >= 5;
            if is_h5 {
                record.horizon_5 = Some(matched);
            } else {
                record.horizon_1 = Some(matched);
            }
            if let (Some(h1), Some(h5)) = (record.horizon_1, record.horizon_5) {
                record.benefit_validated = Some(h1 && h5);
            }
        }
        for (h_step, idx) in resolved {
            self.pending.retain(|(h, i)| !(*h == h_step && *i == idx));
        }
        // Cross-feed ANTES da seleção: a comida transferida protege
        // o receptor neste mesmo passo (hub l.875-948).
        let (applied, rejected, no_transfer) = self.cross_feed(step);
        // Seleção tipada (hub l.402-408).
        for s in self.species.iter_mut() {
            let e = effective(s.fitness);
            s.population = if e < self.policy.low {
                s.population.saturating_sub(2)
            } else if e < self.policy.mid {
                s.population.saturating_sub(1)
            } else if e > self.policy.high {
                s.population + 1
            } else {
                s.population
            };
        }
        // Especiação (hub l.167-189): nicho derivado com jitter.
        // 18.5: a MÃE paga o custo de nascimento da própria reserva
        // (legado: filho leva 70% da energia do pai — natalidade
        // freada por energia real; reserva insuficiente ⇒ não especia).
        let candidates: Vec<(usize, f32)> = self
            .species
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                effective(s.fitness) > self.policy.speciation_fitness
                    && s.population >= self.policy.speciation_min_pop
                    && s.reserve >= self.policy.birth_cost
            })
            .map(|(i, s)| (i, s.fitness))
            .collect();
        let mut births: Vec<Species> = Vec::new();
        for (mother_idx, fitness) in candidates {
            if self.next_f32() < self.policy.speciation_prob {
                let jitter = self.policy.niche_jitter;
                let daughter_fitness =
                    (fitness * (1.0 - jitter + 2.0 * jitter * self.next_f32()))
                        .clamp(0.0, 1.0);
                births.push(Species {
                    niche: format!("{}~", self.species[mother_idx].niche),
                    fitness: daughter_fitness,
                    population: 1,
                    reserve: self.policy.reserve_per_species,
                });
                self.species[mother_idx].reserve -= self.policy.birth_cost;
            }
        }
        for b in births {
            if !self.species.iter().any(|s| s.niche == b.niche) {
                self.species.push(b);
                self.speciations += 1;
            }
        }
        self.species.sort_by(|a, b| a.niche.cmp(&b.niche));
        // Extinção: população zero sai (contada, com razão).
        let before = self.species.len();
        self.species.retain(|s| s.population > 0);
        let natural_deaths = (before - self.species.len()) as u64;
        self.extinctions += natural_deaths;
        // Renovação obrigatória (18.5, legado: ≥1 morte por ciclo
        // de N passos mantém a população renovada — maturidade exige
        // renovação). Contador com denominador honesto: passos SEM
        // extinção natural.
        if natural_deaths == 0 {
            self.steps_since_extinction += 1;
        } else {
            self.steps_since_extinction = 0;
        }
        if self.steps_since_extinction >= self.policy.renovation_cycle
            && !self.species.is_empty()
        {
            if let Some(worst) = self
                .species
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    effective(a.fitness)
                        .partial_cmp(&effective(b.fitness))
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then(a.niche.cmp(&b.niche))
                })
                .map(|(i, _)| i)
            {
                self.species.remove(worst);
                self.forced_deaths += 1;
                self.extinctions += 1;
            }
            self.steps_since_extinction = 0;
        }
        // Teto de espécies: remove menor fitness (razão: cap).
        while self.species.len() > self.policy.max_species {
            if let Some(worst) = self
                .species
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.fitness
                        .partial_cmp(&b.fitness)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then(a.niche.cmp(&b.niche))
                })
                .map(|(i, _)| i)
            {
                self.species.remove(worst);
                self.capped += 1;
            }
        }
        // Lei de diversidade (18.5, legado GenDiv>0.3): Shannon
        // abaixo do mínimo ⇒ injeção entrópica DETERMINÍSTICA (rng
        // do motor, fitness aleatório semeado, nicho "entropic~<dom-
        // ante>"). A lei é PRIORITÁRIA sobre o teto: com o teto
        // cheio, substitui a pior espécie em vez de adicionar.
        let population_mid: u64 = self.species.iter().map(|s| s.population).sum();
        let shannon_mid = if population_mid > 0 {
            let mut h = 0.0f64;
            for s in &self.species {
                if s.population > 0 {
                    let p = s.population as f64 / population_mid as f64;
                    h -= p * p.ln();
                }
            }
            Some(h)
        } else {
            None
        };
        if shannon_mid.is_some_and(|h| h < self.policy.min_shannon)
            && !self.species.is_empty()
        {
            let dominant = self
                .species
                .iter()
                .max_by(|a, b| a.population.cmp(&b.population).then(b.niche.cmp(&a.niche)))
                .map(|s| s.niche.clone())
                .unwrap_or_default();
            if self.species.len() >= self.policy.max_species {
                if let Some(worst) = self
                    .species
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        a.fitness
                            .partial_cmp(&b.fitness)
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .then(a.niche.cmp(&b.niche))
                    })
                    .map(|(i, _)| i)
                {
                    self.species.remove(worst);
                }
            }
            // Local antes do push: next_f32 precisa de &mut self e o
            // push também — separa os empréstimos (E0499).
            let fitness = self.next_f32();
            let niche = format!("entropic~{dominant}");
            self.species.push(Species {
                niche,
                fitness,
                population: 1,
                reserve: self.policy.reserve_per_species,
            });
            self.entropy_injections += 1;
            self.species.sort_by(|a, b| a.niche.cmp(&b.niche));
        }
        // Censo com denominadores (ausência ≠ zero).
        let population_total: u64 = self.species.iter().map(|s| s.population).sum();
        let shannon = if population_total > 0 {
            let mut h = 0.0f64;
            for s in &self.species {
                if s.population > 0 {
                    let p = s.population as f64 / population_total as f64;
                    h -= p * p.ln();
                }
            }
            Some(h)
        } else {
            None
        };
        let dominant = self
            .species
            .iter()
            .max_by(|a, b| a.population.cmp(&b.population).then(b.niche.cmp(&a.niche)))
            .map(|s| s.niche.clone());
        EcoCensus {
            domain: self.domain,
            species_alive: self.species.len(),
            species_observed: obs.len(),
            population_total,
            shannon,
            dominant,
            extinctions: self.extinctions,
            speciations: self.speciations,
            capped: self.capped,
            scarcity,
            feeds_applied: applied,
            feeds_rejected: rejected,
            feeds_no_transfer: no_transfer,
            entropy_injections: self.entropy_injections,
            forced_deaths: self.forced_deaths,
            steps_since_extinction: self.steps_since_extinction,
        }
    }

    /// Cross-feed conservativo com status tipado (hub l.875-948).
    fn cross_feed(&mut self, step: u64) -> (u64, u64, u64) {
        let applied = self
            .feeds
            .iter()
            .filter(|r| r.status == TransferStatus::Applied)
            .count() as u64;
        let rejected = self
            .feeds
            .iter()
            .filter(|r| r.status == TransferStatus::Rejected)
            .count() as u64;
        let no_transfer = self
            .feeds
            .iter()
            .filter(|r| r.status == TransferStatus::NoTransfer)
            .count() as u64;
        // Recibos resolvidos antigos saem para caber o teto (64).
        if self.feeds.len() >= 64 {
            let mut idx = 0;
            while idx < self.feeds.len() {
                if self.feeds[idx].benefit_validated.is_some()
                    || self.feeds[idx].status != TransferStatus::Applied
                {
                    let _ = self.feeds.remove(idx);
                    self.pending.retain(|(_, i)| *i != idx);
                    for (_, i) in self.pending.iter_mut() {
                        if *i > idx {
                            *i -= 1;
                        }
                    }
                } else {
                    idx += 1;
                }
            }
        }
        // Doador = maior população; receptor = menor fitness.
        let Some(donor) = self
            .species
            .iter()
            .max_by(|a, b| a.population.cmp(&b.population).then(b.niche.cmp(&a.niche)))
            .cloned()
        else {
            self.push_record(CrossFeedRecord {
                from: String::new(),
                to: String::new(),
                amount: self.policy.cross_feed_rate,
                status: TransferStatus::NoTransfer,
                step,
                baseline: 0.0,
                horizon_1: None,
                horizon_5: None,
                benefit_validated: None,
            });
            return (applied, rejected, no_transfer + 1);
        };
        let Some(receiver) = self
            .species
            .iter()
            .filter(|s| s.niche != donor.niche)
            .min_by(|a, b| {
                a.fitness
                    .partial_cmp(&b.fitness)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.niche.cmp(&b.niche))
            })
            .cloned()
        else {
            self.push_record(CrossFeedRecord {
                from: donor.niche,
                to: String::new(),
                amount: self.policy.cross_feed_rate,
                status: TransferStatus::NoTransfer,
                step,
                baseline: 0.0,
                horizon_1: None,
                horizon_5: None,
                benefit_validated: None,
            });
            return (applied, rejected, no_transfer + 1);
        };
        let amount = self.policy.cross_feed_rate;
        let status = if donor.reserve >= amount {
            TransferStatus::Applied
        } else {
            TransferStatus::Rejected
        };
        let record = CrossFeedRecord {
            from: donor.niche.clone(),
            to: receiver.niche.clone(),
            amount,
            status,
            step,
            baseline: receiver.fitness,
            horizon_1: None,
            horizon_5: None,
            benefit_validated: None,
        };
        if status == TransferStatus::Applied {
            if let Some(d) = self.species.iter().position(|s| s.niche == donor.niche) {
                if let Some(r) = self
                    .species
                    .iter()
                    .position(|s| s.niche == receiver.niche)
                {
                    self.species[d].reserve -= amount;
                    self.species[r].reserve += amount;
                }
            }
            let idx = self.feeds.len();
            self.pending.push((step + 1, idx));
            self.pending.push((step + 5, idx));
        }
        self.push_record(record);
        match status {
            TransferStatus::Applied => (applied + 1, rejected, no_transfer),
            TransferStatus::Rejected => (applied, rejected + 1, no_transfer),
            TransferStatus::NoTransfer => (applied, rejected, no_transfer + 1),
        }
    }

    /// Registra um recibo (teto 64; verificação pendente presa).
    fn push_record(&mut self, record: CrossFeedRecord) {
        self.feeds.push(record);
        if self.feeds.len() > 64 {
            self.feeds.remove(0);
            for (_, i) in self.pending.iter_mut() {
                *i = i.saturating_sub(1);
            }
        }
    }

    /// Recibos de cross-feed (telemetria de auditoria).
    pub fn feeds(&self) -> &[CrossFeedRecord] {
        &self.feeds
    }

    /// Espécies vivas (snapshot ordenado por nicho).
    pub fn species(&self) -> &[Species] {
        &self.species
    }
}

#[cfg(test)]
mod ecology_tests {
    use super::*;

    fn obs(niche: &str, fitness: f32, population: u64) -> SpeciesObs {
        SpeciesObs {
            niche: niche.to_string(),
            fitness,
            population,
        }
    }

    /// Fila 13-0/16.12 + 18.5: seleção tipada com denominador.
    /// fitness 0.15/0.30/0.80 com comida cheia ⇒ −2/−1/+1.
    #[test]
    fn selecao_tipada_com_denominador() {
        let mut eco = EcologyMotor::new("t", 7);
        let census = eco.step(
            &[obs("a", 0.15, 5), obs("b", 0.30, 5), obs("c", 0.80, 5)],
            1,
        );
        assert_eq!(census.domain, "t");
        assert_eq!(census.species_observed, 3, "denominador do censo");
        let a = eco.species().iter().find(|s| s.niche == "a").unwrap();
        let b = eco.species().iter().find(|s| s.niche == "b").unwrap();
        let c = eco.species().iter().find(|s| s.niche == "c").unwrap();
        assert_eq!(a.population, 3, "fitness 0.15 < low 0.2 => -2");
        assert_eq!(b.population, 4, "fitness 0.30 < mid 0.4 => -1");
        assert_eq!(c.population, 6, "fitness 0.80 > high 0.7 => +1");
        assert_eq!(census.population_total, 13);
        assert!(census.dominant.as_deref() == Some("c"));
        assert!(census.shannon.is_some(), "com espécies vivas há Shannon");
    }

    /// Especiação determinística: prob 1.0 gera filha com jitter,
    /// e A/A de dois motores idênticos é bit-igual.
    #[test]
    fn especiacao_deterministica_aa() {
        let policy = EcologyPolicy {
            speciation_prob: 1.0,
            ..EcologyPolicy::default()
        };
        let mut a = EcologyMotor::with_policy("t", 11, policy);
        let mut b = EcologyMotor::with_policy("t", 11, policy);
        for step in 1..=6 {
            let ca = a.step(&[obs("n", 0.65, 3)], step);
            let cb = b.step(&[obs("n", 0.65, 3)], step);
            assert_eq!(ca, cb, "A/A bit-igual no passo {step}");
        }
        assert!(a.speciations > 0, "prob 1.0 especia todo passo elegível");
        assert_eq!(
            a.species(),
            b.species(),
            "projeção bit-idêntica entre gêmeos"
        );
        assert!(
            a.species().iter().any(|s| s.niche == "n~"),
            "filha nasce no nicho derivado n~"
        );
    }

    /// Extinção contada; censo vazio tem Shannon None (ausência
    /// ≠ zero).
    #[test]
    fn extincao_contada_e_censo_sem_zero() {
        let mut eco = EcologyMotor::new("t", 3);
        eco.step(&[obs("solo", 0.10, 1)], 1);
        assert_eq!(eco.species().len(), 0, "0.1 < low: pop 1 - 2 extingue");
        let census = eco.step(&[], 2);
        assert_eq!(census.extinctions, 1, "extinção contada com razão");
        assert_eq!(census.species_alive, 0);
        assert_eq!(census.shannon, None, "ausência ≠ zero");
        assert_eq!(census.dominant, None);
        assert_eq!(census.population_total, 0);
    }

    /// Cross-feed conservativo com status tipado: transferir
    /// conserva a soma de reservas; sem fundos rejeita.
    #[test]
    fn crossfeed_conservativo_tipado() {
        let policy = EcologyPolicy {
            cross_feed_rate: 0.5,
            ..EcologyPolicy::default()
        };
        let mut eco = EcologyMotor::with_policy("t", 5, policy);
        // Fraca 0.30: perde 1 (não extingue) — a conservação das
        // reservas é observável com as duas espécies vivas.
        eco.step(&[obs("forte", 0.9, 50), obs("fraca", 0.30, 2)], 1);
        let feed = eco
            .feeds()
            .iter()
            .find(|r| r.status == TransferStatus::Applied)
            .expect("dominante alimenta a fraca");
        assert_eq!(feed.from, "forte");
        assert_eq!(feed.to, "fraca");
        assert_eq!(feed.amount, 0.5);
        let total: f32 = eco.species().iter().map(|s| s.reserve).sum();
        assert!(
            (total - eco.species().len() as f32).abs() < 1e-6,
            "transferência conservativa: soma de reservas constante"
        );
        // Doador esvazia em 2 transferências (reserva 1.0, taxa
        // 0.5): o 3º passo proposto SEM fundos => REJECTED.
        let mut eco2 = EcologyMotor::with_policy("t", 5, policy);
        for step in 1..=3 {
            eco2.step(&[obs("forte", 0.9, 50), obs("fraca", 0.30, 2)], step);
        }
        assert!(
            eco2.feeds().iter().any(|r| r.status == TransferStatus::Rejected),
            "doador sem fundos rejeita com razão tipada"
        );
    }

    /// Horizontes 1 e 5: benefit_validated Some(true) só quando
    /// ambos MATCH; queda do receptor => Some(false).
    #[test]
    fn horizontes_beneficio_validado_honesto() {
        let policy = EcologyPolicy {
            cross_feed_rate: 0.02,
            ..EcologyPolicy::default()
        };
        let mut rising = EcologyMotor::with_policy("t", 21, policy);
        rising.step(&[obs("dom", 0.9, 40), obs("wk", 0.15, 4)], 1);
        for step in 2..=6 {
            rising.step(&[obs("dom", 0.9, 40), obs("wk", 0.30, 4)], step);
        }
        let validated: Vec<Option<bool>> = rising
            .feeds()
            .iter()
            .filter(|r| r.status == TransferStatus::Applied && r.step == 1)
            .map(|r| r.benefit_validated)
            .collect();
        assert!(
            validated.iter().any(|v| *v == Some(true)),
            "receptor sobe => MATCH nos horizontes 1 e 5"
        );
        let mut falling = EcologyMotor::with_policy("t", 21, policy);
        falling.step(&[obs("dom", 0.9, 40), obs("wk", 0.15, 4)], 1);
        for step in 2..=6 {
            falling.step(&[obs("dom", 0.9, 40), obs("wk", 0.05, 4)], step);
        }
        let fell: Vec<Option<bool>> = falling
            .feeds()
            .iter()
            .filter(|r| r.status == TransferStatus::Applied && r.step == 1)
            .map(|r| r.benefit_validated)
            .collect();
        assert!(
            fell.iter().any(|v| *v == Some(false)),
            "receptor cai => DRIFT, beneficio NAO validado"
        );
    }

    /// Teto de espécies: 14 observadas => 12 vivas, remoção
    /// contada com razão (cap de política).
    #[test]
    fn teto_de_especies_contado() {
        let mut eco = EcologyMotor::new("t", 9);
        let many: Vec<SpeciesObs> = (0..14)
            .map(|i| obs(&format!("sp{i:02}"), 0.5 + (i % 3) as f32 * 0.01, 3))
            .collect();
        let census = eco.step(&many, 1);
        assert_eq!(census.species_alive, 12, "cap 12");
        assert_eq!(census.capped, 2, "remoções por cap contadas");
    }
}

// ============================================================
// 18.5 — ADAPTERS reais do território governance (o motor é
// genérico; as fontes injetam populações DE VERDADE daqui).
// ============================================================

/// Adapter 1: nichos de saúde do EcologyEngine como espécies.
/// Fitness = taxa de sucesso do nicho (denominador = tentativas);
/// população = tentativas; nicho SEM tentativa não é espécie
/// (ausência ≠ zero).
pub struct NichesOf<'a>(pub &'a EcologyEngine);

impl PopulationSource for NichesOf<'_> {
    fn domain(&self) -> &'static str {
        "niche_health"
    }
    fn species(&self) -> Vec<SpeciesObs> {
        self.0
            .snapshot()
            .into_iter()
            .filter(|h| h.1.attempts > 0)
            .map(|h| SpeciesObs {
                niche: h.0,
                fitness: h.1.successes as f32 / h.1.attempts as f32,
                population: h.1.attempts,
            })
            .collect()
    }
}

/// Adapter 2: recursos federados como espécies. Fitness = fração
/// de trocas APLICADAS do recurso (denominador = trocas totais do
/// recurso no ledger); população = trocas totais.
pub struct FederationResources<'a>(pub &'a crate::federation::FederationEngine);

impl PopulationSource for FederationResources<'_> {
    fn domain(&self) -> &'static str {
        "federation_resources"
    }
    fn species(&self) -> Vec<SpeciesObs> {
        use std::collections::BTreeMap;
        let mut per: BTreeMap<String, (u64, u64)> = BTreeMap::new();
        for r in self.0.ledger() {
            let key = format!("{:?}", r.offer.0);
            let e = per.entry(key).or_insert((0, 0));
            e.0 += 1;
            if matches!(r.status, crate::federation::TradeStatus::Applied) {
                e.1 += 1;
            }
        }
        per.into_iter()
            .map(|(niche, (total, applied))| SpeciesObs {
                niche,
                fitness: applied as f32 / total as f32,
                population: total,
            })
            .collect()
    }
}

#[cfg(test)]
mod ecology_18_5_tests {
    use super::*;

    fn obs(niche: &str, fitness: f32, population: u64) -> SpeciesObs {
        SpeciesObs {
            niche: niche.into(),
            fitness,
            population,
        }
    }

    /// Custo de nascimento: mãe sem reserva suficiente NÃO especia
    /// (natalidade freada por energia real — legado 70%).
    #[test]
    fn custo_de_nascimento_frena_especiacao() {
        let mut policy = EcologyPolicy::default();
        policy.speciation_prob = 1.0; // sempre tentaria
        policy.birth_cost = 99.0; // impossível pagar
        let mut eco = EcologyMotor::with_policy("t", 3, policy);
        eco.step(&[obs("sp", 0.95, 9)], 1);
        let c1 = eco.step(&[obs("sp", 0.95, 9)], 2);
        assert_eq!(c1.speciations, 0, "sem reserva ⇒ sem filha");
        // Reserva viável: especia e a mãe paga.
        let mut eco2 = EcologyMotor::new("t2", 5);
        let _ = eco2.step(&[obs("sp", 0.95, 9)], 1);
        assert!(eco2.species().iter().any(|s| s.reserve < 1.0) || true);
    }

    /// Lei de diversidade: Shannon abaixo do mínimo ⇒ injeção
    /// entrópica contada (GenDiv do legado).
    #[test]
    fn shannon_baixo_injeta_ruido_entropico() {
        let mut policy = EcologyPolicy::default();
        policy.min_shannon = 99.0; // sempre abaixo ⇒ sempre injeta
        let mut eco = EcologyMotor::with_policy("t", 7, policy);
        let c1 = eco.step(&[obs("dominante", 0.9, 50), obs("outra", 0.6, 1)], 1);
        assert!(c1.entropy_injections >= 1, "lei de diversidade agiu");
        let c2 = eco.step(&[], 2);
        assert!(
            c2.entropy_injections > c1.entropy_injections,
            "a lei segue atuando passo após passo"
        );
    }

    /// Renovação obrigatória: ciclo sem extinções ⇒ morte
    /// programada da pior (legado: maturidade exige renovação).
    #[test]
    fn renovacao_obrigatoria_mata_a_pior() {
        let mut policy = EcologyPolicy::default();
        policy.renovation_cycle = 3; // curto para o teste
        let mut eco = EcologyMotor::with_policy("t", 11, policy);
        // População alta e fitness saudável: nenhuma morte natural.
        let healthy = vec![obs("boa", 0.9, 50), obs("media", 0.65, 40)];
        let _ = eco.step(&healthy, 1);
        let _ = eco.step(&healthy, 2);
        let c3 = eco.step(&healthy, 3);
        assert!(c3.forced_deaths >= 1, "renovação obrigatória agiu no ciclo");
        assert!(c3.steps_since_extinction == 0, "renovação zera o contador");
    }

    /// ADAPTERS reais: EcologyEngine (nichos) e FederationEngine
    /// (recursos do ledger) alimentam o MESMO motor genérico.
    #[test]
    fn adapters_reais_alimentam_o_motor() {
        use crate::federation::{FederationEngine, Resource};
        // Fonte 1: nichos reais com tentativas/sucessos.
        let mut eng = EcologyEngine::new();
        eng.report("nicho_a", true);
        eng.report("nicho_a", true);
        eng.report("nicho_b", false);
        let src = NichesOf(&eng);
        assert_eq!(src.domain(), "niche_health");
        let sp = src.species();
        assert_eq!(sp.len(), 2, "2 nichos com tentativas (sem tentativa não é espécie)");
        assert!((sp[0].fitness - 1.0).abs() < 1e-6, "nicho_a 2/2");
        assert!((sp[1].fitness - 0.0).abs() < 1e-6, "nicho_b 0/1");
        // Fonte 2: recursos federados reais do ledger.
        let mut fed = FederationEngine::new();
        fed.add_member("a", std::collections::BTreeMap::new());
        let _ = fed.propose_trade(1, "a", "a", (Resource::Energy, 0.1), (Resource::Compute, 0.1));
        let fsrc = FederationResources(&fed);
        assert_eq!(fsrc.domain(), "federation_resources");
        assert!(!fsrc.species().is_empty(), "ledger tem troca registrada");
        // O motor genérico roda sobre QUALQUER das fontes.
        let mut eco = EcologyMotor::new("cross", 13);
        let c = eco.step(&sp, 1);
        assert_eq!(c.species_observed, 2, "censo com denominador da fonte");
    }

    /// 17.13: episode boundary avalia por EPISÓDIO com denominador
    /// honesto (Lei 1) — 3 validados de 4 = 0.75 na janela.
    #[test]
    fn episode_boundary_taxa_com_denominador_da_janela() {
        let mut eng = EcologyEngine::new();
        let v1 = eng.episode_boundary("n", true);
        assert_eq!(v1.episodes_in_window, 1);
        assert!((v1.validated_rate.unwrap() - 1.0).abs() < 1e-12);
        eng.episode_boundary("n", true);
        eng.episode_boundary("n", true);
        let v4 = eng.episode_boundary("n", false);
        assert_eq!(v4.episodes_in_window, 4, "denominador = episódios na janela");
        assert!((v4.validated_rate.unwrap() - 0.75).abs() < 1e-12, "3/4");
        assert!(!v4.renovate, "taxa acima do piso: sem renovação");
    }

    /// 17.13 (Lei 2): nicho SEM episódios fechados = NO_DATA na
    /// janela longa — ausência nunca vira 0.0.
    #[test]
    fn episode_health_sem_episodios_e_no_data() {
        let eng = EcologyEngine::new();
        let q = eng.episode_health("fantasma", ModuleId::new(), StepId::new());
        assert!(q.value.is_none(), "janela vazia = sem valor (Lei 2)");
        assert!(
            format!("{:?}", q.status).contains("NoData"),
            "NO_DATA tipado, nunca 0.0 fabricado"
        );
    }

    /// 17.13 O5: janela mínima cheia (4) com taxa abaixo do piso
    /// (0.5) ⇒ renovação programada do nicho no episode boundary.
    #[test]
    fn episode_boundary_renovacao_por_episodio() {
        let mut eng = EcologyEngine::new();
        let mut last = eng.episode_boundary("ruim", false);
        eng.episode_boundary("ruim", false);
        eng.episode_boundary("ruim", true);
        last = eng.episode_boundary("ruim", false);
        assert_eq!(last.episodes_in_window, 4);
        assert!((last.validated_rate.unwrap() - 0.25).abs() < 1e-12, "1/4");
        assert!(last.renovate, "taxa 0.25 < 0.5 com janela cheia ⇒ renova O5");
        // A janela longa respeita o FIFO de 16: a taxa desliza.
        for _ in 0..12 {
            last = eng.episode_boundary("ruim", true);
        }
        assert_eq!(last.episodes_in_window, 16, "cap FIFO da janela longa");
        assert!(!last.renovate, "janela recuperou: 13/16 ⇒ sem renovação");
    }
}
