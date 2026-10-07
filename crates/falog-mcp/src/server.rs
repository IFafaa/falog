use crate::protocol::{self, DEFAULT_PROTOCOL_VERSION, Message};
use crate::tools;
use falog_core::Store;
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};

/// Sent to the client on `initialize`; assistants use it as guidance for the whole session.
const INSTRUCTIONS: &str = "Falog is the user's personal task tracker; they are a developer working for \
several companies at once who also tracks personal tasks and appointments, and they usually dictate by \
voice. Tasks belong to areas: one per employer or client, plus personal ones (Personal, Health...). When \
they describe a request (\"I got a task\", \"so-and-so asked\", \"I need to\", \"I have a doctor's \
appointment\"), call create_task with a short actionable title, the area, a \
description that keeps ALL the context they gave, the requester, the due date as YYYY-MM-DD (resolve \
relative dates against today) and a priority. One message may contain several requests: create one task \
each. Before updating a task mentioned by name, find its id with list_tasks. When they ask what to work \
on or to catch up (\"good morning\", \"what's on my plate\", \"what did I do last week\"), call get_agenda. \
Statuses: todo, in_progress, waiting (blocked on someone, in review), done. Reply in the user's language.";

#[derive(Debug)]
pub struct Server {
    store: Store,
}

impl Server {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    /// Serves newline-delimited JSON-RPC until `input` ends.
    pub fn serve(&self, input: impl BufRead, mut output: impl Write) -> io::Result<()> {
        for line in input.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Some(response) = self.handle_line(&line) {
                writeln!(output, "{response}")?;
                output.flush()?;
            }
        }
        Ok(())
    }

    pub fn handle_line(&self, line: &str) -> Option<Value> {
        // Some shells (Windows PowerShell 5.1) prefix piped input with a UTF-8 BOM.
        let line = line.trim_start_matches('\u{feff}');
        match serde_json::from_str::<Message>(line) {
            Ok(message) => self.handle(message),
            Err(err) => Some(protocol::failure(
                Value::Null,
                protocol::PARSE_ERROR,
                format!("invalid JSON: {err}"),
            )),
        }
    }

    fn handle(&self, message: Message) -> Option<Value> {
        let id = message.id?;
        let Some(method) = message.method else {
            return Some(protocol::failure(id, protocol::INVALID_REQUEST, "missing method"));
        };
        let response = match method.as_str() {
            "initialize" => protocol::success(id, Self::initialize(&message.params)),
            "ping" => protocol::success(id, json!({})),
            "tools/list" => protocol::success(id, json!({ "tools": tools::catalog() })),
            "tools/call" => protocol::success(id, self.call_tool(&message.params)),
            "resources/list" => protocol::success(id, json!({ "resources": [] })),
            "prompts/list" => protocol::success(id, json!({ "prompts": [] })),
            other => protocol::failure(
                id,
                protocol::METHOD_NOT_FOUND,
                format!("method not found: {other}"),
            ),
        };
        Some(response)
    }

    fn initialize(params: &Value) -> Value {
        let version = params["protocolVersion"]
            .as_str()
            .unwrap_or(DEFAULT_PROTOCOL_VERSION);
        json!({
            "protocolVersion": version,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "falog", "version": env!("CARGO_PKG_VERSION") },
            "instructions": INSTRUCTIONS,
        })
    }

    /// Tool failures are reported in the result (`isError`) so the model can read and recover.
    fn call_tool(&self, params: &Value) -> Value {
        let name = params["name"].as_str().unwrap_or_default();
        let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);
        match tools::call(&self.store, name, arguments) {
            Ok(text) => json!({ "content": [{ "type": "text", "text": text }] }),
            Err(err) => {
                json!({ "content": [{ "type": "text", "text": format!("Error: {err:#}") }], "isError": true })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server() -> Server {
        Server::new(Store::open_in_memory().unwrap())
    }

    fn call(server: &Server, id: i64, tool: &str, args: Value) -> Value {
        let line = json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call",
                           "params": { "name": tool, "arguments": args } });
        server.handle_line(&line.to_string()).unwrap()["result"].clone()
    }

    fn text(result: &Value) -> &str {
        result["content"][0]["text"].as_str().unwrap()
    }

    #[test]
    fn initializes_with_client_version() {
        let response = server()
            .handle_line(
                r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26"}}"#,
            )
            .unwrap();
        assert_eq!(response["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(response["result"]["serverInfo"]["name"], "falog");
    }

    #[test]
    fn ignores_notifications_and_rejects_unknown_methods() {
        let server = server();
        assert!(
            server
                .handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
                .is_none()
        );
        let response = server
            .handle_line(r#"{"jsonrpc":"2.0","id":2,"method":"nope"}"#)
            .unwrap();
        assert_eq!(response["error"]["code"], protocol::METHOD_NOT_FOUND);
        let response = server.handle_line("{not json").unwrap();
        assert_eq!(response["error"]["code"], protocol::PARSE_ERROR);
    }

    #[test]
    fn tolerates_a_byte_order_mark() {
        let response = server()
            .handle_line("\u{feff}{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}")
            .unwrap();
        assert_eq!(response["result"], json!({}));
    }

    #[test]
    fn lists_every_tool() {
        let response = server()
            .handle_line(r#"{"jsonrpc":"2.0","id":3,"method":"tools/list"}"#)
            .unwrap();
        let names: Vec<&str> = response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(names.len(), 10);
        assert!(names.contains(&"create_task") && names.contains(&"get_agenda"));
    }

    #[test]
    fn creates_updates_and_reads_tasks() {
        let server = server();
        call(&server, 1, "create_area", json!({ "name": "Acme Café" }));
        let created = call(
            &server,
            2,
            "create_task",
            json!({ "title": "Fix Google login", "area": "acme", "priority": "high",
                    "due_date": "2026-10-09", "requester": "Ana" }),
        );
        assert!(
            text(&created).starts_with("Created #1 [Acme Café] Fix Google login"),
            "{}",
            text(&created)
        );

        let updated = call(
            &server,
            3,
            "update_task",
            json!({ "id": "#1", "status": "doing", "note": "repro'd" }),
        );
        assert!(text(&updated).contains("In progress"), "{}", text(&updated));

        let details = call(&server, 4, "get_task", json!({ "id": 1 }));
        assert!(text(&details).contains("repro'd"));
        assert!(text(&call(&server, 5, "get_agenda", json!({}))).contains("Fix Google login"));
    }

    #[test]
    fn recolors_and_renames_areas() {
        let server = server();
        call(&server, 1, "create_area", json!({ "name": "Globex" }));
        let updated = call(
            &server,
            2,
            "update_area",
            json!({ "area": "globex", "color": "#39ff14" }),
        );
        assert_eq!(text(&updated), "Updated area Globex (#39ff14)");
        let renamed = call(
            &server,
            3,
            "update_area",
            json!({ "area": "glob", "name": "House" }),
        );
        assert_eq!(text(&renamed), "Updated area House (#39ff14)");
        let bad = call(
            &server,
            4,
            "update_area",
            json!({ "area": "medi", "color": "green" }),
        );
        assert_eq!(bad["isError"], true);
    }

    #[test]
    fn reports_tool_errors_in_band() {
        let server = server();
        let result = call(
            &server,
            1,
            "create_task",
            json!({ "title": "X", "area": "nowhere" }),
        );
        assert_eq!(result["isError"], true);
        assert!(text(&result).contains("not found"));
        let result = call(&server, 2, "create_task", json!({}));
        assert_eq!(result["isError"], true);
    }
}
