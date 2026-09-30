//! Ledger de evidência (16.9): recibos por módulo com o nível
//! DECLARADO vs o ALVO da config — gap é tipado, nunca promoção
//! automática (Lei 1: a escada só sobe com verificação). Relatórios
//! SEMPRE com denominador.

use triad_foundation::evidence::EvidenceLevel;

/// Teto de recibos retidos (auditoria janelada; o diário completo
/// vive em var/system.log — este ledger é a auditoria viva).
const LEDGER_CAP: usize = 256;

/// Recibo de evidência de um módulo em uma auditoria.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceReceipt {
    /// Nome canônico do módulo.
    pub module: String,
    /// Nível DECLARADO no descritor (E0..E5).
    pub declared: EvidenceLevel,
    /// Alvo da config `[telemetry.modules]` (ou default).
    pub target: EvidenceLevel,
    /// Declarado ≥ alvo (conforme); abaixo = GAP tipado.
    pub compliant: bool,
    /// Tick da auditoria.
    pub tick: u64,
}

/// Ledger de recibos com relatório denominado.
#[derive(Debug, Default)]
pub struct EvidenceLedger {
    receipts: Vec<EvidenceReceipt>,
}

impl EvidenceLedger {
    /// Novo ledger vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra um recibo (cap FIFO).
    pub fn record(&mut self, receipt: EvidenceReceipt) {
        if self.receipts.len() >= LEDGER_CAP {
            self.receipts.remove(0);
        }
        self.receipts.push(receipt);
    }

    /// Últimos recibos (ordem de inserção).
    pub fn receipts(&self) -> &[EvidenceReceipt] {
        &self.receipts
    }

    /// Relatório da ÚLTIMA auditoria: (conformes, total) —
    /// denominador explícito; 0 auditados = ausência, não zero.
    pub fn last_audit_counts(&self) -> Option<(u64, u64)> {
        // A última auditoria tem o maior tick registrado.
        let last_tick = self.receipts.last().map(|r| r.tick)?;
        let batch: Vec<&EvidenceReceipt> = self
            .receipts
            .iter()
            .filter(|r| r.tick == last_tick)
            .collect();
        let compliant = batch.iter().filter(|r| r.compliant).count() as u64;
        Some((compliant, batch.len() as u64))
    }

    /// Gaps tipados da última auditoria (módulos abaixo do alvo).
    pub fn last_audit_gaps(&self) -> Vec<EvidenceReceipt> {
        let Some(last_tick) = self.receipts.last().map(|r| r.tick) else {
            return Vec::new();
        };
        self.receipts
            .iter()
            .filter(|r| r.tick == last_tick && !r.compliant)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(module: &str, declared: EvidenceLevel, target: EvidenceLevel) -> EvidenceReceipt {
        EvidenceReceipt {
            module: module.to_string(),
            declared,
            target,
            compliant: declared >= target,
            tick: 10,
        }
    }

    #[test]
    fn relatorio_tem_denominador_e_gaps_tipados() {
        let mut l = EvidenceLedger::new();
        l.record(rec("l1", EvidenceLevel::E5EffectValidated, EvidenceLevel::E5EffectValidated));
        l.record(rec("l2", EvidenceLevel::E3Productive, EvidenceLevel::E5EffectValidated));
        let (compliant, total) = l.last_audit_counts().expect("auditoria presente");
        assert_eq!((compliant, total), (1, 2), "denominador explícito");
        let gaps = l.last_audit_gaps();
        assert_eq!(gaps.len(), 1, "gap tipado, não silenciado");
        assert_eq!(gaps[0].module, "l2");
        assert_eq!(gaps[0].declared, EvidenceLevel::E3Productive);
    }

    #[test]
    fn sem_auditoria_e_ausencia_nao_zero() {
        let l = EvidenceLedger::new();
        assert!(l.last_audit_counts().is_none(), "ausência ≠ zero");
        assert!(l.last_audit_gaps().is_empty());
    }

    #[test]
    fn cap_fifo_mantem_o_ledger_bounded() {
        let mut l = EvidenceLedger::new();
        for i in 0..300 {
            l.record(rec(&format!("m{i}"), EvidenceLevel::E3Productive, EvidenceLevel::E3Productive));
        }
        assert!(l.receipts().len() <= 256, "bounded");
        assert_eq!(l.receipts().last().unwrap().module, "m299");
    }
}
