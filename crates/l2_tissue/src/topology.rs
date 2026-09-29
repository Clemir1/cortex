//! Topologia de tecidos: grafo bidirecional em tabela hash.

use std::collections::HashMap;

use triad_foundation as tf;

/// Grafo de tecidos: tecido → lista de vizinhos.
pub struct TopologyManager {
    tissues: HashMap<tf::TissueId, Vec<tf::TissueId>>,
}

impl TopologyManager {
    /// Cria topologia vazia.
    pub fn new() -> Self {
        Self {
            tissues: HashMap::new(),
        }
    }

    /// Registra um tecido na topologia (idempotente).
    pub fn add_tissue(&mut self, id: tf::TissueId) {
        self.tissues.entry(id).or_insert_with(Vec::new);
    }

    /// Remove o tecido e limpa as arestas que apontavam para ele.
    pub fn remove_tissue(&mut self, id: tf::TissueId) {
        if let Some(links) = self.tissues.remove(&id) {
            for other in links {
                if let Some(list) = self.tissues.get_mut(&other) {
                    list.retain(|&x| x != id);
                }
            }
        }
    }

    /// Cria aresta bidirecional a↔b (auto-link é ignorado).
    pub fn link(&mut self, a: tf::TissueId, b: tf::TissueId) {
        if a == b {
            self.add_tissue(a);
            return;
        }
        self.tissues.entry(a).or_insert_with(Vec::new).push(b);
        self.tissues.entry(b).or_insert_with(Vec::new).push(a);
    }

    /// Remove a aresta bidirecional a↔b, se existir.
    pub fn unlink(&mut self, a: tf::TissueId, b: tf::TissueId) {
        if let Some(list) = self.tissues.get_mut(&a) {
            list.retain(|&x| x != b);
        }
        if let Some(list) = self.tissues.get_mut(&b) {
            list.retain(|&x| x != a);
        }
    }

    /// Vizinhos do tecido; lista vazia se o tecido não existe.
    pub fn neighbors(&self, id: tf::TissueId) -> &[tf::TissueId] {
        self.tissues
            .get(&id)
            .map(|list| list.as_slice())
            .unwrap_or(&[])
    }

    /// Grau do tecido: número de vizinhos.
    pub fn degree(&self, id: tf::TissueId) -> usize {
        self.neighbors(id).len()
    }

    /// Total de arestas; cada aresta aparece duas vezes na tabela.
    pub fn edge_count(&self) -> usize {
        let endpoints: usize = self.tissues.values().map(Vec::len).sum();
        endpoints / 2
    }
}
