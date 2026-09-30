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
}
