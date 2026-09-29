//! Contrato temporal da camada L2 (tecidos) — a fase `PostTissue` do
//! ciclo de 4 marcos declarada em [`crate::l1::L1StatePhase`].
//!
//! O snapshot L2 publica a organização mesoscópica que L2 derivou do
//! substrato: quantos tecidos existem, quantos clusters estão vinculados,
//! coesão média e hash da ordem dos tecidos. A especialização de cada
//! tecido viaja nos eventos (`TissueKind::Specialized`), não aqui — aqui
//! vai a IDENTIDADE da representação (versão, hashes, classificação).
//!
//! Invariantes (quebrá-los é `ContractViolation`):
//! - `provider_status = Value` ⇒ cobertura calculada (nunca presumida);
//! - população viva = 0 ⇒ `NoData` com razão (ausência ≠ zero);
//! - população > 0 com ZERO tecidos é estado **Value** e legítimo:
//!   cobertura 0.0 e métricas `None` (ausência de tecido não é energia 0);
//! - métricas não finitas ⇒ `Invalid` com razão.

use serde::{Deserialize, Serialize};
use triad_foundation::id::TissueId;
use triad_foundation::status::Status;

use crate::hash::{hash_id_order, to_hex16};
use crate::l1::L1StatePhase;

/// Identidade e validade da organização tecidual publicada por L2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TissueSnapshot {
    /// Versão monotônica da representação (por ledger L2).
    pub state_version: u64,
    /// Step em que a organização foi derivada do substrato.
    pub step: u64,
    /// Fase do ciclo: sempre `PostTissue` para publicações L2.
    pub phase: L1StatePhase,
    /// População viva do substrato no momento da derivação.
    pub population_total: u32,
    /// Tecidos ativos.
    pub tissue_count: u32,
    /// Clusters vinculados a tecidos.
    pub assigned_members: u32,
    /// Clusters vivos sem tecido (população − atribuídos).
    pub unassigned_members: u32,
    /// Cobertura = assigned / population. `None` quando sem dados.
    pub assignment_coverage: Option<f32>,
    /// Energia média dos tecidos (não dos clusters soltos). `None` se
    /// não há tecidos — ausência ≠ zero.
    pub mean_energy: Option<f64>,
    /// Coesão média dos tecidos. `None` se não há tecidos.
    pub coherence_mean: Option<f64>,
    /// Hash hex (16) da ordem canônica dos tecidos.
    pub tissue_order_hash: Option<String>,
    /// Classificação epistêmica do provider L2.
    pub provider_status: Status,
    /// Motivo da ausência quando `provider_status ≠ Value`.
    pub no_data_reason: Option<String>,
}

impl TissueSnapshot {
    /// Linha de auditoria sem arrays (ADR-0004: eventos pequenos).
    pub fn summary_line(&self) -> String {
        format!(
            "v{} step {} {} pop {} tecidos {} atribuidos {} soltos {} cov {} energia {} coesao {} status {}{}",
            self.state_version,
            self.step,
            self.phase.as_str(),
            self.population_total,
            self.tissue_count,
            self.assigned_members,
            self.unassigned_members,
            self.assignment_coverage
                .map(|c| format!("{c:.3}"))
                .unwrap_or_else(|| "n/a".into()),
            self.mean_energy
                .map(|e| format!("{e:.3}"))
                .unwrap_or_else(|| "n/a".into()),
            self.coherence_mean
                .map(|c| format!("{c:.3}"))
                .unwrap_or_else(|| "n/a".into()),
            self.provider_status,
            self.no_data_reason
                .as_ref()
                .map(|r| format!(" reason={r}"))
                .unwrap_or_default(),
        )
    }
}

/// Recibo de uma leitura da fronteira L2→L3 (consumidor registra o que
/// leu, de qual versão, com qual classificação — E4 consumido).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct L2ReadReceipt {
    /// Chave canônica lida (ex.: `l2.tissue.snapshot`).
    pub key: &'static str,
    /// Classificação da leitura.
    pub provider_status: Status,
    /// Identidade do provider consultado.
    pub provider_id: String,
    /// Se um fallback estrutural foi aplicado (nunca publicável).
    pub fallback_used: bool,
    /// Motivo do fallback/ausência.
    pub fallback_reason: Option<String>,
    /// Versão do snapshot lido.
    pub state_version: Option<u64>,
    /// Fase do snapshot lido.
    pub state_phase: Option<L1StatePhase>,
    /// Hash da ordem de tecidos lido (casa com o snapshot publicado).
    pub state_hash: Option<String>,
}

/// Constrói e classifica o snapshot tecidual a partir da matéria-prima.
///
/// `mean_energy`/`coherence_mean`: médias dos tecidos (`None` se não há
/// tecidos — ausência ≠ zero, nunca 0.0 fabricado).
/// `tissue_order`: ids na ordem canônica de formação (entra no hash).
#[allow(clippy::too_many_arguments)]
pub fn build_tissue_snapshot(
    state_version: u64,
    step: u64,
    population_total: u32,
    tissue_count: u32,
    assigned_members: u32,
    mean_energy: Option<f64>,
    coherence_mean: Option<f64>,
    tissue_order: &[TissueId],
) -> TissueSnapshot {
    // Inconsistência interna: atribuição maior que a população viva.
    if assigned_members > population_total {
        return TissueSnapshot {
            state_version,
            step,
            phase: L1StatePhase::PostTissue,
            population_total,
            tissue_count,
            assigned_members,
            unassigned_members: 0,
            assignment_coverage: None,
            mean_energy: None,
            coherence_mean: None,
            tissue_order_hash: None,
            provider_status: Status::Invalid,
            no_data_reason: Some("assignment_overflow".to_string()),
        };
    }

    if population_total == 0 {
        // Sem substrato não há organização: NoData com razão.
        return TissueSnapshot {
            state_version,
            step,
            phase: L1StatePhase::PostTissue,
            population_total: 0,
            tissue_count: 0,
            assigned_members: 0,
            unassigned_members: 0,
            assignment_coverage: None,
            mean_energy: None,
            coherence_mean: None,
            tissue_order_hash: None,
            provider_status: Status::NoData,
            no_data_reason: Some("population_absent".to_string()),
        };
    }

    let non_finite = mean_energy.map(|x| !x.is_finite()).unwrap_or(false)
        || coherence_mean.map(|x| !x.is_finite()).unwrap_or(false);
    if non_finite {
        return TissueSnapshot {
            state_version,
            step,
            phase: L1StatePhase::PostTissue,
            population_total,
            tissue_count,
            assigned_members,
            unassigned_members: population_total - assigned_members,
            assignment_coverage: None,
            mean_energy: None,
            coherence_mean: None,
            tissue_order_hash: None,
            provider_status: Status::Invalid,
            no_data_reason: Some("tissue_metrics_non_finite".to_string()),
        };
    }

    // População viva (possivelmente sem nenhum tecido): estado real.
    TissueSnapshot {
        state_version,
        step,
        phase: L1StatePhase::PostTissue,
        population_total,
        tissue_count,
        assigned_members,
        unassigned_members: population_total - assigned_members,
        assignment_coverage: Some(assigned_members as f32 / population_total as f32),
        mean_energy,
        coherence_mean,
        tissue_order_hash: hash_id_order(tissue_order).map(to_hex16),
        provider_status: Status::Value,
        no_data_reason: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use triad_foundation::id::TissueId;

    #[test]
    fn populacao_ausente_e_no_data_com_razao() {
        let s = build_tissue_snapshot(1, 0, 0, 0, 0, None, None, &[]);
        assert_eq!(s.provider_status, Status::NoData);
        assert_eq!(s.no_data_reason.as_deref(), Some("population_absent"));
        assert!(s.assignment_coverage.is_none());
    }

    #[test]
    fn populacao_sem_tecidos_e_value_cobertura_zero() {
        // Ausência de tecido é estado REAL: cobertura 0.0 e métricas None
        // (ausência ≠ zero — nunca fabrica energia 0.0).
        let s = build_tissue_snapshot(2, 1, 40, 0, 0, None, None, &[]);
        assert_eq!(s.provider_status, Status::Value);
        assert_eq!(s.assignment_coverage, Some(0.0));
        assert_eq!(s.unassigned_members, 40);
        assert!(s.mean_energy.is_none());
        assert!(s.coherence_mean.is_none());
        assert!(s.tissue_order_hash.is_none());
    }

    #[test]
    fn tecidos_presentes_publicam_hash_e_cobertura() {
        let ids: Vec<TissueId> = (0..4).map(|_| TissueId::new()).collect();
        let s = build_tissue_snapshot(3, 2, 40, 4, 32, Some(0.71), Some(0.55), &ids);
        assert_eq!(s.provider_status, Status::Value);
        assert!((s.assignment_coverage.unwrap() - 0.8).abs() < 1e-6);
        assert_eq!(s.unassigned_members, 8);
        assert!(s.tissue_order_hash.is_some());
        assert_eq!(s.mean_energy, Some(0.71));
    }

    #[test]
    fn metrica_nao_finita_e_invalid() {
        let ids = vec![TissueId::new()];
        let s = build_tissue_snapshot(4, 3, 10, 1, 10, Some(f64::NAN), Some(0.5), &ids);
        assert_eq!(s.provider_status, Status::Invalid);
        assert_eq!(s.no_data_reason.as_deref(), Some("tissue_metrics_non_finite"));
    }

    #[test]
    fn atribuicao_maior_que_populacao_e_invalid() {
        let s = build_tissue_snapshot(5, 4, 10, 2, 12, Some(0.5), Some(0.5), &[]);
        assert_eq!(s.provider_status, Status::Invalid);
        assert_eq!(s.no_data_reason.as_deref(), Some("assignment_overflow"));
    }

    #[test]
    fn fase_e_sempre_post_tissue() {
        let s = build_tissue_snapshot(6, 5, 10, 1, 10, Some(0.5), Some(0.5), &[]);
        assert_eq!(s.phase, L1StatePhase::PostTissue);
        assert_eq!(s.phase.as_str(), "POST_TISSUE");
        assert!(s.summary_line().contains("POST_TISSUE"));
    }
}
