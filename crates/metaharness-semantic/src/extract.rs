//! Conservative syntax-based tool normalization. This module never runs recovered code.
//!
//! Ported from context-lens (MIT, `src/semantic/extract.rs` at `d72b3b4`; the notice is
//! `LICENSE-context-lens` beside this crate's manifest). The behaviour is the source's, with one
//! addition: the wrappers `env`, `timeout`, `xargs`, `sudo`, `nice`, `nohup` and `time` are unwrapped
//! to the program they run, and every command node names its executable, the wrappers it was run
//! through and, for a script, the script's basename (amendment a68).
#![allow(
    clippy::too_many_lines,
    clippy::map_unwrap_or,
    clippy::match_same_arms,
    clippy::single_match_else,
    clippy::redundant_else,
    clippy::needless_pass_by_value,
    clippy::uninlined_format_args,
    reason = "ported from context-lens with its shape kept, so the two stay diffable line by line"
)]
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt::Write as _;
use tree_sitter::{Node as Ast, Parser};

/// The rule version every signature is hashed under, and that a document names.
///
/// Its own name and not the source's `semantic-extract-v2`: the unwraps change what a node says, so
/// a signature computed here must not compare equal to one the source computed.
pub const VERSION: &str = "metaharness-semantic/1";

/// One semantic operation, or one structure node around operations.
///
/// The field set is the source's, plus [`Self::executable`], [`Self::script`] and
/// [`Self::wrappers`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// The index of the enclosing structure node in the same list; always lower than this node's.
    pub parent: Option<usize>,
    /// The semantic kind: `file.read`, `stream.select`, `vcs.status`, `validation.test`, ...; or,
    /// for a container, `sequence`, `pipeline`, `conditional`, `loop`, `parallel` or `wrapper`.
    pub kind: String,
    /// How the kind was recognised: the program, `native`, `redirection`, `malformed_shell`, ...
    pub method: String,
    /// The basename of the argv[0] a shell command runs, after the wrappers are unwrapped.
    /// [`None`] for a native tool, a redirection and a structure node the source adds.
    pub executable: Option<String>,
    /// The basename of the script a command runs: `./x.sh` names `x.sh`, `python3 x.py` names
    /// `x.py`. [`None`] when the command runs no script file.
    pub script: Option<String>,
    /// The wrapper programs the command was run through, outermost first.
    pub wrappers: Vec<String>,
    /// `observed` for the logged invocation itself, `declared` for what its input says it runs.
    pub evidence: String,
    /// `classified`, `partial` or `unsupported`.
    pub resolution: String,
    /// The kind's parameters. Patterns, queries, payloads and argv are hashed, never stored.
    pub params: Value,
    /// The first byte of the node in its source domain (`params.source_domain`).
    pub start: usize,
    /// The byte after the node's last byte in its source domain.
    pub end: usize,
    /// A request signature where the request is fully known, for repeat detection.
    pub signature: Option<String>,
    /// `inspect`, `change`, `verify`, `research`, `wait`, `coordinate` or `unknown`.
    pub activity: String,
    /// Whether this is a structure node rather than an operation.
    pub container: bool,
}

fn hash(s: &str) -> String {
    let mut out = String::with_capacity(64);
    for byte in Sha256::digest(s.as_bytes()) {
        let _ = write!(out, "{byte:02x}");
    }
    out
}
fn text<'a>(node: Ast<'_>, source: &'a str) -> &'a str {
    source.get(node.byte_range()).unwrap_or("")
}
fn children(node: Ast<'_>) -> Vec<Ast<'_>> {
    let mut c = node.walk();
    node.named_children(&mut c).collect()
}
fn activity(kind: &str) -> &'static str {
    if matches!(
        kind,
        "file.read"
            | "file.search"
            | "file.discover"
            | "vcs.status"
            | "vcs.diff"
            | "vcs.inspect"
            | "stream.read"
            | "stream.select"
            | "stream.search"
    ) {
        "inspect"
    } else if kind == "file.modify" || kind.starts_with("vcs.write") {
        "change"
    } else if kind.starts_with("validation.") {
        "verify"
    } else if kind.starts_with("network.") {
        "research"
    } else if matches!(kind, "agent.wait" | "process.poll") {
        "wait"
    } else if kind.starts_with("agent.") {
        "coordinate"
    } else {
        "unknown"
    }
}

struct Extractor {
    nodes: Vec<Node>,
    provider: String,
    depth: usize,
    tools_shadowed: bool,
    stdin_pipe: bool,
}
impl Extractor {
    #[allow(clippy::too_many_arguments)]
    fn add(
        &mut self,
        parent: Option<usize>,
        kind: &str,
        method: &str,
        evidence: &str,
        resolution: &str,
        mut params: Value,
        start: usize,
        end: usize,
        domain: &str,
        container: bool,
        complete: bool,
    ) -> usize {
        let signature = if complete && !container {
            Some(hash(&format!("{VERSION}:{kind}:{}", params)))
        } else {
            None
        };
        if let Some(p) = params.as_object_mut() {
            p.insert("source_domain".into(), json!(domain));
        }
        let index = self.nodes.len();
        self.nodes.push(Node {
            parent,
            kind: kind.into(),
            method: method.into(),
            executable: None,
            script: None,
            wrappers: Vec::new(),
            evidence: evidence.into(),
            resolution: resolution.into(),
            params,
            start,
            end,
            signature,
            activity: activity(kind).into(),
            container,
        });
        index
    }
    fn unresolved(
        &mut self,
        parent: Option<usize>,
        method: &str,
        source: &str,
        start: usize,
        end: usize,
        domain: &str,
    ) {
        self.add(
            parent,
            "other",
            method,
            "declared",
            "unsupported",
            json!({"input_hash":hash(source)}),
            start,
            end,
            domain,
            false,
            false,
        );
    }
    #[allow(clippy::too_many_arguments)]
    fn tool(
        &mut self,
        tool: &str,
        value: &Value,
        raw: &str,
        cwd: Option<&str>,
        parent: Option<usize>,
        evidence: &str,
        start: usize,
        end: usize,
        domain: &str,
    ) {
        if self.depth > 24 {
            self.unresolved(parent, "depth_limit", raw, start, end, domain);
            return;
        }
        self.depth += 1;
        let name = tool.rsplit('.').next().unwrap_or(tool);
        let lower = name.to_ascii_lowercase();
        let lowered_tool = tool.to_ascii_lowercase();
        let obj = value.as_object();
        let get = |key: &str| obj.and_then(|x| x.get(key));
        let string = |key: &str| get(key).and_then(Value::as_str);
        let command = string("cmd").or_else(|| string("command"));
        if matches!(lower.as_str(), "exec" | "multi_tool_use")
            && (value.is_string() || value.is_null() || command.is_none())
        {
            let idx = self.add(
                parent,
                "wrapper",
                "javascript",
                evidence,
                "classified",
                json!({"native_tool":tool}),
                start,
                end,
                domain,
                true,
                false,
            );
            let script = value.as_str().unwrap_or(raw);
            self.javascript(
                script,
                cwd,
                Some(idx),
                &format!("{domain}:node[{idx}]/javascript"),
            );
        } else if matches!(
            lower.as_str(),
            "bash" | "exec_command" | "shell" | "shell_command" | "terminal"
        ) {
            let idx = self.add(
                parent,
                "wrapper",
                "shell",
                evidence,
                if command.is_some() {
                    "classified"
                } else {
                    "partial"
                },
                json!({"native_tool":tool,"cwd":string("workdir").or(cwd),"decoded_argument_pointer":if string("cmd").is_some(){"/cmd"}else{"/command"}}),
                start,
                end,
                domain,
                true,
                false,
            );
            if let Some(command) = command {
                self.bash(
                    command,
                    string("workdir").or(cwd),
                    Some(idx),
                    &format!(
                        "{domain}:node[{idx}]/{}",
                        if string("cmd").is_some() {
                            "cmd"
                        } else {
                            "command"
                        }
                    ),
                );
            } else if let Some(array) = get("command").and_then(Value::as_array) {
                if let Some(args) = array
                    .iter()
                    .map(|a| a.as_str().map(str::to_string))
                    .collect::<Option<Vec<_>>>()
                {
                    if args.len() >= 3
                        && matches!(args[0].as_str(), "bash" | "/bin/bash" | "sh" | "/bin/sh")
                        && matches!(args[1].as_str(), "-c" | "-lc")
                    {
                        self.bash(
                            &args[2],
                            string("workdir").or(cwd),
                            Some(idx),
                            &format!("{domain}:node[{idx}]/command/2"),
                        );
                    } else {
                        self.command(
                            args,
                            string("workdir").or(cwd),
                            Some(idx),
                            start,
                            end,
                            domain,
                            true,
                        );
                    }
                } else {
                    self.unresolved(Some(idx), "dynamic_shell", raw, start, end, domain);
                }
            } else {
                self.unresolved(Some(idx), "missing_command", raw, start, end, domain);
            }
        } else if (lower == "run" && lowered_tool.contains("web")) || lower == "web__run" {
            let idx = self.add(
                parent,
                "wrapper",
                "web_request",
                evidence,
                "classified",
                json!({"native_tool":tool}),
                start,
                end,
                domain,
                true,
                false,
            );
            let mut found = false;
            for (key, kind) in [
                ("search_query", "network.search"),
                ("image_query", "network.search"),
                ("open", "network.fetch"),
                ("click", "network.fetch"),
                ("find", "network.fetch"),
                ("screenshot", "network.fetch"),
            ] {
                if let Some(items) = get(key).and_then(Value::as_array) {
                    for (i, item) in items.iter().enumerate() {
                        found = true;
                        self.add(Some(idx),kind,key,"declared","classified",json!({"request_hash":hash(&item.to_string()),"argument_pointer":format!("/{key}/{i}")}),start,end,domain,false,false);
                    }
                }
            }
            if !found {
                self.unresolved(
                    Some(idx),
                    "unsupported_web_operation",
                    raw,
                    start,
                    end,
                    domain,
                );
            }
        } else {
            let mut params = json!({});
            let (kind, method, mut complete) = match lower.as_str() {
                "read" | "read_file" => {
                    let path = string("file_path").or_else(|| string("path"));
                    params = json!({"path":path.map(|p|lexical_path(p,cwd)),"offset":get("offset"),"limit":get("limit"),"selector":"native_lines","output_mode":"native_rendered","native_defaults":self.provider});
                    ("file.read", "native", false)
                }
                "grep" => {
                    params = json!({"pattern_hash":string("pattern").map(hash),"path":string("path").map(|p|lexical_path(p,cwd)),"glob":string("glob"),"output_mode":string("output_mode").unwrap_or("native_default"),"options_hash":hash(raw)});
                    ("file.search", "native", false)
                }
                "glob" => {
                    params = json!({"pattern_hash":string("pattern").map(hash),"path":string("path").map(|p|lexical_path(p,cwd)),"output_mode":"paths","native_defaults":self.provider});
                    ("file.discover", "native_glob", false)
                }
                "edit" | "write" | "apply_patch" | "multiedit" => {
                    params = json!({"path":string("file_path").or_else(||string("path")).map(|p|lexical_path(p,cwd)),"payload_hash":hash(raw),"mode":if lower=="write"{"rewrite"}else if lower=="edit"{"replacement"}else{"patch"}});
                    ("file.modify", lower.as_str(), false)
                }
                "write_stdin" => {
                    let chars = string("chars").unwrap_or("");
                    params = json!({"process_id":get("session_id"),"input_hash":hash(chars),"empty":chars.is_empty()});
                    (
                        if chars.is_empty() {
                            "process.poll"
                        } else if chars == "\u{3}" {
                            "process.interrupt"
                        } else {
                            "process.input"
                        },
                        "native",
                        false,
                    )
                }
                "wait" => {
                    params = json!({"process_id":get("cell_id"),"terminate":get("terminate")});
                    (
                        if get("terminate") == Some(&json!(true)) {
                            "process.interrupt"
                        } else {
                            "process.poll"
                        },
                        "native",
                        false,
                    )
                }
                "spawn_agent" | "task" | "agent" => {
                    params = json!({"payload_hash":hash(raw),"agent_type":string("subagent_type"),"model":string("model")});
                    ("agent.spawn", "native", false)
                }
                "send_message" | "sendmessage" | "followup_task" => {
                    params = json!({"agent_id":get("target").or_else(||get("recipient")),"payload_hash":hash(raw)});
                    ("agent.message", "native", false)
                }
                "wait_agent" => {
                    params = json!({"agent_id":get("target").or_else(||get("task_id"))});
                    ("agent.wait", "native", false)
                }
                "list_agents" => ("agent.list", "native", false),
                "interrupt_agent" => {
                    params = json!({"agent_id":get("target").or_else(||get("task_id"))});
                    ("agent.interrupt", "native", false)
                }
                "websearch" | "web_search" | "search_query" => {
                    params = json!({"query_hash":string("query").or_else(||string("q")).map(hash)});
                    ("network.search", "native", false)
                }
                "webfetch" | "web_fetch" => {
                    params = json!({"url_hash":string("url").map(hash)});
                    ("network.fetch", "native", false)
                }
                "taskoutput" | "taskstop" => {
                    params =
                        json!({"target_id":get("task_id"),"target_type":"unknown_background_task"});
                    (
                        if lower == "taskstop" {
                            "process.interrupt"
                        } else {
                            "process.poll"
                        },
                        "native",
                        false,
                    )
                }
                "sleep" => {
                    params = json!({"duration_ms":get("duration_ms")});
                    ("process.poll", "sleep", false)
                }
                _ => {
                    params = json!({"native_tool":tool,"input_hash":hash(raw)});
                    ("other", "native", false)
                }
            };
            if value.is_null() {
                complete = false;
            }
            self.add(
                parent,
                kind,
                method,
                evidence,
                if kind == "other" {
                    "unsupported"
                } else {
                    "classified"
                },
                params,
                start,
                end,
                domain,
                false,
                complete,
            );
        }
        self.depth -= 1;
    }

    fn bash(&mut self, source: &str, cwd: Option<&str>, parent: Option<usize>, domain: &str) {
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_bash::LANGUAGE.into())
            .is_err()
        {
            return;
        }
        if let Some(tree) = parser.parse(source, None) {
            if tree.root_node().has_error() {
                self.unresolved(parent, "malformed_shell", source, 0, source.len(), domain);
                return;
            }
            self.bash_node(tree.root_node(), source, cwd, parent, domain);
        }
    }
    fn bash_node(
        &mut self,
        node: Ast<'_>,
        source: &str,
        cwd: Option<&str>,
        parent: Option<usize>,
        domain: &str,
    ) {
        match node.kind() {
            "command" => {
                let Some(name) = node.child_by_field_name("name") else {
                    return;
                };
                let mut parts = vec![shell_literal(name, source)];
                let mut cur = node.walk();
                parts.extend(
                    node.children_by_field_name("argument", &mut cur)
                        .map(|x| shell_literal(x, source)),
                );
                let static_args = parts.into_iter().collect::<Option<Vec<_>>>();
                let extra = children(node).iter().any(|n| {
                    matches!(
                        n.kind(),
                        "variable_assignment" | "file_redirect" | "herestring_redirect"
                    )
                });
                let command_parent = if extra {
                    Some(self.add(
                        parent,
                        "sequence",
                        "command_with_environment_or_redirect",
                        "declared",
                        "partial",
                        json!({}),
                        node.start_byte(),
                        node.end_byte(),
                        domain,
                        true,
                        false,
                    ))
                } else {
                    parent
                };
                match static_args {
                    Some(args) => self.command(
                        args,
                        cwd,
                        command_parent,
                        node.start_byte(),
                        node.end_byte(),
                        domain,
                        !extra,
                    ),
                    None => {
                        let mut params =
                            json!({"input_hash":hash(text(node,source)),"dynamic":true});
                        let name = shell_literal(name, source).unwrap_or_default();
                        params["executable"] = json!(name);
                        let index = self.add(
                            command_parent,
                            "process.run",
                            "dynamic_shell",
                            "declared",
                            "partial",
                            params,
                            node.start_byte(),
                            node.end_byte(),
                            domain,
                            false,
                            false,
                        );
                        let executable = basename(&name);
                        if !executable.is_empty() {
                            self.nodes[index].executable = Some(executable.to_owned());
                        }
                    }
                }
                for child in children(node) {
                    if child.kind() == "file_redirect" {
                        self.redirect(child, source, cwd, command_parent, domain);
                    }
                }
            }
            "file_redirect" => self.redirect(node, source, cwd, parent, domain),
            "comment" | "variable_assignment" => {}
            kind => {
                let container = match kind {
                    "program" => "sequence",
                    "pipeline" => "pipeline",
                    "list" | "if_statement" | "case_statement" => "conditional",
                    "for_statement" | "while_statement" | "c_style_for_statement" => "loop",
                    "subshell" | "compound_statement" | "redirected_statement" => "sequence",
                    "function_definition" => "wrapper",
                    _ => "",
                };
                let kids = children(node);
                let next = if !container.is_empty() && (kids.len() > 1 || container != "sequence") {
                    Some(self.add(
                        parent,
                        container,
                        kind,
                        "declared",
                        if kind == "function_definition" {
                            "partial"
                        } else {
                            "classified"
                        },
                        json!({"execution_count":"unknown"}),
                        node.start_byte(),
                        node.end_byte(),
                        domain,
                        true,
                        false,
                    ))
                } else {
                    parent
                };
                let child_start = self.nodes.len();
                let mut current_cwd = if matches!(container, "loop" | "wrapper") {
                    None
                } else {
                    cwd.map(str::to_owned)
                };
                for (position, child) in kids.into_iter().enumerate() {
                    let previous_stdin = self.stdin_pipe;
                    if container == "pipeline" {
                        self.stdin_pipe = position > 0;
                    }
                    self.bash_node(child, source, current_cwd.as_deref(), next, domain);
                    self.stdin_pipe = previous_stdin;
                    // A directory-changing statement makes later paths uncertain. No execution assumptions.
                    if contains_command(child, source, "cd") {
                        current_cwd = None;
                    }
                }
                if container == "pipeline"
                    && let Some(index) = next
                {
                    let leaves: Vec<&Node> = self.nodes[child_start..]
                        .iter()
                        .filter(|n| !n.container)
                        .collect();
                    if leaves.len() == 2
                        && leaves[0].kind == "file.read"
                        && leaves[0].params["selector"] == "whole"
                        && leaves[0].params["paths"]
                            .as_array()
                            .is_some_and(|v| v.len() == 1)
                        && leaves[1].kind == "stream.select"
                        && leaves[1].params["input"] == "stdin"
                    {
                        let effective = json!({"paths":leaves[0].params["paths"],"selector":leaves[1].params["selector"],"count":leaves[1].params["count"],"first":leaves[1].params["first"],"last":leaves[1].params["last"],"output_mode":"raw_bytes","evidence":"syntactic_composition"});
                        self.nodes[index].params["effective_read"] = effective;
                    }
                }
            }
        }
    }
    fn redirect(
        &mut self,
        node: Ast<'_>,
        source: &str,
        cwd: Option<&str>,
        parent: Option<usize>,
        domain: &str,
    ) {
        let mut cursor = node.walk();
        let operator = node
            .children(&mut cursor)
            .find(|n| {
                !n.is_named()
                    && matches!(
                        n.kind(),
                        ">" | ">>" | "<" | "<>" | ">&" | "<&" | ">|" | "&>" | "&>>"
                    )
            })
            .map(|n| n.kind())
            .unwrap_or("unknown");
        let mut c = node.walk();
        let targets: Vec<Value> = node
            .children_by_field_name("destination", &mut c)
            .map(|n| {
                shell_literal(n, source)
                    .map(|s| json!(lexical_path(&s, cwd)))
                    .unwrap_or(Value::Null)
            })
            .collect();
        let write = matches!(operator, ">" | ">>" | ">|" | "&>" | "&>>" | "<>");
        let descriptor = matches!(operator, ">&" | "<&");
        if !targets.is_empty() && targets.iter().all(|p| p == "/dev/null") {
            self.add(parent,"stream.redirect","null_device","declared","classified",json!({"paths":targets,"operator":operator,"mode":if write || operator==">&" {"discard"}else{"empty_input"},"descriptor":node.child_by_field_name("descriptor").map(|n|text(n,source))}),node.start_byte(),node.end_byte(),domain,true,false);
            return;
        }
        self.add(parent,if descriptor{"process.run"}else if write{"file.modify"}else{"file.read"},if descriptor{"descriptor_redirect"}else{"redirection"},"declared","partial",json!({"paths":if descriptor{vec![]}else{targets},"operator":operator,"mode":if descriptor{"descriptor"}else if matches!(operator,">>"|"&>>"){"append"}else if write{"overwrite"}else{"stdin"},"descriptor":node.child_by_field_name("descriptor").map(|n|text(n,source))}),node.start_byte(),node.end_byte(),domain,false,false);
    }

    #[allow(clippy::too_many_arguments)]
    fn command(
        &mut self,
        args: Vec<String>,
        cwd: Option<&str>,
        parent: Option<usize>,
        start: usize,
        end: usize,
        domain: &str,
        allow_signature: bool,
    ) {
        if args.is_empty() {
            return;
        }
        // The addition over the source: a wrapper is unwrapped to the program it runs. Every
        // decision below is then the source's, made over the program's own argv.
        let unwrapped = unwrap(&args);
        let wrappers = unwrapped.wrappers;
        let operands_from_stdin = unwrapped.xargs;
        let cwd = if unwrapped.cwd_changed { None } else { cwd };
        let args = args[unwrapped.skip..].to_vec();
        let exe = basename(&args[0]);
        let a = &args[1..];
        let mut params = json!({"executable":exe,"argv_hash":hash(&serde_json::to_string(&args).unwrap_or_default()),"cwd":cwd});
        let mut complete = false;
        let (kind, method) = match exe {
            "cat" => {
                let operands = if a.first().map(String::as_str) == Some("--") {
                    &a[1..]
                } else {
                    a
                };
                let only_stdin = operands.is_empty() || operands.iter().all(|p| p == "-");
                let simple = !operands.is_empty() && operands.iter().all(|p| !p.starts_with('-'));
                if only_stdin {
                    params = json!({"input":"stdin","selector":"whole","output_mode":"raw_bytes"});
                } else if simple {
                    params = json!({"paths":operands.iter().map(|p|lexical_path(p,cwd)).collect::<Vec<_>>(),"selector":"whole","output_mode":"raw_bytes"});
                    complete = operands.iter().all(|p| path_known(p, cwd));
                }
                (
                    if only_stdin {
                        "stream.read"
                    } else {
                        "file.read"
                    },
                    "cat",
                )
            }
            "head" | "tail" => {
                let (n, files) = if a.len() >= 2 && a[0] == "-n" {
                    (a[1].parse::<u64>().ok(), &a[2..])
                } else if let Some(count) = a
                    .first()
                    .and_then(|v| v.strip_prefix("-n"))
                    .filter(|v| !v.is_empty())
                {
                    (count.parse::<u64>().ok(), &a[1..])
                } else if let Some(count) = a
                    .first()
                    .and_then(|v| v.strip_prefix('-'))
                    .filter(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_digit()))
                {
                    (count.parse::<u64>().ok(), &a[1..])
                } else {
                    (Some(10), a)
                };
                let stdin = n.is_some() && (files.is_empty() || files == ["-"]);
                let simple = n.is_some() && files.len() == 1 && !files[0].starts_with('-');
                if stdin {
                    params = json!({"input":"stdin","selector":if exe=="head"{"first_lines"}else{"last_lines"},"count":n,"output_mode":"raw_bytes"});
                }
                if simple {
                    params = json!({"paths":[lexical_path(&files[0],cwd)],"selector":if exe=="head"{"first_lines"}else{"last_lines"},"count":n,"output_mode":"raw_bytes"});
                    complete = path_known(&files[0], cwd);
                }
                (if stdin { "stream.select" } else { "file.read" }, exe)
            }
            "sed" => {
                let mut stdin = false;
                if matches!(a.len(), 2 | 3)
                    && a[0] == "-n"
                    && let Some((first, last)) = sed_range(&a[1])
                {
                    stdin = a.len() == 2 || a[2] == "-";
                    params = json!({"selector":"line_range","first":first,"last":last,"output_mode":"raw_bytes"});
                    if stdin {
                        params["input"] = json!("stdin");
                    } else {
                        params["paths"] = json!([lexical_path(&a[2], cwd)]);
                        complete = path_known(&a[2], cwd) && !a[2].starts_with('-');
                    }
                }
                if a.iter().any(|s| s == "-i" || s.starts_with("-i")) {
                    ("file.modify", "sed_in_place")
                } else if stdin {
                    ("stream.select", "sed_selector")
                } else if a.len() == 1 && !a[0].starts_with('-') {
                    params["input"] = json!("stdin");
                    ("stream.select", "sed_program")
                } else {
                    ("file.read", "sed_selector")
                }
            }
            "rg" | "grep" => {
                let discovery = exe == "rg" && a.iter().any(|s| s == "--files");
                let mut flags = Vec::new();
                let mut patterns = Vec::new();
                let mut paths = Vec::new();
                let mut explicit_stdin = false;
                let mut supported = true;
                let mut positional = false;
                let mut i = 0;
                while i < a.len() {
                    let arg = &a[i];
                    if !positional && arg == "--" {
                        positional = true;
                        i += 1;
                        continue;
                    }
                    if !positional
                        && matches!(
                            arg.as_str(),
                            "-e" | "--regexp" | "-g" | "--glob" | "-t" | "--type"
                        )
                    {
                        if let Some(value) = a.get(i + 1) {
                            if arg == "-e" || arg == "--regexp" {
                                patterns.push(hash(value));
                            } else {
                                flags.push(format!("{arg}:{}", hash(value)));
                            }
                            i += 2;
                            continue;
                        } else {
                            supported = false;
                            break;
                        }
                    }
                    if !positional && arg.starts_with('-') && arg != "-" {
                        if matches!(
                            arg.as_str(),
                            "--files"
                                | "-n"
                                | "--line-number"
                                | "-l"
                                | "--files-with-matches"
                                | "-c"
                                | "--count"
                                | "-i"
                                | "--ignore-case"
                                | "-F"
                                | "--fixed-strings"
                                | "--hidden"
                                | "--no-ignore"
                                | "-r"
                                | "-R"
                                | "-H"
                                | "--no-heading"
                                | "--json"
                        ) {
                            flags.push(arg.clone());
                        } else {
                            supported = false;
                        }
                    } else if !discovery && patterns.is_empty() {
                        patterns.push(hash(arg));
                    } else if arg == "-" {
                        explicit_stdin = true;
                    } else {
                        paths.push(lexical_path(arg, cwd));
                    }
                    i += 1;
                }
                let mode = if flags.iter().any(|f| f == "--json") {
                    "json"
                } else if discovery
                    || flags
                        .iter()
                        .any(|f| f == "-l" || f == "--files-with-matches")
                {
                    "paths"
                } else if flags.iter().any(|f| f == "-c" || f == "--count") {
                    "count"
                } else {
                    "matching_lines"
                };
                let recursive = flags.iter().any(|f| matches!(f.as_str(), "-r" | "-R"));
                let stdin_only = !discovery
                    && paths.is_empty()
                    && (explicit_stdin || (!recursive && (exe == "grep" || self.stdin_pipe)));
                let input = if stdin_only {
                    "stdin"
                } else if explicit_stdin {
                    "mixed_files_and_stdin"
                } else {
                    "files"
                };
                params = json!({"engine":exe,"patterns_hash":patterns,"paths":paths,"flags":flags,"output_mode":mode,"cwd":cwd,"supported_flags":supported,"input":input});
                // Native defaults, config and ignore files influence these requests; do not promise equality.
                (
                    if discovery {
                        "file.discover"
                    } else if stdin_only {
                        "stream.search"
                    } else {
                        "file.search"
                    },
                    exe,
                )
            }
            "find" | "ls" => {
                let roots: Vec<String> = a
                    .iter()
                    .take_while(|arg| !arg.starts_with('-') && *arg != "(" && *arg != "!")
                    .map(|p| lexical_path(p, cwd))
                    .collect();
                params["paths"] = json!(roots);
                params["options_hash"] = json!(hash(&serde_json::to_string(a).unwrap_or_default()));
                params["output_mode"] = json!(if exe == "ls" {
                    "directory_listing"
                } else {
                    "paths"
                });
                (
                    if exe == "find" && a.iter().any(|v| v == "-delete") {
                        "file.modify"
                    } else {
                        "file.discover"
                    },
                    exe,
                )
            }
            "git" => {
                let sub = a.first().map(String::as_str).unwrap_or("");
                params["subcommand"] = json!(sub);
                match sub {
                    "status" => ("vcs.status", "git"),
                    "diff" => ("vcs.diff", "git"),
                    "branch"
                        if a.len() > 1
                            && !a[1..].iter().all(|v| {
                                matches!(
                                    v.as_str(),
                                    "-a" | "-r"
                                        | "--all"
                                        | "--remotes"
                                        | "--show-current"
                                        | "--list"
                                        | "-v"
                                        | "-vv"
                                )
                            }) =>
                    {
                        ("vcs.write", "git")
                    }
                    "log" | "show" | "rev-parse" | "branch" => ("vcs.inspect", "git"),
                    "add" | "commit" | "merge" | "rebase" | "reset" | "checkout" | "switch"
                    | "restore" | "cherry-pick" | "push" | "pull" => ("vcs.write", "git"),
                    _ => ("vcs.other", "git"),
                }
            }
            "cargo" => {
                let sub = a
                    .iter()
                    .find(|v| !v.starts_with('+') && !v.starts_with('-'))
                    .map(String::as_str)
                    .unwrap_or("");
                params["runner"] = json!("cargo");
                params["subcommand"] = json!(sub);
                params["options"] = json!(
                    a.iter()
                        .filter(|v| matches!(
                            v.as_str(),
                            "--release"
                                | "--all"
                                | "--all-targets"
                                | "--workspace"
                                | "--lib"
                                | "--bins"
                                | "--tests"
                                | "--doc"
                                | "--offline"
                                | "--locked"
                                | "--nocapture"
                                | "--check"
                        ))
                        .collect::<Vec<_>>()
                );
                params["target_hashes"] = json!(
                    a.windows(2)
                        .filter(|w| matches!(
                            w[0].as_str(),
                            "--test" | "--bin" | "-p" | "--package"
                        ))
                        .map(|w| hash(&w[1]))
                        .collect::<Vec<_>>()
                );
                match sub {
                    "test" | "nextest" => ("validation.test", "cargo"),
                    "build" | "check" => ("validation.build", "cargo"),
                    "clippy" => ("validation.lint", "cargo"),
                    "fmt" if a.iter().any(|v| v == "--check") => ("validation.format", "cargo"),
                    "fmt" => ("file.modify", "format"),
                    _ => ("process.run", "cargo"),
                }
            }
            "npm" | "pnpm" | "yarn" | "bun" => {
                let sub = if a.first().map(String::as_str) == Some("run") {
                    a.get(1)
                } else {
                    a.first()
                }
                .map(String::as_str)
                .unwrap_or("");
                params["runner"] = json!(exe);
                params["script_name"] = json!(sub);
                match sub {
                    "test" => ("validation.test", "declared_script"),
                    "build" => ("validation.build", "declared_script"),
                    "lint" => ("validation.lint", "declared_script"),
                    _ => ("process.run", "package_script"),
                }
            }
            "pytest" | "jest" | "vitest" | "mocha" => {
                params["runner"] = json!(exe);
                ("validation.test", exe)
            }
            "ruff" | "eslint" | "shellcheck" => {
                if a.iter()
                    .any(|v| matches!(v.as_str(), "--fix" | "--fix-only"))
                    || (exe == "ruff"
                        && a.first().map(String::as_str) == Some("format")
                        && !a.iter().any(|v| v == "--check"))
                {
                    ("file.modify", exe)
                } else if exe == "ruff" && a.first().map(String::as_str) == Some("format") {
                    ("validation.format", exe)
                } else {
                    ("validation.lint", exe)
                }
            }
            "curl" | "wget" => {
                params["method"] = json!("unknown");
                ("network.fetch", exe)
            }
            "sleep" => {
                params["seconds"] = a
                    .first()
                    .and_then(|v| v.parse::<f64>().ok())
                    .map(|v| json!(v))
                    .unwrap_or(Value::Null);
                ("process.poll", "sleep")
            }
            "kill" | "pkill" => ("process.interrupt", exe),
            "python" | "python3" | "node" | "ruby" | "perl" => ("process.run", "opaque_program"),
            "bash" | "sh" | "zsh" if a.len() == 2 && matches!(a[0].as_str(), "-c" | "-lc") => {
                let idx = self.add(
                    parent,
                    "wrapper",
                    "shell",
                    "declared",
                    "classified",
                    json!({"executable":exe}),
                    start,
                    end,
                    domain,
                    true,
                    false,
                );
                self.nodes[idx].executable = Some(exe.to_owned());
                self.nodes[idx].wrappers = wrappers;
                if self.depth < 24 {
                    self.depth += 1;
                    self.bash(&a[1], cwd, Some(idx), &format!("{domain}:shell_argument"));
                    self.depth -= 1;
                }
                return;
            }
            _ => ("process.run", "shell_command"),
        };
        // `xargs` appends operands it reads from its standard input: the program reads those
        // files, not its own standard input, and which files they are is not in the command.
        let kind = if operands_from_stdin {
            let kind = match kind {
                "stream.read" | "stream.select" => "file.read",
                "stream.search" => "file.search",
                other => other,
            };
            if kind.starts_with("file.") {
                params["input"] = json!("xargs_operands");
            }
            complete = false;
            kind
        } else {
            kind
        };
        let resolution = if complete {
            "classified"
        } else if matches!(method, "opaque_program" | "shell_command") {
            "unsupported"
        } else {
            "partial"
        };
        // A wrapper can change what the program reads and as whom; a wrapped request is never
        // asserted equal to an unwrapped one.
        let signed = complete && allow_signature && args[0] == exe && wrappers.is_empty();
        let index = self.add(
            parent, kind, method, "declared", resolution, params, start, end, domain, false, signed,
        );
        self.nodes[index].executable = Some(exe.to_owned());
        self.nodes[index].script = script_of(&args);
        self.nodes[index].wrappers = wrappers;
    }

    fn javascript(&mut self, source: &str, cwd: Option<&str>, parent: Option<usize>, domain: &str) {
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_javascript::LANGUAGE.into())
            .is_err()
        {
            return;
        }
        if let Some(tree) = parser.parse(source, None) {
            if tree.root_node().has_error() {
                self.unresolved(
                    parent,
                    "malformed_javascript",
                    source,
                    0,
                    source.len(),
                    domain,
                );
                return;
            }
            let previous_shadow = self.tools_shadowed;
            self.tools_shadowed = shadows_tools(tree.root_node(), source);
            let mut bindings = HashMap::new();
            self.js_node(tree.root_node(), source, cwd, parent, domain, &mut bindings);
            self.tools_shadowed = previous_shadow;
        }
    }
    fn js_node(
        &mut self,
        node: Ast<'_>,
        source: &str,
        cwd: Option<&str>,
        parent: Option<usize>,
        domain: &str,
        bindings: &mut HashMap<String, Value>,
    ) {
        if matches!(node.kind(), "program" | "statement_block") {
            for child in children(node) {
                if matches!(child.kind(), "lexical_declaration" | "variable_declaration") {
                    for declaration in children(child) {
                        if let Some(name) = declaration.child_by_field_name("name") {
                            bindings.remove(text(name, source));
                        }
                    }
                }
            }
        }
        if matches!(node.kind(), "lexical_declaration" | "variable_declaration") {
            for child in children(node) {
                if let Some(name) = child.child_by_field_name("name") {
                    bindings.remove(text(name, source));
                }
            }
            if text(node, source).trim_start().starts_with("const ") {
                for child in children(node) {
                    if child.kind() == "variable_declarator"
                        && let (Some(name), Some(value)) = (
                            child.child_by_field_name("name"),
                            child.child_by_field_name("value"),
                        )
                        && name.kind() == "identifier"
                        && let Some(v) = js_value(value, source, bindings)
                    {
                        bindings.insert(text(name, source).into(), v);
                    }
                }
            }
            for child in children(node) {
                self.js_node(child, source, cwd, parent, domain, bindings);
            }
            return;
        }
        if node.kind() == "assignment_expression"
            || node.kind() == "augmented_assignment_expression"
            || node.kind() == "update_expression"
        {
            let left = node
                .child_by_field_name("left")
                .or_else(|| node.child_by_field_name("argument"));
            if let Some(left) = left {
                let root = text(left, source).split(['.', '[']).next().unwrap_or("");
                if left.kind() == "identifier" {
                    bindings.remove(root);
                } else {
                    bindings.clear();
                }
            }
        }
        if node.kind() == "call_expression" {
            let function = node
                .child_by_field_name("function")
                .map(|n| text(n, source))
                .unwrap_or("");
            if matches!(function, "Promise.all" | "Promise.allSettled") {
                let idx = self.add(
                    parent,
                    "parallel",
                    function,
                    "declared",
                    "classified",
                    json!({"execution_count":"unknown"}),
                    node.start_byte(),
                    node.end_byte(),
                    domain,
                    true,
                    false,
                );
                for child in children(node) {
                    self.js_node(child, source, cwd, Some(idx), domain, bindings);
                }
                return;
            }
            if self.tools_shadowed
                && (function.starts_with("tools.") || function.starts_with("tools["))
            {
                self.unresolved(
                    parent,
                    "shadowed_tools_binding",
                    text(node, source),
                    node.start_byte(),
                    node.end_byte(),
                    domain,
                );
                return;
            }
            if function.starts_with("tools.")
                && function[6..]
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                let arguments = node.child_by_field_name("arguments");
                if arguments.is_some_and(|n| n.named_child_count() != 1) {
                    self.unresolved(
                        parent,
                        "unsupported_tool_arity",
                        text(node, source),
                        node.start_byte(),
                        node.end_byte(),
                        domain,
                    );
                    return;
                }
                let argument = arguments.and_then(|n| n.named_child(0));
                if let Some(argument) = argument
                    && let Some(value) = js_value(argument, source, bindings)
                {
                    self.tool(
                        &function[6..],
                        &value,
                        text(argument, source),
                        cwd,
                        parent,
                        "declared",
                        node.start_byte(),
                        node.end_byte(),
                        domain,
                    );
                    return;
                }
                self.add(
                    parent,
                    "other",
                    "dynamic_tool_call",
                    "declared",
                    "partial",
                    json!({"native_tool":&function[6..],"input_hash":hash(text(node,source))}),
                    node.start_byte(),
                    node.end_byte(),
                    domain,
                    false,
                    false,
                );
                return;
            }
            if function.starts_with("tools[") {
                self.unresolved(
                    parent,
                    "dynamic_tool_name",
                    text(node, source),
                    node.start_byte(),
                    node.end_byte(),
                    domain,
                );
                return;
            }
            // Arbitrary function calls can mutate objects through aliases. Do not substitute stale values.
            if !matches!(
                function,
                "text" | "image" | "console.log" | "JSON.stringify"
            ) {
                bindings.clear();
            }
        }
        let container = match node.kind() {
            "if_statement" | "switch_statement" | "ternary_expression" => Some("conditional"),
            "for_statement" | "for_in_statement" | "while_statement" | "do_statement" => {
                Some("loop")
            }
            "arrow_function" | "function_declaration" | "function_expression" => Some("wrapper"),
            _ => None,
        };
        let next = container
            .map(|kind| {
                self.add(
                    parent,
                    kind,
                    node.kind(),
                    "declared",
                    "partial",
                    json!({"execution_count":"unknown"}),
                    node.start_byte(),
                    node.end_byte(),
                    domain,
                    true,
                    false,
                )
            })
            .or(parent);
        if container.is_some() || node.kind() == "statement_block" {
            let mut local = if container == Some("wrapper") {
                HashMap::new()
            } else {
                bindings.clone()
            };
            for child in children(node) {
                self.js_node(child, source, cwd, next, domain, &mut local);
            }
            // A branch or nested scope can write outer bindings; retaining them would assume execution.
            if contains_js_mutation(node) {
                bindings.clear();
            }
        } else {
            for child in children(node) {
                self.js_node(child, source, cwd, next, domain, bindings);
            }
        }
    }
}

/// Input is serialized native arguments, or the original custom-tool script.
/// `source_domain` names the decoded string domain for every AST range.
#[must_use]
pub fn extract(provider: &str, tool: &str, input: &str, cwd: Option<&str>) -> Vec<Node> {
    if input.len() > 2 * 1024 * 1024 {
        return vec![Node {
            parent: None,
            kind: "other".into(),
            method: "input_size_limit".into(),
            executable: None,
            script: None,
            wrappers: Vec::new(),
            evidence: "observed".into(),
            resolution: "unsupported".into(),
            params: json!({"native_tool":tool,"input_hash":hash(input),"source_domain":"input"}),
            start: 0,
            end: input.len(),
            signature: None,
            activity: "unknown".into(),
            container: false,
        }];
    }
    let value =
        serde_json::from_str::<Value>(input).unwrap_or_else(|_| Value::String(input.into()));
    let mut extractor = Extractor {
        nodes: Vec::new(),
        provider: provider.into(),
        depth: 0,
        tools_shadowed: false,
        stdin_pipe: false,
    };
    extractor.tool(
        tool,
        &value,
        input,
        cwd,
        None,
        "observed",
        0,
        input.len(),
        "input",
    );
    extractor.nodes
}

fn path_known(path: &str, cwd: Option<&str>) -> bool {
    !path.split('/').any(|part| part == "..")
        && !cwd.is_some_and(|s| s.split('/').any(|part| part == ".."))
        && (path.starts_with('/') || cwd.is_some_and(|s| s.starts_with('/')))
}
fn lexical_path(path: &str, cwd: Option<&str>) -> String {
    let combined = if path.starts_with('/') {
        path.to_owned()
    } else if let Some(cwd) = cwd {
        format!("{cwd}/{path}")
    } else {
        path.to_owned()
    };
    let absolute = combined.starts_with('/');
    let mut stack = Vec::new();
    for part in combined.split('/') {
        match part {
            "" | "." => {}
            ".." if stack.last().is_some_and(|s| *s != "..") => {
                stack.pop();
            }
            ".." if !absolute => stack.push(".."),
            ".." => {}
            _ => stack.push(part),
        }
    }
    format!("{}{}", if absolute { "/" } else { "" }, stack.join("/"))
}
/// The last `/`-separated component, as the source names an executable.
fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// What [`unwrap`] found in front of the program: how many argv entries to skip, the wrappers in
/// order, whether `xargs` was among them, and whether one of them changed the directory.
struct Unwrapped {
    skip: usize,
    wrappers: Vec<String>,
    xargs: bool,
    cwd_changed: bool,
}

/// One wrapper's command line: the options it takes, before the program it runs.
struct Grammar {
    /// Options that take no value.
    flags: &'static [&'static str],
    /// Options that take one value, as the next argument or attached (`-n10`, `--signal=KILL`).
    valued: &'static [&'static str],
    /// The valued options that change the working directory.
    chdir: &'static [&'static str],
    /// Whether `NAME=VALUE` arguments may stand before the program (`env`, `sudo`).
    assignments: bool,
    /// How many positional arguments stand before the program (`timeout DURATION`).
    positional: usize,
    /// Whether `-N` is a value of its own (`nice -5`).
    numeric: bool,
}

fn grammar(wrapper: &str) -> Option<Grammar> {
    let grammar = match wrapper {
        "env" => Grammar {
            flags: &[
                "-",
                "-i",
                "--ignore-environment",
                "-0",
                "--null",
                "-v",
                "--debug",
            ],
            valued: &["-u", "--unset", "-C", "--chdir"],
            chdir: &["-C", "--chdir"],
            assignments: true,
            positional: 0,
            numeric: false,
        },
        "timeout" => Grammar {
            flags: &["--preserve-status", "--foreground", "-v", "--verbose"],
            valued: &["-s", "--signal", "-k", "--kill-after"],
            chdir: &[],
            assignments: false,
            positional: 1,
            numeric: false,
        },
        "xargs" => Grammar {
            flags: &[
                "-0",
                "--null",
                "-o",
                "--open-tty",
                "-p",
                "--interactive",
                "-r",
                "--no-run-if-empty",
                "-t",
                "--verbose",
                "-x",
                "--exit",
                "-i",
                "--replace",
                "-e",
                "--eof",
                "-l",
            ],
            valued: &[
                "-a",
                "--arg-file",
                "-d",
                "--delimiter",
                "-E",
                "-I",
                "-L",
                "--max-lines",
                "-n",
                "--max-args",
                "-P",
                "--max-procs",
                "-s",
                "--max-chars",
                "--process-slot-var",
            ],
            chdir: &[],
            assignments: false,
            positional: 0,
            numeric: false,
        },
        "sudo" => Grammar {
            flags: &[
                "-A",
                "--askpass",
                "-b",
                "--background",
                "-E",
                "--preserve-env",
                "-H",
                "--set-home",
                "-i",
                "--login",
                "-k",
                "--reset-timestamp",
                "-n",
                "--non-interactive",
                "-P",
                "--preserve-groups",
                "-S",
                "--stdin",
                "-s",
                "--shell",
            ],
            valued: &[
                "-u",
                "--user",
                "-g",
                "--group",
                "-p",
                "--prompt",
                "-C",
                "--close-from",
                "-D",
                "--chdir",
                "-R",
                "--chroot",
                "-r",
                "--role",
                "-t",
                "--type",
                "-T",
                "--command-timeout",
                "-U",
                "--other-user",
            ],
            chdir: &["-D", "--chdir", "-R", "--chroot"],
            assignments: true,
            positional: 0,
            numeric: false,
        },
        "nice" => Grammar {
            flags: &[],
            valued: &["-n", "--adjustment"],
            chdir: &[],
            assignments: false,
            positional: 0,
            numeric: true,
        },
        "nohup" => Grammar {
            flags: &[],
            valued: &[],
            chdir: &[],
            assignments: false,
            positional: 0,
            numeric: false,
        },
        "time" => Grammar {
            flags: &[
                "-p",
                "--portability",
                "-v",
                "--verbose",
                "-a",
                "--append",
                "-q",
                "--quiet",
            ],
            valued: &["-f", "--format", "-o", "--output"],
            chdir: &[],
            assignments: false,
            positional: 0,
            numeric: false,
        },
        _ => return None,
    };
    Some(grammar)
}

/// How many arguments after the wrapper's own name stand before the program, and whether one of
/// them changed the directory. [`None`] for an option this grammar does not know: an unknown
/// option may take a value, so where the program starts is not known, and the wrapper is then
/// classified as itself rather than guessed through.
fn options_before_program(rest: &[String], grammar: &Grammar) -> Option<(usize, bool)> {
    let mut index = 0;
    let mut chdir = false;
    let mut options = true;
    let mut positional = grammar.positional;
    while let Some(arg) = rest.get(index) {
        let arg = arg.as_str();
        if options && grammar.flags.contains(&arg) {
            index += 1;
            continue;
        }
        if options && arg == "--" {
            options = false;
            index += 1;
            continue;
        }
        if options && arg.starts_with('-') && arg.len() > 1 {
            if grammar.valued.contains(&arg) {
                chdir |= grammar.chdir.contains(&arg);
                index += 2;
                continue;
            }
            if let Some(name) = attached(arg, grammar) {
                chdir |= grammar.chdir.contains(&name);
                index += 1;
                continue;
            }
            if grammar.numeric && arg[1..].bytes().all(|byte| byte.is_ascii_digit()) {
                index += 1;
                continue;
            }
            return None;
        }
        if grammar.assignments && is_assignment(arg) {
            index += 1;
            continue;
        }
        if positional > 0 {
            positional -= 1;
            options = false;
            index += 1;
            continue;
        }
        break;
    }
    (positional == 0).then_some((index, chdir))
}

/// The option an attached spelling names: `--signal=KILL` is `--signal`, `-n10` is `-n`.
fn attached(arg: &str, grammar: &Grammar) -> Option<&'static str> {
    let long = |name: &&'static str| {
        name.starts_with("--")
            && arg
                .strip_prefix(*name)
                .is_some_and(|rest| rest.starts_with('='))
    };
    if let Some(name) = grammar
        .valued
        .iter()
        .chain(grammar.flags)
        .copied()
        .find(long)
    {
        return Some(name);
    }
    grammar
        .valued
        .iter()
        .find(|name| name.len() == 2 && !name.starts_with("--") && arg.starts_with(**name))
        .copied()
}

fn is_assignment(arg: &str) -> bool {
    arg.split_once('=').is_some_and(|(name, _)| {
        !name.is_empty()
            && !name.starts_with(|c: char| c.is_ascii_digit())
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    })
}

/// Unwrap `env`, `timeout`, `xargs`, `sudo`, `nice`, `nohup` and `time`, outermost first, until
/// the program they run. A wrapper with an option its grammar does not know, or with no program
/// after its options (`env` alone prints the environment), is the program itself.
fn unwrap(args: &[String]) -> Unwrapped {
    let mut out = Unwrapped {
        skip: 0,
        wrappers: Vec::new(),
        xargs: false,
        cwd_changed: false,
    };
    while let Some(first) = args.get(out.skip) {
        let wrapper = basename(first);
        let Some(grammar) = grammar(wrapper) else {
            break;
        };
        let rest = &args[out.skip + 1..];
        match options_before_program(rest, &grammar) {
            Some((consumed, chdir)) if consumed < rest.len() => {
                out.wrappers.push(wrapper.to_owned());
                out.xargs |= wrapper == "xargs";
                out.cwd_changed |= chdir;
                out.skip += 1 + consumed;
            }
            _ => break,
        }
    }
    out
}

/// Interpreters whose first operand is the script they run.
const INTERPRETERS: &[&str] = &[
    "python", "python2", "python3", "node", "ruby", "perl", "php", "lua", "bash", "sh", "zsh",
    "dash", "ksh",
];

/// File name endings that make a bare command name a script.
const SCRIPT_ENDINGS: &[&str] = &[
    ".sh", ".bash", ".zsh", ".py", ".rb", ".pl", ".js", ".mjs", ".cjs", ".ts", ".php", ".lua",
];

/// The basename of the script a command runs, where it runs one.
///
/// An interpreter's first operand, unless an option says the code is inline (`-c`, `-e`) or a
/// module (`-m`); or the program itself, when it is named by a relative path (`./x.sh`,
/// `scripts/release`) or carries a script ending. `/usr/bin/cat` is a program, not a script.
fn script_of(args: &[String]) -> Option<String> {
    let program = args.first()?;
    let name = basename(program);
    if INTERPRETERS.contains(&name) {
        let mut index = 1;
        while let Some(arg) = args.get(index) {
            match arg.as_str() {
                "-c" | "-lc" | "-e" | "-p" | "-m" | "--eval" | "--print" | "-E" => return None,
                "--" => {
                    return args
                        .get(index + 1)
                        .map(|script| basename(script).to_owned());
                }
                "-W" | "-X" | "-r" | "--require" | "--import" | "--loader" | "-I" | "-o" => {
                    index += 2;
                }
                option if option.starts_with('-') => index += 1,
                script => return Some(basename(script).to_owned()),
            }
        }
        return None;
    }
    let relative = program.contains('/') && !program.starts_with('/');
    let ending = SCRIPT_ENDINGS.iter().any(|ending| name.ends_with(ending));
    (relative || ending).then(|| name.to_owned())
}

fn shell_literal(node: Ast<'_>, source: &str) -> Option<String> {
    let raw = text(node, source);
    match node.kind() {
        "command_name" => node.named_child(0).and_then(|n| shell_literal(n, source)),
        "word" | "number" => {
            if raw.contains(['*', '?', '[', '~', '$', '`', '{', '}']) {
                return None;
            }
            let mut out = String::new();
            let mut iter = raw.chars();
            while let Some(c) = iter.next() {
                if c == '\\' {
                    out.push(iter.next()?);
                } else {
                    out.push(c);
                }
            }
            Some(out)
        }
        "raw_string" => raw
            .strip_prefix('\'')
            .and_then(|s| s.strip_suffix('\''))
            .map(str::to_owned),
        "string" => {
            if children(node).iter().any(|c| c.kind() != "string_content") {
                return None;
            }
            let inner = raw.strip_prefix('"')?.strip_suffix('"')?;
            let mut out = String::new();
            let mut chars = inner.chars();
            while let Some(c) = chars.next() {
                if c == '\\' {
                    let next = chars.next()?;
                    if matches!(next, '$' | '`' | '"' | '\\') {
                        out.push(next);
                    } else if next != '\n' {
                        out.push('\\');
                        out.push(next);
                    }
                } else {
                    out.push(c);
                }
            }
            Some(out)
        }
        "concatenation" => children(node)
            .iter()
            .map(|n| shell_literal(*n, source))
            .collect::<Option<Vec<_>>>()
            .map(|v| v.concat()),
        _ => None,
    }
}
fn shadows_tools(node: Ast<'_>, source: &str) -> bool {
    let direct = match node.kind() {
        "variable_declarator" | "function_declaration" | "class_declaration" => node
            .child_by_field_name("name")
            .is_some_and(|n| text(n, source) == "tools"),
        "assignment_expression" => node
            .child_by_field_name("left")
            .is_some_and(|n| text(n, source).split(['.', '[']).next() == Some("tools")),
        "formal_parameters" => contains_identifier(node, source, "tools"),
        "arrow_function" => node
            .child_by_field_name("parameter")
            .is_some_and(|n| text(n, source) == "tools"),
        _ => false,
    };
    direct || children(node).iter().any(|n| shadows_tools(*n, source))
}
fn contains_identifier(node: Ast<'_>, source: &str, name: &str) -> bool {
    (node.kind() == "identifier" && text(node, source) == name)
        || children(node)
            .iter()
            .any(|n| contains_identifier(*n, source, name))
}
fn contains_js_mutation(node: Ast<'_>) -> bool {
    matches!(
        node.kind(),
        "assignment_expression" | "augmented_assignment_expression" | "update_expression"
    ) || children(node).iter().any(|n| contains_js_mutation(*n))
}
fn contains_command(node: Ast<'_>, source: &str, name: &str) -> bool {
    if node.kind() == "command"
        && node
            .child_by_field_name("name")
            .and_then(|n| shell_literal(n, source))
            .as_deref()
            == Some(name)
    {
        return true;
    }
    children(node)
        .iter()
        .any(|n| contains_command(*n, source, name))
}
fn sed_range(s: &str) -> Option<(u64, u64)> {
    let s = s.strip_suffix('p')?;
    let (a, b) = s.split_once(',').unwrap_or((s, s));
    let a = a.parse().ok()?;
    let b = b.parse().ok()?;
    if a > 0 && b >= a { Some((a, b)) } else { None }
}
fn js_value(node: Ast<'_>, source: &str, bindings: &HashMap<String, Value>) -> Option<Value> {
    match node.kind() {
        "string" => js_string(text(node, source)).map(Value::String),
        "template_string" => {
            if children(node)
                .iter()
                .any(|c| c.kind() == "template_substitution")
            {
                None
            } else {
                js_string(text(node, source)).map(Value::String)
            }
        }
        "number" | "true" | "false" | "null" => serde_json::from_str(text(node, source)).ok(),
        "identifier" => bindings.get(text(node, source)).cloned(),
        "object" => {
            let mut map = Map::new();
            for child in children(node) {
                if child.kind() == "shorthand_property_identifier" {
                    map.insert(
                        text(child, source).into(),
                        bindings.get(text(child, source))?.clone(),
                    );
                    continue;
                }
                if child.kind() != "pair" {
                    return None;
                }
                let key = child.child_by_field_name("key")?;
                let key = if key.kind() == "string" {
                    js_string(text(key, source))?
                } else if key.kind() == "property_identifier" {
                    text(key, source).into()
                } else {
                    return None;
                };
                map.insert(
                    key,
                    js_value(child.child_by_field_name("value")?, source, bindings)?,
                );
            }
            Some(Value::Object(map))
        }
        "array" => {
            let mut cursor = node.walk();
            let mut needs_value = true;
            for child in node.children(&mut cursor) {
                if child.kind() == "," {
                    if needs_value {
                        return None;
                    }
                    needs_value = true;
                } else if child.is_named() {
                    needs_value = false;
                }
            }
            children(node)
                .iter()
                .map(|n| js_value(*n, source, bindings))
                .collect::<Option<Vec<_>>>()
                .map(Value::Array)
        }
        "parenthesized_expression" => js_value(node.named_child(0)?, source, bindings),
        _ => None,
    }
}
fn js_string(raw: &str) -> Option<String> {
    if raw.starts_with('"') {
        return serde_json::from_str(raw).ok();
    }
    let delimiter = raw.chars().next()?;
    if !matches!(delimiter, '\'' | '`') {
        return None;
    }
    let inner = raw.strip_prefix(delimiter)?.strip_suffix(delimiter)?;
    let mut result = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            result.push(c);
            continue;
        }
        match chars.next()? {
            'n' => result.push('\n'),
            'r' => result.push('\r'),
            't' => result.push('\t'),
            '\\' => result.push('\\'),
            '\'' => result.push('\''),
            '"' => result.push('"'),
            '`' => result.push('`'),
            '\n' => {}
            _ => return None,
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn shell(s: &str) -> Vec<Node> {
        extract(
            "codex",
            "exec_command",
            &json!({"cmd":s}).to_string(),
            Some("/repo"),
        )
    }
    #[test]
    fn native_and_shell_share_kind_not_rendering() {
        let native = extract(
            "claude",
            "Read",
            r#"{"file_path":"/repo/a.rs","offset":2,"limit":4}"#,
            Some("/repo"),
        );
        let shell = shell("sed -n '2,5p' a.rs");
        assert_eq!(native[0].kind, "file.read");
        let read = shell.iter().find(|n| n.kind == "file.read").unwrap();
        assert_eq!(read.params["first"], 2);
        assert_ne!(native[0].signature, read.signature);
        assert_eq!(read.evidence, "declared");
    }
    #[test]
    fn canonical_static_paths_and_quotes_match() {
        let a = shell("cat './a b.rs'");
        let b = shell("cat \"/repo/a b.rs\"");
        assert_eq!(a.last().unwrap().signature, b.last().unwrap().signature);
        assert!(a.last().unwrap().signature.is_some());
        assert_eq!(shell("cat -n a.rs").last().unwrap().signature, None);
    }
    #[test]
    fn quoted_tool_like_text_is_not_a_call() {
        let nodes = extract(
            "codex",
            "exec",
            r#"const s = 'tools.exec_command({cmd: "cat secret"})'; text(s);"#,
            None,
        );
        assert!(!nodes.iter().any(|n| n.kind == "file.read"));
    }
    #[test]
    fn static_javascript_binding_parallel_and_nested_shell() {
        let nodes = extract(
            "codex",
            "exec",
            r#"const args={cmd:"cat 'a b'",workdir:"/repo"}; await Promise.allSettled([tools.exec_command(args), tools.exec_command({cmd:'cargo test'})]);"#,
            None,
        );
        assert!(nodes.iter().any(|n| n.kind == "parallel"));
        assert!(
            nodes
                .iter()
                .any(|n| n.kind == "file.read" && n.params["paths"][0] == "/repo/a b")
        );
        assert!(nodes.iter().any(|n| n.kind == "validation.test"));
        assert_eq!(nodes.iter().filter(|n| n.evidence == "observed").count(), 1);
    }
    #[test]
    fn dynamic_expressions_are_not_resolved_or_executed() {
        let nodes = extract(
            "codex",
            "exec",
            "await tools.exec_command({cmd: makeCommand()});",
            None,
        );
        assert!(nodes.iter().any(|n| n.resolution == "partial"));
        assert!(!nodes.iter().any(|n| n.kind == "file.read"));
        assert!(shell("cat $HOME/x").iter().all(|n| n.signature.is_none()));
    }
    #[test]
    fn supported_pipeline_derives_effective_read_without_extra_leaf() {
        let n = shell("cat a | head -n 3");
        let pipeline = n.iter().find(|n| n.kind == "pipeline").unwrap();
        assert_eq!(pipeline.params["effective_read"]["paths"][0], "/repo/a");
        assert_eq!(pipeline.params["effective_read"]["count"], 3);
        assert_eq!(n.iter().filter(|n| !n.container).count(), 2);
    }
    #[test]
    fn pipeline_redirect_and_loop_preserve_structure() {
        let nodes = shell("for x in a b; do cat \"$x\" | head -n 2 > result; done");
        assert!(nodes.iter().any(|n| n.kind == "loop"));
        assert!(nodes.iter().any(|n| n.kind == "pipeline"));
        assert!(
            nodes
                .iter()
                .any(|n| n.kind == "file.modify" && n.method == "redirection")
        );
        assert!(
            nodes
                .iter()
                .filter(|n| !n.container)
                .all(|n| n.evidence == "declared")
        );
        for (i, n) in nodes.iter().enumerate() {
            assert!(n.parent.is_none_or(|p| p < i));
        }
    }
    #[test]
    fn mutation_invalidates_static_object_binding() {
        let nodes = extract(
            "codex",
            "exec",
            "const a={cmd:'cat a'}; a.cmd=dynamic(); tools.exec_command(a);",
            Some("/repo"),
        );
        assert!(!nodes.iter().any(|n| n.kind == "file.read"));
    }
    #[test]
    fn aliases_branch_mutation_and_parameter_shadowing_do_not_reuse_values() {
        for script in [
            "const a={cmd:'cat a'}; const b=a; b.cmd=dynamic(); tools.exec_command(a);",
            "const a={cmd:'cat a'}; if (x) {a.cmd=dynamic()} tools.exec_command(a);",
            "const a={cmd:'cat a'}; function f(a) {tools.exec_command(a)}",
            "const a={cmd:'cat a'}; mutate(a); tools.exec_command(a);",
        ] {
            let nodes = extract("codex", "exec", script, Some("/repo"));
            assert!(!nodes.iter().any(|n| n.kind == "file.read"), "{script}");
        }
    }
    #[test]
    fn web_batches_share_native_search_kind() {
        let nodes = extract(
            "codex",
            "web.run",
            r#"{"search_query":[{"q":"private query"},{"q":"second"}]}"#,
            None,
        );
        assert_eq!(
            nodes.iter().filter(|n| n.kind == "network.search").count(),
            2
        );
        assert!(
            !serde_json::to_string(&nodes)
                .unwrap()
                .contains("private query")
        );
    }
    #[test]
    fn cwd_after_cd_is_unknown_and_unicode_is_valid() {
        let nodes = shell("cd next; cat 'é.txt'");
        let n = nodes.iter().find(|n| n.kind == "file.read").unwrap();
        assert!(n.signature.is_none());
        assert_eq!(n.params["paths"][0], "é.txt");
    }
    #[test]
    fn malformed_sources_remain_unsupported() {
        let n = extract("codex", "exec", "tools.exec_command({", None);
        assert!(n.iter().any(|n| n.method == "malformed_javascript"));
        assert!(
            shell("cat 'oops")
                .iter()
                .any(|n| n.method == "malformed_shell")
        );
    }
    #[test]
    fn shadowed_tools_and_temporal_dead_zone_are_not_harness_calls() {
        for script in [
            "const tools={exec_command:x}; tools.exec_command({cmd:'cat a'});",
            "function f(tools){tools.exec_command({cmd:'cat a'})}",
            "const a={cmd:'cat a'}; {tools.exec_command(a); const a=dynamic();}",
        ] {
            let nodes = extract("codex", "exec", script, Some("/repo"));
            assert!(!nodes.iter().any(|n| n.kind == "file.read"), "{script}");
        }
    }
    #[test]
    fn descriptor_redirects_and_quoted_operators_do_not_modify_files() {
        let nodes = shell("cat < 'a>b' 2>&1");
        assert!(!nodes.iter().any(|n| n.kind == "file.modify"));
        assert!(nodes.iter().any(|n| n.method == "descriptor_redirect"));
    }
    #[test]
    fn uncertain_paths_and_custom_executables_do_not_match_requests() {
        for script in [
            "cat link/../a",
            "/custom/cat a",
            "f(){ cat a; }; cd x; f",
            "for x in a b; do cat a; cd x; done",
        ] {
            assert!(
                shell(script).iter().all(|n| n.signature.is_none()),
                "{script}"
            );
        }
    }
    #[test]
    fn explicit_mutation_flags_are_change_operations() {
        for script in [
            "git branch new",
            "git branch -D old",
            "find . -delete",
            "ruff format .",
            "eslint --fix .",
        ] {
            assert!(
                shell(script).iter().any(|n| n.activity == "change"),
                "{script}"
            );
        }
    }
    #[test]
    fn sparse_arrays_and_extra_call_arguments_are_unresolved() {
        for source in [
            "tools.shell({command:['cat',,'a']})",
            "const a={cmd:'cat a'}; tools.exec_command(a, mutate(a))",
        ] {
            let nodes = extract("codex", "exec", source, Some("/repo"));
            assert!(!nodes.iter().any(|n| n.kind == "file.read"), "{source}");
        }
    }
    #[test]
    fn nested_source_domains_are_unique() {
        let nodes = extract(
            "codex",
            "exec",
            "tools.exec_command({cmd:'cat a'}); tools.exec_command({cmd:'cat b'})",
            Some("/repo"),
        );
        let reads: Vec<_> = nodes.iter().filter(|n| n.kind == "file.read").collect();
        assert_ne!(
            reads[0].params["source_domain"],
            reads[1].params["source_domain"]
        );
    }
    #[test]
    fn shell_array_respects_workdir() {
        let nodes = extract(
            "codex",
            "shell",
            r#"{"command":["cat","a"],"workdir":"/different"}"#,
            Some("/repo"),
        );
        assert_eq!(nodes.last().unwrap().params["paths"][0], "/different/a");
    }
    #[test]
    fn null_device_redirects_are_structure_not_changes() {
        for command in [
            "rg text src 2>/dev/null",
            "cat a >/dev/null",
            "cat a 2>>/dev/null",
            "cat </dev/null",
        ] {
            let nodes = shell(command);
            assert!(!nodes.iter().any(|n| n.activity == "change"), "{command}");
            let redirect = nodes.iter().find(|n| n.kind == "stream.redirect").unwrap();
            assert!(redirect.container);
            assert_eq!(redirect.params["paths"][0], "/dev/null");
        }
        assert!(
            shell("cat a > output")
                .iter()
                .any(|n| n.kind == "file.modify")
        );
    }
    #[test]
    fn stdin_selectors_are_stream_operations() {
        for command in [
            "head -20",
            "head -n160",
            "head -n 3 -",
            "tail -20",
            "sed -n '1,20p'",
            "sed -n '1,20p' -",
        ] {
            let nodes = shell(command);
            assert!(
                nodes
                    .iter()
                    .any(|n| n.kind == "stream.select" && n.params["input"] == "stdin"),
                "{command}"
            );
            assert!(!nodes.iter().any(|n| n.kind == "file.read"), "{command}");
            assert!(nodes.iter().all(|n| n.signature.is_none()));
        }
        assert_eq!(shell("head -20 file").last().unwrap().kind, "file.read");
    }
    #[test]
    fn search_distinguishes_pipe_input_explicit_stdin_and_file_scope() {
        for command in [
            "cat file | rg needle",
            "cat file | grep needle",
            "rg needle -",
            "grep needle",
        ] {
            assert!(
                shell(command)
                    .iter()
                    .any(|n| n.kind == "stream.search" && n.params["input"] == "stdin"),
                "{command}"
            );
        }
        for command in [
            "rg needle",
            "rg needle src",
            "grep -r needle",
            "cat file | rg needle src",
        ] {
            assert!(
                shell(command).iter().any(|n| n.kind == "file.search"),
                "{command}"
            );
            assert!(
                !shell(command).iter().any(|n| n.kind == "stream.search"),
                "{command}"
            );
        }
    }
    #[test]
    fn patterns_are_hashed_and_modes_retained() {
        let n = shell("rg -n 'private pattern' src");
        let n = n.last().unwrap();
        assert_eq!(n.kind, "file.search");
        assert!(!n.params.to_string().contains("private pattern"));
        assert_eq!(n.params["output_mode"], "matching_lines");
        assert_eq!(
            shell("rg --files src").last().unwrap().kind,
            "file.discover"
        );
    }

    // --- metaharness additions (amendment a68): executable, wrappers, script -----------------

    fn leaves(nodes: &[Node]) -> Vec<&Node> {
        nodes.iter().filter(|n| !n.container).collect()
    }

    #[test]
    fn every_command_names_its_executable_basename() {
        let nodes = shell("git status && /usr/bin/cargo test && cat a | head -n 3");
        let executables: Vec<_> = leaves(&nodes)
            .iter()
            .map(|n| n.executable.as_deref())
            .collect();
        assert_eq!(
            executables,
            [Some("git"), Some("cargo"), Some("cat"), Some("head")]
        );
        assert!(leaves(&nodes).iter().all(|n| n.wrappers.is_empty()));
        let native = extract("claude", "Read", r#"{"file_path":"/repo/a"}"#, None);
        assert_eq!(native[0].executable, None);
    }

    #[test]
    fn each_wrapper_unwraps_to_the_program_it_runs() {
        for (command, wrappers, executable, kind) in [
            (
                "env RUST_LOG=debug cargo test",
                &["env"][..],
                "cargo",
                "validation.test",
            ),
            (
                "env -u HOME -i A=1 git status",
                &["env"][..],
                "git",
                "vcs.status",
            ),
            (
                "timeout 60 cargo build",
                &["timeout"][..],
                "cargo",
                "validation.build",
            ),
            (
                "timeout -s KILL -k 5 1m git diff",
                &["timeout"][..],
                "git",
                "vcs.diff",
            ),
            ("xargs rg needle", &["xargs"][..], "rg", "file.search"),
            (
                "xargs -0 -n 1 grep -l needle",
                &["xargs"][..],
                "grep",
                "file.search",
            ),
            ("sudo git status", &["sudo"][..], "git", "vcs.status"),
            (
                "sudo -u build -E cargo clippy",
                &["sudo"][..],
                "cargo",
                "validation.lint",
            ),
            (
                "nice -n 10 cargo test",
                &["nice"][..],
                "cargo",
                "validation.test",
            ),
            (
                "nice -5 cargo test",
                &["nice"][..],
                "cargo",
                "validation.test",
            ),
            (
                "nohup cargo build",
                &["nohup"][..],
                "cargo",
                "validation.build",
            ),
            ("time cargo test", &["time"][..], "cargo", "validation.test"),
            (
                "/usr/bin/time -v cargo test",
                &["time"][..],
                "cargo",
                "validation.test",
            ),
            (
                "sudo timeout 5 env A=1 nice nohup git log",
                &["sudo", "timeout", "env", "nice", "nohup"][..],
                "git",
                "vcs.inspect",
            ),
        ] {
            let nodes = shell(command);
            let leaves = leaves(&nodes);
            assert_eq!(leaves.len(), 1, "{command}: {nodes:?}");
            let leaf = leaves[0];
            assert_eq!(leaf.executable.as_deref(), Some(executable), "{command}");
            assert_eq!(leaf.wrappers, wrappers, "{command}");
            assert_eq!(leaf.kind, kind, "{command}");
        }
    }

    #[test]
    fn a_wrapper_around_a_shell_unwraps_into_the_shell_it_runs() {
        let nodes = shell("timeout 30 bash -c 'git status | head -n 2'");
        let wrapper = nodes
            .iter()
            .find(|n| n.kind == "wrapper" && n.executable.as_deref() == Some("bash"))
            .expect("the bash -c wrapper");
        assert_eq!(wrapper.wrappers, ["timeout"]);
        let kinds: Vec<_> = leaves(&nodes).iter().map(|n| n.kind.as_str()).collect();
        assert_eq!(kinds, ["vcs.status", "stream.select"]);
        assert!(nodes.iter().any(|n| n.kind == "pipeline"));
    }

    #[test]
    fn a_wrapper_with_an_unknown_option_or_no_program_is_not_unwrapped() {
        for (command, executable) in [
            ("env", "env"),
            ("timeout --bogus 5 git status", "timeout"),
            ("sudo -l", "sudo"),
            ("nohup", "nohup"),
        ] {
            let nodes = shell(command);
            let leaves = leaves(&nodes);
            assert_eq!(leaves.len(), 1, "{command}");
            assert_eq!(
                leaves[0].executable.as_deref(),
                Some(executable),
                "{command}"
            );
            assert!(leaves[0].wrappers.is_empty(), "{command}");
            assert_eq!(leaves[0].kind, "process.run", "{command}");
        }
    }

    #[test]
    fn a_wrapped_request_carries_no_signature_and_xargs_operands_are_not_stdin() {
        let plain = shell("cat /repo/a");
        assert!(plain.last().unwrap().signature.is_some());
        for command in [
            "sudo cat /repo/a",
            "env A=1 cat /repo/a",
            "timeout 5 cat /repo/a",
        ] {
            let nodes = shell(command);
            let leaf = leaves(&nodes)[0];
            assert_eq!(leaf.kind, "file.read", "{command}");
            assert!(leaf.signature.is_none(), "{command}");
        }
        for command in [
            "find . -name x | xargs cat",
            "ls | xargs head -n 2",
            "ls | xargs grep needle",
        ] {
            let nodes = shell(command);
            let leaf = *leaves(&nodes).last().unwrap();
            assert!(leaf.kind.starts_with("file."), "{command}: {}", leaf.kind);
            assert_eq!(leaf.params["input"], "xargs_operands", "{command}");
            assert!(leaf.signature.is_none(), "{command}");
        }
    }

    #[test]
    fn a_script_names_its_basename() {
        for (command, executable, script) in [
            ("./scripts/check.sh --fast", "check.sh", "check.sh"),
            ("scripts/release", "release", "release"),
            ("python3 tools/report.py --all", "python3", "report.py"),
            ("python -u x.py", "python", "x.py"),
            ("node ./bin/cli.mjs run", "node", "cli.mjs"),
            ("bash ./deploy.sh", "bash", "deploy.sh"),
            ("timeout 5 ./x.sh", "x.sh", "x.sh"),
        ] {
            let nodes = shell(command);
            let leaf = leaves(&nodes)[0];
            assert_eq!(leaf.executable.as_deref(), Some(executable), "{command}");
            assert_eq!(leaf.script.as_deref(), Some(script), "{command}");
        }
        for command in [
            "python3 -c 'print(1)'",
            "python3 -m pytest",
            "node -e 1",
            "/usr/bin/cat a",
        ] {
            let nodes = shell(command);
            assert_eq!(leaves(&nodes)[0].script, None, "{command}");
        }
    }

    #[test]
    fn no_argument_text_is_stored_beyond_the_source() {
        for command in [
            "sudo -u sentinel-user git status",
            "env SENTINEL=sentinel-value cargo test sentinel-filter",
            "timeout 5 rg 'sentinel pattern' src",
            "xargs -I sentinel-repl grep sentinel-needle",
            "python3 -c 'sentinel code'",
        ] {
            let text = serde_json::to_string(&shell(command)).unwrap();
            assert!(!text.contains("sentinel"), "{command}: {text}");
        }
    }
}
