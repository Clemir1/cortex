//! Modelo do mundo: crenças versionadas por observação + ROLLOUT
//! RASO DETERMINÍSTICO (seção 16.3): trajetória contrafactual das
//! crenças para comparação com o observado — sem fabricar: o
//! rollout só PROLONGA o que foi medido (extrapolação linear da
//! tendência OBSERVADA; com menos de 2 amostras, persistência
//! pura da única observação real). Ausência continua ausência
//! (NO_DATA), nunca zero.

use std::collections::{HashMap, VecDeque};

use triad_contracts as tc;
use triad_foundation as tf;
use tracing::trace;

/// Janela de observações por crença para a tendência (rotativa:
/// cap fixo — o organismo não cresce sem limite). Declarada antes
/// de medir: 8 amostras bastam para a inclinação local estável.
pub const TREND_WINDOW: usize = 8;

/// Observação pontual de uma crença.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Observation {
    /// Passo lógico da observação.
    pub step: u64,
    /// Confiança observada (0.0..1.0).
    pub confidence: f32,
}

/// Modelo do mundo: crenças versionadas por observação.
pub struct WorldModel {
    /// Crenças correntes por chave.
    beliefs: HashMap<String, f32>,
    /// Histórico rotativo por chave (insumo da tendência/rollout).
    history: HashMap<String, VecDeque<Observation>>,
    /// Versão monotônica do modelo.
    version: u64,
}

impl WorldModel {
    /// Modelo do mundo vazio.
    pub fn new() -> Self {
        Self {
            beliefs: HashMap::new(),
            history: HashMap::new(),
            version: 0,
        }
    }

    /// Registra observação: atualiza confiança, empurra no
    /// histórico (janela rotativa) e avança a versão.
    pub fn observe(&mut self, key: &str, confidence: f32, step: u64) {
        let conf = confidence.clamp(0.0, 1.0);
        self.beliefs.insert(key.to_string(), conf);
        let hist = self
            .history
            .entry(key.to_string())
            .or_default();
        if hist.len() >= TREND_WINDOW {
            hist.pop_front();
        }
        hist.push_back(Observation {
            step,
            confidence: conf,
        });
        self.version += 1;
        trace!(chave = %key, passo = step, versao = self.version, "crenca observada");
    }

    /// Consulta uma crença qualificada; ausência nunca vira zero.
    pub fn belief(
        &self,
        key: &str,
        source: tf::id::ModuleId,
        step: tf::id::StepId,
    ) -> tc::Qualified<f32> {
        match self.beliefs.get(key) {
            Some(confidence) => tc::Qualified::value(*confidence, source, step),
            None => tc::Qualified::no_data("crença ausente", source, step),
        }
    }

    /// Tendência da crença (inclinação por PASSO, com denominador:
    /// Δconfiança / Δpassos da janela). Menos de 2 amostras ou
    /// passos idênticos: NO_DATA — nunca inclinação inventada.
    pub fn trend(
        &self,
        key: &str,
        source: tf::id::ModuleId,
        step: tf::id::StepId,
    ) -> tc::Qualified<f32> {
        let Some(hist) = self.history.get(key) else {
            return tc::Qualified::no_data("crença ausente", source, step);
        };
        if hist.len() < 2 {
            return tc::Qualified::no_data("janela insuficiente (1 amostra)", source, step);
        }
        let first = hist.front().expect("len>=2");
        let last = hist.back().expect("len>=2");
        let dstep = last.step.saturating_sub(first.step);
        if dstep == 0 {
            return tc::Qualified::no_data("passos idênticos na janela", source, step);
        }
        let slope = (last.confidence - first.confidence) / dstep as f32;
        tc::Qualified::value(slope, source, step)
    }

    /// ROLLOUT RASO (16.3): trajetória contrafactual DETERMINÍSTICA
    /// de `depth` passos a partir da última confiança observada.
    /// Modelo declarado: extrapolação linear da tendência medida
    /// (`v_k = clamp(last + (k+1)·trend, 0, 1)`); com uma única
    /// amostra, PERSISTÊNCIA (a observação real prolongada — o
    /// contrafactual conservador); sem crença: NO_DATA (ausência ≠
    /// zero). Nunca fabrica: só prolonga o que foi observado.
    pub fn rollout(
        &self,
        key: &str,
        depth: usize,
        source: tf::id::ModuleId,
        step: tf::id::StepId,
    ) -> tc::Qualified<Vec<f32>> {
        let Some(last_conf) = self.beliefs.get(key) else {
            return tc::Qualified::no_data("crença ausente", source, step);
        };
        if depth == 0 {
            return tc::Qualified::no_data("profundidade zero", source, step);
        }
        let slope = self
            .trend(key, source, step)
            .as_ref_value()
            .copied()
            .unwrap_or(0.0); // 1 amostra: persistência pura (declarada)
        let path: Vec<f32> = (1..=depth)
            .map(|k| (*last_conf + slope * k as f32).clamp(0.0, 1.0))
            .collect();
        tc::Qualified::value(path, source, step)
    }

    /// Versão monotônica do modelo.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Número de crenças correntes.
    pub fn len(&self) -> usize {
        self.beliefs.len()
    }
}

impl Default for WorldModel {
    /// Estado inicial: modelo vazio.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> (tf::id::ModuleId, tf::id::StepId) {
        (tf::id::ModuleId::new(), tf::id::StepId::new())
    }

    #[test]
    fn rollout_extrapolada_a_tendencia_observada() {
        let (src, st) = ctx();
        let mut w = WorldModel::new();
        // Tendência medida: +0.10 por passo (2 amostras, passos 10→12).
        w.observe("k", 0.30, 10);
        w.observe("k", 0.50, 12);
        let q = w.rollout("k", 3, src, st);
        let path = q.as_ref_value().expect("rollout com janela");
        // slope = 0.10/passo (f32): v_1 = 0.60, v_2 = 0.70,
        // v_3 = 0.5+0.1·3 = 0.79999995 (literal EXATO da aritmética
        // f32 — sem tolerância; o A/A bit-idêntico é o assert abaixo).
        assert_eq!(*path, vec![0.6, 0.7, 0.79999995]);
        // Determinístico: mesma sequência ⇒ mesma trajetória (A/A).
        let mut w2 = WorldModel::new();
        w2.observe("k", 0.30, 10);
        w2.observe("k", 0.50, 12);
        let q2 = w2.rollout("k", 3, src, st);
        assert_eq!(
            q2.as_ref_value().expect("path"),
            path,
            "mesma sequência de observações ⇒ mesmo rollout (bit-idêntico)"
        );
    }

    #[test]
    fn rollout_com_uma_amostra_e_persistencia_pura() {
        let (src, st) = ctx();
        let mut w = WorldModel::new();
        w.observe("k", 0.42, 7);
        let q = w.rollout("k", 3, src, st);
        let path = q.as_ref_value().expect("persistência da única observação real");
        assert_eq!(*path, vec![0.42, 0.42, 0.42]);
        // Tendência sem 2 amostras: NO_DATA (ausência ≠ zero).
        assert!(w.trend("k", src, st).as_ref_value().is_none());
    }

    #[test]
    fn ausencia_continua_ausencia_nunca_zero() {
        let (src, st) = ctx();
        let w = WorldModel::new();
        assert!(w.belief("inexistente", src, st).as_ref_value().is_none());
        assert!(w.rollout("inexistente", 3, src, st).as_ref_value().is_none());
        assert!(w.trend("inexistente", src, st).as_ref_value().is_none());
    }

    #[test]
    fn rollout_respeita_o_limite_e_a_profundidade_zero() {
        let (src, st) = ctx();
        let mut w = WorldModel::new();
        w.observe("k", 0.95, 1);
        w.observe("k", 1.00, 2); // slope = +0.05
        let q = w.rollout("k", 4, src, st);
        let path = q.as_ref_value().expect("clamp no domínio [0,1]");
        assert_eq!(*path, vec![1.0, 1.0, 1.0, 1.0], "clamp superior");
        assert!(
            w.rollout("k", 0, src, st).as_ref_value().is_none(),
            "profundidade zero = NO_DATA"
        );
    }

    #[test]
    fn janela_rotativa_mantem_a_ultima_observacao_e_o_trend_frescos() {
        let (src, st) = ctx();
        let mut w = WorldModel::new();
        // Enche a janela (TREND_WINDOW+1 observações): a primeira cai.
        for i in 0..=(TREND_WINDOW as u64) {
            w.observe("k", 0.1, i);
        }
        assert!(w.history.get("k").map(|h| h.len()).unwrap_or(0) <= TREND_WINDOW);
        // Última confiança é a corrente; tendência usa a janela viva.
        let qb = w.belief("k", src, st);
        let b = qb.as_ref_value().expect("crença corrente");
        assert!((*b - 0.1).abs() < 1e-6);
        let qt = w.trend("k", src, st);
        let t = qt.as_ref_value().expect("trend com janela");
        assert!((*t).abs() < 1e-6, "constante ⇒ inclinação zero com denominador");
    }
}
