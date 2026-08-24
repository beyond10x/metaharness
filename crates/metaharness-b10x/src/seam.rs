//! Reading the loop's record, one line at a time.
//!
//! The whole adapter, minus the launch. `b10x-harness --json` writes one JSON object per line and
//! this maps each onto the wire every arm of the evaluation is judged from.

use metaharness_protocol::{
    Command, Decision, DecisionCensus, Digest, Emission, Event, HarnessSeam, HermeticAttestation,
    Seam, SeamFactory, TranscriptRef, Usage,
};
use serde_json::Value;

use crate::{ADAPTER_CLASS, ADAPTER_ID};

/// Builds a seam once the launch plan has produced the two things only it can.
pub struct B10xSeams {
    /// The version the launch observed, for `session.started`.
    version: Option<String>,
    /// The model the caller asked for, so the opening record names one before the loop does.
    model: Option<String>,
    /// The workspace the run was pointed at.
    cwd: Option<String>,
}

impl B10xSeams {
    pub fn new(version: Option<String>, model: Option<String>, cwd: Option<String>) -> Self {
        Self {
            version,
            model,
            cwd,
        }
    }
}

impl SeamFactory for B10xSeams {
    fn build(
        &mut self,
        transcript: TranscriptRef,
        attestation: HermeticAttestation,
        seam: Seam,
    ) -> Box<dyn HarnessSeam> {
        Box::new(B10xSeam {
            version: self.version.clone(),
            model: self.model.clone(),
            cwd: self.cwd.clone(),
            transcript,
            attestation,
            // Recorded and not used to decide anything: this adapter adjudicates nothing, and the
            // tier it was built at is a fact about the launch that a reader may want.
            control: seam,
            line: 0,
            started: false,
            ended: false,
            census: DecisionCensus::default(),
            spent_micro_usd: None,
        })
    }
}

/// One run's reader.
pub struct B10xSeam {
    version: Option<String>,
    model: Option<String>,
    cwd: Option<String>,
    transcript: TranscriptRef,
    attestation: HermeticAttestation,
    control: Seam,
    line: u64,
    started: bool,
    ended: bool,
    census: DecisionCensus,
    /// What the run has cost so far, in millionths of a US dollar, summed from the loop's own
    /// per-turn figures.
    ///
    /// [`None`] until a turn is priced, and `None` at the end means no rate card covered this run
    /// — which reaches `session.ended.total_cost_usd` as `null` rather than as a zero.
    spent_micro_usd: Option<u64>,
}

impl B10xSeam {
    /// The control tier this seam was built at.
    ///
    /// Recorded and never acted on: this adapter adjudicates nothing, and the tier is a fact about
    /// the launch a reader may want rather than an input to a decision that is not taken.
    pub fn control(&self) -> Seam {
        self.control
    }

    /// Whether an opening record has been read.
    pub fn started(&self) -> bool {
        self.started
    }

    /// Whether a terminal record has been read.
    ///
    /// `false` at the end of a stream means the run stopped without one, and nothing here invents
    /// a substitute.
    pub fn ended(&self) -> bool {
        self.ended
    }

    fn opaque(&self, line: &str, subtype: Option<&str>) -> Emission {
        Emission::untimed(Event::Opaque {
            vendor_type: Some(ADAPTER_ID.to_owned()),
            vendor_subtype: subtype.map(ToOwned::to_owned),
            digest: Digest::of(line.as_bytes()),
            source_line: Some(self.line),
        })
    }
}

impl HarnessSeam for B10xSeam {
    fn push_line(&mut self, line: &str) -> Vec<Emission> {
        self.line += 1;
        if line.trim().is_empty() {
            return Vec::new();
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return vec![self.opaque(line, None)];
        };
        let kind = value
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let string = |field: &str| {
            value
                .get(field)
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        };
        let number = |field: &str| value.get(field).and_then(Value::as_u64);

        match kind {
            "started" => {
                self.started = true;
                vec![Emission::untimed(Event::SessionStarted {
                    adapter: ADAPTER_ID.to_owned(),
                    adapter_class: ADAPTER_CLASS.to_owned(),
                    harness_version: self.version.clone(),
                    // The loop keeps no session identity of its own: every turn replays the whole
                    // conversation and nothing provider-side is retained.
                    session_id: None,
                    model: string("model").or_else(|| self.model.clone()),
                    permission_mode: None,
                    // Never ambient. `b10x-harness` reads no credential it was not pointed at, and
                    // this is that fact travelling onto the wire rather than a guess.
                    credential_source: Some("named".to_owned()),
                    output_style: None,
                    cwd: self.cwd.clone(),
                    // The catalogue behind the verbs, which is the only thing that differs from
                    // run to run here. The loop states it; this passes it through.
                    available_operations: value.get("operations").and_then(Value::as_array).map(
                        |names| {
                            names
                                .iter()
                                .filter_map(|name| name.as_str().map(ToOwned::to_owned))
                                .collect()
                        },
                    ),
                    offered_tools: value.get("published_tools").and_then(Value::as_array).map(
                        |tools| {
                            tools
                                .iter()
                                .filter_map(|tool| tool.as_str().map(ToOwned::to_owned))
                                .collect()
                        },
                    ),
                    // Absent because the loop has none of these, not because nobody looked.
                    slash_commands: None,
                    skills: None,
                    agents: None,
                    plugins: None,
                    mcp_servers: None,
                    inputs_digest: None,
                    transcript: self.transcript.clone(),
                    hermetic: self.attestation.clone(),
                })]
            }
            "text-delta" => vec![Emission::untimed(Event::Text {
                text: string("text").unwrap_or_default(),
                request_id: None,
            })],
            "tool-requested" => vec![Emission::untimed(Event::ToolRequested {
                // Left empty here on purpose, exactly as `operations` is: what a call touches is
                // resolved by whoever holds the run's published rendering, and an adapter that
                // answered for itself would be a second owner of one rule (design § 8.4 O6).
                subjects: Vec::new(),
                // Left empty here on purpose: the resolution needs the adapter\'s *published*
                // rendering, which the loop holds and an adapter must not (design § 8.4 O6).
                operations: Vec::new(),
                call_id: string("call_id").unwrap_or_default(),
                name: string("name").unwrap_or_default(),
                input: value.get("arguments").cloned().unwrap_or(Value::Null),
                // **The claim this adapter exists to make honestly.** Nothing adjudicated this
                // call, because the toolset it was drawn from is the policy. Emitting `false` and
                // `Seam::None` says *nobody decided this*, which is a fact; omitting the fields
                // would leave a reader to infer it from silence.
                decision_required: false,
                deadline_ms: None,
                seam: Seam::None,
            })],
            "tool-completed" => vec![Emission::untimed(Event::ToolResult {
                call_id: string("call_id").unwrap_or_default(),
                is_error: value.get("failed").and_then(Value::as_bool),
                // The loop's record names the outcome and not its bytes; a content field invented
                // here would be this adapter writing the tool's answer for it.
                content: None,
                bytes: None,
                tool_use_result: None,
            })],
            "usage" => vec![Emission::untimed(Event::Usage {
                request_id: None,
                model: string("model").or_else(|| self.model.clone()),
                usage: Usage {
                    input_tokens: number("input_tokens"),
                    output_tokens: number("output_tokens"),
                    cache_read_input_tokens: number("cached_input_tokens"),
                    cache_creation_input_tokens: None,
                    service_tier: None,
                    thinking_tokens: None,
                    iterations: None,
                    speed: None,
                    // The loop prices a turn in its own `cost` record, which arrives on the next
                    // line — after this event is already written. Backfilling it would mean
                    // buffering the stream to edit an emission that has left, so the figure a
                    // reader compares arms on is stated once, as the run total on `session.ended`.
                    cost_usd: None,
                },
            })],
            // Folded into the run's total and carried across as opaque, on the rule every
            // unmapped line is carried by. It has no counterpart of its own: `Usage::cost_usd` is
            // for a slice the vendor priced, and this arrives a line too late to fill it.
            "cost" => {
                if let Some(micro) = number("micro_usd") {
                    self.spent_micro_usd =
                        Some(self.spent_micro_usd.unwrap_or(0).saturating_add(micro));
                }
                // Read and used; the run total goes out on `session.ended`. See the control-plane
                // arm below for why emitting nothing here is not a drop.
                Vec::new()
            }
            "finished" => {
                self.ended = true;
                let stop = value.get("stop").cloned().unwrap_or(Value::Null);
                let reason = stop
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_owned();
                vec![Emission::untimed(Event::SessionEnded {
                    is_error: Some(reason != "completed"),
                    subtype: Some(reason.clone()),
                    stop_reason: None,
                    terminal_reason: Some(reason),
                    api_error_status: None,
                    // The loop's own count, beside the stop rather than inside it. Only the two
                    // bound-bound stops carry one, so a reader asking how long a run was got an
                    // answer from a run that hit a ceiling and nothing from one that finished —
                    // and an advisory bound on run length could not decide a single completed run.
                    // The stop's figure stays as the fallback for a record written before this.
                    num_turns: number("turns").or_else(|| stop.get("turns").and_then(Value::as_u64)),
                    duration_ms: None,
                    duration_api_ms: None,
                    ttft_ms: None,
                    time_to_request_ms: None,
                    // What the run cost at the rates `--prices` declared, summed from the
                    // loop's own per-turn figures. `null` where no card priced it: the OpenAI
                    // Responses wire returns no price and neither does the catalogue behind Claude
                    // Code, which states a figure anyway. A subscription is not a reason for a run
                    // to be uncosted — but a run nobody gave rates to has no figure to state, and a
                    // zero would be a lie about one that cost money.
                    //
                    // Still not computed by metaharness (design § 4.1, D4): the multiplication is
                    // the loop's, exactly as Claude Code's own `total_cost_usd` is Claude Code's.
                    // This reads what the harness said, as it does for every other kind.
                    total_cost_usd: self.spent_micro_usd.and_then(|micro| {
                        serde_json::from_str(&b10x_harness_loop::micro_usd_as_decimal(micro)).ok()
                    }),
                    // The loop refuses a call the published set does not name, and that refusal is
                    // the loop's own. It is not a *permission* denial: nothing was permitted or
                    // withheld, the tool simply was not there.
                    permission_denials: Some(Vec::new()),
                    subagents_spawned: Some(0),
                    usage: None,
                    model_usage: None,
                    census: self.census.clone(),
                })]
            }
            // **Control plane, not opaque, and the difference was costing this arm its score.**
            //
            // `Opaque` means *I could not read this*, and a consumer treats it accordingly: an
            // unread event could have been the tool call an expectation was looking for, so every
            // count over the run goes `unk`. Sending the loop's own bookkeeping down that road put
            // 130 opaque events in a 12-call run, and the corpus answered `unk` for seven rows out
            // of eleven — about a stream it had read perfectly.
            //
            // A turn boundary and a warning are metaharness's own `CONTROL_PLANE_EVENTS`: understood,
            // projecting into no `trace-ir/1` family, and **not** uncertain. They are emitted as
            // what they are.
            "turn-started" => vec![Emission::untimed(Event::TurnStarted {
                turn: number("turn").and_then(|turn| u32::try_from(turn).ok()).unwrap_or(0),
                // Nothing narrowed this run: the toolset it drew from is the policy.
                frame_digest: None,
            })],
            "warning" => vec![Emission::untimed(Event::Warning {
                code: string("code").unwrap_or_default(),
                message: string("message").unwrap_or_default(),
            })],
            // Read, understood, and carrying nothing any `trace-ir/1` family models: a fragment of
            // a tool's arguments, an approval on a loop that adjudicates nothing, the rate card,
            // and a per-turn cost already folded into the run's total above.
            //
            // **Emitting nothing is not the drop design D4 forbids.** D4 protects an event nobody
            // could read; these were read. And the line itself is not lost: metaharness writes the
            // child's stdout to the run's transcript verbatim, so the raw record still holds every
            // one of them for anyone who wants to look.
            "tool-arguments-delta" | "approval-required" | "approval-resolved" | "rates" => {
                Vec::new()
            }
            // A kind this build does not know. Opaque, and every rule above still applies to it.
            other => vec![self.opaque(line, Some(other))],
        }
    }

    fn finish(&mut self) -> Vec<Emission> {
        // No terminal record is invented for a run that stopped without one. A checker reading a
        // synthesised `completed` would call a killed run a finished one, and the honest answer to
        // *how did this end* is that nobody found out.
        Vec::new()
    }

    fn set_census(&mut self, census: DecisionCensus) {
        self.census = census;
    }

    fn decision_line(&self, call_id: &str, decision: &Decision) -> String {
        // **Unreachable, and it says so rather than doing something.** This adapter runs in observe
        // mode; no call ever asks, so no decision ever needs a line. Returning a plausible one
        // would leave a wire that could carry an adjudication this arm must not have.
        format!(
            "{{\"unreachable\":\"the b10x adapter observes and does not decide\",\
             \"call_id\":{call_id:?},\"well_formed\":{}}}",
            decision.is_well_formed()
        )
    }

    fn control_line(&self, _command: &Command) -> Option<String> {
        // The loop has no control wire. `None` is the protocol's own way of saying *this command
        // reaches the child by no line at all*, which is the truth here rather than a gap.
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use metaharness_protocol::HermeticMode;

    fn seam() -> Box<dyn HarnessSeam> {
        B10xSeams::new(
            Some("0.1.0".to_owned()),
            Some("gpt-5.6-sol".to_owned()),
            Some("/work".to_owned()),
        )
        .build(
            TranscriptRef {
                path: None,
                digest: None,
                bytes: None,
            },
            HermeticAttestation::none(HermeticMode::Strict),
            Seam::None,
        )
    }

    fn one(seam: &mut dyn HarnessSeam, line: &str) -> Event {
        let mut emitted = seam.push_line(line);
        assert_eq!(emitted.len(), 1, "{line}");
        emitted.remove(0).event
    }

    #[test]
    fn the_opening_record_names_the_class_the_protocol_already_had() {
        // Design § 8.4 O5 in the other direction: a reader filtering on `adapter_class` must be
        // able to see that nothing here drives a vendor binary.
        let mut seam = seam();
        let event = one(
            &mut *seam,
            r#"{"kind":"started","model":"gpt-5.6-sol","published_tools":["workspace_read","run"]}"#,
        );
        let Event::SessionStarted {
            adapter,
            adapter_class,
            harness_version,
            offered_tools,
            credential_source,
            slash_commands,
            plugins,
            ..
        } = event
        else {
            panic!("an opening record")
        };
        assert_eq!(adapter, "b10x");
        assert_eq!(
            adapter_class, "direct_provider",
            "the protocol's own word for *the embedder holds the conversation*, not a synonym"
        );
        assert_eq!(harness_version.as_deref(), Some("0.1.0"));
        assert_eq!(
            offered_tools,
            Some(vec!["workspace_read".to_owned(), "run".to_owned()])
        );
        assert_eq!(
            credential_source.as_deref(),
            Some("named"),
            "the loop reads no credential it was not pointed at, and the wire says so"
        );
        assert!(
            slash_commands.is_none(),
            "the loop has none, so it states none"
        );
        assert!(plugins.is_none());
    }

    #[test]
    fn nobody_adjudicated_the_call_and_the_record_says_so_rather_than_omitting_it() {
        // The whole reason this adapter exists in observe mode. Arm `native` measures a run whose
        // toolset *is* the policy; a seam that decided its calls would measure arm `driven`.
        let mut seam = seam();
        let event = one(
            &mut *seam,
            r#"{"kind":"tool-requested","call_id":"c-1","name":"workspace_read","arguments":{"path":"a"}}"#,
        );
        let Event::ToolRequested {
            decision_required,
            seam: at,
            name,
            input,
            ..
        } = event
        else {
            panic!("a tool call")
        };
        assert!(!decision_required);
        assert_eq!(at, Seam::None, "no seam covered it, and that is the fact");
        assert_eq!(name, "workspace_read");
        assert_eq!(input["path"], "a");
    }

    #[test]
    fn the_adapter_refuses_to_decide_and_says_so_where_a_decision_would_go() {
        // Returning a plausible line would leave a wire that could carry an adjudication this arm
        // must not have.
        let seam = seam();
        let line = seam.decision_line("c-1", &Decision::Allow);
        assert!(line.contains("observes and does not decide"), "{line}");
        assert!(
            seam.control_line(&Command::Halt {
                reason: "stop".to_owned()
            })
            .is_none(),
            "the loop has no control wire, and `None` is the protocol's word for that"
        );
    }

    #[test]
    fn a_line_this_build_does_not_map_is_carried_across_with_its_own_line_number() {
        let mut seam = seam();
        seam.push_line(r#"{"kind":"started","model":"m","published_tools":[]}"#);
        let event = one(&mut *seam, r#"{"kind":"invented-later","detail":1}"#);
        let Event::Opaque {
            vendor_type,
            vendor_subtype,
            source_line,
            ..
        } = event
        else {
            panic!("carried across")
        };
        assert_eq!(vendor_type.as_deref(), Some("b10x"));
        assert_eq!(vendor_subtype.as_deref(), Some("invented-later"));
        assert_eq!(source_line, Some(2), "which line of the record said it");

        // Even a line that is not JSON at all: a reader that dropped it would be reporting a
        // shorter run than the one that happened.
        let event = one(&mut *seam, "not json");
        assert!(matches!(
            event,
            Event::Opaque {
                vendor_subtype: None,
                ..
            }
        ));
    }

    #[test]
    fn a_run_that_stopped_without_a_terminal_record_gets_no_invented_one() {
        let mut seam = seam();
        seam.push_line(r#"{"kind":"started","model":"m","published_tools":[]}"#);
        assert!(
            seam.finish().is_empty(),
            "a checker reading a synthesised `completed` would call a killed run a finished one"
        );
    }

    #[test]
    fn a_stop_that_is_not_completion_keeps_its_own_word_and_is_an_error() {
        let mut seam = seam();
        let event = one(
            &mut *seam,
            r#"{"kind":"finished","stop":{"kind":"budget-exhausted","turns":9}}"#,
        );
        let Event::SessionEnded {
            is_error,
            terminal_reason,
            num_turns,
            total_cost_usd,
            permission_denials,
            ..
        } = event
        else {
            panic!("a terminal record")
        };
        assert_eq!(is_error, Some(true));
        assert_eq!(terminal_reason.as_deref(), Some("budget-exhausted"));
        assert_eq!(num_turns, Some(9));
        assert!(
            total_cost_usd.is_none(),
            "the gateway reports no price; a zero would be a lie about a run that cost money"
        );
        assert_eq!(
            permission_denials,
            Some(Vec::new()),
            "the loop withheld nothing: a tool outside the set was never there to withhold"
        );
    }

    #[test]
    fn the_census_metaharness_kept_reaches_the_terminal_record() {
        // Set rather than computed by the reader, because the census counts what *metaharness*
        // decided and the loop's record cannot see it.
        let mut seam = seam();
        let mut census = DecisionCensus::default();
        census.abstained = 7;
        seam.set_census(census);
        let event = one(
            &mut *seam,
            r#"{"kind":"finished","stop":{"kind":"completed"}}"#,
        );
        let Event::SessionEnded { census, .. } = event else {
            panic!("a terminal record")
        };
        assert_eq!(
            census.abstained, 7,
            "abstained and not allowed: *we claimed nothing* is not *we let it through*"
        );
        assert_eq!(census.allowed, 0);
    }

    #[test]
    fn usage_carries_what_the_loop_reported_and_invents_none_of_the_rest() {
        let mut seam = seam();
        let event = one(
            &mut *seam,
            r#"{"kind":"usage","model":"gpt-5.6-sol","input_tokens":297,"output_tokens":25,"cached_input_tokens":0}"#,
        );
        let Event::Usage { usage, model, .. } = event else {
            panic!("usage")
        };
        assert_eq!(model.as_deref(), Some("gpt-5.6-sol"));
        assert_eq!(usage.input_tokens, Some(297));
        assert_eq!(usage.cache_read_input_tokens, Some(0));
        assert!(usage.thinking_tokens.is_none(), "the loop reports none");
        assert!(usage.cost_usd.is_none());
    }
}

/// The neutral-operation → tool table for this loop.
///
/// Every operation the b10x toolset serves, and `None` for the three it does not: this loop has no
/// web fetch, no skill mechanism, no subagents and no todo list, and saying so is the point — a
/// consumer reading `None` knows the run could not have done it, where a missing key would only
/// say nobody wrote one down.
///
/// `shell` renders to `run`, which is the one row worth pausing on. `run` is **not** a shell: it
/// takes an argv and a program from a declared set. It is the closest thing this loop has to the
/// neutral operation *start a process*, and rendering it here is what lets a consumer ask that
/// question of every harness in one vocabulary. What the operation admits is still the loop's own
/// business, and `exec.argv-only` is still true.
///
/// # What these names are, now that the model sees three verbs
///
/// The right-hand side is the **catalogue entry**, not the tool the model calls. Under the three
/// verbs every operation travels through `tool_invoke`, so a table mapping operations to tool names
/// would answer `tool_invoke` six times and lose exactly the distinction a reader wants. The entry
/// is the answer to *what is a write called here*, and [`metaharness_protocol::Event::ToolRequested`]
/// carries the operation itself for the reader who needs it per call.
///
/// # Read from the catalogue rather than written out again
///
/// This table used to be six string literals, and they went stale the day the three verbs landed:
/// it named `workspace_read`, `workspace_write`, `workspace_edit`, `workspace_list` and
/// `workspace_grep`, none of which had existed since. Nothing failed, because a rendering table
/// nobody cross-checks is a document that can only be wrong. It now comes from
/// [`b10x_harness_tools::entry_names`], which is the same function the catalogue builds itself
/// from, so the two cannot disagree.
fn rendering() -> std::collections::BTreeMap<String, Option<String>> {
    use metaharness_protocol::Operation;
    let entries = b10x_harness_tools::entry_names();
    let mut table: std::collections::BTreeMap<String, Option<String>> = Operation::PARAMETERLESS
        .iter()
        .map(|operation| {
            let name = operation.name();
            // `None` for an operation the catalogue has no entry for — no web fetch, no skill
            // mechanism, no subagents, no todo list. A consumer reading `None` knows the run could
            // not have done it, where a missing key would only say nobody wrote one down.
            let entry = entries.get(name).map(|entry| (*entry).to_string());
            (name.to_string(), entry)
        })
        .collect();
    table.insert("mcp.call".to_string(), None);
    table
}

/// What this adapter can and cannot do, published as a value.
///
/// **Observe only, and every tier that implies a decision is `Unverified` rather than
/// `Delivered`.** There is no registration seam, no hook, no control request: the loop decides in
/// process and this adapter reads what it did. Declaring a tier delivered here would let an
/// embedder ask for a decision mode that silently does nothing, which is the failure the whole
/// capability document exists to prevent.
pub fn capabilities() -> metaharness_protocol::Capabilities {
    use metaharness_protocol::{
        AdapterClass, AdapterId, COMMAND_NAMES, Capabilities, CommandSupport, RefusalCode, Tier,
        TierStatus,
    };
    use std::collections::BTreeMap;

    // Refused by default, and `tool.decide` stays refused for the reason above: nothing asks, so
    // there is nothing to answer, and claiming it would let an embedder select a decision mode
    // that silently does nothing.
    let mut commands: BTreeMap<String, CommandSupport> = COMMAND_NAMES
        .iter()
        .map(|name| {
            (
                (*name).to_string(),
                CommandSupport::Refused(RefusalCode::UnsupportedControl),
            )
        })
        .collect();
    // **`interrupt` and `halt` are honoured, and refusing them was a mistake that made this arm
    // unstartable.** `required_commands` puts both in every list — a run that cannot be stopped is
    // not a run anyone should start — so refusing them refused the run itself, before an endpoint
    // or a model was ever read. The arm could not be launched at all.
    //
    // The refusal's stated reason was that a stop "would need a control wire the loop does not
    // have". That was the wrong place to look. metaharness **spawns this child**, so stopping it is
    // a question about a process it owns, not about a channel into a loop: `Command::Halt` already
    // ends by killing the process and winding the run up, and it does that without asking the
    // adapter for anything.
    //
    // Both map to the same act here, and that is stated rather than dressed up: a `DirectProvider`
    // adapter has no finer-grained stop to offer, because there is no channel on which a running
    // turn could be told to end early. Claiming that `interrupt` leaves the child alive and merely
    // cancels its turn would be the actual lie.
    for name in ["interrupt", "halt"] {
        commands.insert(name.to_string(), CommandSupport::Honoured);
    }

    Capabilities {
        adapter: AdapterId {
            id: crate::ADAPTER_ID.to_string(),
            // Not `Harness`. The protocol's own word for *the embedder holds the conversation and
            // calls a model API*, which until now nothing had been.
            class: AdapterClass::DirectProvider,
        },
        versions_pinned: crate::PINNED_VERSIONS
            .iter()
            .map(|version| (*version).to_string())
            .collect(),
        tiers: BTreeMap::from([
            (Tier::Registration, TierStatus::Unverified),
            (Tier::Call, TierStatus::Unverified),
            (Tier::Turn, TierStatus::Unverified),
            (Tier::Kill, TierStatus::Unverified),
        ]),
        commands,
        decision_modes: BTreeMap::from([
            ("observe".to_string(), TierStatus::Delivered),
            ("ask".to_string(), TierStatus::Unverified),
            ("frame".to_string(), TierStatus::Unverified),
        ]),
        // **Published, and the empty table this held for one commit was a mistake.** The rendering
        // is what lets anything downstream ask *did this run write a file* without knowing whose
        // verb a write is spelled with — design § 8.4 O6, *a rendering that only exists inside a
        // run cannot be read*. An adapter that published none forced every consumer to learn its
        // tool names, which is how a corpus ends up with `Bash` written into it.
        rendering: rendering(),
    }
}

#[cfg(test)]
mod turn_count_tests {
    use metaharness_protocol::{Event, HarnessSeam, Seam, SeamFactory, TranscriptRef};

    fn seam() -> Box<dyn HarnessSeam> {
        super::B10xSeams::new(None, None, None).build(
            TranscriptRef {
                path: None,
                digest: None,
                bytes: None,
            },
            metaharness_protocol::HermeticAttestation::none(
                metaharness_protocol::HermeticMode::Off,
            ),
            Seam::None,
        )
    }

    fn ended(line: &str) -> Option<u64> {
        seam().push_line(line).into_iter().find_map(|emission| match emission.event {
            Event::SessionEnded { num_turns, .. } => num_turns,
            _ => None,
        })
    }

    #[test]
    fn a_completed_run_reports_how_long_it_was() {
        // The count lives beside the stop, not inside it: only the two bound-bound stops ever
        // carried one, so a reader asking how long a run was got an answer from a run that hit a
        // ceiling and nothing from one that finished. An advisory bound on run length could not
        // decide a single completed run - which is exactly what a live scoring pass reported.
        assert_eq!(
            ended(r#"{"kind":"finished","stop":{"kind":"completed"},"turns":7}"#),
            Some(7)
        );
    }

    #[test]
    fn a_record_written_before_that_keeps_the_stops_own_figure() {
        // A bound-bound stop has always carried it, and losing it on a replay of an older capture
        // would be this seam taking a fact away.
        assert_eq!(
            ended(r#"{"kind":"finished","stop":{"kind":"budget-exhausted","turns":9}}"#),
            Some(9)
        );
    }
}

#[cfg(test)]
mod rendering_tests {
    #[test]
    fn every_operation_this_loop_serves_names_the_catalogue_entry_that_serves_it() {
        // The table is what lets a consumer ask *did this run write a file* without knowing whose
        // verb a write is spelled with. An adapter that published none forces every consumer to
        // learn its tool names, which is how a corpus ends up with one vendor's `Bash` in it.
        //
        // The literals here are the point of the test and not a duplicate of the source: they are
        // what pins the table to names that **exist**. The five it held before named tools that had
        // been gone since the three verbs landed, and the test agreed with them.
        let table = super::rendering();
        for (operation, entry) in [
            ("file.read", "file_read"),
            ("file.write", "file_write"),
            ("file.edit", "file_edit"),
            ("dir.list", "dir_list"),
            ("search", "search"),
            ("shell", "run"),
        ] {
            assert_eq!(
                table.get(operation).and_then(Option::as_deref),
                Some(entry),
                "{operation}"
            );
        }
    }

    #[test]
    fn the_table_names_only_entries_the_catalogue_actually_publishes() {
        // The check the literals above cannot make on their own: whatever this table says, it must
        // be a name the tool surface answers to. This is the assertion whose absence let five dead
        // names sit in a published capability document.
        let published: Vec<&str> = b10x_harness_tools::entry_names().into_values().collect();
        for (operation, entry) in super::rendering() {
            let Some(entry) = entry else { continue };
            assert!(
                published.contains(&entry.as_str()),
                "`{operation}` renders to `{entry}`, which the catalogue does not publish: \
                 {published:?}"
            );
        }
    }

    #[test]
    fn an_operation_this_loop_cannot_perform_is_named_and_answered_none() {
        // `None` says the run could not have done it. A missing key would only say nobody wrote
        // one down, and those are different facts about the same run.
        let table = super::rendering();
        for absent in [
            "web.read",
            "skill.load",
            "subagent.spawn",
            "task.todo",
            "mcp.call",
        ] {
            assert!(table.contains_key(absent), "{absent} is named");
            assert_eq!(table[absent], None, "{absent} is not served");
        }
    }
}
