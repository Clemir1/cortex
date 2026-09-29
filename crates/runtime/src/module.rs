use triad_foundation as tf;
use triad_contracts as tc;

/// Contrato base de todo módulo cognitivo executado pelo runtime.
pub trait CognitiveModule: Send + Sync {
    /// Descritor declarado do módulo (somente referenciado, nunca construído aqui).
    fn descriptor(&self) -> &tc::ModuleDescriptor;
    /// Estado atual do ciclo de vida do módulo.
    fn state(&self) -> crate::module_state::ModuleState;
    /// Executa um tick e empurra os eventos produzidos em `out`.
    fn tick(&self, ctx: &crate::context::TypedContext, out: &mut Vec<tc::EventEnvelope>) -> tf::TriadResult<()>;
}
