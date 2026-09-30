//! Config de Lua injetada de `[lua]` + `[lua.sandbox]` — a fronteira
//! Rust/Lua (rust_lua_boundary.md): Rust é o dono do organismo; Lua
//! só AJUSTA POLÍTICAS via Proposal — NUNCA escreve estado, NUNCA
//! age. Sandbox rígido: os/io/package/require/debug REMOVIDOS das
//! globals do runtime.

use serde::{Deserialize, Serialize};

/// `[lua.sandbox]` — restrições INEGOCIÁVEIS (todas false/limites).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SandboxCfg {
    pub allow_io: bool,
    pub allow_os: bool,
    pub allow_network: bool,
    pub allow_filesystem: bool,
    pub allow_threads: bool,
    /// Janela de execução por script (ms) — declarada; policies da
    /// casa são determinísticas sem loops (o hook de interrupção
    /// aguarda mlua API adequada — chave nunca apagada).
    pub script_timeout_ms: u64,
    pub max_scripts_per_step: u64,
}

impl Default for SandboxCfg {
    fn default() -> Self {
        Self {
            allow_io: false,
            allow_os: false,
            allow_network: false,
            allow_filesystem: false,
            allow_threads: false,
            script_timeout_ms: 50,
            max_scripts_per_step: 16,
        }
    }
}

/// `[lua]` — política completa do host de políticas.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LuaCfg {
    pub enabled: bool,
    pub sandboxed: bool,
    pub policies_path: String,
    pub governance_path: String,
    pub development_path: String,
    pub experiments_path: String,
    pub experiments_enabled: bool,
    pub sandbox: SandboxCfg,
}

impl Default for LuaCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            sandboxed: true,
            policies_path: "lua/policies".to_string(),
            governance_path: "lua/governance".to_string(),
            development_path: "lua/development".to_string(),
            experiments_path: "lua/experiments".to_string(),
            experiments_enabled: true,
            sandbox: SandboxCfg::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_da_casa_parseia_lua_1_1() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../config/default.toml"
        ))
        .expect("default.toml presente");
        let raw: toml::Value = src.parse().expect("TOML válido");
        let t = raw.get("lua").expect("[lua]");
        assert_eq!(t.get("sandboxed"), Some(&toml::Value::Boolean(true)));
        assert_eq!(
            t.get("policies_path"),
            Some(&toml::Value::String("lua/policies".into()))
        );
        let sb = t.get("sandbox").expect("[lua.sandbox]");
        assert_eq!(sb.get("allow_io"), Some(&toml::Value::Boolean(false)));
        assert_eq!(sb.get("allow_os"), Some(&toml::Value::Boolean(false)));
        assert_eq!(sb.get("script_timeout_ms"), Some(&toml::Value::Integer(50)));
        let cfg: LuaCfg = t.clone().try_into().expect("parse 1:1");
        assert!(cfg.sandboxed);
        assert!(!cfg.sandbox.allow_network);
        assert_eq!(cfg.sandbox.max_scripts_per_step, 16);
    }

    #[test]
    fn defaults_congelados_sandbox_fechado() {
        let c = LuaCfg::default();
        assert!(!c.sandbox.allow_io);
        assert!(!c.sandbox.allow_os);
        assert!(!c.sandbox.allow_network);
        assert!(!c.sandbox.allow_filesystem);
        assert!(!c.sandbox.allow_threads);
    }
}
