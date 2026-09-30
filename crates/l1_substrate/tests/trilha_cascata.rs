//! 18.3 — cadeia completa com cascatas RECONSTRUÍVEIS e A/A bit-exato.
//!
//! Herança validada do legado (relatório 1/3): linhagem íntegra
//! (E(t).parent == E(t-1) — chain_closure_check.py:7), fechamento de
//! elo só com efeito observável (neocortex_criterion.py:119) e tra-
//! bilita função do ESTADO (nunca do wall-clock) ⇒ A/A bit-exato.

use triad_l1_substrate::L1Runner;
use triad_runtime::{elo_fecha, reconstruir};

fn snapshot_bits(r: &L1Runner) -> Vec<u64> {
    // Amostra determinística de bits de estado (primeiros clusters).
    r.clusters
        .iter()
        .take(16)
        .flat_map(|c| {
            let mut v: Vec<u64> = c.state.iter().map(|x| x.to_bits()).collect();
            v.push(c.energy.to_bits());
            v.push(c.tau_age.to_bits());
            v
        })
        .collect()
}

/// A trilha de CADA passo é íntegra (nenhum span órfão) e a cascata
/// declarada aparece quando o evento ocorre (prova da reconstrução).
#[test]
fn cascata_reconstruida_em_cada_passo() {
    let mut r = L1Runner::new(42, 800);
    let mut viu_salto_mortes = false;
    let mut viu_salto_divisao = false;
    for _ in 0..40 {
        let rep = r.step();
        let saltos = reconstruir(&r.trace).expect("trilha íntegra (sem span órfão)");
        // Toda trilha tem a raiz do passo com efeito = população.
        assert!(r.trace.iter().any(|s| s.tag == "passo"));
        if rep.deaths + rep.merges > 0 {
            assert!(
                saltos.contains(&("mortes", "compactacao")),
                "mortes ocorreram ⇒ salto declarado deve ser reconstruído"
            );
            viu_salto_mortes = true;
        }
        if rep.divisions > 0 {
            assert!(
                saltos.contains(&("divisoes", "matriz_push")),
                "divisões ocorreram ⇒ salto declarado deve ser reconstruído"
            );
            viu_salto_divisao = true;
        }
    }
    // @800 em 40 passos o organismo real divide E mata (regime maduro):
    // a cascata declarada É exercitada, não só declarada.
    assert!(viu_salto_mortes, "nenhuma morte em 40 passos @800?");
    assert!(viu_salto_divisao, "nenhuma divisão em 40 passos @800?");
}

/// A/A bit-exato: mesma seed ⇒ mesma trilha de spans E mesmo estado.
/// A trilha é função do estado/seed — nunca do wall-clock.
#[test]
fn aa_trilha_bit_exata_entre_runs() {
    let mut a = L1Runner::new(7, 600);
    let mut b = L1Runner::new(7, 600);
    for _ in 0..8 {
        let _ra = a.step();
        let _rb = b.step();
        assert_eq!(a.trace, b.trace, "A/A: trilhas devem ser bit-idênticas");
        assert_eq!(snapshot_bits(&a), snapshot_bits(&b), "A/A: estado idêntico");
    }
}

/// Ausência ≠ zero: elo não executado é NO_DATA (efeito None), nunca
/// efeito zero; elo executado com produção > 0 fecha.
#[test]
fn elo_ausente_nunca_e_zero() {
    let mut r = L1Runner::new(11, 600);
    for _ in 0..6 {
        let _ = r.step();
        let emerg = r
            .trace
            .iter()
            .find(|s| s.tag == "emergencia_energetica")
            .expect("span da emergência sempre presente (executada ou não)");
        let veredito = elo_fecha(emerg);
        match emerg.efeito {
            None => assert!(
                !veredito.is_value() && veredito.value.is_none(),
                "elo não executado ⇒ NO_DATA, nunca zero"
            ),
            Some(e) => assert!(
                veredito.is_value() && veredito.value == Some(e > 0),
                "elo executado fecha só com efeito > 0"
            ),
        }
        // Raiz do passo SEMPRE fecha com efeito (população > 0).
        let raiz = r.trace.iter().find(|s| s.tag == "passo").unwrap();
        assert!(elo_fecha(raiz).is_value());
    }
}
