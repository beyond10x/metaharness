//! A named run's vendor child leads a process group of its own (design amendment a72 § 5), so a
//! terminal's `SIGINT` reaches metaharness and no longer the vendor. While a named run is going,
//! `SIGINT`, `SIGTERM` and `SIGHUP` are forwarded to that group, and metaharness then ends as the
//! signal would have ended it, after the run has ended and its root and locks are gone.

use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};

/// How long the group has, after the forwarded signal, before it is sent `SIGKILL`: a vendor that
/// handles the signal ends its session in this time, and one that ignores it (a background job of
/// a shell ignores `SIGINT`) is ended anyway.
const FORWARD_GRACE: Duration = Duration::from_secs(1);

/// How long the run has, after the group was killed, to end and be dropped before metaharness
/// ends by the signal without it.
const END_GRACE: Duration = Duration::from_secs(5);

/// The handlers for one named run, from before its start until it is dropped.
pub(crate) struct GroupForwarding {
    shared: Arc<Shared>,
    handle: signal_hook::iterator::Handle,
}

#[derive(Default)]
struct Shared {
    state: Mutex<Forwarded>,
    ended: Condvar,
}

#[derive(Default)]
struct Forwarded {
    /// The run's process group, once its start returned one.
    group: Option<i32>,
    /// The first of the three signals received.
    received: Option<i32>,
    /// The grace after the forwarded signal is over and the group was sent `SIGKILL`.
    escalated: bool,
    /// The run was dropped.
    ended: bool,
}

impl GroupForwarding {
    /// Take over `SIGINT`, `SIGTERM` and `SIGHUP` for this process.
    pub(crate) fn install() -> std::io::Result<Self> {
        let mut signals = signal_hook::iterator::Signals::new([SIGINT, SIGTERM, SIGHUP])?;
        let handle = signals.handle();
        let shared = Arc::new(Shared::default());
        let theirs = Arc::clone(&shared);
        std::thread::spawn(move || {
            if let Some(signal) = signals.forever().next() {
                end_on(&theirs, signal);
            }
        });
        Ok(Self { shared, handle })
    }

    /// The run started, its vendor child leading `group`. A signal that came during the start is
    /// forwarded now.
    pub(crate) fn lead(&self, group: Option<i32>) {
        let mut state = lock(&self.shared);
        state.group = group;
        if let (Some(group), Some(signal)) = (group, state.received) {
            signal_group(group, signal);
            if state.escalated {
                signal_group(group, signal_hook::consts::SIGKILL);
            }
        }
    }

    /// The run was dropped, or never started. When a signal came, metaharness now ends by it.
    pub(crate) fn finish(self) {
        let received = {
            let mut state = lock(&self.shared);
            state.ended = true;
            self.shared.ended.notify_all();
            state.received
        };
        self.handle.close();
        if let Some(signal) = received {
            die_by(signal);
        }
    }
}

/// The first signal: forward it to the group, give the group [`FORWARD_GRACE`], then `SIGKILL` it,
/// and end by the signal after [`END_GRACE`] if the run has still not been dropped.
fn end_on(shared: &Shared, signal: i32) {
    let mut state = lock(shared);
    state.received = Some(signal);
    if let Some(group) = state.group {
        signal_group(group, signal);
    }
    let (mut state, _) = shared
        .ended
        .wait_timeout_while(state, FORWARD_GRACE, |state| !state.ended)
        .unwrap_or_else(PoisonError::into_inner);
    if state.ended {
        return;
    }
    state.escalated = true;
    if let Some(group) = state.group {
        signal_group(group, signal_hook::consts::SIGKILL);
    }
    let (state, _) = shared
        .ended
        .wait_timeout_while(state, END_GRACE, |state| !state.ended)
        .unwrap_or_else(PoisonError::into_inner);
    if !state.ended {
        drop(state);
        die_by(signal);
    }
}

fn lock(shared: &Shared) -> MutexGuard<'_, Forwarded> {
    shared.state.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Send `signal` to the process group `group`. A group already gone is not an error.
fn signal_group(group: i32, signal: i32) {
    let signal = match signal {
        SIGINT => rustix::process::Signal::INT,
        SIGTERM => rustix::process::Signal::TERM,
        SIGHUP => rustix::process::Signal::HUP,
        _ => rustix::process::Signal::KILL,
    };
    if let Some(group) = rustix::process::Pid::from_raw(group) {
        let _ = rustix::process::kill_process_group(group, signal);
    }
}

/// End this process as `signal`'s default action would have, `128 + signal` should that fail.
fn die_by(signal: i32) -> ! {
    let _ = signal_hook::low_level::emulate_default_handler(signal);
    std::process::exit(128 + signal)
}
