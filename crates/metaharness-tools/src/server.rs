//! JSON-RPC 2.0 over stdio: one request object per line, one response object per line.

use std::io::{BufRead, Write};

use b10x_harness_tools::Verbs;
use b10x_harness_wire::{CallId, ToolCall, ToolName, ToolPort};
use serde_json::{Value, json};

/// The name the vendor prefixes its tools with, so the model sees `mcp__metaharness__tool_search`.
///
/// It is also the string `--allowedTools mcp__metaharness` grants, and the two must agree — a
/// mismatch grants a server that does not exist and the run has no tools at all.
pub const SERVER_NAME: &str = "metaharness";

/// The MCP revision this server answers with when the client names none.
///
/// A client that names one gets its own back: this server implements the subset every revision
/// since `2024-11-05` agrees on, so refusing a client over a date would refuse a run for nothing.
pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// JSON-RPC's "method not found".
const METHOD_NOT_FOUND: i64 = -32601;
/// JSON-RPC's "invalid request".
const INVALID_REQUEST: i64 = -32600;

/// The three verbs, answering MCP.
pub struct Server {
    verbs: Verbs,
}

impl std::fmt::Debug for Server {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Server").finish_non_exhaustive()
    }
}

impl Server {
    /// Serves this catalogue.
    #[must_use]
    pub fn new(verbs: Verbs) -> Self {
        Self { verbs }
    }

    /// Answers one request, or `None` when it was a notification.
    ///
    /// Split out from [`serve`] so the message shapes can be asserted on without a pipe, a child
    /// process or a vendor: what this server gets wrong will be a field name, and a test that had
    /// to spawn something to see one would not be written.
    pub fn handle(&mut self, request: &Value) -> Option<Value> {
        let id = request.get("id").cloned();
        let method = request.get("method").and_then(Value::as_str);
        let params = request.get("params").cloned().unwrap_or_else(|| json!({}));

        let outcome = match method {
            Some("initialize") => Ok(Self::initialize(&params)),
            Some("tools/list") => Ok(self.list()),
            Some("tools/call") => Ok(self.call(&params)),
            Some("ping") => Ok(json!({})),
            // Every `notifications/*` message is one, `initialized` above all: the client sends it
            // after `initialize` and expects silence. Answering would be a protocol error.
            Some(other) if other.starts_with("notifications/") => return None,
            Some(other) => Err((METHOD_NOT_FOUND, format!("`{other}` is not a method"))),
            None => Err((INVALID_REQUEST, "a request must name a `method`".to_owned())),
        };

        // A notification is performed and not answered — including one that failed. There is no
        // `id` to answer to, and inventing one is worse than the silence.
        let id = id?;
        Some(match outcome {
            Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
            Err((code, message)) => {
                json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
            }
        })
    }

    fn initialize(params: &Value) -> Value {
        json!({
            "protocolVersion": params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or(PROTOCOL_VERSION),
            "capabilities": {"tools": {}},
            "serverInfo": {"name": SERVER_NAME, "version": env!("CARGO_PKG_VERSION")},
        })
    }

    fn list(&self) -> Value {
        let tools: Vec<Value> = self
            .verbs
            .specs()
            .iter()
            .map(|spec| {
                json!({
                    "name": spec.name.as_str(),
                    "description": spec.description,
                    "inputSchema": spec.input_schema,
                })
            })
            .collect();
        json!({"tools": tools})
    }

    fn call(&mut self, params: &Value) -> Value {
        let Some(name) = params.get("name").and_then(Value::as_str) else {
            return failed("a `tools/call` must name a tool");
        };
        let Ok(name) = ToolName::new(name) else {
            return failed(&format!("`{name}` is not a tool name this server can hold"));
        };
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));

        // MCP carries no call identity of its own, so one is minted here. It reaches the verbs, is
        // never sent anywhere, and exists because `ToolCall` is the same type the b10x loop uses —
        // one tool surface means one call type, not two that agree by inspection.
        let outcome = self.verbs.call(&ToolCall {
            call_id: CallId::new("mcp").expect("a constant call id is legal"),
            name,
            arguments,
        });

        // A tool that failed answers `isError` with the reason as text, rather than a JSON-RPC
        // error: the vendor turns this into a tool result the model reads and can act on, whereas a
        // protocol error is the *client's* problem and never reaches the model at all.
        let text = if outcome.failed {
            outcome
                .output
                .as_str()
                .map_or_else(|| outcome.output.to_string(), ToOwned::to_owned)
        } else {
            serde_json::to_string_pretty(&outcome.output)
                .unwrap_or_else(|error| format!("the tool's answer could not be rendered: {error}"))
        };
        json!({
            "content": [{"type": "text", "text": text}],
            "isError": outcome.failed,
        })
    }
}

fn failed(message: &str) -> Value {
    json!({"content": [{"type": "text", "text": message}], "isError": true})
}

/// Reads requests from `input` until it ends, writing one response line per answered request.
///
/// # Errors
///
/// Returns the first write error. A line that is not JSON is skipped rather than fatal: a client
/// that sent one has a bug the run should survive, and the alternative — exiting — takes the tools
/// away mid-turn and the model is told nothing.
pub fn serve(
    server: &mut Server,
    input: impl BufRead,
    mut output: impl Write,
) -> std::io::Result<()> {
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(request) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if let Some(response) = server.handle(&request) {
            writeln!(output, "{response}")?;
            output.flush()?;
        }
    }
    Ok(())
}
