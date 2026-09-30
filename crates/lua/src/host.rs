//! PolicyHost (17.12) — fronteira Rust/Lua REAL: carrega as policies
//! da casa (init.lua + policies/ + governance/ + development/),
//! sandbox rígido (os/io/package/require/debug REMOVIDOS), executa a
//! policy com contexto e recebe Proposal; RUST VALIDA e aplica.
//! Lua NUNCA escreve estado — retorna Proposal ou nil (banda morta).
//! Hashes versionados por arquivo (FNV-1a do conteúdo).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mlua::{Function, Lua, LuaSerdeExt, Nil, Table, Value};
use tracing::{debug, warn};

use crate::config::LuaCfg;
use crate::proposal::{LuaProposal, PolicyReject, ValidatedProposal};

/// Erro de boot por arquivo — tipado; boot NUNCA cai (init.lua:
/// "o host nunca cai" — arquivo ruim é registrado e reportado).
#[derive(Debug, Clone, PartialEq)]
pub struct FileLoadIssue {
    pub file: String,
    pub message: String,
}

/// Contexto entregue à policy (tabela Lua `context`).
#[derive(Debug, Clone, Default)]
pub struct PolicyContext {
    pub mean_energy: Option<f64>,
    pub prediction_error: Option<f64>,
    pub active_fraction: Option<f64>,
}

/// FNV-1a 64 — hash versionado do CONTEÚDO do arquivo .lua.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Host de políticas Lua — fronteira da casa (rust_lua_boundary.md).
pub struct PolicyHost {
    lua: Lua,
    /// name -> FNV-1a do arquivo de origem (versionado).
    hashes: BTreeMap<String, u64>,
    /// Módulos utilitários (governance/development): name -> hash.
    module_hashes: BTreeMap<String, u64>,
    /// Arquivos que falharam no load (registrados, não fatais).
    load_issues: Vec<FileLoadIssue>,
    /// Erros de REGISTRO reportados por triad.errors do init.lua.
    registry_errors: Vec<String>,
}

/// Modo de varredura de pasta de arquivos .lua.
#[derive(Clone, Copy, PartialEq)]
enum Modo {
    /// lua/policies: registro triad.register_policy obrigatório.
    Policy,
    /// lua/governance, lua/development, lua/experiments: módulos
    /// utilitários — o retorno do chunk vira global (sem require).
    Modulo,
}

impl PolicyHost {
    /// Boot do host: sandbox + init.lua + todos os .lua das pastas
    /// declaradas na config. Boot falha APENAS se a infraestrutura
    /// mínima (init.lua) não existir — arquivo de policy ruim é
    /// registrado e o boot segue (frente honesta).
    pub fn boot(cfg: &LuaCfg, raiz: &Path) -> Result<Self, String> {
        let lua = Lua::new();
        if cfg.sandboxed {
            // Sandbox por projeto (init.lua): NENHUM os/io/require.
            let g = lua.globals();
            for key in ["os", "io", "debug", "package", "require"] {
                let _ = g.set(key, Nil);
            }
        }
        // init.lua primeiro (define triad.register_policy). As paths
        // da config são relativas à RAIZ do projeto (ex.: "lua/
        // policies"); o init vive em lua/init.lua.
        let init_path = raiz.join("lua").join("init.lua");
        if !init_path.is_file() {
            return Err(format!("init.lua ausente: {}", init_path.display()));
        }
        let init_src = std::fs::read_to_string(&init_path)
            .map_err(|e| format!("ler init.lua: {e}"))?;
        let _init_hash = fnv1a64(init_src.as_bytes());
        lua.load(&init_src)
            .set_name("init.lua")
            .exec()
            .map_err(|e| format!("executar init.lua: {e}"))?;

        let mut host = Self {
            lua,
            hashes: BTreeMap::new(),
            module_hashes: BTreeMap::new(),
            load_issues: Vec::new(),
            registry_errors: Vec::new(),
        };

        // Policies (lua/policies): registro OBRIGATÓRIO — triad.
        // register_policy. Governance/development/experiments:
        // MÓDULOS utilitários (retornam tabela) — o host injeta o
        // retorno como GLOBAL de mesmo nome (sem require; as
        // policies leem direto) e versiona o hash.
        let policy_dir = raiz.join(&cfg.policies_path);
        host.varrer(&policy_dir, Modo::Policy);
        for rel in [&cfg.governance_path, &cfg.development_path] {
            host.varrer(&raiz.join(rel), Modo::Modulo);
        }
        if cfg.experiments_enabled {
            host.varrer(&raiz.join(&cfg.experiments_path), Modo::Modulo);
        }

        // triad.errors: erros de REGISTRO reportados pelo próprio Lua.
        if let Ok(triad) = host.lua.globals().get::<_, Table>("triad") {
            if let Ok(errs) = triad.get::<_, Table>("errors") {
                for pair in errs.pairs::<Table, Value>() {
                    if let Ok((t, _)) = pair {
                        let policy = t.get::<_, String>("policy").unwrap_or_default();
                        let message = t.get::<_, String>("message").unwrap_or_default();
                        if !message.is_empty() {
                            host.registry_errors.push(format!("{policy}: {message}"));
                        }
                    }
                }
            }
        }
        Ok(host)
    }

    fn policy_raw(&self, name: &str) -> mlua::Result<Option<Function>> {
        let triad: Table = self.lua.globals().get::<_, _>("triad")?;
        let policies: Table = triad.get::<_, _>("policies")?;
        match policies.get::<_, Value>(name)? {
            Value::Nil => Ok(None),
            f => Ok(Some(self.lua.unpack(f)?)),
        }
    }

    /// Varre uma pasta de .lua conforme o modo; problemas são
    /// REGISTRADOS (tipados) — o boot NUNCA cai por policy ruim.
    fn varrer(&mut self, dir: &Path, modo: Modo) {
        let mut files: Vec<PathBuf> = match std::fs::read_dir(dir) {
            Ok(rd) => rd
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().map(|x| x == "lua").unwrap_or(false))
                .collect(),
            Err(_) => {
                debug!(pasta = %dir.display(), "pasta inexistente (ausência contada)");
                Vec::new()
            }
        };
        files.sort();
        for f in files {
            let nome = f
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("?")
                .to_string();
            let label = f.display().to_string();
            let src = match std::fs::read_to_string(&f) {
                Ok(s) => s,
                Err(e) => {
                    self.load_issues.push(FileLoadIssue {
                        file: label,
                        message: format!("ler: {e}"),
                    });
                    continue;
                }
            };
            let hash = fnv1a64(src.as_bytes());
            match modo {
                Modo::Policy => match self.lua.load(&src).set_name(&label).exec() {
                    Ok(()) => {
                        let registrada =
                            self.policy_raw(&nome).ok().flatten().is_some();
                        if registrada {
                            self.hashes.insert(nome.clone(), hash);
                            debug!(policy = %nome, hash, "policy registrada");
                        } else {
                            self.load_issues.push(FileLoadIssue {
                                file: label,
                                message: "executado sem triad.register_policy".into(),
                            });
                        }
                    }
                    Err(e) => self.load_issues.push(FileLoadIssue {
                        file: label,
                        message: format!("exec: {e}"),
                    }),
                },
                Modo::Modulo => match self.lua.load(&src).set_name(&label).eval::<Value>() {
                    Ok(v) => {
                        // Retorno vira global de mesmo nome: policies
                        // leem direto (sem require, sandbox mantido).
                        if let Err(e) = self.lua.globals().set(nome.as_str(), v) {
                            self.load_issues.push(FileLoadIssue {
                                file: label,
                                message: format!("global: {e}"),
                            });
                            continue;
                        }
                        self.module_hashes.insert(nome.clone(), hash);
                        debug!(modulo = %nome, hash, "módulo utilitário versionado");
                    }
                    Err(e) => self.load_issues.push(FileLoadIssue {
                        file: label,
                        message: format!("eval: {e}"),
                    }),
                },
            }
        }
    }

    /// Módulos utilitários carregados (governance/development),
    /// com hash versionado.
    pub fn modules(&self) -> Vec<(String, u64)> {
        self.module_hashes
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    /// Names registrados (ordenados) com hash versionado.
    pub fn policies(&self) -> Vec<(String, u64)> {
        self.hashes
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    /// Hash versionado de uma policy (FNV do conteúdo).
    pub fn policy_hash(&self, name: &str) -> Option<u64> {
        self.hashes.get(name).copied()
    }

    /// Arquivos com problema de load (tipado; boot não cai).
    pub fn load_issues(&self) -> &[FileLoadIssue] {
        &self.load_issues
    }

    /// Erros de registro reportados pelo init.lua.
    pub fn registry_errors(&self) -> &[String] {
        &self.registry_errors
    }

    /// EXECUTA a policy com contexto e VALIDA a Proposal em Rust.
    /// `Ok(None)` = banda morta (a policy propos de propósito nada).
    pub fn call(
        &mut self,
        name: &str,
        ctx: PolicyContext,
    ) -> Result<Option<ValidatedProposal>, PolicyReject> {
        let hash = match self.hashes.get(name).copied() {
            Some(h) => h,
            None => {
                // Ausência de policy: caminho TIPADO distinto de erro
                // de validação — representado como alvo desconhecido
                // com denominação de política ausente.
                return Err(PolicyReject::UnknownTarget {
                    module: format!("<policy:{name}>"),
                    parameter: "ausente".to_string(),
                });
            }
        };
        // Lookup separado do match; falhas voltam TIPADAS no reject.
        let lookup = self.policy_raw(name);
        let f = match lookup {
            Ok(Some(f)) => f,
            Ok(None) => {
                warn!(policy = %name, "registro ausente");
                return Err(PolicyReject::UnknownTarget {
                    module: format!("<policy:{name}>"),
                    parameter: "registro ausente".to_string(),
                });
            }
            Err(e) => {
                warn!(policy = %name, erro = %e, "lookup falhou");
                return Err(PolicyReject::UnknownTarget {
                    module: format!("<policy:{name}>"),
                    parameter: "lookup falhou".to_string(),
                });
            }
        };
        // Contexto como tabela Lua (o host ESCREVE, Lua só lê).
        let context = self
            .lua
            .create_table()
            .map_err(|_| PolicyReject::EmptyReason)?;
        if let Some(v) = ctx.mean_energy {
            let _ = context.set("mean_energy", v);
        }
        if let Some(v) = ctx.prediction_error {
            let _ = context.set("prediction_error", v);
        }
        if let Some(v) = ctx.active_fraction {
            let _ = context.set("active_fraction", v);
        }
        let raw: Value = match f.call(context) {
            Ok(v) => v,
            Err(e) => {
                warn!(policy = %name, erro = %e, "policy falhou na execução");
                return Err(PolicyReject::EmptyReason);
            }
        };
        match raw {
            // Banda morta: policy propos nada — válido e comum.
            Value::Nil => Ok(None),
            v => {
                // LuaSerdeExt (feature serialize): tabela → Proposal CRUA.
                let proposal: LuaProposal = self
                    .lua
                    .from_value(v)
                    .map_err(|_| PolicyReject::EmptyReason)?;
                proposal.validate(hash).map(Some)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn casa() -> PathBuf {
        // RAIZ do projeto: as paths da config já trazem o prefixo lua/.
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
    }

    fn host() -> PolicyHost {
        PolicyHost::boot(&LuaCfg::default(), &casa()).expect("boot da casa")
    }

    #[test]
    fn carrega_todas_as_policies_da_casa_com_hash_versionado() {
        let h = host();
        let names: Vec<String> = h.policies().into_iter().map(|(n, _)| n).collect();
        // lua/policies: registro OBRIGATÓRIO (as 7 da casa — a
        // 18.4 adicionou "federation" para o staging CNP; a 17.11
        // (LawEngine) adicionou "law_soft" para a soft law de
        // orçamento com faixa garantida pela whitelist).
        for esperada in [
            "learning",
            "attention",
            "energy",
            "crisis",
            "recovery",
            "federation",
            "law_soft",
        ] {
            assert!(
                names.contains(&esperada.to_string()),
                "policy {esperada} ausente; carregadas: {names:?}"
            );
        }
        assert_eq!(names.len(), 7, "exatamente as policies registráveis: {names:?}");
        // governance/development: MÓDULOS utilitários versionados.
        let mods = h.modules();
        for m in ["corticalization", "regeneration", "ecology", "federation"] {
            assert!(
                mods.iter().any(|(n, _)| n == m),
                "módulo {m} ausente: {mods:?}"
            );
        }
        // Hash versionado ESTÁVEL (A/A sobre o mesmo conteúdo).
        let h2 = host();
        assert_eq!(h.policy_hash("learning"), h2.policy_hash("learning"));
        assert!(h.load_issues().is_empty(), "nenhum arquivo da casa falha: {:?}", h.load_issues());
        assert!(h.registry_errors().is_empty(), "sem erros de registro: {:?}", h.registry_errors());
    }

    #[test]
    fn modulos_utilitarios_ficam_acessiveis_as_policies() {
        let h = host();
        // corticalization global injetada pelo host (sem require):
        // a policy pode chamar direto. Critério da casa: sustentável
        // em software => NÃO migra.
        let r: (bool, String) = h
            .lua
            .load("return corticalization.should_migrate(0.9, 0.9, 0.9)")
            .eval()
            .expect("global injetada");
        assert!(!r.0, "energia alta sustenta em software");
        assert_eq!(r.1, "EnergySustain");
        // Nicho determinístico do ecology (soma de bytes % 256).
        let niche: i64 = h
            .lua
            .load("return ecology.niche(\"l1.substrate\")")
            .eval()
            .expect("ecology global");
        assert!((0..256).contains(&niche));
    }

    #[test]
    fn sandbox_remove_os_io_package_require_debug() {
        let h = host();
        let g = h.lua.globals();
        for key in ["os", "io", "package", "require", "debug"] {
            let v: Value = g.get(key).expect("global presente");
            assert!(matches!(v, Value::Nil), "sandbox violado: {key} acessível");
        }
    }

    #[test]
    fn policy_learning_da_casa_propoe_eta_valido_ou_banda_morta() {
        let mut h = host();
        // Erro alto => eta alto, dentro da faixa da casa.
        let p = h
            .call("learning", PolicyContext { prediction_error: Some(0.5), ..Default::default() })
            .expect("call valida");
        let p = p.expect("erro alto propõe algo");
        assert_eq!(p.module, "learning");
        assert_eq!(p.parameter, "eta");
        assert!((0.005..=0.02).contains(&p.value), "eta {}", p.value);
        assert_eq!(p.ttl, 20);
        assert!(!p.reason.is_empty(), "Lei 3: razão sempre");
        assert!(p.policy_hash != 0, "procedência versionada");
        // Erro baixo => eta no piso, também válido.
        let p = h
            .call("learning", PolicyContext { prediction_error: Some(0.0), ..Default::default() })
            .expect("call valida")
            .expect("propõe");
        assert!((p.value - 0.005).abs() < 1e-9, "piso da faixa");
    }

    #[test]
    fn banda_morta_da_attention_retorna_none() {
        let mut h = host();
        // fraction 0.5 está DENTRO da banda (0.4..0.7): nada a propor.
        let r = h
            .call("attention", PolicyContext { active_fraction: Some(0.5), ..Default::default() })
            .expect("call valida");
        assert!(r.is_none(), "banda morta = nil");
    }

    #[test]
    fn policy_ausente_e_rejeicao_tipada() {
        let mut h = host();
        let r = h.call("inexistente", PolicyContext::default());
        assert!(matches!(r, Err(PolicyReject::UnknownTarget { .. })));
    }

    #[test]
    fn boot_sobrevive_a_arquivo_quebrado() {
        let dir = std::env::temp_dir().join(format!("luaboot_{}", std::process::id()));
        let _ = std::fs::create_dir_all(dir.join("lua/policies"));
        std::fs::write(dir.join("lua/init.lua"), include_str!("../../../lua/init.lua")).unwrap();
        std::fs::write(dir.join("lua/policies/ok.lua"),
            "triad.register_policy(\"ok\", function() return nil end)").unwrap();
        // Arquivo QUEBRADO (sintaxe): registrado, boot não cai.
        std::fs::write(dir.join("lua/policies/quebrada.lua"), "isto não é lua válido {{{").unwrap();
        let h = PolicyHost::boot(&LuaCfg::default(), &dir).expect("boot segue");
        assert_eq!(h.load_issues().len(), 1, "problema tipado registrado: {:?}", h.load_issues());
        assert!(h.policy_hash("ok").is_some(), "a policy boa carrega");
        let _ = std::fs::remove_dir_all(&dir);
    }}
