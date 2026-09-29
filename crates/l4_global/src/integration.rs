//! Ponte da L4 para o runtime: workspace, decisão e ação integrados.

use std::sync::Mutex;
use tracing::{debug, warn};
use triad_contracts as tc;
use triad_foundation as tf;
use triad_runtime as rt;

use crate::action::execute;
use crate::decision::decide_best;
use crate::degraded::DegradedMode;
use crate::workspace::GlobalWorkspace;
use crate::world_model::WorldModel;
use crate::{ActionRecord, PendingDecision, WorkspaceStage};

/// Módulo L4: integra espaço de trabalho, decisão e ação no runtime.
pub struct L4Module {
    descriptor: tc::ModuleDescriptor,
    workspace: Mutex<GlobalWorkspace>,
    world: Mutex<WorldModel>,
    degraded: Mutex<DegradedMode>,
    state: Mutex<rt::ModuleState>,
}

impl L4Module {
    /// Cria o módulo L4 com workspace, modelo do mundo e modo degradado.
    pub fn new() -> Self {
        Self {
            descriptor: tc::ModuleDescriptor {
                module_id: tf::id::ModuleId::new(),
                name: "l4.global".into(),
                layer: tc::Layer::L4,
                domain: "global-cognition".into(),
            },
            workspace: Mutex::new(GlobalWorkspace::new()),
            world: Mutex::new(WorldModel::new()),
            degraded: Mutex::new(DegradedMode::new()),
            state: Mutex::new(rt::ModuleState::Active),
        }
    }
}

impl Default for L4Module {
    /// Estado inicial: módulo ativo.
    fn default() -> Self {
        Self::new()
    }
}

impl rt::CognitiveModule for L4Module {
    /// Descritor do módulo registrado no runtime.
    fn descriptor(&self) -> &tc::ModuleDescriptor {
        &self.descriptor
    }

    /// Estado corrente; mutex envenenado não bloqueia a leitura.
    fn state(&self) -> rt::ModuleState {
        *self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Compete conteúdos, decide com identidade e executa a ação vencedora.
    fn tick(&self, ctx: &rt::TypedContext, out: &mut Vec<tc::EventEnvelope>) -> tf::TriadResult<()> {
        let module_id = self.descriptor.module_id;
        let step = ctx.clock().step;
        let tick = ctx.clock().tick;

        let mut workspace = self.workspace.lock().unwrap_or_else(|p| p.into_inner());
        // Conteúdos determinísticos por tick: percepção e memória local.
        workspace.submit(WorkspaceStage::Sensory, "percepcao-externa", 0.7);
        workspace.submit(WorkspaceStage::Local, "memoria-local", 0.5);

        if let Some(winner) = workspace.broadcast() {
            // Trilha de auditoria: conteúdo que venceu a competição global.
            debug!(
                saliencia = winner.salience,
                conteudo = %winner.content,
                "l4.workspace broadcast venceu"
            );
            let pending = PendingDecision {
                event_id: tf::id::EventId::new(),
                options: vec![
                    tc::DecisionOption {
                        action: format!("explorar:{}", winner.content),
                        predicted_value: tf::Reward::construct(0.5)
                            .unwrap_or_else(|| tf::Reward::construct(0.0).unwrap()),
                        confidence: tf::Confidence::construct(0.6)
                            .unwrap_or_else(|| tf::Confidence::construct(0.0).unwrap()),
                    },
                    tc::DecisionOption {
                        action: format!("consolidar:{}", winner.content),
                        predicted_value: tf::Reward::construct(0.3)
                            .unwrap_or_else(|| tf::Reward::construct(0.0).unwrap()),
                        confidence: tf::Confidence::construct(0.7)
                            .unwrap_or_else(|| tf::Confidence::construct(0.0).unwrap()),
                    },
                ],
                // Lei da casa: decisão SEM identity_context é rejeitada.
                identity_context: Some(tc::IdentityContext {
                    values: vec!["growth".into(), "stability".into()],
                    continuity: tf::Confidence::construct(1.0)
                        .unwrap_or_else(|| tf::Confidence::construct(0.0).unwrap()),
                    source_module: module_id,
                }),
                limbic_modulation: None,
                expected_outcome: winner.content.clone(),
            };
            // Publica a decisão pendente ANTES de decidir (trilha de auditoria).
            ctx.set(
                "l4.decision.pending",
                tc::Qualified::value(pending.clone(), module_id, step),
            );

            match decide_best(&pending) {
                Ok(option) => {
                    let decision_id = tf::id::DecisionId::new();
                    let record: ActionRecord = execute(&option, decision_id, step);
                    debug!(acao = %record.action, "l4.acao executada");
                    ctx.set(
                        "l4.action.executed",
                        tc::Qualified::value(record.clone(), module_id, step),
                    );
                    let mut world = self.world.lock().unwrap_or_else(|p| p.into_inner());
                    world.observe(&option.action, option.confidence.value());
                }
                Err(e) => {
                    // Decisão rejeitada não vira ação; modo degradado registra.
                    warn!("l4.decisao rejeitada: {e}");
                    eprintln!("[L4] decisao rejeitada: {e}");
                    let mut degraded = self.degraded.lock().unwrap_or_else(|p| p.into_inner());
                    degraded.enter("decisao rejeitada", tick);
                }
            }
        }

        out.push(rt::envelope(
            "l4.global",
            tick,
            tc::EventType::Decision,
            tc::Priority::Normal,
        ));
        Ok(())
    }
}
