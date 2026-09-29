//! Gêmea bit-idêntica E5 (checklist 16.11) — o degrau máximo da escada
//! de evidência, como EXPERIMENTO OFFLINE (Lei 7: o contrafactual de
//! duplicação roda FORA do loop principal; custo 2× é política da
//! orquestradora, declarada no relatório 15-0 seção 5).
//!
//! Mecânica: a gêmea é uma SEGUNDA run do MESMO organismo (mesma seed,
//! mesmo clock) em que as decisões L4 viram DEFER pareado global —
//! `commit_threshold` acima do máximo de confiança (1.1) força o defer
//! nas MESMAS decisões (determinismo por seed). O veredito E5 compara,
//! POR DECISÃO, o valor do conteúdo prometido em t+1 na run ativa contra
//! a gêmea: `twin_effect = ativa(t+1) − gêmea(t+1)`. Gêmea válida =
//! divergência contrafatual dentro da tolerância (num organismo
//! determinístico sem propagação física da decisão, o esperado é 0 —
//! qualquer desvio maior invalida o contrafactual e REBAIXA o veredito
//! a E4 com a divergência documentada; ausência ≠ zero, nunca inflada).

use triad_foundation as tf;

/// Decisão commitada observada na run ATIVA (do `closed_log` do L4).
#[derive(Debug, Clone)]
pub struct TwinDecision {
    /// Tick em que a decisão foi commitada (t).
    pub decided_tick: u64,
    /// Conteúdo prometido (chave comensurável).
    pub content_key: String,
    /// Valor do conteúdo em t+1 na run ATIVA.
    pub actual_t_plus_1: f32,
    /// Sham local no tick do commit (persistência sem a ação).
    pub sham: f32,
}

/// Veredito da gêmea por decisão — E5 só quando o contrafactual é válido.
#[derive(Debug, Clone)]
pub struct TwinVerdict {
    pub decided_tick: u64,
    pub content_key: String,
    /// Efeito contrafatual GLOBAL: ativa(t+1) − gêmea(t+1). `None` se a
    /// gêmea não cobriu a decisão (ausência ≠ zero).
    pub twin_effect: Option<f32>,
    /// Efeito líquido contra o sham LOCAL (o critério E4 vigente).
    pub net_effect: f32,
    /// E5 quando a gêmea é válida; E4 quando inválida ou ausente.
    pub evidence: tf::evidence::EvidenceLevel,
    pub confirmed: bool,
}

/// Computa os vereditos da gêmea para cada decisão da run ativa.
///
/// `twin_t_plus_1`: pares (decided_tick, content_key, valor em t+1)
/// coletados da gêmea. `tolerance`: a MESMA tolerância do critério E4
/// do ciclo (Lei 5). Gêmea válida ⇒ `evidence = E5EffectValidated` e
/// confirmação exige `net_effect >= −tolerance` (idem E4); gêmea
/// divergente além da tolerância ⇒ rebaixa a `E4Consumed` com
/// `confirmed` falso e a divergência documentada (honestidade da
/// escada: E5 nunca é inflado).
pub fn twin_verdicts(
    decisions: &[TwinDecision],
    twin_t_plus_1: &[(u64, String, f32)],
    tolerance: f32,
) -> Vec<TwinVerdict> {
    decisions
        .iter()
        .map(|d| {
            let net_effect = d.actual_t_plus_1 - d.sham;
            let twin_value = twin_t_plus_1
                .iter()
                .find(|(t, k, _)| *t == d.decided_tick && k == &d.content_key)
                .map(|(_, _, v)| *v);
            match twin_value {
                Some(tv) => {
                    let twin_effect = d.actual_t_plus_1 - tv;
                    // Contrafactual válido: a diferença entre as runs é
                    // apenas o efeito atribuído à decisão (esperado ~0
                    // aqui — nada além da decisão propaga).
                    let gemea_valida = twin_effect.abs() <= tolerance.max(1e-6);
                    let confirmed = gemea_valida && net_effect >= -tolerance;
                    TwinVerdict {
                        decided_tick: d.decided_tick,
                        content_key: d.content_key.clone(),
                        twin_effect: Some(twin_effect),
                        net_effect,
                        evidence: if gemea_valida {
                            tf::evidence::EvidenceLevel::E5EffectValidated
                        } else {
                            tf::evidence::EvidenceLevel::E4Consumed
                        },
                        confirmed,
                    }
                }
                None => TwinVerdict {
                    decided_tick: d.decided_tick,
                    content_key: d.content_key.clone(),
                    twin_effect: None,
                    net_effect,
                    evidence: tf::evidence::EvidenceLevel::E4Consumed,
                    confirmed: net_effect >= -tolerance,
                },
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tf::evidence::EvidenceLevel;

    fn decision(tick: u64, key: &str, actual: f32, sham: f32) -> TwinDecision {
        TwinDecision {
            decided_tick: tick,
            content_key: key.to_string(),
            actual_t_plus_1: actual,
            sham,
        }
    }

    #[test]
    fn gemea_valida_confirma_e5() {
        // Ativa: 0.9 em t+1; gêmea idêntica (0.9) ⇒ twin_effect 0.
        let ds = [decision(3, "sinal:coesao", 0.9, 0.8)];
        let tv = [(3u64, "sinal:coesao".to_string(), 0.9f32)];
        let v = twin_verdicts(&ds, &tv, 0.1);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].evidence, EvidenceLevel::E5EffectValidated);
        assert!(v[0].confirmed, "efeito líquido +0.1 dentro da tolerância");
        assert!((v[0].twin_effect.unwrap()).abs() < 1e-9);
    }

    #[test]
    fn gemea_divergente_rebaixa_para_e4() {
        // Gêmea divergiu 0.5 (>> tolerância 0.1): contrafactual inválido.
        let ds = [decision(3, "sinal:coesao", 0.9, 0.8)];
        let tv = [(3u64, "sinal:coesao".to_string(), 0.4f32)];
        let v = twin_verdicts(&ds, &tv, 0.1);
        assert_eq!(v[0].evidence, EvidenceLevel::E4Consumed);
        assert!(!v[0].confirmed, "gêmea inválida nunca confirma E5");
        assert!((v[0].twin_effect.unwrap() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn gemea_ausente_fica_e4_com_none() {
        let ds = [decision(3, "sinal:coesao", 0.9, 0.8)];
        let v = twin_verdicts(&ds, &[], 0.1);
        assert_eq!(v[0].evidence, EvidenceLevel::E4Consumed);
        assert!(v[0].twin_effect.is_none(), "ausência ≠ zero");
        assert!(v[0].confirmed, "critério E4 segue válido sozinho");
    }

    #[test]
    fn decisao_degradada_nao_confirma_mesmo_com_gemea_valida() {
        let ds = [decision(3, "sinal:coesao", 0.5, 0.9)]; // caiu vs sham
        let tv = [(3u64, "sinal:coesao".to_string(), 0.5f32)];
        let v = twin_verdicts(&ds, &tv, 0.1);
        assert_eq!(v[0].evidence, EvidenceLevel::E5EffectValidated);
        assert!(!v[0].confirmed, "efeito líquido −0.4 além da tolerância");
    }
}
