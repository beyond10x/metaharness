use serde_json::{Value, json};

use super::*;

fn tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a temporary tree");
    std::fs::write(dir.path().join("README.md"), "hello harness\n").expect("a file");
    dir
}

fn server(root: &std::path::Path) -> Server {
    let operations =
        LocalOperations::unconfined(root, vec!["/bin/echo".to_owned()]).expect("opens");
    Server::new(Verbs::new(Catalogue::of(operations)))
}

fn request(id: u64, method: &str, params: &Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params.clone()})
}

fn tool_names(answer: &Value) -> Vec<&str> {
    answer["result"]["tools"]
        .as_array()
        .expect("a list")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect()
}

fn text(answer: &Value) -> &str {
    answer["result"]["content"][0]["text"]
        .as_str()
        .expect("one text block")
}

#[test]
fn initialize_answers_the_version_the_client_asked_for_and_a_tools_capability() {
    let dir = tree();
    let mut server = server(dir.path());

    let answer = server
        .handle(&request(
            1,
            "initialize",
            &json!({"protocolVersion": "2024-11-05"}),
        ))
        .expect("answered");
    assert_eq!(answer["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(answer["result"]["serverInfo"]["name"], SERVER_NAME);
    assert!(answer["result"]["capabilities"]["tools"].is_object());

    let answer = server
        .handle(&request(2, "initialize", &json!({})))
        .expect("answered");
    assert_eq!(answer["result"]["protocolVersion"], PROTOCOL_VERSION);
}

#[test]
fn the_initialized_notification_is_performed_and_never_answered() {
    // A response to a notification is a protocol error, and the client that gets one has no `id`
    // to match it against. Silence is the whole contract.
    let dir = tree();
    assert!(
        server(dir.path())
            .handle(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
            .is_none()
    );
}

#[test]
fn the_model_is_offered_the_three_verbs_and_nothing_the_vendor_would_have_given_it() {
    let dir = tree();
    let answer = server(dir.path())
        .handle(&request(1, "tools/list", &json!({})))
        .expect("answered");

    assert_eq!(
        tool_names(&answer),
        vec![SEARCH_VERB, DESCRIBE_VERB, INVOKE_VERB],
        "no Bash, because there is no Bash"
    );
    let schema = &answer["result"]["tools"][2]["inputSchema"];
    assert_eq!(schema["required"], json!(["name", "arguments"]));
}

#[test]
fn a_search_reaches_the_catalogue_and_a_describe_reaches_one_entry() {
    let dir = tree();
    let mut server = server(dir.path());

    let answer = server
        .handle(&request(
            1,
            "tools/call",
            &json!({"name": SEARCH_VERB, "arguments": {}}),
        ))
        .expect("answered");
    assert_eq!(answer["result"]["isError"], false);
    let listed: Value = serde_json::from_str(text(&answer)).expect("the verb answers JSON");
    let names: Vec<&str> = listed["tools"]
        .as_array()
        .expect("a list")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert_eq!(
        names,
        vec![
            "file_read",
            "dir_list",
            "search",
            "file_write",
            "file_edit",
            "run"
        ]
    );

    let answer = server
        .handle(&request(
            2,
            "tools/call",
            &json!({"name": DESCRIBE_VERB, "arguments": {"name": "file_edit"}}),
        ))
        .expect("answered");
    let described: Value = serde_json::from_str(text(&answer)).expect("JSON");
    assert_eq!(described["operation"], "file.edit");
}

#[test]
fn an_invocation_changes_the_tree_and_the_answer_is_the_entrys_own() {
    let dir = tree();
    let mut server = server(dir.path());

    let answer = server
        .handle(&request(
            1,
            "tools/call",
            &json!({"name": INVOKE_VERB, "arguments": {
                "name": "file_write",
                "arguments": {"path": "written.txt", "text": "by a tool\n"}
            }}),
        ))
        .expect("answered");
    assert_eq!(answer["result"]["isError"], false, "{answer}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("written.txt")).expect("on disk"),
        "by a tool\n"
    );
}

#[test]
fn a_tool_the_run_does_not_have_is_an_is_error_result_and_not_a_protocol_error() {
    // The difference matters to the model, not to the client: a JSON-RPC error is the *client's*
    // problem and never reaches the model, so a refusal delivered that way is a turn it spends
    // learning nothing.
    let dir = tree();
    let answer = server(dir.path())
        .handle(&request(
            1,
            "tools/call",
            &json!({"name": INVOKE_VERB, "arguments": {"name": "Bash", "arguments": {"command": "id"}}}),
        ))
        .expect("answered");

    assert!(answer.get("error").is_none(), "not a protocol error");
    assert_eq!(answer["result"]["isError"], true);
    let said = text(&answer);
    assert!(said.contains("`Bash` is not a tool this run has"), "{said}");
    assert!(said.contains("file_read"), "and it lists what is: {said}");
}

#[test]
fn a_method_this_server_does_not_have_is_a_protocol_error_naming_it() {
    let dir = tree();
    let answer = server(dir.path())
        .handle(&request(1, "resources/list", &json!({})))
        .expect("answered");
    assert_eq!(answer["error"]["code"], -32601);
    assert!(
        answer["error"]["message"]
            .as_str()
            .expect("a message")
            .contains("resources/list")
    );
}

#[test]
fn a_read_only_server_publishes_the_same_three_verbs_over_a_shorter_catalogue() {
    // The verbs never change; what stands behind them does. That is the property that lets one
    // corpus read a confined run and an unconfined one.
    let dir = tree();
    let mut server = Server::new(Verbs::new(Catalogue::of(
        LocalOperations::new(dir.path()).expect("opens"),
    )));

    let answer = server
        .handle(&request(1, "tools/list", &json!({})))
        .expect("answered");
    assert_eq!(
        tool_names(&answer),
        vec![SEARCH_VERB, DESCRIBE_VERB, INVOKE_VERB]
    );

    let answer = server
        .handle(&request(
            2,
            "tools/call",
            &json!({"name": INVOKE_VERB, "arguments": {
                "name": "file_write", "arguments": {"path": "no.txt", "text": "x"}
            }}),
        ))
        .expect("answered");
    assert_eq!(answer["result"]["isError"], true);
    assert!(!dir.path().join("no.txt").exists());
}

#[test]
fn a_line_that_is_not_json_is_skipped_rather_than_ending_the_run() {
    let dir = tree();
    let mut server = server(dir.path());
    let input = format!(
        "{}\nnot json at all\n\n{}\n",
        request(1, "tools/list", &json!({})),
        request(2, "ping", &json!({}))
    );

    let mut written = Vec::new();
    serve(&mut server, std::io::Cursor::new(input), &mut written).expect("it serves");

    let answered: Vec<Value> = String::from_utf8(written)
        .expect("utf-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("each line is one object"))
        .collect();
    assert_eq!(answered.len(), 2, "the bad line cost nothing: {answered:?}");
    assert_eq!(answered[0]["id"], 1);
    assert_eq!(answered[1]["id"], 2);
}

// --- reading a run back: one vocabulary for every harness ----------------------------------------

#[test]
fn the_vendor_prefix_comes_off_so_one_function_reads_every_harnesss_record() {
    assert_eq!(unprefixed("mcp__metaharness__tool_invoke"), "tool_invoke");
    assert_eq!(
        unprefixed("tool_invoke"),
        "tool_invoke",
        "b10x records them bare"
    );
    assert_eq!(
        unprefixed("mcp__linear__create_issue"),
        "create_issue",
        "the server name is the launch's, so only the tail is matched"
    );
    assert_eq!(unprefixed("Bash"), "Bash", "a vendor tool is left alone");
}

#[test]
fn an_invocation_reads_as_the_operation_its_entry_is_whatever_the_harness_called_the_verb() {
    // The blindness, gone: two harnesses, two tool names, one answer.
    for tool in ["mcp__metaharness__tool_invoke", INVOKE_VERB] {
        assert_eq!(
            resolve_verb(
                tool,
                &json!({"name": "file_write", "arguments": {"path": "a"}})
            ),
            Some(Resolved::Operations(vec!["file.write".to_owned()])),
            "{tool}"
        );
    }
    assert_eq!(
        resolve_verb(
            INVOKE_VERB,
            &json!({"name": "run", "arguments": {"argv": ["cargo"]}})
        ),
        Some(Resolved::Operations(vec!["shell".to_owned()])),
        "`run` is `shell`, which is the name a frame admits"
    );
}

#[test]
fn asking_what_tools_exist_is_its_own_answer_and_not_an_operation() {
    // Neither an act nor an unknown. A frame that denied it would be refusing the model permission
    // to read the list of things it may do — a refusal with no subject — and the model would then
    // guess arguments instead of asking for them.
    for verb in [SEARCH_VERB, DESCRIBE_VERB] {
        assert_eq!(
            resolve_verb(verb, &json!({})),
            Some(Resolved::Catalogue),
            "{verb}"
        );
    }
}

#[test]
fn an_invocation_naming_no_entry_is_unknown_rather_than_an_operation_that_never_happened() {
    assert_eq!(
        resolve_verb(INVOKE_VERB, &json!({})),
        Some(Resolved::Unknown)
    );
    assert_eq!(
        resolve_verb(INVOKE_VERB, &json!({"name": "Bash"})),
        Some(Resolved::Unknown),
        "a name outside the vocabulary reached no tool, so it is no operation"
    );
}

#[test]
fn a_tool_that_is_not_one_of_the_verbs_is_left_to_the_callers_own_table() {
    // `None`, not `Unknown`: the caller has a rendering table for its vendor's tools, and an
    // answer here would override it.
    for tool in ["Bash", "Write", "apply_patch", "mcp__linear__create_issue"] {
        assert_eq!(resolve_verb(tool, &json!({})), None, "{tool}");
    }
}
