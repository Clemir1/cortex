//! Leis da casa do Triad_AEE verificadas por integração.

use std::collections::HashSet;

use triad_contracts as tc;
use triad_foundation as tf;

/// E1: toda taxa nasce de uma divisão por denominador conhecido (0.5 = 1/2).
#[test]
fn rate_exige_denominador() {
    let rate = tf::Rate::construct(0.5f32);
    assert!(rate.is_some(), "0.5 = 1/2 tem denominador 2 conhecido");
    // Denominador zero é ausência, não taxa zero.
    assert!(tf::Rate::from_ratio(3, 0).is_none());
    assert_eq!(tf::Rate::from_ratio(1, 4).unwrap().value(), 0.25);
}

/// UUIDv7: cada ::new() gera um id distinto — 1000 ids sem colisão.
#[test]
fn ids_sao_unicos() {
    let mut set = HashSet::new();
    for _ in 0..1000 {
        set.insert(tf::ModuleId::new());
    }
    assert_eq!(set.len(), 1000);
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
}
