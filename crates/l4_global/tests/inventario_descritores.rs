//! Inventário canônico de descritores (seção 17.1, doc/CAMADA.txt).
//!
//! Um único teste integra a CADEIA REAL montada no app (L1→L5 +
//! development) e exige o cartão canônico COMPLETO de cada módulo:
//! classificação por função primária, ordens O1–O5 como dimensão
//! própria, dono do estado declarado e evidência na escada E0–E5.
//! Ausência de declaração é FALHA — nunca default silencioso.

use triad_contracts as tc;
use triad_runtime as rt;

fn pendencias(d: &tc::ModuleDescriptor) -> String {
    let m = d.is_canonical();
    if m.is_empty() {
        "nenhuma".to_string()
    } else {
        m.join(",")
    }
}

#[test]
fn cadeia_real_declara_descritores_canonicos_completos() {
    // Cadeia real (mesma montagem do app): L1 → L2 (pelo runner
    // compartilhado) → L3 → L4 → L5; development é transversal.
    let l1 = triad_l1_substrate::ClusterModule::new(42, 24);
    let l2 = triad_l2_tissue::TissueModule::new(l1.shared_runner(), 42);
    let l3 = triad_l3_local::L3Module::new();
    let l4 = triad_l4_global::L4Module::new();
    let l5 = triad_l5_meta::L5Module::new_with_l4_and_config(
        None,
        None,
        triad_l5_meta::L5Config::default(),
    );
    let dev = triad_development::DevelopmentModule::new();

    // L1: substrato e dinâmica — O1 (controle da própria
    // dinâmica) + O2 (plasticidade adaptativa); proteção MÁXIMA.
    let d = rt::CognitiveModule::descriptor(&l1);
    assert_eq!(d.name, "l1.substrate");
    assert_eq!(d.layer, tc::Layer::L1);
    assert!(d.support_layer.is_none(), "L1 é cognitiva: suporte ausente é válido");
    assert_eq!(d.orders_label(), "O1+O2");
    assert_eq!(d.state_owner, "l1.substrate");
    assert!(d.outputs.contains(&"l1.substrate".to_string()));
    assert!(d.outputs.contains(&"l1.chladni".to_string()), "sinal servido por leitura read-only (ADR-0007)");
    assert_eq!(d.criticality, tc::Criticality::Critical);
    assert_eq!(d.evidence_requirement.as_str(), "E3");
    assert_eq!(d.execution_backend, tc::ExecutionBackend::CpuSeq);
    assert_eq!(pendencias(d), "nenhuma");

    // L2: organização mesoscópica — O2 (adaptação com recibos no
    // L2Ledger); drena as propostas L3 pelo inbox do DONO.
    let d = rt::CognitiveModule::descriptor(&l2);
    assert_eq!(d.name, "l2.tissue");
    assert_eq!(d.layer, tc::Layer::L2);
    assert_eq!(d.orders_label(), "O2");
    assert!(d.inputs.contains(&"inbox.l2.adaptation".to_string()));
    assert_eq!(d.criticality, tc::Criticality::High);
    assert_eq!(d.evidence_requirement.as_str(), "E3");
    assert_eq!(pendencias(d), "nenhuma");

    // L3: cognição local — O1 (predição local) + O4 (atenção);
    // atua no L2 apenas por SUBMISSÃO; memória real aplicada (E4).
    let d = rt::CognitiveModule::descriptor(&l3);
    assert_eq!(d.name, "l3.local");
    assert_eq!(d.layer, tc::Layer::L3);
    assert_eq!(d.orders_label(), "O1+O4");
    assert!(d.actuators.contains(&"inbox.l2.adaptation".to_string()));
    assert!(d.inputs.contains(&"l1.chladni".to_string()));
    assert_eq!(d.evidence_requirement.as_str(), "E4");
    assert_eq!(pendencias(d), "nenhuma");

    // L4: integração global — O1 (decisão) + O3 (broadcast
    // coordena); ciclo fecha com efeito futuro VERIFICADO (Lei 5 =
    // E5); reforça a memória L3 por submissão (trilha 16.2).
    let d = rt::CognitiveModule::descriptor(&l4);
    assert_eq!(d.name, "l4.global");
    assert_eq!(d.layer, tc::Layer::L4);
    assert_eq!(d.orders_label(), "O1+O3");
    assert!(d.actuators.contains(&"inbox.l3.reinforcement".to_string()));
    assert_eq!(d.criticality, tc::Criticality::Critical);
    assert_eq!(d.evidence_requirement.as_str(), "E5");
    assert_eq!(pendencias(d), "nenhuma");

    // L5: metacognição — O2+O3+O4; metacontrolador exige efeito
    // observado com baseline (E3, Lei 6); criticality normal (a
    // meta pode hibernar sem matar o núcleo — wake por causa).
    let d = rt::CognitiveModule::descriptor(&l5);
    assert_eq!(d.name, "l5.meta");
    assert_eq!(d.layer, tc::Layer::L5);
    assert_eq!(d.orders_label(), "O2+O3+O4");
    assert_eq!(d.state_owner, "l5.meta");
    assert_eq!(d.criticality, tc::Criticality::Normal);
    assert_eq!(d.evidence_requirement.as_str(), "E3");
    assert_eq!(pendencias(d), "nenhuma");

    // development: TRANSVERSAL — suporte obrigatório declarado;
    // O5 (seleção estrutural); 16.6: estágios por banda de
    // ressonância REAL com efeito nos orçamentos ⇒ E3 (produtivo).
    let d = rt::CognitiveModule::descriptor(&dev);
    assert_eq!(d.name, "development");
    assert_eq!(d.layer, tc::Layer::Transversal);
    assert_eq!(d.support_layer, Some(tc::SupportLayer::Development));
    assert_eq!(d.orders_label(), "O5");
    assert_eq!(d.evidence_requirement.as_str(), "E3");
    assert_eq!(pendencias(d), "nenhuma");
}

/// O inventário é REPRODUZÍVEL (A/A): a linha de resumo depende só
/// da classificação declarada, nunca do id aleatório do descritor.
#[test]
fn resumo_do_inventario_e_reproduzivel_aa() {
    let a = triad_l1_substrate::ClusterModule::new(7, 10);
    let b = triad_l1_substrate::ClusterModule::new(7, 10);
    let sa = rt::CognitiveModule::descriptor(&a).summary();
    let sb = rt::CognitiveModule::descriptor(&b).summary();
    assert_eq!(sa, sb, "mesma construção ⇒ mesma linha de inventário");
    assert!(sa.contains("l1.substrate | L1"));
    assert!(sa.contains("O1+O2"));
    assert!(sa.contains("CANONICO"));
}
