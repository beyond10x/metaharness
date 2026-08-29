//! One credential file, one lock, and a token nobody else in this process is holding.
//!
//! This is LP-2 of `docs/design/loopback-provider-v0.1.md`, and the module the loopback proxy
//! asks for a bearer. It exists because the alternative — the copy path the adapters still use by
//! default — produces a snapshot with a lifetime: a governed run on 2026-08-22 died an hour in on
//! an OAuth session that could not be refreshed, and a file copied at spawn cannot refresh itself
//! (protocol design Q13, amendment a1).
//!
//! # The v1 narrowing, stated
//!
//! **"Refresh" here is a re-read of the operator's live file under an exclusive lock. metaharness
//! never performs the OAuth dance and never writes the credential.** The vendor's own tooling
//! owns that file; this module only observes it. Two consequences, both deliberate:
//!
//! * V-LP5 — "does the vendor rotate refresh tokens on use?", the mutual-invalidation hypothesis
//!   the design labelled *unverified* — cannot bite, because a party that never writes cannot
//!   invalidate anyone. The race is removed rather than answered.
//! * A refresh only succeeds if something else already refreshed. When the re-read token is
//!   byte-identical to the stale one, that is the **"refresh failed"** signal
//!   ([`CustodyError::StillStale`]) and the caller relays the upstream's own 401 rather than
//!   inventing a diagnosis.
//!
//! Performing the refresh ourselves is a later milestone and is out of v1 on purpose: it would put
//! metaharness back in the business of writing the operator's credential, which is the custody
//! rule (design § 1.2) this whole design exists to keep.
//!
//! # The file shapes this reads — one per vendor, named by [`Kind`]
//!
//! Two vendors keep their login in two places under two shapes, and this module is the one line
//! that knows both. **Field names only**; no value in this repository, this module's tests, or any
//! error it raises ever comes from an operator's real file.
//!
//! Claude Code's `~/.claude/.credentials.json`, inspected on 2026-08-23:
//!
//! ```text
//! { "claudeAiOauth": { "accessToken", "refreshToken", "expiresAt",
//!                      "refreshTokenExpiresAt", "scopes", "subscriptionType", "rateLimitTier" } }
//! ```
//!
//! Codex's `~/.codex/auth.json`, whose field names are read from the **pinned binary's own serde
//! metadata** (`codex` 0.145.0: `struct AuthDotJson with 7 elements` — `OPENAI_API_KEY`,
//! `auth_mode`, `tokens`, `last_refresh`, `agent_identity`, `personal_access_token`,
//! `bedrock_api_key`; the token object carries `access_token`, `refresh_token`, `account_id`,
//! `id_token`, …). It holds **either** of two logins, and which one it holds decides whether the
//! codex loopback door opens at all (V-LP6, [`CodexLogin`]):
//!
//! ```text
//! { "OPENAI_API_KEY": "…" }                       // an API key the proxy can replay
//! { "tokens": { "access_token": "…", … } }        // a ChatGPT-plan login
//! ```
//!
//! Only the access token is read from either. `expiresAt`/`last_refresh` are deliberately **not**
//! consulted: a clock-based prediction of expiry is a second opinion that can disagree with the
//! upstream, and the upstream's 401 is the only authority on whether a token still works.
//!
//! # Why the lock is a sidecar and not the credential file
//!
//! The lock is `flock` on `<credential>.mh-lock`, a file this module creates and the vendor does
//! not know about. Locking the credential file's own descriptor would look tidier and would be
//! wrong: a refresher that replaces the file by `rename` leaves every holder locking an inode
//! nobody reads any more, so the serialization would quietly stop serializing. A sidecar at a
//! stable path survives the replacement.
//!
//! `flock` rather than a lock file created with `create_new`: the kernel releases an `flock` when
//! the holder's descriptor closes, including when the holder is killed. A `create_new` lock file
//! would strand every later run behind one crashed one, and the recovery for that is a stale-lock
//! heuristic — which is a second, quieter bug.

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

use metaharness_protocol::Kind;

pub use metaharness_codex::CodexLogin;

/// One credential file, and the lock that makes reads of it serialize.
///
/// Cheap to clone by `Arc` and safe to share: it holds no token, only paths and which vendor's
/// shape to read. The token is read from the file at every call, because a token cached in a field
/// is the copy-with-a-lifetime problem again, one scope smaller.
#[derive(Debug)]
pub struct CredentialCustody {
    kind: Kind,
    credential: PathBuf,
    lock: PathBuf,
    /// Which login the file held when it was opened — a **class**, never a value.
    ///
    /// Read once because it is what the launch is refused or permitted on (V-LP6), and a class
    /// that changed mid-run would mean the operator logged out and in as somebody else, which is
    /// not a case this build promises to follow. `None` on a vendor that draws no such
    /// distinction — Claude Code's file is one shape and its proxy replays one bearer either way.
    login: Option<CodexLogin>,
}

impl CredentialCustody {
    /// The operator's live credential file, in the shape this vendor keeps it in.
    ///
    /// Reads it once, under the lock, to prove the shape is one this module knows. A run whose
    /// custody is malformed is refused **at launch** rather than an hour in, which is the failure
    /// Q13 recorded. The token read for that check is dropped and never stored; what is kept is
    /// which of the two codex logins it was ([`CredentialCustody::login`]).
    ///
    /// # Errors
    ///
    /// [`io::ErrorKind::NotFound`] and friends when the file or its directory cannot be used, and
    /// [`io::ErrorKind::InvalidData`] when the file is not a shape above. Every message names the
    /// path; none of them can name a token, because the parse either found a string at a known
    /// field or reports that it did not.
    pub fn open(kind: Kind, path: &Path) -> io::Result<Self> {
        let mut custody = Self {
            kind,
            credential: path.to_path_buf(),
            lock: lock_path(path),
            login: None,
        };
        match custody.read_credential() {
            Ok((_, login)) => {
                custody.login = login;
                Ok(custody)
            }
            Err(CustodyError::Io { detail }) => {
                Err(io::Error::new(io::ErrorKind::NotFound, detail))
            }
            Err(other) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                other.to_string(),
            )),
        }
    }

    /// Which login this custody holds, as read when it was opened.
    ///
    /// `Some` only for [`Kind::Codex`], where the two classes route differently and one of them is
    /// refused by name (V-LP6). `None` for Claude Code, whose file is one shape — reported as an
    /// absent distinction rather than as a default, because a default here would be this module
    /// claiming something about a vendor it was not asked about.
    #[must_use]
    pub fn login(&self) -> Option<CodexLogin> {
        self.login
    }

    /// The current access token, read under the file lock.
    ///
    /// # Errors
    ///
    /// [`CustodyError::Io`] when the file went away or the lock could not be taken,
    /// [`CustodyError::Malformed`] when it is no longer the shape [`CredentialCustody::open`]
    /// accepted — which is a real mid-run possibility, since another process owns the file.
    pub fn bearer(&self) -> Result<String, CustodyError> {
        self.read_token()
    }

    /// The token half of [`CredentialCustody::read_credential`], for callers that want no class.
    fn read_token(&self) -> Result<String, CustodyError> {
        self.read_credential().map(|(token, _)| token)
    }

    /// Re-read the file under the lock, expecting the vendor's own tooling to have refreshed it.
    ///
    /// Returns the fresh token, or [`CustodyError::StillStale`] when the re-read token is
    /// byte-identical to `stale` — which is the "refresh failed" signal, and the only one this
    /// module is entitled to give: it does not perform the refresh (see the module doc), so the
    /// one fact it can report is whether somebody else did.
    ///
    /// # Errors
    ///
    /// [`CustodyError::StillStale`] as above, plus the same I/O and shape errors as
    /// [`CredentialCustody::bearer`].
    pub fn refreshed(&self, stale: &str) -> Result<String, CustodyError> {
        let fresh = self.read_token()?;
        if fresh == stale {
            return Err(CustodyError::StillStale);
        }
        Ok(fresh)
    }

    /// The path being watched, for a caller that wants to say which custody a run used.
    ///
    /// A path, never a value: this is the most a report is allowed to know.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.credential
    }

    /// Take the exclusive lock, read the whole file, and pull the access token out of it.
    ///
    /// The lock is held for the whole read and released when `_guard` closes at the end of the
    /// function, so a reader can never observe a half-written file — which is exactly what a
    /// refresher that writes in place produces, and what a torn read of would hand the upstream
    /// as a credential.
    fn read_credential(&self) -> Result<(String, Option<CodexLogin>), CustodyError> {
        let _guard = self.locked()?;
        let bytes = std::fs::read(&self.credential).map_err(|error| CustodyError::Io {
            detail: format!("{}: {error}", self.credential.display()),
        })?;
        let read = match self.kind {
            Kind::Claude => claude_access_token(&bytes).map(|token| (token, None)),
            Kind::Codex => codex_credential(&bytes).map(|(token, login)| (token, Some(login))),
            // Custody is for a vendor login this process must read, hold and proxy. `b10x-harness`
            // is handed a file path and reads it itself, so metaharness never has the credential
            // and has nothing to take custody of. A parser here would be one written for a shape
            // nobody sends.
            Kind::B10x => Err(
                "the b10x loop reads its own credential file; metaharness takes no custody of it"
                    .to_owned(),
            ),
        };
        read.map_err(|detail| CustodyError::Malformed {
            path: self.credential.clone(),
            detail,
        })
    }

    /// Block until this process holds the custody lock exclusively.
    ///
    /// The returned handle *is* the lock: the kernel drops the `flock` when the descriptor
    /// closes, so the caller holds the guard for as long as the critical section lasts and does
    /// nothing else with it.
    fn locked(&self) -> Result<File, CustodyError> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.lock)
            .map_err(|error| CustodyError::Io {
                detail: format!("{}: {error}", self.lock.display()),
            })?;
        rustix::fs::flock(&file, rustix::fs::FlockOperation::LockExclusive).map_err(|errno| {
            CustodyError::Io {
                detail: format!(
                    "{}: could not take the custody lock: {errno}",
                    self.lock.display()
                ),
            }
        })?;
        Ok(file)
    }
}

/// Where the sidecar lock for a credential file lives.
///
/// A sibling of the credential rather than a path under the temporary directory, so two runs that
/// name the same credential by the same path lock the same file without agreeing on a convention
/// first — and so an operator who wonders what is holding a run can see it with `ls`.
fn lock_path(credential: &Path) -> PathBuf {
    let mut name = credential
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("credentials"))
        .to_os_string();
    name.push(".mh-lock");
    credential.with_file_name(name)
}

/// The access token out of Claude Code's credential shape, or why it was not there.
///
/// The error strings name the **field that was wrong**, never its contents: a diagnostic that
/// echoed the file would put a live credential into a log, which is the one thing this module
/// cannot do.
fn claude_access_token(bytes: &[u8]) -> Result<String, String> {
    let parsed = parse(bytes)?;
    let oauth = parsed
        .get("claudeAiOauth")
        .ok_or_else(|| "no claudeAiOauth object".to_string())?;
    let token = oauth
        .get("accessToken")
        .ok_or_else(|| "claudeAiOauth has no accessToken".to_string())?;
    string_field(token, "claudeAiOauth.accessToken")
}

/// The bearer and the login class out of codex's `auth.json`, or why neither was there.
///
/// The two classes are read in the order the file itself distinguishes them: an `OPENAI_API_KEY`
/// string is an API-key login, and a `tokens.access_token` string is a ChatGPT-plan one. A file
/// carrying **both** is read as the API key, because that is the class the loopback door routes
/// and reading it the other way would refuse a run this build can honour — and the refusal it
/// would raise names an unverified vendor behaviour, which is a thing to state, not to guess into.
fn codex_credential(bytes: &[u8]) -> Result<(String, CodexLogin), String> {
    let parsed = parse(bytes)?;
    if let Some(key) = parsed.get("OPENAI_API_KEY").filter(|key| !key.is_null()) {
        return Ok((string_field(key, "OPENAI_API_KEY")?, CodexLogin::ApiKey));
    }
    if let Some(tokens) = parsed.get("tokens").filter(|tokens| !tokens.is_null()) {
        let token = tokens
            .get("access_token")
            .ok_or_else(|| "tokens has no access_token".to_string())?;
        return Ok((
            string_field(token, "tokens.access_token")?,
            CodexLogin::Subscription,
        ));
    }
    Err("neither an OPENAI_API_KEY nor a tokens object: this is not a codex login".to_string())
}

fn parse(bytes: &[u8]) -> Result<serde_json::Value, String> {
    serde_json::from_slice(bytes).map_err(|error| format!("not JSON: {error}"))
}

/// One string field, named rather than valued in every error it can raise.
fn string_field(value: &serde_json::Value, named: &str) -> Result<String, String> {
    let text = value
        .as_str()
        .ok_or_else(|| format!("{named} is not a string"))?;
    if text.is_empty() {
        return Err(format!("{named} is empty"));
    }
    Ok(text.to_string())
}

/// Why custody could not answer.
///
/// No variant carries a token. `Malformed` carries the path and a shape complaint, and the shape
/// complaint is generated from field names — see [`access_token`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustodyError {
    /// The file or its lock could not be used. Carries what the platform said, with the path.
    Io {
        /// The platform's message, prefixed by the path it was about.
        detail: String,
    },
    /// The file is not the credential shape this module reads.
    Malformed {
        /// Which file.
        path: PathBuf,
        /// Which field was missing or the wrong type — never its value.
        detail: String,
    },
    /// The re-read token is byte-identical to the stale one: nothing refreshed it.
    ///
    /// The caller's obligation on this is to relay the upstream's own 401 to the child and count
    /// the failure — not to retry, which would spin, and not to substitute a friendlier error,
    /// which would hide the vendor's reason.
    StillStale,
}

impl fmt::Display for CustodyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CustodyError::Io { detail } => write!(f, "the credential could not be read: {detail}"),
            CustodyError::Malformed { path, detail } => write!(
                f,
                "{} is not a credential file in the shape its vendor writes: {detail}",
                path.display()
            ),
            CustodyError::StillStale => f.write_str(
                "the credential re-read identically after an upstream 401: metaharness does not \
                 perform the OAuth refresh itself in v1, so this means the vendor's own tooling \
                 has not refreshed the file. Log in again with the vendor CLI",
            ),
        }
    }
}

impl std::error::Error for CustodyError {}

impl From<io::Error> for CustodyError {
    fn from(error: io::Error) -> Self {
        CustodyError::Io {
            detail: error.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    /// A credential file with the vendor's shape and a token nobody's account has ever held.
    fn fake_credential(dir: &Path, token: &str) -> PathBuf {
        let path = dir.join(".credentials.json");
        write_credential(&path, token);
        path
    }

    fn write_credential(path: &Path, token: &str) {
        let body = serde_json::json!({
            "claudeAiOauth": {
                "accessToken": token,
                "refreshToken": "fake-refresh-token",
                "expiresAt": 4_102_444_800_000_i64,
                "refreshTokenExpiresAt": 4_102_444_800_000_i64,
                "scopes": ["user:inference"],
                "subscriptionType": "fake",
                "rateLimitTier": "fake",
            }
        });
        std::fs::write(path, serde_json::to_vec(&body).expect("a credential body"))
            .expect("the fake credential");
    }

    #[test]
    fn the_bearer_is_the_access_token_out_of_the_vendors_shape() {
        let dir = tempfile::TempDir::new().expect("a directory");
        let path = fake_credential(dir.path(), "fake-token-one");
        let custody =
            CredentialCustody::open(Kind::Claude, &path).expect("the fake credential opens");
        assert_eq!(
            custody.bearer().expect("a bearer"),
            "fake-token-one",
            "the bearer is claudeAiOauth.accessToken verbatim; anything else means this module \
             would hand the upstream a value the vendor did not issue"
        );
    }

    /// A codex `auth.json` in the API-key shape: the class the loopback door routes (LP-4).
    ///
    /// The field name is the pinned binary's own (`AuthDotJson`); the value is one no account has
    /// ever been issued.
    #[test]
    fn a_codex_api_key_login_reads_as_a_bearer_and_as_the_class_the_door_routes() {
        let dir = tempfile::TempDir::new().expect("a directory");
        let path = dir.path().join("auth.json");
        std::fs::write(
            &path,
            br#"{"OPENAI_API_KEY":"sk-not-a-real-key","tokens":null,"last_refresh":null}"#,
        )
        .expect("a codex login");
        let custody = CredentialCustody::open(Kind::Codex, &path).expect("it opens");
        assert_eq!(custody.bearer().expect("a bearer"), "sk-not-a-real-key");
        assert_eq!(custody.login(), Some(CodexLogin::ApiKey));
    }

    /// A codex `auth.json` in the ChatGPT-plan shape opens — and is classified, which is what the
    /// launch refuses on. **Classifying is not routing**: V-LP6 is unanswered, so the door says so
    /// by name rather than sending a subscription token at a provider nobody has proven honours it.
    #[test]
    fn a_codex_subscription_login_opens_and_is_classified_rather_than_refused_here() {
        let dir = tempfile::TempDir::new().expect("a directory");
        let path = dir.path().join("auth.json");
        std::fs::write(
            &path,
            br#"{"OPENAI_API_KEY":null,"tokens":{"access_token":"fake-chatgpt-access","refresh_token":"fake-refresh","account_id":"fake-account"},"last_refresh":"2026-08-23T00:00:00Z"}"#,
        )
        .expect("a codex login");
        let custody = CredentialCustody::open(Kind::Codex, &path).expect("it opens");
        assert_eq!(custody.bearer().expect("a bearer"), "fake-chatgpt-access");
        assert_eq!(custody.login(), Some(CodexLogin::Subscription));
    }

    /// A Claude Code login draws no such distinction, and the absence is reported as one.
    #[test]
    fn a_claude_login_reports_no_codex_login_class_at_all() {
        let dir = tempfile::TempDir::new().expect("a directory");
        let path = fake_credential(dir.path(), "fake-token-one");
        let custody = CredentialCustody::open(Kind::Claude, &path).expect("it opens");
        assert_eq!(custody.login(), None);
    }

    /// Each vendor's shape is read as that vendor's, and the other one is a named refusal rather
    /// than an empty token: reading a codex file with the claude parser would otherwise produce
    /// "no token" for a file that holds a perfectly good one.
    #[test]
    fn one_vendors_credential_is_not_read_as_the_others() {
        let dir = tempfile::TempDir::new().expect("a directory");
        let claude = fake_credential(dir.path(), "fake-token-one");
        let error = CredentialCustody::open(Kind::Codex, &claude)
            .expect_err("a claude file is not a codex login");
        assert!(
            error.to_string().contains("not a codex login"),
            "the refusal must name the shape it wanted: {error}"
        );

        let codex = dir.path().join("auth.json");
        std::fs::write(&codex, br#"{"OPENAI_API_KEY":"sk-not-a-real-key"}"#)
            .expect("a codex login");
        let error = CredentialCustody::open(Kind::Claude, &codex)
            .expect_err("a codex file is not a claude login");
        assert!(
            error.to_string().contains("claudeAiOauth"),
            "the refusal must name the field it wanted: {error}"
        );
    }

    /// A malformed credential is refused at `open`, not at the first request an hour into a run —
    /// which is the shape of the incident (Q13) this whole subsystem answers.
    #[test]
    fn a_file_that_is_not_the_vendors_shape_is_refused_at_open_naming_the_field() {
        let dir = tempfile::TempDir::new().expect("a directory");
        let path = dir.path().join(".credentials.json");
        std::fs::write(&path, br#"{"someOtherVendor": {"token": "x"}}"#).expect("a wrong shape");
        let error =
            CredentialCustody::open(Kind::Claude, &path).expect_err("a wrong shape is not custody");
        assert_eq!(
            error.kind(),
            io::ErrorKind::InvalidData,
            "a readable file with the wrong shape is InvalidData, not NotFound: the operator's \
             next move differs (fix the file vs log in), so the two must not be one error"
        );
        assert!(
            error.to_string().contains("claudeAiOauth"),
            "the refusal must name the missing field so the operator does not open the source \
             to find out what shape was wanted, got: {error}"
        );
    }

    #[test]
    fn a_missing_credential_file_is_notfound_and_names_the_path() {
        let dir = tempfile::TempDir::new().expect("a directory");
        let path = dir.path().join("absent.json");
        let error = CredentialCustody::open(Kind::Claude, &path).expect_err("nothing to open");
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert!(
            error.to_string().contains("absent.json"),
            "the refusal names the path it looked at, got: {error}"
        );
    }

    /// The signal the proxy counts as `refresh_failed`: nothing wrote the file, so the token that
    /// the upstream already rejected is the token that came back.
    #[test]
    fn a_reread_that_is_byte_identical_is_stillstale_and_not_a_success() {
        let dir = tempfile::TempDir::new().expect("a directory");
        let path = fake_credential(dir.path(), "fake-token-stale");
        let custody = CredentialCustody::open(Kind::Claude, &path).expect("it opens");
        assert_eq!(
            custody.refreshed("fake-token-stale"),
            Err(CustodyError::StillStale),
            "an unchanged file must not read as a refresh, or the proxy would retry with the \
             token the upstream just rejected and count it as a recovery"
        );
    }

    #[test]
    fn a_reread_after_the_vendor_rewrote_the_file_yields_the_new_token() {
        let dir = tempfile::TempDir::new().expect("a directory");
        let path = fake_credential(dir.path(), "fake-token-stale");
        let custody = CredentialCustody::open(Kind::Claude, &path).expect("it opens");
        write_credential(&path, "fake-token-fresh");
        assert_eq!(
            custody
                .refreshed("fake-token-stale")
                .expect("a fresh token"),
            "fake-token-fresh",
            "custody re-reads the live file rather than a cached copy, which is the whole point \
             of not copying it into the child"
        );
    }

    /// Vector 7. Two custody handles on one file, racing a writer that rewrites it in two steps
    /// under the same lock. Every `refreshed()` must return a whole credential: a `Malformed`
    /// here is a torn read, which in production is a truncated token handed to the upstream as
    /// if it were the operator's.
    #[test]
    fn two_custodies_on_one_file_serialize_and_never_see_a_torn_write() {
        two_custodies_race_a_writer(Duration::from_millis(1));
    }

    /// The same vector with a writer 25x slower, which is what makes the fixture provable.
    ///
    /// **The reader loop's bound is the thing under test here, not the lock.** With a fixed
    /// `for 0..200`, 400 lock-acquire-read cycles finish in well under a millisecond, so a writer
    /// that flips every 50 ms is never observed to flip at all and `seen` holds one token — the
    /// test then fails its own setup check and reads as a serialization failure, which is what
    /// made it flaky under load. Slowing the writer turns "sometimes, on a loaded machine" into
    /// "always", so the bound can be demonstrated rather than argued about: revert the reader loop
    /// to `for 0..200` and this test fails every run, on an idle machine, in 20 ms.
    #[test]
    fn a_slow_writer_is_still_raced_because_the_readers_wait_for_it() {
        two_custodies_race_a_writer(Duration::from_millis(25));
    }

    fn two_custodies_race_a_writer(writer_cycle: Duration) {
        let dir = tempfile::TempDir::new().expect("a directory");
        let path = fake_credential(dir.path(), "fake-token-stale");
        let lock = lock_path(&path);
        let stop = Arc::new(AtomicBool::new(false));

        // The in-place rewrite a refresher performs: truncate, pause, write. Between those two
        // lines the file is not a credential at all — which is what the lock exists to hide.
        let writer = {
            let path = path.clone();
            let stop = Arc::clone(&stop);
            std::thread::spawn(move || {
                let mut fresh = false;
                while !stop.load(Ordering::SeqCst) {
                    let handle = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&lock)
                        .expect("the same sidecar the custodies take");
                    rustix::fs::flock(&handle, rustix::fs::FlockOperation::LockExclusive)
                        .expect("the lock");
                    std::fs::write(&path, b"").expect("the truncation");
                    std::thread::sleep(writer_cycle);
                    write_credential(
                        &path,
                        if fresh {
                            "fake-token-fresh"
                        } else {
                            "fake-token-stale"
                        },
                    );
                    fresh = !fresh;
                    drop(handle);
                    std::thread::sleep(writer_cycle);
                }
            })
        };

        let torn: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let seen: Arc<Mutex<BTreeSet<String>>> = Arc::new(Mutex::new(BTreeSet::new()));
        let readers: Vec<_> = (0..2)
            .map(|_| {
                let path = path.clone();
                let torn = Arc::clone(&torn);
                let seen = Arc::clone(&seen);
                std::thread::spawn(move || {
                    let custody = CredentialCustody::open(Kind::Claude, &path)
                        .expect("a second handle on one file");
                    // **Bounded by outcome and a deadline, not by an iteration count.** 200 reads
                    // apiece is 400 lock-acquire-read cycles, which on an idle machine finish
                    // inside the writer's first 2 ms cycle — so the readers can complete before
                    // the writer has flipped the token even once, and the `seen.len() >= 2`
                    // self-check below then fails for a reason that is about scheduling and not
                    // about serialization. The deadline is generous because it is a backstop
                    // against a writer that never runs at all, not a timing assumption.
                    let deadline = std::time::Instant::now() + Duration::from_secs(5);
                    let mut reads = 0;
                    loop {
                        // A `stale` no file ever holds, so every read is a "fresh" answer and the
                        // call exercises the locked re-read rather than the equality shortcut.
                        match custody.refreshed("no-file-holds-this") {
                            Ok(token) => {
                                seen.lock().expect("the tally").insert(token);
                            }
                            Err(error) => torn.lock().expect("the tally").push(error.to_string()),
                        }
                        reads += 1;
                        let raced = seen.lock().expect("the tally").len() >= 2;
                        if reads >= 200 && (raced || std::time::Instant::now() >= deadline) {
                            break;
                        }
                    }
                })
            })
            .collect();
        for reader in readers {
            reader.join().expect("a reader");
        }
        stop.store(true, Ordering::SeqCst);
        writer.join().expect("the writer");

        let torn = torn.lock().expect("the tally");
        assert!(
            torn.is_empty(),
            "every read under the lock must see a whole credential; these did not: {torn:?}"
        );
        let seen = seen.lock().expect("the tally");
        assert!(
            seen.len() >= 2,
            "the writer did not rewrite the file once in five seconds, so nothing here was raced \
             and the assertion above proves nothing about serialization. This is a statement about \
             the fixture, not about custody: read it as the test failing to set itself up, and \
             look at the writer thread rather than at the lock. Tokens observed: {seen:?}"
        );
    }
}
