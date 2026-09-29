//! Estado epistêmico que todo valor tipado carrega (ausência ≠ zero).

use serde::{Deserialize, Serialize};

/// Estado epistêmico de um valor. Nenhum dado é neutro: sem status não há
/// valor publicável.
///
/// Regras duras (doc/architecture/contratos_camadas.md §5):
/// - `NoData`, `Stale`, `Invalid` e `Fallback` NUNCA viram `Value`;
/// - `Fallback` é estrutural, nunca publicável como valor medido.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    /// Medido e válido.
    Value,
    /// Sem provider ou sem amostra elegível.
    NoData,
    /// Fora da janela temporal.
    Stale,
    /// Rejeitado por contrato.
    Invalid,
    /// Aguardando produção.
    Pending,
    /// Desativado por política (não por ausência).
    Disabled,
    /// Erro na produção.
    Error,
    /// Estrutural; nunca publicável como valor medido.
    Fallback,
}

impl Status {
    /// Nome canônico (ex.: `"NO_DATA"`).
    pub const fn as_str(&self) -> &'static str {
        match self {
            Status::Value => "VALUE",
            Status::NoData => "NO_DATA",
            Status::Stale => "STALE",
            Status::Invalid => "INVALID",
            Status::Pending => "PENDING",
            Status::Disabled => "DISABLED",
            Status::Error => "ERROR",
            Status::Fallback => "FALLBACK",
        }
    }

    /// Converte o nome canônico. `None` se desconhecido.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "VALUE" => Some(Status::Value),
            "NO_DATA" => Some(Status::NoData),
            "STALE" => Some(Status::Stale),
            "INVALID" => Some(Status::Invalid),
            "PENDING" => Some(Status::Pending),
            "DISABLED" => Some(Status::Disabled),
            "ERROR" => Some(Status::Error),
            "FALLBACK" => Some(Status::Fallback),
            _ => None,
        }
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use triad_foundation::status::Status;

    #[test]
    fn as_str_e_from_str_sao_canonicos() {
        let todos = [
            Status::Value,
            Status::NoData,
            Status::Stale,
            Status::Invalid,
            Status::Pending,
            Status::Disabled,
            Status::Error,
            Status::Fallback,
        ];
        for st in todos {
            assert_eq!(Status::from_str(st.as_str()), Some(st));
        }
        assert_eq!(Status::from_str("nonsense"), None);
    }

    #[test]
    fn serde_usa_screaming_snake() {
        assert_eq!(serde_json::to_string(&Status::Value).unwrap(), "\"VALUE\"");
        assert_eq!(serde_json::to_string(&Status::NoData).unwrap(), "\"NO_DATA\"");
        let st: Status = serde_json::from_str("\"FALLBACK\"").unwrap();
        assert_eq!(st, Status::Fallback);
    }

    #[test]
    fn display_igual_as_str() {
        assert_eq!(Status::Stale.to_string(), "STALE");
        assert_eq!(Status::Fallback.to_string(), "FALLBACK");
    }
}
