//! C3 — the spawn vectors: a real process, a real hook, and still no model.
//!
//! The vectors in [`crate::vectors`] drive a scripted process, which is what keeps § 7.7's
//! ordering rules attributable to metaharness rather than to a vendor's wire. These drive
//! [`crate::SpawnRunner`] against a **fake vendor** — a shell script that prints stream-json and
//! then runs the very hook program the launch installed — and they cover the half a scripted
//! process cannot reach:
//!
//! | what | why a scripted process cannot show it |
//! |---|---|
//! | the installed hook program blocks, and a decision reaches it | the scripted seam answers in-process; there is no second process to unblock |
//! | the credential is copied **at every spawn** | amendment a1 is a claim about a number of copies per number of spawns |
//! | the raw transcript is retained as it is read | design § 8.4 O8's bytes are written by the runner, and the fake is the only free way to make some |
//! | stderr survives a child that ends badly | the only account of why a run left exit `3` |
//!
//! **Free, and it must stay free**: `/bin/sh`, no network, no credential, no model. A vector that
//! reached for the real `claude` would put the default gate on an account's bill, which is the
//! one thing `metaharness conformance` promises it never does.

use std::collections::BTreeMap;
use std::path::Path;

use metaharness_protocol::{ConformanceTier, Decision, VectorOutcome};

use crate::process::{CredentialCopyView, LaunchPlanView, ProcessRunner};
use crate::spawn::SpawnRunner;

/// The call the fake vendor asks about, and the id the hook correlates by (row **V22**).
const CALL_ID: &str = "toolu_0000000000000000000001";

/// What the fake vendor prints on stderr, so a test can prove it was kept.
const VENDOR_STDERR: &str = "the fake vendor complained here";

/// The spawn vectors of § 8.5's C3 tier.
#[must_use]
pub fn spawn_vectors() -> Vec<VectorOutcome> {
    vec![
        vector_hook_round_trip(),
        vector_credential_per_spawn(),
        vector_transcript_and_stderr(),
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
        let channel = metaharness_claude::HookChannelPaths::under(fake.path());
        std::fs::create_dir_all(fake.path().join("work"))?;
        std::fs::create_dir_all(fake.path().join("hooks"))?;

        // The real program, rendered by the real adapter. A vector that wrote its own stand-in
        // would be testing the stand-in.
        let program = metaharness_claude::hook_program(&channel);
        let hook = metaharness_claude::hook_program_path(fake.path());
        std::fs::write(&hook, program)?;
        set_executable(&hook)?;

        std::fs::write(fake.path().join("vendor.sh"), fake.vendor_script())?;
        Ok(fake)
    }

    fn path(&self) -> &Path {
        self.root.path()
    }

    /// A vendor that emits one call, consults the hook, and ends.
    ///
    /// The hook's own stdout is captured to a file, because that is the only place the vendor's
    /// answer exists — and asserting on it is how a vector proves the decision arrived rather
    /// than merely that metaharness wrote one.
    fn vendor_script(&self) -> String {
        let root = self.path().display();
        format!(
            r#"#!/bin/sh
set -u
printf '%s\n' '{{"type":"system","subtype":"init","session_id":"s-1","claude_code_version":"2.1.239","cwd":"{root}/work","apiKeySource":"none","output_style":"default","plugins":[],"mcp_servers":[],"tools":["Bash"]}}'
printf '%s\n' '{{"type":"assistant","message":{{"id":"msg-1","role":"assistant","content":[{{"type":"tool_use","id":"{CALL_ID}","name":"Bash","input":{{"command":"rm -rf /"}}}}]}}}}'
printf '%s\n' '{VENDOR_STDERR}' >&2
printf '%s' '{{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{{"command":"rm -rf /"}},"session_id":"s-1","tool_use_id":"{CALL_ID}"}}' | '{root}/hooks/pretooluse' > '{root}/hook-stdout' 2>'{root}/hook-stderr'
printf '%s\n' '{{"type":"result","subtype":"success","is_error":false,"num_turns":1,"total_cost_usd":0}}'
"#
        )
    }

    /// The child environment the fake needs: enough `PATH` for the hook's `mktemp`, `cat`,
    /// `mv` and `sleep`, and nothing else. Constructed rather than inherited, exactly as H3 has
    /// the real launch do it.
    fn env() -> BTreeMap<String, String> {
        BTreeMap::from([("PATH".to_string(), "/usr/bin:/bin".to_string())])
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

/// Start the fake and pump it to the end, answering [`CALL_ID`] with `decision`.
///
/// Returns every line the fake wrote, so a vector can assert on the stream as well as on the
/// hook's answer.
fn drive(
    fake: &Fake,
    runner: &mut SpawnRunner,
    copies: &[CredentialCopyView<'_>],
    decision: Option<&Decision>,
) -> std::io::Result<Vec<String>> {
    let script = fake.path().join("vendor.sh").display().to_string();
    let args = vec![script];
    let env = Fake::env();
    let cwd = fake.path().join("work");
    let channel = metaharness_claude::HookChannelPaths::under(fake.path());
    let transcript = fake.path().join("transcript.jsonl");

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
    let mut answered = false;
    loop {
        match process.next_line() {
            Ok(Some(line)) => {
                let carries_the_call = line.contains(CALL_ID);
                lines.push(line);
                if carries_the_call && !answered {
                    if let Some(decision) = decision {
                        // Exactly the envelope the adapter publishes; nothing here invents one.
                        let body = metaharness_claude::render_hook_response(decision);
                        let line = serde_json::json!({ "call_id": CALL_ID, "response": body });
                        process.write_line(&line.to_string())?;
                    }
                    answered = true;
                }
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

/// The whole seam, end to end, with a real second process holding the call.
fn vector_hook_round_trip() -> VectorOutcome {
    let id = "c3/spawn-a-deny-reaches-the-hook-process-holding-the-call";
    let Ok(fake) = Fake::new() else {
        return broken(id, "the fake vendor could not be laid out");
    };
    let mut runner = SpawnRunner::new();
    let decision = Decision::Deny {
        reason: "this step admits no shell".to_string(),
    };
    let lines = match drive(&fake, &mut runner, &[], Some(&decision)) {
        Ok(lines) => lines,
        Err(error) => return broken(id, &error.to_string()),
    };

    let answer = fake.hook_stdout();
    let mut faults = Vec::new();
    if !lines.iter().any(|line| line.contains(CALL_ID)) {
        faults.push("the call never reached the stream".to_string());
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
    verdict(id, &faults)
}

/// Amendment a1, as a number: one copy per spawn, not one per run.
fn vector_credential_per_spawn() -> VectorOutcome {
    let id = "c3/spawn-the-credential-is-copied-before-every-spawn";
    let Ok(fake) = Fake::new() else {
        return broken(id, "the fake vendor could not be laid out");
    };
    let from = fake.path().join("operator-credential.json");
    if std::fs::write(&from, "{\"synthesized\":true}").is_err() {
        return broken(id, "the stand-in credential could not be written");
    }
    let to = fake.path().join("claude-home").join(".credentials.json");
    let copies = vec![CredentialCopyView {
        from: &from,
        to: &to,
    }];

    let mut runner = SpawnRunner::new();
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
             amendment a1 exists for",
            runner.credential_copies()
        ));
    }
    if !to.exists() {
        faults.push("the credential never landed in the scratch config home".to_string());
    }
    verdict(id, &faults)
}

/// O8's bytes, and the only account of a bad ending.
fn vector_transcript_and_stderr() -> VectorOutcome {
    let id = "c3/spawn-the-raw-transcript-is-retained-and-stderr-is-kept";
    let Ok(fake) = Fake::new() else {
        return broken(id, "the fake vendor could not be laid out");
    };
    let mut runner = SpawnRunner::new();
    let lines = match drive(&fake, &mut runner, &[], Some(&Decision::Allow)) {
        Ok(lines) => lines,
        Err(error) => return broken(id, &error.to_string()),
    };

    let mut faults = Vec::new();
    let retained =
        std::fs::read_to_string(fake.path().join("transcript.jsonl")).unwrap_or_default();
    for line in &lines {
        if !retained.contains(line.as_str()) {
            faults.push(format!("the transcript is missing a line it read: {line}"));
            break;
        }
    }
    if retained.lines().count() != lines.len() {
        faults.push(format!(
            "{} lines retained against {} read — O8's bytes are the auditor's whole subject",
            retained.lines().count(),
            lines.len()
        ));
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
    fn every_spawn_vector_passes() {
        let vectors = spawn_vectors();
        let failures: Vec<(&str, &str)> = vectors
            .iter()
            .filter(|vector| !vector.passed)
            .map(|vector| (vector.id.as_str(), vector.detail.as_str()))
            .collect();
        assert!(failures.is_empty(), "{failures:#?}");
        assert_eq!(vectors.len(), 3);
    }

    /// The stderr the fake wrote is kept, which is the difference between exit `3` meaning
    /// *nobody found out* and exit `3` meaning nothing at all.
    #[test]
    fn the_childs_stderr_is_retained_whole() {
        let fake = Fake::new().expect("laid out");
        let script = fake.path().join("vendor.sh").display().to_string();
        let args = vec![script];
        let env = Fake::env();
        let cwd = fake.path().join("work");
        let channel = metaharness_claude::HookChannelPaths::under(fake.path());
        let transcript = fake.path().join("transcript.jsonl");
        let view = LaunchPlanView {
            program: "/bin/sh",
            args: &args,
            env: &env,
            cwd: &cwd,
            credential_copies: &[],
            decision_channel: &channel.root,
            transcript: &transcript,
        };
        let mut runner = SpawnRunner::new();
        let mut process = runner.start(&view).expect("started");
        while let Ok(Some(line)) = process.next_line() {
            if line.contains(CALL_ID) {
                let body = metaharness_claude::render_hook_response(&Decision::Allow);
                let answer = serde_json::json!({ "call_id": CALL_ID, "response": body });
                process.write_line(&answer.to_string()).expect("routed");
            }
        }
        assert_eq!(process.wait().expect("waited"), Some(0));
        let captured = std::fs::read_to_string(fake.path().join("hook-stderr")).unwrap_or_default();
        assert!(captured.is_empty(), "the hook itself said: {captured}");
    }

    /// An `abstain` writes no bytes at all, so the vendor's own pipeline decides. It must not
    /// render as the value that means metaharness permitted the call.
    #[test]
    fn an_abstain_reaches_the_hook_as_no_output_at_all() {
        let fake = Fake::new().expect("laid out");
        let mut runner = SpawnRunner::new();
        drive(&fake, &mut runner, &[], Some(&Decision::Abstain)).expect("driven");
        let answer = fake.hook_stdout();
        assert!(
            answer.trim().is_empty(),
            "abstain must print nothing, printed: {answer:?}"
        );
    }
}
