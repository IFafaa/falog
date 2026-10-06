//! Minimal JSON-RPC 2.0 framing for MCP's stdio transport (one JSON message per line).

use serde::Deserialize;
use serde_json::{Value, json};

/// Used when the client does not state a protocol version.
pub const DEFAULT_PROTOCOL_VERSION: &str = "2025-06-18";

pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const METHOD_NOT_FOUND: i64 = -32601;

/// An incoming request or notification. Notifications have no `id` and get no response.
#[derive(Debug, Deserialize)]
pub struct Message {
    #[serde(default)]
    pub id: Option<Value>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub params: Value,
}

pub fn success(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

pub fn failure(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message.into() } })
}
