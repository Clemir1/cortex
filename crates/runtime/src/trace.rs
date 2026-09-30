//! TraceEngine (SEÇÃO 18, caixa 18.3 — CAMADA.txt:1058): trilha de
//! spans com linhagem (`trace_id`/`span_id`/`parent_event`) — "é daqui
//! que cascatas são reconstruídas".
//!
//! Herança do legado (relatório 1/3 dos subagentes):
//! - IDs derivados DETERMINISTICAMENTE de (seed, passo, tag, seq) —
//!   mesma família do rng por propósito da física da 17.6: a trilha é
//!   função do estado e da seed, NUNCA do wall-clock nem de uuid
//!   aleatório no hot path ⇒ A/A bit-exato entre runs.
//! - CASCADE_MAP DECLARATIVO (event_triggers.py:65-78): cascatas como
//!   DADOS, não código espalhado; proveniência origin/parent em cada
//!   salto.
//! - Fechamento por elo com EFEITO OBSERVÁVEL (neocortex_criterion.
//!   py:119): elo só fecha com produção mensurável; elo não executado
//!   é NO_DATA explícito — ausência ≠ zero, nunca zero.
//! - OBSERVACIONAL APENAS: spans nunca viram input causal (a trilha
//!   é auditoria, não estado).

use std::hash::{Hash, Hasher};
use triad_contracts::Qualified;

/// Raiz da trilha de um passo (um trace por passo do organismo).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TraceId(pub u64);

/// Um elo da trilha (um span por fase/evento com efeito).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SpanId(pub u64);

/// Span da trilha: linhagem + produção mensurável.
/// `efeito` = contagem observável da fase (devidos, mortes, movidos…).
/// `None` = elo NÃO executado neste passo (NO_DATA) — nunca zero.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TraceSpan {
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub parent: Option<SpanId>,
    pub step: u64,
    pub tag: &'static str,
    pub efeito: Option<u64>,
}

/// Tags de propósito da derivação (evita colisão com os tags 0xA5/0x5A
/// da física da 17.6 — família separada de ids).
const TAG_TRACE: u64 = 0x7ACE_0001;

impl TraceId {
    /// Raiz do passo: função determinística de (seed, passo).
    pub fn raiz(seed: u64, step: u64) -> Self {
        Self(derivar(seed, step, TAG_TRACE, b"raiz", 0))
    }
}

impl SpanId {
    /// Elo: função determinística de (seed, passo, tag, seq) — sem
    /// aleatoriedade, sem alocação no hot path.
    pub fn derivar(seed: u64, step: u64, tag: &str, seq: u64) -> Self {
        Self(derivar(seed, step, TAG_TRACE, tag.as_bytes(), seq))
    }
}

fn derivar(seed: u64, step: u64, proposito: u64, tag: &[u8], seq: u64) -> u64 {
    let mut h = std::hash::DefaultHasher::new();
    seed.hash(&mut h);
    step.hash(&mut h);
    proposito.hash(&mut h);
    tag.hash(&mut h);
    seq.hash(&mut h);
    h.finish()
}

/// Cascata DECLARATIVA (herança do event_triggers.py:65-78): evento de
/// origem (tag) → elos followup (camada, tag). O span do followup
/// recebe `parent` = span da origem — provenância propagada a cada
/// salto (reconstrução de cascatas é consulta, não arqueologia).
///
/// Entradas REAIS do organismo hoje (nada aspiracional): emergência
/// energética dispara a histerese coletiva do survival; mortes
/// disparam a compactação de população; divisões disparam o push na
/// matriz de estados.
pub const CASCADE_MAP: &[(&'static str, &[(&'static str, &'static str)])] = &[
    ("emergencia_energetica", &[("l1", "survival_histerese")]),
    ("mortes", &[("l1", "compactacao")]),
    ("divisoes", &[("l1", "matriz_push")]),
];

/// Followups declarados para uma tag de origem (vazio = sem cascata).
pub fn followups(origem: &str) -> &'static [(&'static str, &'static str)] {
    CASCADE_MAP
        .iter()
        .find(|(tag, _)| *tag == origem)
        .map(|(_, f)| *f)
        .unwrap_or(&[])
}

/// Reconstrói a cascata da trilha (CAMADA.txt:1064): devolve os saltos
/// presentes (origem → followup). Contrato de linhagem do legado
/// (chain_closure_check.py:7): todo span com `parent` deve apontar
/// para um span EXISTENTE da mesma trilha — span órfão é erro.
pub fn reconstruir(trilha: &[TraceSpan]) -> Result<Vec<(&'static str, &'static str)>, String> {
    let mut saltos = Vec::new();
    for span in trilha {
        if let Some(p) = span.parent {
            if !trilha.iter().any(|s| s.span_id == p) {
                return Err(format!(
                    "span órfão: tag {} (passo {}) aponta para pai ausente",
                    span.tag, span.step
                ));
            }
            // O span atual é um FOLLOWUP declarado cujo PAI é um span
            // da ORIGEM — provenância real, não coincidência de tag.
            if let Some((origem, _)) = CASCADE_MAP.iter().find(|(o, f)| {
                f.iter().any(|(_, ft)| *ft == span.tag)
                    && trilha.iter().any(|s| s.span_id == p && s.tag == *o)
            }) {
                saltos.push((*origem, span.tag));
            }
        }
    }
    Ok(saltos)
}

/// Fechamento do elo (padrão do legado, neocortex_criterion.py:119):
/// fecha SÓ com efeito observável > 0; zero produzido = elo aberto
/// (VALUE(false)); não executado = NO_DATA (ausência ≠ zero).
pub fn elo_fecha(span: &TraceSpan) -> Qualified<bool> {
    let (src, step) = (
        triad_foundation::id::ModuleId::new(),
        triad_foundation::id::StepId::new(),
    );
    match span.efeito {
        Some(e) => Qualified::value(e > 0, src, step),
        None => Qualified::no_data("elo não executado neste passo", src, step),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_sao_deterministicos_e_distintos() {
        let a = SpanId::derivar(42, 7, "fisica", 0);
        let b = SpanId::derivar(42, 7, "fisica", 0);
        assert_eq!(a, b, "mesma entrada ⇒ mesmo id (A/A)");
        assert_ne!(a, SpanId::derivar(42, 7, "energia", 0));
        assert_ne!(a, SpanId::derivar(43, 7, "fisica", 0));
        assert_ne!(a, SpanId::derivar(42, 8, "fisica", 0));
    }

    #[test]
    fn cascata_map_tem_proveniencia() {
        // Toda entrada do mapa declara followups não vazios.
        for (origem, f) in CASCADE_MAP {
            assert!(!f.is_empty(), "{origem} sem followups");
        }
        assert!(!followups("mortes").is_empty());
        assert!(followups("inexistente").is_empty());
    }

    #[test]
    fn reconstrucao_detecta_orfao_e_valida_salto() {
        let trace = TraceId::raiz(42, 1);
        let mortes = SpanId::derivar(42, 1, "mortes", 0);
        let comp = SpanId::derivar(42, 1, "compactacao", 0);
        let trilha = vec![
            TraceSpan { trace_id: trace, span_id: mortes, parent: None, step: 1, tag: "mortes", efeito: Some(3) },
            TraceSpan { trace_id: trace, span_id: comp, parent: Some(mortes), step: 1, tag: "compactacao", efeito: Some(3) },
        ];
        let saltos = reconstruir(&trilha).expect("linhagem íntegra");
        assert_eq!(saltos, vec![("mortes", "compactacao")]);
        // Órfão: parent aponta para span ausente ⇒ erro explícito.
        let orfa = vec![TraceSpan { trace_id: trace, span_id: comp, parent: Some(SpanId(999)), step: 1, tag: "compactacao", efeito: None }];
        assert!(reconstruir(&orfa).is_err());
    }

    #[test]
    fn elo_fecha_so_com_efeito_observavel() {
        let trace = TraceId::raiz(42, 1);
        let id = SpanId::derivar(42, 1, "fisica", 0);
        let fechado = TraceSpan { trace_id: trace, span_id: id, parent: None, step: 1, tag: "fisica", efeito: Some(5) };
        let aberto = TraceSpan { trace_id: trace, span_id: id, parent: None, step: 1, tag: "fisica", efeito: Some(0) };
        let ausente = TraceSpan { trace_id: trace, span_id: id, parent: None, step: 1, tag: "fisica", efeito: None };
        let q_f = elo_fecha(&fechado);
        assert!(q_f.is_value() && q_f.value == Some(true));
        let q_a = elo_fecha(&aberto);
        assert!(q_a.is_value() && q_a.value == Some(false));
        let nd = elo_fecha(&ausente);
        assert!(!nd.is_value() && nd.value.is_none(), "ausência ≠ zero");
    }
}
