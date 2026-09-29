//! Marshalling Rust/Lua e registro de políticas do Triad_AEE.
//! Lua é apenas política: aqui só decodificamos propostas, nunca escrevemos estado.

pub mod types;
pub mod host;
pub mod bridge;

pub use triad_foundation as tf;
pub use triad_contracts as tc;

pub use bridge::decode_proposal;
pub use host::PolicyHost;
pub use types::{LuaValue, PolicyProposal};
