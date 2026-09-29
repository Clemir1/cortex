//! Ledger L2: publicaÃ§Ãµes `POST_TISSUE` com versÃ£o monotÃ´nica e recibos
//! de leitura L2â†’L3 (append-only â€” quem publicou, quem consumiu, qual
//! classificaÃ§Ã£o). Espelha o `L1Ledger` do substrato: produtor E3,
//! consumidor E4, ausÃªncia registrada com razÃ£o, nunca fabricada.

use std::collections::VecDeque;

use triad_contracts::l2::{build_tissue_snapshot, L2ReadReceipt, TissueSnapshot};
use triad_foundation::id::TissueId;
use triad_foundation::status::Status;

use crate::config::L2Config;

/// Ledger de fronteira da camada L2.
pub struct L2Ledger {
    /// VersÃ£o monotÃ´nica (sobe a toda publicaÃ§Ã£o, nunca rebaixa).
    version: u64,
    /// HistÃ³rico recente de snapshots (append-only, capacidade fixa).
    history: VecDeque<TissueSnapshot>,
    /// Recibos de leitura registrados por consumidores.
    read_receipts: Vec<L2ReadReceipt>,
    /// Contadores de publicaÃ§Ã£o (a telemetria nunca mente).
    published_value: u64,
    published_absence: u64,
    /// Contadores de consumo.
    consumed_value: u64,
    consumed_absence: u64,
}

impl L2Ledger {
    pub fn new() -> Self {
        Self {
            version: 0,
            history: VecDeque::with_capacity(L2Config::LEDGER_HISTORY),
            read_receipts: Vec::new(),
            published_value: 0,
            published_absence: 0,
            consumed_value: 0,
            consumed_absence: 0,
        }
    }

    /// VersÃ£o corrente (0 = nada publicado).
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Contadores de publicaÃ§Ã£o (value, absence).
    pub fn published_counts(&self) -> (u64, u64) {
        (self.published_value, self.published_absence)
    }

    /// Contadores de consumo (value, absence).
    pub fn consumed_counts(&self) -> (u64, u64) {
        (self.consumed_value, self.consumed_absence)
    }

    /// Publica um snapshot da organizaÃ§Ã£o tecidual (fase `PostTissue`).
    /// A versÃ£o sobe SEMPRE â€” inclusive em NoData/Invalid (a ausÃªncia
    /// tambÃ©m versiona; consumers detectam o retreat).
    #[allow(clippy::too_many_arguments)]
    pub fn publish(
        &mut self,
        step: u64,
        population_total: u32,
        tissue_count: u32,
        assigned_members: u32,
        mean_energy: Option<f64>,
        coherence_mean: Option<f64>,
        tissue_order: &[TissueId],
    ) -> TissueSnapshot {
        self.version += 1;
        let snap = build_tissue_snapshot(
            self.version,
            step,
            population_total,
            tissue_count,
            assigned_members,
            mean_energy,
            coherence_mean,
            tissue_order,
        );
        if snap.provider_status == Status::Value {
            self.published_value += 1;
        } else {
            self.published_absence += 1;
        }
        self.history.push_front(snap.clone());
        while self.history.len() > L2Config::LEDGER_HISTORY {
            self.history.pop_back();
        }
        snap
    }

    /// Publica ausÃªncia com a razÃ£o da FONTE (quando o L1 nÃ£o publicou
    /// POST_PHYSICAL vÃ¡lido, o L2 nÃ£o deriva visÃ£o â€” ausÃªncia â‰  zero).
    /// A versÃ£o sobe tambÃ©m aqui: a ausÃªncia versiona.
    pub fn publish_no_data(&mut self, step: u64, reason: &str) -> TissueSnapshot {
        self.version += 1;
        self.published_absence += 1;
        let snap = TissueSnapshot {
            state_version: self.version,
            step,
            phase: triad_contracts::l1::L1StatePhase::PostTissue,
            population_total: 0,
            tissue_count: 0,
            assigned_members: 0,
            unassigned_members: 0,
            assignment_coverage: None,
            mean_energy: None,
            coherence_mean: None,
            tissue_order_hash: None,
            provider_status: Status::NoData,
            no_data_reason: Some(reason.to_string()),
        };
        self.history.push_front(snap.clone());
        while self.history.len() > L2Config::LEDGER_HISTORY {
            self.history.pop_back();
        }
        snap
    }

    /// Snapshot mais recente (qualquer classificaÃ§Ã£o).
    pub fn latest(&self) -> Option<&TissueSnapshot> {
        self.history.front()
    }

    /// Leitura instrumentada da fronteira L2â†’L3: devolve recibo E4 e
    /// registra consumo. `key` Ã© a chave canÃ´nica consultada.
    pub fn read(&mut self, key: &'static str) -> L2ReadReceipt {
        let (receipt, is_value) = match self.history.front() {
            None => (
                L2ReadReceipt {
                    key,
                    provider_status: Status::NoData,
                    provider_id: "l2_tissue".to_string(),
                    fallback_used: false,
                    fallback_reason: Some("phase_never_published".to_string()),
                    state_version: None,
                    state_phase: None,
                    state_hash: None,
                },
                false,
            ),
            Some(s) => (
                L2ReadReceipt {
                    key,
                    provider_status: s.provider_status,
                    provider_id: "l2_tissue".to_string(),
                    fallback_used: false,
                    fallback_reason: s.no_data_reason.clone(),
                    state_version: Some(s.state_version),
                    state_phase: Some(s.phase),
                    state_hash: s.tissue_order_hash.clone(),
                },
                s.provider_status == Status::Value,
            ),
        };
        if is_value {
            self.consumed_value += 1;
        } else {
            self.consumed_absence += 1;
        }
        self.read_receipts.push(receipt.clone());
        receipt
    }

    /// Todos os recibos emitidos (trilha de auditoria).
    pub fn receipts(&self) -> &[L2ReadReceipt] {
        &self.read_receipts
    }

    /// HistÃ³rico (mais recente primeiro).
    pub fn history(&self) -> &VecDeque<TissueSnapshot> {
        &self.history
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use triad_contracts::l1::L1StatePhase;

    fn id(seq: u64) -> TissueId {
        TissueId::from_uuid(uuid::Uuid::from_u64_pair(21, seq))
    }

    #[test]
    fn publicacao_versiona_monotonicamente_mesmo_em_no_data() {
        let mut l = L2Ledger::new();
        let s1 = l.publish(0, 0, 0, 0, None, None, &[]);
        assert_eq!(s1.state_version, 1);
        assert_eq!(s1.provider_status, Status::NoData);
        let s2 = l.publish(1, 10, 2, 8, Some(0.7), Some(0.4), &[id(1), id(2)]);
        assert_eq!(s2.state_version, 2);
        assert_eq!(s2.phase, L1StatePhase::PostTissue);
        assert_eq!(l.version(), 2);
        assert_eq!(l.published_counts(), (1, 1));
    }

    #[test]
    fn leitura_sem_publicacao_e_no_data_com_razao() {
        let mut l = L2Ledger::new();
        let r = l.read("l2.tissue.snapshot");
        assert_eq!(r.provider_status, Status::NoData);
        assert_eq!(r.fallback_reason.as_deref(), Some("phase_never_published"));
        assert_eq!(l.consumed_counts(), (0, 1));
    }

    #[test]
    fn leitura_de_value_registra_recibo_e4() {
        let mut l = L2Ledger::new();
        l.publish(0, 10, 2, 8, Some(0.7), Some(0.4), &[id(1), id(2)]);
        let r = l.read("l2.tissue.snapshot");
        assert_eq!(r.provider_status, Status::Value);
        assert_eq!(r.state_version, Some(1));
        assert_eq!(r.state_phase, Some(L1StatePhase::PostTissue));
        assert!(r.state_hash.is_some());
        assert_eq!(l.consumed_counts(), (1, 0));
        assert_eq!(l.receipts().len(), 1);
    }

    #[test]
    fn historico_e_apend_only_com_capacidade_fixa() {
        let mut l = L2Ledger::new();
        for step in 0..(L2Config::LEDGER_HISTORY as u64 + 10) {
            l.publish(step, 10, 1, 10, Some(0.5), Some(0.5), &[id(1)]);
        }
        assert_eq!(l.history().len(), L2Config::LEDGER_HISTORY);
        // O mais recente estÃ¡ na frente; versÃ£o nunca rebaixou.
        assert_eq!(l.latest().unwrap().step, L2Config::LEDGER_HISTORY as u64 + 9);
        assert_eq!(l.version(), L2Config::LEDGER_HISTORY as u64 + 10);
    }
}
