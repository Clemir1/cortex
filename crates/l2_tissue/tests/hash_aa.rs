//! 18.7 — A/A da instrumentação por camada (hash observacional):
//! mesma seed ⇒ MESMO state_hash (bit-exato, função pura do estado);
//! o tick muda o hash; camada sem observação ainda = `None`
//! (ausência ≠ zero — nunca um hash falso). A latência varia entre
//! runs, o hash NÃO (nunca há wall-clock dentro do hash).

use std::sync::{Arc, Mutex};
use triad_foundation as tf;
use triad_l1_substrate::ClusterModule;
use triad_l2_tissue::TissueModule;
use triad_runtime::{CognitiveModule as _, TypedContext};

fn avanca_e_tica(
    m: &dyn triad_runtime::CognitiveModule,
    clock: &mut tf::LogicalClock,
    out: &mut Vec<triad_contracts::EventEnvelope>,
) {
    clock.advance();
    let ctx = TypedContext::new(*clock);
    m.tick(&ctx, out).expect("tick da camada");
    out.clear();
}

/// L1: gêmeos com a mesma seed bitam o mesmo hash em CADA passo; o
/// hash responde ao tick (muda quando o estado muda).
#[test]
fn l1_hash_aa_bit_exato_e_responde_ao_estado() {
    let mut a = ClusterModule::new(42, 200);
    let mut b = ClusterModule::new(42, 200);
    let (mut ca, mut cb) = (tf::LogicalClock::new(), tf::LogicalClock::new());
    let (mut oa, mut ob) = (Vec::new(), Vec::new());
    let h0_a = a.state_hash();
    let h0_b = b.state_hash();
    // Gênese idêntica: mesmo hash (ou mesma ausência) nos dois.
    assert_eq!(h0_a, h0_b, "gêmeos na gênese: mesmo hash/ausência");
    avanca_e_tica(&a, &mut ca, &mut oa);
    avanca_e_tica(&b, &mut cb, &mut ob);
    let h1_a = a.state_hash();
    let h1_b = b.state_hash();
    assert_eq!(h1_a, h1_b, "A/A L1: mesma seed ⇒ mesmo hash pós-tick");
    assert_ne!(h0_a, h1_a, "o tick mudou o estado ⇒ o hash mudou");
    // Segundo passo: a igualdade A/A persiste (hash é função do estado).
    avanca_e_tica(&a, &mut ca, &mut oa);
    avanca_e_tica(&b, &mut cb, &mut ob);
    assert_eq!(a.state_hash(), b.state_hash(), "A/A L1 persiste no passo 2");
}

/// L2: antes do primeiro step a camada não tem observação ⇒ `None`
/// (ausência ≠ zero); após o step, gêmeos batem o hash demográfico.
#[test]
fn l2_hash_ausencia_ne_zero_e_aa_pos_step() {
    let l1a = Arc::new(Mutex::new(triad_l1_substrate::L1Runner::new(42, 200)));
    let l1b = Arc::new(Mutex::new(triad_l1_substrate::L1Runner::new(42, 200)));
    let mut a = TissueModule::new(Arc::clone(&l1a), 42);
    let mut b = TissueModule::new(Arc::clone(&l1b), 42);
    // Ausência ANTES da primeira observação — nunca um zero falso.
    assert!(a.state_hash().is_none(), "sem step L2 ainda ⇒ ausência (None)");
    assert!(b.state_hash().is_none());
    let (mut ca, mut cb) = (tf::LogicalClock::new(), tf::LogicalClock::new());
    let (mut oa, mut ob) = (Vec::new(), Vec::new());
    avanca_e_tica(&a, &mut ca, &mut oa);
    avanca_e_tica(&b, &mut cb, &mut ob);
    let (ha, hb) = (a.state_hash(), b.state_hash());
    // Pós-step: hash presente E igual entre gêmeos (bit-exato).
    assert!(ha.is_some(), "pós-step L2 ⇒ hash presente");
    assert_eq!(ha, hb, "A/A L2: mesma seed ⇒ mesmo hash demográfico");
}
