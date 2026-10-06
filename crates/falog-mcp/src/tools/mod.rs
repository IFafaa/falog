//! The MCP tools: their JSON schemas ([`catalog`]) and implementations ([`call`]).

mod args;
mod catalog;
mod handlers;

pub use catalog::catalog;
pub use handlers::call;
