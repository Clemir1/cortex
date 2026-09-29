//! Configuração centralizada do organismo — fonte única em `config/`.
//!
//! Diretriz do dono: TODAS as configs vivem em `config/*.toml` e os
//! módulos IMPORTAM de lá. Este loader é a porta única:
//!
//! - `config/default.toml` é a base; perfis (`crisis.toml`,
//!   `research.toml`, `corticalization.toml`) são DELTAS: chaves não
//!   citadas herdam o default (merge profundo de tabelas).
//! - Seção ausente ⇒ herda o default COM razão registrada em `notes`
//!   (nada silencioso).
//! - Arquivo base ausente ⇒ erro explícito no boot — fallback silencioso
//!   é violação da lei de evidência.
//! - As camadas não dependem da platform: cada crate expõe sua struct
//!   `Deserialize` e o app extrai a seção com `get_section::<T>()`.
//! - Separação documentada em `config/README.md`: o TOML carrega
//!   POLÍTICA do organismo; mecânica interna (dimensões, janelas tau,
//!   limiares de disparo) fica como consts nos crates.

use serde::de::DeserializeOwned;
use serde::Deserialize;
use triad_foundation as tf;

/// Caminho canônico da configuração base.
pub const DEFAULT_CONFIG_PATH: &str = "config/default.toml";

/// Resolve caminho da config: primeiro relativo ao cwd (produção, raiz do
/// workspace); senão relativo ao manifest (testes rodam em crates/).
/// Arquivo ausente nos dois continua sendo erro explícito no boot.
fn resolve(path: &str) -> String {
    if std::path::Path::new(path).exists() {
        return path.to_string();
    }
    format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR"))
}

/// Identidade e estado inicial do organismo (`[organism]`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct OrganismCfg {
    /// Nome canônico do organismo.
    pub name: String,
    /// Seed única do perfil base.
    pub seed: u64,
    /// Estado inicial da máquina de crise.
    pub initial_state: String,
    /// Modo operacional inicial.
    pub initial_mode: String,
}

impl Default for OrganismCfg {
    fn default() -> Self {
        Self {
            name: "cortex".into(),
            seed: 42,
            initial_state: "NORMAL".into(),
            initial_mode: "awake".into(),
        }
    }
}

/// Ciclo de execução e orçamento (`[runtime]`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RuntimeCfg {
    /// Duração nominal de um tick, em milissegundos.
    pub tick_ms: u64,
    /// Estratégia do scheduler.
    pub scheduler: String,
    /// Teto de eventos processados por tick.
    pub max_events_per_tick: usize,
    /// Orçamento de CPU por step, em milissegundos.
    pub step_budget_ms: u64,
    /// Janela de políticas Lua por step.
    pub lua_window_ms: u64,
    /// Recursões legítimas exigem terminação declarada.
    pub require_declared_termination: bool,
}

impl Default for RuntimeCfg {
    fn default() -> Self {
        Self {
            tick_ms: 100,
            scheduler: "priority_deadline".into(),
            max_events_per_tick: 2048,
            step_budget_ms: 80,
            lua_window_ms: 5,
            require_declared_termination: true,
        }
    }
}

/// Configuração da plataforma carregada de `config/`.
#[derive(Debug, Clone)]
pub struct PlatformConfig {
    /// `[organism]` — identidade e gênese.
    pub organism: OrganismCfg,
    /// `[runtime]` — ciclo, scheduler, orçamento.
    pub runtime: RuntimeCfg,
    /// TOML completo (as camadas extraem suas seções via `get_section`).
    raw: toml::Value,
    /// De onde esta configuração veio (arquivo ou defaults internos).
    source: String,
    /// Razões de herança (seções ausentes, deltas aplicados).
    notes: Vec<String>,
}

impl Default for PlatformConfig {
    /// Defaults internos documentados — usados quando não há arquivo.
    fn default() -> Self {
        Self {
            organism: OrganismCfg::default(),
            runtime: RuntimeCfg::default(),
            raw: toml::Value::Table(toml::map::Map::new()),
            source: "defaults internos (sem arquivo)".into(),
            notes: vec!["config sem arquivo: defaults internos".into()],
        }
    }
}

impl PlatformConfig {
    /// Carrega a base canônica `config/default.toml`. Arquivo ausente é
    /// erro explícito — o boot não inventa configuração.
    pub fn load_default() -> tf::TriadResult<Self> {
        Self::load(&resolve(DEFAULT_CONFIG_PATH))
    }

    /// Carrega base + perfil delta (`config/<name>.toml` herda o default).
    pub fn load_profile(name: &str) -> tf::TriadResult<Self> {
        let base = Self::load_default()?;
        let delta_path = resolve(&format!("config/{name}.toml"));
        let text = std::fs::read_to_string(&delta_path).map_err(|e| {
            tf::TriadError::Invalid {
                about: "config".into(),
                reason: format!("perfil '{delta_path}' ausente: {e}"),
            }
        })?;
        let delta: toml::Value = text.parse().map_err(|e| {
            tf::TriadError::Invalid {
                about: "config".into(),
                reason: format!("perfil '{delta_path}' inválido: {e}"),
            }
        })?;
        Ok(base.merged_with(&delta, &delta_path))
    }

    /// Carrega um arquivo TOML qualquer como configuração completa.
    pub fn load(path: &str) -> tf::TriadResult<Self> {
        let text = std::fs::read_to_string(path).map_err(|e| {
            tf::TriadError::Invalid {
                about: "config".into(),
                reason: format!("'{path}' ausente: {e}"),
            }
        })?;
        let raw: toml::Value = text.parse().map_err(|e| {
            tf::TriadError::Invalid {
                about: "config".into(),
                reason: format!("'{path}' não é TOML válido: {e}"),
            }
        })?;
        Ok(Self::from_toml(raw, path))
    }

    /// Constrói a config a partir do TOML, registrando heranças.
    pub fn from_toml(raw: toml::Value, source: &str) -> Self {
        let mut notes = Vec::new();
        let organism = match raw.get("organism") {
            Some(v) => OrganismCfg::deserialize(v.clone()).unwrap_or_else(|e| {
                notes.push(format!(
                    "[organism] ilegível ({e}) — herdando default com razão"
                ));
                OrganismCfg::default()
            }),
            None => {
                notes.push("[organism] ausente — herdando default com razão".into());
                OrganismCfg::default()
            }
        };
        let runtime = match raw.get("runtime") {
            Some(v) => RuntimeCfg::deserialize(v.clone()).unwrap_or_else(|e| {
                notes.push(format!(
                    "[runtime] ilegível ({e}) — herdando default com razão"
                ));
                RuntimeCfg::default()
            }),
            None => {
                notes.push("[runtime] ausente — herdando default com razão".into());
                RuntimeCfg::default()
            }
        };
        Self {
            organism,
            runtime,
            raw,
            source: source.into(),
            notes,
        }
    }

    /// Aplica um delta em cima desta base (merge profundo de tabelas;
    /// chaves não citadas herdam).
    pub fn merged_with(mut self, delta: &toml::Value, delta_source: &str) -> Self {
        merge_tables(&mut self.raw, delta);
        let mut merged = Self::from_toml(self.raw, &format!("{delta_source} sobre {DEFAULT_CONFIG_PATH}"));
        merged
            .notes
            .push(format!("perfil delta '{delta_source}' aplicado (herda o default)"));
        merged
    }

    /// Extrai uma seção tipada para uma camada (ex.: `get_section::<
    /// triad_l2_tissue::config::L2Config>("l2")`). Ausência é erro
    /// explícito — cada camada sabe o que exige.
    pub fn get_section<T: DeserializeOwned>(&self, path: &str) -> tf::TriadResult<T> {
        let mut current = &self.raw;
        for part in path.split('.') {
            current = current.get(part).ok_or_else(|| tf::TriadError::Invalid {
                about: "config".into(),
                reason: format!("seção '[{path}]' ausente em {}", self.source),
            })?;
        }
        T::deserialize(current.clone()).map_err(|e| tf::TriadError::Invalid {
            about: "config".into(),
            reason: format!("seção '[{path}]' ilegível: {e}"),
        })
    }

    /// De onde esta configuração veio.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Razões registradas (heranças, deltas) — auditoria da config.
    pub fn notes(&self) -> &[String] {
        &self.notes
    }
}

/// Merge profundo: tabelas mesclam recursivamente, folhas e arrays
/// substituem (semipreservação de política).
fn merge_tables(base: &mut toml::Value, delta: &toml::Value) {
    if let (toml::Value::Table(b), toml::Value::Table(d)) = (base, delta) {
        for (k, v) in d {
            match b.get_mut(k) {
                Some(toml::Value::Table(_)) if v.is_table() => {
                    merge_tables(b.get_mut(k).expect("chave presente"), v)
                }
                _ => {
                    b.insert(k.clone(), v.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, Default)]
    struct FakeSection {
        enabled: Option<bool>,
        threshold: Option<f64>,
    }

    #[test]
    fn base_canonica_carrega_com_valores_reais_do_toml() {
        // O teste roda da raiz do workspace: o arquivo é o contrato.
        let cfg = PlatformConfig::load_default().expect("config/default.toml existe");
        assert_eq!(cfg.organism.name, "cortex");
        assert_eq!(cfg.organism.seed, 42);
        assert_eq!(cfg.runtime.max_events_per_tick, 2048);
        // As seções das camadas vêm inteiras via get_section.
        let sec: FakeSection = cfg
            .get_section("l2.affinity")
            .expect("[l2.affinity] existe");
        assert_eq!(sec.threshold, Some(0.25), "threshold real do TOML");
        assert!(cfg.notes().is_empty(), "base completa não registra herança");
    }

    #[test]
    fn perfil_delta_herdando_o_que_nao_cita() {
        let cfg = PlatformConfig::load_profile("crisis").expect("config/crisis.toml existe");
        // O delta cita [crisis.*]; o resto herda do default.
        assert_eq!(cfg.organism.seed, 42, "herdado do default");
        assert!(cfg.source().contains("crisis"), "fonte registra o delta");
        let l2: FakeSection = cfg.get_section("l2.affinity").expect("herdado");
        assert_eq!(l2.threshold, Some(0.25));
    }

    #[test]
    fn seção_ausente_é_erro_explícito_nunca_silencioso() {
        let cfg = PlatformConfig::load_default().expect("base existe");
        let err = cfg.get_section::<FakeSection>("regiao.que.nao.existe");
        assert!(err.is_err(), "ausência nunca vira default fantasma");
    }

    #[test]
    fn arquivo_ausente_é_erro_de_boot() {
        let err = PlatformConfig::load("config/inexistente.toml");
        assert!(err.is_err(), "boot não inventa configuração");
    }

    #[test]
    fn seção_organismo_ausente_herdou_com_razão_registrada() {
        let raw = "[runtime]\nmax_events_per_tick = 7\n".parse().unwrap();
        let cfg = PlatformConfig::from_toml(raw, "teste");
        assert_eq!(cfg.organism.seed, 42, "default herdado");
        assert!(
            cfg.notes().iter().any(|n| n.contains("[organism] ausente")),
            "herança registrada com razão"
        );
    }
}
