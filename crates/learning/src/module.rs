//! Módulo transversal de learning (17.8/16.6 antiga numeração):
//! CONSUME os ciclos FECHADOS do L4 (`closed_log` — telemetria por
//! decisão com `verified_future_effect`, Lei 5) e atribui CRÉDITO
//! REAL POR MÓDULO com denominador, esquecimento exponencial e
//! limite de atualização por step. LEI 4: sempre ativo — sem fonte,
//! ausência contada (nunca zero fantasma, nunca standby).
//!
//! A cadeia com terminação: prediction→outcome→error→credit→
//! update→future_behavior→validation. O crédito produzido aqui é
//! um REGISTRO de avaliação por módulo (o update de PESOS vive em
//! quem os possui — fronteira Rust/Lua: nenhum estado de outra
//! camada é mutado aqui).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use triad_contracts as tc;
use triad_foundation as tf;
use triad_l4_global as l4;
use triad_runtime as rt;
use tracing::{debug, info, warn};

use crate::config::LearningCfg;
use crate::forgetting::ForgettingEngine;
use crate::policy::LearningPolicy;

/// Teto de registros já processados lembrados (cursor FIFO: o
/// closed_log do L4 tem cap 64; lembrar o dobro basta sem crescer
/// sem limite).
const CURSOR_CAP: usize = 128;
/// Teto de traços de crédito mantidos (poda dos fracos).
const TRACE_CAP: usize = 256;
/// Intervalo de PODA em ticks: traços novos têm janela para
/// ACUMULAR reforços antes da poda (poda todo tick cegaria o
/// instrumento antes de qualquer acumulação).
const PRUNE_INTERVAL: u64 = 64;

/// Telemetria honesta do learning — denominadores sempre visíveis.
#[derive(Debug, Default, Clone)]
pub struct LearningStats {
    /// Ticks executados (denominador natural).
    pub ticks: u64,
    /// Registros FECHADOS novos consumidos do L4.
    pub records_consumed: u64,
    /// Ciclos CONFIRMADOS que geraram crédito.
    pub credits_awarded: u64,
    /// Ciclos fechados NÃO confirmados (sem crédito, contados).
    pub skipped_unconfirmed: u64,
    /// Registros JÁ vistos no cursor (idempotência).
    pub already_seen: u64,
    /// Ticks SEM fonte L4 (ausência contada — nunca standby).
    pub no_source_ticks: u64,
    /// Traços podados por fraqueza.
    pub pruned_traces: u64,
    /// Rodadas de poda executadas (denominador: ticks/PRUNE_INTERVAL).
    pub prune_rounds: u64,
    /// eta corrente da política (nunca suspensa).
    pub eta: f32,
    /// Soma |efeito líquido| dos novos (erro médio com denominador
    /// `credits_awarded + skipped_unconfirmed`).
    pub abs_effect_sum: f32,
    /// Proposals eta APLICADAS do PolicyHost (17.12) — Rust valida.
    pub eta_policy_applied: u64,
    /// Proposals eta REJEITADAS (fora da faixa da casa) — tipadas.
    pub eta_policy_rejected: u64,
    /// 17.8: validações de janela com efeito VERIFICADO (erro nos
    /// dois horizontes < base + decisão futura) — denominador total.
    pub validation_hits: u64,
    /// 17.8: janelas fechadas INCOMPLETAS (ausência de observação —
    /// contadas, SEM penalidade: ausência ≠ zero).
    pub validation_incomplete: u64,
    /// 17.8: janelas com efeito NÃO sustentado (erro não caiu ou
    /// sem decisão futura com observação presente — penalizadas).
    pub validation_misses: u64,
}

/// Módulo transversal de learning: fecha a cadeia com crédito real.
pub struct LearningModule {
    descriptor: tc::ModuleDescriptor,
    config: LearningCfg,
    /// Ponte read-only ao L4 (closed_log: cicos fechados com
    /// verified_future_effect). None = ausência contada.
    l4: Option<Arc<l4::L4Module>>,
    inner: Mutex<Inner>,
}

struct Inner {
    /// Traço de crédito POR MÓDULO (chave "l3.prediction" etc.) com
    /// decaimento e poda — o crédito velho esquece.
    traces: ForgettingEngine,
    /// Política eta (nunca suspensa — Lei 4).
    policy: LearningPolicy,
    /// Cursor de registros processados: (decided_tick, content_key).
    cursor: VecDeque<(u64, String)>,
    /// Contadores POR MÓDULO (records, credits) — a TAXA de sucesso
    /// por módulo SEMPRE com denominador (créditos/registros).
    per_module: std::collections::HashMap<String, (u64, u64)>,
    /// 17.8: JANELAS de validação de efeito (legado:
    /// learning_effect_validation.py — abre a cada update, acumula
    /// por idade 1..5, valida em age>=5; incompleta = tipada).
    janelas: VecDeque<Janela>,
    stats: LearningStats,
}

/// 17.8: janela de validação temporal do crédito (t+1/t+5).
/// Fiel ao legado: exige erro observado MENOR que a base nos DOIS
/// horizontes + decisão futura identificada; motivos TIPADOS.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Janela {
    /// Tick do crédito que abriu a janela.
    tick_credit: u64,
    /// Módulo creditado (rótulo de proveniência — NUNCA cadeia
    /// numérica: o audit do legado mostra crédito como rótulo).
    module: String,
    /// |efeito líquido| do crédito (a BASE a superar).
    base_err: f32,
    /// Erro observado no horizonte curto (None = sem observação).
    obs_h1: Option<f32>,
    /// Erro observado no horizonte longo.
    obs_h5: Option<f32>,
    /// Decisão FUTURA identificada (diferente da origem).
    futura_decisao: bool,
    /// Horizontes da config (t+1/t+5 — validation_horizons).
    h1: u64,
    h5: u64,
}

/// Motivo tipado do fechamento de janela (legado:
/// T{h}_WINDOW_INCOMPLETE / FUTURE_DECISION_MISSING / validado).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VereditoJanela {
    Validated,
    T1Incomplete,
    T5Incomplete,
    FutureDecisionMissing,
    ErrNotLower,
}


/// Penalidade fixa por janela de efeito NÃO validada com observação
/// presente (aprendizagem não sustentada decai mais — calibrável).
const VALIDATION_PENALTY: f32 = 0.01;
/// Teto de janelas abertas (FIFO — auditável, bounded).
const JANELAS_CAP: usize = 256;

impl LearningModule {
    /// Compatibilidade: política default, SEM fonte (ausência contada).
    pub fn new() -> Self {
        Self::new_with_config(LearningCfg::default())
    }

    /// Learning REAL com política central `[learning]`.
    pub fn new_with_config(config: LearningCfg) -> Self {
        let mut stats = LearningStats::default();
        stats.eta = 0.01;
        Self {
            inner: Mutex::new(Inner {
                traces: ForgettingEngine::new(TRACE_CAP),
                policy: LearningPolicy::new(),
                cursor: VecDeque::new(),
                per_module: std::collections::HashMap::new(),
                janelas: VecDeque::new(),
                stats,
            }),
            // Descritor canônico (CAMADA.txt, 17.1): T/LEARNING —
            // transversal com suporte declarado; ADAPTAÇÃO (O2);
            // consome efeitos VERIFICADOS (Lei 5) e produz crédito
            // por módulo real ⇒ E3 produtivo (o E5 da própria
            // aprendizagem exige validação futura t+5 — declarado).
            descriptor: tc::ModuleDescriptor::new(
                tf::ModuleId::new(),
                "learning",
                tc::Layer::Transversal,
                "learning",
            )
            .with_support(Some(tc::SupportLayer::Learning))
            .with_orders(&[tc::CyberneticOrder::O2Adaptation])
            .with_state_owner("learning")
            .with_inputs(&["runtime.tick", "l4.learning"])
            .with_outputs(&["learning.credit"])
            .with_backend(tc::ExecutionBackend::CpuSeq)
            .with_criticality(tc::Criticality::Normal)
            .with_dependencies(&["l4.global"])
            .with_evidence(tf::evidence::EvidenceLevel::E3Productive)
            .with_recovery(tc::RecoveryPolicy::RestartModule),
            config,
            l4: None,
        }
    }

    /// Liga a ponte read-only ao L4 (closed_log por decisão).
    pub fn with_l4_source(mut self, l4: Arc<l4::L4Module>) -> Self {
        self.l4 = Some(l4);
        self
    }

    /// Telemetria corrente (auditoria).
    pub fn stats(&self) -> LearningStats {
        self.inner
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .stats
            .clone()
    }

    /// Aplica uma PROPOSAL VALIDADA do PolicyHost (Lua, 17.12):
    /// Rust é o guardião final — faixa da casa [0.005, 0.02];
    /// razão e hash registrados (Lei 3); a aprendizagem NUNCA é
    /// suspensa (Lei 4) — proposal só AJUSTA eta.
    pub fn apply_policy_eta(
        &self,
        value: f32,
        reason: &str,
        policy_hash: u64,
    ) -> Result<f32, String> {
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return Err("mutex de learning envenenado".to_string()),
        };
        let (min, max) = (inner.policy.min_eta, inner.policy.max_eta);
        if !value.is_finite() || value < min || value > max {
            inner.stats.eta_policy_rejected += 1;
            warn!(
                valor = value, min, max,
                policy = format!("{policy_hash:016x}"),
                "proposal eta REJEITADA (fora da faixa da casa — Rust guarda a lei)"
            );
            return Err(format!(
                "eta {value} fora da faixa [{min},{max}] (policy {policy_hash:016x})"
            ));
        }
        inner.policy.eta = value;
        inner.stats.eta = value;
        inner.stats.eta_policy_applied += 1;
        info!(
            eta = value,
            policy = format!("{policy_hash:016x}"),
            razao = reason,
            "proposal eta APLICADA (crise muda política, nunca suspende)"
        );
        Ok(value)
    }

    /// eta corrente da política (nunca suspensa).
    pub fn current_eta(&self) -> f32 {
        self.inner
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .policy
            .eta
    }

    /// 17.8: TRANSFER como o legado define — TAXA DE FLUXO por
    /// fronteira (layer_transfer.py), NUNCA migração de pesos:
    /// quinteto (input, accepted, acted, effect, learned) da
    /// fronteira l4.global→learning, taxas COM denominador e o
    /// GARGALO (menor taxa da cadeia).
    pub fn transfer_rates(&self) -> TransferRates {
        let s = self.stats();
        let input = s.records_consumed;
        let accepted = s.credits_awarded + s.skipped_unconfirmed;
        let acted = s.credits_awarded;
        let effect = s.validation_hits + s.validation_misses;
        let learned = s.validation_hits;
        let f1 = if input > 0 { accepted as f32 / input as f32 } else { 0.0 };
        let f2 = if accepted > 0 { acted as f32 / accepted as f32 } else { 0.0 };
        let f3 = if acted > 0 { effect as f32 / acted as f32 } else { 0.0 };
        let f4 = if acted > 0 { learned as f32 / acted as f32 } else { 0.0 };
        let gargalo = [f1, f2, f3, f4]
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| ["F1", "F2", "F3", "F4"][i])
            .unwrap_or("n/d");
        TransferRates {
            fronteira: "l4.global->learning".to_string(),
            input,
            accepted,
            acted,
            effect,
            learned,
            f1,
            f2,
            f3,
            f4,
            gargalo: gargalo.to_string(),
        }
    }

    /// Crédito (força do traço) por módulo como valor QUALIFICADO
    /// (ausência = NO_DATA — nunca zero).
    pub fn module_credit(
        &self,
        module: &str,
        source: tf::ModuleId,
        step: tf::StepId,
    ) -> tc::Qualified<f32> {
        let inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        inner.traces.strength(module, source, step)
    }

    /// Taxa de sucesso POR MÓDULO com denominador explícito:
    /// (módulo, créditos, registros) — ordenado por registros.
    pub fn module_rates(&self) -> Vec<(String, u64, u64)> {
        let inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let mut v: Vec<(String, u64, u64)> = inner
            .per_module
            .iter()
            .map(|(k, (credits, records))| (k.clone(), *credits, *records))
            .collect();
        v.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
        v
    }

    /// SNAPSHOT cross-run (17.9): estado serializável do learning —
    /// traços vivos, contadores por módulo e telemetria essencial.
    /// Determinístico: mesma história ⇒ mesmo snapshot (A/A testável).
    pub fn snapshot(&self) -> LearningSnapshot {
        let inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        LearningSnapshot {
            traces: inner.traces.export(),
            per_module: {
                let mut v: Vec<(String, u64, u64)> = inner
                    .per_module
                    .iter()
                    .map(|(k, (credits, records))| {
                        (k.clone(), *credits, *records)
                    })
                    .collect();
                v.sort_by(|a, b| a.0.cmp(&b.0));
                v
            },
            records_consumed: inner.stats.records_consumed,
            credits_awarded: inner.stats.credits_awarded,
            skipped_unconfirmed: inner.stats.skipped_unconfirmed,
            no_source_ticks: inner.stats.no_source_ticks,
            eta: inner.policy.eta,
            janelas: inner
                .janelas
                .iter()
                .map(|j| {
                    (
                        j.tick_credit,
                        j.module.clone(),
                        j.base_err,
                        j.obs_h1,
                        j.obs_h5,
                        j.futura_decisao,
                        j.h1,
                        j.h5,
                    )
                })
                .collect(),
            validation: (
                inner.stats.validation_hits,
                inner.stats.validation_incomplete,
                inner.stats.validation_misses,
            ),
        }
    }

    /// RESTORE cross-run (17.9): substitui o estado interno pelo do
    /// snapshot (chamado no boot com procedência VERIFICADA pelo
    /// Checkpointer — aqui o estado é aceito como canônico salvo).
    pub fn restore_snapshot(&self, snap: &LearningSnapshot) {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        inner.traces.import(snap.traces.clone());
        inner.per_module.clear();
        for (k, credits, records) in &snap.per_module {
            inner
                .per_module
                .insert(k.clone(), (*credits, *records));
        }
        inner.stats.records_consumed = snap.records_consumed;
        inner.stats.credits_awarded = snap.credits_awarded;
        inner.stats.skipped_unconfirmed = snap.skipped_unconfirmed;
        inner.stats.no_source_ticks = snap.no_source_ticks;
        inner.policy.eta = snap.eta;
        inner.stats.eta = snap.eta;
        // 17.8: janelas e vereditos sobrevivem ao restore (17.9).
        inner.janelas.clear();
        for (tick_credit, module, base_err, obs_h1, obs_h5, futura, h1, h5) in
            &snap.janelas
        {
            inner.janelas.push_back(Janela {
                tick_credit: *tick_credit,
                module: module.clone(),
                base_err: *base_err,
                obs_h1: *obs_h1,
                obs_h5: *obs_h5,
                futura_decisao: *futura,
                h1: *h1,
                h5: *h5,
            });
        }
        (
            inner.stats.validation_hits,
            inner.stats.validation_incomplete,
            inner.stats.validation_misses,
        ) = snap.validation;
    }
}

impl Default for LearningModule {
    fn default() -> Self {
        Self::new()
    }
}

/// MÓDULO DONO do conteúdo de um ciclo fechado (crédito por módulo —
/// o efeito líquido pertence a quem produziu o conteúdo prometido).
fn owner_module_of(content_key: &str) -> &'static str {
    if content_key.starts_with("predicao:") {
        "l3.prediction"
    } else if content_key == "sinal:coesao" {
        "l2.tissue"
    } else if content_key.starts_with("foco:") {
        "l3.attention"
    } else {
        "l4.global"
    }
}

impl rt::CognitiveModule for LearningModule {
    fn descriptor(&self) -> &tc::ModuleDescriptor {
        &self.descriptor
    }

    /// LEI 4: learning NUNCA em standby — sempre Active (a config
    /// documenta; o estado é invariante).
    fn state(&self) -> rt::ModuleState {
        rt::ModuleState::Active
    }

    fn tick(
        &self,
        ctx: &rt::TypedContext,
        out: &mut Vec<tc::EventEnvelope>,
    ) -> tf::TriadResult<()> {
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => {
                return Err(tf::TriadError::ContractViolation {
                    detail: "mutex de learning envenenado".to_string(),
                })
            }
        };
        inner.stats.ticks += 1;
        let tick = ctx.clock().tick;

        // (0) 17.8: JANELAS de validação (legado: validam em age>=5).
        // Fecho as maduras ANTES de consumir novos registros: HIT
        // exige erro observado < base em t+1 E t+5 + decisão futura;
        // incompletas são CONTADAS sem penalidade (ausência ≠ zero).
        let mut fechar = Vec::new();
        for (i, j) in inner.janelas.iter().enumerate() {
            if tick.saturating_sub(j.tick_credit) >= j.h5 {
                fechar.push(i);
            }
        }
        for i in fechar.into_iter().rev() {
            let j = inner.janelas.remove(i).expect("idx valido");
            let veredito = match (j.obs_h1, j.obs_h5, j.futura_decisao) {
                (None, _, _) => VereditoJanela::T1Incomplete,
                (_, None, _) => VereditoJanela::T5Incomplete,
                (Some(_), Some(_), false) => VereditoJanela::FutureDecisionMissing,
                (Some(e1), Some(e5), true) => {
                    if e1 < j.base_err && e5 < j.base_err {
                        VereditoJanela::Validated
                    } else {
                        VereditoJanela::ErrNotLower
                    }
                }
            };
            match veredito {
                VereditoJanela::Validated => {
                    inner.stats.validation_hits += 1;
                    debug!(modulo = %j.module, "janela de efeito VALIDADA (t+1 e t+5 abaixo da base)");
                }
                VereditoJanela::T1Incomplete
                | VereditoJanela::T5Incomplete => {
                    inner.stats.validation_incomplete += 1;
                    debug!(modulo = %j.module, ?veredito, "janela incompleta: contada, sem penalidade");
                }
                VereditoJanela::FutureDecisionMissing
                | VereditoJanela::ErrNotLower => {
                    inner.stats.validation_misses += 1;
                    // Penalidade pequena e DECLARADA: crédito não
                    // sustentado enfraquece (nunca zera — Lei 4).
                    inner.traces.reinforce(&j.module, -VALIDATION_PENALTY);
                    warn!(modulo = %j.module, ?veredito, "janela de efeito NAO validada (penalizada)");
                }
            }
        }

        // (1) Consome ciclos FECHADOS do L4 (Lei 5 garantida no
        // dono: closed_log só contém cicos com verified_future_effect).
        let mut novos = 0u64;
        match &self.l4 {
            Some(l4src) => {
                for rec in l4src.closed_log() {
                    let key = (rec.decided_tick, rec.content_key.clone());
                    if inner.cursor.contains(&key) {
                        inner.stats.already_seen += 1;
                        continue;
                    }
                    if inner.cursor.len() >= CURSOR_CAP {
                        inner.cursor.pop_front();
                    }
                    inner.cursor.push_back(key);
                    inner.stats.records_consumed += 1;
                    novos += 1;
                    inner.stats.abs_effect_sum += rec.net_effect.abs();
                    let owner = owner_module_of(&rec.content_key).to_string();
                    // (records, credits) do módulo — conta em escopo
                    // próprio (borrow de per_module vive pouco).
                    let confirmed = rec.confirmed;
                    {
                        let entry =
                            inner.per_module.entry(owner.clone()).or_insert((0, 0));
                        entry.0 += 1;
                        if confirmed {
                            entry.1 += 1;
                        }
                    }
                    if confirmed {
                        // CRÉDITO REAL POR MÓDULO: efeito líquido
                        // pesado, limitado por step (estabilidade —
                        // nenhum evento isolado muda a avaliação além
                        // do máximo declarado).
                        let credit = (rec.net_effect
                            * self.config.prediction_error_weight)
                            .clamp(
                                -self.config.max_update_rate,
                                self.config.max_update_rate,
                            );
                        inner.traces.reinforce(&owner, credit);
                        inner.stats.credits_awarded += 1;
                        // 17.8: observação para as janelas ABERTAS do
                        // mesmo módulo — erro observado por idade
                        // (age = tick − tick_credit; horizontes 1 e 5)
                        // e marca a decisão futura identificada.
                        {
                            let err_obs = rec.net_effect.abs();
                            for j in inner.janelas.iter_mut() {
                                if j.module != owner {
                                    continue;
                                }
                                let age = tick.saturating_sub(j.tick_credit);
                                if age >= 1 && age <= j.h5 {
                                    j.futura_decisao = true;
                                    // Bucket por horizonte (legado acumula
                                    // por idade 1..5): a ÚLTIMA amostra
                                    // dentro de cada janela prevalece.
                                    if age <= j.h1 {
                                        j.obs_h1 = Some(err_obs);
                                    } else {
                                        j.obs_h5 = Some(err_obs);
                                    }
                                }
                            }
                        }
                        // 17.8: JANELA abre a cada update (legado:
                        // system.py:8313-8378) — base |efeito|, rótulo
                        // de proveniência (NUNCA cadeia numérica).
                        if inner.janelas.len() >= JANELAS_CAP {
                            inner.janelas.pop_front();
                        }
                        let (h1, h5) = (
                            *self.config.validation_horizons.first().unwrap_or(&1),
                            *self.config.validation_horizons.last().unwrap_or(&5),
                        );
                        inner.janelas.push_back(Janela {
                            tick_credit: tick,
                            module: owner.clone(),
                            base_err: rec.net_effect.abs(),
                            obs_h1: None,
                            obs_h5: None,
                            futura_decisao: false,
                            h1,
                            h5,
                        });
                        debug!(
                            modulo = %owner,
                            credito = credit,
                            efeito_liquido = rec.net_effect,
                            chave = %rec.content_key,
                            "credito atribuido por ciclo confirmado (Lei 5)"
                        );
                    } else {
                        inner.stats.skipped_unconfirmed += 1;
                        warn!(
                            chave = %rec.content_key,
                            "ciclo fechado NAO confirmado: sem credito (contado)"
                        );
                    }
                }
            }
            None => {
                inner.stats.no_source_ticks += 1;
            }
        }

        // (2) Política eta: adapta pelo erro médio dos novos (nunca
        // suspensa — Lei 4).
        let denom = inner.stats.credits_awarded + inner.stats.skipped_unconfirmed;
        if novos > 0 && denom > 0 {
            let mean_err = inner.stats.abs_effect_sum / denom as f32;
            inner.policy.adapt_eta(mean_err);
            inner.stats.eta = inner.policy.eta;
        }

        // (3) Esquecimento por step (decay exponencial declarado) e
        // poda INTERVALADA (a cada PRUNE_INTERVAL ticks): traços
        // novos têm janela de acumulação antes da fraqueza cortar.
        inner.traces.decay(1.0 - self.config.forgetting_rate.clamp(0.0, 0.99));
        if inner.stats.ticks % PRUNE_INTERVAL == 0 {
            inner.stats.pruned_traces += inner.traces.prune() as u64;
            inner.stats.prune_rounds += 1;
        }

        ctx.set(
            "learning.status",
            triad_learning_status(&inner.stats, inner.traces.len()),
        );
        if novos > 0 {
            info!(
                novos = novos,
                creditos = inner.stats.credits_awarded,
                "learning consumiu ciclos fechados e atribuiu credito"
            );
            out.push(rt::envelope(
                "learning.credit",
                tick,
                tc::EventType::Learning,
                tc::Priority::Normal,
            ));
        }
        Ok(())
    }
}

/// 17.8: TRANSFER como taxa de fluxo por fronteira (legado
/// layer_transfer.py) — quinteto COM denominador + gargalo.
pub struct TransferRates {
    /// Fronteira observada.
    pub fronteira: String,
    /// Registros que entraram na fronteira.
    pub input: u64,
    /// Confirmados pela validação do L4 (Lei 5).
    pub accepted: u64,
    /// Que viraram crédito (atuaram no traço).
    pub acted: u64,
    /// Janelas fechadas com veredito (efeito checado no tempo).
    pub effect: u64,
    /// Janelas VALIDADAS (t+1 e t+5 abaixo da base + decisão futura).
    pub learned: u64,
    /// Taxas de conversão COM denominador.
    pub f1: f32,
    pub f2: f32,
    pub f3: f32,
    pub f4: f32,
    /// Menor taxa (onde a cadeia engargala).
    pub gargalo: String,
}

fn triad_learning_status(s: &LearningStats, traces: usize) -> LearningStatus {
    LearningStatus {
        ticks: s.ticks,
        records_consumed: s.records_consumed,
        credits_awarded: s.credits_awarded,
        skipped_unconfirmed: s.skipped_unconfirmed,
        no_source_ticks: s.no_source_ticks,
        pruned_traces: s.pruned_traces,
        eta: s.eta,
        live_traces: traces,
    }
}

/// Fotografia do learning publicada no contexto a cada tick.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LearningStatus {
    pub ticks: u64,
    pub records_consumed: u64,
    pub credits_awarded: u64,
    pub skipped_unconfirmed: u64,
    pub no_source_ticks: u64,
    pub pruned_traces: u64,
    pub eta: f32,
    pub live_traces: usize,
}

/// SNAPSHOT cross-run (17.9): estado serializável do learning.
/// O cursor de idempotência NÃO é salvo — no boot o closed_log
/// restaurado é o MESMO conjunto, e o cursor re-aprende do zero
/// (reprocessar um registro já contado seria erro; o restore
/// reconstrói os contadores pelo snapshot, não pelo replay).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LearningSnapshot {
    /// Traços vivos (chave, força) — ordenado por chave.
    pub traces: Vec<(String, f32)>,
    /// Contadores por módulo (módulo, créditos, registros).
    pub per_module: Vec<(String, u64, u64)>,
    pub records_consumed: u64,
    pub credits_awarded: u64,
    pub skipped_unconfirmed: u64,
    pub no_source_ticks: u64,
    pub eta: f32,
    /// 17.8: janelas de validação abertas (fidelidade cross-run).
    #[serde(default)]
    pub janelas: Vec<(u64, String, f32, Option<f32>, Option<f32>, bool, u64, u64)>,
    /// 17.8: vereditos acumulados (hits, incompletas, penalizadas).
    #[serde(default)]
    pub validation: (u64, u64, u64),
}

#[cfg(test)]
mod tests {
    use super::*;
    use triad_runtime::CognitiveModule as _;

    #[test]
    fn sem_fonte_l4_ausencia_contada_nunca_standby() {
        let lrn = LearningModule::new();
        let mut clock = tf::LogicalClock::new();
        for _ in 0..5 {
            clock.advance();
            let ctx = rt::TypedContext::new(clock);
            let mut out = Vec::new();
            lrn.tick(&ctx, &mut out).expect("tick");
        }
        let s = lrn.stats();
        assert_eq!(s.ticks, 5);
        assert_eq!(s.no_source_ticks, 5, "ausência contada por tick");
        assert_eq!(s.records_consumed, 0);
        assert_eq!(s.credits_awarded, 0);
        assert_eq!(s.pruned_traces, 0, "sem traços: nada podado");
    }

    #[test]
    fn dono_do_conteudo_e_declarado_por_prefixo() {
        assert_eq!(owner_module_of("predicao:cobertura"), "l3.prediction");
        assert_eq!(owner_module_of("sinal:coesao"), "l2.tissue");
        assert_eq!(owner_module_of("foco:conceito"), "l3.attention");
        assert_eq!(owner_module_of("outro:qualquer"), "l4.global");
    }

    #[test]
    fn a_a_determinismo_do_modulo_sem_fonte() {
        let run = || {
            let lrn = LearningModule::new();
            let mut clock = tf::LogicalClock::new();
            let mut log: Vec<u64> = Vec::new();
            for _ in 0..4 {
                clock.advance();
                let ctx = rt::TypedContext::new(clock);
                let mut out = Vec::new();
                lrn.tick(&ctx, &mut out).expect("tick");
                log.push(out.len() as u64);
            }
            log.push(lrn.stats().no_source_ticks);
            log
        };
        assert_eq!(run(), run(), "mesma sequência ⇒ mesmo learning (A/A)");
    }

    /// Ciclo REAL L1→L4 fechando decisões; o learning consome e
    /// credita por módulo COM denominador (Lei 1/5).
    #[test]
    fn consome_l4_real_e_credita_por_modulo_com_denominador() {
        use triad_l1_substrate as l1;
        use triad_l2_tissue as l2;
        use triad_l3_local as l3;

        let l1 = Arc::new(l1::ClusterModule::new(42, 24));
        let l2 = Arc::new(l2::TissueModule::new(l1.shared_runner(), 42));
        let l3 = Arc::new(l3::L3Module::new_with_substrate(Arc::clone(&l2)));
        let l4m = Arc::new(l4::L4Module::new_with_l3(Arc::clone(&l3)));
        let lrn = LearningModule::new_with_config(LearningCfg::default())
            .with_l4_source(Arc::clone(&l4m));
        let mut clock = tf::LogicalClock::new();
        for _ in 0..12 {
            clock.advance();
            let ctx = rt::TypedContext::new(clock);
            let mut out = Vec::new();
            l1.tick(&ctx, &mut out).expect("l1");
            l2.tick(&ctx, &mut out).expect("l2");
            l3.tick(&ctx, &mut out).expect("l3");
            l4m.tick(&ctx, &mut out).expect("l4");
            lrn.tick(&ctx, &mut out).expect("learning");
        }
        let s = lrn.stats();
        assert!(s.records_consumed > 0, "L4 fecha ciclos reais em 12 ticks");
        assert_eq!(s.no_source_ticks, 0, "fonte presente o tempo todo");
        let rates = lrn.module_rates();
        assert!(!rates.is_empty(), "ao menos um módulo creditado");
        let total: u64 = rates.iter().map(|r| r.2).sum();
        assert_eq!(
            total, s.records_consumed,
            "taxas por módulo somam ao consumo (denominador único)"
        );
        for (modulo, creditos, registros) in &rates {
            assert!(
                *creditos <= *registros,
                "{modulo}: creditos {creditos} > registros {registros}"
            );
        }
    }

    #[test]
    fn proposal_eta_de_lua_e_validada_e_aplicada_pelo_rust() {
        let lrn = LearningModule::new();
        // DENTRO da faixa da casa: aplicada, com razão e hash (Lei 3).
        let aplicado = lrn
            .apply_policy_eta(0.012, "ajuste da policy learning.lua", 0xabcd)
            .expect("aplicada");
        assert!((aplicado - 0.012).abs() < 1e-9);
        assert!((lrn.current_eta() - 0.012).abs() < 1e-9);
        assert_eq!(lrn.stats().eta_policy_applied, 1);
        // FORA da faixa: REJEITADA tipada — eta intacto, contada.
        let err = lrn
            .apply_policy_eta(0.03, "fora da faixa", 0xabcd)
            .expect_err("rejeitada");
        assert!(err.contains("fora da faixa"), "{err}");
        assert!((lrn.current_eta() - 0.012).abs() < 1e-9, "eta intacto");
        assert_eq!(lrn.stats().eta_policy_rejected, 1);
        assert_eq!(lrn.stats().eta_policy_applied, 1, "denominador honesto");
    }

    /// 17.8 INTEGRADA: janelas de validação t+1/t+5 com vereditos
    /// tipados + transfer como TAXA DE FLUXO por fronteira (legado
    /// layer_transfer.py) COM denominador — nunca migração de pesos.
    #[test]
    fn janelas_de_validacao_e_transfer_com_denominador() {
        use triad_l1_substrate as l1;
        use triad_l2_tissue as l2;
        use triad_l3_local as l3;
        let l1m = std::sync::Arc::new(l1::ClusterModule::new(42, 24));
        let l2m = std::sync::Arc::new(l2::TissueModule::new(l1m.shared_runner(), 42));
        let l3m = std::sync::Arc::new(l3::L3Module::new_with_substrate(std::sync::Arc::clone(&l2m)));
        let l4m = std::sync::Arc::new(l4::L4Module::new_with_l3(std::sync::Arc::clone(&l3m)));
        let lrn = LearningModule::new().with_l4_source(std::sync::Arc::clone(&l4m));
        use triad_runtime::CognitiveModule as _;
        let mut clock = tf::LogicalClock::new();
        for _ in 0..12 {
            clock.advance();
            let ctx = rt::TypedContext::new(clock);
            let mut out = Vec::new();
            l1m.tick(&ctx, &mut out).expect("l1");
            l2m.tick(&ctx, &mut out).expect("l2");
            l3m.tick(&ctx, &mut out).expect("l3");
            l4m.tick(&ctx, &mut out).expect("l4");
            let mut out_l = Vec::new();
            lrn.tick(&ctx, &mut out_l).expect("learning");
        }
        let s = lrn.stats();
        assert!(s.credits_awarded > 0, "cenário integração credita");
        // Vereditos: hits + incompletas + penalizadas == janelas
        // fechadas — DENOMINADOR único (nenhum veredito invisível).
        let fechadas = s.validation_hits + s.validation_incomplete + s.validation_misses;
        assert!(fechadas > 0, "janelas fecham em 12 ticks (h5=5)");
        // Transfer: quinteto coerente (accepted >= acted >= learned).
        let t = lrn.transfer_rates();
        assert_eq!(t.fronteira, "l4.global->learning");
        assert!(t.accepted >= t.acted);
        assert!(t.input >= t.accepted);
        assert!(t.acted >= t.learned);
        assert!((0.0..=1.0).contains(&t.f1), "taxa com denominador");
        assert!(!t.gargalo.is_empty(), "gargalo sempre identificado");
    }

    /// 20.3b (sessão 7, sob diretriz da dona): reforço nulo/negativo
    /// sobre traço AUSENTE é NO-OP — ausência NUNCA vira força 0.0
    /// (Lei 2: nada nasce do que nunca existiu); reforço positivo
    /// sobre ausente é NASCIMENTO tipado; A/A do snapshot.
    #[test]
    fn ausencia_nunca_vira_forca_zero_no_reforco_203b() {
        use crate::forgetting::ForgettingEngine;
        let mut f = ForgettingEngine::new(8);
        // reforço NEGATIVO sobre ausente: nenhum traço fantasma nasce.
        f.reinforce("modulo_fantasma", -0.3);
        assert_eq!(f.len(), 0, "ausência não fabrica traço 0.0 (Lei 2)");
        let q = f.strength(
            "modulo_fantasma",
            triad_foundation::ModuleId::new(),
            triad_foundation::StepId::new(),
        );
        assert!(q.value.is_none(), "ausência permanece NO_DATA");
        // reforço POSITIVO sobre ausente: nascimento com a força dada.
        f.reinforce("modulo_novo", 0.4);
        assert_eq!(f.export(), vec![("modulo_novo".to_string(), 0.4)]);
        // reforço sobre EXISTENTE soma sobre a força REAL medida.
        f.reinforce("modulo_novo", 0.3);
        assert_eq!(f.export(), vec![("modulo_novo".to_string(), 0.7)]);
        // A/A: mesma história => mesmo snapshot (determinístico).
        let gemea = || {
            let mut g = ForgettingEngine::new(8);
            g.reinforce("a", -0.5);
            g.reinforce("b", 0.4);
            g.reinforce("b", 0.3);
            g.export()
        };
        assert_eq!(gemea(), gemea(), "gêmeos bit-exatos (A/A)");
    }
}
