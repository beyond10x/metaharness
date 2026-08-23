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
//! # The file shape this reads
//!
//! Claude Code's `~/.claude/.credentials.json`, inspected on 2026-08-23 for **field names only**
//! — no value in this repository, this module's tests, or any error it raises ever comes from the
//! operator's real file:
//!
//! ```text
//! { "claudeAiOauth": { "accessToken", "refreshToken", "expiresAt",
//!                      "refreshTokenExpiresAt", "scopes", "subscriptionType", "rateLimitTier" } }
//! ```
//!
//! Only `claudeAiOauth.accessToken` is read. `expiresAt` is deliberately **not** consulted: a
//! clock-based prediction of expiry is a second opinion that can disagree with the upstream, and
//! the upstream's 401 is the only authority on whether a token still works.
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

/// One credential file, and the lock that makes reads of it serialize.
///
/// Cheap to clone by `Arc` and safe to share: it holds no token, only paths. The token is read
/// from the file at every call, because a token cached in a field is the copy-with-a-lifetime
/// problem again, one scope smaller.
#[derive(Debug)]
pub struct CredentialCustody {
    credential: PathBuf,
    lock: PathBuf,
}

impl CredentialCustody {
    /// The operator's live credential file (Claude Code's `.credentials.json` shape).
    ///
    /// Reads it once, under the lock, to prove the shape is the one this module knows. A run
    /// whose custody is malformed is refused **at launch** rather than an hour in, which is the
    /// failure Q13 recorded. The token read for that check is dropped and never stored.
    ///
    /// # Errors
    ///
    /// [`io::ErrorKind::NotFound`] and friends when the file or its directory cannot be used, and
    /// [`io::ErrorKind::InvalidData`] when the file is not the shape above. Every message names
    /// the path; none of them can name a token, because the parse either found a string at
    /// `claudeAiOauth.accessToken` or reports that it did not.
    pub fn open(path: &Path) -> io::Result<Self> {
        let custody = Self {
            credential: path.to_path_buf(),
            lock: lock_path(path),
        };
        match custody.read_token() {
            Ok(_) => Ok(custody),
            Err(CustodyError::Io { detail }) => {
                Err(io::Error::new(io::ErrorKind::NotFound, detail))
            }
            Err(other) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                other.to_string(),
            )),
        }
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
    fn read_token(&self) -> Result<String, CustodyError> {
        let _guard = self.locked()?;
        let bytes = std::fs::read(&self.credential).map_err(|error| CustodyError::Io {
            detail: format!("{}: {error}", self.credential.display()),
        })?;
        access_token(&bytes).map_err(|detail| CustodyError::Malformed {
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
fn access_token(bytes: &[u8]) -> Result<String, String> {
    let parsed: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| format!("not JSON: {error}"))?;
    let oauth = parsed
        .get("claudeAiOauth")
        .ok_or_else(|| "no claudeAiOauth object".to_string())?;
    let token = oauth
        .get("accessToken")
        .ok_or_else(|| "claudeAiOauth has no accessToken".to_string())?;
    let token = token
        .as_str()
        .ok_or_else(|| "claudeAiOauth.accessToken is not a string".to_string())?;
    if token.is_empty() {
        return Err("claudeAiOauth.accessToken is empty".to_string());
    }
    Ok(token.to_string())
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
                "{} is not a Claude Code credential file: {detail}",
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
        let custody = CredentialCustody::open(&path).expect("the fake credential opens");
        assert_eq!(
            custody.bearer().expect("a bearer"),
            "fake-token-one",
            "the bearer is claudeAiOauth.accessToken verbatim; anything else means this module \
             would hand the upstream a value the vendor did not issue"
        );
    }

    /// A malformed credential is refused at `open`, not at the first request an hour into a run —
    /// which is the shape of the incident (Q13) this whole subsystem answers.
    #[test]
    fn a_file_that_is_not_the_vendors_shape_is_refused_at_open_naming_the_field() {
        let dir = tempfile::TempDir::new().expect("a directory");
        let path = dir.path().join(".credentials.json");
        std::fs::write(&path, br#"{"someOtherVendor": {"token": "x"}}"#).expect("a wrong shape");
        let error = CredentialCustody::open(&path).expect_err("a wrong shape is not custody");
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
        let error = CredentialCustody::open(&path).expect_err("nothing to open");
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
        let custody = CredentialCustody::open(&path).expect("it opens");
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
        let custody = CredentialCustody::open(&path).expect("it opens");
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
                    std::thread::sleep(Duration::from_millis(1));
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
                    std::thread::sleep(Duration::from_millis(1));
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
                    let custody =
                        CredentialCustody::open(&path).expect("a second handle on one file");
                    for _ in 0..200 {
                        // A `stale` no file ever holds, so every read is a "fresh" answer and the
                        // call exercises the locked re-read rather than the equality shortcut.
                        match custody.refreshed("no-file-holds-this") {
                            Ok(token) => {
                                seen.lock().expect("the tally").insert(token);
                            }
                            Err(error) => torn.lock().expect("the tally").push(error.to_string()),
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
            "the writer must actually have raced the readers, or this test proves nothing about \
             serialization; tokens observed: {seen:?}"
        );
    }
}
