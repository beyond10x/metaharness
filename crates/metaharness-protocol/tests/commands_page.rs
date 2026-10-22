//! The public protocol pages, read by the decoder they document.
//!
//! A line a reader copies from a page is a line they will send, or a line they will expect to
//! read. If the decoder refuses it, the page taught them something false.

use metaharness_protocol::{
    COMMAND_FORMAT, Command, CommandLine, CommandOutcome, Decision, EVENT_FORMAT, Event, EventLine,
    RefusalCode, command_line_id, parse_command_line, parse_event_line,
};

const COMMANDS_PAGE: &str = include_str!("../../../website/docs/protocol/commands.md");
const EVENTS_PAGE: &str = include_str!("../../../website/docs/protocol/events.md");

/// A code fence as Markdown reads one: at most three spaces of indent, then three or more of one
/// fence character, then the info string. Returns the character, the run's length and the info
/// string. A backtick fence whose info string holds a backtick is not a fence.
fn fence(line: &str) -> Option<(char, usize, &str)> {
    let rest = line.trim_start_matches(' ');
    if line.len() - rest.len() > 3 {
        return None;
    }
    let character = rest.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let length = rest.len() - rest.trim_start_matches(character).len();
    if length < 3 {
        return None;
    }
    let info = rest[length..].trim();
    if character == '`' && info.contains('`') {
        return None;
    }
    Some((character, length, info))
}

/// Every non-blank line of every fenced block on `page` whose info string starts with `json`,
/// numbered from 1. A block closes on a fence of its own character, at least as long as the one
/// that opened it, with no info string; anything else inside it is content. Panics on a fence the
/// page leaves open.
fn json_lines(page: &str) -> Vec<(usize, &str)> {
    let mut open: Option<(char, usize, bool)> = None;
    let mut lines = Vec::new();
    for (number, line) in page.lines().enumerate() {
        match open {
            None => {
                if let Some((character, length, info)) = fence(line) {
                    open = Some((character, length, info.starts_with("json")));
                }
            }
            Some((character, length, json)) => {
                let closes = fence(line).is_some_and(|(close, run, info)| {
                    close == character && run >= length && info.is_empty()
                });
                if closes {
                    open = None;
                } else if json && !line.trim().is_empty() {
                    lines.push((number + 1, line));
                }
            }
        }
    }
    assert!(open.is_none(), "the page leaves a code fence open");
    lines
}

/// The page's JSON lines, decoded by the production decoder chosen by each line's format tag.
/// Panics, naming the page and line, on a line that is not JSON, carries no format this build
/// reads, or does not decode, and on a page that shows no JSON line at all.
fn decoded(name: &str, page: &str) -> (Vec<CommandLine>, Vec<EventLine>) {
    let lines = json_lines(page);
    assert!(!lines.is_empty(), "{name} shows no json example");
    let mut commands = Vec::new();
    let mut events = Vec::new();
    for (number, line) in lines {
        let format = serde_json::from_str::<serde_json::Value>(line)
            .unwrap_or_else(|error| panic!("{name}:{number} is not JSON: {error}"))
            .get("format")
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string);
        match format.as_deref() {
            Some(COMMAND_FORMAT) => commands.push(
                parse_command_line(line).unwrap_or_else(|error| panic!("{name}:{number}: {error}")),
            ),
            Some(EVENT_FORMAT) => events.push(
                parse_event_line(line).unwrap_or_else(|error| panic!("{name}:{number}: {error}")),
            ),
            other => panic!("{name}:{number} carries no format this build reads: {other:?}"),
        }
    }
    (commands, events)
}

/// The reader takes a block of either fence character, of any fence length, whose info string
/// starts with `json`, and skips every other block. A shorter fence of the same character inside a
/// longer one is content, not a close.
#[test]
fn the_page_reader_takes_every_block_whose_info_string_starts_with_json() {
    let page = "\
intro

```json
{\"a\":1}
```

~~~json
{\"b\":2}
~~~

````json title=\"nested\"
{\"c\":3}
```
{\"d\":4}
````

  ```jsonc
{\"e\":5}
  ```

```console
$ not json
```

~~~~ text
{\"skipped\":true}
~~~~
";
    let lines: Vec<&str> = json_lines(page).into_iter().map(|(_, line)| line).collect();
    assert_eq!(
        lines,
        [
            "{\"a\":1}",
            "{\"b\":2}",
            "{\"c\":3}",
            "```",
            "{\"d\":4}",
            "{\"e\":5}"
        ]
    );
}

/// A page whose examples the reader cannot see is not a page the test has checked. One with no
/// `json` block at all fails rather than passing for having nothing to decode.
#[test]
#[should_panic(expected = "shows no json example")]
fn a_page_with_no_json_block_fails_rather_than_passing_empty() {
    decoded(
        "empty.md",
        "# Nothing\n\n```console\n$ metaharness run claude\n```\n",
    );
}

/// A page that leaves a fence open hides everything after it from the reader.
#[test]
#[should_panic(expected = "leaves a code fence open")]
fn a_page_that_leaves_a_fence_open_fails() {
    json_lines("intro\n\n~~~json\n{\"a\":1}\n");
}

/// Every JSON line the commands page shows is one the production decoder takes. The page once
/// showed `"decision":"allow"`, which the decoder refuses because a decision is an object tagged
/// by its own `decision` field; a reader who copied it got a refusal back.
#[test]
fn every_json_example_on_the_commands_page_parses_with_the_production_decoder() {
    let (commands, events) = decoded("commands.md", COMMANDS_PAGE);

    let decisions: Vec<&Decision> = commands
        .iter()
        .filter_map(|line| match &line.command {
            Command::ToolDecide { decision, .. } => Some(decision),
            _ => None,
        })
        .collect();
    assert!(
        decisions.contains(&&Decision::Allow),
        "the page shows an allow"
    );
    assert!(
        decisions
            .iter()
            .any(|decision| matches!(decision, Decision::Deny { .. })),
        "the page shows a deny"
    );
    assert!(
        events.iter().any(|line| matches!(
            &line.event,
            Event::CommandResult {
                outcome: CommandOutcome::Refused { refused },
                ..
            } if refused.code == RefusalCode::Malformed
        )),
        "the page shows the answer to a line that does not parse"
    );
}

/// The `MALFORMED` answer the commands page quotes is the one the decoder gives for the page's
/// first allow written flat: the same id, and the reason word for word.
#[test]
fn the_malformed_answer_on_the_commands_page_is_the_decoders_own() {
    let lines = json_lines(COMMANDS_PAGE);
    let nested = r#""decision":{"decision":"allow"}"#;
    let allow = lines
        .iter()
        .map(|(_, line)| *line)
        .find(|line| line.contains(nested))
        .expect("the page shows an allow");
    let flat = allow.replace(nested, r#""decision":"allow""#);
    let reason = parse_command_line(&flat)
        .expect_err("the flat allow does not parse")
        .to_string();

    let (_, events) = decoded("commands.md", COMMANDS_PAGE);
    let shown: Vec<(&str, &str)> = events
        .iter()
        .filter_map(|line| match &line.event {
            Event::CommandResult {
                id,
                outcome: CommandOutcome::Refused { refused },
            } if refused.code == RefusalCode::Malformed => {
                Some((id.as_str(), refused.reason.as_str()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        shown,
        vec![(command_line_id(&flat).as_str(), reason.as_str())],
        "the page quotes the answer the decoder gives"
    );
}

/// Every JSON line the events page shows is one the production decoder takes. Its one example
/// once lacked `seq`, `run`, `input`, `decision_required` and `seam`, and the decoder refused it.
#[test]
fn every_json_example_on_the_events_page_parses_with_the_production_decoder() {
    let (commands, events) = decoded("events.md", EVENTS_PAGE);
    assert!(commands.is_empty(), "the events page shows no command");
    assert!(
        events
            .iter()
            .any(|line| matches!(line.event, Event::ToolRequested { .. })),
        "the page shows a tool.requested"
    );
}
