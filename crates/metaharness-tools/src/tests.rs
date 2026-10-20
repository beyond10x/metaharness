use serde_json::{Value, json};

use super::*;
use crate::VerbSurface::{HarnessLoop, Served};

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
    // **The catalogue's own roster, not a copy of it.** What this test is about is that the search
    // verb reaches the catalogue and answers with the whole of it, in order. Which tools the
    // harness publishes is the harness's fact, and a hand-copied list asserted it a second time
    // here — so `find` arriving as a seventh entry turned this red without anything in metaharness
    // being wrong. Compared against `entry_names`, the function the catalogue builds itself from,
    // which is the same comparison `metaharness-b10x`'s seam already makes.
    // As sets, not as sequences: `entry_names` is keyed by operation, so the order it yields is
    // the operation ids' and not the catalogue's, and asserting on it would pin the wrong thing.
    let mut published: Vec<&str> = harness_catalogue::entry_names().into_values().collect();
    published.sort_unstable();
    let mut answered = names.clone();
    answered.sort_unstable();
    assert_eq!(
        answered, published,
        "the verb answers the catalogue whole: no entry withheld, none invented"
    );
    // Not vacuous in the direction that matters: the entries metaharness *maps* must still exist,
    // and a catalogue that lost one would pass the comparison above by shrinking on both sides.
    for entry in [
        "file_read",
        "dir_list",
        "search",
        "file_write",
        "file_edit",
        "run",
    ] {
        assert!(
            names.contains(&entry),
            "metaharness resolves `{entry}` and the catalogue no longer publishes it: {names:?}"
        );
    }

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
    for (tool, surface) in [
        ("mcp__metaharness__tool_invoke", Served),
        (INVOKE_VERB, HarnessLoop),
    ] {
        assert_eq!(
            resolve_verb(
                tool,
                &json!({"name": "file_write", "arguments": {"path": "a"}}),
                surface
            ),
            Some(Resolved::Operations(vec!["file.write".to_owned()])),
            "{tool}"
        );
    }
    assert_eq!(
        resolve_verb(
            INVOKE_VERB,
            &json!({"name": "run", "arguments": {"argv": ["cargo"]}}),
            HarnessLoop
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
            resolve_verb(verb, &json!({}), HarnessLoop),
            Some(Resolved::Catalogue),
            "{verb}"
        );
    }
}

#[test]
fn an_invocation_naming_no_entry_is_unknown_rather_than_an_operation_that_never_happened() {
    assert_eq!(
        resolve_verb(INVOKE_VERB, &json!({}), HarnessLoop),
        Some(Resolved::Unknown)
    );
    assert_eq!(
        resolve_verb(INVOKE_VERB, &json!({"name": "Bash"}), HarnessLoop),
        Some(Resolved::Unknown),
        "a name outside the vocabulary reached no tool, so it is no operation"
    );
}

#[test]
fn a_tool_that_is_not_one_of_the_verbs_is_left_to_the_callers_own_table() {
    // `None`, not `Unknown`: the caller has a rendering table for its vendor's tools, and an
    // answer here would override it.
    for tool in ["Bash", "Write", "apply_patch", "mcp__linear__create_issue"] {
        assert_eq!(resolve_verb(tool, &json!({}), HarnessLoop), None, "{tool}");
    }
}

// --- what a call touched ------------------------------------------------------------------------

#[test]
fn an_invocation_names_the_file_its_entry_would_touch() {
    // `operations` says a write happened; this says which file. Without both, an expectation can
    // assert that a run wrote something and never that it wrote the test before the source.
    assert_eq!(
        subjects_of_verb(
            "mcp__metaharness__tool_invoke",
            &json!({"name": "file_write", "arguments": {"path": "src/lib.rs", "text": "x"}}),
            Served
        ),
        vec!["file:src/lib.rs".to_owned()]
    );
    assert_eq!(
        subjects_of_verb(
            INVOKE_VERB,
            &json!({"name": "run", "arguments": {"argv": ["/usr/bin/python3", "t.py"]}}),
            HarnessLoop
        ),
        vec!["proc:/usr/bin/python3".to_owned()],
        "a `run` names the program, not the whole argv"
    );
}

#[test]
fn a_path_is_reported_as_the_caller_wrote_it() {
    // The tidy answer canonicalisation would give is exactly wrong for the one call whose whole
    // problem is where it pointed.
    assert_eq!(
        subjects_of_verb(
            INVOKE_VERB,
            &json!({"name": "file_read", "arguments": {"path": "../../etc/passwd"}}),
            HarnessLoop
        ),
        vec!["file:../../etc/passwd".to_owned()]
    );
}

#[test]
fn a_catalogue_question_and_an_unknown_entry_touch_nothing() {
    for tool in [SEARCH_VERB, DESCRIBE_VERB] {
        assert!(
            subjects_of_verb(tool, &json!({"name": "file_read"}), HarnessLoop).is_empty(),
            "{tool}"
        );
    }
    assert_eq!(
        subjects_of_verb(INVOKE_VERB, &json!({"name": "Bash"}), HarnessLoop),
        [] as [String; 0]
    );
    assert!(
        subjects_of_verb("Write", &json!({"file_path": "a"}), HarnessLoop).is_empty(),
        "not a verb"
    );
}

#[test]
fn a_vendor_call_names_the_path_whichever_argument_holds_it() {
    // Three names and no vendor table: this asks *which key holds a path*, where *which tool is a
    // write* is the rendering's question and has an owner already.
    assert_eq!(
        subjects_of_vendor_call(&json!({"file_path": "src/lib.rs", "old_string": "a"})),
        vec!["file:src/lib.rs".to_owned()]
    );
    assert_eq!(
        subjects_of_vendor_call(&json!({"notebook_path": "book.ipynb"})),
        vec!["file:book.ipynb".to_owned()]
    );
}

#[test]
fn a_shell_command_is_left_alone_rather_than_guessed_at() {
    // A command string is not a program. Pulling an argv[0] out of one is parsing a language this
    // crate does not speak, and a `proc:` subject guessed that way would be a claim about what ran.
    assert_eq!(
        subjects_of_vendor_call(&json!({"command": "cargo test --workspace"})),
        [] as [String; 0]
    );
}

// --- `tool-contract/1` names: harness's mapping, read and never copied --------------------------

/// The contract names harness `0.15.1` maps, with the entry each one maps to.
///
/// A pin of the mapping as it stands at the revision this workspace locks, not the mapping
/// itself: production code reads `harness_catalogue::harness_operation` and holds no table. A
/// re-pin that adds, drops or moves a name turns `the_names_harness_maps_are_the_ones_this_suite_pins`
/// red, and the changelog says which.
const MAPPED: [(&str, &str); 5] = [
    ("fs.read", "file_read"),
    ("fs.list", "dir_list"),
    ("fs.glob", "find"),
    ("code.write", "file_write"),
    ("code.edit", "file_edit"),
];

/// What `contract` resolves to on the served surface, asserted equal to what its harness `entry`
/// resolves to.
fn resolved_as_its_entry(contract: &str, entry: &str, arguments: &Value) -> Option<Resolved> {
    let by_contract = resolve_verb(
        INVOKE_VERB,
        &json!({"name": contract, "arguments": arguments}),
        Served,
    );
    let by_entry = resolve_verb(
        INVOKE_VERB,
        &json!({"name": entry, "arguments": arguments}),
        Served,
    );
    assert_eq!(by_contract, by_entry, "`{contract}` resolves as `{entry}`");
    by_contract
}

/// What `contract` touches on the served surface, asserted equal to what its harness `entry`
/// touches.
fn touching_what_its_entry_touches(contract: &str, entry: &str, arguments: &Value) -> Vec<String> {
    let tool = "mcp__metaharness__tool_invoke";
    let by_contract = subjects_of_verb(
        tool,
        &json!({"name": contract, "arguments": arguments}),
        Served,
    );
    let by_entry = subjects_of_verb(
        tool,
        &json!({"name": entry, "arguments": arguments}),
        Served,
    );
    assert_eq!(
        by_contract, by_entry,
        "`{contract}` touches what `{entry}` does"
    );
    by_contract
}

fn resolved_to(operation: &str) -> Resolved {
    Resolved::Operations(vec![operation.to_owned()])
}

#[test]
fn fs_read_resolves_as_file_read() {
    assert_eq!(
        resolved_as_its_entry("fs.read", "file_read", &json!({"path": "src/lib.rs"})),
        Some(resolved_to("file.read"))
    );
}

#[test]
fn fs_list_resolves_as_dir_list() {
    assert_eq!(
        resolved_as_its_entry("fs.list", "dir_list", &json!({"path": "crates"})),
        Some(resolved_to("dir.list"))
    );
}

#[test]
fn fs_glob_resolves_as_find() {
    assert_eq!(
        resolved_as_its_entry(
            "fs.glob",
            "find",
            &json!({"glob": "*.rs", "path": "crates"})
        ),
        Some(resolved_to("find"))
    );
}

#[test]
fn code_write_resolves_as_file_write() {
    assert_eq!(
        resolved_as_its_entry(
            "code.write",
            "file_write",
            &json!({"path": "src/new.rs", "text": "x"})
        ),
        Some(resolved_to("file.write"))
    );
}

#[test]
fn code_edit_resolves_as_file_edit() {
    assert_eq!(
        resolved_as_its_entry(
            "code.edit",
            "file_edit",
            &json!({"path": "src/lib.rs", "old": "a", "new": "b"})
        ),
        Some(resolved_to("file.edit"))
    );
}

#[test]
fn fs_read_touches_the_file_file_read_would() {
    assert_eq!(
        touching_what_its_entry_touches("fs.read", "file_read", &json!({"path": "src/lib.rs"})),
        vec!["file:src/lib.rs".to_owned()]
    );
}

#[test]
fn fs_list_touches_the_directory_dir_list_would() {
    assert_eq!(
        touching_what_its_entry_touches("fs.list", "dir_list", &json!({"path": "crates"})),
        vec!["file:crates".to_owned()]
    );
}

#[test]
fn fs_glob_touches_the_directory_find_would() {
    assert_eq!(
        touching_what_its_entry_touches(
            "fs.glob",
            "find",
            &json!({"glob": "*.rs", "path": "crates"})
        ),
        vec!["file:crates".to_owned()]
    );
}

#[test]
fn code_write_touches_the_file_file_write_would() {
    assert_eq!(
        touching_what_its_entry_touches(
            "code.write",
            "file_write",
            &json!({"path": "src/new.rs", "text": "x"})
        ),
        vec!["file:src/new.rs".to_owned()]
    );
}

#[test]
fn code_edit_touches_the_file_file_edit_would() {
    assert_eq!(
        touching_what_its_entry_touches(
            "code.edit",
            "file_edit",
            &json!({"path": "src/lib.rs", "old": "a", "new": "b"})
        ),
        vec!["file:src/lib.rs".to_owned()]
    );
}

#[test]
fn a_contract_name_reaching_outside_the_workspace_names_where_it_reaches() {
    // The safety half. `Run::out_of_scope` admits a call that names no subject — silence is not a
    // violation — so a contract name that resolved to an operation and touched nothing would pass a
    // frame's subject scope whatever path it carried. It has to name the path as the caller wrote
    // it, exactly as its entry does, for the scope to refuse it.
    for (contract, entry) in MAPPED {
        let arguments =
            json!({"path": "../../etc/passwd", "glob": "*", "text": "x", "old": "a", "new": "b"});
        assert_eq!(
            touching_what_its_entry_touches(contract, entry, &arguments),
            vec!["file:../../etc/passwd".to_owned()],
            "{contract}"
        );
    }
}

#[test]
fn a_dotted_name_harness_does_not_map_stays_unknown() {
    // `code.delete` and `process.run` are `tool-contract/1` commands harness `0.15.1` does not
    // route; `file.read` is the neutral operation, which is not an entry. None reached a tool, on
    // either surface.
    for surface in [Served, HarnessLoop] {
        for name in ["code.delete", "process.run", "file.read"] {
            let call = json!({"name": name, "arguments": {"path": "src/lib.rs"}});
            assert_eq!(
                resolve_verb(INVOKE_VERB, &call, surface),
                Some(Resolved::Unknown),
                "{name} on {surface:?}"
            );
            assert!(
                subjects_of_verb(INVOKE_VERB, &call, surface).is_empty(),
                "{name} on {surface:?}"
            );
        }
    }
}

#[test]
fn a_contract_name_is_its_entry_where_metaharness_serves_it_and_unknown_on_a_b10x_record() {
    // metaharness's server performs `fs.read` as `file_read`. The b10x loop's verbs at the pinned
    // release answer harness names only and refused it, so on its record the call reached no tool,
    // and an operation or a subject there would put an effect in the record that never happened.
    let call = json!({"name": "fs.read", "arguments": {"path": "src/lib.rs"}});

    assert_eq!(
        resolve_verb("mcp__metaharness__tool_invoke", &call, Served),
        Some(resolved_to("file.read"))
    );
    assert_eq!(
        subjects_of_verb("mcp__metaharness__tool_invoke", &call, Served),
        vec!["file:src/lib.rs".to_owned()]
    );

    assert_eq!(
        resolve_verb(INVOKE_VERB, &call, HarnessLoop),
        Some(Resolved::Unknown)
    );
    assert_eq!(
        subjects_of_verb(INVOKE_VERB, &call, HarnessLoop),
        [] as [String; 0]
    );
}

#[test]
fn a_harness_name_resolves_and_touches_the_same_on_either_surface() {
    // The surface decides only what a contract name was. A harness entry name is itself on both.
    for (entry, arguments) in [
        ("file_read", json!({"path": "src/lib.rs"})),
        ("dir_list", json!({"path": "crates"})),
        ("search", json!({"pattern": "fn", "path": "crates"})),
        ("find", json!({"glob": "*.rs", "path": "crates"})),
        ("file_write", json!({"path": "src/new.rs", "text": "x"})),
        (
            "file_edit",
            json!({"path": "src/lib.rs", "old": "a", "new": "b"}),
        ),
        ("run", json!({"argv": ["/usr/bin/python3", "t.py"]})),
    ] {
        let call = json!({"name": entry, "arguments": arguments});
        let served = resolve_verb(INVOKE_VERB, &call, Served);
        assert!(matches!(served, Some(Resolved::Operations(_))), "{entry}");
        assert_eq!(
            served,
            resolve_verb(INVOKE_VERB, &call, HarnessLoop),
            "{entry}"
        );
        let touched = subjects_of_verb(INVOKE_VERB, &call, Served);
        assert!(!touched.is_empty(), "{entry}");
        assert_eq!(
            touched,
            subjects_of_verb(INVOKE_VERB, &call, HarnessLoop),
            "{entry}"
        );
    }
}

#[test]
fn the_names_harness_maps_are_the_ones_this_suite_pins() {
    // The class, read off harness rather than listed: every entry harness maps to a contract name
    // resolves and touches through that name exactly as through its own. A mapping harness adds on
    // a re-pin is covered here without an edit, and the set comparison says that it moved.
    let mut mapped: Vec<(&str, &str)> = harness_catalogue::entry_names()
        .into_values()
        .filter_map(|entry| harness_catalogue::tools_operation(entry).map(|id| (id, entry)))
        .collect();
    mapped.sort_unstable();
    let mut pinned = MAPPED.to_vec();
    pinned.sort_unstable();
    assert_eq!(mapped, pinned, "the mapping moved with the pin");

    let arguments = json!({"path": "src/lib.rs", "glob": "*", "text": "x", "old": "a", "new": "b"});
    for (contract, entry) in mapped {
        assert!(
            matches!(
                resolved_as_its_entry(contract, entry, &arguments),
                Some(Resolved::Operations(_))
            ),
            "{contract}"
        );
        assert!(
            !touching_what_its_entry_touches(contract, entry, &arguments).is_empty(),
            "{contract}"
        );
    }
}

#[test]
fn an_invocation_naming_a_contract_name_is_performed_as_its_harness_entry() {
    let dir = tree();
    let mut server = server(dir.path());
    let mut invoke = |name: &str, arguments: Value| {
        server
            .handle(&request(
                1,
                "tools/call",
                &json!({"name": INVOKE_VERB, "arguments": {"name": name, "arguments": arguments}}),
            ))
            .expect("answered")
    };

    let by_contract = invoke("fs.read", json!({"path": "README.md"}));
    assert_eq!(by_contract["result"]["isError"], false, "{by_contract}");
    assert!(
        text(&by_contract).contains("hello harness"),
        "{by_contract}"
    );
    assert_eq!(
        by_contract,
        invoke("file_read", json!({"path": "README.md"})),
        "the same answer the entry gives"
    );

    let written = invoke(
        "code.write",
        json!({"path": "written.txt", "text": "by contract\n"}),
    );
    assert_eq!(written["result"]["isError"], false, "{written}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("written.txt")).expect("on disk"),
        "by contract\n"
    );
}

#[test]
fn a_describe_naming_a_contract_name_describes_its_harness_entry() {
    let dir = tree();
    let mut server = server(dir.path());
    let mut describe = |name: &str| {
        server
            .handle(&request(
                1,
                "tools/call",
                &json!({"name": DESCRIBE_VERB, "arguments": {"name": name}}),
            ))
            .expect("answered")
    };
    for (contract, entry) in MAPPED {
        let by_contract = describe(contract);
        assert_eq!(by_contract["result"]["isError"], false, "{by_contract}");
        let described: Value = serde_json::from_str(text(&by_contract)).expect("JSON");
        assert_eq!(
            described["name"], entry,
            "harness's name is the one answered"
        );
        assert_eq!(by_contract, describe(entry), "{contract}");
    }
}

#[test]
fn a_contract_write_asking_for_a_digest_check_is_refused_because_the_served_one_runs_none() {
    // tool-contract/1 gives `code.write` and `code.edit` an `expected_sha256` precondition whose
    // failure is `precondition-failed`, nothing changed. Harness `0.15.1` performs both entries
    // without reading the field, so a contract name carrying it is refused rather than performed
    // with the guard dropped — whatever it holds, the file's own digest and `null` included.
    const DIGEST_OF_README: &str =
        "ca8380141fe6ae546862de4378a80805c5c86eaf8f67a6b91cea383f1bee7527";
    let dir = tree();
    let mut server = server(dir.path());
    for (contract, arguments) in [
        (
            "code.write",
            json!({"path": "README.md", "text": "replaced\n", "expected_sha256": DIGEST_OF_README}),
        ),
        (
            "code.edit",
            json!({"path": "README.md", "old": "hello", "new": "bye", "expected_sha256": DIGEST_OF_README}),
        ),
        (
            "code.write",
            json!({"path": "README.md", "text": "replaced\n", "expected_sha256": null}),
        ),
    ] {
        let answer = server
            .handle(&request(
                1,
                "tools/call",
                &json!({"name": INVOKE_VERB, "arguments": {"name": contract, "arguments": arguments}}),
            ))
            .expect("answered");
        assert!(
            answer.get("error").is_none(),
            "not a protocol error: {answer}"
        );
        assert_eq!(answer["result"]["isError"], true, "{answer}");
        let said = text(&answer);
        assert!(
            said.contains("precondition-failed") && said.contains("expected_sha256"),
            "{said}"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("README.md")).expect("on disk"),
            "hello harness\n",
            "{contract} changed nothing"
        );
    }
}
