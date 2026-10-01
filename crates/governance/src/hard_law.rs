//! Leis DURAS da casa (17.11, sessão 7): cada lei é DADO — um
//! `LawEntry` com id, descrição e checagem PURA sobre a
//! fotografia observacional (`LawObservation`), reunidas no
//! `HardLawSet`. O enforcement (auditar pós-tick, contar com
//! denominador, nunca causar efeito) vive no `law_engine`.

use tracing::{trace, warn};

/// Veredito de uma checagem contra as leis duras (API simples
/// para checagens pontuais de proposta).
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

/// Uma violação registrada — SEMPRE com razão legível.
#[derive(Debug, Clone, PartialEq)]
pub struct Violation {
    pub law_id: &'static str,
    pub reason: String,
}

/// Lei dura como DADO: id, descrição e checagem PURA sobre a
/// fotografia. Adicionar lei da casa = adicionar entrada.
#[derive(Debug, Clone)]
pub struct LawEntry {
    pub id: &'static str,
    pub description: &'static str,
    pub check: fn(&super::law_engine::LawObservation) -> Option<String>,
}

/// Conjunto das leis duras da casa — DADO, não código disperso.
#[derive(Debug, Clone)]
pub struct HardLawSet {
    pub laws: Vec<LawEntry>,
}

impl HardLawSet {
    /// As oito leis da casa (AGENTS.md), cada uma uma entrada.
    pub fn house() -> Self {
        use super::law_engine::LawObservation as Obs;
        let c_evidence = |o: &Obs| {
            o.promotion_without_evidence
                .then(|| "promoção sem evidência E0–E5 (Lei 1)".to_string())
        };
        let c_denominator = |o: &Obs| {
            o.rate_without_denominator
                .then(|| "taxa sem denominador (Lei 1)".to_string())
        };
        let c_absence = |o: &Obs| {
            o.absence_became_value
                .then(|| "ausência virou valor — Qualified violado (Lei 2)".to_string())
        };
        let c_learning = |o: &Obs| {
            o.learning_suspended.then(|| {
                "aprendizagem suspensa — crise muda POLÍTICA, nunca desativa (Lei 4)".to_string()
            })
        };
        let c_fallback = |o: &Obs| {
            o.fallback_without_reason
                .then(|| "FALLBACK sem provider_id/razão (Lei 3)".to_string())
        };
        let c_ttl = |o: &Obs| {
            o.intervention_without_ttl.then(|| {
                "intervenção sem ttl/reversão por efeito observado (Lei 6)".to_string()
            })
        };
        let c_genome = |o: &Obs| {
            o.genome_in_loop
                .then(|| "genome avaliado dentro do loop principal (Lei 7)".to_string())
        };
        let c_regions = |o: &Obs| {
            o.region_outside_extensions
                .then(|| "região cerebral fora de extensions/full_brain (Lei 8)".to_string())
        };
        Self {
            laws: vec![
                LawEntry { id: "e0_e5_evidence", description: "sem auto-promoção: evidência E0–E5 obrigatória", check: c_evidence },
                LawEntry { id: "rates_with_denominator", description: "taxas sempre com denominador", check: c_denominator },
                LawEntry { id: "absence_not_zero", description: "ausência nunca vira valor (Qualified)", check: c_absence },
                LawEntry { id: "learning_never_suspended", description: "aprendizagem nunca suspensa; crise muda política", check: c_learning },
                LawEntry { id: "fallback_with_reason", description: "FALLBACK exige provider_id + razão", check: c_fallback },
                LawEntry { id: "intervention_ttl_reverts", description: "intervenção sem efeito observado reverte no ttl", check: c_ttl },
                LawEntry { id: "genome_offline", description: "Genome::evaluate_offline fora do loop", check: c_genome },
                LawEntry { id: "regions_in_extensions", description: "regiões cerebrais em extensions/full_brain", check: c_regions },
            ],
        }
    }

    /// Audita a fotografia contra TODAS as leis: violações com
    /// razão, em ordem canônica de registro. Pura — sem efeito.
    pub fn check_all(&self, o: &super::law_engine::LawObservation) -> Vec<Violation> {
        self.laws
            .iter()
            .filter_map(|law| (law.check)(o).map(|reason| Violation {
                law_id: law.id,
                reason,
            }))
            .collect()
    }

    /// Checagem pontual de proposta (API simples): Allow/Deny.
    pub fn check_quick(&self, ok: bool, motivo: &str) -> Verdict {
        if ok {
            trace!("leis duras conferidas: liberado");
            Verdict::Allow
        } else {
            warn!(motivo, "lei dura violada");
            Verdict::Deny {
                reason: motivo.to_string(),
            }
        }
    }
}
