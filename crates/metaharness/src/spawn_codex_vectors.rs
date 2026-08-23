//! C3 — the codex spawn vectors: a real process, a real hook, a real session file, and still no
//! model.
//!
//! The Claude spawn vectors in [`crate::spawn_vectors`] drive a fake vendor that prints
//! stream-json down a pipe. These drive [`crate::CodexSpawnRunner`] against a fake that behaves
//! the way `codex exec` does — **it writes its record to a file** under a scratch `CODEX_HOME`,
//! runs the very hook program the launch installed, and blocks on it — so they cover the two
//! things this adapter has that the other one does not:
//!
//! | what | why nothing cheaper shows it |
//! |---|---|
//! | the rollout is discovered, tailed and retained as the transcript | the session file's name is a `UUIDv7` the child picks, so a test that named it would be testing a name metaharness invented |
//! | a decision reaches the hook process holding the call | the seam is a second process; there is nothing to unblock in a scripted one |
//! | the thin `--json` stdout is retained **beside** the record and not as it | the two are different files carrying different things, and reading one for the other is the failure |
//! | the credential is copied at **every** spawn | H6 is a claim about a number of copies per number of spawns |
//!
//! **Free, and it must stay free**: `/bin/sh`, no network, no credential, no model. A vector that
//! reached for the real `codex` would put the default gate on an account's bill.

use std::collections::BTreeMap;
use std::path::Path;

use metaharness_protocol::{ConformanceTier, Decision, VectorOutcome};
use serde_json::Value;

use crate::process::{CredentialCopyView, LaunchPlanView, ProcessRunner};
use crate::spawn_codex::CodexSpawnRunner;

/// The vendor's own call id, as it appears in the rollout record.
const VENDOR_CALL_ID: &str = "call-fixture-1";

/// What the fake vendor prints on stderr, so a vector can prove it was kept.
const VENDOR_STDERR: &str = "the fake codex complained here";

/// What the fake vendor records as a call's output **only if the seam allowed it**.
///
/// The marker is the difference between the two decisions at this seam: a deny leaves the rollout
/// with no output record for the call at all — which is exactly what the live 0.145.0 run showed,
/// `Output:` empty beneath `Command blocked by PreToolUse hook` — and an allow leaves one.
const ALLOWED_OUTPUT: &str = "the call ran because metaharness allowed it";

/// The codex spawn vectors of design § 8.5's C3 tier.
#[must_use]
pub fn spawn_vectors() -> Vec<VectorOutcome> {
    vec![
        vector_hook_round_trip(),
        vector_allow_round_trip(),
        vector_rollout_is_the_retained_record(),
        vector_credential_per_spawn(),
    ]
}

/// One prepared fake vendor and the world it runs in.
struct Fake {
    root: tempfile::TempDir,
}

impl Fake {
    /// Lay out a scratch root with the hook program the adapter would have installed.
    fn new() -> std::io::Result<Self> {
        let root = tempfile::TempDir::new()?;
        let fake = Self { root };
        let channel = metaharness_codex::HookChannelPaths::under(fake.path());
        std::fs::create_dir_all(fake.path().join("work"))?;
        std::fs::create_dir_all(fake.path().join("hooks"))?;
        std::fs::create_dir_all(fake.path().join("codex-home"))?;

        // The real program, rendered by the real adapter. A vector that wrote its own stand-in
        // would be testing the stand-in.
        let program = metaharness_codex::hook_program(&channel);
        let hook = metaharness_codex::hook_program_path(fake.path());
        std::fs::write(&hook, program)?;
        set_executable(&hook)?;

        std::fs::write(fake.path().join("vendor.sh"), fake.vendor_script())?;
        Ok(fake)
    }

    fn path(&self) -> &Path {
        self.root.path()
    }

    fn codex_home(&self) -> std::path::PathBuf {
        self.path().join("codex-home")
    }

    /// A vendor that opens a session, records one call, consults the hook, **acts on its answer**,
    /// and ends.
    ///
    /// The record is written **to a file** and the hook is a separate process, which is the whole
    /// point: this is the shape `codex exec` has and `claude -p` does not. The session directory
    /// and the `rollout-` name are the vendor's own layout, verified against 2,437 local files.
    ///
    /// # What "acts on its answer" is and is not
    ///
    /// The `if` below runs the call only when the hook printed `permissionDecision: "allow"`, and
    /// that branch is **this stub's behaviour, not evidence about codex**. What it makes testable
    /// is metaharness's own half in both polarities on one wire: a deny leaves no output record and
    /// an allow leaves one, so a vector cannot pass by rendering a decision nothing consumed. That
    /// the *vendor* honours an `allow` at `PreToolUse` is a paid observation, and the 0.145.0
    /// binary carries a string that would refuse one — see `metaharness_codex::render_hook_response`
    /// and the C4 vector in `tests/live_codex.rs`.
    fn vendor_script(&self) -> String {
        let root = self.path().display();
        format!(
            r#"#!/bin/sh
set -u
SESSION="{root}/codex-home/sessions/2026/08/22"
mkdir -p "$SESSION"
ROLLOUT="$SESSION/rollout-2026-08-22T10-00-00-fixture.jsonl"

printf '%s\n' '{{"timestamp":"2026-08-22T10:00:00.000Z","type":"session_meta","payload":{{"id":"fixture-session","session_id":"fixture-session","cli_version":"0.145.0","cwd":"{root}/work","originator":"codex_exec"}}}}' > "$ROLLOUT"
printf '%s\n' '{{"timestamp":"2026-08-22T10:00:01.000Z","type":"event_msg","payload":{{"type":"task_started","turn_id":"t1"}}}}' >> "$ROLLOUT"
printf '%s\n' '{{"timestamp":"2026-08-22T10:00:02.000Z","type":"response_item","payload":{{"type":"custom_tool_call","call_id":"{VENDOR_CALL_ID}","name":"exec","arguments":"{{\"command\":\"rm -rf /\"}}"}}}}' >> "$ROLLOUT"
printf '%s\n' '{{"thread.started":"fixture"}}'
printf '%s\n' '{VENDOR_STDERR}' >&2

printf '%s' '{{"hook_event_name":"PreToolUse","tool_name":"shell","tool_use_id":"{VENDOR_CALL_ID}","session_id":"fixture-session","turn_id":"t1","cwd":"{root}/work","model":"fixture","permission_mode":"default","transcript_path":null,"tool_input":{{"command":"rm -rf /"}}}}' | '{root}/hooks/pretooluse' > '{root}/hook-stdout' 2>'{root}/hook-stderr'

# The seam's answer decides whether the call has an output record at all. A denied call leaves
# none — which is what the live 0.145.0 run showed — and an allowed call leaves this one.
if grep -q '"permissionDecision":"allow"' '{root}/hook-stdout' 2>/dev/null; then
    printf '%s\n' '{{"timestamp":"2026-08-22T10:00:04.000Z","type":"response_item","payload":{{"type":"custom_tool_call_output","call_id":"{VENDOR_CALL_ID}","output":"{ALLOWED_OUTPUT}"}}}}' >> "$ROLLOUT"
fi

printf '%s\n' '{{"timestamp":"2026-08-22T10:00:05.000Z","type":"event_msg","payload":{{"type":"task_complete","turn_id":"t1","duration_ms":4200}}}}' >> "$ROLLOUT"
"#
        )
    }

    /// The child environment the fake needs: enough `PATH` for the hook's `mktemp`, `cat`, `mv`
    /// and `sleep`, plus the `CODEX_HOME` the runner needs in order to know where to look for the
    /// record. Constructed rather than inherited, exactly as H3 has the real launch do it.
    fn env(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("PATH".to_string(), "/usr/bin:/bin".to_string()),
            (
                "CODEX_HOME".to_string(),
                self.codex_home().display().to_string(),
            ),
        ])
    }

    /// What the hook printed, which is what the vendor would have honoured.
    fn hook_stdout(&self) -> String {
        std::fs::read_to_string(self.path().join("hook-stdout")).unwrap_or_default()
    }
}

#[cfg(unix)]
fn set_executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(path, permissions)
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Start the fake and pump it to the end, answering the hook request with `decision`.
///
/// Returns every line the process handed over, so a vector can assert on the record as well as on
/// the hook's answer.
fn drive(
    fake: &Fake,
    runner: &mut CodexSpawnRunner,
    copies: &[CredentialCopyView<'_>],
    decision: Option<&Decision>,
) -> std::io::Result<Vec<String>> {
    let script = fake.path().join("vendor.sh").display().to_string();
    let args = vec![script];
    let env = fake.env();
    let cwd = fake.path().join("work");
    let channel = metaharness_codex::HookChannelPaths::under(fake.path());
    let transcript = fake.path().join("rollout.jsonl");

    let view = LaunchPlanView {
        program: "/bin/sh",
        args: &args,
        env: &env,
        cwd: &cwd,
        credential_copies: copies,
        decision_channel: &channel.root,
        transcript: &transcript,
    };
    let mut process = runner.start(&view)?;

    let mut lines = Vec::new();
    loop {
        match process.next_line() {
            Ok(Some(line)) => {
                // The seam's own envelope: the adapter publishes it, and this reads back the
                // rendezvous name the hook process minted so the answer goes to that process.
                if let Some(key) = hook_key(&line)
                    && let Some(decision) = decision
                {
                    let body = metaharness_codex::render_hook_response(decision);
                    let answer = serde_json::json!({ "call_id": key, "response": body });
                    process.write_line(&answer.to_string())?;
                }
                lines.push(line);
            }
            Ok(None) => break,
            // The fake is holding a call nobody is going to answer, which is the case the hook's
            // own backstop exists for. A vector must not sit out that backstop.
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(error) => return Err(error),
        }
    }
    process.wait()?;
    Ok(lines)
}

/// The rendezvous name a hook-request line carries, or `None` when the line is a record.
fn hook_key(line: &str) -> Option<String> {
    let value: Value = serde_json::from_str(line).ok()?;
    if value.get("metaharness.codex/1").and_then(Value::as_str)? != "hook_request" {
        return None;
    }
    value
        .get("key")
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

/// The whole seam, end to end, with a real second process holding the call.
fn vector_hook_round_trip() -> VectorOutcome {
    let id = "c3/codex-spawn-a-deny-reaches-the-hook-process-holding-the-call";
    let Ok(fake) = Fake::new() else {
        return broken(id, "the fake vendor could not be laid out");
    };
    let mut runner = CodexSpawnRunner::new();
    let decision = Decision::Deny {
        reason: "this step admits no shell".to_string(),
    };
    let lines = match drive(&fake, &mut runner, &[], Some(&decision)) {
        Ok(lines) => lines,
        Err(error) => return broken(id, &error.to_string()),
    };

    let answer = fake.hook_stdout();
    let mut faults = Vec::new();
    if !lines.iter().any(|line| hook_key(line).is_some()) {
        faults.push("the hook request never reached the seam".to_string());
    }
    if !answer.contains(r#""permissionDecision":"deny""#) {
        faults.push(format!("the hook printed no deny: {answer:?}"));
    }
    if !answer.contains("this step admits no shell") {
        faults.push(format!(
            "the hook printed no reason the model could act on: {answer:?}"
        ));
    }
    // The hook must not have fallen through to its own backstop: that would deny too, and a
    // vector that could not tell the two apart would pass while the channel was dead.
    if answer.contains("did not answer this call within") {
        faults.push("the hook fell through to its own backstop, so nothing was delivered".into());
    }
    // The negative half, and the one that matters: the call left **no output record**. A vector
    // with only the positive half would pass while the command ran anyway and the model was told
    // off about it afterwards — which is an audit, not a control (design § 7.2).
    if lines.iter().any(|line| line.contains(ALLOWED_OUTPUT)) {
        faults.push("the denied call produced an output record, so the effect landed".into());
    }
    verdict(id, &faults)
}

/// The grant half of the same wire: an `allow` renders to the vendor's shape, reaches the process
/// holding the call, and the call proceeds.
///
/// **What it proves and what it does not.** Everything on metaharness's side of the hook: the
/// envelope is the one the vendor's own `PreToolUseHookSpecificOutputWire` names, it carries no
/// `permissionDecisionReason` (a reason is the deny's obligation, and 0.145.0 refuses
/// `permissionDecisionReason` *without* a `permissionDecision`), it is not the backstop's deny, and
/// a vendor that honours it runs the call. **Not** proven here: that codex honours it. No vendor
/// binary runs in this tier, and 0.145.0 carries a literal that would refuse an allow at
/// `PreToolUse` — so the live half is a C4 vector (`tests/live_codex.rs`) and stays labelled
/// undriven until somebody spends it.
fn vector_allow_round_trip() -> VectorOutcome {
    let id = "c3/codex-spawn-an-allow-reaches-the-hook-process-and-the-call-proceeds";
    let Ok(fake) = Fake::new() else {
        return broken(id, "the fake vendor could not be laid out");
    };
    let mut runner = CodexSpawnRunner::new();
    let lines = match drive(&fake, &mut runner, &[], Some(&Decision::Allow)) {
        Ok(lines) => lines,
        Err(error) => return broken(id, &error.to_string()),
    };

    let answer = fake.hook_stdout();
    let mut faults = Vec::new();
    if !lines.iter().any(|line| hook_key(line).is_some()) {
        faults.push("the hook request never reached the seam".to_string());
    }
    match serde_json::from_str::<Value>(answer.trim()) {
        Ok(parsed) => {
            let output = &parsed["hookSpecificOutput"];
            if output["hookEventName"] != "PreToolUse" {
                faults.push(format!("the envelope names the wrong event: {answer:?}"));
            }
            if output["permissionDecision"] != "allow" {
                faults.push(format!("the hook printed no allow: {answer:?}"));
            }
            if output.get("permissionDecisionReason").is_some() {
                faults.push(format!(
                    "an allow carries no reason on this wire, and 0.145.0 refuses a reason \
                     without a decision: {answer:?}"
                ));
            }
        }
        Err(error) => faults.push(format!(
            "the hook printed no JSON at all ({error}): {answer:?}"
        )),
    }
    if answer.contains("did not answer this call within") || answer.contains("\"deny\"") {
        faults.push("the hook answered with a denial, so nothing was granted".to_string());
    }
    // The decision was consumed, not merely printed: the call has an output record, and it came
    // back through the seam rather than only existing on disk.
    if !lines.iter().any(|line| line.contains(ALLOWED_OUTPUT)) {
        faults.push(
            "the allowed call produced no output record at the seam, so nothing acted on the \
             decision"
                .to_string(),
        );
    }
    verdict(id, &faults)
}

/// The record is the session file, retained; the thin stream is retained beside it; stderr is kept.
fn vector_rollout_is_the_retained_record() -> VectorOutcome {
    let id = "c3/codex-spawn-the-rollout-is-tailed-and-retained-as-the-record";
    let Ok(fake) = Fake::new() else {
        return broken(id, "the fake vendor could not be laid out");
    };
    let mut runner = CodexSpawnRunner::new();
    let lines = match drive(&fake, &mut runner, &[], Some(&Decision::Allow)) {
        Ok(lines) => lines,
        Err(error) => return broken(id, &error.to_string()),
    };

    let mut faults = Vec::new();
    let records: Vec<&String> = lines
        .iter()
        .filter(|line| hook_key(line).is_none())
        .collect();
    // Every record the vendor wrote to its session file came back through the seam, in order.
    for expected in [
        "session_meta",
        "task_started",
        "custom_tool_call",
        "task_complete",
    ] {
        if !records.iter().any(|line| line.contains(expected)) {
            faults.push(format!("the record {expected} never reached the seam"));
        }
    }
    let retained = std::fs::read_to_string(fake.path().join("rollout.jsonl")).unwrap_or_default();
    for line in &records {
        if !retained.contains(line.as_str()) {
            faults.push(format!("the transcript is missing a line it read: {line}"));
            break;
        }
    }
    if retained.lines().count() != records.len() {
        faults.push(format!(
            "{} lines retained against {} read — O8's bytes are the auditor's whole subject",
            retained.lines().count(),
            records.len()
        ));
    }
    // The thin stream is retained too, and it is **not** the record: it carries the vendor's
    // stdout and none of the rollout's timestamps.
    let stdout =
        std::fs::read_to_string(fake.path().join("rollout.stdout.jsonl")).unwrap_or_default();
    if !stdout.contains("thread.started") {
        faults.push(format!(
            "the thin --json stream was not retained: {stdout:?}"
        ));
    }
    if stdout.contains("session_meta") {
        faults.push("the thin stream and the record are the same file".to_string());
    }
    verdict(id, &faults)
}

/// H6, as a number: one copy per spawn, not one per run.
fn vector_credential_per_spawn() -> VectorOutcome {
    let id = "c3/codex-spawn-the-credential-is-copied-before-every-spawn";
    let Ok(fake) = Fake::new() else {
        return broken(id, "the fake vendor could not be laid out");
    };
    let from = fake.path().join("operator-auth.json");
    if std::fs::write(&from, "{\"synthesized\":true}").is_err() {
        return broken(id, "the stand-in credential could not be written");
    }
    let to = fake.codex_home().join("auth.json");
    let copies = vec![CredentialCopyView {
        from: &from,
        to: &to,
    }];

    let mut runner = CodexSpawnRunner::new();
    for _ in 0..2 {
        if let Err(error) = drive(&fake, &mut runner, &copies, Some(&Decision::Allow)) {
            return broken(id, &error.to_string());
        }
    }

    let mut faults = Vec::new();
    if runner.spawns() != 2 {
        faults.push(format!("{} spawns, expected 2", runner.spawns()));
    }
    if runner.credential_copies() != 2 {
        faults.push(format!(
            "{} credential copies across 2 spawns — a copy taken once and reused is the failure \
             H6's amendment exists for",
            runner.credential_copies()
        ));
    }
    if !to.exists() {
        faults.push("the credential never landed in the scratch CODEX_HOME".to_string());
    }
    verdict(id, &faults)
}

fn verdict(id: &str, faults: &[String]) -> VectorOutcome {
    if faults.is_empty() {
        VectorOutcome::passed(id, ConformanceTier::C3)
    } else {
        VectorOutcome::failed(id, ConformanceTier::C3, faults.join("; "))
    }
}

fn broken(id: &str, detail: &str) -> VectorOutcome {
    VectorOutcome::failed(
        id,
        ConformanceTier::C3,
        format!("the vector observed nothing: {detail}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_codex_spawn_vector_passes() {
        let vectors = spawn_vectors();
        let failures: Vec<(&str, &str)> = vectors
            .iter()
            .filter(|vector| !vector.passed)
            .map(|vector| (vector.id.as_str(), vector.detail.as_str()))
            .collect();
        assert!(failures.is_empty(), "{failures:#?}");
        assert_eq!(vectors.len(), 4);
    }

    /// Fail-closed polarity, mutated: the deny vector must be **able** to see an effect that
    /// landed. Driving the same stub with an allow produces the output record the deny vector
    /// forbids, so that vector is a guard rather than an assertion nothing can violate.
    #[test]
    fn the_deny_vectors_negative_half_can_actually_go_red() {
        let fake = Fake::new().expect("laid out");
        let mut runner = CodexSpawnRunner::new();
        let lines = drive(&fake, &mut runner, &[], Some(&Decision::Allow)).expect("driven");
        assert!(
            lines.iter().any(|line| line.contains(ALLOWED_OUTPUT)),
            "an allowed call must leave the output record the deny vector asserts is absent, or \
             that assertion is unfalsifiable"
        );
    }

    /// The stderr the fake wrote is kept, which is the difference between exit `3` meaning
    /// *nobody found out* and exit `3` meaning nothing at all.
    #[test]
    fn the_childs_stderr_is_retained_whole() {
        let fake = Fake::new().expect("laid out");
        let script = fake.path().join("vendor.sh").display().to_string();
        let args = vec![script];
        let env = fake.env();
        let cwd = fake.path().join("work");
        let channel = metaharness_codex::HookChannelPaths::under(fake.path());
        let transcript = fake.path().join("rollout.jsonl");
        let view = LaunchPlanView {
            program: "/bin/sh",
            args: &args,
            env: &env,
            cwd: &cwd,
            credential_copies: &[],
            decision_channel: &channel.root,
            transcript: &transcript,
        };
        let mut runner = CodexSpawnRunner::new();
        let mut process = runner.start(&view).expect("started");
        while let Ok(Some(line)) = process.next_line() {
            if let Some(key) = hook_key(&line) {
                let body = metaharness_codex::render_hook_response(&Decision::Allow);
                let answer = serde_json::json!({ "call_id": key, "response": body });
                process.write_line(&answer.to_string()).expect("routed");
            }
        }
        assert_eq!(process.wait().expect("waited"), Some(0));
        let hook_stderr =
            std::fs::read_to_string(fake.path().join("hook-stderr")).unwrap_or_default();
        assert!(
            hook_stderr.is_empty(),
            "the hook itself said: {hook_stderr}"
        );
    }

    /// An `abstain` writes no bytes at all, so the vendor's own approval policy decides. It must
    /// not render as the value that means metaharness permitted the call.
    #[test]
    fn an_abstain_reaches_the_hook_as_no_output_at_all() {
        let fake = Fake::new().expect("laid out");
        let mut runner = CodexSpawnRunner::new();
        drive(&fake, &mut runner, &[], Some(&Decision::Abstain)).expect("driven");
        let answer = fake.hook_stdout();
        assert!(
            answer.trim().is_empty(),
            "abstain must print nothing, printed: {answer:?}"
        );
    }

    /// A run nobody answers falls through to the hook's **own** backstop, which denies with a
    /// reason rather than passing the call through. The seam fails closed with no metaharness at
    /// all — that is the property, and it is why the program never exits non-zero.
    #[test]
    fn an_unanswered_call_falls_through_to_the_hooks_own_fail_closed_backstop() {
        let program = metaharness_codex::hook_program(&metaharness_codex::HookChannelPaths::under(
            Path::new("/nonexistent-channel-root"),
        ));
        assert!(program.contains(r#""permissionDecision":"deny""#));
        assert!(program.contains("could not be reached over its decision channel"));
    }
}
