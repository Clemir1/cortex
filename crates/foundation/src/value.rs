//! Valor tipado com qualificação epistêmica (ausência ≠ zero).

use crate::error::TriadError;
use crate::evidence::EvidenceLevel;
use crate::id::{ModuleId, StepId};
use crate::provenance::Provenance;
use crate::status::Status;
use serde::{Deserialize, Serialize};

/// Valor tipado: todo dado do organismo carrega status e rastreio
/// (contratos_camadas.md §1 — envelope de valor).
///
/// Invariante: `value = Some(_)` SOMENTE quando `status = Value`.
/// `NoData`/`Stale`/`Invalid`/`Fallback` nunca viram `Value`.
/// `Fallback` é estrutural, nunca publicável como valor medido.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Qualified<T> {
    /// O valor em si. `Some(_)` somente quando `status = Value`.
    pub value: Option<T>,
    /// Estado epistêmico do valor.
    pub status: Status,
    /// Módulo que produziu o valor.
    pub source: ModuleId,
    /// Passo lógico da produção.
    pub step: StepId,
    /// Versão do estado produtor (0 = recém-criado).
    pub version: u64,
    /// Confiança no valor (aplicável a `Value`).
    pub confidence: Option<f32>,
    /// Proveniência: cadeia de eventos, evidência e provider.
    pub provenance: Option<Provenance>,
    /// Motivo da ausência, rejeição ou fallback.
    pub reason: Option<String>,
}

impl<T> Qualified<T> {
    /// Valor medido e válido.
    pub fn value(v: T, source: ModuleId, step: StepId) -> Self {
        Self {
            value: Some(v),
            status: Status::Value,
            source,
            step,
            version: 0,
            confidence: None,
            provenance: None,
            reason: None,
        }
    }

    /// Sem provider ou sem amostra elegível.
    pub fn no_data(reason: &str, source: ModuleId, step: StepId) -> Self {
        Self::absent(Status::NoData, Some(reason.to_string()), source, step)
    }

    /// Fora da janela temporal.
    pub fn stale(source: ModuleId, step: StepId) -> Self {
        Self::absent(Status::Stale, None, source, step)
    }

    /// Rejeitado por contrato.
    pub fn invalid(reason: &str, source: ModuleId, step: StepId) -> Self {
        Self::absent(Status::Invalid, Some(reason.to_string()), source, step)
    }

    /// Fallback estrutural — nunca publicável como valor medido.
    /// A proveniência registra o provider estrutural (nível E0:
    /// fallback não é evidência de execução).
    pub fn fallback(reason: &str, provider: ModuleId, source: ModuleId, step: StepId) -> Self {
        let mut q = Self::absent(Status::Fallback, Some(reason.to_string()), source, step);
        q.provenance = Some(Provenance {
            source_chain: Vec::new(),
            evidence_level: EvidenceLevel::E0Declared,
            provider_id: provider,
        });
        q
    }

    /// Construtor comum de ausências: `value = None` garantido.
    fn absent(status: Status, reason: Option<String>, source: ModuleId, step: StepId) -> Self {
        Self {
            value: None,
            status,
            source,
            step,
            version: 0,
            confidence: None,
            provenance: None,
            reason,
        }
    }

    /// `true` somente quando `status = Value` com valor presente.
    pub fn is_value(&self) -> bool {
        self.status == Status::Value && self.value.is_some()
    }

    /// Referência ao valor, ou `None` se status ≠ `Value`.
    pub fn as_ref_value(&self) -> Option<&T> {
        if self.status == Status::Value {
            self.value.as_ref()
        } else {
            None
        }
    }

    /// Extrai o valor. Erro tipado por status se ≠ `Value`.
    /// `Pending`/`Disabled`/`Error` geram `ContractViolation`.
    pub fn expect_value(self) -> Result<T, TriadError> {
        if self.status == Status::Value {
            return self.value.ok_or_else(|| TriadError::ContractViolation {
                detail: "status VALUE sem valor (invariante quebrada)".to_string(),
            });
        }
        let about = self.about();
        match self.status {
            Status::NoData => Err(TriadError::NoData { about }),
            Status::Stale => Err(TriadError::Stale { about }),
            Status::Invalid => Err(TriadError::Invalid {
                about,
                reason: self.reason.unwrap_or_else(|| "sem motivo registrado".into()),
            }),
            Status::Fallback => Err(TriadError::Fallback {
                about,
                reason: self.reason.unwrap_or_else(|| "sem motivo registrado".into()),
                provider: self
                    .provenance
                    .map(|p| p.provider_id)
                    .unwrap_or(self.source),
            }),
            _ => Err(TriadError::ContractViolation {
                detail: format!("status {} não é VALUE", self.status),
            }),
        }
    }

    /// Descrição curta do alvo do valor (fonte + passo).
    fn about(&self) -> String {
        format!("valor de {:?} no passo {:?}", self.source, self.step)
    }
}

#[cfg(test)]
mod tests {
    use triad_foundation::error::TriadError;
    use triad_foundation::id::{ModuleId, StepId};
    use triad_foundation::status::Status;
    use triad_foundation::value::Qualified;

    fn ctx() -> (ModuleId, StepId) {
        (ModuleId::new(), StepId::new())
    }

    #[test]
    fn valor_e_valido() {
        let (src, step) = ctx();
        let q = Qualified::value(42, src, step);
        assert!(q.is_value());
        assert_eq!(q.as_ref_value(), Some(&42));
        assert_eq!(q.status, Status::Value);
        assert_eq!(q.expect_value().unwrap(), 42);
    }

    #[test]
    fn ausencia_nunca_e_valor() {
        let (src, step) = ctx();
        let nd = Qualified::<f32>::no_data("sem amostra", src, step);
        assert!(!nd.is_value());
        assert!(nd.as_ref_value().is_none());
        assert!(matches!(nd.expect_value(), Err(TriadError::NoData { .. })));

        let st = Qualified::<f32>::stale(src, step);
        assert!(matches!(st.expect_value(), Err(TriadError::Stale { .. })));

        let inv = Qualified::<f32>::invalid("fora do contrato", src, step);
        assert!(matches!(
            inv.expect_value(),
            Err(TriadError::Invalid { reason, .. }) if reason == "fora do contrato"
        ));
    }

    #[test]
    fn fallback_grava_provenance_com_provider() {
        let (src, step) = ctx();
        let provider = ModuleId::new();
        let q = Qualified::<f32>::fallback("sem provider primário", provider, src, step);
        let provider_id = q
            .provenance
            .as_ref()
            .expect("fallback exige provenância")
            .provider_id;
        assert_eq!(provider_id, provider);
        match q.expect_value() {
            Err(TriadError::Fallback { provider, reason, .. }) => {
                assert_eq!(provider, provider_id);
                assert_eq!(reason, "sem provider primário");
            }
            other => panic!("esperava Fallback, veio {:?}", other),
        }
    }

    #[test]
    fn invariante_some_somente_em_value() {
        let (src, step) = ctx();
        let ausentes = [
            Qualified::<f32>::no_data("sem", src, step),
            Qualified::<f32>::stale(src, step),
            Qualified::<f32>::invalid("ruim", src, step),
            Qualified::<f32>::fallback("estrutural", ModuleId::new(), src, step),
        ];
        for q in &ausentes {
            assert!(q.value.is_none(), "status {:?} com value Some", q.status);
            assert!(!q.is_value());
        }
    }

    #[test]
    fn serde_qualified_roundtrip() {
        let (src, step) = ctx();
        let q = Qualified::value(0.5f32, src, step);
        let s = serde_json::to_string(&q).unwrap();
        assert!(s.contains("\"VALUE\""));
        let r: Qualified<f32> = serde_json::from_str(&s).unwrap();
        assert_eq!(r, q);
    }
}
