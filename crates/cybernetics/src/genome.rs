//! Genoma de parâmetros avaliado FORA do loop principal (lei da casa 7).
use tracing::{debug, trace};

/// Candidato de genoma com fitness medido fora do loop principal.
#[derive(Debug, Clone)]
pub struct Candidate {
    /// Identificador único do gene avaliado.
    pub id: String,
    /// Aptidão (fitness) medida offline.
    pub fitness: f32,
}

/// Genoma: conjunto de genes nomeados com valores contínuos.
#[derive(Debug, Clone)]
pub struct Genome {
    /// Pares (nome do gene, valor) do genoma.
    pub genes: Vec<(String, f32)>,
}

impl Genome {
    /// Cria um genoma a partir da lista de pares (nome, valor).
    pub fn new(genes: Vec<(String, f32)>) -> Self {
        Self { genes }
    }

    /// Aplica mutação determinística via LCG inline, sem crates externas.
    pub fn mutate(&self, seed: u64, rate: f32) -> Genome {
        let mut state = seed;
        let threshold = (rate * 1000.0) as u64;
        let mut genes = Vec::with_capacity(self.genes.len());
        for (name, value) in &self.genes {
            let value = *value;
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            if (state >> 33) % 1000 < threshold {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let roll = ((state >> 33) % 1000) as f32 / 1000.0;
                let delta = roll * 0.2 - 0.1;
                let mutated = (value + value * delta).clamp(0.0, 1.0);
                trace!(gene = %name, valor = mutated, "mutação aplicada");
                genes.push((name.clone(), mutated));
            } else {
                genes.push((name.clone(), value));
            }
        }
        Genome::new(genes)
    }

    /// Avalia genes contra scores externos; NUNCA chamar dentro do tick do organismo.
    pub fn evaluate_offline(&self, scores: &[(String, f32)]) -> Vec<Candidate> {
        let mut candidates: Vec<Candidate> = self
            .genes
            .iter()
            .map(|(id, _)| {
                let fitness = scores
                    .iter()
                    .find(|(scored, _)| scored == id)
                    .map(|(_, score)| *score)
                    .unwrap_or(0.0);
                Candidate {
                    id: id.clone(),
                    fitness,
                }
            })
            .collect();
        candidates.sort_by(|a, b| {
            b.fitness
                .partial_cmp(&a.fitness)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
        debug!(avaliados = candidates.len(), "genoma avaliado offline");
        candidates
    }
}
