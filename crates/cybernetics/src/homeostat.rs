//! Homeostato: adaptação de parâmetros (ordem O1 de Ashby).

use tracing::debug;

/// Controlador homeostático com ação corretiva proporcional-integral.
#[derive(Debug, Clone)]
pub struct Homeostat {
    pub target: f32,
    pub tolerance: f32,
    pub current: f32,
    pub integral: f32,
    pub gain: f32,
}

impl Homeostat {
    /// Cria um homeostato com leitura atual no alvo e ganho 0.5.
    pub fn new(target: f32, tolerance: f32) -> Self {
        Self {
            target,
            tolerance,
            current: target,
            integral: 0.0,
            gain: 0.5,
        }
    }

    /// Registra a leitura atual e acumula o erro no integral (limitado a ±5.0).
    pub fn observe(&mut self, value: f32) {
        self.current = value;
        let error = self.target - value;
        self.integral = (self.integral + error).clamp(-5.0, 5.0);
    }

    /// Retorna true se a leitura está dentro da tolerância do alvo.
    pub fn is_satisfied(&self) -> bool {
        (self.current - self.target).abs() <= self.tolerance
    }

    /// Ação corretiva O1: ganho * erro + 0.1 * integral.
    pub fn corrective(&self) -> f32 {
        if !self.is_satisfied() {
            debug!(desvio = self.current - self.target, "leitura fora da banda");
        }
        self.gain * (self.target - self.current) + 0.1 * self.integral
    }

    /// 19.8-b (O2 retuning) — adapta o GANHO do controlador O1
    /// (herança edge_of_chaos.py:431-452: o O2 retuna o controlador,
    /// não o alvo). Clamp do ganho em [0.5×, 2×] do NOMINAL 0.5
    /// (nunca zero — controle nunca desliga); razão canônica para a
    /// trilha. Devolve o ganho RESULTANTE (telemetria honesta).
    pub fn retune_gain(&mut self, delta: f32, reason: &str) -> f32 {
        self.gain = (self.gain + delta).clamp(0.25, 1.0);
        debug!(
            ganho = self.gain,
            delta,
            razao = reason,
            "O2 retuning do ganho do O1"
        );
        self.gain
    }

    /// Ganho corrente (observação para telemetria A/A).
    pub fn gain(&self) -> f32 {
        self.gain
    }
}
