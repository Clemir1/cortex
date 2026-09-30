//! L3 REPRESENTAÇÃO REAL (17.5) — canônico do CAMADA.txt:355-495:
//! 1. SemanticRepresentation, 2. SemanticBinding, 3. ConceptStore,
//! 6. OntogeneticMemory (consolidação/reconsolidação JANELADA — a
//! trilha 16.2 é o embrião), 11. MetaestabilityLocal (atratores
//! locais) e 13. TemporalRepresentation (ponte HOTM/Reservoir→
//! semântica — "essa ponte é fundamental", CAMADA.txt:495).
//! Tudo com contadores COM DENOMINADOR e vereditos tipados.

use std::collections::HashMap;

use triad_foundation as tf;

/// Janela de consolidação (steps): reforço dentro da janela
/// consolida a traço; fora, decai SEM virar zero (Lei 2).
pub const CONSOLIDATION_WINDOW: u64 = 5;
/// Ativação mínima para atrator local (MetaestabilityLocal).
pub const ATTRACTOR_THRESHOLD: f32 = 0.5;
/// Decaimento da ativação por tick (representação viva).
pub const ACTIVATION_DECAY: f32 = 0.95;

/// 3. ConceptStore (CAMADA.txt:391): estado canônico dos conceitos
/// — armazenamento, não módulo cognitivo. Dedupe por rótulo.
#[derive(Debug, Default)]
pub struct ConceptStore {
    by_label: HashMap<String, tf::id::ConceptId>,
    records: HashMap<tf::id::ConceptId, ConceptRecord>,
    inserts: u64,
    dedupes: u64,
}

/// Registro canônico de um conceito.
#[derive(Debug, Clone, PartialEq)]
pub struct ConceptRecord {
    pub label: String,
    /// Representação viva (ativação/coherence/grounding).
    pub representation: SemanticRepresentation,
    pub created_step: u64,
}

/// 1. SemanticRepresentation (CAMADA.txt:360): activation,
/// coherence, grounding — o "isto representa alguma coisa".
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SemanticRepresentation {
    /// Ativação corrente [0,1] (decai por tick, sobe com uso).
    pub activation: f32,
    /// Coerência interna [0,1] (estabilidade da representação).
    pub coherence: f32,
    /// Grounding: ligada a fluxo real (temporal/experiência)?
    pub grounded: bool,
}

impl SemanticRepresentation {
    /// Nova representação com ativação inicial e coerência dada.
    pub fn new(activation: f32, coherence: f32) -> Self {
        Self {
            activation: activation.clamp(0.0, 1.0),
            coherence: coherence.clamp(0.0, 1.0),
            grounded: false,
        }
    }

    /// Decai a ativação um tick (nunca abaixo de 0; ausência ≠ zero
    /// — a representação continua no store).
    pub fn decay(&mut self) {
        self.activation *= ACTIVATION_DECAY;
    }

    /// Uso eleva a ativação e ganha coerência (média móvel).
    pub fn activate(&mut self, boost: f32) {
        self.activation = (self.activation + boost).clamp(0.0, 1.0);
        self.coherence = (self.coherence * 0.9 + 0.1).clamp(0.0, 1.0);
    }
}

/// Motivo tipado da janela ontogenética (ausência ≠ zero).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceVerdict {
    /// Reforço dentro da janela: traço consolidada.
    Consolidated,
    /// Acesso a memória consolidada reabriu a janela e fechou com
    /// reforço — reconsolidação.
    Reconsolidated,
    /// Janela venceu sem reforço: traço decai (contada, não zero).
    TraceDecayed,
    /// Janela ainda aberta.
    WindowOpen,
}

/// 6. OntogeneticMemory (CAMADA.txt:414): memória ligada à
/// experiência local com CONSOLIDAÇÃO/RECONSOLIDAÇÃO JANELADA.
#[derive(Debug, Clone, PartialEq)]
pub struct OntoTrace {
    pub label: String,
    pub strength: f32,
    pub window_until: u64,
    pub consolidated: bool,
}

/// Contadores ontogenéticos COM DENOMINADOR.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct OntoStats {
    pub eventos: u64,
    pub consolidadas: u64,
    pub reconsolidadas: u64,
    pub decaidas: u64,
    pub abertas: u64,
}

/// Memória ontogenética janelada (embrião: trilha 16.2).
#[derive(Debug, Default)]
pub struct OntogeneticMemory {
    traces: HashMap<String, OntoTrace>,
    pub stats: OntoStats,
}

impl OntogeneticMemory {
    /// Evento de experiência: reforça traço existente ou abre
    /// traço nova com janela de consolidação.
    pub fn experience(&mut self, label: &str, step: u64) -> TraceVerdict {
        self.stats.eventos += 1;
        if let Some(t) = self.traces.get_mut(label) {
            t.strength = (t.strength + 0.2).clamp(0.0, 1.0);
            if t.consolidated {
                // Reconsolidação: acesso reabre a janela e o reforço
                // fecha com força mantida (tipado).
                t.window_until = step + CONSOLIDATION_WINDOW;
                self.stats.reconsolidadas += 1;
                TraceVerdict::Reconsolidated
            } else if step <= t.window_until {
                t.consolidated = true;
                self.stats.consolidadas += 1;
                TraceVerdict::Consolidated
            } else {
                t.window_until = step + CONSOLIDATION_WINDOW;
                self.stats.abertas += 1;
                TraceVerdict::WindowOpen
            }
        } else {
            self.traces.insert(
                label.to_string(),
                OntoTrace {
                    label: label.to_string(),
                    strength: 0.2,
                    window_until: step + CONSOLIDATION_WINDOW,
                    consolidated: false,
                },
            );
            self.stats.abertas += 1;
            TraceVerdict::WindowOpen
        }
    }

    /// Fecha janelas vencidas (tick): traços não consolidadas
    /// DECÁEM (força −0.05) e são removidas quando ≤0 — contadas
    /// tipadas, nunca sumidas.
    pub fn close_windows(&mut self, step: u64) -> u64 {
        let mut decaidas_agora = 0u64;
        let mut remover: Vec<String> = Vec::new();
        for (label, t) in self.traces.iter_mut() {
            if !t.consolidated && step > t.window_until {
                t.strength -= 0.05;
                decaidas_agora += 1;
                if t.strength <= 0.0 {
                    remover.push(label.clone());
                }
            }
        }
        for l in remover {
            self.traces.remove(&l);
        }
        self.stats.decaidas += decaidas_agora;
        decaidas_agora
    }

    /// Traços vivas (consolidadas + janela aberta) — denominador.
    pub fn live_count(&self) -> (usize, usize) {
        let consolidadas = self
            .traces
            .values()
            .filter(|t| t.consolidated)
            .count();
        (consolidadas, self.traces.len())
    }
}

/// 2. SemanticBinding (CAMADA.txt:377): liga representações/
/// conceitos/contextos — aresta com razão explícita.
#[derive(Debug, Clone, PartialEq)]
pub struct BindingEdge {
    pub a: tf::id::ConceptId,
    pub b: tf::id::ConceptId,
    pub strength: f32,
    pub reason: String,
}

/// Arestas de binding com denominador.
#[derive(Debug, Default)]
pub struct SemanticBinding {
    edges: Vec<BindingEdge>,
    rejected: BTreeMapLite,
}

/// Contador simples de rejeições por motivo.
#[derive(Debug, Default)]
pub struct BTreeMapLite(std::collections::BTreeMap<&'static str, u64>);

impl SemanticBinding {
    /// Liga dois conceitos (recusa aresta circular e força fora de
    /// faixa — tipadas COM razão).
    pub fn bind(
        &mut self,
        a: tf::id::ConceptId,
        b: tf::id::ConceptId,
        strength: f32,
        reason: &str,
    ) -> Result<(), &'static str> {
        if a == b {
            self.rejected.0.insert("SelfBinding", 1);
            return Err("aresta circular");
        }
        if !strength.is_finite() || !(0.0..=1.0).contains(&strength) {
            self.rejected.0.insert("BadStrength", 1);
            return Err("fora de faixa");
        }
        if reason.trim().is_empty() {
            self.rejected.0.insert("NoReason", 1);
            return Err("razão vazia");
        }
        self.edges.push(BindingEdge {
            a,
            b,
            strength,
            reason: reason.to_string(),
        });
        Ok(())
    }

    /// (arestas, rejeições por motivo) — denominadores.
    pub fn stats(&self) -> (usize, &std::collections::BTreeMap<&'static str, u64>) {
        (self.edges.len(), &self.rejected.0)
    }
}

/// 13. TemporalRepresentation (CAMADA.txt:486): ponte
/// HOTM/Reservoir→semântica — estados temporais RECORRENTES
/// ligados a conceitos. A recorrência é por assinatura
/// quantizada (banda × nível de ressonância em 8 passos).
#[derive(Debug, Default)]
pub struct TemporalRepresentation {
    seen: HashMap<String, u32>,
    pub ligacoes: u64,
    pub observacoes: u64,
}

impl TemporalRepresentation {
    /// Quantiza a ressonância em 8 níveis (0..7).
    pub fn quantize(resonance: f32) -> u8 {
        (resonance.clamp(0.0, 1.0) * 7.999) as u8
    }

    /// Observa uma assinatura de ressonância do substrato
    /// (HOTM); a partir da 2ª ocorrência liga um conceito
    /// "temporal:{banda}:{nível}" — a ponte real.
    pub fn observe(&mut self, sig: &crate::memory::ResonanceSignature) -> Option<String> {
        self.observacoes += 1;
        let key = format!(
            "temporal:{}:{}",
            sig.band.as_str(),
            Self::quantize(sig.resonance)
        );
        let n = self.seen.entry(key.clone()).or_insert(0);
        *n += 1;
        if *n == 2 {
            self.ligacoes += 1;
            Some(key)
        } else {
            None
        }
    }

    /// Ocorrências de uma chave (prova de recorrência).
    pub fn seen_count(&self, key: &str) -> u32 {
        self.seen.get(key).copied().unwrap_or(0)
    }
}

/// 11. MetaestabilityLocal (CAMADA.txt:464): atratores locais —
/// ativações que persistem acima do limiar.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MetaReport {
    pub n_conceitos: usize,
    pub n_atratores: usize,
    pub estabilidade_media: f32,
}

/// Detecta atratores sobre as ativações do campo — denominador =
/// n de conceitos com representação viva.
pub fn metaestability(store: &ConceptStore) -> MetaReport {
    let reps: Vec<&ConceptRecord> = store.records.values().collect();
    if reps.is_empty() {
        return MetaReport::default();
    }
    let n_atratores = reps
        .iter()
        .filter(|r| r.representation.activation >= ATTRACTOR_THRESHOLD)
        .count();
    let estabilidade = reps
        .iter()
        .map(|r| r.representation.coherence)
        .sum::<f32>()
        / reps.len() as f32;
    MetaReport {
        n_conceitos: reps.len(),
        n_atratores,
        estabilidade_media: estabilidade,
    }
}

impl ConceptStore {
    /// Novo store vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insere (ou dedupe por rótulo) um conceito — denominadores
    /// de inserts/dedupes.
    pub fn insert(
        &mut self,
        label: &str,
        step: u64,
        activation: f32,
    ) -> tf::id::ConceptId {
        if let Some(id) = self.by_label.get(label) {
            self.dedupes += 1;
            if let Some(r) = self.records.get_mut(id) {
                r.representation.activate(0.1);
            }
            return *id;
        }
        self.inserts += 1;
        let id = tf::id::ConceptId::new();
        self.by_label.insert(label.to_string(), id);
        self.records.insert(
            id,
            ConceptRecord {
                label: label.to_string(),
                representation: SemanticRepresentation::new(activation, 0.2),
                created_step: step,
            },
        );
        id
    }

    /// Marca o grounding de um conceito (ligou a fluxo real).
    pub fn ground(&mut self, id: &tf::id::ConceptId) -> bool {
        match self.records.get_mut(id) {
            Some(r) => {
                r.representation.grounded = true;
                r.representation.activate(0.2);
                true
            }
            None => false,
        }
    }

    /// Registro de um conceito.
    pub fn record(&self, id: &tf::id::ConceptId) -> Option<&ConceptRecord> {
        self.records.get(id)
    }

    /// Por rótulo.
    pub fn by_label(&self, label: &str) -> Option<&ConceptRecord> {
        self.by_label.get(label).and_then(|id| self.records.get(id))
    }

    /// (inserts, dedupes) — denominador de criação.
    pub fn insert_counts(&self) -> (u64, u64) {
        (self.inserts, self.dedupes)
    }

    /// Decai todas as ativações um tick (representação viva).
    pub fn decay_all(&mut self) {
        for r in self.records.values_mut() {
            r.representation.decay();
        }
    }
}

/// Agregador da representação L3 (montagem única no L3Module):
/// ConceptStore + Binding + OntogeneticMemory + Temporal.
pub struct L3Representation {
    pub store: ConceptStore,
    pub binding: SemanticBinding,
    pub onto: OntogeneticMemory,
    pub temporal: TemporalRepresentation,
}

impl Default for L3Representation {
    fn default() -> Self {
        Self::new()
    }
}

impl L3Representation {
    /// Nova representação vazia.
    pub fn new() -> Self {
        Self {
            store: ConceptStore::new(),
            binding: SemanticBinding::default(),
            onto: OntogeneticMemory::default(),
            temporal: TemporalRepresentation::default(),
        }
    }

    /// Tick da representação: decays + fechamento de janelas
    /// ontogenéticas. Devolve traços decaídas no tick (contada).
    pub fn tick(&mut self, step: u64) -> u64 {
        self.store.decay_all();
        self.onto.close_windows(step)
    }

    /// Observa o sinal Chladni do substrato (ponte HOTM→semântica):
    /// recorrência liga conceito groundado no store.
    pub fn observe_chladni(
        &mut self,
        sig: &crate::memory::ResonanceSignature,
        step: u64,
    ) -> Option<tf::id::ConceptId> {
        let key = self.temporal.observe(sig)?;
        let id = self.store.insert(&key, step, 0.6);
        self.store.ground(&id);
        Some(id)
    }

    /// 18.5: censo ecológico dos conceitos — (rótulo, ativação,
    /// população 1) por conceito vivo. Adapter SYMBOLIC do
    /// `EcologyMotor` (T/governance), ordenado por nicho
    /// (determinismo). Fitness bruta = ativação corrente [0,1].
    pub fn ecology_species(&self) -> Vec<(String, f32, u64)> {
        let mut out: Vec<(String, f32, u64)> = self
            .store
            .records
            .values()
            .map(|r| (r.label.clone(), r.representation.activation, 1))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    /// Estatística completa COM DENOMINADORES (para o app).
    pub fn report(&self) -> RepReport {
        let (inserts, dedupes) = self.store.insert_counts();
        let (edges, _) = self.binding.stats();
        let (consolidadas, vivas) = self.onto.live_count();
        let meta = metaestability(&self.store);
        RepReport {
            conceitos: meta.n_conceitos,
            inserts,
            dedupes,
            bindings: edges,
            traces_consolidadas: consolidadas,
            traces_vivas: vivas,
            eventos: self.onto.stats.eventos,
            eventos_consolidados: self.onto.stats.consolidadas,
            reconsolidadas: self.onto.stats.reconsolidadas,
            decaidas: self.onto.stats.decaidas,
            ligacoes_temporais: self.temporal.ligacoes,
            observacoes_temporais: self.temporal.observacoes,
            atratores: meta.n_atratores,
            estabilidade_media: meta.estabilidade_media,
        }
    }

    /// 18.7 — contribuição ao hash observacional da camada L3
    /// (padrão da sessão 6: função pura do estado canônico,
    /// nunca wall-clock). Alimenta o hasher com o CONTEÚDO real
    /// em ordem canônica: registros ordenados por RÓTULO
    /// (independe da ordem de inserção do HashMap), ativação em
    /// BITS (determinismo total de f32), contadores INTEIROS do
    /// binding/onto/temporal. Ausência de estado = nada escrito
    /// — quem decide None × Some é o dono do tick (L3Module).
    pub fn feed_hash(&self, h: &mut impl std::hash::Hasher) {
        use std::hash::Hash;
        // Registros em ordem canônica de rótulo.
        let mut pares: Vec<(&String, f32, bool, u64)> = self
            .store
            .records
            .values()
            .map(|r| {
                (
                    &r.label,
                    r.representation.activation,
                    r.representation.grounded,
                    r.created_step,
                )
            })
            .collect();
        pares.sort_by(|a, b| a.0.cmp(b.0));
        h.write_usize(pares.len());
        for (label, activation, grounded, created) in pares {
            label.hash(h);
            h.write_u32(activation.to_bits());
            h.write_u8(grounded as u8);
            h.write_u64(created);
        }
        // Contadores inteiros (denominadores vivos, sem f32
        // ambíguo: tudo o que é contável entra como inteiro).
        let (inserts, dedupes) = self.store.insert_counts();
        let (edges, _) = self.binding.stats();
        let (consolidadas, vivas) = self.onto.live_count();
        h.write_u64(inserts);
        h.write_u64(dedupes);
        h.write_u64(edges as u64);
        h.write_u64(consolidadas as u64);
        h.write_u64(vivas as u64);
        h.write_u64(self.onto.stats.eventos);
        h.write_u64(self.temporal.ligacoes);
        h.write_u64(self.temporal.observacoes);
    }
}

/// Fotografia COM DENOMINADORES da representação L3.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RepReport {
    pub conceitos: usize,
    pub inserts: u64,
    pub dedupes: u64,
    pub bindings: usize,
    /// Traços consolidadas VIVAS agora (fotografia).
    pub traces_consolidadas: usize,
    /// Traços vivas (consolidadas + janela aberta).
    pub traces_vivas: usize,
    pub eventos: u64,
    /// Vereditos Consolidated emitidos (histórico).
    pub eventos_consolidados: u64,
    /// Vereditos Reconsolidated emitidos (histórico).
    pub reconsolidadas: u64,
    pub decaidas: u64,
    pub ligacoes_temporais: u64,
    pub observacoes_temporais: u64,
    pub atratores: usize,
    pub estabilidade_media: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concept_store_dedupe_com_denominador() {
        let mut s = ConceptStore::new();
        let a = s.insert("casa", 1, 0.5);
        let b = s.insert("casa", 2, 0.5);
        assert_eq!(a, b, "dedupe por rótulo");
        let _ = s.insert("porta", 3, 0.2);
        let (ins, ded) = s.insert_counts();
        assert_eq!((ins, ded), (2, 1), "denominador explícito");
        assert!(s.by_label("casa").is_some());
    }

    #[test]
    fn ontogenetica_janelada_tipada() {
        let mut m = OntogeneticMemory::default();
        assert_eq!(m.experience("choque", 10), TraceVerdict::WindowOpen);
        // Reforço DENTRO da janela consolida.
        assert_eq!(m.experience("choque", 12), TraceVerdict::Consolidated);
        // Acesso consolidado reabre: reconsolidação tipada.
        assert_eq!(m.experience("choque", 30), TraceVerdict::Reconsolidated);
        // Traço sem reforço decai quando a janela vence.
        m.experience("esquecida", 10);
        let decaidas = m.close_windows(20);
        assert!(decaidas >= 1, "janela vencida decai contada");
        let (consolidadas, vivas) = m.live_count();
        assert_eq!(consolidadas, 1);
        assert_eq!(vivas, 2, "consolidada + esquecida ainda viva");
        assert_eq!(m.stats.eventos, 4, "denominador");
        assert_eq!(m.stats.consolidadas, 1);
        assert_eq!(m.stats.reconsolidadas, 1);
    }

    #[test]
    fn binding_recusa_tipada_e_aceita_com_razao() {
        let mut sb = SemanticBinding::default();
        let a = tf::id::ConceptId::new();
        let b = tf::id::ConceptId::new();
        assert!(sb.bind(a, a, 0.5, "circular").is_err());
        assert!(sb.bind(a, b, 2.0, "fora").is_err());
        assert!(sb.bind(a, b, 0.5, "  ").is_err());
        assert!(sb.bind(a, b, 0.5, "co-ocorrência temporal").is_ok());
        let (n, reje) = sb.stats();
        assert_eq!(n, 1);
        assert_eq!(reje.get("SelfBinding"), Some(&1));
        assert_eq!(reje.get("BadStrength"), Some(&1));
        assert_eq!(reje.get("NoReason"), Some(&1));
    }

    #[test]
    fn temporal_ponte_liga_na_recorrencia() {
        let mut t = TemporalRepresentation::default();
        let band = triad_chladni::CognitiveBand::Memory;
        let sig = crate::memory::ResonanceSignature {
            band,
            resonance: 0.55,
            sample_size: 8,
        };
        assert!(t.observe(&sig).is_none(), "1ª ocorrência só registra");
        let key = t.observe(&sig).expect("2ª liga conceito");
        assert!(key.starts_with("temporal:"));
        assert_eq!(t.seen_count(&key), 2);
        assert_eq!(t.ligacoes, 1);
        assert_eq!(t.observacoes, 2, "denominador");
        // Quantização estável (A/A).
        assert_eq!(TemporalRepresentation::quantize(0.55), 4);
        assert_eq!(
            TemporalRepresentation::quantize(0.55),
            TemporalRepresentation::quantize(0.55)
        );
    }

    #[test]
    fn metaestabilidade_atratores_com_denominador() {
        let mut s = ConceptStore::new();
        let _alto = s.insert("alto", 1, 0.9);
        let _baixo = s.insert("baixo", 2, 0.1);
        let rep = metaestability(&s);
        assert_eq!(rep.n_conceitos, 2, "denominador");
        assert_eq!(rep.n_atratores, 1, "só o alto é atrator");
        // Vazio: denominador zero honrado (ausência ≠ zero).
        assert_eq!(metaestability(&ConceptStore::new()).n_conceitos, 0);
    }

    #[test]
    fn representacao_decai_e_nunca_zera_fantasma() {
        let mut r = SemanticRepresentation::new(1.0, 0.5);
        for _ in 0..10 {
            r.decay();
        }
        assert!(r.activation > 0.0, "decai assintótico, nunca zera de vez");
        r.activate(1.0);
        assert_eq!(r.activation, 1.0);
        assert!(r.coherence > 0.5, "uso ganha coerência");
    }
}
