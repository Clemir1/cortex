//! Reforço de memória L4→L3: o ciclo de aprendizado FECHADO e
//! CONFIRMADO do L4 reforça o conceito na memória episódica local.
//!
//! Mecânica da casa (espelho do inbox de adaptação L3→L2): a fila é
//! NO DONO (L3); o L4 apenas SUBMETE (nunca escreve estado alheio); o
//! L3 drena no PRÓPRIO tick — quem aplica é o dono, com recibo e
//! contadores. O reforço carrega a razão tipada da verificação
//! (seção 15) e a decisão de reconsolidação da config do L4.

/// Pedido de reforço submetido pelo L4 (um por ciclo confirmado).
#[derive(Debug, Clone)]
pub struct Reinforcement {
    /// Rótulo do conceito (content_key comensurável, ex. `foco:X`).
    pub label: String,
    /// Passo do fechamento do ciclo no L4.
    pub step: u64,
    /// Efeito líquido verificado (actual − sham de deriva).
    pub net_effect: f32,
    /// Reconsolidar episódio prévio do mesmo rótulo (config do L4).
    pub reconsolidate: bool,
}

/// Recibo do reforço aplicado pelo L3 (o dono registra o consumo).
#[derive(Debug, Clone)]
pub struct ReinforcementReceipt {
    /// Rótulo reforçado.
    pub label: String,
    /// Passo da aplicação no tick L3.
    pub applied_step: u64,
    /// Reconsolidou episódio prévio (true) ou gravou novo (false).
    pub reconsolidated: bool,
}

/// Capacidade do inbox por tick (overflow = descarte CONTADO com
/// razão tipada, nunca silencioso).
pub const INBOX_CAPACITY: usize = 128;

#[cfg(test)]
mod tests {
    use super::*;
    use triad_foundation as tf;

    #[test]
    fn reforco_e_recibo_carregam_a_provenancia() {
        let r = Reinforcement {
            label: "foco:concepto".into(),
            step: 42,
            net_effect: 0.01,
            reconsolidate: true,
        };
        assert_eq!(r.label, "foco:concepto");
        let receipt = ReinforcementReceipt {
            label: r.label.clone(),
            applied_step: 43,
            reconsolidated: true,
        };
        assert!(receipt.reconsolidated);
        assert!(INBOX_CAPACITY > 0);
        let _ = tf::StepId::new(); // proveniência disponível no drain real
    }
}
