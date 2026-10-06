//! Core of Falog: the task domain, natural-language due dates, agenda grouping
//! and the SQLite store shared by the desktop app and the MCP server.

pub mod agenda;
pub mod date;
pub mod domain;
pub mod error;
pub mod store;
pub mod text;

pub use error::{Error, Result};
pub use store::Store;
