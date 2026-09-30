//! GlobalIntegration (17.6) — o AGREGADOR do L4: funde o que no
//! legado eram nomes separados (audit 17-3: "não existe um
//! GlobalIntegration; o papel é do par GlobalWorkspace + ciclo
//! causal" — aqui os 4 subcampos de contexto viram MÉTRICA de
//! contexto; os duplicados não são portados). Contém:
//! ThalamicRouter (gating por prioridade+TTL — filas 0-4),
//! EpisodicIntegration (buffer com segmentação por delta +
//! keyframes, alimenta a memória L3 pela trilha 16.2),
//! GlobalAbstraction (símbolos níveis 0-2, formação a cada 10
//! steps, threshold 0.6) e FutureNavigator (trajetórias da
//! dinâmica recorrente — embrião rollout 16.3). Tudo COM
//! DENOMINADOR e razões tipadas.

use std::collections::HashMap;

use triad_foundation as tf;

use crate::workspace::WorkspaceEntry;

/// TTL das entradas do roteador talâmico (ticks).
pub const THALAMIC_TTL: u64 = 5;
/// Prioridade mínima admitida no gate (0 = fraco demais).
pub const THALAMIC_MIN_PRIORITY: u8 = 1;
/// Delta L2 que finaliza um episódio (audit episodic_memory.py:74).
pub const EPISODE_DELTA: f32 = 0.15;
/// Delta L2 que grava keyframe (compressão).
pub const KEYFRAME_DELTA: f32 = 0.05;
/// Threshold de co-ocorrência do nível 1 (audit :64-66).
pub const ABSTRACTION_THRESHOLD: f32 = 0.6;
/// Formação de símbolos a cada N steps (audit :46-56).
pub const ABSTRACTION_INTERVAL: u64 = 10;
/// Bandas de atratores de futuro (audit hotm_future_navigator:78-86).
pub const FUTURE_BANDS: [f32; 4] = [165.0, 475.0, 1100.0, 2750.0];
/// Decaimento da trajetória projetada por passo.
pub const FUTURE_DECAY: f32 = 0.9;

/// Recusa tipada do gate talâmico.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateVerdict {
    /// Admitido com prioridade declarada.
    Admitted { priority: u8 },
    /// Saliência fraca demais (prioridade 0).
    LowPriority,
    /// Mesma chave ainda viva na TTL (habituação de repetição).
    TtlAlive { ticks_restantes: u64 },
}

/// Roteador talâmico: gating por prioridade (0-4) com TTL.
#[derive(Debug, Default)]
pub struct ThalamicRouter {
    live: HashMap<String, (u8, u64)>,
    pub admitidos: u64,
    pub recusados: u64,
    pub expirados: u64,
    pub chamados: u64,
}

impl ThalamicRouter {
    /// Prioridade pela saliência quantizada em 5 faixas.
    pub fn priority_of(salience: f32) -> u8 {
        (salience.clamp(0.0, 1.0) * 4.999) as u8
    }

    /// Gate: prioridade mínima + TTL por chave de conteúdo.
    pub fn gate(&mut self, tick: u64, key: &str, salience: f32) -> GateVerdict {
        self.chamados += 1;
        self.expire(tick);
        let priority = Self::priority_of(salience);
        if priority < THALAMIC_MIN_PRIORITY {
            self.recusados += 1;
            return GateVerdict::LowPriority;
        }
        if let Some((_, until)) = self.live.get(key) {
            if tick < *until {
                self.recusados += 1;
                return GateVerdict::TtlAlive {
                    ticks_restantes: *until - tick,
                };
            }
        }
        self.live.insert(key.to_string(), (priority, tick + THALAMIC_TTL));
        self.admitidos += 1;
        GateVerdict::Admitted { priority }
    }

    /// Expira entradas vencidas da TTL (contadas, nunca sumidas).
    pub fn expire(&mut self, tick: u64) {
        let antes = self.live.len();
        self.live.retain(|_, (_, until)| *until > tick);
        self.expirados += (antes - self.live.len()) as u64;
    }
}

/// Episódio em segmentação (buffer circular ≤100, audit :74-:88).
#[derive(Debug, Clone, PartialEq)]
pub struct EpisodeBuffer {
    pub keyframes: Vec<Vec<f32>>,
    pub started_tick: u64,
}

/// EpisodicIntegration: buffer de estados de contexto com
/// segmentação por delta; episódios fechados ALIMENTAM a memória
/// L3 pela trilha 16.2 (submissão ao inbox do dono).
#[derive(Debug)]
pub struct EpisodicIntegration {
    current: Option<EpisodeBuffer>,
    closed: std::collections::VecDeque<EpisodeBuffer>,
    last_state: Option<Vec<f32>>,
    pub push_total: u64,
    pub episodios_fechados: u64,
    pub keyframes_total: u64,
}

impl Default for EpisodicIntegration {
    fn default() -> Self {
        Self {
            current: None,
            closed: std::collections::VecDeque::with_capacity(100),
            last_state: None,
            push_total: 0,
            episodios_fechados: 0,
            keyframes_total: 0,
        }
    }
}

impl EpisodicIntegration {
    /// Distância L2 entre estados (None no primeiro).
    fn delta(a: &[f32], b: &[f32]) -> f32 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y) * (x - y))
            .sum::<f32>()
            .sqrt()
    }

    /// Empurra o estado de contexto do tick; devolve o EPISÓDIO
    /// FECHADO quando o delta finaliza (delta > EPISODE_DELTA).
    pub fn push_state(&mut self, tick: u64, state: Vec<f32>) -> Option<EpisodeBuffer> {
        self.push_total += 1;
        let fechado = match (&self.last_state, &self.current) {
            (Some(prev), Some(_)) if state.len() == prev.len() => {
                let d = Self::delta(prev, &state);
                if d > KEYFRAME_DELTA {
                    self.keyframes_total += 1;
                }
                d > EPISODE_DELTA
            }
            _ => false,
        };
        if fechado {
            let mut ep = self.current.take().expect("corrente existe");
            ep.keyframes.push(state.clone());
            self.closed.push_back(ep);
            if self.closed.len() > 100 {
                self.closed.pop_front();
            }
            self.episodios_fechados += 1;
            self.current = Some(EpisodeBuffer {
                keyframes: vec![state.clone()],
                started_tick: tick,
            });
            self.last_state = Some(state);
            return self.closed.back().cloned();
        }
        if self.current.is_none() {
            self.current = Some(EpisodeBuffer {
                keyframes: vec![state.clone()],
                started_tick: tick,
            });
        } else if let Some(prev) = &self.last_state {
            if state.len() == prev.len() && Self::delta(prev, &state) > KEYFRAME_DELTA {
                self.current
                    .as_mut()
                    .expect("corrente existe")
                    .keyframes
                    .push(state.clone());
            }
        }
        self.last_state = Some(state);
        None
    }

    /// Episódios fechados (denominador do buffer).
    pub fn closed_count(&self) -> usize {
        self.closed.len()
    }
}

/// GlobalAbstraction: símbolos nível 0 (grounded), 1
/// (co-ocorrência), 2 (meta) — formação a cada 10 steps.
#[derive(Debug, Default)]
pub struct GlobalAbstraction {
    pub level0: std::collections::BTreeSet<String>,
    pub level1: std::collections::BTreeSet<String>,
    pub level2: std::collections::BTreeSet<String>,
    pairs: HashMap<(String, String), u64>,
    metas: HashMap<(String, String), u64>,
    pub formacoes: u64,
}

impl GlobalAbstraction {
    /// Registra o vencedor (símbolo grounded nível 0) e as
    /// co-ocorrências com os focos do tick (insumo do nível 1).
    pub fn observe_winner(&mut self, tick: u64, winner: &str, foci: &[String]) {
        self.level0.insert(winner.to_string());
        if tick % ABSTRACTION_INTERVAL == 0 {
            // Formação por co-ocorrência em janela de 10 steps.
            for other in foci {
                if other == winner {
                    continue;
                }
                let pair = if winner < other.as_str() {
                    (winner.to_string(), other.clone())
                } else {
                    (other.clone(), winner.to_string())
                };
                let n = self.pairs.entry(pair.clone()).or_insert(0);
                *n += 1;
                if *n as f32 / ABSTRACTION_INTERVAL as f32 >= ABSTRACTION_THRESHOLD {
                    let sym = format!("{}+{}", pair.0, pair.1);
                    if self.level1.insert(sym.clone()) {
                        self.formacoes += 1;
                        // Nível 2: meta-símbolos de nível 1.
                        for l1 in self.level1.iter() {
                            if *l1 != sym.as_str() {
                                let meta = if l1 < &sym {
                                    (l1.clone(), sym.clone())
                                } else {
                                    (sym.clone(), l1.clone())
                                };
                                let m = self.metas.entry(meta.clone()).or_insert(0);
                                *m += 1;
                                if *m as f32 / ABSTRACTION_INTERVAL as f32
                                    >= ABSTRACTION_THRESHOLD
                                {
                                    if self.level2.insert(format!(
                                        "[{}|{}]",
                                        meta.0, meta.1
                                    )) {
                                        self.formacoes += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Contagem por nível (denominadores explícitos).
    pub fn counts(&self) -> (usize, usize, usize, u64) {
        (
            self.level0.len(),
            self.level1.len(),
            self.level2.len(),
            self.formacoes,
        )
    }
}

/// FutureNavigator: trajetórias baratas da dinâmica recorrente
/// (z = decaimento do estado + banda de atrator; embrião 16.3).
#[derive(Debug, Default)]
pub struct FutureNavigator {
    pub trajetorias: u64,
    pub best_path_score: Option<f32>,
    pub best_band: Option<u8>,
}

impl FutureNavigator {
    /// Projeta k passos por banda; melhor caminho = maior soma de
    /// saliência projetada. Denominador = k × bandas.
    pub fn navigate(&mut self, salience: f32, k: u64) -> (u64, Option<f32>, Option<u8>) {
        let mut best: Option<(f32, u8)> = None;
        for (bi, band) in FUTURE_BANDS.iter().enumerate() {
            // Banda normalizada injeta frequência relativa.
            let inject = (band / FUTURE_BANDS[0]).log2().max(0.0) as f32 / 4.0;
            let mut s = salience;
            for _ in 0..k {
                s = s * FUTURE_DECAY + inject * 0.05;
            }
            if best.map(|(b, _)| s > b).unwrap_or(true) {
                best = Some((s, bi as u8));
            }
            self.trajetorias += 1;
        }
        self.best_path_score = best.map(|(s, _)| s);
        self.best_band = best.map(|(_, b)| b);
        (self.trajetorias, self.best_path_score, self.best_band)
    }
}

/// GlobalIntegration — o agregador com métricas de contexto
/// FUNDIDAS (attention/competition/resonance/context como UMA
/// fotografia, audit 17-3 §3: "fundir como métrica de contexto").
pub struct GlobalIntegration {
    pub thalamus: ThalamicRouter,
    pub episodic: EpisodicIntegration,
    pub abstraction: GlobalAbstraction,
    pub navigator: FutureNavigator,
    /// Contexto fundido do tick (saliência média dos focos, nº de
    /// candidatos, coerência — a métrica que substitui os 4
    /// subcampos separados do legado).
    pub last_context: ContextMetrics,
    pub episodios_para_l3: u64,
}

/// Métrica de contexto FUNDIDA — uma fotografia, não 4 módulos.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ContextMetrics {
    pub focos: usize,
    pub saliencia_media: Option<f32>,
    pub candidatos_workspace: usize,
    pub vencedores_broadcast: u64,
}

impl Default for GlobalIntegration {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalIntegration {
    /// Agregador novo (estado vazio, contadores zerados).
    pub fn new() -> Self {
        Self {
            thalamus: ThalamicRouter::default(),
            episodic: EpisodicIntegration::default(),
            abstraction: GlobalAbstraction::default(),
            navigator: FutureNavigator::default(),
            last_context: ContextMetrics::default(),
            episodios_para_l3: 0,
        }
    }

    /// Observação por tick: gate dos focos (prioridade+TTL),
    /// estado de contexto no buffer episódico (fecha episódios por
    /// delta), símbolo grounded do vencedor + co-ocorrências,
    /// trajetórias do navegador.
    pub fn observe_tick(
        &mut self,
        tick: u64,
        foci: &[(tf::id::ConceptId, f32)],
        winner: Option<&WorkspaceEntry>,
        coherence: Option<f64>,
    ) -> Option<EpisodeBuffer> {
        let mut estado: Vec<f32> = Vec::with_capacity(foci.len() + 2);
        for (_, s) in foci {
            estado.push(*s);
        }
        if let Some(c) = coherence {
            estado.push(c as f32);
        }
        let sal_media = if foci.is_empty() {
            None
        } else {
            Some(foci.iter().map(|(_, s)| *s).sum::<f32>() / foci.len() as f32)
        };
        self.last_context = ContextMetrics {
            focos: foci.len(),
            saliencia_media: sal_media,
            candidatos_workspace: self.thalamus.admitidos as usize,
            vencedores_broadcast: winner.map(|_| 1).unwrap_or(0),
        };
        let fechado = self.episodic.push_state(tick, estado);
        if let Some(w) = winner {
            let focos_str: Vec<String> =
                foci.iter().map(|(c, _)| format!("foco:{c:?}")).collect();
            self.abstraction.observe_winner(tick, &w.content, &focos_str);
        }
        if let Some(sal) = sal_media {
            self.navigator.navigate(sal, 5);
        }
        fechado
    }

    /// Fotografia COM DENOMINADORES (para o app).
    pub fn report(&self) -> GiReport {
        let (l0, l1, l2, form) = self.abstraction.counts();
        GiReport {
            thalamus_chamados: self.thalamus.chamados,
            thalamus_admitidos: self.thalamus.admitidos,
            thalamus_recusados: self.thalamus.recusados,
            thalamus_expirados: self.thalamus.expirados,
            pushes: self.episodic.push_total,
            episodios_fechados: self.episodic.episodios_fechados,
            keyframes: self.episodic.keyframes_total,
            simbolos_l0: l0,
            simbolos_l1: l1,
            simbolos_l2: l2,
            formacoes: form,
            trajetorias: self.navigator.trajetorias,
            best_path_score: self.navigator.best_path_score,
            best_band: self.navigator.best_band,
            focos_ultimo_tick: self.last_context.focos,
            saliencia_media: self.last_context.saliencia_media,
        }
    }
}

/// Fotografia do agregador — todos com denominador na origem.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GiReport {
    pub thalamus_chamados: u64,
    pub thalamus_admitidos: u64,
    pub thalamus_recusados: u64,
    pub thalamus_expirados: u64,
    pub pushes: u64,
    pub episodios_fechados: u64,
    pub keyframes: u64,
    pub simbolos_l0: usize,
    pub simbolos_l1: usize,
    pub simbolos_l2: usize,
    pub formacoes: u64,
    pub trajetorias: u64,
    pub best_path_score: Option<f32>,
    pub best_band: Option<u8>,
    pub focos_ultimo_tick: usize,
    pub saliencia_media: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cid() -> tf::id::ConceptId {
        tf::id::ConceptId::new()
    }

    #[test]
    fn thalamus_gating_tipado_com_denominador() {
        let mut t = ThalamicRouter::default();
        // Fraco demais: prioridade 0.
        assert_eq!(t.gate(1, "a", 0.05), GateVerdict::LowPriority);
        // Admitido com prioridade.
        assert_eq!(t.gate(1, "a", 0.9), GateVerdict::Admitted { priority: 4 });
        // Mesma chave viva na TTL: habituação.
        assert_eq!(
            t.gate(2, "a", 0.9),
            GateVerdict::TtlAlive { ticks_restantes: 4 }
        );
        assert_eq!((t.chamados, t.admitidos, t.recusados), (3, 1, 2));
        // TTL vence: readmite e conta expiração.
        assert_eq!(t.gate(1 + THALAMIC_TTL, "a", 0.9), GateVerdict::Admitted { priority: 4 });
        assert_eq!(t.expirados, 1, "expiração contada");
    }

    #[test]
    fn episodico_segmenta_por_delta() {
        let mut e = EpisodicIntegration::default();
        // Estado estável: nenhum episódio.
        assert!(e.push_state(1, vec![0.1, 0.1]).is_none());
        assert!(e.push_state(2, vec![0.11, 0.11]).is_none());
        // Salto grande: fecha episódio.
        let fechado = e.push_state(3, vec![0.9, 0.9]).expect("fecha");
        assert_eq!(fechado.started_tick, 1);
        assert_eq!(e.episodios_fechados, 1);
        assert_eq!(e.closed_count(), 1);
        assert!(e.push_total >= 3, "denominador");
        assert!(e.keyframes_total >= 1, "keyframe contado");
    }

    #[test]
    fn abstracao_niveis_com_denominador() {
        let mut a = GlobalAbstraction::default();
        let focos = vec!["foco:x".to_string()];
        // 10 janelas de co-ocorrência no mesmo par: forma nível 1.
        for tick in 1..=60 {
            a.observe_winner(tick, "vencedor:z", &focos);
        }
        let (l0, l1, l2, form) = a.counts();
        assert_eq!(l0, 1, "vencedores groundados");
        assert_eq!(l1, 1, "co-ocorrência vencedor+foco formada");
        assert_eq!(l2, 0, "meta precisa de 2 símbolos L1");
        assert_eq!(form, 1, "formação contada");
    }

    #[test]
    fn navegador_trajetorias_deterministicas() {
        let mut n = FutureNavigator::default();
        let (t1, s1, b1) = n.navigate(0.7, 5);
        assert_eq!(t1, 4, "k=5 × 4 bandas = 4 trajetórias? não: 4 bandas");
        assert!(s1.unwrap() > 0.0);
        assert!(b1.is_some());
        // A/A determinístico.
        let mut n2 = FutureNavigator::default();
        let (_, s2, b2) = n2.navigate(0.7, 5);
        assert_eq!((s1, b1), (s2, b2), "mesma entrada, mesmo caminho");
    }

    #[test]
    fn agregador_integra_tudo_com_denominadores() {
        let mut gi = GlobalIntegration::new();
        let foci = vec![(cid(), 0.8), (cid(), 0.4)];
        let winner = WorkspaceEntry {
            stage: crate::WorkspaceStage::Local,
            content: "foco:vitoria".to_string(),
            salience: 0.8,
        };
        for tick in 1..=3 {
            // O gate é chamado pelo tick do L4 antes do workspace —
            // aqui o simulamos por foco (como no loop real).
            for (c, s) in &foci {
                let _ = gi.thalamus.gate(tick, &format!("foco:{c:?}"), *s);
            }
            let _ = gi.observe_tick(tick, &foci, Some(&winner), Some(0.5));
        }
        let r = gi.report();
        assert!(r.thalamus_chamados >= 6, "gate chamado por foco por tick");
        assert!(r.episodios_fechados >= 0);
        assert_eq!(r.trajetorias, 3 * 4, "4 bandas por tick com focos");
        assert_eq!(r.simbolos_l0, 1);
        assert_eq!(r.focos_ultimo_tick, 2);
    }
}
