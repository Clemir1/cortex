//! Estágios de development (16.6): maturação guiada pela RESSONÂNCIA
//! REAL do substrato (sinal Chladni, ADR-0007/13.5). Cada estágio
//! declara uma BANDA alvo de ressonância e uma permanência mínima
//! (`dwell_steps` consecutivos dentro da banda) para avançar —
//! sem sinal o estágio CONGELA (ausência ≠ zero: sem medição não
//! há progresso, nunca retrocesso inventado).
//!
//! Cada estágio tem efeito REAL nos orçamentos dos subsistemas
//! (tabela declarada): o organismo amadurece, o reparo cresce.

use serde::{Deserialize, Serialize};
use tracing::info;

/// Estágios canônicos de development do organismo (P3 da 13-0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DevelopmentStage {
    /// Embrião: formação inicial — banda baixa (crescimento livre).
    Embryo,
    /// Clivagem: divisão/estruturação — banda emergente.
    Cleavage,
    /// Corticalização: migração cortical — banda alta.
    Corticalization,
    /// Maduro: forma estável — mantém a banda mais alta.
    Mature,
}

impl DevelopmentStage {
    /// Nome canônico (auditoria/eventos).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Embryo => "EMBRYO",
            Self::Cleavage => "CLEAVAGE",
            Self::Corticalization => "CORTICALIZATION",
            Self::Mature => "MATURE",
        }
    }

    /// Próximo estágio; `None` no MADURO (não há pós-maturação).
    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Embryo => Some(Self::Cleavage),
            Self::Cleavage => Some(Self::Corticalization),
            Self::Corticalization => Some(Self::Mature),
            Self::Mature => None,
        }
    }

    /// Orçamento de regeneração por tick do estágio (efeito REAL:
    /// o organismo amadurece, o reparo cresce). Tabela declarada.
    pub fn heal_budget(&self) -> u32 {
        match self {
            Self::Embryo => 2,
            Self::Cleavage => 4,
            Self::Corticalization => 6,
            Self::Mature => 8,
        }
    }
}

/// `[development.stages]` — banda alvo por estágio e permanência.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StagesCfg {
    /// Ticks CONSECUTIVOS dentro da banda exigidos para avançar
    /// (estabilidade — maturação não é evento único).
    pub dwell_steps: u64,
    /// Banda de ressonância Chladni [min, max] para avançar de EMBRYO.
    pub embryo_band: (f32, f32),
    /// Banda para avançar de CLEAVAGE.
    pub cleavage_band: (f32, f32),
    /// Banda para avançar de CORTICALIZATION.
    pub corticalization_band: (f32, f32),
    /// Banda de MANUTENÇÃO do MADURO (nunca "avança além" — mantém).
    pub mature_band: (f32, f32),
}

impl Default for StagesCfg {
    fn default() -> Self {
        Self {
            dwell_steps: 20,
            // CALIBRADAS POR EVIDÊNCIA (app 16.6, ressonância real
            // ~0.565 estável, banda Stability do L1): o embrião
            // cresce LIVRE [0,0.60] e a harmonia SOBE com a forma —
            // cada estágio estreita em torno da faixa real.
            embryo_band: (0.00, 0.60),
            cleavage_band: (0.50, 0.75),
            corticalization_band: (0.45, 0.85),
            mature_band: (0.45, 1.00),
        }
    }
}

/// Banda alvo do estágio corrente (a transição SAINDO dele).
pub fn band_for(cfg: &StagesCfg, stage: DevelopmentStage) -> (f32, f32) {
    match stage {
        DevelopmentStage::Embryo => cfg.embryo_band,
        DevelopmentStage::Cleavage => cfg.cleavage_band,
        DevelopmentStage::Corticalization => cfg.corticalization_band,
        DevelopmentStage::Mature => cfg.mature_band,
    }
}

/// Desfecho tipado de um tick do tracker (nunca booleano mudo).
#[derive(Debug, Clone, PartialEq)]
pub enum StageTick {
    /// Ressonância dentro da banda; `ticks_in_band` consecutivos.
    InBand { ticks_in_band: u64 },
    /// Ressonância FORA da banda — contagem zera (nunca retrocede).
    OutOfBand { resonance: f32 },
    /// Sem sinal (ausência ≠ zero): congela, contada.
    NoSignal,
    /// AVANÇO de estágio (dwell cumprido dentro da banda).
    Advanced {
        from: DevelopmentStage,
        to: DevelopmentStage,
        resonance: f32,
    },
    /// Maduro: permanece (não há pós-maturação).
    StayedMature { resonance: f32 },
}

/// Rastreador de estágio: consume a ressonância REAL por tick.
#[derive(Debug)]
pub struct StageTracker {
    stage: DevelopmentStage,
    cfg: StagesCfg,
    ticks_in_band: u64,
    no_signal_ticks: u64,
    advances: u64,
}

/// SNAPSHOT cross-run (17.9) do rastreador de estágio — o organismo
/// amadurecido RETOMA do estágio alcançado entre execuções.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StageSnapshot {
    pub stage: DevelopmentStage,
    pub ticks_in_band: u64,
    pub no_signal_ticks: u64,
    pub advances: u64,
}

impl StageTracker {
    /// Novo tracker no EMBRIÃO com a banda declarada da config.
    pub fn new(cfg: StagesCfg) -> Self {
        Self {
            stage: DevelopmentStage::Embryo,
            cfg,
            ticks_in_band: 0,
            no_signal_ticks: 0,
            advances: 0,
        }
    }

    /// Um tick com a ressonância observada (`None` = ausência).
    pub fn tick(&mut self, resonance: Option<f32>) -> StageTick {
        let Some(res) = resonance else {
            self.no_signal_ticks += 1;
            // Ausência não zera o progresso, não retrocede, não avança.
            return StageTick::NoSignal;
        };
        if self.stage == DevelopmentStage::Mature {
            return StageTick::StayedMature { resonance: res };
        }
        let (lo, hi) = band_for(&self.cfg, self.stage);
        if !(lo..=hi).contains(&res) {
            self.ticks_in_band = 0;
            return StageTick::OutOfBand { resonance: res };
        }
        self.ticks_in_band += 1;
        if self.ticks_in_band >= self.cfg.dwell_steps {
            let from = self.stage;
            let to = self.stage.next().expect("checado != Mature");
            self.stage = to;
            self.ticks_in_band = 0;
            self.advances += 1;
            info!(
                de = from.as_str(),
                para = to.as_str(),
                ressonancia = res,
                "estagio de development AVANCOU (banda cumprida)"
            );
            return StageTick::Advanced {
                from,
                to,
                resonance: res,
            };
        }
        StageTick::InBand {
            ticks_in_band: self.ticks_in_band,
        }
    }

    /// Estágio corrente.
    pub fn stage(&self) -> DevelopmentStage {
        self.stage
    }

    /// Avanços de estágio já ocorridos (com denominador natural:
    /// máximo 3 no ciclo de vida EMBRYO→MATURE).
    pub fn advances(&self) -> u64 {
        self.advances
    }

    /// Ticks sem sinal (ausência contada — nunca zero fantasma).
    pub fn no_signal_ticks(&self) -> u64 {
        self.no_signal_ticks
    }

    /// Ticks consecutivos dentro da banda corrente.
    pub fn ticks_in_band(&self) -> u64 {
        self.ticks_in_band
    }

    /// SNAPSHOT cross-run (17.9): estágio corrente + contadores.
    pub fn snapshot(&self) -> StageSnapshot {
        StageSnapshot {
            stage: self.stage,
            ticks_in_band: self.ticks_in_band,
            no_signal_ticks: self.no_signal_ticks,
            advances: self.advances,
        }
    }

    /// RESTORE cross-run (17.9): retoma do estágio alcançado
    /// (chamado no boot com procedência verificada pelo Checkpointer).
    pub fn restore(&mut self, snap: StageSnapshot) {
        self.stage = snap.stage;
        self.ticks_in_band = snap.ticks_in_band;
        self.no_signal_ticks = snap.no_signal_ticks;
        self.advances = snap.advances;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(dwell: u64) -> StagesCfg {
        StagesCfg {
            dwell_steps: dwell,
            ..StagesCfg::default()
        }
    }

    #[test]
    fn avanca_por_banda_com_dwell_consecutivo() {
        let mut t = StageTracker::new(cfg(3));
        // Embryo: banda [0, 0.35]; ressonância 0.30 dentro.
        assert_eq!(
            t.tick(Some(0.30)),
            StageTick::InBand { ticks_in_band: 1 }
        );
        assert_eq!(
            t.tick(Some(0.30)),
            StageTick::InBand { ticks_in_band: 2 }
        );
        // FORA da banda zera a contagem (0.9 fora de [0, 0.35]).
        assert!(matches!(t.tick(Some(0.90)), StageTick::OutOfBand { .. }));
        assert_eq!(
            t.tick(Some(0.30)),
            StageTick::InBand { ticks_in_band: 1 }
        );
        assert_eq!(
            t.tick(Some(0.30)),
            StageTick::InBand { ticks_in_band: 2 }
        );
        // Dwell cumprido (3 consecutivos pós-reset) ⇒ AVANÇO tipado.
        assert_eq!(
            t.tick(Some(0.30)),
            StageTick::Advanced {
                from: DevelopmentStage::Embryo,
                to: DevelopmentStage::Cleavage,
                resonance: 0.30,
            }
        );
        assert_eq!(t.stage(), DevelopmentStage::Cleavage);
        assert_eq!(t.advances(), 1);
    }

    #[test]
    fn ausencia_congela_nunca_retrocede_nem_avanca() {
        let mut t = StageTracker::new(cfg(2));
        t.tick(Some(0.30));
        assert_eq!(t.tick(None), StageTick::NoSignal);
        assert_eq!(t.tick(None), StageTick::NoSignal);
        assert_eq!(t.no_signal_ticks(), 2);
        assert_eq!(t.stage(), DevelopmentStage::Embryo, "sem sinal congela");
        // O progresso anterior NÃO zerou (retoma da contagem viva):
        // o tick 1 sobreviveu às ausências; este é o 2º consecutivo
        // ⇒ dwell=2 cumprido ⇒ AVANÇO (prova da preservação).
        assert_eq!(
            t.tick(Some(0.30)),
            StageTick::Advanced {
                from: DevelopmentStage::Embryo,
                to: DevelopmentStage::Cleavage,
                resonance: 0.30,
            }
        );
        assert_eq!(t.stage(), DevelopmentStage::Cleavage);
    }

    #[test]
    fn maturidade_e_terminal_com_banda_de_manutencao() {
        let mut t = StageTracker::new(cfg(1));
        // Ressonâncias DENTRO da banda de cada estágio (dwell=1 ⇒ 1 tick):
        // Embryo [0,0.35] com 0.30; Cleavage [0.25,0.55] com 0.50;
        // Corticalization [0.45,0.80] com 0.65.
        assert!(matches!(
            t.tick(Some(0.30)),
            StageTick::Advanced { .. }
        ));
        assert!(matches!(
            t.tick(Some(0.50)),
            StageTick::Advanced { .. }
        ));
        assert!(matches!(
            t.tick(Some(0.65)),
            StageTick::Advanced { .. }
        ));
        assert_eq!(t.stage(), DevelopmentStage::Mature);
        // Maduro: permanece tipado (0.50 dentro da banda de manutenção).
        assert_eq!(
            t.tick(Some(0.50)),
            StageTick::StayedMature { resonance: 0.50 }
        );
        assert_eq!(t.advances(), 3, "ciclo completo EMBRYO→MATURE");
    }

    #[test]
    fn a_a_determinismo_do_tracker() {
        let run = || {
            let mut t = StageTracker::new(cfg(2));
            let mut log = Vec::new();
            for r in [Some(0.30), None, Some(0.90), Some(0.31), Some(0.31), Some(0.50)] {
                log.push(format!("{:?}", t.tick(r)));
            }
            log.push(t.stage().as_str().to_string());
            log
        };
        assert_eq!(run(), run(), "mesma sequência ⇒ mesmos desfechos (A/A)");
    }

    #[test]
    fn orcamento_de_cura_cresce_com_a_maturidade() {
        assert_eq!(DevelopmentStage::Embryo.heal_budget(), 2);
        assert_eq!(DevelopmentStage::Cleavage.heal_budget(), 4);
        assert_eq!(DevelopmentStage::Corticalization.heal_budget(), 6);
        assert_eq!(DevelopmentStage::Mature.heal_budget(), 8);
        assert!(DevelopmentStage::Mature.next().is_none());
    }
}
