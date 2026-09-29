use tracing::{trace, warn};

/// Veredito de uma checagem contra as leis duras.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Checagem liberada.
    Allow,
    /// Checagem negada com motivo.
    Deny {
        /// Motivo da negação.
        reason: String,
    },
}

/// Evidência coletada de uma proposta para checagem de leis duras.
#[derive(Debug, Clone, Copy)]
pub struct LawCheck {
    /// A proposta apresenta evidência E0–E5.
    pub has_evidence: bool,
    /// Toda taxa declarada carrega denominador.
    pub has_denominator: bool,
    /// Ausência nunca vira valor (respeita Qualified).
    pub respects_qualified: bool,
    /// A proposta suspende a aprendizagem.
    pub suspends_learning: bool,
    /// O genoma roda fora do loop principal.
    pub offline_evaluation: bool,
    /// A decisão carrega identity_context.
    pub has_identity_context: bool,
    /// A intervenção tem ttl e reverte sem efeito observado.
    pub intervention_ttl_ok: bool,
}

/// Conjunto das leis duras (E0–E5 e regras da casa).
pub struct HardLawSet {
    /// Nomes das leis duras registradas.
    laws: Vec<String>,
}

impl HardLawSet {
    /// Cria o conjunto com os sete nomes das leis duras.
    pub fn new() -> Self {
        Self {
            laws: vec![
                "E0-E5: sem auto-promocao".to_string(),
                "taxas sempre com denominador".to_string(),
                "ausencia nao e zero".to_string(),
                "aprendizagem nunca suspensa".to_string(),
                "genome fora do loop".to_string(),
                "decisao sem identity_context e rejeitada".to_string(),
                "intervencao sem ttl reverte".to_string(),
            ],
        }
    }

    /// Lista os nomes das leis duras registradas.
    pub fn laws(&self) -> &[String] {
        &self.laws
    }

    /// Motivo da primeira violação encontrada, se houver.
    pub fn deny_reason(&self, c: &LawCheck) -> Option<String> {
        if !c.has_evidence {
            Some("E0-E5: sem auto-promocao".to_string())
        } else if !c.has_denominator {
            Some("taxa sem denominador".to_string())
        } else if !c.respects_qualified {
            Some("ausencia virou valor".to_string())
        } else if c.suspends_learning {
            Some("aprendizagem suspensa".to_string())
        } else if !c.offline_evaluation {
            Some("genoma dentro do loop".to_string())
        } else if !c.has_identity_context {
            Some("decisao sem identity_context".to_string())
        } else if !c.intervention_ttl_ok {
            Some("intervencao sem ttl".to_string())
        } else {
            None
        }
    }

    /// Aplica a checagem e devolve o veredito da primeira violação.
    pub fn check(&self, c: &LawCheck) -> Verdict {
        match self.deny_reason(c) {
            Some(reason) => {
                warn!(motivo = reason.as_str(), "lei dura violada");
                Verdict::Deny { reason }
            }
            None => {
                trace!("leis duras conferidas: liberado");
                Verdict::Allow
            }
        }
    }
}
