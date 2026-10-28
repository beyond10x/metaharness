//! A Claude Code session **as Claude Code stores it on disk**, read into protocol events.
//!
//! `~/.claude/projects/<project>/<session>.jsonl` is not `stream-json`. The inner `message` of a
//! `user` or `assistant` record is the same shape, and it is read by the same mapping
//! ([`crate::transcript`]'s `assistant_blocks`, `message_usage` and `user_message`); the envelope
//! around it differs, and so does almost everything else in the file:
//!
//! * there is no `system/init` record and no `result` record, so there is no vendor opening and no
//!   terminal record. The opening is assembled from the envelope (`sessionId`, `version`, `cwd`,
//!   `permissionMode`, the first assistant `message.model`), and a session never says it ended;
//! * the envelope spells the request id `requestId` and the per-tool result sibling
//!   `toolUseResult`, and marks text the harness added with `isMeta`;
//! * one API response is split into one record per content block, and **each of them repeats the
//!   response's `usage`**: on local sessions read 2026-10-28, 1,329 of 1,329 requests written over
//!   more than one record repeated one identical usage block. A `usage` event is therefore owed
//!   once per `requestId`, on its first record; a second would bill the request twice;
//! * the file carries many record types the protocol has no word for (`attachment`,
//!   `api-request`, `queue-operation`, `custom-title`, `system/turn_duration` and others). Each is
//!   [`Event::Opaque`], with its digest and its 1-based line.
//!
//! **Sub-agents.** Each sub-agent is its own file, `<session>/subagents/agent-<id>.jsonl`, beside
//! an optional `agent-<id>.meta.json`. The spawning call is linked two ways in the files, and both
//! agreed on every local sub-agent checked: the parent's `Agent` result carries
//! `toolUseResult.agentId`, and the meta file carries `toolUseId`. The reader takes the result's
//! link first and the meta file's second. A nested sub-agent's spawning call is in a sibling
//! sub-agent file, which is why the links are collected across every file before any event is
//! written. Each sub-agent's events are appended after the session's own, opened by one
//! [`warning_code::VENDOR_SUBAGENT`] warning naming the sub-agent, the spawning call and the file
//! (amendment a63).
//!
//! **Total by construction**, as [`crate::TranscriptReader`] is: every record becomes at least one
//! event, an unmapped one becomes `opaque`, unknown fields are ignored and no clock is read.
//!
//! **Unpinned.** The session file is not a format any pinned binary was qualified on; this reader
//! was written against the field names of local sessions recorded by Claude Code 2.1.294 to
//! 2.1.309, read 2026-10-28, and claims nothing about any other version (invariant 4).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use metaharness_protocol::{
    Digest, Emission, Event, HermeticAttestation, Seam, TranscriptRef, warning_code,
};
use serde_json::Value;

use crate::ADAPTER_ID;
use crate::transcript::{Record, assistant_blocks, message_usage, opaque, str_field, user_message};

/// One sub-agent transcript beside a session, as read from disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubagentFile {
    /// The sub-agent's id: the file name without `agent-` and `.jsonl`.
    pub agent_id: String,
    /// The file, relative to the session file's directory: `<session>/subagents/<file>`. Never an
    /// absolute path, because a document that carried one would carry the machine's directories.
    pub path: String,
    /// The SHA-256 of the file's bytes as read.
    pub digest: Digest,
    /// The file's text. Bytes that are not UTF-8 are replaced, and the digest is over the bytes.
    pub text: String,
    /// The meta file's `toolUseId`, where there is a meta file and it names one.
    pub meta_call: Option<String>,
}

/// The sub-agent transcripts stored beside a session file, in file-name order.
///
/// Empty when `<session>/subagents/` does not exist: a session that spawned nothing has no such
/// directory.
///
/// # Errors
///
/// Any I/O error other than the directory being absent, and any sub-agent file that cannot be
/// read. A meta file that is absent or unreadable JSON is no link, not an error.
pub fn read_subagents(session: &Path) -> std::io::Result<Vec<SubagentFile>> {
    let Some(stem) = session
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
    else {
        return Ok(Vec::new());
    };
    let directory = session.with_file_name(&stem).join("subagents");
    let entries = match std::fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut names = Vec::new();
    for entry in entries {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if Path::new(&name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("jsonl"))
        {
            names.push(name);
        }
    }
    names.sort();
    let mut files = Vec::with_capacity(names.len());
    for name in names {
        let bytes = std::fs::read(directory.join(&name))?;
        let base = &name[..name.len() - ".jsonl".len()];
        let meta_call = std::fs::read(directory.join(format!("{base}.meta.json")))
            .ok()
            .and_then(|meta| serde_json::from_slice::<Value>(&meta).ok())
            .and_then(|meta| {
                meta.get("toolUseId")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            });
        files.push(SubagentFile {
            agent_id: base.strip_prefix("agent-").unwrap_or(base).to_owned(),
            path: format!("{stem}/subagents/{name}"),
            digest: Digest::of(&bytes),
            text: String::from_utf8_lossy(&bytes).into_owned(),
            meta_call,
        });
    }
    Ok(files)
}

/// Reads one on-disk Claude Code session, and its sub-agent files, into protocol events.
#[derive(Debug, Clone)]
pub struct SessionReader {
    transcript: TranscriptRef,
    attestation: HermeticAttestation,
}

/// One line of one file, parsed where it parses.
struct Line<'a> {
    number: u64,
    raw: &'a str,
    record: Option<Record>,
}

impl SessionReader {
    /// A reader carrying this transcript reference and attestation into `session.started`.
    ///
    /// A replay is not a launch: every `tool.requested` carries [`Seam::None`], and the caller
    /// passes the attestation that claims nothing (story:project-reads-vendor-transcripts).
    #[must_use]
    pub fn new(transcript: TranscriptRef, attestation: HermeticAttestation) -> Self {
        Self {
            transcript,
            attestation,
        }
    }

    /// The whole session, each emission beside the 1-based line of `session` it is cited at.
    ///
    /// The session's own events cite the line they were read from. A sub-agent's events, and the
    /// warning that opens its block, cite the session file's last line: they are read after the
    /// whole session file, as an event a reader owes at end of record is (amendment a28). An
    /// `opaque` event inside a sub-agent block carries its own line of the sub-agent file, which
    /// the block's warning names.
    ///
    /// The first event is `session.started` only when some record of the session file carries the
    /// session envelope's `sessionId`; a file with none is not a session file, and the caller
    /// refuses it by that rule.
    #[must_use]
    pub fn read(&self, session: &str, subagents: &[SubagentFile]) -> Vec<(u64, Emission)> {
        let parent = lines(session);
        let children: Vec<Vec<Line<'_>>> = subagents.iter().map(|file| lines(&file.text)).collect();
        let links = spawning_calls(parent.iter().chain(children.iter().flatten()));

        let mut seen_requests = BTreeSet::new();
        let mut out = Vec::new();
        let mut opening = self.opening(&parent);
        for line in &parent {
            let mut emissions = map_line(line, &mut seen_requests);
            if !emissions.is_empty()
                && let Some(started) = opening.take()
            {
                emissions.insert(0, Emission::untimed(started));
            }
            out.extend(
                emissions
                    .into_iter()
                    .map(|emission| (line.number, emission)),
            );
        }

        let last = parent.last().map_or(0, |line| line.number);
        for (file, lines) in subagents.iter().zip(&children) {
            let call = links
                .get(&file.agent_id)
                .cloned()
                .or_else(|| file.meta_call.clone());
            out.push((last, Emission::untimed(header(file, call.as_deref()))));
            for line in lines {
                out.extend(
                    map_line(line, &mut seen_requests)
                        .into_iter()
                        .map(|emission| (last, emission)),
                );
            }
        }
        out
    }

    /// The opening, assembled from the session file's envelope; `None` when no record carries
    /// `sessionId`.
    fn opening(&self, lines: &[Line<'_>]) -> Option<Event> {
        let records: Vec<&Record> = lines
            .iter()
            .filter_map(|line| line.record.as_ref())
            .collect();
        let first = |key: &str| records.iter().find_map(|record| str_field(record, key));
        let session_id = first("sessionId")?;
        let model = records
            .iter()
            .filter(|record| str_field(record, "type").as_deref() == Some("assistant"))
            .find_map(|record| {
                record
                    .get("message")
                    .and_then(|message| message.get("model"))
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            });
        Some(Event::SessionStarted {
            // Left to whoever holds the run's published rendering (design § 8.4 O6).
            available_operations: None,
            // The session file states neither what was offered nor what was withheld: `None` is
            // *it did not say*, never *nothing*.
            withheld: None,
            context_files: Vec::new(),
            adapter: ADAPTER_ID.to_string(),
            adapter_class: "harness".to_string(),
            harness_version: first("version"),
            session_id: Some(session_id),
            model,
            permission_mode: first("permissionMode"),
            credential_source: None,
            output_style: None,
            cwd: first("cwd"),
            offered_tools: None,
            slash_commands: None,
            skills: None,
            agents: None,
            plugins: None,
            mcp_servers: None,
            inputs_digest: None,
            transcript: self.transcript.clone(),
            hermetic: self.attestation.clone(),
        })
    }
}

fn lines(text: &str) -> Vec<Line<'_>> {
    text.lines()
        .zip(1..)
        .map(|(raw, number)| Line {
            number,
            raw,
            record: match serde_json::from_str::<Value>(raw) {
                Ok(Value::Object(record)) => Some(record),
                _ => None,
            },
        })
        .collect()
}

/// Every sub-agent id a session's results name, beside the call whose result named it.
///
/// The parent's `Agent` result carries the spawned sub-agent's id as `toolUseResult.agentId`; the
/// call it answers is that record's `tool_result` block's `tool_use_id`. The first link for an id
/// wins.
fn spawning_calls<'a>(lines: impl Iterator<Item = &'a Line<'a>>) -> BTreeMap<String, String> {
    let mut by_agent = BTreeMap::new();
    for record in lines.filter_map(|line| line.record.as_ref()) {
        let Some(agent) = record
            .get("toolUseResult")
            .and_then(|result| result.get("agentId"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        let call = record
            .get("message")
            .and_then(|message| message.get("content"))
            .and_then(Value::as_array)
            .and_then(|blocks| {
                blocks.iter().find_map(|block| {
                    (block.get("type").and_then(Value::as_str) == Some("tool_result"))
                        .then(|| block.get("tool_use_id").and_then(Value::as_str))
                        .flatten()
                })
            });
        if let Some(call) = call {
            by_agent
                .entry(agent.to_owned())
                .or_insert_with(|| call.to_owned());
        }
    }
    by_agent
}

/// The warning that opens a sub-agent's block, in the grammar
/// [`warning_code::VENDOR_SUBAGENT`] states.
fn header(file: &SubagentFile, call: Option<&str>) -> Event {
    let spawned = match call {
        Some(call) => format!("spawned by call `{call}`"),
        None => "spawned by no recorded call".to_owned(),
    };
    Event::Warning {
        code: warning_code::VENDOR_SUBAGENT.to_owned(),
        message: format!(
            "sub-agent `{}` {spawned}, read from `{}` (sha256:{})",
            file.agent_id,
            file.path,
            file.digest.as_str()
        ),
    }
}

/// One line's events; never empty for a line that is not blank.
fn map_line(line: &Line<'_>, seen_requests: &mut BTreeSet<String>) -> Vec<Emission> {
    if line.raw.trim().is_empty() {
        // Not a record, exactly as in `stream-json`: no `type` to name and no bytes to preserve.
        return Vec::new();
    }
    let Some(record) = &line.record else {
        return vec![Emission::untimed(opaque(line.raw, None, None, line.number))];
    };
    let mut events = map_record(record, line.number, seen_requests);
    if events.is_empty() {
        events.push(opaque(
            line.raw,
            str_field(record, "type"),
            str_field(record, "subtype"),
            line.number,
        ));
    }
    let at = str_field(record, "timestamp");
    events
        .into_iter()
        .map(|event| match &at {
            Some(timestamp) => Emission::at(timestamp.clone(), event),
            None => Emission::untimed(event),
        })
        .collect()
}

/// Which events one record becomes; empty means `opaque`.
fn map_record(record: &Record, line: u64, seen_requests: &mut BTreeSet<String>) -> Vec<Event> {
    let Some(message) = record.get("message").and_then(Value::as_object) else {
        return Vec::new();
    };
    match str_field(record, "type").as_deref() {
        Some("assistant") => {
            let request_id = str_field(record, "requestId");
            let mut events = assistant_blocks(message, request_id.as_ref(), Seam::None, line);
            let first_record_of_request = request_id
                .as_ref()
                .is_none_or(|id| seen_requests.insert(id.clone()));
            if first_record_of_request {
                events.extend(message_usage(message, request_id));
            }
            events
        }
        Some("user") => {
            let flagged = |key: &str| record.get(key).and_then(Value::as_bool).unwrap_or(false);
            let origin = if flagged("isMeta") || flagged("isSynthetic") {
                "synthetic"
            } else {
                "user"
            };
            let tool_use_result = record.get("toolUseResult").filter(|value| !value.is_null());
            user_message(message, origin, tool_use_result, line)
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use metaharness_protocol::{EventStream, HermeticMode, RunId};

    const SESSION: &str = "5e551011-0000-4000-8000-000000000001";

    fn fixture_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/golden/session")
    }

    fn reader() -> SessionReader {
        SessionReader::new(
            TranscriptRef {
                path: Some("session.jsonl".to_owned()),
                digest: None,
                bytes: None,
            },
            HermeticAttestation::none(HermeticMode::Off),
        )
    }

    fn events(session: &str, subagents: &[SubagentFile]) -> Vec<(u64, Event)> {
        reader()
            .read(session, subagents)
            .into_iter()
            .map(|(line, emission)| (line, emission.event))
            .collect()
    }

    const ENVELOPE: &str = r#""sessionId":"s-1","version":"2.1.309","cwd":"/fixture/w""#;

    /// Done when 2, second half: a record of a `type` nobody has heard of yields `opaque`, with
    /// the digest of its bytes and its 1-based line.
    #[test]
    fn a_record_of_an_unknown_type_is_opaque_with_its_digest_and_line() {
        let unknown = r#"{"type":"invented-by-a-later-release","sessionId":"s-1","x":1}"#;
        let session = format!(
            "{{\"type\":\"user\",{ENVELOPE},\"message\":{{\"role\":\"user\",\"content\":\"hi\"}}}}\n{unknown}\n"
        );
        let read = events(&session, &[]);
        let (line, opaque) = read.last().expect("an event").clone();
        assert_eq!(line, 2);
        let Event::Opaque {
            vendor_type,
            vendor_subtype,
            digest,
            source_line,
        } = opaque
        else {
            panic!("expected opaque, got {opaque:?}");
        };
        assert_eq!(vendor_type.as_deref(), Some("invented-by-a-later-release"));
        assert_eq!(vendor_subtype, None);
        assert_eq!(digest, Digest::of(unknown.as_bytes()));
        assert_eq!(source_line, Some(2));
    }

    /// Total by construction: every non-blank line of a session yields at least one event, a line
    /// that is not JSON included, and the opening rides on the first line that yields any.
    #[test]
    fn every_record_yields_at_least_one_event_and_the_session_opens_first() {
        let session = format!(
            "{{\"type\":\"custom-title\",\"customTitle\":\"t\",\"sessionId\":\"s-1\"}}\n\nnot json\n\
             {{\"type\":\"user\",{ENVELOPE},\"message\":{{\"role\":\"user\",\"content\":\"hi\"}}}}\n\
             {{\"type\":\"assistant\",{ENVELOPE}}}\n"
        );
        let read = events(&session, &[]);
        let lines: BTreeSet<u64> = read.iter().map(|(line, _)| *line).collect();
        assert_eq!(lines, BTreeSet::from([1, 3, 4, 5]), "{read:?}");
        let Event::SessionStarted {
            session_id,
            harness_version,
            cwd,
            adapter,
            ..
        } = &read[0].1
        else {
            panic!("the first event opens the session: {read:?}");
        };
        assert_eq!(session_id.as_deref(), Some("s-1"));
        assert_eq!(harness_version.as_deref(), Some("2.1.309"));
        assert_eq!(cwd.as_deref(), Some("/fixture/w"));
        assert_eq!(adapter, ADAPTER_ID);
        assert!(
            matches!(read[1].1, Event::Opaque { .. }),
            "the title record itself"
        );
        assert!(
            matches!(read[2].1, Event::Opaque { .. }),
            "the line that is not JSON"
        );
        assert!(
            matches!(read.last().expect("an event").1, Event::Opaque { .. }),
            "an assistant record without a message"
        );
    }

    /// A file whose records carry no `sessionId` is not a session file, and the reader does not
    /// invent an opening for it.
    #[test]
    fn a_file_without_the_session_envelope_does_not_open_a_session() {
        let read = events(
            r#"{"type":"system","subtype":"init","session_id":"s"}"#,
            &[],
        );
        assert!(
            !read
                .iter()
                .any(|(_, event)| matches!(event, Event::SessionStarted { .. }))
        );
    }

    /// One API response written over several records repeats its usage on each; it is billed once.
    #[test]
    fn a_request_split_over_records_owes_one_usage_event() {
        let record = |block: &str| {
            format!(
                "{{\"type\":\"assistant\",{ENVELOPE},\"requestId\":\"req-1\",\"message\":{{\"model\":\"m\",\"content\":[{block}],\"usage\":{{\"input_tokens\":5,\"output_tokens\":7}}}}}}"
            )
        };
        let session = format!(
            "{}\n{}\n",
            record(r#"{"type":"thinking","thinking":"plan"}"#),
            record(r#"{"type":"text","text":"done"}"#)
        );
        let read = events(&session, &[]);
        let usages = read
            .iter()
            .filter(|(_, event)| matches!(event, Event::Usage { .. }))
            .count();
        assert_eq!(usages, 1, "{read:?}");
        assert!(
            read.iter()
                .any(|(_, event)| matches!(event, Event::Thinking { .. }))
        );
        assert!(read.iter().any(|(_, event)| matches!(event, Event::Text { request_id, .. } if request_id.as_deref() == Some("req-1"))));
    }

    /// The session envelope's spellings: `toolUseResult` rides on the result, and `isMeta` text
    /// is the harness's, not the operator's.
    #[test]
    fn the_session_envelopes_spellings_are_read() {
        let session = format!(
            "{{\"type\":\"user\",{ENVELOPE},\"message\":{{\"role\":\"user\",\"content\":[{{\"type\":\"tool_result\",\"tool_use_id\":\"c-1\",\"content\":\"ok\"}}]}},\"toolUseResult\":{{\"stdout\":\"ok\"}}}}\n\
             {{\"type\":\"user\",{ENVELOPE},\"isMeta\":true,\"message\":{{\"role\":\"user\",\"content\":\"note\"}}}}\n"
        );
        let read = events(&session, &[]);
        assert!(read.iter().any(|(_, event)| matches!(
            event,
            Event::ToolResult { call_id, tool_use_result: Some(result), .. }
                if call_id == "c-1" && result["stdout"] == "ok"
        )));
        assert!(read.iter().any(|(_, event)| matches!(
            event,
            Event::Injection { origin: Some(origin), .. } if origin == "synthetic"
        )));
    }

    fn subagent(id: &str, text: &str, meta_call: Option<&str>) -> SubagentFile {
        SubagentFile {
            agent_id: id.to_owned(),
            path: format!("s-1/subagents/agent-{id}.jsonl"),
            digest: Digest::of(text.as_bytes()),
            text: text.to_owned(),
            meta_call: meta_call.map(str::to_owned),
        }
    }

    fn headers(read: &[(u64, Event)]) -> Vec<String> {
        read.iter()
            .filter_map(|(_, event)| match event {
                Event::Warning { code, message } if code == warning_code::VENDOR_SUBAGENT => {
                    Some(message.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// The result's `agentId` links a sub-agent to its call, a nested sub-agent is linked through
    /// its sibling's result, the meta file's `toolUseId` is the fallback, and a sub-agent nothing
    /// links says so rather than borrowing a call.
    #[test]
    fn each_subagent_block_names_the_call_that_spawned_it() {
        let result = |call: &str, agent: &str| {
            format!(
                "{{\"type\":\"user\",{ENVELOPE},\"message\":{{\"role\":\"user\",\"content\":[{{\"type\":\"tool_result\",\"tool_use_id\":\"{call}\",\"content\":\"done\"}}]}},\"toolUseResult\":{{\"agentId\":\"{agent}\"}}}}\n"
            )
        };
        let session = result("call-a", "a");
        let a = subagent(
            "a",
            &result("call-b", "b"),
            Some("ignored-when-the-result-links"),
        );
        let b = subagent(
            "b",
            "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"go\"}}\n",
            None,
        );
        let c = subagent(
            "c",
            "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"go\"}}\n",
            Some("call-c"),
        );
        let d = subagent(
            "d",
            "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"go\"}}\n",
            None,
        );
        let read = events(&session, &[a, b, c, d]);
        let found = headers(&read);
        assert_eq!(found.len(), 4, "{found:?}");
        assert!(
            found[0].starts_with(
                "sub-agent `a` spawned by call `call-a`, read from `s-1/subagents/agent-a.jsonl`"
            ),
            "{}",
            found[0]
        );
        assert!(
            found[1].starts_with("sub-agent `b` spawned by call `call-b`"),
            "{}",
            found[1]
        );
        assert!(
            found[2].starts_with("sub-agent `c` spawned by call `call-c`"),
            "{}",
            found[2]
        );
        assert!(
            found[3].starts_with("sub-agent `d` spawned by no recorded call"),
            "{}",
            found[3]
        );
        assert!(
            read.iter().all(|(line, _)| *line == 1),
            "every event of a one-line session cites its line: {read:?}"
        );
    }

    /// The sub-agent files beside a session are found, in name order, with their meta links.
    #[test]
    fn the_subagent_files_beside_a_session_are_read_in_name_order() {
        let found = read_subagents(&fixture_dir().join(format!("{SESSION}.jsonl")))
            .expect("the fixture reads");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].agent_id, "a0f1c2d3e4b5a697");
        assert_eq!(
            found[0].path,
            format!("{SESSION}/subagents/agent-a0f1c2d3e4b5a697.jsonl")
        );
        assert_eq!(found[0].meta_call, None);

        let scratch = tempfile::tempdir().expect("a scratch directory");
        let session = scratch.path().join("s.jsonl");
        assert_eq!(
            read_subagents(&session).expect("no directory is no sub-agents"),
            Vec::<SubagentFile>::new()
        );
        let directory = scratch.path().join("s/subagents");
        std::fs::create_dir_all(&directory).expect("mkdir");
        std::fs::write(directory.join("agent-z.jsonl"), "{}\n").expect("write");
        std::fs::write(directory.join("agent-y.jsonl"), "{}\n").expect("write");
        std::fs::write(
            directory.join("agent-y.meta.json"),
            r#"{"toolUseId":"call-y"}"#,
        )
        .expect("write");
        let found = read_subagents(&session).expect("reads");
        let ids: Vec<&str> = found.iter().map(|file| file.agent_id.as_str()).collect();
        assert_eq!(ids, ["y", "z"]);
        assert_eq!(found[0].meta_call.as_deref(), Some("call-y"));
    }

    /// The committed session, read the way `metaharness project --from claude-code-session` reads
    /// it, framed under the file's stem: the committed expected stream, byte for byte.
    fn replay_fixture() -> String {
        let path = fixture_dir().join(format!("{SESSION}.jsonl"));
        let bytes = std::fs::read(&path).expect("the fixture is committed");
        let transcript = TranscriptRef {
            path: Some(format!("{SESSION}.jsonl")),
            digest: Some(Digest::of(&bytes)),
            bytes: u64::try_from(bytes.len()).ok(),
        };
        let subagents = read_subagents(&path).expect("the sub-agents read");
        let text = std::str::from_utf8(&bytes).expect("UTF-8");
        let mut stream = EventStream::new(RunId::new(SESSION));
        let mut out = String::new();
        for (_, emission) in
            SessionReader::new(transcript, HermeticAttestation::none(HermeticMode::Off))
                .read(text, &subagents)
        {
            out.push_str(&serde_json::to_string(&stream.stamp(emission)).expect("serializes"));
            out.push('\n');
        }
        out
    }

    fn expected_path() -> std::path::PathBuf {
        fixture_dir().join(format!("{SESSION}.expected.jsonl"))
    }

    #[test]
    fn the_committed_session_reads_to_its_committed_expected_stream() {
        let expected =
            std::fs::read_to_string(expected_path()).expect("the expectation is committed");
        assert_eq!(replay_fixture(), expected);
    }

    /// Done when 2, first half: every record type of the fixture maps to a named event or to
    /// `opaque`, and the ones the protocol has no word for are the opaque ones.
    #[test]
    fn every_record_type_of_the_fixture_is_named_or_opaque() {
        let stream = replay_fixture();
        let mut opaque_types = BTreeSet::new();
        let mut names = BTreeSet::new();
        for line in stream.lines() {
            let event: Value = serde_json::from_str(line).expect("JSON");
            let name = event["event"].as_str().expect("a name").to_owned();
            if name == "opaque" {
                opaque_types.insert(event["vendor_type"].as_str().unwrap_or_default().to_owned());
            }
            names.insert(name);
        }
        assert_eq!(
            opaque_types,
            BTreeSet::from(
                [
                    "attachment",
                    "custom-title",
                    "last-prompt",
                    "permission-mode",
                    "system"
                ]
                .map(str::to_owned)
            )
        );
        for name in [
            "session.started",
            "injection",
            "thinking",
            "tool.requested",
            "tool.result",
            "text",
            "usage",
            "warning",
        ] {
            assert!(names.contains(name), "{name} in {names:?}");
        }
    }

    /// Writes the expected stream from the committed fixture. `#[ignore]`d because it writes into
    /// the source tree; run it after a deliberate change to the mapping, then read the diff.
    #[test]
    #[ignore = "writes fixtures/golden/session/<session>.expected.jsonl; run after a deliberate change"]
    fn regenerate_the_session_expectation() {
        std::fs::write(expected_path(), replay_fixture()).expect("the expectation is written");
    }
}
