//! Tipos de fronteira entre Rust e políticas Lua.

/// Valor Lua reduzido aceito no marshalling (nil, bool, número, string).
#[derive(Debug, Clone)]
pub enum LuaValue {
    Nil,
    Bool(bool),
    Num(f32),
    Str(String),
}

impl LuaValue {
    /// Converte para f32 apenas se for `Num`; demais variantes → `None`.
    pub fn to_f32(&self) -> Option<f32> {
        match self {
            LuaValue::Num(n) => Some(*n),
            _ => None,
        }
    }
}

/// Forma tipada da tabela que toda política Lua devolve.
#[derive(Debug, Clone)]
pub struct PolicyProposal {
    pub action: String,
    pub value: f32,
    pub confidence: f32,
    pub ttl: u32,
    pub reason: String,
}
