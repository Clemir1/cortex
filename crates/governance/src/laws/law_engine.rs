//! 17.11 (sessão 7) — LawEngine: as LEIS DA CASA como DADO e o
//! enforcement OBSERVACIONAL pós-tick (ESPEC repassada em
//! var/verificacao/17-11_ESPEC_lawengine_repassada.md).
//!
//! Contrato do circuito:
//! - Lei como DADO: cada lei é um `LawEntry` (id, descrição e
//!   função de checagem PURA) num `HardLawSet` — nunca código
//!   espalhado; adicionar lei = adicionar entrada.
//! - Enforcement OBSERVACIONAL: `LawEngine::audit(&obs)` roda
//!   PÓS-tick sobre a FOTOGRAFIA do passo (`LawObservation`) e
//!   REGISTRA violações com razão tipada — NUNCA é entrada
//!   causal, NUNCA bloqueia/desliga (Lei 4: crise muda política;
//!   o LawEngine audita, a POLÍTICA decide).
//! - Denominadores: cada auditoria incrementa `audits`; as
//!   violações por lei são sempre legíveis como x/audits.
//! - Soft law: a política "law_soft" (Lua, whitelist 17.12,
//!   faixa [0.5, 1.0] garantida por RUST) propõe o fator do
//!   orçamento de eventos sob pressão observada — economia de
//!   política, NUNCA suspensão.

use std::collections::BTreeMap;
use tracing::warn;

use crate::hard_law::{HardLawSet, Violation};

/// Fotografia observacional de um passo — o que as leis auditam.
/// Ausência de flag ≠ violação: campos default são o estado
/// LEGAL observado (ausência ≠ zero).
#[derive(Debug, Clone, Default)]
pub struct LawObservation {
    /// Passo lógico observado (denominador natural da janela).
    pub tick: u64,
    /// Módulos executados no passo (nomes canônicos).
    pub executed: Vec<String>,
    /// Sinal de orçamento de eventos estourado neste passo.
    pub budget_exceeded: bool,
    /// Módulo degradado por erro no tick (observável do report).
    pub degraded: Vec<String>,
    /// E0–E5: promoção sem evidência detectada.
    pub promotion_without_evidence: bool,
    /// Taxa declarada sem denominador detectada.
    pub rate_without_denominator: bool,
    /// Ausência transformada em valor (Qualified violado).
    pub absence_became_value: bool,
    /// Aprendizagem suspensa/desligada (Lei 4 — NUNCA).
    pub learning_suspended: bool,
    /// FALLBACK sem provider_id/razão (Lei 3).
    pub fallback_without_reason: bool,
    /// Intervenção sem ttl / sem reversão por efeito ausente (Lei 6).
    pub intervention_without_ttl: bool,
    /// Genome avaliado DENTRO do loop principal (Lei 7).
    pub genome_in_loop: bool,
    /// Região cerebral fora de extensions/full_brain (Lei 8).
    pub region_outside_extensions: bool,
}

impl LawObservation {
    /// Construtor observacional a partir do nome dos módulos
    /// executados no passo: a Lei 4 é verificável ESTRUTURALMENTE
    /// — learning ausente dos executados E marcado como pulado
    /// por política é SUSPENSÃO; ausente apenas por cadência é
    /// política legítima (o chamador decide o que é legítimo —
    /// aqui só se fotografa).
    pub fn from_step(
        tick: u64,
        executed: Vec<String>,
        budget_exceeded: bool,
        degraded: Vec<String>,
    ) -> Self {
        Self {
            tick,
            executed,
            budget_exceeded,
            degraded,
            ..Default::default()
        }
    }
}

/// Resultado de uma auditoria pós-tick (resumo legível).
#[derive(Debug, Clone, Default)]
pub struct LawAudit {
    pub violations: Vec<Violation>,
}

/// Telemetria do LawEngine — denominadores sempre visíveis.
#[derive(Debug, Clone, Default)]
pub struct LawStats {
    /// Auditorias pós-tick executadas (denominador de tudo).
    pub audits: u64,
    /// Violações totais registradas (≤ audits × leis).
    pub violations_total: u64,
    /// Violações POR LEI (x/audits cada).
    pub violations_by_law: BTreeMap<&'static str, u64>,
    /// Passos com orçamento estourado observados (pressão p/
    /// soft law; denominador audits).
    pub budget_exceeded_steps: u64,
}

/// LawEngine: enforcement observacional pós-tick das leis da
/// casa + gatilho da soft law de orçamento (política, nunca
/// suspensão). NUNCA muta estado de módulos: só AUDITA e REGISTRA.
pub struct LawEngine {
    hard: HardLawSet,
    stats: LawStats,
    /// Janela de pressão dos últimos 10 passos (orçamento).
    window: std::collections::VecDeque<bool>,
    /// Soft law da casa registrada no livro (17.11).
    pub soft_book: crate::soft_law::SoftLawBook,
}

impl LawEngine {
    /// Engine com as 8 leis da casa + soft law de orçamento.
    pub fn new() -> Self {
        let mut soft_book = crate::soft_law::SoftLawBook::new();
        if let Some(soft) = crate::soft_law::SoftLaw::new(
            "law_soft_budget",
            "17-11",
            "fator do orçamento de eventos sob pressão (Lei 4: política, nunca suspensão)",
            1.0,
        ) {
            soft_book.add(soft);
        }
        Self {
            hard: HardLawSet::house(),
            stats: LawStats::default(),
            window: std::collections::VecDeque::with_capacity(10),
            soft_book,
        }
    }

    /// Auditoria pós-tick OBSERVACIONAL: registra violações com
    /// razão e contadores com denominador. Nunca causa efeito.
    pub fn audit(&mut self, obs: &LawObservation) -> LawAudit {
        self.stats.audits += 1;
        if obs.budget_exceeded {
            self.stats.budget_exceeded_steps += 1;
        }
        self.window.push_back(obs.budget_exceeded);
        if self.window.len() > 10 {
            self.window.pop_front();
        }
        let violations = self.hard.check_all(obs);
        for v in &violations {
            warn!(lei = v.law_id, razao = %v.reason, "LawEngine: lei dura violada");
            *self.stats.violations_by_law.entry(v.law_id).or_insert(0) += 1;
        }
        self.stats.violations_total += violations.len() as u64;
        LawAudit { violations }
    }

    /// Pressão de orçamento da janela: passos com budget_exceeded
    /// / passos observados. None = janela sem dados (ausência ≠
    /// zero: a soft law NÃO propõe sem sinal real).
    pub fn budget_pressure(&self) -> Option<f64> {
        if self.window.is_empty() {
            return None;
        }
        let excedidos = self.window.iter().filter(|&&b| b).count() as f64;
        Some(excedidos / self.window.len() as f64)
    }

    /// Telemetria corrente (auditoria).
    pub fn stats(&self) -> LawStats {
        self.stats.clone()
    }

    /// Número de leis duras registradas (dado visível).
    pub fn law_count(&self) -> usize {
        self.hard.laws.len()
    }
}

impl Default for LawEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lei como DADO: as 8 leis da casa registram, auditam com
    /// denominador e violação forçada TIPODA carrega razão.
    #[test]
    fn leis_como_dado_auditam_com_denominador_e_razao() {
        let mut engine = LawEngine::new();
        assert_eq!(engine.law_count(), 8, "as oito leis da casa");
        assert_eq!(engine.soft_book.len(), 1, "soft law de orçamento registrada");
        // Passo LEGAL: zero violações, auditoria contada.
        let ok = LawObservation::from_step(1, vec!["l1.substrate".into()], false, vec![]);
        let a = engine.audit(&ok);
        assert!(a.violations.is_empty(), "passo legal ⇒ nenhuma violação");
        // Passo com VIOLAÇÕES FORÇADAS TIPADAS (ESPEC 17-11).
        let mut violado = LawObservation::from_step(2, vec![], true, vec![]);
        violado.learning_suspended = true;
        violado.genome_in_loop = true;
        violado.absence_became_value = true;
        violado.rate_without_denominator = true;
        let a2 = engine.audit(&violado);
        assert_eq!(a2.violations.len(), 4, "quatro leis violadas");
        for v in &a2.violations {
            assert!(!v.reason.is_empty(), "razão canônica presente");
        }
        let s = engine.stats();
        assert_eq!(s.audits, 2, "denominador: 2 auditorias");
        assert_eq!(s.violations_total, 4, "4 de (2×8) — denominador visível");
        assert_eq!(s.violations_by_law["learning_never_suspended"], 1);
        assert_eq!(s.violations_by_law["genome_offline"], 1);
        // Denominador respeitado: nenhuma lei viola mais que audits.
        for (_, n) in &s.violations_by_law {
            assert!(*n <= s.audits);
        }
    }

    /// Lei 4 por construção: o enforcement OBSERVA, nunca desliga
    /// — a auditoria não tem efeito causal (só contadores).
    #[test]
    fn enforcement_e_observacional_lei4() {
        let mut engine = LawEngine::new();
        let mut obs = LawObservation::from_step(1, vec!["learning".into()], false, vec![]);
        obs.learning_suspended = true;
        let before = engine.stats();
        let a = engine.audit(&obs);
        // A OBSERVAÇÃO não é mutada pelo engine (fotografia pura).
        assert!(obs.learning_suspended, "fotografia intacta");
        assert_eq!(a.violations.len(), 1);
        let after = engine.stats();
        assert_eq!(after.audits - before.audits, 1);
        assert!(after.violations_total >= before.violations_total);
        // Nenhum método do engine desativa módulo: structuralmente
        // a única saída é (LawAudit, LawStats) — dados, não efeitos.
    }

    /// A/A bit-exato: mesmas fotografia ⇒ mesmas violações e
    /// mesmos contadores (função pura das observações).
    #[test]
    fn audit_e_deterministico_aa() {
        let run = || {
            let mut e = LawEngine::new();
            for t in 1..=12 {
                let mut o = LawObservation::from_step(t, vec!["learning".into()], t % 4 == 0, vec![]);
                o.budget_exceeded = t % 4 == 0;
                if t == 8 {
                    o.intervention_without_ttl = true;
                }
                e.audit(&o);
            }
            (e.stats().audits, e.stats().violations_total, e.budget_pressure())
        };
        assert_eq!(run(), run(), "gêmeos bit-exatos (A/A)");
    }

    /// Pressão de orçamento: janela honesta com denominador e
    /// ausência ≠ zero (None sem dados).
    #[test]
    fn pressao_de_orcamento_tem_denominador() {
        let mut e = LawEngine::new();
        assert_eq!(e.budget_pressure(), None, "sem dados ⇒ ausência ≠ zero");
        for t in 0..10 {
            let o = LawObservation::from_step(t, vec![], t % 2 == 0, vec![]);
            e.audit(&o);
        }
        let p = e.budget_pressure().expect("janela cheia");
        assert!((p - 0.5).abs() < 1e-9, "5/10 passos ⇒ pressão 0.5");
        assert_eq!(e.stats().budget_exceeded_steps, 5, "denominador 10, 5 excedidos");
    }

    /// Soft law da casa: exige ADR e peso válido (skeleton vivo).
    #[test]
    fn soft_law_da_casa_tem_adr_e_peso() {
        let e = LawEngine::new();
        assert_eq!(e.soft_book.total_weight(), 1.0);
        assert!(crate::soft_law::SoftLaw::new("x", "", "sem ADR", 1.0).is_none());
    }
}
