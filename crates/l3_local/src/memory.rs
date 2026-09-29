//! Memória episódica local da camada L3.

use triad_contracts as tc;
use triad_foundation as tf;
use tracing::{debug, trace};

use tf::id::{EpisodeId, ModuleId, StepId};

/// Idade máxima (em passos) para um episódio ser considerado válido.
pub const VALIDITY_WINDOW: u64 = 5;

/// Capacidade máxima de episódios na memória local.
const MAX_EPISODES: usize = 1024;

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

    /// Registra episódio com força 1.0; acima de 1024 remove o mais fraco.
    pub fn record(&mut self, label: &str, step: u64) {
        self.episodes.push(Episode {
            id: EpisodeId::new(),
            label: label.to_string(),
            step,
            strength: 1.0,
        });
        debug!(label = label, step = step, "episodio gravado");
        if self.episodes.len() > MAX_EPISODES {
            self.evict_weakest();
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
}
