//! Configuração L2 — importada de `config/default.toml [l2.*]`.
//!
//! Diretriz do dono: configuração centralizada. As structs abaixo são
//! `Deserialize` e o app as extrai do TOML central via
//! `PlatformConfig::get_section::<L2Config>("l2")`. As consts são os
//! DEFAULTS documentados — herdados quando a seção não cita a chave.
//!
//! Valores mutáveis em runtime (threshold de afinidade, min/max members)
//! vivem em [`crate::formation::FormationParams`] e só mudam pelo gate
//! de adaptação com histerese ([`crate::adaptation`]) — nunca por
//! escrita direta de outra camada.

/// `[l2.tissues]` — estrutura dos tecidos.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct TissuesCfg {
    /// Tecidos na gênese (embryogenesis) — o L2 forma por demanda; a
    /// gênese usa este número como referência inicial.
    pub initial_count: usize,
    /// Teto estrutural de tecidos simultâneos.
    pub max_count: usize,
    /// Abaixo disso o tecido se dissolve (após a carência de gênese).
    pub min_members: usize,
    /// Capacidade por tecido.
    pub max_members: usize,
    /// Alvo de coesão (referência de qualidade; coesão é dado).
    pub coherence_target: f64,
    /// Folga antes de sinalizar falta de capacidade.
    pub capacity_margin: f64,
    /// 17.14: limiar de especialização do feedback top-down
    /// (L3→L2) — tecidos com especialização ABAIXO recebem o
    /// realce. Mutável em runtime SÓ pelo gate de adaptação
    /// (chave `l2.tissue.specialization_threshold`).
    pub specialization_threshold: f64,
}

impl Default for TissuesCfg {
    fn default() -> Self {
        Self {
            initial_count: 8,
            max_count: 32,
            min_members: 3,
            max_members: 64,
            coherence_target: 0.5,
            capacity_margin: 0.2,
            specialization_threshold: 0.6,
        }
    }
}

/// `[l2.affinity]` — membership, re-vinculação e bridges.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AffinityCfg {
    /// Afinidade mínima (1 − distância média/2) para membership.
    pub threshold: f64,
    /// Velocidade de re-vinculação (por step): 0.02 ⇒ um membro
    /// reavalia a cada 1/0.02 = 50 steps (histerese, nunca churn).
    pub update_rate: f64,
    /// Re-vincular só se o outro tecido for melhor por mais que isto
    /// (evita ping-pong entre tecidos irmãos).
    pub rebind_margin: f64,
    /// Carência de gênese: tecido novo só pode ser dissolvido após
    /// este número de steps.
    pub dissolve_grace_steps: u64,
    /// Pontes inter-tecido.
    pub bridge_enabled: bool,
    /// Teto de bridges por tecido.
    pub bridge_max_per_tissue: usize,
    /// Arestas inter-tecido mínimas (no grafo local do L1) para
    /// decretar uma bridge estrutural.
    pub bridge_min_edges: usize,
}

impl Default for AffinityCfg {
    fn default() -> Self {
        Self {
            threshold: 0.25,
            update_rate: 0.02,
            rebind_margin: 0.1,
            dissolve_grace_steps: 5,
            bridge_enabled: true,
            bridge_max_per_tissue: 4,
            bridge_min_edges: 2,
        }
    }
}

impl AffinityCfg {
    /// Intervalo de re-vinculação em steps (1/update_rate).
    pub fn rebind_interval_steps(&self) -> u64 {
        if self.update_rate <= 0.0 {
            return u64::MAX; // update_rate zero ⇒ nunca re-vincula
        }
        (1.0 / self.update_rate) as u64
    }
}

/// `[l2.adaptation]` — gate de feedback estrutural L3→L2.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AdaptationCfg {
    /// Feedback estrutural L3→L2 habilitado.
    pub feedback_enabled: bool,
    /// Micro-ajustes dentro da banda são adiados (insignificantes).
    pub hysteresis_band: f64,
    /// Cadência mínima entre pedidos aceitos POR PARÂMETRO.
    pub min_interval_steps: u64,
    /// Salto máximo aplicado por pedido; pedidos maiores são CLAMPADOS
    /// ao teto (o pedido anda na direção pedida, no máximo do teto).
    pub max_delta_per_request: f64,
}

impl Default for AdaptationCfg {
    fn default() -> Self {
        Self {
            feedback_enabled: true,
            hysteresis_band: 0.1,
            min_interval_steps: 20,
            max_delta_per_request: 0.1,
        }
    }
}

/// Seção `[l2]` completa — porta de entrada da configuração L2.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct L2Config {
    /// `[l2.tissues]`.
    pub tissues: TissuesCfg,
    /// `[l2.affinity]`.
    pub affinity: AffinityCfg,
    /// `[l2.adaptation]`.
    pub adaptation: AdaptationCfg,
}

impl L2Config {
    /// Histórico do ledger L2 (mecânica interna; não é política do
    /// organismo — fica como const central do crate).
    pub const LEDGER_HISTORY: usize = 32;
}

// ---------------------------------------------------------------------------
// Defaults congelados — as consts antigas permanecem como referência
// documentada (equivalência 1:1 com os `Default` acima).
// ---------------------------------------------------------------------------
/// Default congelado: teto de tecidos (histórico das consts).
pub const DEFAULT_MAX_TISSUES: usize = 32;
/// Default congelado: afinidade mínima (histórico das consts).
pub const DEFAULT_AFFINITY_THRESHOLD: f64 = 0.25;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_batem_com_os_valores_historicos() {
        let d = L2Config::default();
        assert_eq!(d.tissues.max_count, 32);
        assert_eq!(d.tissues.min_members, 3);
        assert_eq!(d.tissues.max_members, 64);
        assert!((d.affinity.threshold - 0.25).abs() < 1e-9);
        assert!((d.affinity.update_rate - 0.02).abs() < 1e-9);
        assert_eq!(d.affinity.rebind_interval_steps(), 50);
        assert!((d.affinity.rebind_margin - 0.1).abs() < 1e-9);
        assert_eq!(d.affinity.dissolve_grace_steps, 5);
        assert!(d.affinity.bridge_enabled);
        assert_eq!(d.affinity.bridge_max_per_tissue, 4);
        assert_eq!(d.affinity.bridge_min_edges, 2);
        assert!(d.adaptation.feedback_enabled);
        assert!((d.adaptation.hysteresis_band - 0.1).abs() < 1e-9);
        assert_eq!(d.adaptation.min_interval_steps, 20);
        assert!((d.adaptation.max_delta_per_request - 0.1).abs() < 1e-9);
        assert_eq!(L2Config::LEDGER_HISTORY, 32);
    }

    #[test]
    fn toml_da_seção_l2_carrega_pelo_deserialize() {
        // O formato exato do config/default.toml [l2.*].
        let text = r#"
            [tissues]
            initial_count = 8
            max_count = 32
            min_members = 3
            max_members = 64
            coherence_target = 0.5
            capacity_margin = 0.2

            [affinity]
            threshold = 0.25
            update_rate = 0.02
            rebind_margin = 0.1
            dissolve_grace_steps = 5
            bridge_enabled = true
            bridge_max_per_tissue = 4
            bridge_min_edges = 2

            [adaptation]
            feedback_enabled = true
            hysteresis_band = 0.1
            min_interval_steps = 20
            max_delta_per_request = 0.1
        "#;
        let cfg: L2Config = toml::from_str(text).expect("[l2] bem formado");
        assert_eq!(cfg.tissues.max_count, 32);
        assert_eq!(cfg.affinity.bridge_min_edges, 2);
        assert!((cfg.adaptation.max_delta_per_request - 0.1).abs() < 1e-9);
    }

    #[test]
    fn seção_parcial_herdou_o_resto() {
        // Só o que citou muda; o resto herda o default.
        let text = "[affinity]\nthreshold = 0.4\n";
        let cfg: L2Config = toml::from_str(text).expect("seção parcial");
        assert!((cfg.affinity.threshold - 0.4).abs() < 1e-9);
        assert_eq!(cfg.tissues.max_count, 32, "herdado");
        assert_eq!(cfg.affinity.bridge_min_edges, 2, "herdado");
    }
}
