//! Módulo transversal de telemetria (16.9): AUDITA a escada de
//! evidência E0–E5 por módulo — descritores VIVOS vs alvo da config
//! `[telemetry.modules]` — com recibos denominados e GAPS tipados.
//! A escada só sobe com verificação: este módulo REPORTA, nunca
//! promove (Lei 1). `[telemetry.heavy]` off é LIDO e RESPEITADO:
//! nenhum payload é capturado no hot path.

use std::sync::Mutex;

use triad_contracts as tc;
use triad_foundation as tf;
use triad_runtime as rt;
use tracing::{debug, info, warn};

use crate::config::{target_for, TelemetryCfg};
use crate::ledger::{EvidenceLedger, EvidenceReceipt};

/// Telemetria honesta do auditor — denominadores sempre visíveis.
#[derive(Debug, Default, Clone)]
pub struct TelemetryStats {
    /// Ticks executados (denominador natural).
    pub ticks: u64,
    /// Auditorias executadas (a cada emit_interval_steps).
    pub audits: u64,
    /// Última auditoria: (conformes, auditados).
    pub last_compliant: u64,
    pub last_audited: u64,
    /// Gaps acumulados (módulo abaixo do alvo — tipado no ledger).
    pub gap_events: u64,
}

/// Módulo T/telemetry: auditor da escada de evidência.
pub struct TelemetryModule {
    descriptor: tc::ModuleDescriptor,
    config: TelemetryCfg,
    /// Descritores VIVOS dos módulos montados (injetados no boot —
    /// o auditor lê, nunca alcança os módulos).
    descriptors: Vec<tc::ModuleDescriptor>,
    inner: Mutex<Inner>,
}

struct Inner {
    ledger: EvidenceLedger,
    stats: TelemetryStats,
}

impl TelemetryModule {
    /// Compatibilidade: política default, SEM descritores (auditoria
    /// vira ausência contada — não inventa conformidade).
    pub fn new() -> Self {
        Self::new_with_config(TelemetryCfg::default())
    }

    /// Telemetria REAL com política central `[telemetry]`.
    pub fn new_with_config(config: TelemetryCfg) -> Self {
        Self {
            inner: Mutex::new(Inner {
                ledger: EvidenceLedger::new(),
                stats: TelemetryStats::default(),
            }),
            // Descritor canônico: T/OBSERVABILITY — AUDITORIA da
            // escada (O4 observa) sobre descritores declarados ⇒ E3
            // (consome declarações reais e publica relatório com
            // denominador; E5 do próprio auditor aguarda validação
            // futura — reportada honestamente pelo próprio gap).
            descriptor: tc::ModuleDescriptor::new(
                tf::ModuleId::new(),
                "telemetry",
                tc::Layer::Transversal,
                "telemetry",
            )
            .with_support(Some(tc::SupportLayer::Observability))
            .with_orders(&[tc::CyberneticOrder::O4Observation])
            .with_state_owner("telemetry")
            .with_inputs(&["runtime.tick"])
            .with_outputs(&["telemetry.evidence"])
            .with_backend(tc::ExecutionBackend::CpuSeq)
            .with_criticality(tc::Criticality::Normal)
            .with_dependencies(&["runtime.scheduler"])
            .with_evidence(tf::evidence::EvidenceLevel::E3Productive)
            .with_recovery(tc::RecoveryPolicy::RestartModule),
            config,
            descriptors: Vec::new(),
        }
    }

    /// Injeta os DESCRITORES dos módulos montados (boot): o auditor
    /// compara declarado vs alvo da config.
    pub fn with_descriptors(
        mut self,
        descriptors: Vec<tc::ModuleDescriptor>,
    ) -> Self {
        self.descriptors = descriptors;
        self
    }

    /// Inclui o PRÓPRIO descritor na auditoria (o auditor se audita:
    /// gap próprio é tipado — nunca promoção automática).
    pub fn include_self(&mut self) {
        let own = self.descriptor.clone();
        self.descriptors.push(own);
    }

    /// Gaps tipados da ÚLTIMA auditoria (módulos abaixo do alvo).
    pub fn last_gaps(&self) -> Vec<EvidenceReceipt> {
        self.inner
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .ledger
            .last_audit_gaps()
    }

    /// Telemetria corrente (auditoria).
    pub fn stats(&self) -> TelemetryStats {
        self.inner
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .stats
            .clone()
    }

    /// Ledger de recibos (auditoria viva).
    pub fn ledger_snapshot(&self) -> Vec<EvidenceReceipt> {
        self.inner
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .ledger
            .receipts()
            .to_vec()
    }
}

impl Default for TelemetryModule {
    fn default() -> Self {
        Self::new()
    }
}

impl rt::CognitiveModule for TelemetryModule {
    fn descriptor(&self) -> &tc::ModuleDescriptor {
        &self.descriptor
    }

    /// LEI 4: telemetria sempre ativa (crise muda política, nunca
    /// desliga observação).
    fn state(&self) -> rt::ModuleState {
        rt::ModuleState::Active
    }

    fn tick(
        &self,
        ctx: &rt::TypedContext,
        out: &mut Vec<tc::EventEnvelope>,
    ) -> tf::TriadResult<()> {
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => {
                return Err(tf::TriadError::ContractViolation {
                    detail: "mutex de telemetria enveninado".to_string(),
                })
            }
        };
        inner.stats.ticks += 1;
        let tick = ctx.clock().tick;
        let interval = self.config.emit_interval_steps.max(1);

        if !self.config.enabled || tick % interval != 0 {
            return Ok(());
        }

        inner.stats.audits += 1;
        if self.descriptors.is_empty() {
            // Ausência contada: sem descritores não há auditoria —
            // nunca conformidade fantasma.
            warn!("telemetry: sem descritores injetados (ausência contada)");
            return Ok(());
        }

        let mut compliant = 0u64;
        let mut audited = 0u64;
        for d in &self.descriptors {
            let target = target_for(&self.config, &d.name);
            let declared = d.evidence_requirement;
            let receipt = EvidenceReceipt {
                module: d.name.clone(),
                declared,
                target,
                compliant: declared >= target,
                tick,
            };
            if receipt.compliant {
                compliant += 1;
            } else {
                inner.stats.gap_events += 1;
                warn!(
                    modulo = %d.name,
                    declarado = declared.as_str(),
                    alvo = target.as_str(),
                    "GAP de evidencia tipado (escada so sobe com verificacao)"
                );
            }
            inner.ledger.record(receipt);
            audited += 1;
        }
        inner.stats.last_compliant = compliant;
        inner.stats.last_audited = audited;

        let taxa = if audited == 0 {
            None
        } else {
            Some(compliant as f32 / audited as f32)
        };
        ctx.set(
            "telemetry.status",
            TelemetryStatus {
                audits: inner.stats.audits,
                audited,
                compliant,
                compliance_rate: taxa,
                heavy_enabled: self.config.heavy.enabled,
            },
        );
        info!(
            conformes = compliant,
            auditados = audited,
            "auditoria da escada de evidencia emitida (com denominador)"
        );
        out.push(rt::envelope(
            "telemetry.evidence",
            tick,
            tc::EventType::Governance,
            tc::Priority::Normal,
        ));
        Ok(())
    }
}

/// Fotografia publicada no contexto a cada auditoria.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TelemetryStatus {
    pub audits: u64,
    pub audited: u64,
    pub compliant: u64,
    /// Taxa de conformidade COM denominador (None = sem auditados).
    pub compliance_rate: Option<f32>,
    pub heavy_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use triad_runtime::CognitiveModule as _;

    fn desc(name: &str, level: tf::evidence::EvidenceLevel) -> tc::ModuleDescriptor {
        tc::ModuleDescriptor::new(tf::ModuleId::new(), name, tc::Layer::Transversal, name)
            .with_support(Some(tc::SupportLayer::Observability))
            .with_orders(&[tc::CyberneticOrder::O4Observation])
            .with_state_owner("telemetry")
            .with_inputs(&["runtime.tick"])
            .with_outputs(&["telemetry.evidence"])
            .with_backend(tc::ExecutionBackend::CpuSeq)
            .with_criticality(tc::Criticality::Normal)
            .with_dependencies(&["runtime.scheduler"])
            .with_evidence(level)
            .with_recovery(tc::RecoveryPolicy::RestartModule)
    }

    #[test]
    fn audita_descritores_com_denominador_e_gap_tipado() {
        let tel = TelemetryModule::new_with_config(TelemetryCfg {
            emit_interval_steps: 1,
            ..TelemetryCfg::default()
        })
        .with_descriptors(vec![
            desc("l1_substrate", tf::evidence::EvidenceLevel::E5EffectValidated),
            desc("ponte_qualquer", tf::evidence::EvidenceLevel::E2Executed),
        ]);
        let mut clock = tf::LogicalClock::new();
        clock.advance();
        let ctx = rt::TypedContext::new(clock);
        let mut out = Vec::new();
        tel.tick(&ctx, &mut out).expect("tick 1");
        let s = tel.stats();
        assert_eq!(s.audits, 1);
        assert_eq!(s.last_audited, 2, "denominador");
        assert_eq!(s.last_compliant, 1);
        let gaps = tel
            .ledger_snapshot()
            .into_iter()
            .filter(|r| !r.compliant)
            .collect::<Vec<_>>();
        assert_eq!(gaps.len(), 1, "gap tipado, não silenciado");
        assert_eq!(gaps[0].module, "ponte_qualquer");
        assert!(!out.is_empty(), "evento por auditoria");
    }

    #[test]
    fn sem_descritores_e_ausencia_contada() {
        let tel = TelemetryModule::new_with_config(TelemetryCfg {
            emit_interval_steps: 1,
            ..TelemetryCfg::default()
        });
        let mut clock = tf::LogicalClock::new();
        clock.advance();
        let ctx = rt::TypedContext::new(clock);
        let mut out = Vec::new();
        tel.tick(&ctx, &mut out).expect("tick");
        let s = tel.stats();
        assert_eq!(s.audits, 1, "a auditoria rodou");
        assert_eq!(s.last_audited, 0, "sem conformidade fantasma");
        assert!(out.is_empty(), "sem evento sem dado");
    }

    #[test]
    fn intervalo_da_config_e_respeitado() {
        let tel = TelemetryModule::new_with_config(TelemetryCfg {
            emit_interval_steps: 10,
            ..TelemetryCfg::default()
        })
        .with_descriptors(vec![desc("x", tf::evidence::EvidenceLevel::E3Productive)]);
        let mut clock = tf::LogicalClock::new();
        let mut audits = 0u64;
        for t in 1..=21 {
            clock.advance();
            let ctx = rt::TypedContext::new(clock);
            let mut out = Vec::new();
            tel.tick(&ctx, &mut out).expect("tick");
            audits += out.len() as u64;
            let _ = t;
        }
        assert_eq!(audits, 2, "intervalo 10: ticks 10 e 20");
    }

    #[test]
    fn a_a_determinismo_do_auditor() {
        let run = || {
            let tel = TelemetryModule::new_with_config(TelemetryCfg {
                emit_interval_steps: 1,
                ..TelemetryCfg::default()
            })
            .with_descriptors(vec![
                desc("l1", tf::evidence::EvidenceLevel::E5EffectValidated),
                desc("l2", tf::evidence::EvidenceLevel::E3Productive),
            ]);
            let mut clock = tf::LogicalClock::new();
            clock.advance();
            let ctx = rt::TypedContext::new(clock);
            let mut out = Vec::new();
            tel.tick(&ctx, &mut out).expect("tick");
            let s = tel.stats();
            vec![
                s.audits as u64,
                s.last_audited as u64,
                s.last_compliant as u64,
                tel.ledger_snapshot().len() as u64,
            ]
        };
        assert_eq!(run(), run(), "mesmos descritores ⇒ mesma auditoria (A/A)");
    }
}
