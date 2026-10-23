use super::{IDLE_TIMEOUT_MS, Refusal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseReason {
    Completed,
    Cancelled,
    Stale,
    Refused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Offered,
    Ready,
    Transferring,
    Closed(CloseReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Invalidation {
    ItemOrBackpackChanged,
    Distance,
    Logout,
    Reconnect,
    ChannelTransfer,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accept {
    First,
    Repeated,
    Transfer,
}

/// Pure lifecycle, parameterized by immutable owner-issued offer and accept bindings.
/// It cannot settle TRANSFERRING: only the durable owner may discard it after resolving
/// the frozen occurrence. There is deliberately no public `close(outcome)` method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trade<T> {
    phase: Phase,
    offers: [Option<T>; 2],
    accepts: [Option<T>; 2],
    pair: Option<T>,
    last_activity: u64,
    last_observed: u64,
}

impl<T: PartialEq> Trade<T> {
    pub const fn phase(&self) -> Phase {
        self.phase
    }
    pub fn offered(offer: T, now_ms: u64) -> Self {
        Self {
            phase: Phase::Offered,
            offers: [Some(offer), None],
            accepts: [None, None],
            pair: None,
            last_activity: now_ms,
            last_observed: now_ms,
        }
    }

    pub fn observe(&mut self, now_ms: u64) -> Result<(), Refusal> {
        if now_ms < self.last_observed {
            return Err(Refusal::ClockReversed);
        }
        self.last_observed = now_ms;
        if matches!(self.phase, Phase::Offered | Phase::Ready)
            && now_ms - self.last_activity >= IDLE_TIMEOUT_MS
        {
            self.phase = Phase::Closed(CloseReason::Cancelled);
        }
        Ok(())
    }

    pub fn counter_offer(&mut self, offer: T, pair: T, now_ms: u64) -> Result<(), Refusal> {
        self.observe(now_ms)?;
        if self.phase != Phase::Offered {
            return Err(Refusal::Stale);
        }
        self.offers[1] = Some(offer);
        self.pair = Some(pair);
        self.phase = Phase::Ready;
        self.last_activity = now_ms;
        Ok(())
    }

    /// The owner must revalidate current authority, offers and pair facts before calling.
    /// A stale/foreign pair does not cancel an unrelated READY pair.
    pub fn accept(
        &mut self,
        side: usize,
        pair: &T,
        command: T,
        now_ms: u64,
    ) -> Result<Accept, Refusal> {
        self.observe(now_ms)?;
        if side >= 2 || self.phase != Phase::Ready || self.pair.as_ref() != Some(pair) {
            return Err(Refusal::Stale);
        }
        // A repeated first acceptance cannot change its immutable CommandRef or extend idle time.
        if self.accepts[side].is_some() {
            return Ok(Accept::Repeated);
        }
        self.accepts[side] = Some(command);
        self.last_activity = now_ms;
        if self.accepts[1 - side].is_some() {
            self.phase = Phase::Transferring;
            Ok(Accept::Transfer)
        } else {
            Ok(Accept::First)
        }
    }

    pub fn invalidate(&mut self, cause: Invalidation, now_ms: u64) -> Result<(), Refusal> {
        self.observe(now_ms)?;
        if self.phase == Phase::Transferring {
            return Err(Refusal::Transferring);
        }
        if !matches!(self.phase, Phase::Closed(_)) {
            self.phase = Phase::Closed(match cause {
                Invalidation::ItemOrBackpackChanged => CloseReason::Stale,
                _ => CloseReason::Cancelled,
            });
        }
        Ok(())
    }
}
