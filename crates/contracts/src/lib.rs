//! # triad-contracts — contratos tipados entre camadas L1–L5
//!
//! Puro dados + serde; zero lógica de negócio, zero deps internas além de
//! foundation (ADR-0001). Direção de dependência: foundation → contracts →
//! camadas. Toda fronteira publica valores com status, versão, hash e
//! proveniência (ADR-0004).
//!
//! Famílias de contrato: envelopes, eventos, estado, mensagens, evidência e
//! descritores — mais os contratos temporais:
//! - [`hash`] — hashes estáveis (FNV-1a 64) de estado e ordem de ids;
//! - [`l1`] — contrato temporal da camada L1: fases versionadas, snapshot
//!   com cobertura/política de amostra, recibos de leitura;
//! - [`l2`] — contrato temporal da camada L2 (fase `PostTissue`): snapshot
//!   tecidual com cobertura de atribuição, hash da ordem de tecidos e
//!   recibos de leitura L2→L3.

pub mod descriptor;
pub mod event_envelope;
pub mod events;
pub mod evidence;
pub mod hash;
pub mod l1;
pub mod l2;
pub mod messages;
pub mod state;

/// Valor qualificado da fundação, re-exportado como fronteira canônica:
/// todo consumidor de contratos qualifica dados via `tc::Qualified`.
pub use triad_foundation::value::Qualified;

pub use descriptor::{Layer, ModuleDescriptor};
pub use event_envelope::{DirtyMask, EventEnvelope, EventType, Priority};
pub use evidence::{BoundaryRecord, Chain, StageQuintet};
pub use events::{
    CognitiveEvent, CognitiveKind, DecisionOption, DecisionProposal, GovernanceIntervention,
    IdentityContext, InterventionStatus, LearningEnvelope, LearningStatus, LimbicModulation,
    MemoryEvent, MemoryKind, OutcomeObserved, TissueEvent, TissueKind, TissueState,
    VerifiedEffect,
};
pub use messages::{
    AdaptationRequest, GlobalConstraint, LocalGoal, ParameterAdjustment, PolicyProposal,
    ResourceAllocation,
};
pub use state::{CheckpointLineage, ClusterStateRef, Delta, PlasticityMode, StatePhase, Versioned};

#[cfg(test)]
mod tests {
    use triad_foundation as tf;

    use super::*;

    #[test]
    fn event_envelope_serde_round_trip() {
        let envelope = EventEnvelope {
            event_id: tf::id::EventId::new(),
            parent_event_id: None,
            entity_id: "cluster-7".to_string(),
            entity_version: 3,
            event_type: EventType::Cognitive,
            priority: Priority::High,
            deadline: None,
            dirty_mask: DirtyMask::STATE | DirtyMask::METRICS,
        };
        let json = serde_json::to_string(&envelope).expect("serialize envelope");
        let back: EventEnvelope = serde_json::from_str(&json).expect("deserialize envelope");
        assert_eq!(envelope, back);
    }

    #[test]
    fn decision_proposal_serde_round_trip() {
        let proposal = DecisionProposal {
            event_id: tf::id::EventId::new(),
            options: vec![],
            identity_context: None,
            limbic_modulation: None,
            workspace_contents: vec![],
            learning_status: LearningStatus::Blocked,
            expected_outcome: "recompensa esperada".to_string(),
        };
        let json = serde_json::to_string(&proposal).expect("serialize proposal");
        let back: DecisionProposal = serde_json::from_str(&json).expect("deserialize proposal");
        assert_eq!(proposal, back);
    }

    #[test]
    fn learning_envelope_is_closed_round_trip() {
        let open = LearningEnvelope {
            event_id: tf::id::EventId::new(),
            decision_id: tf::id::DecisionId::new(),
            prediction: "ganho de energia".to_string(),
            action: "explorar setor".to_string(),
            expected_outcome: "energia+".to_string(),
            actual_outcome: "energia-".to_string(),
            error: Some(0.42),
            credit_assignment: vec![],
            updated_modules: vec![],
            memory_update: None,
            policy_update: None,
            verified_future_effect: None,
        };
        assert!(!open.is_closed());

        let closed = LearningEnvelope {
            verified_future_effect: Some(VerifiedEffect {
                observed_step: tf::id::StepId::new(),
                description: "efeito futuro observado".to_string(),
                evidence: tf::evidence::EvidenceLevel::E3Productive,
            }),
            ..open
        };
        assert!(closed.is_closed());

        let json = serde_json::to_string(&closed).expect("serialize envelope");
        let back: LearningEnvelope = serde_json::from_str(&json).expect("deserialize envelope");
        assert_eq!(closed, back);
        assert!(back.is_closed());
    }
}
