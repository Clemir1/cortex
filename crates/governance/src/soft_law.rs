use tracing::debug;

/// Lei suave com ADR de origem e peso.
pub struct SoftLaw {
    /// Identificador único da lei suave.
    pub id: String,
    /// ADR que origina a lei suave.
    pub adr_id: String,
    /// Descrição legível da lei suave.
    pub description: String,
    /// Peso da lei suave na governança.
    pub weight: f32,
}

impl SoftLaw {
    /// Cria uma lei suave; None se sem ADR ou peso negativo.
    pub fn new(id: &str, adr_id: &str, description: &str, weight: f32) -> Option<Self> {
        if adr_id.is_empty() || weight < 0.0 {
            debug!("lei suave rejeitada: exige ADR");
            return None;
        }
        Some(Self {
            id: id.to_string(),
            adr_id: adr_id.to_string(),
            description: description.to_string(),
            weight,
        })
    }
}

/// Livro de leis suaves registradas.
pub struct SoftLawBook {
    /// Leis suaves registradas.
    laws: Vec<SoftLaw>,
}

impl SoftLawBook {
    /// Cria um livro vazio.
    pub fn new() -> Self {
        Self { laws: Vec::new() }
    }

    /// Registra uma lei suave no livro.
    pub fn add(&mut self, law: SoftLaw) {
        debug!(id = law.id.as_str(), "lei suave aceita no livro");
        self.laws.push(law);
    }

    /// Conta quantas leis suaves estão registradas.
    pub fn len(&self) -> usize {
        self.laws.len()
    }

    /// Soma o peso de todas as leis registradas.
    pub fn total_weight(&self) -> f32 {
        self.laws.iter().map(|law| law.weight).sum()
    }
}
