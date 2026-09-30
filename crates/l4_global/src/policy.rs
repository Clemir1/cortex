//! Política de intervenção do L4 (seção 16.7, Lei 6 executável).
//!
//! O L5 NUNCA muta o estado do L4: SUBMETE uma proposta de mudança
//! de política a um inbox (protocolo — espelho do
//! `submit_reinforcement` L4→L3). O L4, DONO do próprio estado,
//! valida (range, TTL, alvo duplicado, teto), APLICA com o valor
//! anterior guardado e REVERTE quando o TTL esgota sem efeito
//! observado (Lei 6: reversão obrigatória, com razão tipada).
//! Crise muda POLÍTICA, nunca desativa sistemas (Lei 4) — o alvo
//! `CommitThreshold` é um LIMIAR de decisão, não um interruptor.

use std::collections::HashMap;

use triad_foundation as tf;

/// Teto de políticas ATIVAS simultâneas (todas com o mesmo peso:
/// nada de política monopolista). Declarado antes de medir.
pub const MAX_ACTIVE_POLICIES: usize = 4;
/// Faixa válida de TTL das propostas (ticks). Fora disso: recusa
/// tipada — o dono nunca aceita prazo absurdo.
pub const TTL_RANGE: std::ops::RangeInclusive<u32> = 1..=64;

/// Alvo de política que o L4 aceita (extensível: um alvo novo é um
/// enum novo + consumo no ponto de decisão — nunca string solta).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PolicyTarget {
    /// Limiar de confiança para commit de decisão
    /// (`[l4.decision] commit_threshold`).
    CommitThreshold,
}

impl PolicyTarget {
    /// Chave canônica do alvo (auditoria/instrumentação).
    pub fn key(&self) -> &'static str {
        match self {
            Self::CommitThreshold => "l4.decision.commit_threshold",
        }
    }
}

/// Proposta de mudança de política (protocolo L5→L4, fila no dono).
#[derive(Debug, Clone)]
pub struct PolicyProposal {
    /// Intervenção de origem (traçabilidade L5 — Lei 6 auditável).
    pub intervention_id: tf::ModuleId,
    /// Alvo da política.
    pub target: PolicyTarget,
    /// Valor corrente DEKLARADO pelo proponente (auditoria).
    pub current: f32,
    /// Valor proposto (0.0..=1.0).
    pub proposed: f32,
    /// Motivo exigido (nunca mudança de política sem razão).
    pub reason: String,
    /// Prazo em ticks: sem efeito observado ⇒ revert.
    pub ttl_ticks: u32,
    /// Tick de emissão (auditoria).
    pub issued_tick: u64,
}

/// Recusa tipada da proposta (nunca um booleano mudo).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyReject {
    /// Já existe política ATIVA (ou pendente) para o alvo.
    DuplicateTarget,
    /// Teto de políticas ativas simultâneas.
    Full,
    /// Valor proposto fora de 0.0..=1.0.
    OutOfRange,
    /// TTL fora da faixa válida.
    InvalidTtl,
}

impl PolicyReject {
    /// Nome canônico da recusa.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DuplicateTarget => "REJECTED_DUPLICATE_TARGET",
            Self::Full => "REJECTED_FULL",
            Self::OutOfRange => "REJECTED_OUT_OF_RANGE",
            Self::InvalidTtl => "REJECTED_INVALID_TTL",
        }
    }
}

/// Política ATIVA: proposta aplicada com o valor anterior guardado
/// para a reversão exata (Lei 6: o estado volta ao que era).
#[derive(Debug, Clone)]
pub struct ActivePolicy {
    /// Proposta aplicada.
    pub proposal: PolicyProposal,
    /// Valor ANTES da aplicação (reversão exata).
    pub previous: f32,
    /// Tick em que expira (issued + ttl).
    pub expires_tick: u64,
}

/// Inbox de políticas do L4: valida, aplica, expira e reverte.
/// Determinístico por construção: fila na ordem de chegada, um
/// alvo ativo por vez, reversão para o `previous` guardado.
pub struct PolicyInbox {
    pending: Vec<PolicyProposal>,
    active: HashMap<String, ActivePolicy>,
}

impl PolicyInbox {
    /// Inbox vazio.
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
            active: HashMap::new(),
        }
    }

    /// Valida e enfileira uma proposta (o APPLY acontece no drain
    /// do próximo tick do dono — nunca inline na submissão).
    pub fn submit(&mut self, p: PolicyProposal) -> Result<(), PolicyReject> {
        if !(0.0..=1.0).contains(&p.proposed) {
            return Err(PolicyReject::OutOfRange);
        }
        if !TTL_RANGE.contains(&p.ttl_ticks) {
            return Err(PolicyReject::InvalidTtl);
        }
        if self.active.contains_key(p.target.key())
            || self.pending.iter().any(|q| q.target == p.target)
        {
            return Err(PolicyReject::DuplicateTarget);
        }
        if self.active.len() + self.pending.len() >= MAX_ACTIVE_POLICIES {
            return Err(PolicyReject::Full);
        }
        self.pending.push(p);
        Ok(())
    }

    /// Propostas pendentes (auditoria).
    pub fn pending(&self) -> &[PolicyProposal] {
        &self.pending
    }

    /// Políticas ativas (auditoria).
    pub fn active(&self) -> impl Iterator<Item = &ActivePolicy> {
        self.active.values()
    }

    /// APPLICA as pendentes no tick corrente do dono: captura o
    /// `previous` (valor efetivo REAL no momento da aplicação —
    /// não o declarado pelo proponente) e agenda a expiração.
    /// Devolve as políticas aplicadas neste tick (recibo).
    pub fn drain(&mut self, tick: u64, effective_now: f32) -> Vec<ActivePolicy> {
        let mut applied = Vec::new();
        for p in std::mem::take(&mut self.pending) {
            if self.active.contains_key(p.target.key()) {
                // Duplicada entre submissão e drain: recusa silenciosa
                // NÃO — contada pelo chamador via comprimento do recibo.
                continue;
            }
            let ap = ActivePolicy {
                previous: effective_now,
                expires_tick: tick + p.ttl_ticks as u64,
                proposal: p,
            };
            self.active.insert(ap.proposal.target.key().to_string(), ap.clone());
            applied.push(ap);
        }
        applied
    }

    /// EXPIRA as políticas cujo TTL esgotou: devolve-as para o
    /// recibo de REVERSÃO (Lei 6 — o valor volta ao `previous`).
    pub fn expire(&mut self, tick: u64) -> Vec<ActivePolicy> {
        let mut reverted = Vec::new();
        let expired: Vec<String> = self
            .active
            .iter()
            .filter(|(_, ap)| tick >= ap.expires_tick)
            .map(|(k, _)| k.clone())
            .collect();
        for k in expired {
            if let Some(ap) = self.active.remove(&k) {
                reverted.push(ap);
            }
        }
        reverted
    }

    /// Valor EFETIVO do alvo: override ativo ou base da config.
    pub fn effective(&self, target: PolicyTarget, base: f32) -> f32 {
        self.active
            .get(target.key())
            .map(|ap| ap.proposal.proposed)
            .unwrap_or(base)
    }

    /// Reversão ANTECIPADA por efeito observado (o L5 manteve a
    /// intervenção: a política vira permanente? NÃO — o override
    /// expira por TTL e o valor observado vai para o learning; a
    /// política PERMANENTE é decisão do dono via config). Aqui:
    /// remove o override ativo devolvendo o `previous` (recibo).
    pub fn release(&mut self, target: PolicyTarget) -> Option<ActivePolicy> {
        self.active.remove(target.key())
    }
}

impl Default for PolicyInbox {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proposta(ttl: u32, proposed: f32) -> PolicyProposal {
        PolicyProposal {
            intervention_id: tf::ModuleId::new(),
            target: PolicyTarget::CommitThreshold,
            current: 0.5,
            proposed,
            reason: "confirmacao abaixo de 0.5".to_string(),
            ttl_ticks: ttl,
            issued_tick: 10,
        }
    }

    #[test]
    fn aplica_captura_previous_real_e_reverte_exato() {
        let mut inbox = PolicyInbox::new();
        assert!(inbox.submit(proposta(3, 0.35)).is_ok());
        // Previous é o EFETIVO no drain (0.50), não o declarado.
        let applied = inbox.drain(10, 0.50);
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0].previous, 0.50);
        assert_eq!(applied[0].expires_tick, 13);
        assert_eq!(inbox.effective(PolicyTarget::CommitThreshold, 0.50), 0.35);
        // Lei 6: TTL esgota ⇒ reversão EXATA para o previous.
        assert!(inbox.expire(12).is_empty(), "ainda dentro do TTL");
        let reverted = inbox.expire(13);
        assert_eq!(reverted.len(), 1);
        assert_eq!(reverted[0].previous, 0.50);
        assert_eq!(inbox.effective(PolicyTarget::CommitThreshold, 0.50), 0.50);
        assert!(inbox.active().next().is_none());
    }

    #[test]
    fn recusas_tipadas_range_ttl_duplicada() {
        let mut inbox = PolicyInbox::new();
        assert_eq!(
            inbox.submit(proposta(3, 1.5)),
            Err(PolicyReject::OutOfRange)
        );
        assert_eq!(
            inbox.submit(proposta(0, 0.4)),
            Err(PolicyReject::InvalidTtl)
        );
        assert_eq!(
            inbox.submit(proposta(99, 0.4)),
            Err(PolicyReject::InvalidTtl)
        );
        assert!(inbox.submit(proposta(3, 0.4)).is_ok());
        // Duplicada PENDENTE: mesmo alvo na fila.
        assert_eq!(
            inbox.submit(proposta(3, 0.3)),
            Err(PolicyReject::DuplicateTarget)
        );
        inbox.drain(10, 0.5);
        // Duplicada ATIVA: mesmo alvo aplicado.
        assert_eq!(
            inbox.submit(proposta(3, 0.2)),
            Err(PolicyReject::DuplicateTarget)
        );
        // NOTA honesta: com um ÚNICO PolicyTarget o teto
        // (MAX_ACTIVE_POLICIES) é inatingível — a duplicada dispara
        // antes. O caminho Full fica coberto quando existirem 2+
        // alvos reais (extensão do enum).
        assert_eq!(inbox.pending().len(), 0);
        assert_eq!(inbox.active().count(), 1);
    }

    #[test]
    fn a_a_determinismo_do_inbox() {
        // Mesma sequência de operações ⇒ mesmos efeitos (A/A).
        let run = || {
            let mut i = PolicyInbox::new();
            let mut log: Vec<(usize, f32)> = Vec::new();
            for tick in 10..=16 {
                if tick == 10 {
                    i.submit(proposta(3, 0.35)).expect("ok");
                }
                log.push((i.drain(tick, 0.50).len(), 0.0));
                let rev = i.expire(tick);
                log.push((rev.len(), rev.first().map(|a| a.previous).unwrap_or(0.0)));
                log.push((
                    0,
                    i.effective(PolicyTarget::CommitThreshold, 0.50),
                ));
            }
            log
        };
        assert_eq!(run(), run(), "mesma sequência ⇒ mesmo inbox (bit-idêntico)");
    }

    #[test]
    fn release_antecipado_devolve_previous_sem_esperar_ttl() {
        let mut inbox = PolicyInbox::new();
        inbox.submit(proposta(60, 0.30)).expect("ok");
        assert_eq!(inbox.drain(10, 0.50).len(), 1);
        assert_eq!(inbox.effective(PolicyTarget::CommitThreshold, 0.50), 0.30);
        let rel = inbox.release(PolicyTarget::CommitThreshold).expect("ativa");
        assert_eq!(rel.previous, 0.50);
        assert_eq!(inbox.effective(PolicyTarget::CommitThreshold, 0.50), 0.50);
    }
}
