//! 18.3 — cadeia completa com cascatas RECONSTRUÍVEIS e A/A bit-exato.
//!
//! Herança validada do legado (relatório 1/3): linhagem íntegra
//! (E(t).parent == E(t-1) — chain_closure_check.py:7), fechamento de
//! elo só com efeito observável (neocortex_criterion.py:119) e tra-
//! bilha função do ESTADO (nunca do wall-clock) ⇒ A/A bit-exato.
//! Nota de regime: com `L1Config::default()` @800-1.200 não ocorrem
//! eventos vitais em 65 passos (medido: mortes=0, divisões=0) — por
//! isso o teste ponta-a-ponta valida o lado ESTRUTURAL honesto (elo
//! aberto = efeito 0 sem followup) e o salto POSITIVO é provado no
//! unit test do runtime com spans reais.

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

/// A trilha de CADA passo é íntegra (nenhum span órfão); a cascata
/// declarada aparece quando o evento ocorre e o elo fica ABERTO
/// (efeito 0, sem followup) quando não ocorre — honesto em qualquer
/// regime. O salto positivo é provado no unit test do runtime
/// (`reconstrucao_detecta_orfao_e_valida_salto`).
#[test]
fn cascata_reconstruida_em_cada_passo() {
    let mut r = L1Runner::new(42, 800);
    for _ in 0..40 {
        let rep = r.step();
        let saltos = reconstruir(&r.trace).expect("trilha íntegra (sem span órfão)");
        // Toda trilha tem a raiz do passo com efeito = população.
        assert!(r.trace.iter().any(|s| s.tag == "passo"));
        let mortes = r.trace.iter().find(|s| s.tag == "mortes").unwrap();
        if rep.deaths + rep.merges > 0 {
            assert_eq!(mortes.efeito, Some(rep.deaths as u64));
            assert!(
                saltos.contains(&("mortes", "compactacao")),
                "mortes ocorreram ⇒ salto declarado deve ser reconstruído"
            );
        } else {
            // Elo aberto: executou e nada produziu (efeito 0), e o
            // followup NÃO existe na trilha (não foi disparado).
            assert_eq!(mortes.efeito, Some(0));
            assert!(!saltos.contains(&("mortes", "compactacao")));
            assert!(!r.trace.iter().any(|s| s.tag == "compactacao"));
        }
        if rep.divisions > 0 {
            assert!(saltos.contains(&("divisoes", "matriz_push")));
        } else {
            assert!(!r.trace.iter().any(|s| s.tag == "matriz_push"));
        }
    }
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
