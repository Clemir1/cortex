//! T/persistence (17.9, absorve a 16.10): cross-run REAL — o estado
//! canônico relevante sobrevive entre execuções em SNAPSHOT serde
//! (JSON) sob `var/runs/`, escrita ATÔMICA (temp + rename), SEM
//! banco prematuro. RESTORE com VERIFICAÇÃO DE PROCEDÊNCIA:
//! schema versionado + checksum sobre os registros + REPLAY que
//! precisa reproduzir as taxas do learning (bit-idêntico). Snapshot
//! corrompido/trocado é rejeitado com erro TIPADO — nunca aceito
//! silencioso (Lei 2 aplicada a arquivo: ausência ≠ zero, corrupção
//! ≠ estado).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use triad_development as dev;
use triad_l4_global as l4;
use triad_learning as lrn;
use tracing::info;

/// Schema versionado do snapshot (mudança de layout = nova versão;
/// versões antigas são REJEITADAS com erro tipado, nunca "adaptadas").
pub const SNAPSHOT_SCHEMA: &str = "triad.persistence.v1";
/// Diretório canônico dos snapshots (var/runs/ — mesma área do
/// sistema de verificação).
pub const RUNS_DIR: &str = "var/runs";

/// Erro TIPADO de persistência — cada falha tem nome, nunca booleano mudo.
#[derive(Debug, Clone, PartialEq)]
pub enum PersistenceError {
    /// Arquivo ausente (ausência ≠ zero: sem snapshot é estado limpo
    /// informado, não erro de execução — o caller decide).
    Absent { path: String },
    /// Schema desconhecido/mais novo que o código (recusa explícita).
    SchemaUnknown { found: String, expected: String },
    /// Checksum não bate — snapshot corrompido ou adulterado.
    ChecksumMismatch { expected: u64, found: u64 },
    /// JSON ilegível (truncado, encoding quebrado).
    Corrupt { reason: String },
    /// Falha de I/O (diretório, permissão, disco).
    Io { path: String, reason: String },
    /// Replay de procedência divergiu das taxas do snapshot.
    ProvenanceDiverged {
        modulo: String,
        snapshot: (u64, u64),
        replayed: (u64, u64),
    },
}

/// Metadados de procedência do snapshot.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SnapshotMeta {
    /// Schema versionado (constante SNAPSHOT_SCHEMA).
    pub schema: String,
    /// Tick do organismo quando o snapshot foi tirado.
    pub created_tick: u64,
    /// Semente causal do run (reprodutibilidade declarada).
    pub seed: u64,
    /// Número de registros fechados incluídos.
    pub records: u64,
    /// Checksum FNV-1a sobre o closed_log serializado — detecta
    /// corrupção/adulteração (não é assinatura: é verificação de
    /// procedência interna).
    pub closed_log_checksum: u64,
}

/// SNAPSHOT completo do organismo (17.9): cicos fechados do L4,
/// estado do learning e estágio do development.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    pub meta: SnapshotMeta,
    /// Histórico de cicos fechados (cap 64 do L4 preservado).
    pub closed_log: Vec<l4::ClosedCycleRecord>,
    pub learning: lrn::LearningSnapshot,
    pub development: dev::StageSnapshot,
}

/// FNV-1a 64-bit sobre os bytes — determinístico e portável (sem
/// depender do hasher da stdlib, que pode variar).
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Checkpointer: salva e carrega snapshots com verificação de
/// procedência. Sem estado interno — as funções são puras sobre
/// caminhos explícitos.
pub struct Checkpointer;

impl Checkpointer {
    /// Monta o snapshot dos módulos vivos (tick + seed de
    /// procedência) — determinístico: mesma história ⇒ mesmo checksum.
    pub fn build(
        tick: u64,
        seed: u64,
        l4m: &l4::L4Module,
        learning: &lrn::LearningModule,
        development: &dev::DevelopmentModule,
    ) -> Snapshot {
        let closed_log = l4m.closed_log();
        let checksum = fnv1a64(
            &serde_json::to_vec(&closed_log).expect("closed_log serializável"),
        );
        Snapshot {
            meta: SnapshotMeta {
                schema: SNAPSHOT_SCHEMA.to_string(),
                created_tick: tick,
                seed,
                records: closed_log.len() as u64,
                closed_log_checksum: checksum,
            },
            closed_log,
            learning: learning.snapshot(),
            development: development.snapshot(),
        }
    }

    /// SALVA com escrita atômica: temp + rename (ou run antigo vê o
    /// arquivo completo, ou não vê nada — nunca um JSON truncado).
    pub fn save(dir: &Path, snap: &Snapshot) -> Result<PathBuf, PersistenceError> {
        fs::create_dir_all(dir)
            .map_err(|e| PersistenceError::Io {
                path: dir.display().to_string(),
                reason: e.to_string(),
            })?;
        let name = format!(
            "snapshot_tick{:06}_seed{}.json",
            snap.meta.created_tick, snap.meta.seed
        );
        let final_path = dir.join(&name);
        let tmp_path = dir.join(format!("{name}.tmp"));
        let bytes = serde_json::to_vec_pretty(snap).map_err(|e| PersistenceError::Corrupt {
            reason: format!("serialização: {e}"),
        })?;
        {
            let mut f = fs::File::create(&tmp_path).map_err(|e| PersistenceError::Io {
                path: tmp_path.display().to_string(),
                reason: e.to_string(),
            })?;
            f.write_all(&bytes).map_err(|e| PersistenceError::Io {
                path: tmp_path.display().to_string(),
                reason: e.to_string(),
            })?;
            f.sync_all().map_err(|e| PersistenceError::Io {
                path: tmp_path.display().to_string(),
                reason: e.to_string(),
            })?;
        }
        // rename atômico no mesmo volume.
        if final_path.exists() {
            fs::remove_file(&final_path).map_err(|e| PersistenceError::Io {
                path: final_path.display().to_string(),
                reason: e.to_string(),
            })?;
        }
        fs::rename(&tmp_path, &final_path).map_err(|e| PersistenceError::Io {
            path: tmp_path.display().to_string(),
            reason: e.to_string(),
        })?;
        info!(
            caminho = %final_path.display(),
            registros = snap.meta.records,
            checksum = snap.meta.closed_log_checksum,
            "snapshot salvo (atomico)"
        );
        Ok(final_path)
    }

    /// CARREGA e VERIFICA procedência: schema, checksum do
    /// closed_log. O REPLAY (verificação mais forte) é separado —
    /// `verify_provenance` — porque o load é passivo e o replay
    /// compara com o learning vivo.
    pub fn load(path: &Path) -> Result<Snapshot, PersistenceError> {
        if !path.exists() {
            return Err(PersistenceError::Absent {
                path: path.display().to_string(),
            });
        }
        let bytes = fs::read(path).map_err(|e| PersistenceError::Io {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        let snap: Snapshot =
            serde_json::from_slice(&bytes).map_err(|e| PersistenceError::Corrupt {
                reason: e.to_string(),
            })?;
        if snap.meta.schema != SNAPSHOT_SCHEMA {
            return Err(PersistenceError::SchemaUnknown {
                found: snap.meta.schema,
                expected: SNAPSHOT_SCHEMA.to_string(),
            });
        }
        let found = fnv1a64(
            &serde_json::to_vec(&snap.closed_log).expect("re-serialização"),
        );
        if found != snap.meta.closed_log_checksum {
            return Err(PersistenceError::ChecksumMismatch {
                expected: snap.meta.closed_log_checksum,
                found,
            });
        }
        Ok(snap)
    }

    /// Snapshot mais recente no diretório (por created_tick no nome) —
    /// `Ok(None)` = ausência LIMPA informada (não é erro).
    pub fn latest(dir: &Path) -> Result<Option<PathBuf>, PersistenceError> {
        if !dir.exists() {
            return Ok(None);
        }
        let mut best: Option<(u64, PathBuf)> = None;
        let entries =
            fs::read_dir(dir).map_err(|e| PersistenceError::Io {
                path: dir.display().to_string(),
                reason: e.to_string(),
            })?;
        for entry in entries.flatten() {
            let p = entry.path();
            let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !name.starts_with("snapshot_tick") || !name.ends_with(".json") {
                continue;
            }
            // tick{:06}_seedNN.json — parse do tick para ordenar.
            let tick: Option<u64> = name
                .trim_start_matches("snapshot_tick")
                .split('_')
                .next()
                .and_then(|s| s.parse().ok());
            if let Some(t) = tick {
                if best.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
                    best = Some((t, p));
                }
            }
        }
        Ok(best.map(|(_, p)| p))
    }
}

/// REPLAY de procedência (verificação forte): reprocessa o
/// closed_log do snapshot por um CreditAssigner-fiel (mesma regra de
/// dono por prefixo do learning) e exige que as TAXAS POR MÓDULO
/// batam com o snapshot do learning — bit-idêntico. Divergiu =
/// snapshot não procede do organismo declarado (rejeitado).
pub fn verify_provenance(
    snap: &Snapshot,
) -> Result<ProvenanceReport, PersistenceError> {
    let mut replayed: std::collections::HashMap<String, (u64, u64)> =
        std::collections::HashMap::new();
    for rec in &snap.closed_log {
        let owner = if rec.content_key.starts_with("predicao:") {
            "l3.prediction"
        } else if rec.content_key == "sinal:coesao" {
            "l2.tissue"
        } else if rec.content_key.starts_with("foco:") {
            "l3.attention"
        } else {
            "l4.global"
        };
        let e = replayed.entry(owner.to_string()).or_insert((0, 0));
        e.0 += 1;
        if rec.confirmed {
            e.1 += 1;
        }
    }
    for (modulo, credits, records) in &snap.learning.per_module {
        let got = replayed.get(modulo).copied().unwrap_or((0, 0));
        if got != (*credits, *records) {
            return Err(PersistenceError::ProvenanceDiverged {
                modulo: modulo.clone(),
                snapshot: (*credits, *records),
                replayed: got,
            });
        }
    }
    Ok(ProvenanceReport {
        records_replayed: snap.closed_log.len() as u64,
        modules_matched: snap.learning.per_module.len() as u64,
    })
}

/// Resultado do replay de procedência (verificação passou).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProvenanceReport {
    pub records_replayed: u64,
    pub modules_matched: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_detecta_corrupcao_do_closed_log() {
        // Adultera um registro DEPOIS do checksum: load tem que rejeitar.
        let mut bad = demo_snapshot();
        bad.closed_log[0].net_effect += 999.0;
        let bytes = serde_json::to_vec_pretty(&bad).expect("serde");
        let path = std::env::temp_dir().join("triad_persist_corrupt.json");
        fs::write(&path, bytes).expect("write");
        match Checkpointer::load(&path) {
            Err(PersistenceError::ChecksumMismatch { .. }) => {
                fs::remove_file(&path).ok();
            }
            other => panic!("corrupção aceita: {other:?}"),
        }
    }

    #[test]
    fn schema_desconhecido_e_rejeitado_tipado() {
        let mut bad = demo_snapshot();
        bad.meta.schema = "triad.persistence.v99".to_string();
        let bytes = serde_json::to_vec_pretty(&bad).expect("serde");
        let path = std::env::temp_dir().join("triad_persist_schema.json");
        fs::write(&path, bytes).expect("write");
        match Checkpointer::load(&path) {
            Err(PersistenceError::SchemaUnknown { found, .. }) => {
                assert_eq!(found, "triad.persistence.v99");
                fs::remove_file(&path).ok();
            }
            other => panic!("schema estranho aceito: {other:?}"),
        }
    }

    #[test]
    fn ausencia_e_limpa_nao_erro() {
        let dir = std::env::temp_dir().join("triad_persist_vazio_x");
        let _ = fs::remove_dir_all(&dir);
        match Checkpointer::latest(&dir) {
            Ok(None) => {}
            other => panic!("ausência deveria ser Ok(None): {other:?}"),
        }
        match Checkpointer::load(&dir.join("nao_existe.json")) {
            Err(PersistenceError::Absent { .. }) => {}
            other => panic!("arquivo ausente deveria ser Absent: {other:?}"),
        }
    }

    #[test]
    fn fnv_e_deterministico_e_sensivel() {
        let a = fnv1a64(b"triad");
        assert_eq!(a, fnv1a64(b"triad"), "A/A");
        assert_ne!(a, fnv1a64(b"triad!"), "sensível ao byte");
        assert_ne!(fnv1a64(b""), a);
    }

    /// Snapshot de demonstração coerente (checksum calculado por cima).
    fn demo_snapshot() -> Snapshot {
        let closed_log = vec![l4::ClosedCycleRecord {
            decided_tick: 10,
            content_key: "predicao:cobertura".to_string(),
            actual: 0.8,
            sham: 0.5,
            net_effect: 0.3,
            confirmed: true,
        }];
        let checksum =
            fnv1a64(&serde_json::to_vec(&closed_log).expect("serde"));
        Snapshot {
            meta: SnapshotMeta {
                schema: SNAPSHOT_SCHEMA.to_string(),
                created_tick: 10,
                seed: 42,
                records: closed_log.len() as u64,
                closed_log_checksum: checksum,
            },
            closed_log,
            learning: lrn::LearningSnapshot {
                traces: vec![("l3.prediction".to_string(), 0.1)],
                per_module: vec![("l3.prediction".to_string(), 1, 1)],
                records_consumed: 1,
                credits_awarded: 1,
                skipped_unconfirmed: 0,
                no_source_ticks: 0,
                eta: 0.005,
                janelas: Vec::new(),
                validation: (0, 0, 0),
            },
            development: dev::StageSnapshot {
                stage: dev::DevelopmentStage::Cleavage,
                ticks_in_band: 3,
                no_signal_ticks: 0,
                advances: 1,
            },
        }
    }
}
