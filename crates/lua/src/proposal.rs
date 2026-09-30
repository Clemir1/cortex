//! Proposal de política Lua (17.12) — o CONTRATO da fronteira
//! (rust_lua_boundary.md): Lua SEMPRE retorna Proposal (ou nil =
//! banda morta); Rust VALIDA e aplica. Nada de estado canônico em
//! Lua. A LEI dos limites vive em RUST: cada (módulo, parâmetro)
//! aceito tem FAIXA estrutural declarada aqui — Lua propõe DENTRO.

use serde::{Deserialize, Serialize};

/// Parâmetro de política aceito — whitelist declarada por RUST.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamSpec {
    pub module: &'static str,
    pub parameter: &'static str,
    pub min: f64,
    pub max: f64,
}

impl ParamSpec {
    /// Whitelist canônica — cada entrada cita a lei estrutural.
    pub const fn whitelist() -> [ParamSpec; 7] {
        [
            // Lei 4: eta jamais suspende — faixa da casa (0.005..0.02).
            ParamSpec {
                module: "learning",
                parameter: "eta",
                min: 0.005,
                max: 0.02,
            },
            // Atenção L3: threshold de saliência (policy da casa).
            ParamSpec {
                module: "l3.attention",
                parameter: "salience_threshold",
                min: 0.25,
                max: 0.45,
            },
            // O1: intake energético do substrato (0.0..1.5).
            ParamSpec {
                module: "l1.substrate",
                parameter: "energy.intake_rate",
                min: 0.0,
                max: 1.5,
            },
            // 17.4: especialização tecidual L2 (threshold via
            // Proposal da policy Lua; Rust guarda a faixa).
            ParamSpec {
                module: "l2.tissue",
                parameter: "specialization_threshold",
                min: 0.05,
                max: 0.95,
            },
            // O5/cybernetics: cadência não inferior a 10 steps.
            ParamSpec {
                module: "cybernetics",
                parameter: "o5_interval_steps",
                min: 10.0,
                max: 1000.0,
            },
            // 18.4: taxa máxima de uma perna por proposta federativa
            // (staging feature-off; a policy propõe, o Rust valida a
            // faixa e o motor clampa cada perna).
            ParamSpec {
                module: "federation",
                parameter: "trade_rate",
                min: 0.0,
                max: 0.5,
            },
            // 17.11 (LawEngine): fator do orçamento de eventos sob
            // pressão (soft law). FAIXA GARANTE A LEI 4: mínimo 0.5
            // — economia de política, o sistema NUNCA desliga.
            ParamSpec {
                module: "law_soft",
                parameter: "max_events_frac",
                min: 0.5,
                max: 1.0,
            },
        ]
    }

    pub fn lookup(module: &str, parameter: &str) -> Option<ParamSpec> {
        Self::whitelist()
            .into_iter()
            .find(|s| s.module == module && s.parameter == parameter)
    }
}

/// Proposal CRUA retornada por Lua (serialização mlua 1:1).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LuaProposal {
    pub target_module: String,
    pub target_parameter: String,
    pub value: f64,
    /// Obrigatória (Lei 3): razão do ajuste nunca vazia.
    pub reason: String,
    pub confidence: f64,
    pub ttl: u64,
}

/// Rejeição TIPADA — nunca booleano mudo.
#[derive(Debug, Clone, PartialEq)]
pub enum PolicyReject {
    /// (módulo, parâmetro) fora da whitelist do Rust.
    UnknownTarget { module: String, parameter: String },
    /// Valor fora da faixa estrutural do parâmetro.
    ValueOutOfRange { parameter: String, value: f64, min: f64, max: f64 },
    /// Lei 3: razão vazia ou ausente.
    EmptyReason,
    /// Confiança fora de [0,1].
    ConfidenceOutOfRange { value: f64 },
    /// TTL fora de 1..=64 (mesma janela do PolicyInbox da casa).
    TtlOutOfRange { value: u64 },
    /// NaN/inf não são números de política.
    ValueNotFinite { value: f64 },
}

/// Proposal VALIDADA por Rust — carrega a procedência (hash
/// versionado do arquivo .lua de origem).
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedProposal {
    pub module: String,
    pub parameter: String,
    pub value: f64,
    pub reason: String,
    pub confidence: f64,
    pub ttl: u64,
    /// FNV-1a do CONTEÚDO do arquivo .lua de origem (versionado).
    pub policy_hash: u64,
}

impl LuaProposal {
    /// VALIDAÇÃO RUST: whitelist + faixas + reason + confiança + ttl.
    pub fn validate(&self, policy_hash: u64) -> Result<ValidatedProposal, PolicyReject> {
        let Some(spec) = ParamSpec::lookup(&self.target_module, &self.target_parameter) else {
            return Err(PolicyReject::UnknownTarget {
                module: self.target_module.clone(),
                parameter: self.target_parameter.clone(),
            });
        };
        if !self.value.is_finite() {
            return Err(PolicyReject::ValueNotFinite { value: self.value });
        }
        if self.value < spec.min || self.value > spec.max {
            return Err(PolicyReject::ValueOutOfRange {
                parameter: spec.parameter.to_string(),
                value: self.value,
                min: spec.min,
                max: spec.max,
            });
        }
        if self.reason.trim().is_empty() {
            return Err(PolicyReject::EmptyReason);
        }
        if !(0.0..=1.0).contains(&self.confidence) {
            return Err(PolicyReject::ConfidenceOutOfRange {
                value: self.confidence,
            });
        }
        if !(1..=64).contains(&self.ttl) {
            return Err(PolicyReject::TtlOutOfRange { value: self.ttl });
        }
        Ok(ValidatedProposal {
            module: spec.module.to_string(),
            parameter: spec.parameter.to_string(),
            value: self.value,
            reason: self.reason.trim().to_string(),
            confidence: self.confidence,
            ttl: self.ttl,
            policy_hash,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proposal(module: &str, param: &str, value: f64, reason: &str, ttl: u64) -> LuaProposal {
        LuaProposal {
            target_module: module.to_string(),
            target_parameter: param.to_string(),
            value,
            reason: reason.to_string(),
            confidence: 0.7,
            ttl,
        }
    }

    #[test]
    fn whitelist_valida_as_faixas_estruturais() {
        assert_eq!(
            ParamSpec::lookup("learning", "eta"),
            Some(ParamSpec {
                module: "learning",
                parameter: "eta",
                min: 0.005,
                max: 0.02
            })
        );
        assert!(ParamSpec::lookup("lua", "qualquer").is_none(), "Lua não é alvo");
        // 17.4: especialização tecidual L2 admitida na whitelist.
        assert!(ParamSpec::lookup("l2.tissue", "specialization_threshold").is_some());
    }

    #[test]
    fn proposal_valida_tipada() {
        let p = proposal("learning", "eta", 0.01, "ajuste de taxa", 20);
        let v = p.validate(42).expect("válida");
        assert_eq!(v.value, 0.01);
        assert_eq!(v.policy_hash, 42, "procedência versionada");
    }

    #[test]
    fn rejeicoes_sao_tipadas() {
        let p = proposal("qualquer", "coisa", 1.0, "r", 20);
        assert!(matches!(p.validate(1), Err(PolicyReject::UnknownTarget { .. })));

        let p = proposal("learning", "eta", 0.9, "r", 20);
        assert_eq!(
            p.validate(1),
            Err(PolicyReject::ValueOutOfRange {
                parameter: "eta".into(),
                value: 0.9,
                min: 0.005,
                max: 0.02
            })
        );

        let p = proposal("learning", "eta", 0.01, "   ", 20);
        assert_eq!(p.validate(1), Err(PolicyReject::EmptyReason), "Lei 3");

        let p = proposal("learning", "eta", 0.01, "r", 0);
        assert_eq!(p.validate(1), Err(PolicyReject::TtlOutOfRange { value: 0 }));

        let p = proposal("learning", "eta", 0.01, "r", 65);
        assert_eq!(p.validate(1), Err(PolicyReject::TtlOutOfRange { value: 65 }));

        let p = proposal("learning", "eta", f64::NAN, "r", 20);
        assert!(matches!(p.validate(1), Err(PolicyReject::ValueNotFinite { .. })));
    }
}
