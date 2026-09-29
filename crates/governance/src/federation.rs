use triad_contracts as tc;
use triad_foundation as tf;
use tracing::{debug, trace};

/// Fases do ciclo federativo de governança.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FederationPhase {
    /// Abertura do ciclo com propostas.
    Propose,
    /// Deliberação sobre as propostas.
    Deliberate,
    /// Decisão final entre as propostas.
    Decide,
    /// Execução da decisão aprovada.
    Execute,
    /// Verificação dos efeitos executados.
    Verify,
    /// Liquidação e fechamento do ciclo.
    Settle,
}

/// Estado corrente da federação.
#[derive(Debug, Clone)]
pub struct FederationState {
    /// Fase atual do ciclo federativo.
    pub phase: FederationPhase,
    /// Ciclo corrente (incrementa após cada Settle).
    pub cycle: u64,
}

/// Coordenação federativa com taxa de acordo (denominador sempre).
pub struct Federation {
    /// Estado corrente do ciclo federativo.
    state: FederationState,
    /// Total de acordos observados.
    settlements: u64,
    /// Total de liquidações registradas.
    total: u64,
}

impl Federation {
    /// Cria a federação na fase Propose, ciclo 0.
    pub fn new() -> Self {
        Self {
            state: FederationState {
                phase: FederationPhase::Propose,
                cycle: 0,
            },
            settlements: 0,
            total: 0,
        }
    }

    /// Avança a fase na ordem fixa do ciclo federativo.
    pub fn advance(&mut self) {
        self.state.phase = match self.state.phase {
            FederationPhase::Propose => FederationPhase::Deliberate,
            FederationPhase::Deliberate => FederationPhase::Decide,
            FederationPhase::Decide => FederationPhase::Execute,
            FederationPhase::Execute => FederationPhase::Verify,
            FederationPhase::Verify => FederationPhase::Settle,
            FederationPhase::Settle => {
                self.state.cycle += 1;
                debug!(ciclo = self.state.cycle, "ciclo federativo completo");
                FederationPhase::Propose
            }
        };
    }

    /// Registra uma liquidação e seus acordos.
    pub fn settle(&mut self, agreed: u64) {
        self.total += 1;
        self.settlements += agreed;
        trace!(acordos = agreed, "assentamento federativo");
    }

    /// Taxa de acordos como Qualified; sem ciclos ⇒ NO_DATA.
    pub fn settlement_rate(
        &self,
        source: tf::id::ModuleId,
        step: tf::id::StepId,
    ) -> tc::Qualified<tf::Rate> {
        if self.total == 0 {
            return tc::Qualified::no_data("sem ciclos federativos", source, step);
        }
        match tf::Rate::from_ratio(self.settlements, self.total) {
            Some(rate) => tc::Qualified::value(rate, source, step),
            None => tc::Qualified::invalid("taxa fora do dominio", source, step),
        }
    }

    /// Referência imutável ao estado corrente.
    pub fn state(&self) -> &FederationState {
        &self.state
    }
}
