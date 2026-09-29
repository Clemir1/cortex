//! Memória global de episódios com reconsolidação e esquecimento seletivo.

use triad_foundation as tf;
use tracing::{debug, trace};

/// Episódio consolidado na memória global.
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub id: tf::EpisodeId,
    pub description: String,
    pub reinforced: u32,
    pub created_step: u64,
}

/// Memória global: episódios versionados com esquecimento seletivo.
pub struct GlobalMemory {
    episodes: Vec<crate::memory::MemoryEntry>,
    version: u64,
}

impl GlobalMemory {
    /// Cria memória global vazia.
    pub fn new() -> Self {
        Self {
            episodes: Vec::new(),
            version: 0,
        }
    }

    /// Versão atual da memória (incrementa a cada registro).
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Registra novo episódio; acima de 2048 esquece o de menor reforço.
    pub fn record(&mut self, description: &str, step: u64) -> tf::EpisodeId {
        let id = tf::EpisodeId::new();
        self.episodes.push(MemoryEntry {
            id,
            description: description.to_string(),
            reinforced: 0,
            created_step: step,
        });
        self.version += 1;
        debug!(step = step, "episódio registrado na memória global");
        // Esquecimento seletivo: remove o de MENOR reforço (empate = mais antigo).
        if self.episodes.len() > 2048 {
            if let Some(weakest) = self
                .episodes
                .iter()
                .enumerate()
                .min_by_key(|(_, e)| e.reinforced)
                .map(|(i, _)| i)
            {
                self.episodes.remove(weakest);
                debug!(total = self.episodes.len(), "poda: episódio mais fraco esquecido");
            }
        }
        id
    }

    /// Reforça episódios que contêm a palavra-chave; retorna quantos reforçou.
    /// Use `count()` como denominador para taxas honestas.
    pub fn reconsolidate(&mut self, keyword: &str) -> usize {
        let mut reinforced = 0;
        for entry in self.episodes.iter_mut() {
            if entry.description.contains(keyword) {
                entry.reinforced += 1;
                reinforced += 1;
            }
        }
        trace!(reforcos = reinforced, "reconsolidação concluída");
        reinforced
    }

    /// Total de episódios armazenados (denominador honesto das taxas).
    pub fn count(&self) -> usize {
        self.episodes.len()
    }

    /// Episódio de maior reforço; memória vazia devolve NO_DATA (ausência ≠ zero).
    pub fn strongest(&self) -> tf::Qualified<&MemoryEntry> {
        match self.episodes.iter().max_by_key(|e| e.reinforced) {
            Some(entry) => tf::Qualified::value(entry, tf::ModuleId::new(), tf::StepId::new()),
            None => {
                debug!("strongest sem dado: memória global vazia");
                tf::Qualified::no_data(
                    "memória global vazia",
                    tf::ModuleId::new(),
                    tf::StepId::new(),
                )
            }
        }
    }
}
