//! Governador límbico: regula stress, reserva de energia e aversão.
//!
//! REAL (seção 16.1): entradas derivadas do ciclo L4/L1 reais
//! (não-confirmação como pressão; energia média dos clusters);
//! banda com HISTERESE (entra acima de `stress_enter`, sai abaixo de
//! `stress_exit`) e razão tipada por transição — crise muda POLÍTICA,
//! nunca suspende (Lei 4).

use crate::config::LimbicCfg;
use triad_contracts as tc;
use triad_foundation as tf;

/// Banda límbica com histerese (estado como dado, nunca booleano solto).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimbicBand {
    /// Repouso regulado.
    Stable,
    /// Pressão sustentada: modulação ativa, política ajustada.
    Alert,
}

/// Razão tipada da última transição de banda (auditoria).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BandReason {
    /// Entrou em alerta: stress cruzou `stress_enter`.
    StressEntered,
    /// Aliviou: stress caiu abaixo de `stress_exit` (histerese).
    StressRelieved,
    /// Sem transição neste update.
    Unchanged,
}

impl BandReason {
    /// Nome canônico da razão.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StressEntered => "STRESS_ENTERED",
            Self::StressRelieved => "STRESS_RELIEVED",
            Self::Unchanged => "UNCHANGED",
        }
    }
}

/// Estado límbico regulado do organismo.
pub struct LimbicGovernor {
    /// Nível de stress corrente (0.0..1.0).
    pub stress: f32,
    /// Reserva de energia disponível (0.0..1.0).
    pub energy_reserve: f32,
    /// Sinal de aversão corrente (0.0..1.0).
    pub aversion: f32,
    /// Banda corrente (histerese).
    band: LimbicBand,
    /// Última razão de transição.
    reason: BandReason,
    /// Política de histerese/reserva de `[l5.limbic]`.
    cfg: LimbicCfg,
}

impl LimbicGovernor {
    /// Novo governador em repouso saudável (política default).
    pub fn new() -> Self {
        Self::with_config(LimbicCfg::default())
    }

    /// Governador com a política injetada de `[l5.limbic]`.
    pub fn with_config(cfg: LimbicCfg) -> Self {
        Self {
            stress: 0.1,
            energy_reserve: 1.0,
            aversion: 0.1,
            band: LimbicBand::Stable,
            reason: BandReason::Unchanged,
            cfg,
        }
    }

    /// Absorve sinais externos com clamp 0.0..1.0 em cada eixo e
    /// aplica a HISTERESE da banda com razão tipada.
    pub fn update(&mut self, external_stress: f32, external_energy: f32, aversion_signal: f32) -> BandReason {
        self.stress = external_stress.clamp(0.0, 1.0);
        self.energy_reserve = external_energy.clamp(0.0, 1.0);
        self.aversion = aversion_signal.clamp(0.0, 1.0);
        self.reason = match self.band {
            LimbicBand::Stable if self.stress >= self.cfg.stress_enter => {
                self.band = LimbicBand::Alert;
                BandReason::StressEntered
            }
            LimbicBand::Alert if self.stress <= self.cfg.stress_exit => {
                self.band = LimbicBand::Stable;
                BandReason::StressRelieved
            }
            _ => BandReason::Unchanged,
        };
        self.reason
    }

    /// Banda corrente (histerese).
    pub fn band(&self) -> LimbicBand {
        self.band
    }

    /// Última razão de transição de banda.
    pub fn reason(&self) -> BandReason {
        self.reason
    }

    /// Projeta o estado interno como modulação límbica.
    pub fn modulation(&self) -> tc::LimbicModulation {
        // construct(0.0) é sempre Some (0.0 dentro da faixa): fallback estrutural.
        tc::LimbicModulation {
            stress: tf::Stress::construct(self.stress.clamp(0.0, 1.0))
                .unwrap_or_else(|| tf::Stress::construct(0.0).unwrap()),
            energy: tf::Energy::construct(self.energy_reserve.clamp(0.0, 1.0))
                .unwrap_or_else(|| tf::Energy::construct(0.0).unwrap()),
            aversion: Some(
                tf::Aversion::construct(self.aversion.clamp(0.0, 1.0))
                    .unwrap_or_else(|| tf::Aversion::construct(0.0).unwrap()),
            ),
        }
    }

    /// Orçamento liberado da reserva: a fração configurada é reserva
    /// de sobrevivência (nunca suspender — Lei 4).
    pub fn reserve_budget(&self, requested: f32) -> f32 {
        requested.min(self.energy_reserve * self.cfg.reserve_fraction.max(0.0).min(1.0))
    }
}

impl Default for LimbicGovernor {
    /// Estado inicial: repouso com política default.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn histerese_entra_e_sai_com_razao_tipada() {
        let mut g = LimbicGovernor::new();
        assert_eq!(g.band(), LimbicBand::Stable);
        // Entra em 0.75.
        assert_eq!(g.update(0.8, 1.0, 0.1), BandReason::StressEntered);
        assert_eq!(g.band(), LimbicBand::Alert);
        // Meio da banda: NÃO sai (histerese — 0.65 > 0.60).
        assert_eq!(g.update(0.65, 1.0, 0.1), BandReason::Unchanged);
        assert_eq!(g.band(), LimbicBand::Alert);
        // Sai só abaixo de 0.60.
        assert_eq!(g.update(0.59, 1.0, 0.1), BandReason::StressRelieved);
        assert_eq!(g.band(), LimbicBand::Stable);
    }

    #[test]
    fn reserva_limitada_pela_fracao_da_config() {
        let mut g = LimbicGovernor::new();
        g.update(0.1, 1.0, 0.0);
        assert!((g.reserve_budget(10.0) - 0.5).abs() < 1e-6, "metade é sobrevivencia");
        g.update(0.1, 0.2, 0.0);
        assert!((g.reserve_budget(10.0) - 0.1).abs() < 1e-6, "teto da reserva manda");
    }

    #[test]
    fn clamps_e_modulacao() {
        let mut g = LimbicGovernor::new();
        g.update(7.0, -3.0, 2.0);
        let m = g.modulation();
        assert!((g.stress - 1.0).abs() < 1e-6);
        assert!((g.energy_reserve - 0.0).abs() < 1e-6);
        let _ = (m.stress, m.energy, m.aversion);
    }
}
