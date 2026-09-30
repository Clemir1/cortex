//! T/lua (17.12): PolicyHost REAL — Lua é APENAS política (retorna
//! Proposal ou nil); Rust VALIDA (whitelist + faixas + Lei 3) e
//! aplica. Sandbox rígido; hashes versionados por arquivo.

pub mod config;
pub mod host;
pub mod proposal;

pub use config::{LuaCfg, SandboxCfg};
pub use host::{FileLoadIssue, PolicyContext, PolicyHost};
pub use proposal::{LuaProposal, ParamSpec, PolicyReject, ValidatedProposal};
