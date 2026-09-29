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

    /// Registra um evento de validação: linha legível no .log e objeto
    /// JSON no .json (mesma ordem de campos). `payload` são pares
    /// chave→valor (tudo string; quem tem número formata antes — o
    /// valor no JSON fica com a string exata registrada).
    pub fn event(&self, kind: &str, payload: &[(&str, String)]) -> std::io::Result<()> {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let mut obj = Map::new();
        obj.insert("ts".into(), Value::from(ts as u64));
        obj.insert("kind".into(), Value::from(kind));
        let mut legivel = format!("[{ts}] {kind}");
        for (k, v) in payload {
            obj.insert((*k).into(), Value::from(v.as_str()));
            legivel.push_str(&format!(" | {k}={v}"));
        }
        legivel.push('\n');
        self.log
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .write_all(legivel.as_bytes())?;
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
}
