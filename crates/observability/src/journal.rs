//! Telemetria profunda do sistema (diretriz do dono): a cada teste e
//! validação o organismo registra a interface `var/system.log`
//! (linhas legíveis) e a telemetria estruturada `var/system.json`
//! (um objeto JSON por evento) — herdeiros do system.log/system.json
//! do legado.
//!
//! Escrita em APPEND com linhas curtas: múltiplos binários de teste
//! podem validar em paralelo sem corromper os arquivos. Ausência de
//! valor nunca vira zero: quem chama só registra o que mediu.

use serde_json::{Map, Value};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Diário do sistema: par .log (legível) + .json (estruturado).
pub struct SystemJournal {
    log: Mutex<File>,
    json: Mutex<File>,
    /// Metadados do RUN corrente (21.2: rotação por execução — presente
    /// apenas em journals abertos por `open_run`; harnesses de teste
    /// seguem em append simples, sem run).
    run: Option<RunMeta>,
}

/// Identidade do run para rotação e índice científico.
struct RunMeta {
    id: u64,
    started_unix: u64,
    /// Base do journal (o índice `var/runs_index.jsonl` é escrito
    /// AQUI — o run é dono do seu histórico, nunca de outra base).
    base: std::path::PathBuf,
}

fn unix_ts_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// run_id estável no run: timestamp × contador atômico global —
/// distinto entre runs consecutivos do mesmo segundo.
fn run_id_now() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    unix_ts_now().wrapping_mul(1000).wrapping_add(seq % 1000)
}

/// Separador de bloco (largura do system_02.log do legado).
const SEP_ABERTURA: &str =
    "======================================================================";
/// Separador do rodapé de conclusão (largura do legado).
const SEP_FECHAMENTO: &str =
    "============================================================";

impl SystemJournal {
    /// Abre (ou cria) `var/system.log` e `var/system.json` em append.
    /// Falha de I/O é propagada — nunca silenciosa.
    pub fn open(base: impl AsRef<Path>) -> std::io::Result<Self> {
        let dir = base.as_ref().join("var");
        std::fs::create_dir_all(&dir)?;
        let log = OpenOptions::new().create(true).append(true)
            .open(dir.join("system.log"))?;
        let json = OpenOptions::new().create(true).append(true)
            .open(dir.join("system.json"))?;
        Ok(Self {
            log: Mutex::new(log),
            json: Mutex::new(json),
            run: None,
        })
    }

    /// Conveniência: abre relativo ao diretório corrente (`var/`).
    pub fn open_default() -> std::io::Result<Self> {
        Self::open(".")
    }

    /// Abre no `var/` da RAIZ DO WORKSPACE — para harnesses de teste:
    /// o cargo executa cada binário de teste com cwd = diretório do
    /// CRATE; abrir relativo criaria `var/` duplicados dentro dos
    /// crates. A raiz é resolvida pelo manifest DO OBSERVABILITY
    /// (constante em compile time: `crates/observability/../../` =
    /// raiz), nunca pelo cwd do processo que chama.
    pub fn open_workspace() -> std::io::Result<Self> {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let root = Path::new(manifest)
            .parent()
            .and_then(Path::parent)
            .unwrap_or_else(|| Path::new(manifest));
        Self::open(root)
    }

    /// 21.2 (diretriz da dona — análise científica sem duplicações):
    /// abre o JOURNAL DO RUN com ROTAÇÃO. Antes de criar o novo
    /// `var/system.log`, o anterior é ARQUIVADO em
    /// `var/logs/system-<ts>-<runid>.log` (e o `.json` idem): uma
    /// execução = um arquivo; o histórico nunca se mistura e nada é
    /// perdido. O novo arquivo recebe o cabeçalho canônico
    /// `[RUN <id> INICIO ts=<unix>]`. Harnesses de teste continuam em
    /// append (open_workspace) — eles validam DURANTE o run corrente.
    pub fn open_run() -> std::io::Result<Self> {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let root = Path::new(manifest)
            .parent()
            .and_then(Path::parent)
            .unwrap_or_else(|| Path::new(manifest));
        Self::open_run_at(root)
    }

    /// `open_run` com base explícita (testável).
    pub fn open_run_at(base: impl AsRef<Path>) -> std::io::Result<Self> {
        let var = base.as_ref().join("var");
        std::fs::create_dir_all(&var)?;
        let id = run_id_now();
        let ts = unix_ts_now();
        // ROTAÇÃO: preserva a execução anterior (57 runs acumulados
        // era o sintoma da ausência disto).
        for name in ["system.log", "system.json"] {
            let cur = var.join(name);
            let cheio = std::fs::metadata(&cur).map(|m| m.len() > 0).unwrap_or(false);
            if cheio {
                let logs = var.join("logs");
                std::fs::create_dir_all(&logs)?;
                let ext = name.rsplit('.').next().unwrap_or("log");
                let arc = logs.join(format!("system-{ts}-{id}.{ext}"));
                std::fs::rename(&cur, arc)?;
            }
        }
        let base_pb = base.as_ref().to_path_buf();
        let mut j = Self::open(base)?;
        j.run = Some(RunMeta {
            id,
            started_unix: ts,
            base: base_pb,
        });
        {
            let mut f = j.log.lock().unwrap_or_else(|p| p.into_inner());
            writeln!(f, "[RUN {id} INICIO ts={ts}]")?;
        }
        j.json_event("run_inicio", &[("run_id", id.to_string()), ("ts", ts.to_string())])?;
        Ok(j)
    }

    /// Fecha o RUN cientificamente: rodapé `[RUN <id> FIM ts]` no
    /// log corrente + UMA linha no índice `var/runs_index.jsonl`
    /// (`{"run_id", "ts_inicio", "ts_fim", "resumo": <resumo_json>}`)
    /// — a análise científica compara runs ENTRE SI pelo índice, sem
    /// duplicação textual. `resumo_json` vem do chamador (métricas-
    /// chave já serializadas). No-op em journal sem run (harness).
    pub fn finish_run(&self, resumo_json: &str) -> std::io::Result<()> {
        let Some(r) = &self.run else {
            return Ok(());
        };
        let ended = unix_ts_now();
        {
            let mut f = self.log.lock().unwrap_or_else(|p| p.into_inner());
            writeln!(f, "[RUN {} FIM ts={}]", r.id, ended)?;
        }
        self.json_event(
            "run_fim",
            &[
                ("run_id", r.id.to_string()),
                ("ts", ended.to_string()),
                ("resumo", resumo_json.to_string()),
            ],
        )?;
        // Índice científico: 1 linha JSON por run (append) NA BASE do run.
        let idx = r.base.join("var").join("runs_index.jsonl");
        let mut f = OpenOptions::new().create(true).append(true).open(idx)?;
        writeln!(
            f,
            "{{\"run_id\":{},\"ts_inicio\":{},\"ts_fim\":{},\"resumo\":{}}}",
            r.id, r.started_unix, ended, resumo_json
        )
    }

    /// Bloco de abertura (formato do system_02.log do legado):
    /// linha de `=`, título central indentado, linha de `=`.
    pub fn section(&self, titulo: &str) -> std::io::Result<()> {
        let mut f = self.log.lock().unwrap_or_else(|p| p.into_inner());
        writeln!(f, "{SEP_ABERTURA}")?;
        writeln!(f, "  {titulo}")?;
        writeln!(f, "{SEP_ABERTURA}")?;
        self.json_event("section", &[("titulo", titulo.to_string())])
    }

    /// Linha de bloco indentada: `  chave: valor | chave: valor`
    /// (configuração do run, como no legado).
    pub fn section_line(&self, payload: &[(&str, String)]) -> std::io::Result<()> {
        let mut linha = String::from("  ");
        for (i, (k, v)) in payload.iter().enumerate() {
            if i > 0 {
                linha.push_str(" | ");
            }
            linha.push_str(&format!("{k}: {v}"));
        }
        let mut f = self.log.lock().unwrap_or_else(|p| p.into_inner());
        writeln!(f, "{linha}")?;
        drop(f);
        self.json_event("section_line", payload)
    }

    /// Evento de subsistema no FORMATO DO LEGADO: linha
    /// `[TAG] chave=valor | chave=valor` no .log (sem timestamp na
    /// linha — como o system_02.log) e objeto `{"ts", "kind", ...}`
    /// no .json (a telemetria profunda guarda o tempo).
    pub fn event(&self, kind: &str, payload: &[(&str, String)]) -> std::io::Result<()> {
        let mut legivel = format!("[{kind}]");
        for (k, v) in payload {
            legivel.push_str(&format!(" {k}={v}"));
        }
        legivel.push('\n');
        {
            let mut f = self.log.lock().unwrap_or_else(|p| p.into_inner());
            f.write_all(legivel.as_bytes())?;
        }
        self.json_event(kind, payload)
    }

    /// Rodapé de conclusão (formato do legado):
    /// `>>> EXECUÇÃO DO ORGANISMO CONCLUÍDA <<<` + linha de resumo.
    pub fn footer(&self, payload: &[(&str, String)]) -> std::io::Result<()> {
        let mut f = self.log.lock().unwrap_or_else(|p| p.into_inner());
        writeln!(f, "{SEP_FECHAMENTO}")?;
        writeln!(f, "  >>> EXECUÇÃO DO ORGANISMO CONCLUÍDA <<<")?;
        writeln!(f, "{SEP_FECHAMENTO}")?;
        drop(f);
        self.section_line(payload)?;
        self.json_event("run_concluida", payload)
    }

    /// Núcleo estruturado: um objeto JSON por evento com timestamp.
    fn json_event(&self, kind: &str, payload: &[(&str, String)]) -> std::io::Result<()> {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let mut obj = Map::new();
        obj.insert("ts".into(), Value::from(ts as u64));
        obj.insert("kind".into(), Value::from(kind));
        for (k, v) in payload {
            obj.insert((*k).into(), Value::from(v.as_str()));
        }
        let mut linha = serde_json::to_string(&Value::Object(obj))?;
        linha.push('\n');
        self.json
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .write_all(linha.as_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O journal escreve ambos os formatos com os mesmos campos
    /// (validado em diretório temporário, não em var/).
    #[test]
    fn evento_escreve_log_e_json_equivalentes() {
        let tmp = std::env::temp_dir().join(format!("triad-journal-{}", std::process::id()));
        let journal = SystemJournal::open(&tmp).expect("abrir journal");
        journal
            .event("validacao_teste", &[("resultado", "ok".into()), ("n", "5".into())])
            .expect("registrar evento");
        let log = std::fs::read_to_string(tmp.join("var/system.log")).expect("ler log");
        let json = std::fs::read_to_string(tmp.join("var/system.json")).expect("ler json");
        assert!(log.contains("validacao_teste"));
        assert!(log.contains("resultado=ok"));
        let obj: Value = serde_json::from_str(json.trim()).expect("json válido");
        assert_eq!(obj["kind"], "validacao_teste");
        assert_eq!(obj["resultado"], "ok");
        assert_eq!(obj["n"], "5");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// O formato segue o system_02.log do LEGADO: linha `[TAG] k=v`
    /// (sem timestamp na linha), blocos `======` e rodapé de
    /// conclusão — o timestamp vive apenas no system.json.
    #[test]
    fn formato_e_semelhante_ao_system_log_do_legado() {
        let tmp = std::env::temp_dir().join(format!("triad-journal-fmt-{}", std::process::id()));
        let journal = SystemJournal::open(&tmp).expect("abrir journal");
        journal.section("TRIAD_AEE -- EMBRIOGENESE COMPUTACIONAL COMPLETA").expect("section");
        journal
            .section_line(&[
                ("Clusters inicio", "30000".into()),
                ("Passos", "65".into()),
                ("Seed causal", "42".into()),
            ])
            .expect("section_line");
        journal
            .event("GPU-ORCH", &[("selected_backend", "CPU".into())])
            .expect("event");
        journal
            .footer(&[
                ("Passos", "65".into()),
                ("Clusters", "2972".into()),
                ("Tempo", "340.2s".into()),
            ])
            .expect("footer");
        let log = std::fs::read_to_string(tmp.join("var/system.log")).expect("ler log");
        assert!(log.contains("======================================================================"));
        assert!(log.contains("  TRIAD_AEE -- EMBRIOGENESE COMPUTACIONAL COMPLETA"));
        assert!(log.contains("  Clusters inicio: 30000 | Passos: 65 | Seed causal: 42"));
        assert!(log.contains("[GPU-ORCH] selected_backend=CPU"));
        assert!(log.contains("  >>> EXECUÇÃO DO ORGANISMO CONCLUÍDA <<<"));
        assert!(log.contains("  Passos: 65 | Clusters: 2972 | Tempo: 340.2s"));
        // Timestamp NÃO aparece na linha legível (como no legado)…
        assert!(!log.contains("ts="));
        // …mas vive no JSON estruturado (telemetria profunda).
        let json = std::fs::read_to_string(tmp.join("var/system.json")).expect("ler json");
        assert!(json.contains("\"ts\":"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// 21.2: ROTAÇÃO POR RUN — o system.log anterior é ARQUIVADO em
    /// var/logs/ (nada perdido, execuções nunca se misturam), o novo
    /// recebe o cabeçalho [RUN id INICIO] e o finish escreve UMA linha
    /// no índice científico runs_index.jsonl.
    #[test]
    fn rotaciona_run_anterior_e_indexa_cientificamente() {
        let tmp = std::env::temp_dir().join(format!("triad-journal-run-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("var")).expect("criar var");
        std::fs::write(tmp.join("var/system.log"), "RUN ANTIGO\n").expect("log velho");
        std::fs::write(tmp.join("var/system.json"), "{\"antigo\":true}\n").expect("json velho");

        let j = SystemJournal::open_run_at(&tmp).expect("abrir run");
        // anterior PRESERVADO em var/logs/
        let arq = std::fs::read_dir(tmp.join("var/logs")).expect("logs dir");
        let nomes: Vec<String> = arq
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(nomes.iter().any(|n| n.starts_with("system-") && n.ends_with(".log")), "{nomes:?}");
        let velho = std::fs::read_to_string(
            tmp.join("var/logs").join(nomes.iter().find(|n| n.ends_with(".log")).unwrap()),
        )
        .expect("arquivo arquivado");
        assert!(velho.contains("RUN ANTIGO"), "histórico preservado");
        // novo system.log = SÓ o run corrente
        let novo = std::fs::read_to_string(tmp.join("var/system.log")).expect("novo log");
        assert!(novo.contains("[RUN ") && novo.contains(" INICIO ts="), "{novo}");
        assert!(!novo.contains("RUN ANTIGO"), "sem mistura de execuções");

        j.finish_run("{\"passos\":65,\"veredito\":\"sem violar as leis da casa\"}")
            .expect("finish");
        let novo = std::fs::read_to_string(tmp.join("var/system.log")).expect("log pós-fim");
        assert!(novo.contains(" FIM ts="), "rodapé de fim do run");
        // índice científico: 1 linha com resumo
        let idx = std::fs::read_to_string(tmp.join("var/runs_index.jsonl")).expect("índice");
        assert!(idx.contains("\"run_id\":"), "{idx}");
        assert!(idx.contains("\"passos\":65"), "{idx}");
        assert!(idx.trim_end().lines().count() == 1, "exatamente 1 linha por run");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
