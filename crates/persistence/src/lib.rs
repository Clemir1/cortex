//! T/persistence: cross-run real — snapshots serde com verificação de
//! procedência (17.9, absorve a 16.10). Sem banco prematuro.

pub mod checkpointer;

pub use checkpointer::{
    fnv1a64, verify_provenance, Checkpointer, PersistenceError, ProvenanceReport,
    RUNS_DIR, SNAPSHOT_SCHEMA, Snapshot, SnapshotMeta,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;
    use triad_development as dev;
    use triad_l1_substrate as l1;
    use triad_l2_tissue as l2;
    use triad_l3_local as l3;
    use triad_l4_global as l4;
    use triad_learning as lrn;
    use triad_runtime as rt;
    use triad_runtime::CognitiveModule as _;
    use triad_foundation as tf;

    /// Organismo real L1→L4 + learning + development; N ticks; devolve
    /// os handles para o teste montar o snapshot.
    fn organismo_com_historia(
        ticks: usize,
    ) -> (
        Arc<l4::L4Module>,
        Arc<lrn::LearningModule>,
        Arc<dev::DevelopmentModule>,
    ) {
        let l1 = Arc::new(l1::ClusterModule::new(42, 24));
        let l2m = Arc::new(l2::TissueModule::new(l1.shared_runner(), 42));
        let l3m = Arc::new(l3::L3Module::new_with_substrate(Arc::clone(&l2m)));
        let l4m = Arc::new(l4::L4Module::new_with_l3(Arc::clone(&l3m)));
        let lrnm = Arc::new(
            lrn::LearningModule::new_with_config(lrn::LearningCfg::default())
                .with_l4_source(Arc::clone(&l4m)),
        );
        let devm = Arc::new(
            dev::DevelopmentModule::new_with_config(dev::DevelopmentCfg::default())
                .with_chladni_source(Arc::clone(&l1)),
        );
        let mut clock = tf::LogicalClock::new();
        for _ in 0..ticks {
            clock.advance();
            let ctx = rt::TypedContext::new(clock);
            let mut out = Vec::new();
            l1.tick(&ctx, &mut out).expect("l1");
            l2m.tick(&ctx, &mut out).expect("l2");
            l3m.tick(&ctx, &mut out).expect("l3");
            l4m.tick(&ctx, &mut out).expect("l4");
            lrnm.tick(&ctx, &mut out).expect("learning");
            devm.tick(&ctx, &mut out).expect("development");
        }
        (l4m, lrnm, devm)
    }

    #[test]
    fn ciclo_completo_save_load_replay_restore() {
        let (l4m, lrnm, devm) = organismo_com_historia(12);
        let snap = Checkpointer::build(12, 42, &l4m, &lrnm, &devm);
        assert!(snap.meta.records > 0, "história real fecha cicos");
        assert_eq!(snap.meta.schema, SNAPSHOT_SCHEMA);

        // SAVE → LOAD: round-trip preserva bit a bit (checksum).
        let dir = std::env::temp_dir().join("triad_persist_ciclo");
        let _ = std::fs::remove_dir_all(&dir);
        let path = Checkpointer::save(&dir, &snap).expect("save");
        let loaded = Checkpointer::load(&path).expect("load");
        assert_eq!(
            loaded.meta.closed_log_checksum, snap.meta.closed_log_checksum,
            "round-trip bit-idêntico"
        );
        assert_eq!(loaded.closed_log.len(), snap.closed_log.len());

        // REPLAY de procedência: taxas do snapshot batem com o reprocesso.
        let report = verify_provenance(&loaded).expect("procedência");
        assert_eq!(report.records_replayed, loaded.meta.records);
        assert!(report.modules_matched >= 1);

        // LATEST encontra o snapshot salvo.
        let latest = Checkpointer::latest(&dir).expect("latest");
        assert_eq!(latest, Some(PathBuf::from(&path)));

        // RESTORE real: um learning NOVO retoma o estado do snapshot.
        let fresh = lrn::LearningModule::new();
        assert_eq!(fresh.stats().records_consumed, 0, "começa vazio");
        fresh.restore_snapshot(&loaded.learning);
        let s = fresh.stats();
        assert_eq!(s.records_consumed, loaded.learning.records_consumed);
        let rates = fresh.module_rates();
        assert!(!rates.is_empty(), "taxas restauradas");
        for (modulo, _credits, records) in &rates {
            let (_, _, rec_snap) = loaded
                .learning
                .per_module
                .iter()
                .find(|(m, _, _)| m == modulo)
                .expect("módulo presente no snapshot");
            assert_eq!(rec_snap, records, "registros restaurados exatos");
        }

        // DEVELOPMENT retoma o estágio alcançado.
        let devfresh = dev::DevelopmentModule::new();
        assert_eq!(
            devfresh.stats().stage,
            Some(dev::DevelopmentStage::Embryo),
            "começa no embrião"
        );
        devfresh.restore_snapshot(&loaded.development);
        assert_eq!(devfresh.stats().stage, Some(loaded.development.stage));
        assert_eq!(devfresh.stats().stage_advances, loaded.development.advances);
        let _ = std::fs::remove_dir_all(&dir);
        // 20.4d — veredito no var/system.log (helper tolerante: IO
        // nunca reprova o teste).
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("var/system.log")
            .and_then(|mut f| {
                use std::io::Write as _;
                writeln!(
                    f,
                    "TESTE-ciclo_completo_save_load_replay_restore: VERDE (round-trip bit-exato, replay {} registros, taxa e desenvolvimento restaurados)",
                    loaded.meta.records
                )
            });
    }

    #[test]
    fn a_a_snapshot_deterministico() {
        let (l4a, lrna, deva) = organismo_com_historia(8);
        let (l4b, lrnb, devb) = organismo_com_historia(8);
        let sa = Checkpointer::build(8, 42, &l4a, &lrna, &deva);
        let sb = Checkpointer::build(8, 42, &l4b, &lrnb, &devb);
        assert_eq!(
            sa.meta.closed_log_checksum, sb.meta.closed_log_checksum,
            "mesma história ⇒ mesmo checksum (A/A)"
        );
        assert_eq!(sa.development.stage, sb.development.stage);
        assert_eq!(sa.learning.per_module, sb.learning.per_module);
    }
}
