//! Decodifica tabelas devolvidas por políticas Lua em propostas tipadas.
//! Fronteira Rust/Lua: Lua só opina via `Qualified`; ausência nunca vira zero.

use crate::types::{LuaValue, PolicyProposal};
use triad_contracts as tc;
use triad_foundation as tf;

/// Busca campo na tabela; `Nil` conta como ausente (em Lua, campo nil não existe).
fn field<'a>(pairs: &'a [(String, LuaValue)], key: &str) -> Option<&'a LuaValue> {
    match pairs.iter().find(|(k, _)| k.as_str() == key) {
        Some((_, LuaValue::Nil)) => None,
        Some((_, v)) => Some(v),
        None => None,
    }
}

/// Decodifica pares chave→valor (tabela Lua) numa proposta qualificada.
/// Regra da casa: `reason` obrigatória; campo numérico ausente vira `no_data`.
pub fn decode_proposal(
    pairs: &[(String, LuaValue)],
    source: tf::id::ModuleId,
    step: tf::id::StepId,
) -> tc::Qualified<PolicyProposal> {
    let action = match field(pairs, "action") {
        Some(LuaValue::Str(s)) => s.clone(),
        _ => return tc::Qualified::invalid("action ausente", source, step),
    };

    let reason = match field(pairs, "reason") {
        Some(LuaValue::Str(s)) if !s.is_empty() => s.clone(),
        _ => return tc::Qualified::invalid("reason obrigatória", source, step),
    };

    let confidence = match field(pairs, "confidence") {
        Some(v) => match v.to_f32() {
            Some(n) => n,
            None => return tc::Qualified::invalid("confidence não-numérica", source, step),
        },
        None => return tc::Qualified::invalid("confidence ausente", source, step),
    };
    if !(0.0..=1.0).contains(&confidence) {
        return tc::Qualified::invalid("confidence fora de 0..=1", source, step);
    }

    let value = match field(pairs, "value") {
        Some(v) => match v.to_f32() {
            Some(n) => n,
            None => return tc::Qualified::invalid("value não-numérico", source, step),
        },
        None => return tc::Qualified::no_data("value ausente", source, step),
    };

    let ttl_f = match field(pairs, "ttl") {
        Some(v) => match v.to_f32() {
            Some(n) => n,
            None => return tc::Qualified::invalid("ttl não-numérico", source, step),
        },
        None => return tc::Qualified::no_data("ttl ausente", source, step),
    };
    if !(ttl_f >= 0.0) {
        return tc::Qualified::invalid("ttl negativo", source, step);
    }

    tc::Qualified::value(
        PolicyProposal {
            action,
            value,
            confidence,
            ttl: ttl_f as u32,
            reason,
        },
        source,
        step,
    )
}
