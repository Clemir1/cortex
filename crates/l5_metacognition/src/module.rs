//! Ponte da L5 para o runtime: publica identidade e intervenções.

use triad_contracts as tc;
use triad_foundation as tf;
use triad_runtime as rt;

use crate::governor::LimbicGovernor;
use crate::identity::IdentityStore;
use crate::meta_controller::MetaController;

/// Ponte da L5 para o runtime: publica identidade e intervenções.
pub struct L5Module {
    /// Descritor do módulo registrado no runtime.
    descriptor: tc::ModuleDescriptor,
    /// Arco de identidade observado e publicado a cada tick.
    identity: IdentityStore,
    /// Governador límbico acoplado à ponte.
    governor: LimbicGovernor,
    /// Controlador metacognitivo de intervenções ativas.
    meta: MetaController,
    /// Estado corrente do módulo no runtime.
    state: std::sync::Mutex<rt::ModuleState>,
}

impl L5Module {
    /// Cria a ponte com componentes já construídos.
    pub fn new(
        descriptor: tc::ModuleDescriptor,
        identity: IdentityStore,
        governor: LimbicGovernor,
        meta: MetaController,
    ) -> Self {
        Self {
            descriptor,
            identity,
            governor,
            meta,
            // Nasce ativo: Boot/Embryo não podem tickar no runtime.
            state: std::sync::Mutex::new(rt::ModuleState::Active),
        }
    }
}

impl rt::CognitiveModule for L5Module {
    /// Descritor do módulo registrado no runtime.
    fn descriptor(&self) -> &tc::ModuleDescriptor {
        &self.descriptor
    }

    /// Estado corrente; mutex envenenado não bloqueia a leitura.
    fn state(&self) -> rt::ModuleState {
        *self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Publica contexto de identidade, emite evento meta e ativa o módulo.
    fn tick(&self, ctx: &rt::TypedContext, out: &mut Vec<tc::EventEnvelope>) -> tf::TriadResult<()> {
        let identity_ctx = self.identity.identity_context(self.descriptor.module_id);
        ctx.set("l5.identity.context", identity_ctx);
        // MetaController não expõe contagem/vetor Clone por &self: nada a publicar.
        out.push(rt::envelope("l5.meta", 1, tc::EventType::Cognitive, tc::Priority::Normal));
        *self.state.lock().unwrap_or_else(|p| p.into_inner()) = rt::ModuleState::Active;
        Ok(())
    }
}
