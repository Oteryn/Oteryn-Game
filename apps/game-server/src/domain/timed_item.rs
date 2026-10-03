//! Live values, the per-item write lane and stop of a timed item (TIMED-RT-1a; decision
//! `TIMEDITEM0B-RUNTIME-CHARGES-AND-DURATION-V1` §5-§8).
//!
//! A [`TimedItemLane`] holds one live item's charges and remaining time in its hosting runtime
//! and issues its timed writes one at a time (`TIMEDITEM0B-RL-05`): a checkpoint at the A13
//! cadence or at a stop, and the expiry once a live value reaches 0. The next write is issued
//! only when the previous outcome is known, so an expiry is always keyed to the revision the
//! last committed write produced (the round 12 race, D285). Storage is not touched here: the
//! hosting runtime runs each [`LaneWrite`] through `durability::item_timed_state` and reports
//! the outcome back. [`ActorTimedLanes`] bounds the live items of one actor
//! (`TIMEDITEM0B-RL-04`) and tells logout, handoff and death settlement when every lane is
//! empty (§6.1).

use std::collections::BTreeMap;

/// `TIMEDITEM0-RL-01`: charges of one item.
pub const TIMEDITEM0_RL_01_CHARGES_MAX: u32 = 65_535;
/// `TIMEDITEM0-RL-02`: remaining active time of one item (7 days).
pub const TIMEDITEM0_RL_02_REMAINING_MS_MAX: u64 = 604_800_000;
/// `TIMEDITEM0B-RL-01`: at most this long between checkpoints of a changed live item.
pub const TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS: u64 = 60_000;
/// `TIMEDITEM0B-RL-02`: an ambiguous write not resolved within this bound holds the lane until
/// reconciliation reads its record.
pub const TIMEDITEM0B_RL_02_AMBIGUOUS_BOUND_MS: u64 = 2_000;
/// `TIMEDITEM0B-RL-04`: live timed items per actor (ten equipment slots plus one exercise
/// weapon).
pub const TIMEDITEM0B_RL_04_LIVE_ITEMS_PER_ACTOR: usize = 11;
/// `TIMEDITEM0B-RL-05`: timed writes in flight per lane.
pub const TIMEDITEM0B_RL_05_WRITES_IN_FLIGHT_PER_LANE: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimedItemError {
    /// A value is outside `TIMEDITEM0-RL-01`/`-RL-02`, or the item has neither value.
    InvalidValues,
    /// The lane already has its one write in flight (`TIMEDITEM0B-RL-05`).
    WriteInFlight,
    /// The lane is not waiting for the outcome this call reports.
    NoWriteInFlight,
    /// The actor already has `TIMEDITEM0B-RL-04` live items, or this one is live already.
    LiveItemLimit,
    /// The lane no longer accepts writes (expired, lost authority or held).
    Closed,
}

/// Charges and remaining time of one item. A stored value is never 0 charges; a live value
/// reaching 0 is the expiry (§8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedValues {
    charges: Option<u32>,
    remaining_ms: Option<u64>,
}

impl TimedValues {
    /// Stored or full values: charges 1..=RL-01, time 0..=RL-02, at least one of them.
    pub fn new(charges: Option<u32>, remaining_ms: Option<u64>) -> Result<Self, TimedItemError> {
        if (charges.is_none() && remaining_ms.is_none())
            || charges.is_some_and(|value| !(1..=TIMEDITEM0_RL_01_CHARGES_MAX).contains(&value))
            || remaining_ms.is_some_and(|value| value > TIMEDITEM0_RL_02_REMAINING_MS_MAX)
        {
            return Err(TimedItemError::InvalidValues);
        }
        Ok(Self {
            charges,
            remaining_ms,
        })
    }

    #[must_use]
    pub const fn charges(self) -> Option<u32> {
        self.charges
    }

    #[must_use]
    pub const fn remaining_ms(self) -> Option<u64> {
        self.remaining_ms
    }

    const fn exhausted(self) -> Option<ExpireReason> {
        if matches!(self.charges, Some(0)) {
            Some(ExpireReason::ChargesExhausted)
        } else if matches!(self.remaining_ms, Some(0)) {
            Some(ExpireReason::TimeExhausted)
        } else {
            None
        }
    }
}

/// Why a live item expires (§8). `Deadline` belongs to the tile owner's lane (§10.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpireReason {
    TimeExhausted,
    ChargesExhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneWriteKind {
    /// §6.3: stores the live values.
    Checkpoint { values: TimedValues },
    /// §8: writes the final state; the storage side picks the decay target.
    Expire { reason: ExpireReason },
}

/// One timed write, keyed by (item, `expected_revision`). A retry reuses the same
/// TransactionId and expected revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaneWrite {
    pub kind: LaneWriteKind,
    pub expected_revision: u64,
    pub transaction_id: [u8; 16],
}

/// The outcome of the lane's write in flight (§5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneWriteOutcome {
    Committed,
    /// Known not committed (serialization or deadlock): retried unchanged.
    NotCommitted,
    /// DUR-03 §23.3: the lane looks the record up before anything else.
    Ambiguous,
}

/// What the record lookup of an ambiguous write found (§5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmbiguousLookup {
    /// The record of (item, expected revision) carries this write's TransactionId.
    FoundSameTransaction,
    /// No record, and the row is still at the expected revision.
    NotFoundAtExpected,
    /// Still unknown after `TIMEDITEM0B-RL-02`.
    Unknown,
}

/// What the lane does next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneStep {
    /// Nothing to write now.
    Idle,
    /// Run this write (a new one or a retry of the one in flight).
    Write(LaneWrite),
    /// Look up the record of the write in flight before anything else.
    LookUpRecord(LaneWrite),
    /// The item is inactive and reserved until reconciliation reads the record.
    HeldForReconciliation,
    /// The stop is complete: the item is frozen at its row.
    Stopped,
    /// The expiry committed; nothing more is written for this lifetime.
    Expired,
    /// The runtime lost authority (stale fences): nothing more is written for this item.
    LostAuthority,
    /// An integrity fault froze the item at the row's values; an alarm is raised.
    IntegrityFrozen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LaneState {
    Running,
    /// Clock and charge use stopped; the stop checkpoint (if any) is on the lane.
    Stopping,
    Stopped,
    Expired,
    LostAuthority,
    IntegrityFrozen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InFlight {
    Issued(LaneWrite),
    AwaitingLookup(LaneWrite),
    Held(LaneWrite),
}

/// The ordered write queue and live values of one live item (§5.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedItemLane {
    revision: u64,
    stored: TimedValues,
    live: TimedValues,
    state: LaneState,
    in_flight: Option<InFlight>,
    pending_expiry: Option<ExpireReason>,
    expiry_retried: bool,
    last_checkpoint_ms: u64,
    /// The host's checkpoint cadence, at most `TIMEDITEM0B-RL-01`.
    checkpoint_interval_ms: u64,
    /// When the write in flight took its snapshot; a committed checkpoint dates from it.
    issued_ms: u64,
}

impl TimedItemLane {
    /// An item becomes live from its row (`revision` > 0) or, without a row, from its
    /// definition's full values at revision 0 (TIMED-ITEM-0 §4).
    #[must_use]
    pub fn live_from_row(revision: u64, stored: TimedValues, now_ms: u64) -> Self {
        Self {
            revision,
            stored,
            live: stored,
            state: LaneState::Running,
            in_flight: None,
            pending_expiry: stored.exhausted(),
            expiry_retried: false,
            last_checkpoint_ms: now_ms,
            checkpoint_interval_ms: TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS,
            issued_ms: now_ms,
        }
    }

    /// Checkpoint a changed live item every `interval_ms` (1..=`TIMEDITEM0B-RL-01`).
    pub fn with_checkpoint_interval(mut self, interval_ms: u64) -> Result<Self, TimedItemError> {
        if !(1..=TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS).contains(&interval_ms) {
            return Err(TimedItemError::InvalidValues);
        }
        self.checkpoint_interval_ms = interval_ms;
        Ok(self)
    }

    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub const fn live_values(&self) -> TimedValues {
        self.live
    }

    /// §11.1's "live, with charges and time above 0": running, no expiry pending and not
    /// held.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.state == LaneState::Running
            && self.pending_expiry.is_none()
            && !matches!(self.in_flight, Some(InFlight::Held(_)))
    }

    /// The lane has no write in flight and none still to issue: a stop has reached
    /// [`LaneStep::Stopped`] and a pending expiry has committed (or the lane closed). Logout,
    /// handoff and death settlement wait for it.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.in_flight.is_none()
            && self.state != LaneState::Stopping
            && !(self.state == LaneState::Running && self.pending_expiry.is_some())
    }

    /// Spend one live charge (§7). Never written per spend: the next checkpoint commits it.
    /// The charge reaching 0 makes the item inactive at once and queues its expiry.
    pub fn spend_charge(&mut self) -> bool {
        if !self.is_active() {
            return false;
        }
        let Some(charges) = self.live.charges.filter(|value| *value > 0) else {
            return false;
        };
        self.live.charges = Some(charges - 1);
        self.pending_expiry = self.live.exhausted();
        true
    }

    /// Run the live clock by `elapsed_ms`; reaching 0 queues the expiry (§8).
    pub fn advance(&mut self, elapsed_ms: u64) {
        if !self.is_active() {
            return;
        }
        if let Some(remaining) = self.live.remaining_ms {
            self.live.remaining_ms = Some(remaining.saturating_sub(elapsed_ms));
            self.pending_expiry = self.live.exhausted();
        }
    }

    /// Stop (§5.3): the clock and charge use stop now, and the stop checkpoint is queued. The
    /// item is frozen once [`Self::next_step`] reports [`LaneStep::Stopped`].
    pub fn stop(&mut self) {
        if self.state == LaneState::Running {
            self.state = LaneState::Stopping;
        }
    }

    /// The lane's next step. A new write is issued only when nothing is in flight; an expiry
    /// is keyed to the revision the last committed write produced. `transaction_id` is used
    /// only when a new write is issued.
    pub fn next_step(&mut self, now_ms: u64, transaction_id: [u8; 16]) -> LaneStep {
        match self.state {
            LaneState::Expired => return LaneStep::Expired,
            LaneState::LostAuthority => return LaneStep::LostAuthority,
            LaneState::IntegrityFrozen => return LaneStep::IntegrityFrozen,
            LaneState::Stopped => return LaneStep::Stopped,
            LaneState::Running | LaneState::Stopping => {}
        }
        match self.in_flight {
            Some(InFlight::Issued(write)) => return LaneStep::Write(write),
            Some(InFlight::AwaitingLookup(write)) => return LaneStep::LookUpRecord(write),
            Some(InFlight::Held(_)) => return LaneStep::HeldForReconciliation,
            None => {}
        }
        let kind = if let Some(reason) = self.pending_expiry {
            LaneWriteKind::Expire { reason }
        } else if self.live == self.stored {
            // A checkpoint or stop of unchanged values writes nothing.
            if self.state == LaneState::Stopping {
                self.state = LaneState::Stopped;
                return LaneStep::Stopped;
            }
            return LaneStep::Idle;
        } else if self.state == LaneState::Stopping
            || now_ms.saturating_sub(self.last_checkpoint_ms) >= self.checkpoint_interval_ms
        {
            LaneWriteKind::Checkpoint { values: self.live }
        } else {
            return LaneStep::Idle;
        };
        let write = LaneWrite {
            kind,
            expected_revision: self.revision,
            transaction_id,
        };
        self.in_flight = Some(InFlight::Issued(write));
        self.issued_ms = now_ms;
        LaneStep::Write(write)
    }

    /// Report the outcome of the write in flight.
    pub fn on_outcome(&mut self, outcome: LaneWriteOutcome) -> Result<(), TimedItemError> {
        let Some(InFlight::Issued(write)) = self.in_flight else {
            return Err(TimedItemError::NoWriteInFlight);
        };
        match outcome {
            LaneWriteOutcome::Committed => self.committed(write),
            LaneWriteOutcome::NotCommitted => {}
            LaneWriteOutcome::Ambiguous => {
                self.in_flight = Some(InFlight::AwaitingLookup(write));
            }
        }
        Ok(())
    }

    /// Report what the record lookup of the ambiguous write found.
    pub fn on_lookup(&mut self, lookup: AmbiguousLookup) -> Result<(), TimedItemError> {
        let write = match self.in_flight {
            Some(InFlight::AwaitingLookup(write) | InFlight::Held(write)) => write,
            _ => return Err(TimedItemError::NoWriteInFlight),
        };
        match lookup {
            AmbiguousLookup::FoundSameTransaction => self.committed(write),
            AmbiguousLookup::NotFoundAtExpected => {
                self.in_flight = Some(InFlight::Issued(write));
            }
            AmbiguousLookup::Unknown => self.in_flight = Some(InFlight::Held(write)),
        }
        Ok(())
    }

    /// The write found another revision (§5.2 "Unexpected revision"). With stale fences
    /// nothing more is written. With current fences an expiry is retried once at the row's
    /// revision when the row is of the same definition, so the live zero is preserved;
    /// anything else freezes the item at the row's values.
    pub fn on_unexpected_revision(
        &mut self,
        fences_current: bool,
        row_revision: u64,
        row_values: TimedValues,
        same_definition: bool,
    ) -> Result<(), TimedItemError> {
        let Some(InFlight::Issued(write)) = self.in_flight else {
            return Err(TimedItemError::NoWriteInFlight);
        };
        if !fences_current {
            self.in_flight = None;
            self.state = LaneState::LostAuthority;
            return Ok(());
        }
        if matches!(write.kind, LaneWriteKind::Expire { .. })
            && same_definition
            && !self.expiry_retried
        {
            self.expiry_retried = true;
            self.in_flight = Some(InFlight::Issued(LaneWrite {
                expected_revision: row_revision,
                ..write
            }));
            self.revision = row_revision;
            return Ok(());
        }
        self.in_flight = None;
        self.revision = row_revision;
        self.stored = row_values;
        self.live = row_values;
        self.state = LaneState::IntegrityFrozen;
        Ok(())
    }

    /// Another lane of the actor found its fences stale: nothing more is written for this item
    /// either (§5.2). A write in flight is abandoned; the next owner reads the row.
    pub fn lose_authority(&mut self) {
        if matches!(
            self.state,
            LaneState::Running | LaneState::Stopping | LaneState::Stopped
        ) {
            self.in_flight = None;
            self.state = LaneState::LostAuthority;
        }
    }

    fn committed(&mut self, write: LaneWrite) {
        self.in_flight = None;
        self.revision = write.expected_revision.saturating_add(1);
        match write.kind {
            LaneWriteKind::Checkpoint { values } => {
                self.stored = values;
                // The stored values are the snapshot taken at issue, so the next interval runs
                // from then, however long the write took or was retried.
                self.last_checkpoint_ms = self.issued_ms;
            }
            LaneWriteKind::Expire { .. } => {
                self.pending_expiry = None;
                self.state = LaneState::Expired;
            }
        }
    }
}

/// The live timed items of one actor (§5.1), at most `TIMEDITEM0B-RL-04`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActorTimedLanes {
    lanes: BTreeMap<[u8; 16], TimedItemLane>,
}

impl ActorTimedLanes {
    pub fn insert(
        &mut self,
        item_instance_id: [u8; 16],
        lane: TimedItemLane,
    ) -> Result<(), TimedItemError> {
        if self.lanes.len() >= TIMEDITEM0B_RL_04_LIVE_ITEMS_PER_ACTOR
            || self.lanes.contains_key(&item_instance_id)
        {
            return Err(TimedItemError::LiveItemLimit);
        }
        self.lanes.insert(item_instance_id, lane);
        Ok(())
    }

    pub fn get_mut(&mut self, item_instance_id: &[u8; 16]) -> Option<&mut TimedItemLane> {
        self.lanes.get_mut(item_instance_id)
    }

    /// Remove a lane once its item left the slot or session (after its stop).
    pub fn remove(&mut self, item_instance_id: &[u8; 16]) -> Option<TimedItemLane> {
        self.lanes.remove(item_instance_id)
    }

    /// Stop every live item (logout, handoff, death settlement; §6.1).
    pub fn stop_all(&mut self) {
        self.lanes.values_mut().for_each(TimedItemLane::stop);
    }

    /// Every lane is empty: logout, handoff and death settlement may proceed (§6.1).
    #[must_use]
    pub fn all_empty(&self) -> bool {
        self.lanes.values().all(TimedItemLane::is_empty)
    }

    /// The actor's fences are stale: every lane stops writing (§5.2).
    pub fn lose_authority(&mut self) {
        self.lanes
            .values_mut()
            .for_each(TimedItemLane::lose_authority);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.lanes.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lanes.is_empty()
    }
}

/// An ambiguous write whose record is still unknown after `TIMEDITEM0B-RL-02` from its
/// ambiguous outcome is reported as [`AmbiguousLookup::Unknown`] and holds the lane.
#[must_use]
pub const fn ambiguous_bound_exceeded(ambiguous_at_ms: u64, now_ms: u64) -> bool {
    now_ms.saturating_sub(ambiguous_at_ms) > TIMEDITEM0B_RL_02_AMBIGUOUS_BOUND_MS
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    const TX1: [u8; 16] = [1; 16];
    const TX2: [u8; 16] = [2; 16];

    fn values(charges: Option<u32>, remaining_ms: Option<u64>) -> TimedValues {
        TimedValues::new(charges, remaining_ms).expect("valid values")
    }

    fn write_of(step: LaneStep) -> LaneWrite {
        match step {
            LaneStep::Write(write) => write,
            other => panic!("expected a write, got {other:?}"),
        }
    }

    #[test]
    fn values_registered_bounds_at_max_and_max_plus_one() {
        assert!(TimedValues::new(Some(TIMEDITEM0_RL_01_CHARGES_MAX), None).is_ok());
        assert_eq!(
            TimedValues::new(Some(TIMEDITEM0_RL_01_CHARGES_MAX + 1), None),
            Err(TimedItemError::InvalidValues)
        );
        assert!(TimedValues::new(None, Some(TIMEDITEM0_RL_02_REMAINING_MS_MAX)).is_ok());
        assert_eq!(
            TimedValues::new(None, Some(TIMEDITEM0_RL_02_REMAINING_MS_MAX + 1)),
            Err(TimedItemError::InvalidValues)
        );
        assert_eq!(
            TimedValues::new(Some(0), None),
            Err(TimedItemError::InvalidValues)
        );
        assert_eq!(
            TimedValues::new(None, None),
            Err(TimedItemError::InvalidValues)
        );
    }

    #[test]
    fn spending_a_charge_writes_nothing_until_the_next_checkpoint() {
        let mut lane = TimedItemLane::live_from_row(0, values(Some(5), None), 0);
        assert!(lane.spend_charge());
        assert_eq!(lane.next_step(1_000, TX1), LaneStep::Idle);
        let write = write_of(lane.next_step(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS, TX1));
        assert_eq!(write.expected_revision, 0);
        assert_eq!(
            write.kind,
            LaneWriteKind::Checkpoint {
                values: values(Some(4), None)
            }
        );
    }

    #[test]
    fn unchanged_values_write_nothing() {
        let mut lane = TimedItemLane::live_from_row(3, values(None, Some(10_000)), 0);
        assert_eq!(lane.next_step(120_000, TX1), LaneStep::Idle);
        lane.stop();
        assert_eq!(lane.next_step(120_000, TX1), LaneStep::Stopped);
        assert!(!lane.is_active());
    }

    #[test]
    fn checkpoint_cadence_is_at_most_rl_01() {
        let mut lane = TimedItemLane::live_from_row(0, values(None, Some(600_000)), 0);
        lane.advance(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS - 1);
        assert_eq!(
            lane.next_step(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS - 1, TX1),
            LaneStep::Idle
        );
        lane.advance(1);
        let write = write_of(lane.next_step(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS, TX1));
        assert_eq!(
            write.kind,
            LaneWriteKind::Checkpoint {
                values: values(None, Some(540_000))
            }
        );
    }

    #[test]
    fn the_next_interval_runs_from_the_snapshot_not_the_commit() {
        let mut lane = TimedItemLane::live_from_row(0, values(None, Some(600_000)), 0);
        lane.advance(60_000);
        let write = write_of(lane.next_step(60_000, TX1));
        // Retried for 50 s while the clock keeps running.
        lane.on_outcome(LaneWriteOutcome::NotCommitted)
            .expect("in flight");
        assert_eq!(lane.next_step(80_000, TX2), LaneStep::Write(write));
        lane.advance(50_000);
        lane.on_outcome(LaneWriteOutcome::Committed)
            .expect("in flight");
        // The usage after the snapshot is due one interval after the snapshot (120 s), not
        // one interval after the commit (170 s).
        assert_eq!(lane.next_step(119_999, TX2), LaneStep::Idle);
        let next = write_of(lane.next_step(120_000, TX2));
        assert_eq!(
            next.kind,
            LaneWriteKind::Checkpoint {
                values: values(None, Some(490_000))
            }
        );
    }

    #[test]
    fn a_second_write_in_flight_is_impossible() {
        let mut lane = TimedItemLane::live_from_row(0, values(Some(2), None), 0);
        lane.spend_charge();
        lane.stop();
        let first = write_of(lane.next_step(0, TX1));
        // RL-05 max = 1: asking again returns the same write in flight, never a second one.
        assert_eq!(lane.next_step(0, TX2), LaneStep::Write(first));
        assert!(!lane.is_empty());
        assert_eq!(TIMEDITEM0B_RL_05_WRITES_IN_FLIGHT_PER_LANE, 1);
    }

    #[test]
    fn expiry_waits_for_the_checkpoint_in_flight_and_uses_its_revision() {
        let mut lane = TimedItemLane::live_from_row(4, values(Some(2), None), 0);
        lane.spend_charge();
        let checkpoint = write_of(lane.next_step(60_000, TX1));
        assert_eq!(checkpoint.expected_revision, 4);
        // The last charge is spent while the checkpoint is in flight.
        assert!(lane.spend_charge());
        assert!(!lane.is_active());
        assert_eq!(lane.next_step(60_001, TX2), LaneStep::Write(checkpoint));
        lane.on_outcome(LaneWriteOutcome::Committed)
            .expect("in flight");
        let expiry = write_of(lane.next_step(60_003, TX2));
        assert_eq!(expiry.expected_revision, 5);
        assert_eq!(
            expiry.kind,
            LaneWriteKind::Expire {
                reason: ExpireReason::ChargesExhausted
            }
        );
        lane.on_outcome(LaneWriteOutcome::Committed)
            .expect("in flight");
        assert_eq!(lane.next_step(60_005, TX1), LaneStep::Expired);
        assert!(!lane.is_active());
    }

    #[test]
    fn crash_after_checkpoint_reloads_the_checkpointed_value() {
        // The checkpoint committed at revision 1 with one charge; the live zero was lost with
        // the crash. The new owner starts from the row (one interval in the player's favour).
        let lane = TimedItemLane::live_from_row(1, values(Some(1), None), 0);
        assert!(lane.is_active());
        assert_eq!(lane.live_values().charges(), Some(1));
    }

    #[test]
    fn time_reaching_zero_queues_the_expiry() {
        let mut lane = TimedItemLane::live_from_row(2, values(None, Some(500)), 0);
        lane.advance(500);
        assert!(!lane.is_active());
        let expiry = write_of(lane.next_step(500, TX1));
        assert_eq!(
            expiry.kind,
            LaneWriteKind::Expire {
                reason: ExpireReason::TimeExhausted
            }
        );
        assert_eq!(expiry.expected_revision, 2);
    }

    #[test]
    fn not_committed_retries_the_same_write() {
        let mut lane = TimedItemLane::live_from_row(0, values(Some(3), None), 0);
        lane.spend_charge();
        lane.stop();
        let write = write_of(lane.next_step(0, TX1));
        lane.on_outcome(LaneWriteOutcome::NotCommitted)
            .expect("in flight");
        assert_eq!(lane.next_step(2, TX2), LaneStep::Write(write));
    }

    #[test]
    fn an_ambiguous_write_found_by_its_record_is_not_retried() {
        let mut lane = TimedItemLane::live_from_row(0, values(Some(3), None), 0);
        lane.spend_charge();
        lane.stop();
        let write = write_of(lane.next_step(0, TX1));
        lane.on_outcome(LaneWriteOutcome::Ambiguous)
            .expect("in flight");
        assert_eq!(lane.next_step(2, TX2), LaneStep::LookUpRecord(write));
        lane.on_lookup(AmbiguousLookup::FoundSameTransaction)
            .expect("awaiting lookup");
        assert_eq!(lane.revision(), 1);
        assert_eq!(lane.next_step(4, TX2), LaneStep::Stopped);
    }

    #[test]
    fn an_unresolved_ambiguous_write_holds_the_lane() {
        let mut lane = TimedItemLane::live_from_row(0, values(Some(3), None), 0);
        lane.spend_charge();
        let write = write_of(lane.next_step(60_000, TX1));
        lane.on_outcome(LaneWriteOutcome::Ambiguous)
            .expect("in flight");
        lane.on_lookup(AmbiguousLookup::Unknown)
            .expect("awaiting lookup");
        assert!(!lane.is_active());
        assert!(!lane.spend_charge());
        assert_eq!(lane.next_step(62_002, TX2), LaneStep::HeldForReconciliation);
        lane.on_lookup(AmbiguousLookup::NotFoundAtExpected)
            .expect("held");
        assert_eq!(lane.next_step(70_001, TX2), LaneStep::Write(write));
    }

    #[test]
    fn stale_fences_write_nothing_more() {
        let mut lane = TimedItemLane::live_from_row(0, values(Some(3), None), 0);
        lane.spend_charge();
        lane.stop();
        let _ = lane.next_step(0, TX1);
        lane.on_unexpected_revision(false, 7, values(Some(2), None), true)
            .expect("in flight");
        assert_eq!(lane.next_step(1, TX2), LaneStep::LostAuthority);
        assert!(lane.is_empty());
    }

    #[test]
    fn an_expiry_on_an_integrity_fault_is_retried_once_at_the_current_revision() {
        let mut lane = TimedItemLane::live_from_row(2, values(None, Some(10)), 0);
        lane.advance(10);
        let expiry = write_of(lane.next_step(10, TX1));
        lane.on_unexpected_revision(true, 3, values(None, Some(10)), true)
            .expect("in flight");
        let retried = write_of(lane.next_step(11, TX2));
        assert_eq!(retried.expected_revision, 3);
        assert_eq!(retried.transaction_id, expiry.transaction_id);
        assert_eq!(retried.kind, expiry.kind);
        // A second fault freezes the item instead.
        lane.on_unexpected_revision(true, 4, values(None, Some(10)), true)
            .expect("in flight");
        assert_eq!(lane.next_step(12, TX2), LaneStep::IntegrityFrozen);
    }

    #[test]
    fn a_checkpoint_on_an_integrity_fault_freezes_the_item_at_the_row() {
        let mut lane = TimedItemLane::live_from_row(0, values(Some(9), None), 0);
        lane.spend_charge();
        let _ = lane.next_step(60_000, TX1);
        lane.on_unexpected_revision(true, 5, values(Some(7), None), true)
            .expect("in flight");
        assert_eq!(lane.next_step(60_001, TX2), LaneStep::IntegrityFrozen);
        assert_eq!(lane.live_values(), values(Some(7), None));
        assert!(!lane.is_active());
    }

    #[test]
    fn stop_freezes_the_clock_and_stores_the_live_value() {
        // Unequipping 59 s after a checkpoint stores the live value, so re-equipping never
        // adds time.
        let mut lane = TimedItemLane::live_from_row(1, values(None, Some(100_000)), 0);
        lane.advance(59_000);
        lane.stop();
        lane.advance(30_000);
        let write = write_of(lane.next_step(59_000, TX1));
        assert_eq!(
            write.kind,
            LaneWriteKind::Checkpoint {
                values: values(None, Some(41_000))
            }
        );
        lane.on_outcome(LaneWriteOutcome::Committed)
            .expect("in flight");
        assert_eq!(lane.next_step(59_002, TX2), LaneStep::Stopped);
        let relived = TimedItemLane::live_from_row(2, values(None, Some(41_000)), 59_003);
        assert_eq!(relived.live_values().remaining_ms(), Some(41_000));
    }

    #[test]
    fn live_items_per_actor_at_max_and_max_plus_one() {
        let mut lanes = ActorTimedLanes::default();
        for index in 0..TIMEDITEM0B_RL_04_LIVE_ITEMS_PER_ACTOR {
            let id = [u8::try_from(index).expect("small"); 16];
            lanes
                .insert(
                    id,
                    TimedItemLane::live_from_row(0, values(Some(1), None), 0),
                )
                .expect("within RL-04");
        }
        assert_eq!(lanes.len(), TIMEDITEM0B_RL_04_LIVE_ITEMS_PER_ACTOR);
        assert_eq!(
            lanes.insert(
                [0xff; 16],
                TimedItemLane::live_from_row(0, values(Some(1), None), 0)
            ),
            Err(TimedItemError::LiveItemLimit)
        );
    }

    #[test]
    fn logout_waits_until_every_lane_is_empty() {
        let mut lanes = ActorTimedLanes::default();
        let mut ring = TimedItemLane::live_from_row(0, values(None, Some(1_000)), 0);
        ring.advance(10);
        lanes.insert([1; 16], ring).expect("within RL-04");
        lanes
            .insert(
                [2; 16],
                TimedItemLane::live_from_row(0, values(Some(1), None), 0),
            )
            .expect("within RL-04");
        lanes.stop_all();
        // Stopped but not yet issued: the final checkpoints are still owed.
        assert!(!lanes.all_empty());
        let lane = lanes.get_mut(&[1; 16]).expect("present");
        let _ = lane.next_step(10, TX1);
        assert!(!lanes.all_empty());
        lanes
            .get_mut(&[1; 16])
            .expect("present")
            .on_outcome(LaneWriteOutcome::Committed)
            .expect("in flight");
        // The ring's stop committed but has not reported Stopped yet; the unchanged lane has
        // not run its step either.
        assert!(!lanes.all_empty());
        assert_eq!(
            lanes.get_mut(&[1; 16]).expect("present").next_step(12, TX2),
            LaneStep::Stopped
        );
        assert_eq!(
            lanes.get_mut(&[2; 16]).expect("present").next_step(12, TX2),
            LaneStep::Stopped
        );
        assert!(lanes.all_empty());
    }

    #[test]
    fn a_pending_expiry_keeps_the_lane_nonempty_until_it_commits() {
        let mut lane = TimedItemLane::live_from_row(0, values(Some(1), None), 0);
        assert!(lane.spend_charge());
        assert!(!lane.is_empty());
        lane.stop();
        assert!(!lane.is_empty());
        let expiry = write_of(lane.next_step(1, TX1));
        assert!(matches!(expiry.kind, LaneWriteKind::Expire { .. }));
        lane.on_outcome(LaneWriteOutcome::Committed)
            .expect("in flight");
        assert!(lane.is_empty());
        assert_eq!(lane.next_step(3, TX2), LaneStep::Expired);
    }

    #[test]
    fn ambiguous_bound_at_max_and_max_plus_one() {
        assert!(!ambiguous_bound_exceeded(
            1_000,
            1_000 + TIMEDITEM0B_RL_02_AMBIGUOUS_BOUND_MS
        ));
        assert!(ambiguous_bound_exceeded(
            1_000,
            1_000 + TIMEDITEM0B_RL_02_AMBIGUOUS_BOUND_MS + 1
        ));
    }
}
