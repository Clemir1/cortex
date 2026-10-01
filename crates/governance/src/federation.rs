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
    /// 20.6c (ADR ratificado): a policy Lua de federação retornou
    /// proposta INVÁLIDA — recibo tipado SEM troca (a policy propos
    /// fora da faixa validada pelo Rust). Ausência de policy NÃO
    /// cai aqui: política é opcional, segue canônico.
    PolicyRejected {
        /// Razão canônica da rejeição (Lei 3: nunca vazia).
        reason: String,
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
            CnpReject::PolicyRejected { .. } => "policy_rejected",
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

/// 17.13 ESCALA HORIZONTAL REAL: porta remota (feature
/// `federation-remote`, DESLIGADA por padrão — feature OFF = o
/// binário é bit-idêntico ao canônico). Dois ORGANISMOS (engines
/// FederationEngine INDEPENDENTES, budgets e chain_hash próprios)
/// trocam recibos por um enlace de memória local determinístico —
/// sem rede externa, sem TCP/UDP, sem relógio: o hash de cada lado
/// evolui só com payload + chain corrente (cross-run bit-exato).
#[cfg(feature = "federation-remote")]
pub mod remote {
    use super::{FederationTransport, TradeRecord};
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    /// Estado do enlace: uma fila POR SENTIDO. As engines nunca
    /// partilham estado — só recibos imutáveis cruzam a fronteira.
    #[derive(Default)]
    struct LinkState {
        to_b: VecDeque<TradeRecord>,
        to_a: VecDeque<TradeRecord>,
    }

    /// Lado do enlace (a engine sabe apenas o próprio lado).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Side {
        /// Organismo A (entrega empilha no canal de B).
        A,
        /// Organismo B (entrega empilha no canal de A).
        B,
    }

    /// Transporte remoto por memória local: a entrega de um lado
    /// empilha o recibo no canal do OUTRO organismo. `deliver` só
    /// retorna false se o enlace estiver fechado (janela morta).
    pub struct RemoteTransport {
        side: Side,
        state: Arc<Mutex<LinkState>>,
        open: bool,
    }

    impl std::fmt::Debug for RemoteTransport {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("RemoteTransport")
                .field("side", &self.side)
                .field("open", &self.open)
                .finish()
        }
    }

    impl RemoteTransport {
        /// Consome os recibos que CHEGARAM a este lado (o espelho
        /// da troca do outro organismo). Ordem FIFO determinística.
        pub fn drain(&mut self) -> Vec<TradeRecord> {
            let mut state = match self.state.lock() {
                Ok(s) => s,
                Err(_) => return Vec::new(), // enlace envenenado: vazio, não erro
            };
            let queue = match self.side {
                Side::A => &mut state.to_a,
                Side::B => &mut state.to_b,
            };
            queue.drain(..).collect()
        }

        /// Fecha a janela deste lado (simula queda do enlace).
        pub fn close(&mut self) {
            self.open = false;
        }
    }

    impl FederationTransport for RemoteTransport {
        fn name(&self) -> &'static str {
            "memory_remote"
        }
        fn deliver(&mut self, record: &TradeRecord) -> bool {
            if !self.open {
                return false; // enlace caído: recibo honesto (TransportUnreachable)
            }
            let Ok(mut state) = self.state.lock() else {
                return false;
            };
            let queue = match self.side {
                Side::A => &mut state.to_b,
                Side::B => &mut state.to_a,
            };
            queue.push_back(record.clone());
            true
        }
    }

    /// 17.13: enlace ponto-a-ponto — cria as DUAS pontas para os
    /// dois organismos (A entrega para B e vice-versa).
    pub fn memory_link() -> (RemoteTransport, RemoteTransport) {
        let state = Arc::new(Mutex::new(LinkState::default()));
        (
            RemoteTransport { side: Side::A, state: Arc::clone(&state), open: true },
            RemoteTransport { side: Side::B, state, open: true },
        )
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
    /// 20.5b (sessão 7, sob diretriz da dona) — ledger de RESOLUÇÕES
    /// por tick: chain_hash de cada horizonte fechado em `now`
    /// (h1 e h5 do mesmo recibo contam cada um). O host do ciclo
    /// drena com `take_resolved_ids(tick)` e alimenta a fonte que o
    /// scheduler consulta (`FederationOutcomeSource`) — o StepReport
    /// publica os ids RESOLVIDOS no tick.
    resolved_ids_log: std::collections::BTreeMap<u64, Vec<u64>>,
    /// 20.6c (feature federation-policy, ADR ratificado): host de
    /// policy Lua PROPOSITORA. None = canônico Rust puro (bit-
    /// idêntico); Some = a policy "federation" é consultada no
    /// ponto de PROPOSTA (need→proposal): ela propõe o cap de
    /// perna, o Rust valida (whitelist + faixas) e aplica. Lua
    /// NUNCA escreve estado (fronteira rust_lua_boundary.md).
    #[cfg(feature = "federation-policy")]
    policy_host: Option<triad_lua::PolicyHost>,
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
            resolved_ids_log: std::collections::BTreeMap::new(),
            #[cfg(feature = "federation-policy")]
            policy_host: None,
        }
    }

    /// 18.4: troca o transporte de entrega (padrão InProcess).
    pub fn with_transport(mut self, transport: Box<dyn FederationTransport>) -> Self {
        self.transport = transport;
        self
    }

    /// 20.6c (ratificado): injeta o host de policy Lua da
    /// federação (só sob a feature). Sem host ⇒ canônico puro.
    #[cfg(feature = "federation-policy")]
    pub fn with_policy_host(mut self, host: triad_lua::PolicyHost) -> Self {
        self.policy_host = Some(host);
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
        // 20.6c (ratificado, feature federation-policy): a policy
        // Lua "federation" é consultada NO PONTO DE PROPOSTA (need→
        // proposal) — ela propõe o CAP DE PERNA; o Rust já validou
        // (whitelist 0.0..0.5 + reason/ttl/confidence no
        // PolicyHost::call) e aqui apenas APERTA (min com o cap do
        // motor — a policy nunca afrouxa acima do controle). Sem
        // host/banda morta/policy ausente = canônico (política é
        // OPCIONAL — ausência ≠ erro); proposta inválida = recibo
        // tipado PolicyRejected SEM troca. As pernas efetivas
        // entram no recibo (proveniência auditável). Feature OFF:
        // este bloco NÃO EXISTE no binário — bit-idêntico.
        #[cfg(feature = "federation-policy")]
        let (offer, payment, status) = if self.sham {
            // Sham ANTES da policy: o controle desligado nunca
            // consulta (nem é afetado) — A/A ativo/sham preservado.
            (offer, payment, TradeStatus::Rejected(CnpReject::ShamControl))
        } else {
            match self.policy_trade_cap(from, offer.0) {
                Err(reason) => (offer, payment, TradeStatus::Rejected(CnpReject::PolicyRejected { reason })),
                Ok(Some(policy_cap)) => {
                    let cap = self.trade_rate_cap.min(policy_cap);
                    let offer = (offer.0, offer.1.min(cap).max(0.0));
                    let payment = (payment.0, payment.1.min(cap).max(0.0));
                    let status = self.evaluate(from, to, offer, payment);
                    (offer, payment, status)
                }
                Ok(None) => {
                    let status = self.evaluate(from, to, offer, payment);
                    (offer, payment, status)
                }
            }
        };
        #[cfg(not(feature = "federation-policy"))]
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

    /// 20.6c (feature federation-policy): consulta a policy Lua
    /// "federation" no ponto de proposta. `Ok(None)` = canônico
    /// (sem host, banda morta — policy propos `nil` — ou policy
    /// ausente: política é OPCIONAL, ausência ≠ erro de troca).
    /// `Ok(Some(cap))` = cap de perna VALIDADO pelo Rust (faixa
    /// estrutural 0.0..0.5 + reason/ttl/confidence — whitelist).
    /// `Err(reason)` = proposta INVÁLIDA ⇒ recibo PolicyRejected
    /// sem troca (Lei 3: razão canônica nunca vazia).
    #[cfg(feature = "federation-policy")]
    fn policy_trade_cap(
        &mut self,
        from: &str,
        resource: Resource,
    ) -> Result<Option<f64>, String> {
        let Some(host) = self.policy_host.as_mut() else {
            return Ok(None); // sem host: canônico bit-idêntico
        };
        // Sinal da policy (federation.lua): escassez do OFERTANTE
        // no recurso oferecido (contexto canônico, Option — o host
        // só passa o que existe, ausência nunca vira zero).
        let scarcity = self.members.get(from).map(|m| m.scarcity(resource));
        let ctx = triad_lua::PolicyContext {
            mean_energy: None,
            prediction_error: None,
            active_fraction: scarcity,
        };
        match host.call("federation", ctx) {
            // Banda morta: policy propos nada — canônico.
            Ok(None) => Ok(None),
            Ok(Some(vp)) => {
                if vp.module == "federation" && vp.parameter == "trade_rate" {
                    tracing::info!(
                        policy_hash = vp.policy_hash,
                        reason = %vp.reason,
                        cap = vp.value,
                        "policy Lua de federação aplicada (20.6c)"
                    );
                    Ok(Some(vp.value))
                } else {
                    // Alvo inesperado: o CNP consulta por trade_rate;
                    // outra proposta é IGNORADA com registro (Rust
                    // arbitra — a policy não redireciona o alvo).
                    tracing::warn!(
                        module = %vp.module,
                        parameter = %vp.parameter,
                        "policy retornou alvo inesperado — seguindo canônico"
                    );
                    Ok(None)
                }
            }
            // Ausência da policy "federation": política opcional —
            // segue canônico (nunca erro da troca; ausência ≠ zero).
            Err(triad_lua::PolicyReject::UnknownTarget { .. }) => {
                tracing::warn!("policy 'federation' ausente — seguindo canônico");
                Ok(None)
            }
            Err(other) => Err(policy_reject_reason(&other)),
        }
    }

    /// 17.13 ESCALA HORIZONTAL (feature federation-remote): aplica
    /// o ESPELHO de uma troca remota — o recibo que o organismo A
    /// entregou pelo enlace é reincidente aqui pelo organismo B:
    /// o membro LOCAL `to` recebe a perna de oferta e paga a perna
    /// de pagamento (o papel de `from` pertence a A — não é
    /// tocado em B). Chain_hash de B: FNV determinístico do MESMO
    /// payload + chain corrente de B — cross-run bit-exato por
    /// construção. Sham: recusa tipada SEM aplicar (o controle
    /// desligado nunca aceita espelho — A/A preservado). Recibo
    /// não-aplicado (A reverteu?): TransportUnreachable honesto.
    #[cfg(feature = "federation-remote")]
    pub fn apply_remote_trade(&mut self, record: &TradeRecord) -> TradeStatus {
        if self.sham {
            return TradeStatus::Rejected(CnpReject::ShamControl);
        }
        if record.status != TradeStatus::Applied {
            return TradeStatus::Rejected(CnpReject::TransportUnreachable {
                transport: "memory_remote".to_string(),
            });
        }
        let Some(mut receiver) = self.members.get(&record.to).cloned() else {
            return TradeStatus::Rejected(CnpReject::UnknownMember { id: record.to.clone() });
        };
        // Reserva da perna de pagamento (mesma régua do evaluate).
        if receiver.available(record.payment.0) < record.payment.1 {
            return TradeStatus::Rejected(CnpReject::InsufficientReserve {
                available: receiver.available(record.payment.0),
                needed: record.payment.1,
            });
        }
        receiver.budget.insert(
            record.payment.0,
            (receiver.budget.get(&record.payment.0).copied().unwrap_or(0.0)
                - record.payment.1)
                .max(0.0),
        );
        *receiver.budget.entry(record.offer.0).or_insert(0.0) += record.offer.1;
        self.members.insert(record.to.clone(), receiver);
        // Recibo ESPELHO no ledger de B (proveniência: o payload da
        // troca original é preservado; o hash é NOSSO, encadeado
        // na trilha local — organismos independentes, hashes
        // independentes).
        let mut mirror = record.clone();
        mirror.chain_hash = self.chain_next(&mirror);
        self.ledger.push(mirror);
        if self.ledger.len() > 64 {
            self.ledger.remove(0);
        }
        TradeStatus::Applied
    }
}

/// 20.6c: razão canônica da rejeição de policy (Lei 3: nunca vazia).
#[cfg(feature = "federation-policy")]
fn policy_reject_reason(reject: &triad_lua::PolicyReject) -> String {
    use triad_lua::PolicyReject;
    match reject {
        PolicyReject::ValueOutOfRange { parameter, value, min, max } => {
            format!("value_out_of_range: {parameter}={value} fora de [{min},{max}]")
        }
        PolicyReject::EmptyReason => "reason vazio (Lei 3)".to_string(),
        PolicyReject::ConfidenceOutOfRange { value } => {
            format!("confidence fora de [0,1]: {value}")
        }
        PolicyReject::TtlOutOfRange { value } => format!("ttl fora de 1..=64: {value}"),
        PolicyReject::ValueNotFinite { value } => format!("valor não finito: {value}"),
        PolicyReject::UnknownTarget { module, parameter } => {
            format!("alvo desconhecido: {module}.{parameter}")
        }
    }
}

impl FederationEngine {
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
                // 20.5b: registra chain_hash resolvido no tick corrente.
                self.resolved_ids_log.entry(now).or_default().push(p.chain_hash);
            }
            if now >= p.at_tick_5 && record.horizon_5.is_none() {
                record.horizon_5 = Some(matched);
                resolved += 1;
                self.resolved_ids_log.entry(now).or_default().push(p.chain_hash);
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

    /// 20.5b (sessão 7, sob diretriz da dona): drena os chain_hashes
    /// RESOLVIDOS no tick dado (h1/h5 fechados em `resolve_outcomes`).
    /// Ausência de resolução ⇒ Vec vazio (ausência ≠ zero). O host do
    /// ciclo chama após cada `resolve_outcomes(t)` para alimentar a
    /// fonte do scheduler (`FederationOutcomeSource`).
    pub fn take_resolved_ids(&mut self, tick: u64) -> Vec<u64> {
        self.resolved_ids_log.remove(&tick).unwrap_or_default()
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

    /// 17.13: budget corrente do membro (observacional; None =
    /// membro/recurso inexistente — ausência ≠ zero, Lei 2).
    pub fn member_budget(&self, id: &str, r: Resource) -> Option<f64> {
        self.members.get(id).and_then(|m| m.budget.get(&r).copied())
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
    /// 20.5b (sessao 7, sob diretriz da dona): `resolve_outcomes`
    /// registra os chain_hashes resolvidos POR TICK e
    /// `take_resolved_ids(tick)` drena a lista — o host alimenta a
    /// fonte que o StepReport consulta. Ausência de resolução =>
    /// lista vazia (ausência não e zero). A/A: mesma história =>
    /// mesmos ids.
    #[test]
    fn outcomes_resolvidos_por_tick_sao_drenaveis_aa_205b() {
        let run = || {
            let mut f = FederationEngine::new();
            f.add_member("a", budget(&[(Resource::Energy, 0.8)]));
            f.add_member("b", budget(&[(Resource::Compute, 0.9)]));
            assert_eq!(
                f.propose_trade(1, "a", "b", (Resource::Energy, 0.3), (Resource::Compute, 0.3)),
                TradeStatus::Applied
            );
            let h = f.ledger()[0].chain_hash;
            f.resolve_outcomes(2);
            let ids_t2 = f.take_resolved_ids(2);
            let ids_t3 = f.take_resolved_ids(3);
            f.resolve_outcomes(6);
            let ids_t6 = f.take_resolved_ids(6);
            let ids_t2_de_novo = f.take_resolved_ids(2);
            (h, ids_t2, ids_t3, ids_t6, ids_t2_de_novo)
        };
        let (h, ids_t2, ids_t3, ids_t6, drenado) = run();
        assert_eq!(ids_t2, vec![h], "h1 fechado no t+1 => id no tick 2");
        assert!(ids_t3.is_empty(), "tick sem resolucao => vazio");
        assert_eq!(ids_t6, vec![h], "h5 fechado no t+5 => id no tick 6");
        assert!(drenado.is_empty(), "drenado: segunda leitura do mesmo tick e vazia");
        let gemea = || run();
        assert_eq!(gemea(), gemea(), "gemeos bit-exatos (A/A)");
    }

}

/// 20.6c (ratificado): testes do WIRE CNP⇄PolicyHost — rodam SÓ
/// sob a feature `federation-policy` (feature-off = binário
/// canônico bit-idêntico, sem estes caminhos). A policy real da
/// casa (lua/policies/federation.lua) é determinística: sem
/// estado, sem tempo, sem aleatoriedade.
#[cfg(all(test, feature = "federation-policy"))]
mod federation_policy_tests {
    use super::*;

    fn budget(pairs: &[(Resource, f64)]) -> std::collections::BTreeMap<Resource, f64> {
        pairs.iter().copied().collect()
    }

    /// Boot REAL do host (sandbox + init.lua + policies da casa) a
    /// partir da raiz do repo — o mesmo que o organismo usa.
    fn host() -> triad_lua::PolicyHost {
        let cfg = triad_lua::LuaCfg::default();
        triad_lua::PolicyHost::boot(&cfg, std::path::Path::new("../../"))
            .expect("boot do PolicyHost com as policies da casa")
    }

    /// A policy real é consultada e o cap VALIDADO entra no recibo:
    /// a perna efetiva é min(cap do motor 0.5, cap da policy) — a
    /// Lua nunca propõe acima do controle do Rust (whitelist).
    #[test]
    fn policy_lua_aplica_cap_validado_na_perna() {
        let mut f = FederationEngine::with_policy(CnpPolicy::default())
            .with_policy_host(host());
        // Escassez conhecida: budget E=0.6 ⇒ escassez 0.4 ⇒ policy
        // (federation.lua): t=(0.4-0.2)/0.6=1/3 ⇒ rate=0.02+0.48/3.
        f.add_member("a", budget(&[(Resource::Energy, 0.6)]));
        f.add_member("b", budget(&[(Resource::Energy, 0.6)]));
        let cap = f
            .policy_trade_cap("a", Resource::Energy)
            .expect("reason tipada")
            .expect("policy da casa presente");
        assert!((0.02..=0.5).contains(&cap), "cap validado fora da faixa: {cap}");
        assert!((cap - 0.18).abs() < 1e-9, "policy determinística: cap={cap}");
        let _status = f.propose_trade(1, "a", "b", (Resource::Energy, 0.5), (Resource::Energy, 0.5));
        let rec = f.ledger().last().expect("recibo");
        let efetiva = 0.5_f64.min(cap);
        assert!(
            (rec.offer.1 - efetiva).abs() < 1e-9 && (rec.payment.1 - efetiva).abs() < 1e-9,
            "pernas efetivas {}/{} != min(cap_motor, cap_policy)={efetiva} (cap={cap})",
            rec.offer.1,
            rec.payment.1
        );
    }

    /// A/A GÊMEOS com a policy LIGADA: mesma seed ⇒ mesma proposta
    /// ⇒ mesma trajetória — chain_hash por passo idêntico (a
    /// policy é determinística; herança da 17.6: sem _ms/run_id).
    #[test]
    fn gemeos_aa_com_policy_bit_exato() {
        let seq = |f: &mut FederationEngine| -> Vec<(TradeStatus, u64)> {
            let mut s = Vec::new();
            for tick in 1..=6 {
                f.propose_trade(tick, "a", "b", (Resource::Energy, 0.4), (Resource::Compute, 0.3));
                f.propose_trade(tick, "b", "a", (Resource::Compute, 0.2), (Resource::Energy, 0.4));
                let rec = f.ledger().last().expect("recibo");
                s.push((rec.status.clone(), rec.chain_hash));
            }
            s
        };
        let mut f1 = FederationEngine::with_policy(CnpPolicy::default()).with_policy_host(host());
        let mut f2 = FederationEngine::with_policy(CnpPolicy::default()).with_policy_host(host());
        for f in [&mut f1, &mut f2] {
            f.add_member("a", budget(&[(Resource::Energy, 0.6), (Resource::Compute, 0.5)]));
            f.add_member("b", budget(&[(Resource::Energy, 0.6), (Resource::Compute, 0.5)]));
        }
        let (s1, s2) = (seq(&mut f1), seq(&mut f2));
        assert_eq!(s1, s2, "gêmeos: trajetória divergiu com a policy ligada");
    }

    /// SHAM + policy: o gate do controle fica ANTES da consulta —
    /// desligado, NENHUMA troca aplicada e recibo tipado (o sham
    /// não pode ser afetado pela policy — 18.4/20.6c A/A).
    #[test]
    fn sham_com_policy_nunca_age() {
        let mut f = FederationEngine::with_policy(CnpPolicy::default()).with_policy_host(host());
        f.add_member("a", budget(&[(Resource::Energy, 0.6)]));
        f.add_member("b", budget(&[(Resource::Energy, 0.6)]));
        f.set_sham(true);
        let status = f.propose_trade(1, "a", "b", (Resource::Energy, 0.3), (Resource::Energy, 0.1));
        assert_eq!(status, TradeStatus::Rejected(CnpReject::ShamControl));
        assert!(
            !f.ledger().iter().any(|r| r.status == TradeStatus::Applied),
            "sham: nenhuma troca aplicada"
        );
    }

    /// SEM host (feature ON, host None): caminho canônico puro — a
    /// policy ausente nunca fabrica cap, erro ou clamp (caso
    /// canônico da casa: membros 0.5, pernas 0.1 ⇒ Applied).
    #[test]
    fn sem_host_e_canonico_puro() {
        let mut f = FederationEngine::with_policy(CnpPolicy::default());
        f.add_member("a", budget(&[(Resource::Energy, 0.5)]));
        f.add_member("b", budget(&[(Resource::Energy, 0.5)]));
        assert_eq!(f.policy_trade_cap("a", Resource::Energy), Ok(None));
        let status = f.propose_trade(1, "a", "b", (Resource::Energy, 0.1), (Resource::Energy, 0.1));
        assert_eq!(status, TradeStatus::Applied);
        let rec = f.ledger().last().expect("recibo");
        assert!(
            (rec.offer.1 - 0.1).abs() < 1e-9 && (rec.payment.1 - 0.1).abs() < 1e-9,
            "pernas canônicas sem clamp de policy"
        );
    }

    /// Razão canônica da rejeição (Lei 3: nunca vazia) — o recibo
    /// PolicyRejected carrega a causa estrutural da policy.
    #[test]
    fn reason_da_rejeicao_nunca_e_vazia() {
        let casos = [
            triad_lua::PolicyReject::ValueOutOfRange {
                parameter: "trade_rate".into(),
                value: 9.0,
                min: 0.0,
                max: 0.5,
            },
            triad_lua::PolicyReject::EmptyReason,
            triad_lua::PolicyReject::ConfidenceOutOfRange { value: 2.0 },
            triad_lua::PolicyReject::TtlOutOfRange { value: 99 },
            triad_lua::PolicyReject::ValueNotFinite { value: f64::NAN },
        ];
        for c in &casos {
            let r = policy_reject_reason(c);
            assert!(!r.trim().is_empty(), "Lei 3: razão vazia");
        }
    }

}

/// 17.13 ESCALA HORIZONTAL REAL: validação cross-run entre DOIS
/// organismos sobre o transporte memory_remote (feature
/// `federation-remote`). Feature OFF: este módulo NÃO EXISTE no
/// binário — bit-idêntico.
#[cfg(all(test, feature = "federation-remote"))]
mod tests_remote {
    use super::remote::memory_link;
    use super::{CnpReject, FederationEngine, Resource, TradeStatus, TradeRecord};
    use std::collections::BTreeMap;

    /// Organismo A: doador "a1" (energia 0.8, memória 0.6).
    fn organismo_a() -> FederationEngine {
        let mut a = FederationEngine::new();
        a.add_member(
            "a1",
            BTreeMap::from([(Resource::Energy, 0.8), (Resource::Memory, 0.6)]),
        );
        a.set_trade_rate(0.5);
        a
    }

    /// Organismo B: receptor "b1" (energia 0.2, memória 0.7) —
    /// engine INDEPENDENTE (budget e chain próprios).
    fn organismo_b() -> FederationEngine {
        let mut b = FederationEngine::new();
        b.add_member(
            "b1",
            BTreeMap::from([(Resource::Energy, 0.2), (Resource::Memory, 0.7)]),
        );
        b
    }

    /// Um run do protocolo: A propõe 2 trocas ao membro espelho
    /// "b1" (registrado em A para a reserva da perna de pagamento),
    /// entrega pelo enlace; B drena e aplica os espelhos. Devolve
    /// hashes dos DOIS chains + budgets-chave para comparação.
    fn run_cross_organism() -> (u64, u64, f64, f64, f64, f64) {
        let mut a = organismo_a();
        a.add_member(
            "b1",
            BTreeMap::from([(Resource::Energy, 0.2), (Resource::Memory, 0.7)]),
        );
        let mut b = organismo_b();
        let (ta, mut tb) = memory_link();
        a = a.with_transport(Box::new(ta)); // ponta A: dentro da engine A
        let s1 = a.propose_trade(1, "a1", "b1", (Resource::Energy, 0.1), (Resource::Memory, 0.1));
        let s2 = a.propose_trade(2, "a1", "b1", (Resource::Energy, 0.2), (Resource::Memory, 0.05));
        assert_eq!(s1, TradeStatus::Applied);
        assert_eq!(s2, TradeStatus::Applied);
        let entregues: Vec<TradeRecord> = tb.drain();
        assert_eq!(entregues.len(), 2, "as duas trocas chegaram ao organismo B");
        for r in &entregues {
            let st = b.apply_remote_trade(r);
            assert_eq!(st, TradeStatus::Applied, "espelho aplicado em B");
        }
        let ha = a.ledger().last().map(|r| r.chain_hash).unwrap_or(0);
        let hb = b.ledger().last().map(|r| r.chain_hash).unwrap_or(0);
        let a_e = a.member_budget("a1", Resource::Energy).unwrap_or(-1.0);
        let a_m = a.member_budget("a1", Resource::Memory).unwrap_or(-1.0);
        let b_e = b.member_budget("b1", Resource::Energy).unwrap_or(-1.0);
        let b_m = b.member_budget("b1", Resource::Memory).unwrap_or(-1.0);
        (ha, hb, a_e, a_m, b_e, b_m)
    }

    /// 17.13 nº1: transfer nos DOIS lados com efeito consistente —
    /// A perdeu oferta/ganhou pagamento; B ganhou oferta/perdeu
    /// pagamento (conservação CROSS: o que saiu de A chegou a B).
    #[test]
    fn cross_organism_transfer_aplicado_nos_dois_lados() {
        let (_, _, a_e, a_m, b_e, b_m) = run_cross_organism();
        // a1 (em A): 0.8 − 0.1 − 0.2 = 0.5 energia; 0.6 + 0.1 + 0.05 = 0.75 memória.
        assert!((a_e - 0.5).abs() < 1e-12, "a1 energia: {a_e}");
        assert!((a_m - 0.75).abs() < 1e-12, "a1 memória: {a_m}");
        // b1 (em B): 0.2 + 0.1 + 0.2 = 0.5 energia; 0.7 − 0.1 − 0.05 = 0.55 memória.
        assert!((b_e - 0.5).abs() < 1e-12, "b1 energia: {b_e}");
        assert!((b_m - 0.55).abs() < 1e-12, "b1 memória: {b_m}");
        // Conservação CROSS (A perdeu X, B ganhou X — o total dos
        // dois organismos é constante: 1.0 energia, 1.3 memória).
        assert!(((a_e + b_e) - 1.0).abs() < 1e-12, "energia cross conservada: {}", a_e + b_e);
        assert!(((a_m + b_m) - 1.3).abs() < 1e-12, "memória cross conservada: {}", a_m + b_m);
    }

    /// 17.13 nº2: re-run BIT-EXATO nos DOIS chains. Os hashes de A
    /// e B COINCIDEM por SIMETRIA CANÔNICA do protocolo espelhado:
    /// mesmas gêneses (chain=0) + mesmos payloads ⇒ mesmo FNV — a
    /// independência dos organismos está provada pelo nº1 (budgets
    /// e conservação SEPARADOS) e nº3 (sham de um lado NUNCA vaza
    /// para o outro); o re-run bit-exato é o contrato 17.13.
    #[test]
    fn cross_organism_chain_hashes_independentes_bit_exatos() {
        let (ha1, hb1, ..) = run_cross_organism();
        let (ha2, hb2, ..) = run_cross_organism();
        assert_eq!(ha1, ha2, "chain_hash de A: re-run bit-exato");
        assert_eq!(hb1, hb2, "chain_hash de B: re-run bit-exato");
        assert_eq!(
            ha1, hb1,
            "simetria canônica: espelho com gêneses iguais produz hashes iguais"
        );
    }

    /// 17.13 nº3 (A/A): sham em A ⇒ NADA na fila (a proposta é
    /// recusada ANTES da entrega); sham em B ⇒ espelho recusado
    /// tipado e budget de b1 INTACTO.
    #[test]
    fn cross_organism_sham_preservado_nos_dois_lados() {
        // Sham de A: a fila do enlace fica VAZIA.
        let mut a = organismo_a();
        a.add_member(
            "b1",
            BTreeMap::from([(Resource::Energy, 0.2), (Resource::Memory, 0.7)]),
        );
        a.set_sham(true);
        let (_, mut tb) = memory_link();
        let s = a.propose_trade(1, "a1", "b1", (Resource::Energy, 0.1), (Resource::Memory, 0.1));
        assert_eq!(s, TradeStatus::Rejected(CnpReject::ShamControl));
        assert!(tb.drain().is_empty(), "sham em A: nenhum recibo entregue");
        // Sham de B: o espelho é recusado tipado SEM aplicar.
        let mut a2 = organismo_a();
        a2.add_member(
            "b1",
            BTreeMap::from([(Resource::Energy, 0.2), (Resource::Memory, 0.7)]),
        );
        let mut b2 = organismo_b();
        b2.set_sham(true);
        let s2 = a2.propose_trade(1, "a1", "b1", (Resource::Energy, 0.1), (Resource::Memory, 0.1));
        assert_eq!(s2, TradeStatus::Applied);
        let rec = a2.ledger().last().cloned().expect("recibo aplicado em A");
        let st = b2.apply_remote_trade(&rec);
        assert_eq!(st, TradeStatus::Rejected(CnpReject::ShamControl));
        let e = b2.member_budget("b1", Resource::Energy).expect("b1 existe");
        assert!((e - 0.2).abs() < 1e-12, "sham em B: budget intacto (era {e})");
    }
}
