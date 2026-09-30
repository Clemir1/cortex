//! TopDownFeedback L3→L2 com ADMISSÃO TIPADA (17.4): recebido ≠
//! admitido — TODO admitido carrega razão explícita e TODO recusado
//! carrega motivo tipado (o legado morria em 16 recebidos/0
//! admitidos sem reason). TissueRouter por CARÊNCIA (o tecido com
//! menor especialização recebe o reforço — homeostase mesoscópica
//! por POLÍTICA, nunca desliga tecido). O threshold aplicado usa o
//! canal AdaptationRequest (canal existente do L2) e o limite da
//! whitelist Lua [0.05, 0.95] é respeitado pelo Rust.

use std::collections::BTreeMap;

use triad_contracts as tc;
use triad_foundation as tf;

/// Limite inferior do threshold de especialização (whitelist Lua).
pub const SPEC_THRESHOLD_MIN: f32 = 0.05;
/// Limite superior do threshold de especialização (whitelist Lua).
pub const SPEC_THRESHOLD_MAX: f32 = 0.95;
/// Ganho máximo do realce admitido (intensidade × fator).
pub const SPEC_ENHANCE_GAIN: f32 = 0.10;
/// Janela de deduplicação: mesmo conceito só realça uma vez.
pub const DEDUPE_WINDOW: u64 = 5;

/// Motivo tipado de recusa — nunca recusa muda.
#[derive(Debug, Clone, PartialEq)]
pub enum RejectReason {
    /// Intensidade fora de [0,1] ou não finita.
    OutOfRange { intensity: f32 },
    /// Conceito já admitido dentro da janela de deduplicação.
    RateLimited { concept: tf::id::ConceptId, ticks_restantes: u64 },
    /// Nenhum tecido com membros vivos para receber o realce.
    NoLiveTissue { tecidos_vivos: usize },
    /// Todos os tecidos vivos já alcançaram o limiar homeostático —
    /// reforço extra seria excesso (o router só completa carências).
    AllAboveThreshold { n_vivos: usize, threshold: f32 },
}

/// Sinal admitido: alvo definido pelo router + novo threshold com
/// razão explícita.
#[derive(Debug, Clone, PartialEq)]
pub struct AdmittedSignal {
    pub concept: tf::id::ConceptId,
    pub tissue_id: tf::id::TissueId,
    pub new_threshold: f32,
    pub reason: String,
}

/// Admissão top-down com estatísticas COM DENOMINADOR. O limiar
/// homeostático (default 0.6) é a META: o router só reforça
/// tecidos vivos ABAIXO dele — o realce completa carência até o
/// limiar, nunca além (excesso é recusado tipado).
pub struct TopDownAdmission {
    tick: u64,
    threshold: f32,
    recebidos: u64,
    admitidos: u64,
    rejeitados: u64,
    por_motivo: BTreeMap<&'static str, u64>,
    ultimo_conceito: Option<(tf::id::ConceptId, u64)>,
    thresholds_aplicados: Vec<(tf::id::TissueId, f32)>,
    last_threshold_reason: Option<String>,
}

impl TopDownAdmission {
    /// Nova admissão (tick inicial 0; limiar 0.6).
    pub fn new() -> Self {
        Self {
            tick: 0,
            threshold: 0.6,
            recebidos: 0,
            admitidos: 0,
            rejeitados: 0,
            por_motivo: BTreeMap::new(),
            ultimo_conceito: None,
            thresholds_aplicados: Vec::new(),
            last_threshold_reason: None,
        }
    }

    /// Conta o tick da janela de deduplicação.
    pub fn tick(&mut self) {
        self.tick += 1;
    }

    /// Limiar homeostático corrente.
    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    /// Ajusta o limiar (canal Lua Proposal validado por Rust:
    /// clamp [0.05, 0.95] da whitelist — o PolicyHost garante a
    /// faixa antes; aqui é o guardião FINAL). Razão obrigatória.
    pub fn set_threshold(&mut self, value: f32, reason: &str) -> Result<f32, &'static str> {
        if !reason.trim().is_empty() && value.is_finite() {
            let v = value.clamp(SPEC_THRESHOLD_MIN, SPEC_THRESHOLD_MAX);
            self.threshold = v;
            self.last_threshold_reason = Some(reason.to_string());
            Ok(v)
        } else if reason.trim().is_empty() {
            Err("razão vazia")
        } else {
            Err("valor não finito")
        }
    }

    /// Última razão do ajuste de limiar (trilha).
    pub fn last_threshold_reason(&self) -> Option<&str> {
        self.last_threshold_reason.as_deref()
    }

    /// TissueRouter por carência: tecido VIVO com MENOR
    /// especialização ABAIXO do limiar recebe o reforço (argmin com
    /// desempate por id — determinístico).
    pub fn route_by_need(
        &self,
        states: &[tc::events::TissueState],
    ) -> Result<tf::id::TissueId, RejectReason> {
        let mut candidatos = states
            .iter()
            .filter(|t| !t.member_clusters.is_empty())
            .filter(|t| t.specialization.value() < self.threshold);
        match candidatos.next() {
            None => {
                let vivos = states
                    .iter()
                    .filter(|t| !t.member_clusters.is_empty())
                    .count();
                if vivos == 0 {
                    Err(RejectReason::NoLiveTissue { tecidos_vivos: 0 })
                } else {
                    Err(RejectReason::AllAboveThreshold {
                        n_vivos: vivos,
                        threshold: self.threshold,
                    })
                }
            }
            Some(best) => {
                let mut best = best;
                for t in candidatos {
                    if (t.specialization.value(), t.tissue_id)
                        < (best.specialization.value(), best.tissue_id)
                    {
                        best = t;
                    }
                }
                Ok(best.tissue_id)
            }
        }
    }

    /// Avalia um sinal contra os estados vivos dos tecidos.
    /// TODO veredito é contado com razão tipada.
    pub fn evaluate(
        &mut self,
        concept: tf::id::ConceptId,
        intensity: f32,
        states: &[tc::events::TissueState],
    ) -> Result<AdmittedSignal, RejectReason> {
        self.recebidos += 1;
        if !intensity.is_finite() || !(0.0..=1.0).contains(&intensity) {
            self.rejeitados += 1;
            *self.por_motivo.entry("OutOfRange").or_insert(0) += 1;
            return Err(RejectReason::OutOfRange { intensity });
        }
        if let Some((prev, tick_admitido)) = self.ultimo_conceito {
            if prev == concept && self.tick.saturating_sub(tick_admitido) < DEDUPE_WINDOW {
                self.rejeitados += 1;
                *self.por_motivo.entry("RateLimited").or_insert(0) += 1;
                let ticks_restantes = DEDUPE_WINDOW - self.tick.saturating_sub(tick_admitido);
                return Err(RejectReason::RateLimited {
                    concept,
                    ticks_restantes,
                });
            }
        }
        let alvo = self.route_by_need(states);
        let tissue_id = match alvo {
            Ok(id) => id,
            Err(motivo) => {
                self.rejeitados += 1;
                let chave = match &motivo {
                    RejectReason::NoLiveTissue { .. } => "NoLiveTissue",
                    RejectReason::AllAboveThreshold { .. } => "AllAboveThreshold",
                    RejectReason::OutOfRange { .. } => "OutOfRange",
                    RejectReason::RateLimited { .. } => "RateLimited",
                };
                *self.por_motivo.entry(chave).or_insert(0) += 1;
                return Err(motivo);
            }
        };
        let atual = states
            .iter()
            .find(|t| t.tissue_id == tissue_id)
            .map(|t| t.specialization.value())
            .unwrap_or(0.0);
        // Realce efetivo: completa a carência ATÉ o limiar — nunca além.
        let novo = (atual + intensity * SPEC_ENHANCE_GAIN).min(self.threshold);
        let reason = format!(
            "conceito {concept:?} → tecido {tissue_id:?} (carência argmin specialization {atual:.3} abaixo do limiar {:.3}; realce {:.3} até {:.3})",
            self.threshold,
            intensity * SPEC_ENHANCE_GAIN,
            novo
        );
        self.admitidos += 1;
        self.ultimo_conceito = Some((concept, self.tick));
        self.thresholds_aplicados.push((tissue_id, novo));
        Ok(AdmittedSignal {
            concept,
            tissue_id,
            new_threshold: novo,
            reason,
        })
    }

    /// (recebidos, admitidos, rejeitados, por_motivo) — taxas só
    /// com denominador `recebidos`.
    pub fn stats(&self) -> (u64, u64, u64, &BTreeMap<&'static str, u64>) {
        (self.recebidos, self.admitidos, self.rejeitados, &self.por_motivo)
    }

    /// Últimos thresholds aplicados (trilha auditável).
    pub fn last_thresholds(&self) -> &[(tf::id::TissueId, f32)] {
        &self.thresholds_aplicados
    }
}

impl Default for TopDownAdmission {
    fn default() -> Self {
        Self::new()
    }
}

/// Homeostasis mesoscópica: equilíbrio das especializações COM
/// DENOMINADOR (n de tecidos vivos). Nunca desliga tecido (Lei 4
/// análoga): o relatório mede, o ROUTER corrige por reforço.
#[derive(Debug, Clone, PartialEq)]
pub struct HomeostasisReport {
    pub n_tecidos_vivos: usize,
    pub desvio_padrao: f32,
    pub min_tissue: Option<tf::id::TissueId>,
    pub max_tissue: Option<tf::id::TissueId>,
}

/// Desvio padrão populacional das especializações dos vivos.
pub fn homeostasis(states: &[tc::events::TissueState]) -> HomeostasisReport {
    let vivos: Vec<&tc::events::TissueState> = states
        .iter()
        .filter(|t| !t.member_clusters.is_empty())
        .collect();
    if vivos.is_empty() {
        return HomeostasisReport {
            n_tecidos_vivos: 0,
            desvio_padrao: 0.0,
            min_tissue: None,
            max_tissue: None,
        };
    }
    let n = vivos.len() as f32;
    let mean = vivos.iter().map(|t| t.specialization.value()).sum::<f32>() / n;
    let var = vivos
        .iter()
        .map(|t| {
            let d = t.specialization.value() - mean;
            d * d
        })
        .sum::<f32>()
        / n;
    let mut min = vivos[0];
    let mut max = vivos[0];
    for t in &vivos {
        if t.specialization.value() < min.specialization.value() {
            min = t;
        }
        if t.specialization.value() > max.specialization.value() {
            max = t;
        }
    }
    HomeostasisReport {
        n_tecidos_vivos: vivos.len(),
        desvio_padrao: var.sqrt(),
        min_tissue: Some(min.tissue_id),
        max_tissue: Some(max.tissue_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tf::units::Confidence;

    fn tissue(id: tf::id::TissueId, spec: f32, membros: usize) -> tc::events::TissueState {
        let member_clusters = (0..membros)
            .map(|_| tf::id::ClusterId::new())
            .collect();
        tc::events::TissueState {
            tissue_id: id,
            cohesion: Confidence::construct(0.5).unwrap(),
            specialization: Confidence::construct(spec).unwrap(),
            integration: Confidence::construct(0.5).unwrap(),
            member_clusters,
        }
    }

    fn tid() -> tf::id::TissueId {
        tf::id::TissueId::new()
    }

    fn concept() -> tf::id::ConceptId {
        tf::id::ConceptId::new()
    }

    #[test]
    fn admissao_admitida_com_razao_e_router_por_carencia() {
        let mut adm = TopDownAdmission::new();
        let t1 = tid();
        let t2 = tid();
        let states = vec![tissue(t1, 0.8, 2), tissue(t2, 0.3, 2)];
        let c = concept();
        let r = adm.evaluate(c, 0.5, &states);
        let s = r.expect("admitido");
        assert_eq!(s.tissue_id, t2, "argmin specialization");
        assert!((s.new_threshold - 0.35).abs() < 1e-6, "realce 0.5×0.10");
        assert!(s.reason.contains("carência"), "razão explícita");
        let (rec, ok, rej, _) = adm.stats();
        assert_eq!((rec, ok, rej), (1, 1, 0));
    }

    #[test]
    fn rejeicoes_tipadas_com_denominador() {
        let mut adm = TopDownAdmission::new();
        let t1 = tid();
        let states = vec![tissue(t1, 0.5, 2)];
        // Fora de faixa.
        assert_eq!(
            adm.evaluate(concept(), 1.5, &states).unwrap_err(),
            RejectReason::OutOfRange { intensity: 1.5 }
        );
        // Sem tecido vivo.
        let mortos = vec![tissue(tid(), 0.5, 0)];
        assert_eq!(
            adm.evaluate(concept(), 0.5, &mortos).unwrap_err(),
            RejectReason::NoLiveTissue { tecidos_vivos: 0 }
        );
        // Admite e tenta deduplicar o MESMO conceito na janela.
        let c = concept();
        assert!(adm.evaluate(c, 0.5, &states).is_ok());
        assert_eq!(
            adm.evaluate(c, 0.5, &states).unwrap_err(),
            RejectReason::RateLimited { concept: c, ticks_restantes: 5 }
        );
        // Conceito DIFERENTE passa na mesma janela (dedupe é por conceito).
        assert!(adm.evaluate(concept(), 0.5, &states).is_ok());
        let (rec, ok, rej, por) = adm.stats();
        assert_eq!(rec, 5, "denominador");
        assert_eq!((ok, rej), (2, 3));
        assert_eq!(por.get("OutOfRange"), Some(&1));
        assert_eq!(por.get("NoLiveTissue"), Some(&1));
        assert_eq!(por.get("RateLimited"), Some(&1));
        // Janela expira após 5 ticks: readmite.
        for _ in 0..DEDUPE_WINDOW {
            adm.tick();
        }
        assert!(adm.evaluate(c, 0.5, &states).is_ok(), "janela expirou");
    }

    #[test]
    fn homeostasis_com_denominador_e_vazio_honrado() {
        let t1 = tid();
        let t2 = tid();
        let states = vec![tissue(t1, 0.2, 1), tissue(t2, 0.6, 1)];
        let h = homeostasis(&states);
        assert_eq!(h.n_tecidos_vivos, 2);
        assert!((h.desvio_padrao - 0.2).abs() < 1e-6);
        assert_eq!(h.min_tissue, Some(t1));
        assert_eq!(h.max_tissue, Some(t2));
        let vazio = homeostasis(&[]);
        assert_eq!(vazio.n_tecidos_vivos, 0, "ausência ≠ zero: denominador 0");
    }

    #[test]
    fn threshold_respeita_faixa_da_whitelist() {
        let mut adm = TopDownAdmission::new();
        adm.set_threshold(0.95, "teste de teto").expect("limiar sobe");
        let states = vec![tissue(tid(), 0.93, 3)];
        let s = adm
            .evaluate(concept(), 1.0, &states)
            .expect("realce máximo");
        assert_eq!(s.new_threshold, 0.95, "completa até o limiar, nunca além");
        // Guardião final: razão vazia é recusada.
        assert_eq!(adm.set_threshold(0.5, "  "), Err("razão vazia"));
        assert!(adm.set_threshold(f32::NAN, "x").is_err(), "não finito");
        // Todos acima do limiar: recusa tipada com denominador.
        let mut adm2 = TopDownAdmission::new();
        let satisfeitos = vec![tissue(tid(), 0.8, 1), tissue(tid(), 0.9, 1)];
        assert_eq!(
            adm2.evaluate(concept(), 0.5, &satisfeitos).unwrap_err(),
            RejectReason::AllAboveThreshold { n_vivos: 2, threshold: 0.6 }
        );
        let (_, ok, _, por) = adm2.stats();
        assert_eq!(ok, 0);
        assert_eq!(por.get("AllAboveThreshold"), Some(&1));
    }
}
