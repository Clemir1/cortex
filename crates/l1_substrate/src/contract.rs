//! Ledger L1 — o contrato temporal de 4 fases com recibos append-only.
//!
//! Espelha o `l1_state_contract.py` legado (runs 741–752) com as
//! correções da auditoria: versão monotônica por ledger, publicação
//! classificada (Value/NoData/Invalid — nunca ausência silenciosa),
//! histórico limitado e **recibos de leitura** — o elo E3→E4 da escada
//! de evidência fica registrado: produtor publica (E3 productive),
//! consumidor lê (E4 consumed), efeito validado (E5) pertence ao
//! learning transversal, que o L1 não falsifica.
//!
//! Append-only: nada sai do ledger — recibos e snapshots só crescem
//! (histórico de snapshots limitado a 32 por memória; recibos contados
//! por tipo). "Apagar telemetria" não existe neste organismo.

use std::collections::VecDeque;
use triad_contracts::l1::{
    build_snapshot, L1ReadReceipt, L1Snapshot, L1StatePhase, SamplePolicy,
};
use triad_foundation::id::ClusterId;
use triad_foundation::status::Status;
use tracing::debug;

/// Ledger de publicações de UMA fronteira L1.
pub struct L1Ledger {
    version: u64,
    pub history: VecDeque<L1Snapshot>,
    pub read_receipts: Vec<L1ReadReceipt>,
    /// Publicações por fase (indexadas por ordinal-1).
    pub published_by_phase: [u64; 4],
    /// Recibos de leitura com `Value`.
    pub consumed_value: u64,
    /// Recibos de leitura com ausência (NoData/Invalid/Stale).
    pub consumed_absence: u64,
    /// Publicações `Value`.
    pub published_value: u64,
    /// Publicações com ausência classificada.
    pub published_absence: u64,
}

impl Default for L1Ledger {
    fn default() -> Self {
        Self::new()
    }
}

impl L1Ledger {
    pub fn new() -> Self {
        Self {
            version: 0,
            history: VecDeque::with_capacity(32),
            read_receipts: Vec::new(),
            published_by_phase: [0; 4],
            consumed_value: 0,
            consumed_absence: 0,
            published_value: 0,
            published_absence: 0,
        }
    }

    /// Versão atual do ledger (monotônica — nunca rebaixada).
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Publica um snapshot da fase, avançando a versão.
    /// Classificação vem de `build_snapshot` — o ledger não inventa status.
    pub fn publish(
        &mut self,
        step: u64,
        phase: L1StatePhase,
        population_total: u32,
        sample_count: u32,
        mean_state: Option<&[f64]>,
        cluster_order: &[ClusterId],
        sample_policy: SamplePolicy,
    ) -> L1Snapshot {
        self.version = self.version.wrapping_add(1);
        debug!(fase = phase.ordinal(), versao = self.version, passo = step, "l1.ledger transição de fase");
        let snap = build_snapshot(
            self.version,
            step,
            phase,
            population_total,
            sample_count,
            mean_state,
            cluster_order,
            sample_policy,
        );
        self.published_by_phase[(phase.ordinal() - 1) as usize] += 1;
        if snap.provider_status == Status::Value {
            self.published_value += 1;
        } else {
            self.published_absence += 1;
        }
        if self.history.len() == 32 {
            self.history.pop_front();
        }
        self.history.push_back(snap.clone());
        snap
    }

    /// Última publicação de uma fase (o consumidor lê isto).
    pub fn latest(&self, phase: L1StatePhase) -> Option<&L1Snapshot> {
        self.history.iter().rev().find(|s| s.phase == phase)
    }

    /// Registra o consumo da última publicação da fase e devolve o recibo
    /// (E4 consumed quando Value; ausência registrada como ausência).
    pub fn read(&mut self, phase: L1StatePhase, consumer: &'static str) -> L1ReadReceipt {
        let receipt = match self.history.iter().rev().find(|s| s.phase == phase) {
            None => L1ReadReceipt {
                key: consumer,
                provider_status: Status::NoData,
                provider_id: "l1_ledger".to_string(),
                fallback_used: false,
                fallback_reason: Some("phase_never_published".to_string()),
                state_version: None,
                state_phase: Some(phase),
                state_hash: None,
            },
            Some(s) => L1ReadReceipt {
                key: consumer,
                provider_status: s.provider_status,
                provider_id: "l1_ledger".to_string(),
                fallback_used: s.provider_status != Status::Value,
                fallback_reason: s.no_data_reason.clone(),
                state_version: Some(s.state_version),
                state_phase: Some(s.phase),
                state_hash: s.mean_state_hash.clone(),
            },
        };
        if receipt.provider_status == Status::Value {
            self.consumed_value += 1;
        } else {
            self.consumed_absence += 1;
        }
        self.read_receipts.push(receipt.clone());
        receipt
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config as cfg;

    #[test]
    fn versao_monotonica_por_publicacao() {
        let mut l = L1Ledger::new();
        let ids = [ClusterId::new(), ClusterId::new()];
        let mean = [0.5_f64; cfg::DIMENSIONALITY];
        let s1 = l.publish(1, L1StatePhase::PostPhysical, 2, 2, Some(&mean), &ids, SamplePolicy::All);
        assert_eq!(s1.state_version, 1);
        let s2 = l.publish(1, L1StatePhase::PreCognitive, 2, 2, Some(&mean), &ids, SamplePolicy::All);
        assert_eq!(s2.state_version, 2);
        assert!(l.version() >= 2);
        assert_eq!(l.published_by_phase[0], 1); // PostPhysical
        assert_eq!(l.published_by_phase[2], 1); // PreCognitive
        assert_eq!(l.published_value, 2);
    }

    #[test]
    fn publicacao_sem_estado_classifica_no_data() {
        let mut l = L1Ledger::new();
        let s = l.publish(5, L1StatePhase::PostPhysical, 0, 0, None, &[], SamplePolicy::All);
        assert_eq!(s.provider_status, Status::NoData);
        assert_eq!(s.no_data_reason.as_deref(), Some("state_absent"));
        assert_eq!(l.published_absence, 1);
        assert_eq!(l.published_value, 0);
    }

    #[test]
    fn leitura_registra_e4_e_ausencia_registrada() {
        let mut l = L1Ledger::new();
        let ids = [ClusterId::new()];
        let mean = [0.3_f64; cfg::DIMENSIONALITY];
        l.publish(1, L1StatePhase::PreCognitive, 1, 1, Some(&mean), &ids, SamplePolicy::All);
        let r = l.read(L1StatePhase::PreCognitive, "l3_attention");
        assert_eq!(r.provider_status, Status::Value);
        assert_eq!(r.state_version, Some(1));
        assert!(!r.fallback_used);
        assert_eq!(l.consumed_value, 1);
        // Fase nunca publicada: recibo de ausência com razão.
        let r2 = l.read(L1StatePhase::PostCognitive, "l2_tissue");
        assert_eq!(r2.provider_status, Status::NoData);
        assert_eq!(r2.fallback_reason.as_deref(), Some("phase_never_published"));
        assert_eq!(l.consumed_absence, 1);
    }

    #[test]
    fn historico_limitado_a_32_append_only() {
        let mut l = L1Ledger::new();
        let ids = [ClusterId::new()];
        for k in 0..40u64 {
            l.publish(k, L1StatePhase::PostPhysical, 1, 1, Some(&[0.1; cfg::DIMENSIONALITY]), &ids, SamplePolicy::All);
        }
        assert_eq!(l.history.len(), 32);
        assert_eq!(l.version(), 40, "versão conta TUDO, histórico é janela");
    }
}
