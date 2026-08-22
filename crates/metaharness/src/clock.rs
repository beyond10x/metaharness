//! The one clock metaharness reads.
//!
//! The protocol's `at` is the timestamp **the vendor recorded** and nothing in
//! `metaharness-protocol` reads a clock (design D2). That rule is about the record. A decision
//! deadline is not a record: it is metaharness's own budget, and § 7.7 rule 2 requires it to
//! expire. So exactly one clock exists, it is behind a trait, and it never touches an event's
//! `at` — which is what keeps a run's numbers committable and diffable.

use std::time::Instant;

/// A source of elapsed milliseconds, and a way to wait.
pub trait Clock {
    /// Milliseconds since this clock's own origin. Never used as an event timestamp.
    fn now_ms(&mut self) -> u64;

    /// Wait until [`Clock::now_ms`] is at least this. Returns immediately if it already is.
    fn sleep_until_ms(&mut self, deadline_ms: u64);
}

/// The real clock: monotone milliseconds since the run was constructed.
#[derive(Debug)]
pub struct SystemClock {
    origin: Instant,
}

impl SystemClock {
    /// A clock whose origin is now.
    #[must_use]
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn now_ms(&mut self) -> u64 {
        u64::try_from(self.origin.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    fn sleep_until_ms(&mut self, deadline_ms: u64) {
        let now = self.now_ms();
        if deadline_ms > now {
            std::thread::sleep(std::time::Duration::from_millis(deadline_ms - now));
        }
    }
}

/// A clock a test moves by hand. **Test support**, and public because the deadline vectors are.
///
/// Waiting is jumping: a deadline vector that slept for real would buy a slow suite and prove
/// the same thing.
///
/// A **shared handle**, so a clone handed to a run and the clone the test kept read the same
/// number. Two independent clocks would let a vector assert a deadline that never expired for
/// the run it was asserting about.
#[derive(Debug, Default, Clone)]
pub struct ManualClock {
    now: std::rc::Rc<std::cell::Cell<u64>>,
}

impl ManualClock {
    /// A clock reading zero.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Move the clock forward by this many milliseconds.
    pub fn advance(&self, milliseconds: u64) {
        self.now.set(self.now.get() + milliseconds);
    }

    /// What it reads now, without borrowing it mutably.
    #[must_use]
    pub fn reading_ms(&self) -> u64 {
        self.now.get()
    }
}

impl Clock for ManualClock {
    fn now_ms(&mut self) -> u64 {
        self.now.get()
    }

    fn sleep_until_ms(&mut self, deadline_ms: u64) {
        self.now.set(self.now.get().max(deadline_ms));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shared_manual_clock_is_read_the_same_by_every_holder() {
        let held = ManualClock::new();
        let mut given_away = held.clone();
        given_away.sleep_until_ms(400);
        assert_eq!(held.reading_ms(), 400);
    }

    #[test]
    fn a_manual_clock_jumps_to_the_deadline_instead_of_sleeping() {
        let mut clock = ManualClock::new();
        clock.sleep_until_ms(5_000);
        assert_eq!(clock.now_ms(), 5_000);
    }

    #[test]
    fn a_clock_never_moves_backwards_when_asked_to_wait_for_the_past() {
        let mut clock = ManualClock::new();
        clock.advance(10);
        clock.sleep_until_ms(1);
        assert_eq!(clock.now_ms(), 10);
    }
}
