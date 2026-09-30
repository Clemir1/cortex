//! Leis da casa do Triad_AEE verificadas por integração.
//!
//! Diretriz do dono: cada teste/validação registra no `var/system.log`
//! (journal legado, append) — a falha de I/O NUNCA reprova o teste.

use std::collections::HashSet;

use triad_contracts as tc;
use triad_foundation as tf;

/// Registra o resultado da lei no var/system.log (append, tolerante).
fn journal(nome: &str, detalhe: &str) {
    if let Ok(j) = triad_observability::SystemJournal::open_workspace() {
        let _ = j.event(
            "TESTE-HOUSE-LAW",
            &[
                ("lei", nome.to_string()),
                ("resultado", "ok".to_string()),
                ("detalhe", detalhe.to_string()),
            ],
        );
    }
}

/// E1: toda taxa nasce de uma divisão por denominador conhecido (0.5 = 1/2).
#[test]
fn rate_exige_denominador() {
    let rate = tf::Rate::construct(0.5f32);
    assert!(rate.is_some(), "0.5 = 1/2 tem denominador 2 conhecido");
    // Denominador zero é ausência, não taxa zero.
    assert!(tf::Rate::from_ratio(3, 0).is_none());
    assert_eq!(tf::Rate::from_ratio(1, 4).unwrap().value(), 0.25);
    journal("E1_taxas_com_denominador", "0.5=1/2; ratio(3,0)=ausencia");
}

/// UUIDv7: cada ::new() gera um id distinto — 1000 ids sem colisão.
#[test]
fn ids_sao_unicos() {
    let mut set = HashSet::new();
    for _ in 0..1000 {
        set.insert(tf::ModuleId::new());
    }
    assert_eq!(set.len(), 1000);
    journal("ids_uuidv7_unicos", "1000 ids sem colisao");
}

/// E2: ausência nunca vira zero — os estados não-VALUE existem por si.
#[test]
fn qualified_nao_e_default() {
    let (src, step) = (tf::ModuleId::new(), tf::StepId::new());
    let value = tc::Qualified::<f32>::value(1.0, src, step);
    assert!(value.is_value());
    let no_data = tc::Qualified::<f32>::no_data("sem amostra", src, step);
    assert!(!no_data.is_value() && no_data.value.is_none());
    let stale = tc::Qualified::<f32>::stale(src, step);
    assert!(!stale.is_value() && stale.value.is_none());
    let invalid = tc::Qualified::<f32>::invalid("fora do contrato", src, step);
    assert!(!invalid.is_value() && invalid.value.is_none());
    journal("E2_ausencia_ne_zero", "no_data/stale/invalid nunca sao VALUE");
}

/// O relógio lógico avança: `tick` é campo público e cresce monotonicamente.
#[test]
fn clock_avanca() {
    let mut clock = tf::LogicalClock::new();
    assert_eq!(clock.tick, 0);
    clock.advance();
    clock.advance();
    clock.advance();
    assert_eq!(clock.tick, 3);
    journal("relogio_logico", "tick monotônico ate 3");
}

/// Todas as variantes de ausência de TriadError instanciam com payload.
#[test]
fn triad_error_variants() {
    let _ = tf::TriadError::NoData { about: "sem".into() };
    let _ = tf::TriadError::Stale { about: "velho".into() };
    let _ = tf::TriadError::Invalid { about: "ruim".into(), reason: "quebra".into() };
    let _ = tf::TriadError::Fallback {
        about: "estrutural".into(),
        reason: "sem provider".into(),
        provider: tf::ModuleId::new(),
    };
    let _ = tf::TriadError::ContractViolation { detail: "lei".into() };
    journal("triad_error_variantes", "5 variantes com payload");
}
