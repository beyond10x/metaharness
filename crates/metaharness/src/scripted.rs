//! The scripted fake: **test support, shipped on purpose**.
//!
//! Every control vector, every deadline and every refusal code in this crate is driven through
//! this fake rather than through a real `claude`, which is what makes the C3 tier free (design
//! § 8.5). It is public because an embedder writing its own decision policy needs the same
//! harness to test that policy against, and a fake that lives in a `#[cfg(test)]` module cannot
//! be borrowed.
//!
//! Two things it records, because both are claims metaharness makes and neither is visible in an
//! event: **what was written to the child, in order** (design § 7.7 rule 1 — the decision goes
//! first) and **how many times a credential was copied** (Q13 — once per spawn, not once per
//! run).

use std::cell::RefCell;
use std::collections::{BTreeSet, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;

use metaharness_protocol::{
    Command, Decision, DecisionCensus, Digest, Emission, Event, HermeticAttestation, McpServerRef,
    PluginRef, Seam, TranscriptRef, Usage,
};
use serde_json::Value;

use crate::process::{HarnessProcess, LaunchPlanView, ProcessRunner};
use metaharness_protocol::HarnessSeam;

/// One step of a script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptStep {
    /// The child writes this line.
    Line(String),
    /// The child is blocked here until a decision for this call has been written to it.
    ///
    /// Modelled explicitly rather than by a real block, because the library surface is
    /// synchronous (design D10): a fake that really blocked would deadlock the single thread
    /// that is supposed to answer it.
    AwaitDecision {
        /// Which call the child is waiting on.
        call_id: String,
    },
}

impl ScriptStep {
    /// A line step from anything stringy.
    #[must_use]
    pub fn line(line: impl Into<String>) -> Self {
        ScriptStep::Line(line.into())
    }

    /// A block on this call.
    #[must_use]
    pub fn awaiting(call_id: impl Into<String>) -> Self {
        ScriptStep::AwaitDecision {
            call_id: call_id.into(),
        }
    }
}

#[derive(Debug, Default)]
struct LogInner {
    written: Vec<String>,
    spawns: u32,
    credential_copies: Vec<(PathBuf, PathBuf)>,
    killed: bool,
    launched: Vec<Vec<String>>,
}

/// What the fake saw, readable after the run took ownership of the process.
#[derive(Debug, Default, Clone)]
pub struct ScriptedLog {
    inner: Rc<RefCell<LogInner>>,
}

impl ScriptedLog {
    /// A fresh log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Every line written to the child, in the order it was written.
    #[must_use]
    pub fn written(&self) -> Vec<String> {
        self.inner.borrow().written.clone()
    }

    /// How many times a child was started.
    #[must_use]
    pub fn spawns(&self) -> u32 {
        self.inner.borrow().spawns
    }

    /// Every credential copy the runner was asked to perform, in spawn order.
    #[must_use]
    pub fn credential_copies(&self) -> Vec<(PathBuf, PathBuf)> {
        self.inner.borrow().credential_copies.clone()
    }

    /// Whether the child was killed.
    #[must_use]
    pub fn killed(&self) -> bool {
        self.inner.borrow().killed
    }

    /// The argv of each spawn: the program followed by its arguments.
    #[must_use]
    pub fn launched(&self) -> Vec<Vec<String>> {
        self.inner.borrow().launched.clone()
    }
}

/// A [`ProcessRunner`] that hands out [`ScriptedProcess`]es replaying one script.
///
/// The credential copies are **recorded and not performed**: a test asserts that they happen at
/// every spawn without a real file leaving the operator's home.
#[derive(Debug, Clone)]
pub struct ScriptedRunner {
    script: Vec<ScriptStep>,
    log: ScriptedLog,
}

impl ScriptedRunner {
    /// A runner that replays this script at every spawn.
    #[must_use]
    pub fn new(script: Vec<ScriptStep>, log: ScriptedLog) -> Self {
        Self { script, log }
    }

    /// A runner replaying these lines and blocking nowhere.
    #[must_use]
    pub fn of_lines<I, S>(lines: I, log: ScriptedLog) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::new(lines.into_iter().map(ScriptStep::line).collect(), log)
    }
}

impl ProcessRunner for ScriptedRunner {
    fn start(&mut self, plan: &LaunchPlanView) -> std::io::Result<Box<dyn HarnessProcess>> {
        {
            let mut inner = self.log.inner.borrow_mut();
            inner.spawns += 1;
            for copy in plan.credential_copies {
                inner
                    .credential_copies
                    .push((copy.from.to_path_buf(), copy.to.to_path_buf()));
            }
            let mut argv = vec![plan.program.to_string()];
            argv.extend(plan.args.iter().cloned());
            inner.launched.push(argv);
        }
        Ok(Box::new(ScriptedProcess {
            steps: self.script.iter().cloned().collect(),
            answered: BTreeSet::new(),
            log: self.log.clone(),
            exit_code: Some(0),
        }))
    }
}

/// A child that replays a transcript and records what was written to it.
#[derive(Debug)]
pub struct ScriptedProcess {
    steps: VecDeque<ScriptStep>,
    answered: BTreeSet<String>,
    log: ScriptedLog,
    exit_code: Option<i32>,
}

impl ScriptedProcess {
    /// A process replaying these steps, logging into this log.
    #[must_use]
    pub fn new(steps: Vec<ScriptStep>, log: ScriptedLog) -> Self {
        Self {
            steps: steps.into(),
            answered: BTreeSet::new(),
            log,
            exit_code: Some(0),
        }
    }
}

impl HarnessProcess for ScriptedProcess {
    fn next_line(&mut self) -> std::io::Result<Option<String>> {
        loop {
            match self.steps.front() {
                None => return Ok(None),
                Some(ScriptStep::Line(_)) => {
                    let Some(ScriptStep::Line(line)) = self.steps.pop_front() else {
                        unreachable!("the front was just matched as a line")
                    };
                    return Ok(Some(line));
                }
                Some(ScriptStep::AwaitDecision { call_id }) => {
                    if self.answered.contains(call_id) {
                        self.steps.pop_front();
                        continue;
                    }
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::WouldBlock,
                        format!("the child is waiting on a decision for {call_id}"),
                    ));
                }
            }
        }
    }

    fn write_line(&mut self, line: &str) -> std::io::Result<()> {
        if let Ok(Value::Object(fields)) = serde_json::from_str::<Value>(line) {
            for key in ["call_id", "decision_for"] {
                if let Some(id) = fields.get(key).and_then(Value::as_str) {
                    self.answered.insert(id.to_string());
                }
            }
        }
        self.log.inner.borrow_mut().written.push(line.to_string());
        Ok(())
    }

    fn kill(&mut self) -> std::io::Result<()> {
        self.log.inner.borrow_mut().killed = true;
        self.steps.clear();
        Ok(())
    }

    fn wait(&mut self) -> std::io::Result<Option<i32>> {
        Ok(self.exit_code)
    }
}

/// A [`HarnessSeam`] that speaks a compact fixture language instead of a vendor's wire.
///
/// **Test support.** It exists so the control machinery — § 7.7's five ordering rules and
/// § 6.1's refusal codes — can be driven without any adapter's transcript format in the way. A
/// vector that failed because a vendor record changed shape would be reporting a C2 defect under
/// a C3 name.
///
/// The language is one JSON object per line with an `emit` key:
///
/// ```text
/// {"emit":"session.started","harness_version":"2.1.240","cwd":"/w","output_style":"default"}
/// {"emit":"tool.requested","call_id":"t1","name":"Bash","input":{"command":"ls"}}
/// {"emit":"tool.result","call_id":"t1","is_error":false}
/// {"emit":"text","text":"done"}
/// {"emit":"session.ended","is_error":false,"subtype":"success"}
/// ```
///
/// Anything else is `Event::Opaque`, which is the same answer the real readers owe (design D4).
pub struct ScriptedSeam {
    transcript: TranscriptRef,
    attestation: HermeticAttestation,
    seam: Seam,
    census: DecisionCensus,
    line: u64,
}

impl std::fmt::Debug for ScriptedSeam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptedSeam")
            .field("line", &self.line)
            .finish_non_exhaustive()
    }
}

impl ScriptedSeam {
    /// A seam that stamps this transcript reference and this attestation into `session.started`.
    #[must_use]
    pub fn new(transcript: TranscriptRef, attestation: HermeticAttestation, seam: Seam) -> Self {
        Self {
            transcript,
            attestation,
            seam,
            census: DecisionCensus::default(),
            line: 0,
        }
    }

    fn opaque(&self, line: &str, vendor_type: Option<String>) -> Emission {
        Emission::untimed(Event::Opaque {
            vendor_type,
            vendor_subtype: None,
            digest: Digest::of(line.as_bytes()),
            source_line: Some(self.line),
        })
    }
}

fn text_of(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn strings_of(value: &Value, key: &str) -> Option<Vec<String>> {
    value.get(key).and_then(Value::as_array).map(|items| {
        items
            .iter()
            .filter_map(|item| item.as_str().map(ToString::to_string))
            .collect()
    })
}

impl HarnessSeam for ScriptedSeam {
    fn push_line(&mut self, line: &str) -> Vec<Emission> {
        self.line += 1;
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return vec![self.opaque(line, None)];
        };
        let emit = value
            .get("emit")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let event = match emit {
            "session.started" => Event::SessionStarted {
                adapter: metaharness_claude::ADAPTER_ID.to_string(),
                adapter_class: "harness".to_string(),
                harness_version: text_of(&value, "harness_version"),
                session_id: text_of(&value, "session_id"),
                model: text_of(&value, "model"),
                permission_mode: text_of(&value, "permission_mode"),
                credential_source: text_of(&value, "credential_source"),
                output_style: text_of(&value, "output_style"),
                cwd: text_of(&value, "cwd"),
                offered_tools: strings_of(&value, "offered_tools"),
                slash_commands: strings_of(&value, "slash_commands"),
                skills: strings_of(&value, "skills"),
                agents: strings_of(&value, "agents"),
                plugins: strings_of(&value, "plugins").map(|names| {
                    names
                        .into_iter()
                        .map(|name| PluginRef {
                            name: Some(name),
                            source: None,
                            version: None,
                        })
                        .collect()
                }),
                mcp_servers: strings_of(&value, "mcp_servers").map(|names| {
                    names
                        .into_iter()
                        .map(|name| McpServerRef {
                            name: Some(name),
                            status: None,
                        })
                        .collect()
                }),
                inputs_digest: text_of(&value, "inputs_digest")
                    .map(|text| Digest::of(text.as_bytes())),
                transcript: self.transcript.clone(),
                hermetic: self.attestation.clone(),
            },
            "session.ended" => Event::SessionEnded {
                is_error: value.get("is_error").and_then(Value::as_bool),
                subtype: text_of(&value, "subtype"),
                stop_reason: text_of(&value, "stop_reason"),
                terminal_reason: text_of(&value, "terminal_reason"),
                api_error_status: text_of(&value, "api_error_status"),
                num_turns: value.get("num_turns").and_then(Value::as_u64),
                duration_ms: value.get("duration_ms").and_then(Value::as_u64),
                duration_api_ms: None,
                ttft_ms: None,
                time_to_request_ms: None,
                total_cost_usd: value.get("total_cost_usd").and_then(Value::as_f64),
                permission_denials: Some(Vec::new()),
                subagents_spawned: None,
                usage: Some(Usage::default()),
                model_usage: None,
                census: self.census.clone(),
            },
            "tool.requested" => Event::ToolRequested {
                call_id: text_of(&value, "call_id").unwrap_or_default(),
                name: text_of(&value, "name").unwrap_or_default(),
                input: value.get("input").cloned().unwrap_or(Value::Null),
                // The scripted adapter is an adapter: it resolves nothing, and the loop fills it.
                operations: Vec::new(),
                decision_required: false,
                deadline_ms: None,
                seam: self.seam,
            },
            "tool.result" => Event::ToolResult {
                call_id: text_of(&value, "call_id").unwrap_or_default(),
                is_error: value.get("is_error").and_then(Value::as_bool),
                content: value.get("content").cloned(),
                bytes: None,
                // Scripted verbatim where a script names one, absent where it does not. The fake
                // must be able to produce the vendor's per-tool result record, because a decision
                // policy tested only against streams that never carry one is a policy tested
                // against half the wire (amendment a9).
                tool_use_result: value.get("tool_use_result").cloned(),
            },
            "text" => Event::Text {
                text: text_of(&value, "text").unwrap_or_default(),
                request_id: text_of(&value, "request_id"),
            },
            "turn.ended" => Event::TurnEnded {
                turn: u32::try_from(value.get("turn").and_then(Value::as_u64).unwrap_or(1))
                    .unwrap_or(u32::MAX),
                stop_reason: text_of(&value, "stop_reason"),
            },
            "auth.expired" => Event::AuthExpired {
                credential_source: text_of(&value, "credential_source"),
                detail: text_of(&value, "detail"),
                source_line: Some(self.line),
            },
            _ => return vec![self.opaque(line, Some(emit.to_string()))],
        };
        vec![Emission::untimed(event)]
    }

    fn finish(&mut self) -> Vec<Emission> {
        Vec::new()
    }

    fn set_census(&mut self, census: DecisionCensus) {
        self.census = census;
    }

    fn decision_line(&self, call_id: &str, decision: &Decision) -> String {
        serde_json::json!({ "call_id": call_id, "decision": decision }).to_string()
    }

    fn control_line(&self, command: &Command) -> Option<String> {
        match command {
            Command::ToolDecide { .. } | Command::FrameSet { .. } | Command::Steer { .. } => None,
            other => Some(serde_json::json!({ "control": other.name() }).to_string()),
        }
    }
}

/// The factory for [`ScriptedSeam`]. **Test support.**
#[derive(Debug, Clone, Copy, Default)]
pub struct ScriptedSeams;

impl metaharness_protocol::SeamFactory for ScriptedSeams {
    fn build(
        &mut self,
        transcript: TranscriptRef,
        attestation: HermeticAttestation,
        seam: Seam,
    ) -> Box<dyn HarnessSeam> {
        Box::new(ScriptedSeam::new(transcript, attestation, seam))
    }
}
