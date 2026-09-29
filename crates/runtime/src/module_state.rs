/// Estado do ciclo de vida de um módulo cognitivo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleState {
    /// Módulo criado, ainda sem boot.
    Embryo,
    /// Módulo em inicialização.
    Boot,
    /// Módulo operando normalmente.
    Active,
    /// Módulo operando de forma degradada.
    Degraded,
    /// Módulo suspenso por política.
    Suspended,
    /// Módulo aposentado, fora de operação.
    Retired,
}

impl ModuleState {
    /// Informa se o módulo pode sofrer tick (Active ou Degraded).
    pub fn can_tick(&self) -> bool {
        matches!(self, ModuleState::Active | ModuleState::Degraded)
    }
}
