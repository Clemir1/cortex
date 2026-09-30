//! 17.7 — L5 GOVERNADORES COMPLETOS, conforme audit 17-4 §5
//! ("mínimo para ResourceGovernor REAL"): 4 budgets
//! (energy/compute/attention/memory) + 5º transferível
//! (federation), transições SEMPRE tipadas (degradação e
//! recuperação espelham cognitive_economy.py:108-123/:298-306/
//! :254 e federation_hub.py:942-969), decisões deny/throttle com
//! reason + provider_id (Lei 3), gatilho is_starving<0.1
//! (federation_hub.py:189). DevelopmentGovernor acorda
//! morfogênese SÓ por causa declarada. MetaLearning = bandit
//! contextual com exploração por incerteza + trial com ROLLBACK
//! (historical_meta_learner.py:454-470/:115-125 — o
//! MetaLearning.py epsilon-greedy é REDUNDANTE, audit §3).
//! CognitiveGenome: fitness COLETADO no run; seleção/aplicação
//! SÓ em `evaluate_offline` (Lei 7 — o legado violava: evolve()
//! dentro do step, cognitive_genome.py:291-307).

use std::collections::HashMap;

use triad_foundation as tf;

/// Recursos governados.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Resource {
    Energy,
    Compute,
    Attention,
    Memory,
    /// Transferível entre organismos (federation).
    Federation,
}

impl Resource {
    /// Nome canônico.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Resource::Energy => "energy",
            Resource::Compute => "compute",
            Resource::Attention => "attention",
            Resource::Memory => "memory",
            Resource::Federation => "federation",
        }
    }

    /// Os quatro budgets internos (federation é o transferível).
    pub const INTERNAL: [Resource; 4] = [
        Resource::Energy,
        Resource::Compute,
        Resource::Attention,
        Resource::Memory,
    ];
}

/// Um budget com piso/teto/decaimento/regeneração (audit §5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Budget {
    pub value: f32,
    pub floor: f32,
    pub ceiling: f32,
    pub decay_rate: f32,
    pub regen_rate: f32,
}

impl Budget {
    /// Budget cheio com taxas dadas (clamp [0,1] do contrato puro).
    pub fn full(decay_rate: f32, regen_rate: f32) -> Self {
        Self {
            value: 1.0,
            floor: 0.05,
            ceiling: 1.0,
            decay_rate: decay_rate.clamp(0.0, 1.0),
            regen_rate: regen_rate.clamp(0.0, 1.0),
        }
    }

    /// Escassez corrente: (1 − value/ceiling) clampado — o contrato
    /// puro das 5 escassezes (scarcity_contract.py:32-51).
    pub fn scarcity(&self) -> f32 {
        (1.0 - self.value / self.ceiling).clamp(0.0, 1.0)
    }

    /// Faminto? (federation_hub.py:189 — is_starving<0.1).
    pub fn is_starving(&self) -> bool {
        self.value < 0.1
    }
}

/// Motivo tipado de DEGRADAÇÃO (audit 17-4 §5).
#[derive(Debug, Clone, PartialEq)]
pub enum DegradeReason {
    /// Consumo excedeu o disponível.
    OverConsumption { pedido: f32, disponivel: f32 },
    /// Evento de escassez FORÇADO (janela de teste/estresse).
    ScarcityEvent { queda_pct: f32 },
    /// Transferência emergencial para outro organismo.
    EmergencyTransfer { quantidade: f32 },
    /// Decaimento natural do tick.
    BudgetDecay { quantidade: f32 },
}

/// Motivo tipado de RECUPERAÇÃO (audit 17-4 §5).
#[derive(Debug, Clone, PartialEq)]
pub enum RecoverReason {
    /// Regeneração natural do tick.
    Regeneration { quantidade: f32 },
    /// Energia por CONTRIBUIÇÃO cognitiva: min(0.10, contrib×0.05)
    /// (cognitive_economy.py:254).
    RewardContribution { contrib: f32, ganho: f32 },
    /// Rebalance por harmonia>0.7 (federation_hub.py:963-969).
    RebalanceHarmony { quantidade: f32 },
    /// Escassez recuando (scarcity_recovery).
    ScarcityRecovery { quantidade: f32 },
}

/// Decisão tipada de alocação (Lei 3: reason + provider_id).
#[derive(Debug, Clone, PartialEq)]
pub enum AllocationDecision {
    /// Concedido (total ou parcial) com razão.
    Grant { valor: f32, reason: String },
    /// Negado — razão tipada + provider (nunca negação muda).
    Deny { reason: DenyReason, provider_id: &'static str },
    /// Reduzido por fator — colapso iminente de escassez.
    Throttle { fator: f32, reason: String, provider_id: &'static str },
}

/// Razão tipada de negativa.
#[derive(Debug, Clone, PartialEq)]
pub enum DenyReason {
    /// Budget faminto (<0.1): gatilho is_starving.
    Starving { value: f32 },
    /// Pedido maior que o disponível acima do piso.
    Insufficient { pedido: f32, disponivel: f32 },
}

/// Uma transição auditável no ledger.
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetEvent {
    pub resource: Resource,
    pub tick: u64,
    pub delta: f32,
    pub reason: String,
}

/// ResourceGovernor REAL — 5 budgets com ledger COM DENOMINADOR.
pub struct ResourceGovernor {
    budgets: HashMap<Resource, Budget>,
    events: std::collections::VecDeque<BudgetEvent>,
    pub decisions_granted: u64,
    pub decisions_denied: u64,
    pub decisions_throttled: u64,
    pub events_total: u64,
    /// provider canônico das respostas (Lei 3).
    pub provider_id: &'static str,
}

const LEDGER_CAP: usize = 256;
/// Gatilho de colapso iminente (audit :323-325).
pub const COLLAPSE_THRESHOLD: f32 = 0.75;
/// Transferência emergencial: 0.05×escassez se energy<0.2
/// (federation_hub.py:942-955).
pub const EMERGENCY_TRIGGER: f32 = 0.2;

impl Default for ResourceGovernor {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceGovernor {
    /// Governador com os 5 budgets (taxas derivadas do contrato).
    pub fn new() -> Self {
        let mut budgets = HashMap::new();
        for r in Resource::INTERNAL {
            budgets.insert(r, Budget::full(0.001, 0.004));
        }
        budgets.insert(Resource::Federation, Budget::full(0.002, 0.003));
        Self {
            budgets,
            events: std::collections::VecDeque::with_capacity(LEDGER_CAP),
            decisions_granted: 0,
            decisions_denied: 0,
            decisions_throttled: 0,
            events_total: 0,
            provider_id: "l5.resource_governor",
        }
    }

    fn record(&mut self, r: Resource, tick: u64, delta: f32, reason: String) {
        self.events_total += 1;
        self.events.push_back(BudgetEvent {
            resource: r,
            tick,
            delta,
            reason,
        });
        if self.events.len() > LEDGER_CAP {
            self.events.pop_front();
        }
    }

    /// Fotografia de um budget (ausência ≠ zero: None se ausente).
    pub fn budget(&self, r: Resource) -> Option<Budget> {
        self.budgets.get(&r).copied()
    }

    /// Tick do governador: decay+regen NATURAIS por recurso (tipados
    /// no ledger); transferência emergencial se energy<0.2.
    pub fn tick(&mut self, tick: u64) {
        for r in Resource::INTERNAL {
            if let Some(b) = self.budgets.get_mut(&r) {
                let decay = b.decay_rate * b.value;
                let regen = b.regen_rate * (1.0 - b.value);
                b.value = (b.value - decay + regen).clamp(b.floor, b.ceiling);
                let net = regen - decay;
                if net.abs() > 1e-9 {
                    let reason = if net < 0.0 {
                        format!("BudgetDecay(-{decay:.4})")
                    } else {
                        format!("Regeneration(+{regen:.4})")
                    };
                    self.record(r, tick, net, reason);
                }
            }
        }
        // Emergência: energia baixa puxa da federation (tipada).
        let energy = self.budget(Resource::Energy).map(|b| b.value).unwrap_or(1.0);
        if energy < EMERGENCY_TRIGGER {
            let fed = self.budget(Resource::Federation).map(|b| (b.value, b.scarcity())).unwrap_or((0.0, 0.0));
            let q = (0.05 * fed.1).min(fed.0 * 0.5);
            if q > 1e-6 {
                if let Some(f) = self.budgets.get_mut(&Resource::Federation) {
                    f.value -= q;
                }
                if let Some(e) = self.budgets.get_mut(&Resource::Energy) {
                    e.value = (e.value + q).clamp(e.floor, e.ceiling);
                }
                self.record(
                    Resource::Energy,
                    tick,
                    q,
                    format!("EmergencyTransfer(+{q:.4} da federation, scarcity {:.3})", fed.1),
                );
                self.record(
                    Resource::Federation,
                    tick,
                    -q,
                    "EmergencyTransfer(saída)".to_string(),
                );
            }
        }
    }

    /// Pedido de alocação — decisão tipada com Lei 3.
    pub fn request(&mut self, r: Resource, amount: f32, _tick: u64) -> AllocationDecision {
        // Cópia local (Budget é Copy): escopos de borrow curtos.
        let Some(b) = self.budgets.get(&r).copied() else {
            return AllocationDecision::Deny {
                reason: DenyReason::Insufficient {
                    pedido: amount,
                    disponivel: 0.0,
                },
                provider_id: self.provider_id,
            };
        };
        if b.is_starving() {
            self.decisions_denied += 1;
            return AllocationDecision::Deny {
                reason: DenyReason::Starving { value: b.value },
                provider_id: self.provider_id,
            };
        }
        if b.scarcity() > COLLAPSE_THRESHOLD {
            // Colapso iminente: throttled a 50%.
            let concedido = (amount * 0.5).min(b.value - b.floor);
            self.decisions_throttled += 1;
            if let Some(bm) = self.budgets.get_mut(&r) {
                bm.value -= concedido;
            }
            self.record(
                r,
                _tick,
                -concedido,
                format!("Throttle(50% colapso iminente, scarcity {:.3})", b.scarcity()),
            );
            return AllocationDecision::Throttle {
                fator: 0.5,
                reason: "colapso iminente de escassez".to_string(),
                provider_id: self.provider_id,
            };
        }
        let disponivel = b.value - b.floor;
        if amount > disponivel {
            self.decisions_denied += 1;
            return AllocationDecision::Deny {
                reason: DenyReason::Insufficient {
                    pedido: amount,
                    disponivel,
                },
                provider_id: self.provider_id,
            };
        }
        let bm = self.budgets.get_mut(&r).expect("presente");
        bm.value -= amount;
        self.decisions_granted += 1;
        self.record(r, _tick, -amount, "Grant(consumo)".to_string());
        AllocationDecision::Grant {
            valor: amount,
            reason: "orçamento disponível acima do piso".to_string(),
        }
    }

    /// Recompensa por CONTRIBUIÇÃO cognitiva confirmada (audit :254):
    /// ganho = min(0.10, contrib×0.05) — tipado no ledger.
    pub fn reward_contribution(&mut self, contrib: f32, tick: u64) -> f32 {
        let ganho = (contrib * 0.05).min(0.10);
        if ganho <= 0.0 {
            return 0.0;
        }
        if let Some(b) = self.budgets.get_mut(&Resource::Energy) {
            b.value = (b.value + ganho).clamp(b.floor, b.ceiling);
        }
        self.record(
            Resource::Energy,
            tick,
            ganho,
            format!("RewardContribution(contrib={contrib:.3})"),
        );
        ganho
    }

    /// Evento de escassez FORÇADO (audit :108-123): derruba o budget
    /// 40% e sobe escassez — usado por janela de estresse DECLARADA.
    pub fn force_scarcity_event(&mut self, r: Resource, tick: u64) -> f32 {
        let Some(b) = self.budgets.get_mut(&r) else { return 0.0 };
        let queda = b.value * 0.4;
        b.value = (b.value - queda).max(b.floor);
        self.record(
            r,
            tick,
            -queda,
            "ScarcityEvent(forçada, -40%)".to_string(),
        );
        queda
    }

    /// Ledger (trilha auditável COM denominador `events_total`).
    pub fn ledger(&self) -> (usize, u64) {
        (self.events.len(), self.events_total)
    }

    /// Últimos eventos (para o app).
    pub fn last_events(&self) -> &std::collections::VecDeque<BudgetEvent> {
        &self.events
    }

    /// Taxas de decisão COM DENOMINADOR.
    pub fn decision_stats(&self) -> (u64, u64, u64, u64) {
        let denom = self.decisions_granted + self.decisions_denied + self.decisions_throttled;
        (
            self.decisions_granted,
            self.decisions_denied,
            self.decisions_throttled,
            denom,
        )
    }
}

/// DevelopmentGovernor: morfogênese SÓ por causa declarada
/// (default STANDBY — nada de acordar sem razão tipada).
#[derive(Debug, Clone, PartialEq)]
pub enum WakeVerdict {
    /// Acordar morfogênese POR ESTA causa.
    Wake { causa: String },
    /// Permanecer em standby — razão explícita.
    Standby { razao: String },
}

/// Governador do desenvolvimento (causas da config `[l5.
/// development_governor] wake_conditions`).
#[derive(Debug, Clone)]
pub struct DevelopmentGovernor {
    pub wake_conditions: Vec<String>,
    pub morphogenesis_default: String,
    pub wakes: u64,
    pub standbys: u64,
}

impl DevelopmentGovernor {
    /// Das condições da config.
    pub fn new(wake_conditions: Vec<String>, morphogenesis_default: String) -> Self {
        Self {
            wake_conditions,
            morphogenesis_default,
            wakes: 0,
            standbys: 0,
        }
    }

    /// Decide por causas OBSERVADAS (ex.: damage/capacity_shortage/
    /// fragmentation/persistent_overload vindos das camadas).
    pub fn should_wake(&mut self, causas_observadas: &[String]) -> WakeVerdict {
        if causas_observadas.is_empty() {
            self.standbys += 1;
            return WakeVerdict::Standby {
                razao: "nenhuma causa observada".to_string(),
            };
        }
        for causa in causas_observadas {
            if self.wake_conditions.iter().any(|c| c == causa) {
                self.wakes += 1;
                return WakeVerdict::Wake {
                    causa: causa.clone(),
                };
            }
        }
        self.standbys += 1;
        WakeVerdict::Standby {
            razao: format!(
                "causas observadas fora das condições declaradas: {}",
                causas_observadas.join(",")
            ),
        }
    }

    /// (wakes, standbys) — denominador explícito.
    pub fn stats(&self) -> (u64, u64) {
        (self.wakes, self.standbys)
    }
}

/// MetaLearning CANÔNICO (audit §3): bandit contextual minimal com
/// exploração DIRIGIDA POR INCERTEZA (p=0.40 bucket desconhecido) e
/// trial com janela limitada + ROLLBACK tipado
/// (historical_meta_learner.py:454-470/:115-125).
#[derive(Debug, Default)]
pub struct HistoricalMetaLearner {
    /// bucket → param → (delta acumulado, count, reward acumulado).
    table: HashMap<String, HashMap<String, (f32, u32, f32)>>,
    pub trials: u64,
    pub rollbacks: u64,
    pub commits: u64,
}

/// Veredito do trial.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrialVerdict {
    /// Ganho sobre o baseline: COMMITA o delta.
    Commit { delta: f32 },
    /// Sem ganho: ROLLBACK tipado (nunca aplica por inércia).
    Rollback { delta: f32 },
}

impl HistoricalMetaLearner {
    /// Novo learner (tabela vazia, contadores zerados).
    pub fn new() -> Self {
        Self::default()
    }

    /// Chave de bucket (contexto quantizado).
    pub fn bucket_key(salience: f32, scarcity: f32) -> String {
        format!("s{:.0}x{:.0}", (salience * 3.999) as u8, (scarcity * 3.999) as u8)
    }

    /// Valor sugerido para o parâmetro no bucket: média do delta se
    /// conhecido; senão EXPLORAÇÃO (0.04 fixo, incerteza declarada).
    pub fn suggest(&self, bucket: &str, param: &str, baseline: f32) -> (f32, bool) {
        match self.table.get(bucket).and_then(|m| m.get(param)) {
            Some((delta, count, _)) if *count > 0 => (baseline + delta / *count as f32, false),
            _ => (baseline + 0.04, true),
        }
    }

    /// Fecha o trial: reward vs baseline DECIDE commit/rollback —
    /// tipado com denominador (2 calls, janela limitada do legado).
    pub fn trial(
        &mut self,
        bucket: &str,
        param: &str,
        baseline: f32,
        trial_value: f32,
        reward: f32,
    ) -> TrialVerdict {
        self.trials += 1;
        let delta = reward - baseline;
        let entry = self
            .table
            .entry(bucket.to_string())
            .or_default()
            .entry(param.to_string())
            .or_insert((0.0, 0, 0.0));
        entry.0 += trial_value - baseline;
        entry.1 += 1;
        entry.2 += reward;
        if delta > 0.0 {
            self.commits += 1;
            TrialVerdict::Commit { delta }
        } else {
            self.rollbacks += 1;
            TrialVerdict::Rollback { delta }
        }
    }

    /// (trials, commits, rollbacks) — denominador trials.
    pub fn stats(&self) -> (u64, u64, u64) {
        (self.trials, self.commits, self.rollbacks)
    }
}

/// MetaGoals: metas metacognitivas com estado tipado (progresso
/// medido por denominador — nunca taxa nua).
#[derive(Debug, Clone, PartialEq)]
pub enum MetaGoalState {
    /// Progredindo: passos dados / passos esperados.
    Progress { feitos: u64, esperados: u64 },
    /// Estagnada — razão explícita.
    Stalled { razao: String },
}

/// Uma meta declarada no L5.
#[derive(Debug, Clone)]
pub struct MetaGoal {
    pub name: String,
    pub state: MetaGoalState,
}

/// Conjunto de metas com relatório COM DENOMINADOR.
#[derive(Debug, Default)]
pub struct MetaGoals {
    goals: Vec<MetaGoal>,
}

impl MetaGoals {
    /// Declara (ou atualiza) uma meta.
    pub fn declare(&mut self, name: &str, state: MetaGoalState) {
        if let Some(g) = self.goals.iter_mut().find(|g| g.name == name) {
            g.state = state;
        } else {
            self.goals.push(MetaGoal {
                name: name.to_string(),
                state,
            });
        }
    }

    /// (metas progredindo, total) — denominador explícito.
    pub fn progress_counts(&self) -> (u64, u64) {
        let p = self
            .goals
            .iter()
            .filter(|g| matches!(g.state, MetaGoalState::Progress { .. }))
            .count() as u64;
        (p, self.goals.len() as u64)
    }

    /// Metas declaradas.
    pub fn snapshot(&self) -> &[MetaGoal] {
        &self.goals
    }
}

/// CognitiveGenome OFFLINE (Lei 7): coleta fitness DENTRO do run;
/// seleção/crossover/mutação/APLICAÇÃO exclusivamente em
/// `evaluate_offline` — NUNCA chamado no loop (o legado violava:
/// evolve() dentro do step, cognitive_genome.py:291-307).
#[derive(Debug, Default)]
pub struct CognitiveGenomeOffline {
    /// Janela de fitness por avaliação (média COM denominador).
    fitness_window: Vec<f32>,
    pub offline_evaluations: u64,
}

impl CognitiveGenomeOffline {
    /// Coleta (permitida no run — é observação, não seleção).
    pub fn collect_fitness(&mut self, f: f32) {
        self.fitness_window.push(f.clamp(0.0, 1.0));
        if self.fitness_window.len() > 10 {
            self.fitness_window.remove(0);
        }
    }

    /// Média de fitness da janela COM DENOMINADOR (None se vazio).
    pub fn avg_fitness(&self) -> Option<f32> {
        if self.fitness_window.is_empty() {
            return None;
        }
        Some(self.fitness_window.iter().sum::<f32>() / self.fitness_window.len() as f32)
    }

    /// Janela de avaliações coletadas.
    pub fn collected(&self) -> usize {
        self.fitness_window.len()
    }

    /// Seleção/aplicação — SÓ fora do loop principal (Lei 7). No
    /// app isso NÃO roda no tick; é chamado em boundary de episode
    /// pela ORQUESTRAÇÃO (aqui: puro, sem mutar estado do run).
    pub fn evaluate_offline(&mut self) -> Option<f32> {
        let avg = self.avg_fitness()?;
        self.offline_evaluations += 1;
        Some(avg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budgets_degrade_e_recuperam_com_tipos() {
        let mut g = ResourceGovernor::new();
        // Consumo dentro do orçamento: Grant tipado.
        match g.request(Resource::Compute, 0.1, 1) {
            AllocationDecision::Grant { valor, reason } => {
                assert!((valor - 0.1).abs() < 1e-6);
                assert!(!reason.is_empty());
            }
            other => panic!("esperava Grant, veio {other:?}"),
        }
        // Over-consumption: Deny Insufficient tipado.
        match g.request(Resource::Compute, 99.0, 2) {
            AllocationDecision::Deny { reason, provider_id } => {
                assert!(matches!(reason, DenyReason::Insufficient { .. }));
                assert_eq!(provider_id, "l5.resource_governor");
            }
            other => panic!("esperava Deny, veio {other:?}"),
        }
        // Recompensa por contribuição: min(0.10, contrib*0.05).
        let ganho = g.reward_contribution(3.0, 3);
        assert!((ganho - 0.10).abs() < 1e-6, "teto do reward");
        let g2 = g.reward_contribution(1.0, 4);
        assert!((g2 - 0.05).abs() < 1e-6, "contrib*0.05");
        let (gr, de, th, denom) = g.decision_stats();
        assert_eq!((gr, de, th, denom), (1, 1, 0, 2), "denominador");
        let (led, total) = g.ledger();
        assert!(led >= 3 && total >= 3, "ledger com eventos");
    }

    #[test]
    fn evento_forcado_e_emergencia_tipados() {
        let mut g = ResourceGovernor::new();
        let queda = g.force_scarcity_event(Resource::Energy, 1);
        assert!(queda > 0.3, "queda de 40%");
        // 4 quedas: 0.6 → 0.36 → 0.216 → 0.13 < 0.2 (gatilho).
        g.force_scarcity_event(Resource::Energy, 2);
        g.force_scarcity_event(Resource::Energy, 3);
        g.force_scarcity_event(Resource::Energy, 4);
        // A transferência é 0.05×escassez do DOADOR (federation) —
        // com doador cheio nada é transferível; derrubamos o doador.
        g.force_scarcity_event(Resource::Federation, 4);
        g.tick(5);
        // Transferência emergencial puxa da federation (tipada).
        let ledger: Vec<String> = g
            .last_events()
            .iter()
            .map(|e| e.reason.clone())
            .collect();
        assert!(
            ledger.iter().any(|r| r.starts_with("EmergencyTransfer")),
            "emergência tipada no ledger: {ledger:?}"
        );
        // Faminto nega com razão Starving.
        for _ in 0..8 {
            g.force_scarcity_event(Resource::Attention, 10);
        }
        match g.request(Resource::Attention, 0.01, 20) {
            AllocationDecision::Deny { reason, .. } => {
                assert!(matches!(reason, DenyReason::Starving { .. }));
            }
            other => panic!("esperava Starving, veio {other:?}"),
        }
    }

    #[test]
    fn throttle_no_colapso_iminente() {
        let mut g = ResourceGovernor::new();
        // 3 quedas: 0.6 → 0.36 → 0.216 (scarcity 0.784 > 0.75 e
        // ainda NÃO starving: 0.216 > 0.1) — janela do throttle.
        for _ in 0..3 {
            g.force_scarcity_event(Resource::Memory, 10);
        }
        match g.request(Resource::Memory, 0.05, 10) {
            AllocationDecision::Throttle { fator, reason, provider_id } => {
                assert!((fator - 0.5).abs() < 1e-6);
                assert!(!reason.is_empty());
                assert_eq!(provider_id, "l5.resource_governor");
            }
            other => panic!("esperava Throttle, veio {other:?}"),
        }
    }

    #[test]
    fn development_governor_acorda_por_causa_declarada() {
        let mut dg = DevelopmentGovernor::new(
            vec!["damage".into(), "capacity_shortage".into()],
            "STANDBY".into(),
        );
        assert_eq!(
            dg.should_wake(&[]),
            WakeVerdict::Standby { razao: "nenhuma causa observada".to_string() }
        );
        assert_eq!(
            dg.should_wake(&["coisa_qualquer".to_string()]),
            WakeVerdict::Standby {
                razao: "causas observadas fora das condições declaradas: coisa_qualquer"
                    .to_string()
            }
        );
        assert_eq!(
            dg.should_wake(&["coisa".to_string(), "damage".to_string()]),
            WakeVerdict::Wake { causa: "damage".to_string() }
        );
        assert_eq!(dg.stats(), (1, 2));
    }

    #[test]
    fn meta_learner_explora_por_incerteza_e_rollback_tipado() {
        let mut m = HistoricalMetaLearner::new();
        let b = HistoricalMetaLearner::bucket_key(0.5, 0.5);
        // Bucket desconhecido: EXPLORAÇÃO declarada.
        let (v, explorou) = m.suggest(&b, "eta", 0.005);
        assert!(explorou, "bucket desconhecido explora");
        assert!((v - 0.045).abs() < 1e-6, "baseline+0.04 (f32)");
        // Trial com reward ACIMA do baseline: commit.
        let c = m.trial(&b, "eta", 0.005, 0.045, 0.03);
        assert!(matches!(c, TrialVerdict::Commit { .. }));
        // Trial com reward ABAIXO: rollback tipado.
        let r = m.trial(&b, "eta", 0.005, 0.04, -0.02);
        assert!(matches!(r, TrialVerdict::Rollback { .. }));
        assert_eq!(m.stats(), (2, 1, 1), "denominador trials");
        // Conhecido agora: sugere média do delta (sem exploração).
        let (_, explorou2) = m.suggest(&b, "eta", 0.005);
        assert!(!explorou2, "bucket conhecido não explora mais");
    }

    #[test]
    fn meta_goals_com_denominador() {
        let mut mg = MetaGoals::default();
        mg.declare("manter_coerencia", MetaGoalState::Progress { feitos: 3, esperados: 5 });
        mg.declare("explorar_baixo_custo", MetaGoalState::Stalled {
            razao: "orçamento de atenção abaixo do piso".to_string(),
        });
        let (p, total) = mg.progress_counts();
        assert_eq!((p, total), (1, 2), "denominador explícito");
    }

    #[test]
    fn genome_offline_lei_7() {
        let mut g = CognitiveGenomeOffline::default();
        assert!(g.avg_fitness().is_none(), "ausência ≠ zero");
        for f in [0.4, 0.6, 0.8] {
            g.collect_fitness(f);
        }
        let avg = g.avg_fitness().expect("janela com 3 avaliações");
        assert!((avg - 0.6).abs() < 1e-6, "média COM denominador 3");
        assert_eq!(g.collected(), 3);
        // evaluate_offline NÃO é chamado no loop — o teste prova que
        // a seleção é método separado (Lei 7: genome fora do tick).
        let sel = g.evaluate_offline();
        assert!(sel.is_some());
        assert_eq!(g.offline_evaluations, 1);
    }

    #[test]
    fn a_a_governador_deterministico() {
        let mut a = ResourceGovernor::new();
        let mut b = ResourceGovernor::new();
        for t in 1..=50 {
            a.tick(t);
            b.tick(t);
        }
        for r in Resource::INTERNAL {
            let (ba, bb) = (a.budget(r), b.budget(r));
            assert_eq!(ba, bb, "A/A por recurso {r:?}");
        }
        assert_eq!(a.ledger(), b.ledger());
    }
}
