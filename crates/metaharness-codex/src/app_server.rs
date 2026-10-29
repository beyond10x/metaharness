//! The `codex app-server` route: a caller's catalogue offered as client tools (design amendment
//! a66).
//!
//! A run that names `--catalogue` launches `codex app-server --strict-config --listen stdio://`
//! instead of `codex exec`, and talks to it in newline-delimited JSON-RPC without a `jsonrpc`
//! field (`codex-rs/app-server-protocol/src/rpc.rs:46-56` at `rust-v0.160.0`). This module holds
//! the vendor's half of that conversation as values and a pure state machine: what the opening
//! messages are, what each message the server sends asks of metaharness, and what the answer to
//! a client tool call looks like. The runner that owns the pipes lives in the core crate and
//! parses none of it (invariant 2).
//!
//! Every shape here is read from the source at `rust-v0.160.0` and driven against the 0.160.0
//! binary (amendment a66 § 6). The same lines at `rust-v0.153.4` were read and not driven.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// The launch's arguments, in order (`codex-rs/cli/src/main.rs:562-575`).
///
/// `--strict-config` fails the start on a config key the binary does not know, where it would
/// otherwise be dropped in silence, which is the failure this adapter is written against.
pub const APP_SERVER_ARGS: [&str; 4] = ["app-server", "--strict-config", "--listen", "stdio://"];

/// The id of the `initialize` request.
pub const INITIALIZE_ID: u64 = 1;
/// The id of the `thread/start` request.
pub const THREAD_START_ID: u64 = 2;
/// The id of the `turn/start` request.
pub const TURN_START_ID: u64 = 3;

/// The JSON-RPC error code a server request this route does not serve is answered with.
pub const METHOD_NOT_FOUND: i64 = -32601;

/// The method of a client tool call (`app-server-protocol/src/protocol/common.rs:1796-1799`).
pub const TOOL_CALL_METHOD: &str = "item/tool/call";

/// The directory under the scratch root the runner reads the opening and the catalogue copy from.
const APP_SERVER_DIR: &str = "app-server";

/// One client tool offered at `thread/start`: a catalogue entry under its published name, with
/// the description and input schema the Claude route's server lists for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicTool {
    /// The entry's `mcp` rendering, which is also the name the model calls.
    pub name: String,
    /// What the model is told the tool does.
    pub description: String,
    /// The JSON Schema its arguments are checked against, by the vendor and again by the server.
    pub input_schema: Value,
}

impl DynamicTool {
    /// The `DynamicToolSpec` the vendor reads (`app-server-protocol/src/protocol/v2/thread.rs`).
    #[must_use]
    pub fn spec(&self) -> Value {
        json!({
            "type": "function",
            "name": self.name,
            "description": self.description,
            "inputSchema": self.input_schema,
        })
    }
}

/// The three opening messages' parameters, decided by the launch plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppServerOpening {
    /// `initialize`: who is asking, and the experimental API that `environments` and
    /// `dynamicTools` need.
    pub initialize: Value,
    /// `thread/start`: the cwd, the model, no environment and the catalogue's tools.
    pub thread_start: Value,
    /// `turn/start`'s `input`: the prompt as the exec route composes it.
    pub turn_input: Value,
}

impl AppServerOpening {
    /// The opening for this run.
    #[must_use]
    pub fn new(cwd: &Path, model: Option<&str>, tools: &[DynamicTool], prompt: &str) -> Self {
        let mut thread_start = serde_json::Map::new();
        thread_start.insert("cwd".to_owned(), json!(cwd.display().to_string()));
        if let Some(model) = model {
            thread_start.insert("model".to_owned(), json!(model));
        }
        // No environment, so no shell, no `apply_patch` and no `view_image`
        // (`core/src/tools/spec_plan.rs:1083-1087`, `:1269`, `:1283`).
        thread_start.insert("environments".to_owned(), json!([]));
        thread_start.insert(
            "dynamicTools".to_owned(),
            Value::Array(tools.iter().map(DynamicTool::spec).collect()),
        );
        Self {
            initialize: json!({
                "clientInfo": {
                    "name": "metaharness",
                    "title": null,
                    "version": env!("CARGO_PKG_VERSION"),
                },
                "capabilities": {"experimentalApi": true},
            }),
            thread_start: Value::Object(thread_start),
            turn_input: json!([{"type": "text", "text": prompt}]),
        }
    }

    /// The opening as one document, which is what the runner reads.
    #[must_use]
    pub fn to_value(&self) -> Value {
        json!({
            "initialize": self.initialize,
            "thread_start": self.thread_start,
            "turn_input": self.turn_input,
        })
    }

    /// The opening read back from [`AppServerOpening::to_value`], or `None` when a part is
    /// missing.
    #[must_use]
    pub fn from_value(value: &Value) -> Option<Self> {
        Some(Self {
            initialize: value.get("initialize")?.clone(),
            thread_start: value.get("thread_start")?.clone(),
            turn_input: value.get("turn_input")?.clone(),
        })
    }
}

/// Where the runner finds what the launch wrote for it: the opening and the catalogue's copy.
#[must_use]
pub fn app_server_dir(scratch_root: &Path) -> PathBuf {
    scratch_root.join(APP_SERVER_DIR)
}

/// The opening document, under [`app_server_dir`].
#[must_use]
pub fn opening_path(app_server_dir: &Path) -> PathBuf {
    app_server_dir.join("opening.json")
}

/// The catalogue's scratch copy the runner's server reads, under [`app_server_dir`].
#[must_use]
pub fn catalogue_copy_path(app_server_dir: &Path) -> PathBuf {
    app_server_dir.join("catalogue.json")
}

/// What one message from the app-server asks the runner to do.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionAction {
    /// Write this message to the child's stdin.
    Send(Value),
    /// A client tool call is waiting for an answer.
    ToolCall {
        /// The name a decision for it is routed under.
        key: String,
        /// The JSON-RPC id the answer must carry.
        request_id: Value,
        /// The tool, with its namespace when it has one: `namespace/tool`, which no catalogue
        /// entry can be named.
        tool: String,
        /// The arguments, as the vendor sent them.
        arguments: Value,
        /// The line the codex seam reads as the call.
        line: String,
    },
    /// The turn is over: close the child's stdin, and the app-server exits.
    CloseInput,
    /// The conversation cannot go on, in these words. The runner closes the child's stdin.
    Failed(String),
}

/// The client half of one app-server conversation, as a state machine with no I/O.
#[derive(Debug, Clone)]
pub struct AppServerSession {
    opening: AppServerOpening,
    thread_id: Option<String>,
    closed: bool,
}

impl AppServerSession {
    /// A session that has sent nothing yet.
    #[must_use]
    pub fn new(opening: AppServerOpening) -> Self {
        Self {
            opening,
            thread_id: None,
            closed: false,
        }
    }

    /// The thread the app-server opened, once it has.
    #[must_use]
    pub fn thread_id(&self) -> Option<&str> {
        self.thread_id.as_deref()
    }

    /// The first message: `initialize`.
    #[must_use]
    pub fn start(&self) -> Vec<SessionAction> {
        vec![SessionAction::Send(request(
            INITIALIZE_ID,
            "initialize",
            &self.opening.initialize,
        ))]
    }

    /// What one message from the app-server asks for.
    #[must_use]
    pub fn receive(&mut self, message: &Value) -> Vec<SessionAction> {
        let method = message.get("method").and_then(Value::as_str);
        match (message.get("id"), method) {
            (Some(id), Some(method)) => Self::server_request(id, method, message),
            (Some(id), None) => self.response(id, message),
            (None, Some("turn/completed")) if !self.closed => {
                self.closed = true;
                vec![SessionAction::CloseInput]
            }
            _ => Vec::new(),
        }
    }

    fn server_request(id: &Value, method: &str, message: &Value) -> Vec<SessionAction> {
        if method != TOOL_CALL_METHOD {
            // Nothing is approved by default: a request this route does not serve is refused.
            return vec![SessionAction::Send(json!({
                "id": id,
                "error": {
                    "code": METHOD_NOT_FOUND,
                    "message": format!(
                        "metaharness serves only {TOOL_CALL_METHOD} on this route; {method} is \
                         refused"
                    ),
                },
            }))];
        }
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        let key = call_key(id);
        vec![SessionAction::ToolCall {
            line: crate::bridge::dynamic_tool_call_line(&key, &params),
            key,
            request_id: id.clone(),
            tool: dynamic_tool_name(&params),
            arguments: params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({})),
        }]
    }

    fn response(&mut self, id: &Value, message: &Value) -> Vec<SessionAction> {
        let Some(id) = id.as_u64() else {
            return Vec::new();
        };
        if let Some(error) = message.get("error") {
            let method = match id {
                INITIALIZE_ID => "initialize",
                THREAD_START_ID => "thread/start",
                TURN_START_ID => "turn/start",
                _ => return Vec::new(),
            };
            self.closed = true;
            return vec![SessionAction::Failed(format!(
                "the app-server refused {method}: {error}"
            ))];
        }
        match id {
            INITIALIZE_ID => vec![
                SessionAction::Send(json!({"method": "initialized"})),
                SessionAction::Send(request(
                    THREAD_START_ID,
                    "thread/start",
                    &self.opening.thread_start,
                )),
            ],
            THREAD_START_ID => {
                let Some(thread) = message["result"]["thread"]["id"].as_str() else {
                    self.closed = true;
                    return vec![SessionAction::Failed(
                        "the app-server answered thread/start with no thread id".to_owned(),
                    )];
                };
                self.thread_id = Some(thread.to_owned());
                vec![SessionAction::Send(request(
                    TURN_START_ID,
                    "turn/start",
                    &json!({"threadId": thread, "input": self.opening.turn_input}),
                ))]
            }
            _ => Vec::new(),
        }
    }
}

fn request(id: u64, method: &str, params: &Value) -> Value {
    json!({"id": id, "method": method, "params": params})
}

/// The name a decision for the call with this JSON-RPC id is routed under.
#[must_use]
pub fn call_key(request_id: &Value) -> String {
    format!("rpc-{request_id}")
}

/// The tool a call names: `tool`, or `namespace/tool` when the vendor sent a namespace.
///
/// A catalogue entry has no namespace and its name holds no `/`, so a namespaced call never
/// matches one and the server refuses it by name.
#[must_use]
pub fn dynamic_tool_name(params: &Value) -> String {
    let tool = params["tool"].as_str().unwrap_or_default();
    match params["namespace"].as_str() {
        Some(namespace) if !namespace.is_empty() => format!("{namespace}/{tool}"),
        _ => tool.to_owned(),
    }
}

/// The answer to one `item/tool/call` (`app-server-protocol/src/protocol/v2/item.rs:1669-1680`):
/// one text item, and whether the call succeeded.
#[must_use]
pub fn tool_call_answer(request_id: &Value, text: &str, success: bool) -> Value {
    json!({
        "id": request_id,
        "result": {
            "contentItems": [{"type": "inputText", "text": text}],
            "success": success,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opening() -> AppServerOpening {
        AppServerOpening::new(
            Path::new("/scratch/work"),
            Some("a-model"),
            &[DynamicTool {
                name: "fs_read".to_owned(),
                description: "Read a file.".to_owned(),
                input_schema: json!({"type": "object"}),
            }],
            "read the readme",
        )
    }

    fn sent(actions: &[SessionAction]) -> Vec<Value> {
        actions
            .iter()
            .filter_map(|action| match action {
                SessionAction::Send(message) => Some(message.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_session_opens_with_initialize_then_thread_start_then_turn_start() {
        let mut session = AppServerSession::new(opening());
        let first = sent(&session.start());
        assert_eq!(first.len(), 1);
        assert_eq!(first[0]["method"], "initialize");
        assert_eq!(first[0]["id"], INITIALIZE_ID);
        assert!(first[0].get("jsonrpc").is_none(), "the app-server omits it");

        let second = sent(&session.receive(&json!({"id": 1, "result": {"userAgent": "x"}})));
        assert_eq!(second[0], json!({"method": "initialized"}));
        assert_eq!(second[1]["method"], "thread/start");
        assert_eq!(second[1]["params"]["environments"], json!([]));
        assert_eq!(second[1]["params"]["dynamicTools"][0]["name"], "fs_read");

        let third = sent(&session.receive(&json!({"id": 2, "result": {"thread": {"id": "th-1"}}})));
        assert_eq!(third[0]["method"], "turn/start");
        assert_eq!(third[0]["params"]["threadId"], "th-1");
        assert_eq!(third[0]["params"]["input"][0]["text"], "read the readme");
        assert_eq!(session.thread_id(), Some("th-1"));

        assert_eq!(
            session.receive(&json!({"id": 3, "result": {}})),
            [] as [SessionAction; 0]
        );
        assert_eq!(
            session.receive(&json!({"method": "turn/started", "params": {}})),
            [] as [SessionAction; 0]
        );
    }

    #[test]
    fn a_tool_call_becomes_an_action_carrying_the_seams_line() {
        let mut session = AppServerSession::new(opening());
        let params = json!({"threadId": "th-1", "turnId": "tu-1", "callId": "call-1",
            "namespace": null, "tool": "fs_read", "arguments": {"path": "README.md"}});
        let actions =
            session.receive(&json!({"id": 0, "method": "item/tool/call", "params": params}));
        let [
            SessionAction::ToolCall {
                key,
                request_id,
                tool,
                arguments,
                line,
            },
        ] = actions.as_slice()
        else {
            panic!("{actions:?}");
        };
        assert_eq!(key, "rpc-0");
        assert_eq!(request_id, &json!(0));
        assert_eq!(tool, "fs_read");
        assert_eq!(arguments, &json!({"path": "README.md"}));
        assert_eq!(
            line,
            &crate::bridge::dynamic_tool_call_line("rpc-0", &params)
        );
    }

    #[test]
    fn a_namespaced_tool_is_named_with_its_namespace() {
        assert_eq!(
            dynamic_tool_name(&json!({"namespace": "ns", "tool": "fs_read"})),
            "ns/fs_read"
        );
        assert_eq!(
            dynamic_tool_name(&json!({"namespace": null, "tool": "fs_read"})),
            "fs_read"
        );
    }

    #[test]
    fn a_server_request_other_than_a_tool_call_is_refused_with_an_error() {
        let mut session = AppServerSession::new(opening());
        let actions = session.receive(
            &json!({"id": 4, "method": "item/commandExecution/requestApproval", "params": {}}),
        );
        let answer = &sent(&actions)[0];
        assert_eq!(answer["id"], 4);
        assert_eq!(answer["error"]["code"], METHOD_NOT_FOUND);
        assert!(answer.get("result").is_none());
    }

    #[test]
    fn turn_completed_closes_the_input_once() {
        let mut session = AppServerSession::new(opening());
        let done = json!({"method": "turn/completed", "params": {"turn": {"status": "completed"}}});
        assert_eq!(session.receive(&done), [SessionAction::CloseInput]);
        assert_eq!(session.receive(&done), [] as [SessionAction; 0]);
    }

    #[test]
    fn a_refused_request_fails_the_session_naming_it() {
        let mut session = AppServerSession::new(opening());
        let actions =
            session.receive(&json!({"id": 2, "error": {"code": -32600, "message": "no"}}));
        let [SessionAction::Failed(words)] = actions.as_slice() else {
            panic!("{actions:?}");
        };
        assert!(words.contains("thread/start"), "{words}");
    }

    #[test]
    fn the_answer_carries_one_input_text_and_whether_it_succeeded() {
        assert_eq!(
            tool_call_answer(&json!(0), "hello", false),
            json!({"id": 0, "result": {"contentItems": [{"type": "inputText", "text": "hello"}],
                "success": false}})
        );
    }

    #[test]
    fn the_opening_reads_back_as_itself() {
        let opening = opening();
        assert_eq!(
            AppServerOpening::from_value(&opening.to_value()),
            Some(opening)
        );
        assert_eq!(AppServerOpening::from_value(&json!({})), None);
    }
}
