//! `falog-mcp`: a Model Context Protocol server over stdio.
//!
//! It reads and writes the same SQLite database as the desktop app, which picks up the
//! changes on its own. Register it with e.g. `claude mcp add falog -- falog-mcp`.

mod format;
mod protocol;
mod server;
mod tools;

use falog_core::Store;
use server::Server;
use std::process::ExitCode;

fn main() -> ExitCode {
    let store = match Store::open_default() {
        Ok(store) => store,
        Err(err) => {
            eprintln!("falog-mcp: {err}");
            return ExitCode::FAILURE;
        }
    };
    match Server::new(store).serve(std::io::stdin().lock(), std::io::stdout().lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("falog-mcp: {err}");
            ExitCode::FAILURE
        }
    }
}
