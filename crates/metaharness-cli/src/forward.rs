//! A named run's vendor child leads a process group of its own (design amendment a72 § 5), so the
//! terminal's signals reach metaharness and no longer the vendor. While a named run is going,
//! metaharness passes them on:
//!
//! - `SIGINT`, `SIGTERM`, `SIGHUP` and `SIGQUIT` go to the group, which is sent `SIGKILL` a moment
//!   later; metaharness then ends by the signal, after the run has ended and its root and locks
//!   are gone.
//! - `SIGTSTP` stops the group and then metaharness itself; `SIGCONT` continues the group.
//!
//! When the run is over, each of these signals takes its default action again.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use metaharness::ProcessGroup;
use signal_hook::consts::{SIGCONT, SIGHUP, SIGINT, SIGKILL, SIGQUIT, SIGTERM, SIGTSTP};

/// The signals that end the run when they reach metaharness.
const ENDING: [i32; 4] = [SIGINT, SIGTERM, SIGHUP, SIGQUIT];

/// Every signal taken over for a named run's life.
const TAKEN: [i32; 6] = [SIGINT, SIGTERM, SIGHUP, SIGQUIT, SIGTSTP, SIGCONT];

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
    default: Arc<AtomicBool>,
}

#[derive(Default)]
struct Shared {
    state: Mutex<Forwarded>,
    ended: Condvar,
}

#[derive(Default)]
struct Forwarded {
    /// The run's process group, once its start returned one.
    group: Option<ProcessGroup>,
    /// The first ending signal received.
    received: Option<i32>,
    /// The grace after the forwarded signal is over and the group was sent `SIGKILL`.
    escalated: bool,
    /// The run was dropped.
    ended: bool,
}

/// The process's switch between forwarding and the default action, registered once per process
/// for every signal in [`TAKEN`]: `true`, and a signal-hook handler emulates the signal's default
/// action, as if it had none.
///
/// signal-hook keeps its low-level handler installed once a signal has had one, and closing a
/// [`signal_hook::iterator::Signals`] only unregisters its action, which would leave the signal
/// caught and ignored for the rest of the process. A caller of `execute` that goes on after a
/// named run (a later unnamed run, a test harness) must find each signal's default action back.
fn default_switch() -> std::io::Result<Arc<AtomicBool>> {
    static SWITCH: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

    let mut held = SWITCH.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(switch) = held.as_ref() {
        return Ok(Arc::clone(switch));
    }
    let switch = Arc::new(AtomicBool::new(true));
    for signal in TAKEN {
        signal_hook::flag::register_conditional_default(signal, Arc::clone(&switch))?;
    }
    *held = Some(Arc::clone(&switch));
    Ok(switch)
}

impl GroupForwarding {
    /// Take over the signals in [`TAKEN`] for this process.
    pub(crate) fn install() -> std::io::Result<Self> {
        let default = default_switch()?;
        default.store(false, Ordering::SeqCst);
        let mut signals = match signal_hook::iterator::Signals::new(TAKEN) {
            Ok(signals) => signals,
            Err(error) => {
                default.store(true, Ordering::SeqCst);
                return Err(error);
            }
        };
        let handle = signals.handle();
        let shared = Arc::new(Shared::default());
        let theirs = Arc::clone(&shared);
        std::thread::spawn(move || {
            let mut ending = false;
            for signal in signals.forever() {
                match signal {
                    SIGTSTP => {
                        forward(&theirs, SIGTSTP);
                        // The default action of `SIGTSTP`: stop, until a `SIGCONT`.
                        let _ = signal_hook::low_level::emulate_default_handler(SIGTSTP);
                    }
                    SIGCONT => forward(&theirs, SIGCONT),
                    _ if !ending && ENDING.contains(&signal) => {
                        ending = true;
                        let theirs = Arc::clone(&theirs);
                        std::thread::spawn(move || end_on(&theirs, signal));
                    }
                    _ => {}
                }
            }
        });
        Ok(Self {
            shared,
            handle,
            default,
        })
    }

    /// The run started, its vendor child leading `group`. A signal that came during the start is
    /// forwarded now.
    pub(crate) fn lead(&self, group: Option<ProcessGroup>) {
        let mut state = lock(&self.shared);
        if let (Some(group), Some(signal)) = (group.as_ref(), state.received) {
            group.signal(signal);
            if state.escalated {
                group.signal(SIGKILL);
            }
        }
        state.group = group;
    }

    /// The run was dropped, or never started. Every taken signal has its default action again,
    /// and when an ending signal came, metaharness now ends by it.
    pub(crate) fn finish(self) {
        let received = {
            let mut state = lock(&self.shared);
            state.ended = true;
            self.shared.ended.notify_all();
            state.received
        };
        self.default.store(true, Ordering::SeqCst);
        self.handle.close();
        if let Some(signal) = received {
            die_by(signal);
        }
    }
}

/// Pass `signal` on to the run's group, when there is one and its leader is unreaped.
fn forward(shared: &Shared, signal: i32) {
    if let Some(group) = lock(shared).group.as_ref() {
        group.signal(signal);
    }
}

/// The first ending signal: forward it to the group, give the group [`FORWARD_GRACE`], then
/// `SIGKILL` it, and end by the signal after [`END_GRACE`] if the run has still not been dropped.
fn end_on(shared: &Shared, signal: i32) {
    let mut state = lock(shared);
    state.received = Some(signal);
    if let Some(group) = state.group.as_ref() {
        group.signal(signal);
    }
    let (mut state, _) = shared
        .ended
        .wait_timeout_while(state, FORWARD_GRACE, |state| !state.ended)
        .unwrap_or_else(PoisonError::into_inner);
    if state.ended {
        return;
    }
    state.escalated = true;
    if let Some(group) = state.group.as_ref() {
        group.signal(SIGKILL);
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

/// End this process as `signal`'s default action would have, `128 + signal` should that fail.
fn die_by(signal: i32) -> ! {
    let _ = signal_hook::low_level::emulate_default_handler(signal);
    std::process::exit(128 + signal)
}
