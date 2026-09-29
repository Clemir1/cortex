//! Contrato temporal da camada L1 (auditado no legado como
//! `core/l1_state_contract.py`; runs 741–752).
//!
//! Todo estado L1 publicado atravessa este contrato: identidade
//! (versão monotônica), fase do step, população, amostra com política e
//! cobertura, hashes de estado médio e da ordem de clusters, status de
//! provider classificado — `Value`/`NoData`/`Stale`/`Invalid`/`Fallback`,
//! nunca ausência silenciosa (auditoria L1 §6 P0).
//!
//! Consumidores (L2/L3, no futuro) leem via recibos
//! ([`L1ReadReceipt`]) que registram o que foi lido, de qual versão, com
//! qual classificação — o ledger de passagem E0–E5 nasce aqui.

use serde::{Deserialize, Serialize};
use triad_foundation::id::ClusterId;
use triad_foundation::status::Status;

use crate::hash::{hash_cluster_order, hash_f64_slice, to_hex16};

/// Fase do step em que a representação L1 foi produzida.
///
/// O ciclo completo do step tem 4 marcos versionados (auditoria L1 §12,
/// Run 748). L1 publica `PostPhysical` e `PreCognitive`; `PostTissue` e
/// `PostCognitive` pertencem aos runners de L2/L3 — o contrato suporta
/// as quatro desde já para que nenhuma camada precise redefinir fases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum L1StatePhase {
    /// Após física do substrato (energia, dinâmica, survival).
    PostPhysical,
    /// Após organização tecidual (L2) — reservada para o runner L2.
    PostTissue,
    /// Antes da cognição: amostra pronta para consumo por L3.
    PreCognitive,
    /// Após cognição local (L3) — reservada para o runner L3.
    PostCognitive,
}

impl L1StatePhase {
    /// Ordem canônica do ciclo (1..=4).
    pub const fn ordinal(self) -> u8 {
        match self {
            Self::PostPhysical => 1,
            Self::PostTissue => 2,
            Self::PreCognitive => 3,
            Self::PostCognitive => 4,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PostPhysical => "POST_PHYSICAL",
            Self::PostTissue => "POST_TISSUE",
            Self::PreCognitive => "PRE_COGNITIVE",
            Self::PostCognitive => "POST_COGNITIVE",
        }
    }
}

/// Política de amostragem usada para a representação publicada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SamplePolicy {
    /// População inteira.
    All,
    /// Amostra estratificada (por banda de iteração estável da matriz).
    Stratified,
}

impl SamplePolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Stratified => "stratified",
        }
    }
}

/// Identidade e validade de uma representação L1 publicada — o snapshot
/// imutável que viaja para L2/L3 (espelho do `L1StateSnapshot` legado).
///
/// Invariantes (quebrá-los é `ContractViolation`):
/// - `provider_status = Value` ⇒ hashes presentes e cobertura definida;
/// - `provider_status ≠ Value` ⇒ `no_data_reason` presente;
/// - versão é monotônica por ledger, nunca rebaixada.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct L1Snapshot {
    /// Versão monotônica da representação (por ledger).
    pub state_version: u64,
    /// Índice sequencial do step (relógio lógico do runner).
    pub step: u64,
    /// Fase do ciclo em que foi produzida.
    pub phase: L1StatePhase,
    /// População viva total no momento da publicação.
    pub population_total: u32,
    /// Dimensão do estado por cluster (97).
    pub state_dimension: u32,
    /// Linhas da amostra publicada.
    pub sample_count: u32,
    /// Cobertura = sample_count / population_total. `None` se sem amostra.
    pub sample_coverage: Option<f32>,
    /// Política de amostragem declarada.
    pub sample_policy: SamplePolicy,
    /// Hash hex (16 chars) do estado médio da amostra.
    pub mean_state_hash: Option<String>,
    /// Hash hex (16 chars) da ordem de iteração dos clusters.
    pub cluster_order_hash: Option<String>,
    /// Classificação epistêmica do provider.
    pub provider_status: Status,
    /// Motivo da ausência quando `provider_status ≠ Value`.
    pub no_data_reason: Option<String>,
}

impl L1Snapshot {
    /// Serialização sem arrays mutáveis (o estado em si não viaja: viajam
    /// identidade, hashes e classificação — eventos pequenos, ADR-0004).
    pub fn summary_line(&self) -> String {
        format!(
            "v{} step {} {} pop {} dim {} sample {} ({}) cov {} status {}{}",
            self.state_version,
            self.step,
            self.phase.as_str(),
            self.population_total,
            self.state_dimension,
            self.sample_count,
            self.sample_policy.as_str(),
            self.sample_coverage.map(|c| format!("{c:.3}")).unwrap_or_else(|| "n/a".into()),
            self.provider_status,
            self.no_data_reason
                .as_ref()
                .map(|r| format!(" reason={r}"))
                .unwrap_or_default(),
        )
    }
}

/// Recibo de uma leitura de fronteira — cada consumidor de estado L1
/// registra o que leu e como foi classificado (o "ledger de passagem":
/// produtor → versão → consumidor → classificação).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct L1ReadReceipt {
    /// Chave canônica lida (ex.: `l1.mean_state`).
    pub key: &'static str,
    /// Classificação da leitura.
    pub provider_status: Status,
    /// Identidade do provider consultado.
    pub provider_id: String,
    /// Se um fallback estrutural foi aplicado (nunca publicável).
    pub fallback_used: bool,
    /// Motivo do fallback/ausência.
    pub fallback_reason: Option<String>,
    /// Versão do estado lido.
    pub state_version: Option<u64>,
    /// Fase do estado lido.
    pub state_phase: Option<L1StatePhase>,
    /// Hash do estado lido (para casar com o snapshot publicado).
    pub state_hash: Option<String>,
}

/// Constrói e classifica um snapshot a partir da matéria-prima —
/// a lógica do `L1StateLedger.publish` legado, sem mutabilidade: recebe
/// versão/step já atribuídos pelo ledger dono e devolve o snapshot
/// classificado (ou `Invalid` com razão — nunca silêncio).
///
/// `mean_state`: média por dimensão da amostra (ou `None`).
/// `sample_count`/`sample_policy`: o que de fato foi publicado
/// (POST_PHYSICAL publica a população; PRE_COGNITIVE publica amostra
/// estratificada — cobertura é calculada, nunca presumida).
#[allow(clippy::too_many_arguments)]
pub fn build_snapshot(
    state_version: u64,
    step: u64,
    phase: L1StatePhase,
    population_total: u32,
    sample_count: u32,
    mean_state: Option<&[f64]>,
    cluster_order: &[ClusterId],
    sample_policy: SamplePolicy,
) -> L1Snapshot {
    let (provider_status, reason, mean_hash, order_hash, coverage) = match mean_state {
        None => (
            Status::NoData,
            Some("state_absent".to_string()),
            None,
            None,
            None,
        ),
        Some(mean) if mean.is_empty() => (
            Status::NoData,
            Some("state_absent".to_string()),
            None,
            None,
            None,
        ),
        Some(mean) if mean.iter().any(|x| !x.is_finite()) => (
            Status::Invalid,
            Some("state_non_finite".to_string()),
            None,
            None,
            None,
        ),
        Some(mean) => {
            let cov = if population_total > 0 {
                Some(sample_count as f32 / population_total as f32)
            } else {
                None
            };
            (
                Status::Value,
                None,
                hash_f64_slice(mean).map(to_hex16),
                hash_cluster_order(cluster_order).map(to_hex16),
                cov,
            )
        }
    };
    L1Snapshot {
        state_version,
        step,
        phase,
        population_total,
        state_dimension: mean_state.map(|m| m.len() as u32).unwrap_or(0),
        sample_count: if provider_status == Status::Value {
            sample_count
        } else {
            0
        },
        sample_coverage: if provider_status == Status::Value {
            coverage
        } else {
            None
        },
        sample_policy,
        mean_state_hash: mean_hash,
        cluster_order_hash: order_hash,
        provider_status,
        no_data_reason: reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use triad_foundation::id::ClusterId;

    #[test]
    fn snapshot_sem_estado_e_no_data_com_razao() {
        let ids = [ClusterId::new(), ClusterId::new()];
        let s = build_snapshot(1, 0, L1StatePhase::PostPhysical, 2, 2, None, &ids, SamplePolicy::All);
        assert_eq!(s.provider_status, Status::NoData);
        assert_eq!(s.no_data_reason.as_deref(), Some("state_absent"));
        assert!(s.mean_state_hash.is_none());
        assert_eq!(s.sample_count, 0);
    }

    #[test]
    fn snapshot_com_estado_e_value_com_hashes() {
        let ids = [ClusterId::new(), ClusterId::new()];
        let mean = [0.5_f64; 97];
        let s = build_snapshot(2, 1, L1StatePhase::PreCognitive, 2, 2, Some(&mean), &ids, SamplePolicy::All);
        assert_eq!(s.provider_status, Status::Value);
        assert_eq!(s.state_dimension, 97);
        assert_eq!(s.sample_coverage, Some(1.0));
        assert!(s.mean_state_hash.is_some());
        assert!(s.cluster_order_hash.is_some());
        assert!(s.no_data_reason.is_none());
    }

    #[test]
    fn amostra_estratificada_publica_cobertura_parcial() {
        let ids: Vec<ClusterId> = (0..100).map(|_| ClusterId::new()).collect();
        let mean = [0.25_f64; 97];
        let s = build_snapshot(3, 2, L1StatePhase::PreCognitive, 100, 25, Some(&mean), &ids, SamplePolicy::Stratified);
        assert_eq!(s.provider_status, Status::Value);
        assert_eq!(s.sample_count, 25);
        assert_eq!(s.sample_policy, SamplePolicy::Stratified);
        assert!((s.sample_coverage.unwrap() - 0.25).abs() < 1e-6);
    }

    #[test]
    fn estado_nao_finito_e_invalid() {
        let ids = [ClusterId::new()];
        let mean = [f64::NAN; 97];
        let s = build_snapshot(4, 3, L1StatePhase::PostPhysical, 1, 1, Some(&mean), &ids, SamplePolicy::All);
        assert_eq!(s.provider_status, Status::Invalid);
        assert_eq!(s.no_data_reason.as_deref(), Some("state_non_finite"));
    }

    #[test]
    fn ordem_das_fases_e_canonica() {
        assert!(L1StatePhase::PostPhysical.ordinal() < L1StatePhase::PostTissue.ordinal());
        assert!(L1StatePhase::PostTissue.ordinal() < L1StatePhase::PreCognitive.ordinal());
        assert!(L1StatePhase::PreCognitive.ordinal() < L1StatePhase::PostCognitive.ordinal());
        assert_eq!(L1StatePhase::PreCognitive.as_str(), "PRE_COGNITIVE");
    }
}
