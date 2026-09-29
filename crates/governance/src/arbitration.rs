use tracing::debug;

/// Resultado da arbitragem: vencedora ou empate (empate ⇒ sem mudança).
#[derive(Debug, Clone, PartialEq)]
pub enum Tie {
    /// Proposta vencedora.
    Winner(String),
    /// Empate: nenhuma mudança.
    Tie,
}

/// Árbitro determinístico entre propostas pontuadas.
pub struct Arbiter {
    /// Marcador de tipo interno.
    _marker: std::marker::PhantomData<()>,
}

impl Arbiter {
    /// Cria um árbitro.
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }

    /// Arbitra as propostas; empate dentro de 1e-9 ⇒ Tie.
    pub fn arbitrate(&self, proposals: &[(String, f32)]) -> Tie {
        if proposals.is_empty() {
            return Tie::Tie;
        }
        let mut best = 0usize;
        let mut top1 = proposals[0].1;
        let mut top2 = f32::NEG_INFINITY;
        for (idx, (_, score)) in proposals.iter().enumerate().skip(1) {
            if *score > top1 {
                top2 = top1;
                top1 = *score;
                best = idx;
            } else if *score > top2 {
                top2 = *score;
            }
        }
        // Os dois maiores scores empatam dentro de 1e-9: sem mudança.
        if proposals.len() > 1 && (top1 - top2).abs() <= 1e-9 {
            debug!("empate na arbitragem: sem mudança");
            return Tie::Tie;
        }
        debug!(vencedora = proposals[best].0.as_str(), "proposta venceu a arbitragem");
        Tie::Winner(proposals[best].0.clone())
    }
}
