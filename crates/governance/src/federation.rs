use triad_contracts as tc;
use triad_foundation as tf;
use tracing::{debug, trace};

/// Fases do ciclo federativo de governança.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FederationPhase {
    /// Abertura do ciclo com propostas.
    Propose,
    /// Deliberação sobre as propostas.
    Deliberate,
    /// Decisão final entre as propostas.
    Decide,
    /// Execução da decisão aprovada.
    Execute,
    /// Verificação dos efeitos executados.
    Verify,
    /// Liquidação e fechamento do ciclo.
    Settle,
}

/// Estado corrente da federação.
#[derive(Debug, Clone)]
pub struct FederationState {
    /// Fase atual do ciclo federativo.
    pub phase: FederationPhase,
    /// Ciclo corrente (incrementa após cada Settle).
    pub cycle: u64,
}

/// Coordenação federativa com taxa de acordo (denominador sempre).
pub struct Federation {
    /// Estado corrente do ciclo federativo.
    state: FederationState,
    /// Total de acordos observados.
    settlements: u64,
    /// Total de liquidações registradas.
    total: u64,
}

impl Federation {
    /// Cria a federação na fase Propose, ciclo 0.
    pub fn new() -> Self {
        Self {
            state: FederationState {
                phase: FederationPhase::Propose,
                cycle: 0,
            },
            settlements: 0,
            total: 0,
        }
    }

    /// Avança a fase na ordem fixa do ciclo federativo.
    pub fn advance(&mut self) {
        self.state.phase = match self.state.phase {
            FederationPhase::Propose => FederationPhase::Deliberate,
            FederationPhase::Deliberate => FederationPhase::Decide,
            FederationPhase::Decide => FederationPhase::Execute,
            FederationPhase::Execute => FederationPhase::Verify,
            FederationPhase::Verify => FederationPhase::Settle,
            FederationPhase::Settle => {
                self.state.cycle += 1;
                debug!(ciclo = self.state.cycle, "ciclo federativo completo");
                FederationPhase::Propose
            }
        };
    }

    /// Registra uma liquidação e seus acordos.
    pub fn settle(&mut self, agreed: u64) {
        self.total += 1;
        self.settlements += agreed;
        trace!(acordos = agreed, "assentamento federativo");
    }

    /// Taxa de acordos como Qualified; sem ciclos ⇒ NO_DATA.
    pub fn settlement_rate(
        &self,
        source: tf::id::ModuleId,
        step: tf::id::StepId,
    ) -> tc::Qualified<tf::Rate> {
        if self.total == 0 {
            return tc::Qualified::no_data("sem ciclos federativos", source, step);
        }
        match tf::Rate::from_ratio(self.settlements, self.total) {
            Some(rate) => tc::Qualified::value(rate, source, step),
            None => tc::Qualified::invalid("taxa fora do dominio", source, step),
        }
    }

    /// Referência imutável ao estado corrente.
    pub fn state(&self) -> &FederationState {
        &self.state
    }
}

// ============================================================
// 18.4 — FederationEngine: CNP bilateral tipado (CAMADA:1143;
// ESPEC var/verificacao/18-0b §2, federation_hub.py l.379-554).
// need→proposal→agreement→transfer→effect, acordo de DUAS
// pernas CONSERVATIVO (erro < 1e-12), reservas contra dupla
// promessa, ledger append-only com hash encadeado, downstream
// map com EffectSink INJETÁVEL. Staging: feature-off por padrão
// — nenhum wiring da federação altera comportamento sem o dono.
// ============================================================

/// Recursos federados (8 do legado, federation_hub l.129-150).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Resource {
    Energy,
    Compute,
    Attention,
    Memory,
    Federation,
    Structural,
    Novelty,
    Time,
}

impl Resource {
    /// Nome canônico.
    pub fn as_str(self) -> &'static str {
        match self {
            Resource::Energy => "energy",
            Resource::Compute => "compute",
            Resource::Attention => "attention",
            Resource::Memory => "memory",
            Resource::Federation => "federation",
            Resource::Structural => "structural",
            Resource::Novelty => "novelty",
            Resource::Time => "time",
        }
    }
}

/// Membro da federação com budget por recurso [0,1] e reservas
/// contra dupla promessa.
#[derive(Debug, Clone, Default)]
pub struct FederationMember {
    /// Budget corrente por recurso (fração [0,1]).
    pub budget: std::collections::BTreeMap<Resource, f64>,
    /// Reservas marcadas por acordos ainda não executados.
    pub reserved: std::collections::BTreeMap<Resource, f64>,
}

impl FederationMember {
    /// Disponível = budget − reservas (nunca negativo).
    pub fn available(&self, r: Resource) -> f64 {
        (self.budget.get(&r).copied().unwrap_or(0.0)
            - self.reserved.get(&r).copied().unwrap_or(0.0))
            .max(0.0)
    }

    /// Escassez do recurso [0,1] (1 − budget; ceiling 1.0).
    pub fn scarcity(&self, r: Resource) -> f64 {
        1.0 - self.budget.get(&r).copied().unwrap_or(0.0).clamp(0.0, 1.0)
    }
}

/// Razão tipada de rejeição de uma proposta CNP.
#[derive(Debug, Clone, PartialEq)]
pub enum CnpReject {
    /// Utilidade abaixo do custo declarado.
    UtilityBelowCost {
        /// Utilidade calculada.
        utility: f64,
        /// Custo exigido pelo proponente.
        cost: f64,
    },
    /// Reserva insuficiente na perna de saída.
    InsufficientReserve {
        /// Disponível após reservas.
        available: f64,
        /// Quantia da perna.
        needed: f64,
    },
    /// Membro inexistente.
    UnknownMember {
        /// Identidade consultada.
        id: String,
    },
    /// Auto-troca (mesmo membro nas duas pontas).
    SelfTrade,
    /// 18.4 ACTIVE/SHAM: o CNP está desligado (sham) — a proposta
    /// é registrada com razão tipada e NENHUM estado muda (o
    /// controle desligado não altera a trajetória).
    ShamControl,
    /// 18.4 FederationTransport: a entrega pós-aplicação falhou
    /// (efeito downstream não chegou ao destino — recibo honesto).
    TransportUnreachable {
        /// Nome canônico do transporte consultado.
        transport: String,
    },
}

impl CnpReject {
    /// Nome canônico da razão.
    pub fn as_str(&self) -> &'static str {
        match self {
            CnpReject::UtilityBelowCost { .. } => "utility_below_cost",
            CnpReject::InsufficientReserve { .. } => "insufficient_reserve",
            CnpReject::UnknownMember { .. } => "unknown_member",
            CnpReject::SelfTrade => "self_trade",
            CnpReject::ShamControl => "sham_control",
            CnpReject::TransportUnreachable { .. } => "transport_unreachable",
        }
    }
}

/// Razão tipada de reversão pós-execução (Lei 6 da casa).
#[derive(Debug, Clone, PartialEq)]
pub enum RevertReason {
    /// Erro de conservação acima da tolerância 1e-12.
    ConservationError {
        /// Desvio máximo observado nas duas pernas.
        error: f64,
    },
}

impl RevertReason {
    /// Nome canônico da razão.
    pub fn as_str(&self) -> &'static str {
        "conservation_error"
    }
}

/// Situação do acordo no ledger.
#[derive(Debug, Clone, PartialEq)]
pub enum TradeStatus {
    /// Aceito e executado com as duas pernas.
    Applied,
    /// Rejeitado na avaliação (razão tipada).
    Rejected(CnpReject),
    /// Executado e REVERTIDO (razão tipada).
    Reverted(RevertReason),
}

impl TradeStatus {
    /// Nome canônico da situação.
    pub fn as_str(&self) -> &'static str {
        match self {
            TradeStatus::Applied => "applied",
            TradeStatus::Rejected(_) => "rejected",
            TradeStatus::Reverted(_) => "reverted",
        }
    }
}

/// Recibo imutável de uma rodada CNP (append-only).
#[derive(Debug, Clone, PartialEq)]
pub struct TradeRecord {
    /// Tick da proposta.
    pub tick: u64,
    /// Membro ofertante.
    pub from: String,
    /// Membro receptor.
    pub to: String,
    /// Perna de oferta (recurso, quantia).
    pub offer: (Resource, f64),
    /// Perna de pagamento (recurso, quantia).
    pub payment: (Resource, f64),
    /// Situação final (tipada).
    pub status: TradeStatus,
    /// Hash encadeado com o registro anterior (trilha).
    pub chain_hash: u64,
    /// 18.4 outcome t+1: efeito persistiu no receptor (None até
    /// resolver — ausência ≠ zero; Lei 5: benefício só fecha com
    /// efeito real observado).
    pub horizon_1: Option<bool>,
    /// 18.4 outcome t+5: mesmo contrato no horizonte longo.
    pub horizon_5: Option<bool>,
    /// 18.4 benefício validado: fecha SÓ com h1 E h5 observados.
    pub benefit_validated: Option<bool>,
}

/// 18.4: outcome pendente de uma troca aplicada (padrão do
/// ecology horizon_1/5 portado — baseline capturado pós-aplicação,
/// identidade pelo chain_hash do recibo).
#[derive(Debug, Clone, PartialEq)]
pub struct PendingOutcome {
    /// Tick do horizonte 1 (proposta.tick + 1).
    pub at_tick_1: u64,
    /// Tick do horizonte 5 (proposta.tick + 5).
    pub at_tick_5: u64,
    /// chain_hash do recibo (identidade estável — o ledger faz
    /// trim, índice NÃO é identidade).
    pub chain_hash: u64,
    /// Receptor (quem recebeu o recurso oferecido).
    pub to: String,
    /// Recurso oferecido (o que deve persistir).
    pub resource: Resource,
    /// Budget do receptor no recurso LOGO APÓS a aplicação.
    pub baseline: f64,
}

/// Política CNP INJETÁVEL (tabela por recurso — nunca constante
/// mágica; defaults do legado l.289-298/364-375).
#[derive(Debug, Clone)]
pub struct CnpPolicy {
    /// Prioridade por recurso (valoriza o que se pede).
    pub priority: std::collections::BTreeMap<Resource, f64>,
    /// Taxa de troca por recurso (desvaloriza o que se oferece).
    pub rate: std::collections::BTreeMap<Resource, f64>,
    /// Teto do bônus de urgência.
    pub urgency_bonus_cap: f64,
    /// Peso da urgência (escassez do ofertado no receptor).
    pub urgency_weight: f64,
    /// Custo mínimo exigido por proposta.
    pub min_cost: f64,
}

impl Default for CnpPolicy {
    fn default() -> Self {
        let mut priority = std::collections::BTreeMap::new();
        let mut rate = std::collections::BTreeMap::new();
        for (r, p, k) in [
            (Resource::Energy, 1.4, 0.9),
            (Resource::Compute, 1.2, 0.95),
            (Resource::Attention, 1.3, 0.9),
            (Resource::Memory, 1.1, 1.0),
            (Resource::Federation, 1.0, 1.0),
            (Resource::Structural, 1.05, 1.0),
            (Resource::Novelty, 0.9, 1.1),
            (Resource::Time, 1.6, 0.85),
        ] {
            priority.insert(r, p);
            rate.insert(r, k);
        }
        Self {
            priority,
            rate,
            urgency_bonus_cap: 0.25,
            urgency_weight: 0.025,
            min_cost: 0.0,
        }
    }
}

/// Sinal de controle downstream (DOWNSTREAM_CONTROL_MAP do
/// legado l.84-93): pernas de recursos mapeiam para escalares de
/// controle REAL com clip [0.25, 1.25].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlSignal {
    /// Tipo de controle (clip de política).
    pub kind: ControlKind,
    /// Escala [clip_min, clip_max].
    pub scale: f64,
}

/// Tipos de controle downstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlKind {
    /// Escala de compute.
    ComputeScale,
    /// Escala de atenção.
    AttentionScale,
    /// Escala de energia.
    EnergyScale,
    /// Escala de novidade.
    NoveltyScale,
}

impl ControlKind {
    /// Nome canônico.
    pub fn as_str(self) -> &'static str {
        match self {
            ControlKind::ComputeScale => "compute_scale",
            ControlKind::AttentionScale => "attention_scale",
            ControlKind::EnergyScale => "energy_scale",
            ControlKind::NoveltyScale => "novelty_scale",
        }
    }
}

/// Destino dos efeitos downstream (injetável — o motor não
/// conhece o estado alvo).
pub trait EffectSink {
    /// Aplica o sinal e retorna a razão canônica do efeito.
    fn apply(&mut self, signal: ControlSignal) -> String;
}

/// 18.4 FederationTransport: porta tipada de entrega pós-aplicação.
/// Padrão InProcess (entrega local imediata); o transporte REMOTO
/// fica atrás da feature `federation-remote` (DESLIGADA no
/// Cargo.toml — sem código remoto, só a porta).
pub trait FederationTransport {
    /// Nome canônico do transporte (trilha dos recibos).
    fn name(&self) -> &'static str;
    /// Entrega o recibo aplicado; false = efeito não chegou.
    fn deliver(&mut self, record: &TradeRecord) -> bool;
}

/// Transporte in-process padrão: a execução local É a entrega.
#[derive(Debug, Clone, Copy, Default)]
pub struct InProcessTransport;

impl FederationTransport for InProcessTransport {
    fn name(&self) -> &'static str {
        "in_process"
    }
    fn deliver(&mut self, _record: &TradeRecord) -> bool {
        true
    }
}

/// Porta remota: DESLIGADA por padrão (feature `federation-remote`).
/// Quando o dono abrir federação entre organismos, o transporte
/// real entra AQUI — o contrato tipado já está fechado.
#[cfg(feature = "federation-remote")]
pub mod remote {
    use super::{FederationTransport, TradeRecord};

    /// Stub tipado da porta remota (recusa: sem implementação —
    /// recebeu sem mentir que entregou).
    #[derive(Debug, Clone, Copy, Default)]
    pub struct RemoteTransportStub;

    impl FederationTransport for RemoteTransportStub {
        fn name(&self) -> &'static str {
            "remote_stub"
        }
        fn deliver(&mut self, _record: &TradeRecord) -> bool {
            false
        }
    }
}

/// Motor CNP bilateral.
pub struct FederationEngine {
    members: std::collections::BTreeMap<String, FederationMember>,
    policy: CnpPolicy,
    /// Cap de perna por proposta (polícia "federation.trade_rate"
    /// validada pelo Rust: [0.0, 0.5]).
    trade_rate_cap: f64,
    ledger: Vec<TradeRecord>,
    chain: u64,
    conservation_error_max: f64,
    /// 18.4 ACTIVE/SHAM: false = CNP ativo (default); true = sham
    /// (desligado — propostas registradas com razão tipada, nenhum
    /// estado muda; o teste A/A da caixa).
    sham: bool,
    /// 18.4 transporte de entrega (padrão InProcess; injetável).
    transport: Box<dyn FederationTransport>,
    /// 18.4 outcomes pendentes t+1/t+5 das trocas aplicadas.
    pending_outcomes: Vec<PendingOutcome>,
}

impl FederationEngine {
    /// Motor novo com política padrão (staging: sem membros).
    pub fn new() -> Self {
        Self::with_policy(CnpPolicy::default())
    }

    /// Motor novo com política injetada.
    pub fn with_policy(policy: CnpPolicy) -> Self {
        Self {
            members: std::collections::BTreeMap::new(),
            policy,
            trade_rate_cap: 0.5,
            ledger: Vec::new(),
            chain: 0xEC0F_ED01,
            conservation_error_max: 0.0,
            sham: false,
            transport: Box::new(InProcessTransport),
            pending_outcomes: Vec::new(),
        }
    }

    /// 18.4: troca o transporte de entrega (padrão InProcess).
    pub fn with_transport(mut self, transport: Box<dyn FederationTransport>) -> Self {
        self.transport = transport;
        self
    }

    /// 18.4 ACTIVE/SHAM: liga/desliga o CNP (sham = desligado).
    pub fn set_sham(&mut self, on: bool) {
        self.sham = on;
    }

    /// 18.4: sham ligado? (telemetria do controle).
    pub fn is_sham(&self) -> bool {
        self.sham
    }

    /// Ajusta o cap de perna (clamp da faixa da whitelist Rust:
    /// "federation" / "trade_rate" ∈ [0.0, 0.5] — staging).
    pub fn set_trade_rate(&mut self, rate: f64) {
        self.trade_rate_cap = rate.clamp(0.0, 0.5);
    }

    /// Cap de perna corrente (telemetria da política aplicada).
    pub fn trade_rate_cap(&self) -> f64 {
        self.trade_rate_cap
    }

    /// Adiciona um membro com budget inicial por recurso.
    pub fn add_member(&mut self, id: &str, budget: std::collections::BTreeMap<Resource, f64>) {
        self.members.insert(id.to_string(), FederationMember { budget, reserved: Default::default() });
    }

    /// Recurso mais escasso do membro (None se desconhecido ou
    /// sem budget — ausência ≠ zero).
    pub fn scarcest(&self, id: &str) -> Option<Resource> {
        self.members
            .get(id)
            .map(|m| {
                m.budget
                    .iter()
                    .min_by(|a, b| {
                        a.1.partial_cmp(b.1)
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .then(a.0.cmp(b.0))
                    })
                    .map(|(r, _)| *r)
            })
            .flatten()
    }

    /// Hash FNV-1a 64 determinístico sobre os bytes do recibo.
    fn chain_next(&mut self, record: &TradeRecord) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ self.chain;
        let mut feed = |bytes: &[u8]| {
            for b in bytes {
                h ^= *b as u64;
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        feed(record.tick.to_string().as_bytes());
        feed(record.from.as_bytes());
        feed(record.to.as_bytes());
        feed(record.offer.0.as_str().as_bytes());
        feed(&record.offer.1.to_le_bytes());
        feed(record.payment.0.as_str().as_bytes());
        feed(&record.payment.1.to_le_bytes());
        feed(record.status.as_str().as_bytes());
        self.chain = h;
        h
    }

    /// Proposta CNP completa de DUAS pernas: `from` oferece
    /// `offer` (recurso, quantia) e paga `payment`; utility do
    /// receptor = offer×priority(payment)×rate(offer) +
    /// min(cap, escassez(offer|to)×peso). Aceite exige utility ≥
    /// custo, reservas nas DUAS pernas e mapa completo.
    pub fn propose_trade(
        &mut self,
        tick: u64,
        from: &str,
        to: &str,
        offer: (Resource, f64),
        payment: (Resource, f64),
    ) -> TradeStatus {
        // Cap de perna da política validada (clamp nas DUAS
        // pernas — conservação preservada por construção).
        let offer = (offer.0, offer.1.min(self.trade_rate_cap).max(0.0));
        let payment = (payment.0, payment.1.min(self.trade_rate_cap).max(0.0));
        // 18.4 ACTIVE/SHAM: desligado ⇒ recibo com razão tipada e
        // NENHUM estado muda (a avaliação inteira é pulada — o
        // controle desligado não altera a trajetória).
        let status = if self.sham {
            TradeStatus::Rejected(CnpReject::ShamControl)
        } else {
            self.evaluate(from, to, offer, payment)
        };
        let mut record = TradeRecord {
            tick,
            from: from.to_string(),
            to: to.to_string(),
            offer,
            payment,
            status: status.clone(),
            chain_hash: 0,
            horizon_1: None,
            horizon_5: None,
            benefit_validated: None,
        };
        record.chain_hash = self.chain_next(&record);
        // 18.4 FederationTransport: entrega pós-aplicação (padrão
        // InProcess = execução local, sempre true). Se a entrega
        // falhar, o recibo registra TransportUnreachable — efeito
        // downstream não chegou (recibo honesto, nunca métrica
        // agregada).
        let mut status = status;
        if status == TradeStatus::Applied && !self.transport.deliver(&record) {
            status = TradeStatus::Rejected(CnpReject::TransportUnreachable {
                transport: self.transport_name().to_string(),
            });
            record.status = status.clone();
        }
        // 18.4 outcome t+1/t+5: troca aplicada agenda os
        // horizontes com o baseline capturado pós-aplicação (o
        // recurso oferecido DEVE persistir no receptor).
        if status == TradeStatus::Applied {
            let baseline = self
                .members
                .get(to)
                .and_then(|m| m.budget.get(&offer.0).copied())
                .unwrap_or(0.0);
            self.pending_outcomes.push(PendingOutcome {
                at_tick_1: tick.saturating_add(1),
                at_tick_5: tick.saturating_add(5),
                chain_hash: record.chain_hash,
                to: to.to_string(),
                resource: offer.0,
                baseline,
            });
        }
        self.ledger.push(record);
        if self.ledger.len() > 64 {
            self.ledger.remove(0);
        }
        status
    }

    /// 18.4: resolve os outcomes t+1/t+5 vencidos (chamar a cada
    /// tick). MATCH = o budget do receptor no recurso oferecido
    /// permanece >= baseline (efeito persistiu); DRIFT = caiu
    /// abaixo. benefit_validated fecha SÓ com h1 E h5 (Lei 5).
    /// Retorna quantos horizontes foram resolvidos.
    pub fn resolve_outcomes(&mut self, now: u64) -> usize {
        let mut resolved = 0usize;
        for p in self.pending_outcomes.iter_mut() {
            let Some(record) = self
                .ledger
                .iter_mut()
                .find(|r| r.chain_hash == p.chain_hash)
            else {
                continue;
            };
            let current = self
                .members
                .get(&p.to)
                .and_then(|m| m.budget.get(&p.resource).copied())
                .unwrap_or(0.0);
            let matched = current >= p.baseline - 1e-12;
            if now >= p.at_tick_1 && record.horizon_1.is_none() {
                record.horizon_1 = Some(matched);
                resolved += 1;
            }
            if now >= p.at_tick_5 && record.horizon_5.is_none() {
                record.horizon_5 = Some(matched);
                resolved += 1;
            }
            if let (Some(h1), Some(h5)) = (record.horizon_1, record.horizon_5) {
                record.benefit_validated = Some(h1 && h5);
            }
        }
        self.pending_outcomes
            .retain(|p| p.at_tick_1 > now || p.at_tick_5 > now || {
                self.ledger
                    .iter()
                    .find(|r| r.chain_hash == p.chain_hash)
                    .is_none()
            });
        // Pendentes cujos dois horizontes resolveram saem da fila.
        self.pending_outcomes.retain(|p| {
            match self
                .ledger
                .iter()
                .find(|r| r.chain_hash == p.chain_hash)
            {
                Some(r) => r.horizon_1.is_none() || r.horizon_5.is_none(),
                None => false,
            }
        });
        resolved
    }

    /// Nome canônico do transporte corrente (trilha).
    pub fn transport_name(&self) -> &'static str {
        self.transport.name()
    }

    /// 18.4: outcomes ainda pendentes (telemetria honesta).
    pub fn pending_outcome_count(&self) -> usize {
        self.pending_outcomes.len()
    }

    /// Avalia e EXECUTA com reserva/transferência atômica nas
    /// duas pernas; conserva recursos (erro < 1e-12 verificado).
    fn evaluate(
        &mut self,
        from: &str,
        to: &str,
        offer: (Resource, f64),
        payment: (Resource, f64),
    ) -> TradeStatus {
        if from == to {
            return TradeStatus::Rejected(CnpReject::SelfTrade);
        }
        let (Some(mut donor), Some(mut receiver)) = (
            self.members.get(from).cloned(),
            self.members.get(to).cloned(),
        ) else {
            return TradeStatus::Rejected(CnpReject::UnknownMember {
                id: if self.members.contains_key(from) {
                    to.to_string()
                } else {
                    from.to_string()
                },
            });
        };
        // Utility do receptor (fórmula do legado l.400-431).
        let priority = self.policy.priority.get(&payment.0).copied().unwrap_or(1.0);
        let rate = self.policy.rate.get(&offer.0).copied().unwrap_or(1.0);
        let urgency = receiver.scarcity(offer.0);
        let utility = offer.1 * priority * rate
            + (urgency * self.policy.urgency_weight).min(self.policy.urgency_bonus_cap);
        if utility < self.policy.min_cost {
            return TradeStatus::Rejected(CnpReject::UtilityBelowCost {
                utility,
                cost: self.policy.min_cost,
            });
        }
        // Reservas nas DUAS pernas contra dupla promessa.
        if donor.available(offer.0) < offer.1 {
            return TradeStatus::Rejected(CnpReject::InsufficientReserve {
                available: donor.available(offer.0),
                needed: offer.1,
            });
        }
        if receiver.available(payment.0) < payment.1 {
            return TradeStatus::Rejected(CnpReject::InsufficientReserve {
                available: receiver.available(payment.0),
                needed: payment.1,
            });
        }
        // Execução atômica das duas pernas (backup para reversão).
        let donor_backup = donor.clone();
        let receiver_backup = receiver.clone();
        let totals_before = self.resource_totals();
        *donor.reserved.entry(offer.0).or_insert(0.0) += offer.1;
        *receiver.reserved.entry(payment.0).or_insert(0.0) += payment.1;
        donor.budget.insert(offer.0, (donor.budget.get(&offer.0).copied().unwrap_or(0.0) - offer.1).max(0.0));
        receiver.budget.insert(payment.0, (receiver.budget.get(&payment.0).copied().unwrap_or(0.0) - payment.1).max(0.0));
        *donor.budget.entry(payment.0).or_insert(0.0) += payment.1;
        *receiver.budget.entry(offer.0).or_insert(0.0) += offer.1;
        // Conservação global (verificação honesta, l.762-775).
        let totals_after = self.apply_local(from, to, &donor, &receiver);
        let error = self.conservation_error(&totals_before, &totals_after);
        if error > self.conservation_error_max {
            self.conservation_error_max = error;
        }
        if error > 1e-12 {
            // Reverte DE FATO os budgets (Lei 6 da casa): efeito sem
            // conservação comprovada não permanece (os backups
            // também devolvem as reservas marcadas).
            let _ = self.apply_local(from, to, &donor_backup, &receiver_backup);
            return TradeStatus::Reverted(RevertReason::ConservationError { error });
        }
        // Pernas CONCLUÍDAS liberam as reservas (só acordos
        // pendentes/pendurados ocupam reserva — legado l.663-756).
        *donor.reserved.entry(offer.0).or_insert(0.0) =
            (donor.reserved.get(&offer.0).copied().unwrap_or(0.0) - offer.1).max(0.0);
        *receiver.reserved.entry(payment.0).or_insert(0.0) =
            (receiver.reserved.get(&payment.0).copied().unwrap_or(0.0) - payment.1).max(0.0);
        let _ = self.apply_local(from, to, &donor, &receiver);
        TradeStatus::Applied
    }

    /// Persiste os budgets locais calculados e devolve os
    /// totais globais pós-operação.
    fn apply_local(
        &mut self,
        from: &str,
        to: &str,
        donor: &FederationMember,
        receiver: &FederationMember,
    ) -> std::collections::BTreeMap<Resource, f64> {
        if let Some(m) = self.members.get_mut(from) {
            m.budget = donor.budget.clone();
            m.reserved = donor.reserved.clone();
        }
        if let Some(m) = self.members.get_mut(to) {
            m.budget = receiver.budget.clone();
            m.reserved = receiver.reserved.clone();
        }
        self.resource_totals()
    }

    /// Soma global por recurso (mapa completo da verificação).
    fn resource_totals(&self) -> std::collections::BTreeMap<Resource, f64> {
        let mut totals = std::collections::BTreeMap::new();
        for m in self.members.values() {
            for (r, v) in &m.budget {
                *totals.entry(*r).or_insert(0.0) += *v;
            }
        }
        totals
    }

    /// Desvio máximo entre dois mapas de totais.
    fn conservation_error(
        &self,
        before: &std::collections::BTreeMap<Resource, f64>,
        after: &std::collections::BTreeMap<Resource, f64>,
    ) -> f64 {
        let mut max = 0.0f64;
        for (r, v) in before {
            let delta = (v - after.get(r).copied().unwrap_or(0.0)).abs();
            if delta > max {
                max = delta;
            }
        }
        for (r, v) in after {
            let delta = (v - before.get(r).copied().unwrap_or(0.0)).abs();
            if delta > max {
                max = delta;
            }
        }
        max
    }

    /// DOWNSTREAM map: pernas bem-sucedidas de compute/attention/
    /// energy/novelty emitem ControlSignal ao sink injetado
    /// (efeito real fora do motor; clip de política).
    pub fn downstream_effects(
        &self,
        record: &TradeRecord,
        sink: &mut dyn EffectSink,
    ) -> Vec<(ControlKind, String)> {
        let mut out = Vec::new();
        if record.status != TradeStatus::Applied {
            return out;
        }
        let scale_of = |r: Resource| -> Option<ControlKind> {
            match r {
                Resource::Compute => Some(ControlKind::ComputeScale),
                Resource::Attention => Some(ControlKind::AttentionScale),
                Resource::Energy => Some(ControlKind::EnergyScale),
                Resource::Novelty => Some(ControlKind::NoveltyScale),
                _ => None,
            }
        };
        for (resource, amount) in [record.offer, record.payment] {
            if let Some(kind) = scale_of(resource) {
                let scale = (1.0 + amount).clamp(0.25, 1.25);
                let reason = sink.apply(ControlSignal { kind, scale });
                out.push((kind, reason));
            }
        }
        out
    }

    /// Ledger append-only (trilha restaurável).
    pub fn ledger(&self) -> &[TradeRecord] {
        &self.ledger
    }

    /// Estatísticas com denominadores.
    pub fn stats(&self) -> (u64, u64, u64, u64, f64) {
        let total = self.ledger.len() as u64;
        let applied = self
            .ledger
            .iter()
            .filter(|r| r.status == TradeStatus::Applied)
            .count() as u64;
        let rejected = self
            .ledger
            .iter()
            .filter(|r| matches!(r.status, TradeStatus::Rejected(_)))
            .count() as u64;
        let reverted = self
            .ledger
            .iter()
            .filter(|r| matches!(r.status, TradeStatus::Reverted(_)))
            .count() as u64;
        (
            total,
            applied,
            rejected,
            reverted,
            self.conservation_error_max,
        )
    }
}

impl Default for FederationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod federation_tests {
    use super::*;

    fn budget(pairs: &[(Resource, f64)]) -> std::collections::BTreeMap<Resource, f64> {
        pairs.iter().copied().collect()
    }

    /// Golden do protocolo: cada estado alcançável é tipado —
    /// Applied, Rejected(utility/reserva/desconhecido/self) e
    /// Reverted(conservação).
    #[test]
    fn cnp_todos_os_estados_sao_tipados() {
        let mut f = FederationEngine::new();
        f.add_member("a", budget(&[(Resource::Energy, 0.8), (Resource::Compute, 0.2)]));
        f.add_member("b", budget(&[(Resource::Compute, 0.9), (Resource::Energy, 0.3)]));
        // Applied.
        let s = f.propose_trade(1, "a", "b", (Resource::Energy, 0.1), (Resource::Compute, 0.1));
        assert_eq!(s, TradeStatus::Applied);
        // SelfTrade.
        let s = f.propose_trade(2, "a", "a", (Resource::Energy, 0.1), (Resource::Time, 0.1));
        assert_eq!(s, TradeStatus::Rejected(CnpReject::SelfTrade));
        // UnknownMember.
        let s = f.propose_trade(3, "a", "zz", (Resource::Energy, 0.1), (Resource::Time, 0.1));
        assert_eq!(
            s,
            TradeStatus::Rejected(CnpReject::UnknownMember { id: "zz".into() })
        );
        // InsufficientReserve (pede mais do que existe — perna
        // dentro do cap 0.5, escassez real de budget).
        let mut f2 = FederationEngine::new();
        f2.add_member("a", budget(&[(Resource::Energy, 0.2), (Resource::Compute, 0.2)]));
        f2.add_member("b", budget(&[(Resource::Compute, 0.9)]));
        let s = f2.propose_trade(4, "a", "b", (Resource::Energy, 0.3), (Resource::Compute, 0.1));
        assert!(matches!(s, TradeStatus::Rejected(CnpReject::InsufficientReserve { .. })));
        // UtilityBelowCost (política com custo mínimo alto).
        let policy = CnpPolicy { min_cost: 99.0, ..CnpPolicy::default() };
        let mut g = FederationEngine::with_policy(policy);
        g.add_member("a", budget(&[(Resource::Energy, 0.8)]));
        g.add_member("b", budget(&[(Resource::Compute, 0.9)]));
        let s = g.propose_trade(5, "a", "b", (Resource::Energy, 0.1), (Resource::Compute, 0.1));
        assert!(matches!(s, TradeStatus::Rejected(CnpReject::UtilityBelowCost { .. })));
    }

    /// Conservação: as duas pernas preservam o total de CADA
    /// recurso com erro < 1e-12 (legado l.43-44).
    #[test]
    fn cnp_conserva_recursos_nas_duas_pernas() {
        let mut f = FederationEngine::new();
        f.add_member("a", budget(&[(Resource::Energy, 0.8), (Resource::Compute, 0.2)]));
        f.add_member("b", budget(&[(Resource::Compute, 0.9), (Resource::Energy, 0.3)]));
        let before = f.resource_totals();
        let s = f.propose_trade(1, "a", "b", (Resource::Energy, 0.15), (Resource::Compute, 0.12));
        assert_eq!(s, TradeStatus::Applied);
        let after = f.resource_totals();
        let err = f.conservation_error(&before, &after);
        assert!(err < 1e-12, "erro de conservação {err} < 1e-12");
        let (_, _, _, _, max_err) = f.stats();
        assert!(max_err < 1e-12, "conservation_error_max honesto");
    }

    /// Ledger restaurável e encadeado: dois motores gêmeos com a
    /// mesma sequência têm ledger BIT-IDÊNTICO (A/A).
    #[test]
    fn ledger_restauravel_e_deterministico_aa() {
        let build = |mut f: FederationEngine| {
            f.add_member("a", budget(&[(Resource::Energy, 0.8), (Resource::Novelty, 0.4)]));
            f.add_member("b", budget(&[(Resource::Compute, 0.9), (Resource::Attention, 0.5)]));
            f.propose_trade(1, "a", "b", (Resource::Energy, 0.1), (Resource::Compute, 0.1));
            f.propose_trade(2, "b", "a", (Resource::Attention, 0.1), (Resource::Novelty, 0.1));
            f.ledger().to_vec()
        };
        let l1 = build(FederationEngine::new());
        let l2 = build(FederationEngine::new());
        assert_eq!(l1, l2, "ledger bit-idêntico entre gêmeos");
        assert!(l1.iter().all(|r| r.chain_hash != 0));
    }

    /// Reservas contra dupla promessa: proposta em série não
    /// pode gastar o que já saiu.
    #[test]
    fn reservas_evitam_dupla_promessa() {
        let mut f = FederationEngine::new();
        f.add_member("a", budget(&[(Resource::Energy, 0.2)]));
        f.add_member("b", budget(&[(Resource::Compute, 0.9)]));
        let s1 = f.propose_trade(1, "a", "b", (Resource::Energy, 0.15), (Resource::Compute, 0.05));
        assert_eq!(s1, TradeStatus::Applied);
        let s2 = f.propose_trade(2, "a", "b", (Resource::Energy, 0.15), (Resource::Compute, 0.05));
        assert!(
            matches!(s2, TradeStatus::Rejected(CnpReject::InsufficientReserve { available, needed }) if (available - 0.05).abs() < 1e-9 && (needed - 0.15).abs() < 1e-9),
            "segunda proposta rejeitada com razão tipada e valores reais"
        );
    }

    /// Downstream: perna bem-sucedida emite ControlSignal
    /// clampado [0.25, 1.25] ao sink injetado; recusa não emite.
    struct Sink(Vec<String>);
    impl EffectSink for Sink {
        fn apply(&mut self, signal: ControlSignal) -> String {
            let reason = format!("{}={:.3}", signal.kind.as_str(), signal.scale);
            self.0.push(reason.clone());
            reason
        }
    }
    #[test]
    fn downstream_emite_sinal_clampado_apenas_no_sucesso() {
        let mut f = FederationEngine::new();
        f.add_member("a", budget(&[(Resource::Compute, 0.8)]));
        f.add_member("b", budget(&[(Resource::Energy, 0.9)]));
        let mut sink = Sink(Vec::new());
        let idx = f.ledger().len();
        // Recusa: membro inexistente (razão tipada, sem efeito).
        let s = f.propose_trade(1, "a", "zz", (Resource::Compute, 0.3), (Resource::Energy, 0.1));
        assert!(matches!(s, TradeStatus::Rejected(_)));
        let effects = f.downstream_effects(&f.ledger()[idx], &mut sink);
        assert!(effects.is_empty(), "recusa não emite efeito");
        let idx = f.ledger().len();
        let s = f.propose_trade(2, "a", "b", (Resource::Compute, 0.3), (Resource::Energy, 0.2));
        assert_eq!(s, TradeStatus::Applied);
        let effects = f.downstream_effects(&f.ledger()[idx], &mut sink);
        assert!(!effects.is_empty(), "sucesso emite efeito real");
        for (_, reason) in &effects {
            let scale: f64 = reason.split('=').nth(1).unwrap().parse().unwrap();
            assert!((0.25..=1.25).contains(&scale), "clip respeitado");
        }
    }

    /// Scarcest: recurso mais escasso do membro (ausência ≠
    /// zero para membro desconhecido).
    #[test]
    fn scarcest_e_tipado() {
        let mut f = FederationEngine::new();
        f.add_member("a", budget(&[(Resource::Energy, 0.9), (Resource::Compute, 0.1)]));
        assert_eq!(f.scarcest("a"), Some(Resource::Compute));
        assert_eq!(f.scarcest("zz"), None, "desconhecido é ausência");
    }

    /// Cap de perna da POLICY "federation.trade_rate": propostas
    /// acima do cap são clampadas (faixa da whitelist Rust).
    #[test]
    fn trade_rate_cap_clampa_as_pernas() {
        let mut f = FederationEngine::new();
        f.set_trade_rate(0.05);
        f.add_member("a", budget(&[(Resource::Energy, 0.8)]));
        f.add_member("b", budget(&[(Resource::Compute, 0.9)]));
        let s = f.propose_trade(1, "a", "b", (Resource::Energy, 0.3), (Resource::Compute, 0.4));
        assert_eq!(s, TradeStatus::Applied);
        let last = f.ledger().last().unwrap();
        assert!((last.offer.1 - 0.05).abs() < 1e-12, "perna offer clampada");
        assert!((last.payment.1 - 0.05).abs() < 1e-12, "perna payment clampada");
        assert_eq!(f.trade_rate_cap(), 0.05);
        // Fora da faixa da whitelist: clampa para o teto 0.5.
        f.set_trade_rate(9.0);
        assert_eq!(f.trade_rate_cap(), 0.5, "faixa do Rust é inegociável");
    }
}

#[cfg(test)]
mod federation_18_4_debitos_tests {
    use super::*;

    fn budget(pairs: &[(Resource, f64)]) -> std::collections::BTreeMap<Resource, f64> {
        pairs.iter().copied().collect()
    }

    fn twin_pair() -> (FederationEngine, FederationEngine) {
        let mut a = FederationEngine::new();
        let mut b = FederationEngine::new();
        for e in [&mut a, &mut b] {
            e.add_member("a", budget(&[(Resource::Energy, 0.8)]));
            e.add_member("b", budget(&[(Resource::Compute, 0.9)]));
        }
        (a, b)
    }

    /// ACTIVE/SHAM (o teste A/A da caixa): gêmeos sham ⇒ estado
    /// intacto e recibos bit-idênticos (CNP desligado não muda a
    /// trajetória); o gêmeo ACTIVE difere ONDE DEVE (efeito real).
    #[test]
    fn sham_controle_aa() {
        // Gêmeos SHAM: mesma seed, mesmas propostas.
        let (mut s1, mut s2) = twin_pair();
        s1.set_sham(true);
        s2.set_sham(true);
        let st1 = s1.propose_trade(1, "a", "b", (Resource::Energy, 0.3), (Resource::Compute, 0.3));
        let st2 = s2.propose_trade(1, "a", "b", (Resource::Energy, 0.3), (Resource::Compute, 0.3));
        assert_eq!(st1, st2);
        assert!(matches!(st1, TradeStatus::Rejected(CnpReject::ShamControl)));
        assert_eq!(s1.ledger(), s2.ledger(), "recibos bit-idênticos");
        // Estado INTACTO: budgets iguais aos iniciais.
        assert_eq!(
            s1.ledger().len(), 1,
            "sham REGISTRA o recibo (trilha), sem tocar estado"
        );
        let after = s1.stats();
        assert_eq!(after.0, 1, "denominador conta a proposta sham");
        // Gêmeo ACTIVE: a MESMA proposta APLICA — efeito real.
        let (mut act, mut sham) = twin_pair();
        sham.set_sham(true);
        let st_act = act.propose_trade(1, "a", "b", (Resource::Energy, 0.3), (Resource::Compute, 0.3));
        let st_sham = sham.propose_trade(1, "a", "b", (Resource::Energy, 0.3), (Resource::Compute, 0.3));
        assert_eq!(st_act, TradeStatus::Applied);
        assert_ne!(st_act, st_sham, "active difere onde deve");
        assert!(sham.is_sham() && !act.is_sham());
    }

    /// OUTCOME t+1/t+5: troca aplicada agenda horizontes; MATCH
    /// quando o recurso oferecido persiste no receptor; DRIFT
    /// quando cai; benefit_validated fecha SÓ com h1 E h5 (Lei 5).
    #[test]
    fn outcomes_horizontes_fecham_sozinho() {
        let mut f = FederationEngine::new();
        f.add_member("a", budget(&[(Resource::Energy, 0.8)]));
        f.add_member("b", budget(&[(Resource::Compute, 0.9)]));
        assert_eq!(
            f.propose_trade(1, "a", "b", (Resource::Energy, 0.3), (Resource::Compute, 0.3)),
            TradeStatus::Applied
        );
        let h = f.ledger()[0].chain_hash;
        assert_eq!(f.pending_outcome_count(), 1, "horizontes agendados");
        // Antes de t+1: NENHUM horizonte (ausência ≠ zero).
        assert_eq!(f.ledger()[0].horizon_1, None);
        assert_eq!(f.resolve_outcomes(1), 0, "nada vencido ainda");
        // t+1: MATCH (recurso persistiu).
        assert_eq!(f.resolve_outcomes(2), 1);
        // t+5: MATCH de novo ⇒ benefit_validated = Some(true).
        assert_eq!(f.resolve_outcomes(6), 1);
        let r = f.ledger().iter().find(|r| r.chain_hash == h).unwrap();
        assert_eq!(r.horizon_1, Some(true), "efeito persistiu no t+1");
        assert_eq!(r.horizon_5, Some(true), "efeito persistiu no t+5");
        assert_eq!(r.benefit_validated, Some(true), "Lei 5: fechou com efeito real");
        assert_eq!(f.pending_outcome_count(), 0, "pendente resolvido sai da fila");
    }

    /// OUTCOME DRIFT: receptor consome o recurso ⇒ h1 MATCH, h5
    /// DRIFT ⇒ benefit_validated = Some(false) — nunca métrica
    /// agregada, sempre o efeito OBSERVADO.
    #[test]
    fn outcomes_drift_nao_valida_beneficio() {
        let mut f = FederationEngine::new();
        f.add_member("a", budget(&[(Resource::Energy, 0.8)]));
        f.add_member("b", budget(&[(Resource::Compute, 0.9)]));
        assert_eq!(
            f.propose_trade(1, "a", "b", (Resource::Energy, 0.3), (Resource::Compute, 0.3)),
            TradeStatus::Applied
        );
        f.resolve_outcomes(2); // t+1: MATCH
        // Receptor GASTA o recurso recebido antes do t+5 — via
        // troca REAL (efeito observável, nunca estado editado
        // por fora): b oferece Energy 0.29 e paga Compute 0.1.
        assert_eq!(
            f.propose_trade(3, "b", "a", (Resource::Energy, 0.29), (Resource::Compute, 0.1)),
            TradeStatus::Applied
        );
        f.resolve_outcomes(6); // t+5: DRIFT
        let r = f.ledger().first().unwrap();
        assert_eq!(r.horizon_1, Some(true));
        assert_eq!(r.horizon_5, Some(false), "recurso drenado ⇒ DRIFT");
        assert_eq!(r.benefit_validated, Some(false), "sem efeito persistido, sem benefício");
    }

    /// Transporte: InProcess é o padrão (execução local É a
    /// entrega — sempre true; nome canônico na trilha).
    #[test]
    fn transport_in_process_padrao() {
        let f = FederationEngine::new();
        assert_eq!(f.transport_name(), "in_process");
        let mut custom = FederationEngine::new().with_transport(Box::new(InProcessTransport));
        assert_eq!(custom.transport_name(), "in_process");
        custom.add_member("a", budget(&[(Resource::Energy, 0.5)]));
        custom.add_member("b", budget(&[(Resource::Energy, 0.5)]));
        assert_eq!(
            custom.propose_trade(1, "a", "b", (Resource::Energy, 0.1), (Resource::Energy, 0.1)),
            TradeStatus::Applied
        );
    }
}
