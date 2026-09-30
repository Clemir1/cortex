use tracing::debug;

/// Resultado da arbitragem: vencedora ou empate (empate ⇒ sem mudança).
#[derive(Debug, Clone, PartialEq)]
pub enum Tie {
    /// Proposta vencedora.
    Winner(String),
    /// Empate: nenhuma mudança.
    Tie,
}

/// Árbitro determinístico entre propostas pontuadas.
pub struct Arbiter {
    /// Marcador de tipo interno.
    _marker: std::marker::PhantomData<()>,
}

impl Arbiter {
    /// Cria um árbitro.
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }

    /// Arbitra as propostas; empate dentro de 1e-9 ⇒ Tie.
    pub fn arbitrate(&self, proposals: &[(String, f32)]) -> Tie {
        if proposals.is_empty() {
            return Tie::Tie;
        }
        let mut best = 0usize;
        let mut top1 = proposals[0].1;
        let mut top2 = f32::NEG_INFINITY;
        for (idx, (_, score)) in proposals.iter().enumerate().skip(1) {
            if *score > top1 {
                top2 = top1;
                top1 = *score;
                best = idx;
            } else if *score > top2 {
                top2 = *score;
            }
        }
        // Os dois maiores scores empatam dentro de 1e-9: sem mudança.
        if proposals.len() > 1 && (top1 - top2).abs() <= 1e-9 {
            debug!("empate na arbitragem: sem mudança");
            return Tie::Tie;
        }
        debug!(vencedora = proposals[best].0.as_str(), "proposta venceu a arbitragem");
        Tie::Winner(proposals[best].0.clone())
    }
}

// ============================================================
// 18.6 — ArbitrationEngine (CAMADA.txt:1172, "Novo"): conflitos
// de controller, actuator ownership, prioridade e resolução —
// DETERMINÍSTICO (prioridade desc, ordem de chegada asc; nunca
// rng) com TRILHA DE AUDITORIA tipada por perdedor.
// ============================================================

/// Pretendente à ownership de um atuador.
#[derive(Debug, Clone, PartialEq)]
pub struct Contender {
    /// Identidade do controller pretendente.
    pub controller: String,
    /// Atuador disputado (ex.: "inbox.l2.adaptation").
    pub actuator: String,
    /// Prioridade declarada (maior vence — política do caller,
    /// mecânica de comparação é daqui).
    pub priority: i32,
    /// Razão canônica da pretensão (trilha do vencedor).
    pub reason: String,
}

/// Razão tipada da perda (trilha de auditoria do perdedor).
#[derive(Debug, Clone, PartialEq)]
pub enum LossReason {
    /// Perdeu por prioridade menor que a do vencedor.
    LowerPriority {
        /// Prioridade do vencedor.
        winner_priority: i32,
    },
    /// Empatou em prioridade e chegou depois.
    LaterArrival {
        /// Ordem de chegada do vencedor (seq menor).
        winner_seq: u64,
    },
}

impl LossReason {
    /// Nome canônico da razão.
    pub fn as_str(&self) -> &'static str {
        match self {
            LossReason::LowerPriority { .. } => "lower_priority",
            LossReason::LaterArrival { .. } => "later_arrival",
        }
    }
}

/// Veredito por atuador (ausência ≠ zero: sem pretendentes é
/// NENHUMA decisão, não uma decisão vazia).
#[derive(Debug, Clone, PartialEq)]
pub struct ArbitrationRecord {
    /// Tick da resolução.
    pub tick: u64,
    /// Atuador arbitrado (ordem canônica por atuador).
    pub actuator: String,
    /// Controller que ganhou a ownership.
    pub winner: Option<String>,
    /// Prioridade do vencedor.
    pub winner_priority: Option<i32>,
    /// Razão canônica do vencedor.
    pub winner_reason: Option<String>,
    /// Perdedores com razão tipada (trilha de auditoria).
    pub losers: Vec<(String, LossReason)>,
}

impl ArbitrationRecord {
    /// Houve conflito real (2+ pretendentes)?
    pub fn had_conflict(&self) -> bool {
        !self.losers.is_empty()
    }
}

/// Estatísticas com denominador explícito (Lei 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArbitrationStats {
    /// Atuadores efetivamente arbitrados com vencedor.
    pub decisions: u64,
    /// Perdedores registrados com razão tipada.
    pub losers_recorded: u64,
    /// Perdedores por prioridade menor.
    pub by_lower_priority: u64,
    /// Perdedores por ordem de chegada.
    pub by_later_arrival: u64,
    /// Pretendentes ainda na fila (tick não resolvido).
    pub queued: u64,
    /// Recibos retidos (teto de trilha).
    pub records_kept: usize,
}

/// Motor de arbitragem determinístico por tick.
pub struct ArbitrationEngine {
    /// Fila de pretendentes desde o último resolve (seq = índice).
    queue: Vec<Contender>,
    /// Trilha de auditoria (teto 64, mais antigos saem).
    records: Vec<ArbitrationRecord>,
    decisions: u64,
    losers_recorded: u64,
    by_lower_priority: u64,
    by_later_arrival: u64,
}

impl Default for ArbitrationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ArbitrationEngine {
    /// Motor novo sem pretendentes.
    pub fn new() -> Self {
        Self {
            queue: Vec::new(),
            records: Vec::new(),
            decisions: 0,
            losers_recorded: 0,
            by_lower_priority: 0,
            by_later_arrival: 0,
        }
    }

    /// Registra um pretendente (a ordem de chamada é a ordem de
    /// chegada — determinismo por construção).
    pub fn submit(&mut self, contender: Contender) {
        self.queue.push(contender);
    }

    /// Resolve TODOS os conflitos enfileirados para o tick:
    /// agrupa por atuador em ordem canônica (BTreeMap), ordena
    /// cada grupo por (prioridade desc, chegada asc) e emite um
    /// recibo por atuador com a trilha tipada dos perdedores.
    pub fn resolve_tick(&mut self, tick: u64) -> Vec<ArbitrationRecord> {
        let mut by_actuator: std::collections::BTreeMap<String, Vec<(usize, Contender)>> =
            std::collections::BTreeMap::new();
        for (seq, c) in std::mem::take(&mut self.queue).into_iter().enumerate() {
            by_actuator.entry(c.actuator.clone()).or_default().push((seq, c));
        }
        let mut out = Vec::new();
        for (actuator, mut group) in by_actuator {
            // Determinismo total: prioridade desc, chegada asc.
            group.sort_by(|a, b| {
                b.1.priority
                    .cmp(&a.1.priority)
                    .then(a.0.cmp(&b.0))
                    .then(a.1.controller.cmp(&b.1.controller))
            });
            let (_, champion) = group[0].clone();
            let mut losers = Vec::new();
            for (seq, c) in group.iter().skip(1) {
                let reason = if c.priority < champion.priority {
                    self.by_lower_priority += 1;
                    LossReason::LowerPriority {
                        winner_priority: champion.priority,
                    }
                } else {
                    self.by_later_arrival += 1;
                    LossReason::LaterArrival {
                        winner_seq: group[0].0 as u64,
                    }
                };
                let _ = seq;
                losers.push((c.controller.clone(), reason));
            }
            self.losers_recorded += losers.len() as u64;
            self.decisions += 1;
            let record = ArbitrationRecord {
                tick,
                actuator,
                winner: Some(champion.controller.clone()),
                winner_priority: Some(champion.priority),
                winner_reason: Some(champion.reason.clone()),
                losers,
            };
            self.records.push(record.clone());
            if self.records.len() > 64 {
                self.records.remove(0);
            }
            out.push(record);
        }
        out
    }

    /// Estatísticas com denominador (decisões + perdedores).
    pub fn stats(&self) -> ArbitrationStats {
        ArbitrationStats {
            decisions: self.decisions,
            losers_recorded: self.losers_recorded,
            by_lower_priority: self.by_lower_priority,
            by_later_arrival: self.by_later_arrival,
            queued: self.queue.len() as u64,
            records_kept: self.records.len(),
        }
    }

    /// Trilha de auditoria retida (teto 64).
    pub fn records(&self) -> &[ArbitrationRecord] {
        &self.records
    }
}

#[cfg(test)]
mod arbitration_tests {
    use super::*;

    fn contender(controller: &str, priority: i32) -> Contender {
        Contender {
            controller: controller.to_string(),
            actuator: "inbox.l2.adaptation".to_string(),
            priority,
            reason: "realce top-down".to_string(),
        }
    }

    /// 18.6: conflito real resolvido por prioridade com trilha
    /// tipada do perdedor.
    #[test]
    fn conflito_resolvido_por_prioridade_com_trilha() {
        let mut engine = ArbitrationEngine::new();
        engine.submit(contender("conceito:azul", 5));
        engine.submit(contender("conceito:vermelho", 9));
        let records = engine.resolve_tick(10);
        assert_eq!(records.len(), 1, "um atuador, um recibo");
        let r = &records[0];
        assert!(r.had_conflict(), "conflito real: 2 pretendentes");
        assert_eq!(r.winner.as_deref(), Some("conceito:vermelho"));
        assert_eq!(r.winner_priority, Some(9));
        assert_eq!(r.tick, 10);
        assert_eq!(r.losers.len(), 1);
        assert_eq!(r.losers[0].0, "conceito:azul");
        assert!(
            matches!(
                &r.losers[0].1,
                LossReason::LowerPriority { winner_priority: 9 }
            ),
            "perdedor com razão tipada lower_priority(9)"
        );
        let stats = engine.stats();
        assert_eq!(stats.decisions, 1, "denominador: decisões");
        assert_eq!(stats.losers_recorded, 1);
        assert_eq!(stats.by_lower_priority, 1);
    }

    /// Empate de prioridade ⇒ ordem de chegada decide (seq
    /// menor vence), razão tipada later_arrival.
    #[test]
    fn empate_resolvido_por_ordem_de_chegada() {
        let mut engine = ArbitrationEngine::new();
        engine.submit(contender("primeiro", 7));
        engine.submit(contender("segundo", 7));
        let records = engine.resolve_tick(1);
        assert_eq!(records[0].winner.as_deref(), Some("primeiro"));
        assert!(
            matches!(
                &records[0].losers[0].1,
                LossReason::LaterArrival { winner_seq: 0 }
            ),
            "perdedor por chegada com razão tipada"
        );
        assert_eq!(engine.stats().by_later_arrival, 1);
    }

    /// Múltiplos atuadores: recibos em ordem canônica de atuador
    /// (determinismo do relatório).
    #[test]
    fn multiplos_atuadores_ordem_canonica() {
        let mut engine = ArbitrationEngine::new();
        engine.submit(Contender {
            controller: "b".into(),
            actuator: "zeta.actuator".into(),
            priority: 1,
            reason: "r".into(),
        });
        engine.submit(Contender {
            controller: "a".into(),
            actuator: "alpha.actuator".into(),
            priority: 2,
            reason: "r".into(),
        });
        let records = engine.resolve_tick(3);
        let actuators: Vec<&str> = records.iter().map(|r| r.actuator.as_str()).collect();
        assert_eq!(actuators, ["alpha.actuator", "zeta.actuator"]);
        assert!(!records[0].had_conflict());
        assert_eq!(records[0].losers.len(), 0, "sem conflito: sem perdedores");
    }

    /// A/A: motores gêmeos com a mesma sequência produzem a
    /// MESMA trilha (bit-idêntica).
    #[test]
    fn arbitragem_e_deterministica_aa() {
        let build = |mut e: ArbitrationEngine| {
            e.submit(contender("x", 3));
            e.submit(contender("y", 8));
            e.submit(contender("w", 8));
            e.submit(Contender {
                controller: "k".into(),
                actuator: "outro.atuador".into(),
                priority: 4,
                reason: "r".into(),
            });
            (e.resolve_tick(1), e.stats())
        };
        let (r1, s1) = build(ArbitrationEngine::new());
        let (r2, s2) = build(ArbitrationEngine::new());
        assert_eq!(r1, r2, "trilhas bit-idênticas");
        assert_eq!(s1, s2);
        assert_eq!(s1.decisions, 2);
        assert_eq!(s1.losers_recorded, 2);
    }

    /// Fila vazia ⇒ nenhuma decisão (ausência ≠ zero) e a fila
    /// drenada conta nos stats com denominador honesto.
    #[test]
    fn fila_vazia_nao_gera_decisao_fabricada() {
        let mut engine = ArbitrationEngine::new();
        assert!(engine.resolve_tick(2).is_empty());
        assert_eq!(engine.stats().decisions, 0, "nada decidido, nada contado");
        assert_eq!(engine.stats().queued, 0);
    }
}
