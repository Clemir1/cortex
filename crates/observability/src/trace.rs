//! TraceEngine (17.10): tracing distribuído em MEMÓRIA — cascatas
//! reconstruíveis com trace_id/span_id/parent_span e telemetria de
//! performance COM DENOMINADOR (µs por span, deltas entre janelas).
//!
//! COMPLEMENTA o SystemJournal (que persiste blocos/eventos em
//! var/system.log + system.json): o Journal é o DIÁRIO de validação;
//! o TraceEngine é a PROVENIÊNCIA CAUSAL da execução — quem originou
//! o quê, em que ordem, custando quanto. Nada é duplicado: spans são
//! registros estruturados (tick, módulo, pai, µs) coletados em
//! memória e renderizados sob demanda.
//!
//! Regras da casa: taxa SEM denominador não existe (µs médio vem
//! com n); ausência ≠ zero (span sem duração é `dur_us: None`,
//! impresso como `d=?`, nunca zero); rejeição tipada (pai
//! desconhecido é `UnknownParent`, nunca silêncio).

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::fnv_span;

/// Erro tipado do TraceEngine — nunca booleano mudo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceError {
    /// Span pai declarado mas inexistente no trace (procedência
    /// quebrada — rejeitada, não "enxertada").
    UnknownParent { span_id: u64 },
    /// Span consultado não existe neste trace.
    UnknownSpan { span_id: u64 },
}

/// Um span: unidade causal rastreada (tick, módulo/evento, pai, µs).
#[derive(Debug, Clone, PartialEq)]
pub struct SpanRecord {
    /// Identificador determinístico do span (FNV-1a do trace + seq).
    pub span_id: u64,
    /// Span pai (None = raiz do trace — o tick).
    pub parent: Option<u64>,
    /// Tick lógico do organismo.
    pub tick: u64,
    /// Nome canônico (módulo ou evento).
    pub name: String,
    /// Duração medida em µs — None quando não medida (ausência ≠ zero).
    pub dur_us: Option<u64>,
}

/// Motor de tracing de UM run: coleta spans e reconstrói cascatas.
pub struct TraceEngine {
    /// Identificador do trace (derivado da seed causal do run).
    pub trace_id: u64,
    spans: Vec<SpanRecord>,
    index: HashMap<u64, usize>,
    seq: u64,
}

impl TraceEngine {
    /// Novo trace para um run com a seed causal dada.
    pub fn new(seed: u64) -> Self {
        Self {
            trace_id: fnv_span(&format!("trace:{seed}").as_bytes()),
            spans: Vec::new(),
            index: HashMap::new(),
            seq: 0,
        }
    }

    fn push(
        &mut self,
        parent: Option<u64>,
        tick: u64,
        name: &str,
        dur_us: Option<u64>,
    ) -> u64 {
        let span_id = fnv_span(
            format!("{}:{}:{}:{}", self.trace_id, self.seq, tick, name)
                .as_bytes(),
        );
        self.seq += 1;
        self.spans.push(SpanRecord {
            span_id,
            parent,
            tick,
            name: name.to_string(),
            dur_us,
        });
        self.index.insert(span_id, self.spans.len() - 1);
        span_id
    }

    /// Span RAIZ do tick (o passo do organismo) — pai de todos os
    /// módulos que executaram nesse tick.
    pub fn root_span(&mut self, tick: u64) -> u64 {
        self.push(None, tick, &format!("tick:{tick}"), None)
    }

    /// Span de MÓDULO (filho do tick, com duração medida).
    /// Pai inexistente é rejeitado tipado.
    pub fn module_span(
        &mut self,
        parent: u64,
        tick: u64,
        module: &str,
        dur_us: u64,
    ) -> Result<u64, TraceError> {
        if !self.index.contains_key(&parent) {
            return Err(TraceError::UnknownParent { span_id: parent });
        }
        Ok(self.push(Some(parent), tick, module, Some(dur_us)))
    }

    /// Span de EVENTO (neto do tick, filho do módulo, sem duração).
    pub fn event_span(
        &mut self,
        parent: u64,
        tick: u64,
        event: &str,
    ) -> Result<u64, TraceError> {
        if !self.index.contains_key(&parent) {
            return Err(TraceError::UnknownParent { span_id: parent });
        }
        Ok(self.push(Some(parent), tick, event, None))
    }

    /// Todos os spans coletados (ordem de inserção).
    pub fn spans(&self) -> &[SpanRecord] {
        &self.spans
    }

    /// CASCATA de um span: ancestrais até a raiz (proveniência causal
    /// reconstruível — o caminho tick→módulo→evento).
    pub fn cascade_of(&self, span_id: u64) -> Result<Vec<SpanRecord>, TraceError> {
        let mut chain = Vec::new();
        let mut cur = span_id;
        loop {
            let Some(&ix) = self.index.get(&cur) else {
                return Err(TraceError::UnknownSpan { span_id: cur });
            };
            let rec = self.spans[ix].clone();
            let parent = rec.parent;
            chain.push(rec);
            match parent {
                Some(p) => cur = p,
                None => break,
            }
        }
        chain.reverse();
        Ok(chain)
    }

    /// Renderiza a cascata como árvore legível (indentação por
    /// nível; `d=?` quando a duração é ausente — nunca zero).
    pub fn render_cascade(&self, span_id: u64) -> Result<String, TraceError> {
        let chain = self.cascade_of(span_id)?;
        let mut out = String::new();
        for (lvl, rec) in chain.iter().enumerate() {
            let dur = match rec.dur_us {
                Some(us) => format!("{us}µs"),
                None => "d=?".to_string(),
            };
            let _ = writeln!(
                out,
                "{}{} [{}] (tick {}, {})",
                "  ".repeat(lvl),
                rec.name,
                rec.span_id,
                rec.tick,
                dur
            );
        }
        Ok(out)
    }

    /// Snapshot do Aggregator: por NOME, (n_spans, µs total) —
    /// denominadores prontos. SEÇÃO 20.3c: o denominador conta só
    /// spans FECHADOS (com duração); span aberto não entra na soma
    /// nem no denominador (ausência de duração ≠ 0µs).
    pub fn snapshot(&self) -> HashMap<String, (u64, u64)> {
        let mut m: HashMap<String, (u64, u64)> = HashMap::new();
        for s in &self.spans {
            if let Some(d) = s.dur_us {
                let e = m.entry(s.name.clone()).or_insert((0, 0));
                e.0 += 1;
                e.1 += d;
            }
        }
        m
    }
}

/// DeltaTelemetry/Aggregator (17.10): DIFERENÇA entre duas janelas
/// de medição — por nome: Δspans, Δµs e µs MÉDIO POR SPAN com
/// denominador explícito (Δµs/Δspans; Δspans=0 ⇒ média ausente,
/// nunca zero).
#[derive(Debug, Clone, PartialEq)]
pub struct DeltaRow {
    pub name: String,
    pub delta_spans: i64,
    pub delta_us: i64,
    /// Média por span na janela: None quando Δspans == 0.
    pub avg_us_per_span: Option<f64>,
}

/// Calcula o delta entre snapshots (após menos antes), ordenado por
/// Δµs descendente.
pub fn delta(
    before: &HashMap<String, (u64, u64)>,
    after: &HashMap<String, (u64, u64)>,
) -> Vec<DeltaRow> {
    let mut names: Vec<&String> = before.keys().chain(after.keys()).collect();
    names.sort();
    names.dedup();
    let mut rows: Vec<DeltaRow> = names
        .into_iter()
        .map(|n| {
            let (bn, bu) = before.get(n).copied().unwrap_or((0, 0));
            let (an, au) = after.get(n).copied().unwrap_or((0, 0));
            let dn = an as i64 - bn as i64;
            let du = au as i64 - bu as i64;
            DeltaRow {
                name: n.clone(),
                delta_spans: dn,
                delta_us: du,
                avg_us_per_span: if dn == 0 {
                    None
                } else {
                    Some(du as f64 / dn as f64)
                },
            }
        })
        .collect();
    rows.sort_by(|a, b| b.delta_us.cmp(&a.delta_us).then(a.name.cmp(&b.name)));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cascata_reconstrui_proveniencia_tick_modulo_evento() {
        let mut t = TraceEngine::new(42);
        let root = t.root_span(7);
        let l1 = t.module_span(root, 7, "l1.substrate", 120).expect("span");
        let ev = t.event_span(l1, 7, "l1.chladni").expect("span");
        let chain = t.cascade_of(ev).expect("cascata");
        let names: Vec<&str> = chain.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["tick:7", "l1.substrate", "l1.chladni"],
            "proveniência causal reconstruída até a raiz"
        );
        assert_eq!(chain[0].parent, None, "raiz sem pai");
        assert_eq!(chain[2].dur_us, None, "evento sem duração: ausência ≠ zero");
        let rendered = t.render_cascade(ev).expect("render");
        assert!(rendered.contains("l1.substrate ["));
        assert!(rendered.contains("d=?"), "ausência impressa como d=?");
        assert!(rendered.contains("120µs"));
    }

    #[test]
    fn pai_desconhecido_e_rejeitado_tipado() {
        let mut t = TraceEngine::new(1);
        let root = t.root_span(3);
        assert_eq!(
            t.module_span(999_999, 3, "x", 5),
            Err(TraceError::UnknownParent { span_id: 999_999 }),
            "procedência quebrada nunca é enxertada"
        );
        assert!(t.cascade_of(999_999).is_err());
        let _ = root;
    }

    #[test]
    fn delta_com_denominador_e_ausencia_explicita() {
        let mut before: HashMap<String, (u64, u64)> = HashMap::new();
        before.insert("l1.substrate".to_string(), (10, 1000));
        let mut after: HashMap<String, (u64, u64)> = HashMap::new();
        after.insert("l1.substrate".to_string(), (30, 2400));
        after.insert("learning".to_string(), (5, 50));
        let d = delta(&before, &after);
        let l1 = d.iter().find(|r| r.name == "l1.substrate").unwrap();
        assert_eq!(l1.delta_spans, 20);
        assert_eq!(l1.delta_us, 1400);
        assert!((l1.avg_us_per_span.unwrap() - 70.0).abs() < 1e-9);
        // Nome NOVO na janela depois: denominador nasce com o dado.
        let lrn = d.iter().find(|r| r.name == "learning").unwrap();
        assert_eq!(lrn.delta_spans, 5);
        // Δspans == 0 (contagem igual nas duas janelas): média AUSENTE.
        let mut b2: HashMap<String, (u64, u64)> = HashMap::new();
        b2.insert("x".to_string(), (3, 300));
        let mut a2: HashMap<String, (u64, u64)> = HashMap::new();
        a2.insert("x".to_string(), (3, 900));
        let d2 = delta(&b2, &a2);
        assert_eq!(d2[0].delta_spans, 0);
        assert_eq!(d2[0].delta_us, 600);
        assert!(
            d2[0].avg_us_per_span.is_none(),
            "Δspans=0 ⇒ média ausente, nunca zero"
        );
    }

    #[test]
    fn a_a_ids_deterministicos_pela_seed() {
        let run = || {
            let mut t = TraceEngine::new(42);
            let r = t.root_span(5);
            let m = t.module_span(r, 5, "l1.substrate", 7).expect("span");
            (t.trace_id, r, m, t.spans().len())
        };
        assert_eq!(run(), run(), "mesma seed ⇒ mesmos span_ids (A/A)");
        // Seeds diferentes ⇒ traces diferentes.
        let mut other = TraceEngine::new(43);
        let _ = other.root_span(5);
        assert_ne!(other.trace_id, TraceEngine::new(42).trace_id);
    }

    #[test]
    fn snapshot_agrega_com_denominador() {
        let mut t = TraceEngine::new(9);
        let r = t.root_span(1);
        t.module_span(r, 1, "l1.substrate", 100).expect("span");
        t.module_span(r, 1, "l1.substrate", 200).expect("span");
        let snap = t.snapshot();
        let (n, us) = snap.get("l1.substrate").copied().unwrap();
        assert_eq!((n, us), (2, 300), "n e µs viajam juntos");
    }
}
