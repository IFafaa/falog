//! The MCP tools: their JSON schemas ([`catalog`]) and implementations ([`call`]).

mod args;
mod calendar;
mod catalog;
mod handlers;

pub use calendar::CalendarAccess;
pub use catalog::catalog;
pub use handlers::call;
