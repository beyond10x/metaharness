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
    pub fn new(
        version: Option<String>,
        model: Option<String>,
        cwd: Option<String>,
    ) -> Self {
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
        let kind = value.get("kind").and_then(Value::as_str).unwrap_or_default();
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
                    offered_tools: value
                        .get("published_tools")
                        .and_then(Value::as_array)
                        .map(|tools| {
                            tools
                                .iter()
                                .filter_map(|tool| tool.as_str().map(ToOwned::to_owned))
                                .collect()
                        }),
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
                    // The gateway relays bytes and reports no price. `None` is the honest answer
                    // and a zero would be a lie about a run that cost money.
                    cost_usd: None,
                },
            })],
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
                    num_turns: stop.get("turns").and_then(Value::as_u64),
                    duration_ms: None,
                    duration_api_ms: None,
                    ttft_ms: None,
                    time_to_request_ms: None,
                    total_cost_usd: None,
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
            // `turn-started`, `tool-arguments-delta`, `approval-required`, `approval-resolved` and
            // `warning` have no counterpart any expectation reads. Carried across rather than
            // dropped (design D4): the failure that costs the most is a checker reporting *the tool
            // was never called* when what happened is that it stopped being able to see tool calls.
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
        assert!(slash_commands.is_none(), "the loop has none, so it states none");
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
            seam.control_line(&Command::Halt { reason: "stop".to_owned() }).is_none(),
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
        assert!(matches!(event, Event::Opaque { vendor_subtype: None, .. }));
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
        let event = one(&mut *seam, r#"{"kind":"finished","stop":{"kind":"completed"}}"#);
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

/// What this adapter can and cannot do, published as a value.
///
/// **Observe only, and every tier that implies a decision is `Unverified` rather than
/// `Delivered`.** There is no registration seam, no hook, no control request: the loop decides in
/// process and this adapter reads what it did. Declaring a tier delivered here would let an
/// embedder ask for a decision mode that silently does nothing, which is the failure the whole
/// capability document exists to prevent.
pub fn capabilities() -> metaharness_protocol::Capabilities {
    use metaharness_protocol::{
        AdapterClass, AdapterId, Capabilities, COMMAND_NAMES, CommandSupport, RefusalCode, Tier,
        TierStatus,
    };
    use std::collections::BTreeMap;

    // Every command refused, and none of them is an oversight: `tool.decide` has nothing to answer
    // because nothing asks, and `interrupt` and `halt` would need a control wire the loop does not
    // have. An adapter that claimed them would leave an embedder believing a run could be stopped
    // through this seam.
    let commands: BTreeMap<String, CommandSupport> = COMMAND_NAMES
        .iter()
        .map(|name| {
            (
                (*name).to_string(),
                CommandSupport::Refused(RefusalCode::UnsupportedControl),
            )
        })
        .collect();

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
        // Nothing is rendered: this adapter publishes no neutral operation onto a vendor tool,
        // because the toolset is the loop's own and metaharness does not compose it.
        rendering: BTreeMap::new(),
    }
}
