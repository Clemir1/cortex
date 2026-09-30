//! Memória episódica local da camada L3.
//!
//! ADR-0007 (recursão Chladni): episódios podem carregar a ASSINATURA
//! de ressonância do substrato no tick da gravação (banda + ressonância
//! do sinal `l1.substrate.chladni`), e a similaridade entre episódios
//! pode ser medida por ressonância — o herdeiro direto do
//! `_compare_patterns` do legado, adaptado ao sinal qualificado novo.
//! Episódio SEM assinatura não participa da similaridade (ausência
//! nunca vira zero fantasma).

use triad_chladni as ch;
use triad_contracts as tc;
use triad_foundation as tf;
use tracing::{debug, trace};

use tf::id::{EpisodeId, ModuleId, StepId};

/// Idade máxima (em passos) para um episódio ser considerado válido.
pub const VALIDITY_WINDOW: u64 = 5;

/// Capacidade máxima de episódios na memória local.
const MAX_EPISODES: usize = 1024;

/// Assinatura de ressonância do substrato no tick da gravação —
/// fotografia qualificada do sinal Chladni (banda cognitiva da energia
/// média, ressonância da amostra, tamanho da amostra).
#[derive(Debug, Clone, PartialEq)]
pub struct ResonanceSignature {
    /// Banda cognitiva da frequência da energia média da amostra.
    pub band: ch::CognitiveBand,
    /// Ressonância média da amostra (features do estado × padrão).
    pub resonance: f32,
    /// Tamanho da amostra efetiva (denominador sempre visível).
    pub sample_size: u64,
}

impl ResonanceSignature {
    /// Constrói a assinatura de uma observação do sinal Chladni —
    /// `None` se o sinal não é VALUE (ausência nunca vira assinatura
    /// fantasma com banda/ressonância fabricadas).
    pub fn from_observation(obs: &ch::Observation) -> Option<Self> {
        let resonance = obs.resonance.as_ref_value().copied()?;
        let band = obs.band?;
        Some(ResonanceSignature {
            band,
            resonance,
            sample_size: obs.sample_size,
        })
    }
}

/// Episódio individual armazenado na memória local.
#[derive(Debug, Clone)]
pub struct Episode {
    /// Identificador único do episódio.
    pub id: EpisodeId,
    /// Rótulo textual do episódio.
    pub label: String,
    /// Passo em que o episódio foi registrado.
    pub step: u64,
    /// Força do episódio.
    pub strength: f32,
    /// Assinatura de ressonância do substrato no tick da gravação
    /// (None = gravado sem sinal — não participa da similaridade).
    pub resonance: Option<ResonanceSignature>,
}

/// Ordem canônica das bandas cognitivas (distância de similaridade).
fn band_rank(band: ch::CognitiveBand) -> i32 {
    match band {
        ch::CognitiveBand::Stability => 0,
        ch::CognitiveBand::Memory => 1,
        ch::CognitiveBand::Creativity => 2,
        ch::CognitiveBand::MetaOrganization => 3,
    }
}

/// Memória episódica local, consumida ao ser usada.
pub struct LocalMemory {
    episodes: Vec<Episode>,
}

impl LocalMemory {
    /// Cria uma memória local vazia.
    pub fn new() -> Self {
        Self { episodes: Vec::new() }
    }

    /// Registra episódio com força 1.0 (sem assinatura de ressonância);
    /// acima de 1024 remove o mais fraco.
    pub fn record(&mut self, label: &str, step: u64) {
        self.record_with_resonance(label, step, None);
    }

    /// Registra episódio com a assinatura de ressonância do tick
    /// (ADR-0007) — o consumidor decide o rótulo; a assinatura viaja
    /// com o episódio para similaridade futura.
    pub fn record_with_resonance(
        &mut self,
        label: &str,
        step: u64,
        resonance: Option<ResonanceSignature>,
    ) {
        self.episodes.push(Episode {
            id: EpisodeId::new(),
            label: label.to_string(),
            step,
            strength: 1.0,
            resonance,
        });
        debug!(label = label, step = step, "episodio gravado");
        if self.episodes.len() > MAX_EPISODES {
            self.evict_weakest();
        }
    }

    /// REFORÇO/RECONSOLIDAÇÃO (seção 16.2 — consumidor L4→L3): grava
    /// episódio do rótulo com força plena quando é o primeiro, ou
    /// RECONSOLIDIDA quando já existe episódio prévio do mesmo rótulo
    /// (strength nova = média entre a força prévia e o reforço pleno
    /// 1.0 — suavização determinística, sem números mágicos soltos).
    /// Devolve true quando reconsolidou um episódio prévio.
    pub fn reinforce(&mut self, label: &str, step: u64, reconsolidate: bool) -> bool {
        let previous = self
            .episodes
            .iter()
            .max_by_key(|e| e.step)
            .filter(|e| e.label == label)
            .map(|e| (e.strength, e.step));
        match previous {
            Some((strength, old_step)) if reconsolidate => {
                // Reconsolidação: o episódio antigo do rótulo cede
                // lugar ao novo com a força combinada.
                let merged = ((strength + 1.0) / 2.0).clamp(0.0, 1.0);
                self.episodes.retain(|e| !(e.label == label && e.step == old_step));
                self.episodes.push(Episode {
                    id: EpisodeId::new(),
                    label: label.to_string(),
                    step,
                    strength: merged,
                    resonance: None,
                });
                debug!(
                    label = label,
                    step = step,
                    forca_anterior = strength,
                    forca_nova = merged,
                    "episodio RECONSOLIDADO (reforco L4 confirmado)"
                );
                true
            }
            _ => {
                self.record(label, step);
                debug!(label = label, step = step, "episodio reforcado (novo)");
                false
            }
        }
    }

    /// Recupera a força do episódio pelo rótulo, respeitando a janela de validade.
    pub fn recall(
        &self,
        label: &str,
        current_step: u64,
        source: ModuleId,
        step_id: StepId,
    ) -> tc::Qualified<f32> {
        let Some(episode) = self.episodes.iter().find(|e| e.label == label) else {
            debug!(label = label, "recall sem episodio valido, retornando NO_DATA");
            return tc::Qualified::no_data("sem episódio", source, step_id);
        };
        let age = current_step.saturating_sub(episode.step);
        if age > VALIDITY_WINDOW {
            tc::Qualified::stale(source, step_id)
        } else {
            tc::Qualified::value(episode.strength, source, step_id)
        }
    }

    /// SIMILARIDADE POR RESSONÂNCIA (ADR-0007, herdeiro do
    /// `_compare_patterns` do legado): episódios com assinatura
    /// ranqueados contra a assinatura corrente. score = peso de banda
    /// (1/(1+Δbanda)) × (1 − min(|Δressonância|, 1)); sem assinatura no
    /// episódio, não participa — ausência nunca vira similaridade 0
    /// fabricada. Ordem estável (empate: mais recente primeiro).
    pub fn similar_by_resonance(
        &self,
        current: &ResonanceSignature,
    ) -> Vec<(Episode, f32)> {
        let mut scored: Vec<(usize, f32)> = self
            .episodes
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let sig = e.resonance.as_ref()?;
                let band_dist =
                    (band_rank(sig.band) - band_rank(current.band)).abs();
                let band_weight = 1.0 / (1.0 + band_dist as f32);
                let res_delta = (sig.resonance - current.resonance).abs().min(1.0);
                let score = (band_weight * (1.0 - res_delta)).clamp(0.0, 1.0);
                Some((i, score))
            })
            .collect();
        // Ordem determinística: score desc, índice desc (mais recente no empate).
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.0.cmp(&a.0))
        });
        trace!(candidatos = scored.len(), "similaridade por ressonância");
        scored
            .into_iter()
            .map(|(i, score)| (self.episodes[i].clone(), score))
            .collect()
    }

    /// Consome todos os episódios do rótulo; true se removeu pelo menos um.
    pub fn consume(&mut self, label: &str) -> bool {
        let before = self.episodes.len();
        self.episodes.retain(|e| e.label != label);
        if before != self.episodes.len() {
            trace!(label = label, "episodios consumidos");
        }
        before != self.episodes.len()
    }

    /// Remove o episódio de menor força; empate, o mais antigo (determinístico).
    fn evict_weakest(&mut self) {
        let weakest = self
            .episodes
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.strength.total_cmp(&b.1.strength))
            .map(|(i, _)| i);
        if let Some(index) = weakest {
            self.episodes.remove(index);
        }
    }

    /// 18.7 — contribuição ao hash observacional da camada L3
    /// (padrão da sessão 6): episódios em ORDEM DE INSERÇÃO
    /// (determinística dada a mesma sequência de reforços) com
    /// força em BITS (f32 determinístico) e o passo de criação.
    /// Ausência (zero episódios) = nada escrito além da contagem
    /// — quem decide None × Some é o dono do tick.
    pub fn feed_hash(&self, h: &mut impl std::hash::Hasher) {
        use std::hash::Hash;
        h.write_usize(self.episodes.len());
        for e in &self.episodes {
            e.label.hash(h);
            h.write_u64(e.step);
            h.write_u32(e.strength.to_bits());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig(band: ch::CognitiveBand, resonance: f32) -> ResonanceSignature {
        ResonanceSignature {
            band,
            resonance,
            sample_size: 4,
        }
    }

    #[test]
    fn reforco_novo_grava_e_reconsolidado_combina_forca() {
        let mut m = LocalMemory::new();
        // Primeiro reforço: episódio novo com força plena.
        assert!(!m.reinforce("foco:a", 10, true));
        let v = m.recall("foco:a", 11, ModuleId::new(), StepId::new());
        assert!(v.is_value());
        assert!((v.as_ref_value().copied().unwrap_or(0.0) - 1.0).abs() < 1e-6);
        // Segundo reforço com reconsolidação: força combinada = (1.0+1.0)/2.
        assert!(m.reinforce("foco:a", 12, true));
        let v2 = m.recall("foco:a", 13, ModuleId::new(), StepId::new());
        assert!(v2.is_value());
        assert!((v2.as_ref_value().copied().unwrap_or(0.0) - 1.0).abs() < 1e-6);
        // Sem reconsolidar: grava episódio novo (false) — política do L4.
        assert!(!m.reinforce("foco:b", 10, false));
        assert!(!m.reinforce("foco:b", 11, false));
        // Rótulos distintos coexistem.
        assert!(m.recall("foco:a", 12, ModuleId::new(), StepId::new()).is_value());
        assert!(m.recall("foco:b", 12, ModuleId::new(), StepId::new()).is_value());
    }

    #[test]
    fn recall_puro_continua_qualificado() {
        let mut m = LocalMemory::new();
        m.record("foco:a", 1);
        let v = m.recall("foco:a", 2, ModuleId::new(), StepId::new());
        assert!(v.is_value());
        // Janela de validade: episódio antigo é STALE, não sumiu.
        let s = m.recall("foco:a", 2 + VALIDITY_WINDOW + 1, ModuleId::new(), StepId::new());
        assert!(!s.is_value());
        // Rótulo ausente: NO_DATA com razão.
        let n = m.recall("foco:zzz", 2, ModuleId::new(), StepId::new());
        assert!(!n.is_value());
    }

    #[test]
    fn similaridade_prefere_mesma_banda_e_ressonancia_proxima() {
        let mut m = LocalMemory::new();
        m.record_with_resonance(
            "ep:igual",
            1,
            Some(sig(ch::CognitiveBand::Stability, 0.50)),
        );
        m.record_with_resonance(
            "ep:banda_diferente",
            2,
            Some(sig(ch::CognitiveBand::MetaOrganization, 0.50)),
        );
        m.record_with_resonance(
            "ep:ressonancia_longe",
            3,
            Some(sig(ch::CognitiveBand::Stability, 0.99)),
        );
        m.record("ep:sem_assinatura", 4);
        let atual = sig(ch::CognitiveBand::Stability, 0.50);
        let ranked = m.similar_by_resonance(&atual);
        assert_eq!(ranked.len(), 3, "sem assinatura não participa");
        assert_eq!(ranked[0].0.label, "ep:igual");
        assert!((ranked[0].1 - 1.0).abs() < 1e-6, "mesma banda, mesmo valor");
        assert!(
            ranked[1].1 < ranked[0].1,
            "ressonância distante perde para igual"
        );
        // Banda distante pesa menos que ressonância distante na mesma banda.
        assert!(
            ranked.iter()
                .find(|(e, _)| e.label == "ep:banda_diferente")
                .map(|(_, s)| *s)
                .unwrap()
                < ranked.iter()
                    .find(|(e, _)| e.label == "ep:ressonancia_longe")
                    .map(|(_, s)| *s)
                    .unwrap(),
            "distância de banda domina a similaridade"
        );
    }

    #[test]
    fn similaridade_e_deterministica() {
        let mut a = LocalMemory::new();
        let mut b = LocalMemory::new();
        for (label, band, res) in [
            ("x1", ch::CognitiveBand::Memory, 0.2),
            ("x2", ch::CognitiveBand::Memory, 0.8),
            ("x3", ch::CognitiveBand::Creativity, 0.5),
        ] {
            a.record_with_resonance(label, 1, Some(sig(band, res)));
            b.record_with_resonance(label, 1, Some(sig(band, res)));
        }
        let atual = sig(ch::CognitiveBand::Memory, 0.3);
        let ra = a.similar_by_resonance(&atual);
        let rb = b.similar_by_resonance(&atual);
        let la: Vec<_> = ra.iter().map(|(e, _)| e.label.clone()).collect();
        let lb: Vec<_> = rb.iter().map(|(e, _)| e.label.clone()).collect();
        assert_eq!(la, lb, "mesma gravação ⇒ mesmo ranque");
    }
}
