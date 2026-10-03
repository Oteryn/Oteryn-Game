//! The hosting runtime's timed lanes of one actor (TIMED-RT-1b; decision
//! `TIMEDITEM0B-RUNTIME-CHARGES-AND-DURATION-V1` §5.1-§5.3, §6.1).
//!
//! A [`TimedItemHost`] makes an actor's timed items live: the items already in their slot at
//! login, respawn or arrival (§5.1 places 1 and 2) and the exercise binding (place 3, an API for
//! EXERCISE-1). It runs their clocks, hands each due [`LaneWrite`] to the caller exactly once
//! (`TIMEDITEM0B-RL-05`), holds a lane whose ambiguous write stays unknown past
//! `TIMEDITEM0B-RL-02`, and tells logout, channel transfer and death settlement when every lane
//! is empty. Storage is not touched here: the caller runs each [`HostWrite`] through
//! `durability::item_timed_state` and reports the outcome back.
//!
//! [`resolve_timed_definition`] is the narrow content read: an item is timed by its
//! `temporal` active-time budget or its `charges` count, and its `temporal.decay_target` is the
//! expiry target. A durable absolute deadline is RT-1c's and is not hosted.

use std::collections::{BTreeMap, BTreeSet};

use super::timed_item::{
    ActorTimedLanes, AmbiguousLookup, LaneStep, LaneWrite, LaneWriteOutcome,
    TIMEDITEM0B_RL_04_LIVE_ITEMS_PER_ACTOR, TimedItemError, TimedItemLane, TimedValues,
    ambiguous_bound_exceeded,
};
use crate::content::{
    ReferenceItemField, ReferenceItemSemantics, ReferenceItemTarget, ReferenceTemporalMode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimedHostError {
    /// The content of a timed field conflicts, or its values are outside the timed limits.
    InvalidDefinition,
    /// Hosting these items would exceed `TIMEDITEM0B-RL-04`, or one is live already.
    LiveItemLimit,
    /// The exercise binding is taken, or there is none.
    ExerciseBinding,
    /// The item has no lane here.
    NotHosted,
    Lane(TimedItemError),
}

impl From<TimedItemError> for TimedHostError {
    fn from(error: TimedItemError) -> Self {
        Self::Lane(error)
    }
}

/// A timed definition: its full values and expiry target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedItemDefinition {
    pub definition: ReferenceItemTarget,
    pub full: TimedValues,
    /// `None` burns the item at expiry (§8).
    pub decay_target: Option<ReferenceItemTarget>,
}

/// The item's timed definition, or `None` when the hosting runtime does not run it.
pub fn resolve_timed_definition(
    definition: &ReferenceItemTarget,
    semantics: &ReferenceItemSemantics,
) -> Result<Option<TimedItemDefinition>, TimedHostError> {
    use ReferenceItemField::{Conflict, Known};
    let (remaining_ms, decay_target) = match &semantics.temporal {
        Conflict => return Err(TimedHostError::InvalidDefinition),
        Known(temporal) => match (&temporal.consumption_mode, &temporal.duration) {
            (Conflict, _) | (_, Conflict) => return Err(TimedHostError::InvalidDefinition),
            (Known(ReferenceTemporalMode::AuthoritativeActiveTimeBudget), Known(duration)) => {
                let target = match &temporal.decay_target {
                    Conflict => return Err(TimedHostError::InvalidDefinition),
                    Known(target) => Some(target.clone()),
                    _ => None,
                };
                (Some(duration.0), target)
            }
            _ => (None, None),
        },
        _ => (None, None),
    };
    let charges = match &semantics.charges {
        Conflict => return Err(TimedHostError::InvalidDefinition),
        Known(charges) => match charges.count {
            Conflict => return Err(TimedHostError::InvalidDefinition),
            Known(count) => Some(count),
            _ => None,
        },
        _ => None,
    };
    if charges.is_none() && remaining_ms.is_none() {
        return Ok(None);
    }
    Ok(Some(TimedItemDefinition {
        definition: definition.clone(),
        full: TimedValues::new(charges, remaining_ms)
            .map_err(|_| TimedHostError::InvalidDefinition)?,
        decay_target,
    }))
}

/// One item to make live: its definition and its row (`None` when it has none).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedItem {
    pub item_instance_id: [u8; 16],
    pub definition: TimedItemDefinition,
    /// The row's revision and values; values `None` is a spent row, which is never live.
    pub row: Option<(u64, Option<TimedValues>)>,
}

/// One write for the caller to run, keyed by the lane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostWrite {
    pub item_instance_id: [u8; 16],
    pub definition: TimedItemDefinition,
    pub write: LaneWrite,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Hosted {
    definition: TimedItemDefinition,
    /// The lane's write was handed to the caller and its outcome is not known yet.
    dispatched: bool,
    /// When the write in flight turned ambiguous.
    ambiguous_at_ms: Option<u64>,
}

/// The live timed items of one actor in its hosting runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedItemHost {
    lanes: ActorTimedLanes,
    hosted: BTreeMap<[u8; 16], Hosted>,
    exercise: Option<[u8; 16]>,
    checkpoint_interval_ms: u64,
}

impl TimedItemHost {
    /// A host that checkpoints a changed live item every `checkpoint_interval_ms`
    /// (1..=`TIMEDITEM0B-RL-01`).
    pub fn new(checkpoint_interval_ms: u64) -> Result<Self, TimedHostError> {
        // The lane owns the bound; probe it once so a bad cadence is refused before any lane.
        TimedItemLane::live_from_row(0, TimedValues::new(Some(1), None)?, 0)
            .with_checkpoint_interval(checkpoint_interval_ms)?;
        Ok(Self {
            lanes: ActorTimedLanes::default(),
            hosted: BTreeMap::new(),
            exercise: None,
            checkpoint_interval_ms,
        })
    }

    /// §5.1 places 1 and 2: the items in their slot at login, respawn or arrival become live.
    /// Spent rows are skipped. All of them are hosted or none: the count is checked against
    /// `TIMEDITEM0B-RL-04` before any lane opens.
    pub fn host_live(&mut self, items: Vec<HostedItem>, now_ms: u64) -> Result<(), TimedHostError> {
        let live: Vec<_> = items
            .into_iter()
            .filter(|item| !matches!(item.row, Some((_, None))))
            .collect();
        let ids: BTreeSet<_> = live.iter().map(|item| item.item_instance_id).collect();
        if ids.len() != live.len()
            || ids.iter().any(|id| self.hosted.contains_key(id))
            || self.lanes.len() + live.len() > TIMEDITEM0B_RL_04_LIVE_ITEMS_PER_ACTOR
        {
            return Err(TimedHostError::LiveItemLimit);
        }
        for item in live {
            self.open(item, now_ms)?;
        }
        Ok(())
    }

    /// §5.1 place 3: the exercise weapon becomes live (EXERCISE-1 calls this).
    pub fn bind_exercise(&mut self, item: HostedItem, now_ms: u64) -> Result<(), TimedHostError> {
        if self.exercise.is_some() || matches!(item.row, Some((_, None))) {
            return Err(TimedHostError::ExerciseBinding);
        }
        if self.hosted.contains_key(&item.item_instance_id)
            || self.lanes.len() >= TIMEDITEM0B_RL_04_LIVE_ITEMS_PER_ACTOR
        {
            return Err(TimedHostError::LiveItemLimit);
        }
        let id = item.item_instance_id;
        self.open(item, now_ms)?;
        self.exercise = Some(id);
        Ok(())
    }

    /// Spend one charge of the exercise weapon (§7); `false` when it has none to spend.
    pub fn spend_exercise_charge(&mut self) -> bool {
        self.exercise
            .and_then(|id| self.lanes.get_mut(&id))
            .is_some_and(TimedItemLane::spend_charge)
    }

    /// The exercise ends: its lane stops (§5.3) and is released once empty.
    pub fn unbind_exercise(&mut self) -> Result<(), TimedHostError> {
        let id = self.exercise.ok_or(TimedHostError::ExerciseBinding)?;
        self.lanes
            .get_mut(&id)
            .ok_or(TimedHostError::NotHosted)?
            .stop();
        Ok(())
    }

    /// Run every live clock by `elapsed_ms`.
    pub fn advance(&mut self, elapsed_ms: u64) {
        for id in self.hosted.keys() {
            if let Some(lane) = self.lanes.get_mut(id) {
                lane.advance(elapsed_ms);
            }
        }
    }

    /// The writes now due, each handed out once until its outcome is reported
    /// (`TIMEDITEM0B-RL-05`). Lanes that stopped or expired are released here.
    pub fn due_writes(
        &mut self,
        now_ms: u64,
        mut transaction_id: impl FnMut() -> [u8; 16],
    ) -> Vec<HostWrite> {
        let mut writes = Vec::new();
        let mut released = Vec::new();
        for (id, hosted) in &mut self.hosted {
            let Some(lane) = self.lanes.get_mut(id) else {
                continue;
            };
            match lane.next_step(now_ms, transaction_id()) {
                LaneStep::Write(write) if !hosted.dispatched => {
                    hosted.dispatched = true;
                    writes.push(HostWrite {
                        item_instance_id: *id,
                        definition: hosted.definition.clone(),
                        write,
                    });
                }
                LaneStep::Stopped | LaneStep::Expired | LaneStep::LostAuthority => {
                    released.push(*id);
                }
                _ => {}
            }
        }
        for id in released {
            self.release(&id);
        }
        writes
    }

    /// Report the outcome of an item's dispatched write.
    pub fn on_outcome(
        &mut self,
        item_instance_id: &[u8; 16],
        outcome: LaneWriteOutcome,
        now_ms: u64,
    ) -> Result<(), TimedHostError> {
        let (lane, hosted) = self.lane(item_instance_id)?;
        lane.on_outcome(outcome)?;
        hosted.dispatched = false;
        hosted.ambiguous_at_ms = (outcome == LaneWriteOutcome::Ambiguous).then_some(now_ms);
        Ok(())
    }

    /// Report the record lookup of an ambiguous write: `None` while it is still unknown. Unknown
    /// past `TIMEDITEM0B-RL-02` from the ambiguous outcome holds the lane for reconciliation.
    pub fn on_lookup(
        &mut self,
        item_instance_id: &[u8; 16],
        lookup: Option<AmbiguousLookup>,
        now_ms: u64,
    ) -> Result<(), TimedHostError> {
        let (lane, hosted) = self.lane(item_instance_id)?;
        let ambiguous_at = hosted
            .ambiguous_at_ms
            .ok_or(TimedHostError::Lane(TimedItemError::NoWriteInFlight))?;
        let lookup = match lookup {
            Some(lookup) => lookup,
            None if ambiguous_bound_exceeded(ambiguous_at, now_ms) => AmbiguousLookup::Unknown,
            None => return Ok(()),
        };
        lane.on_lookup(lookup)?;
        if lookup != AmbiguousLookup::Unknown {
            hosted.ambiguous_at_ms = None;
        }
        Ok(())
    }

    /// The write found another revision (§5.2).
    pub fn on_unexpected_revision(
        &mut self,
        item_instance_id: &[u8; 16],
        fences_current: bool,
        row_revision: u64,
        row_values: TimedValues,
        same_definition: bool,
    ) -> Result<(), TimedHostError> {
        let (lane, hosted) = self.lane(item_instance_id)?;
        lane.on_unexpected_revision(fences_current, row_revision, row_values, same_definition)?;
        hosted.dispatched = false;
        Ok(())
    }

    /// Logout, channel transfer or death settlement (§6.1): every lane stops.
    pub fn stop_all(&mut self) {
        self.lanes.stop_all();
    }

    /// Every lane is empty: logout, channel transfer and death settlement may proceed.
    #[must_use]
    pub fn all_empty(&self) -> bool {
        self.lanes.all_empty()
    }

    #[must_use]
    pub fn live_count(&self) -> usize {
        self.lanes.len()
    }

    fn open(&mut self, item: HostedItem, now_ms: u64) -> Result<(), TimedHostError> {
        let (revision, values) = match item.row {
            Some((revision, Some(values))) => (revision, values),
            Some((_, None)) => return Err(TimedHostError::NotHosted),
            None => (0, item.definition.full),
        };
        let lane = TimedItemLane::live_from_row(revision, values, now_ms)
            .with_checkpoint_interval(self.checkpoint_interval_ms)?;
        self.lanes.insert(item.item_instance_id, lane)?;
        self.hosted.insert(
            item.item_instance_id,
            Hosted {
                definition: item.definition,
                dispatched: false,
                ambiguous_at_ms: None,
            },
        );
        Ok(())
    }

    fn release(&mut self, id: &[u8; 16]) {
        self.lanes.remove(id);
        self.hosted.remove(id);
        if self.exercise == Some(*id) {
            self.exercise = None;
        }
    }

    fn lane(&mut self, id: &[u8; 16]) -> Result<(&mut TimedItemLane, &mut Hosted), TimedHostError> {
        let hosted = self.hosted.get_mut(id).ok_or(TimedHostError::NotHosted)?;
        let lane = self.lanes.get_mut(id).ok_or(TimedHostError::NotHosted)?;
        Ok((lane, hosted))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::content::{ReferenceItemCharges, ReferenceItemTemporal, ReferenceMilliseconds};
    use crate::domain::timed_item::{
        ExpireReason, LaneWriteKind, TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS,
        TIMEDITEM0B_RL_02_AMBIGUOUS_BOUND_MS,
    };

    const RING_MS: u64 = 1_200_000;

    fn target(key: &str) -> ReferenceItemTarget {
        ReferenceItemTarget::new(key, "rev-1").expect("target")
    }

    fn ring_semantics(decay: Option<&str>) -> ReferenceItemSemantics {
        ReferenceItemSemantics {
            temporal: ReferenceItemField::Known(ReferenceItemTemporal {
                consumption_mode: ReferenceItemField::Known(
                    ReferenceTemporalMode::AuthoritativeActiveTimeBudget,
                ),
                duration: ReferenceItemField::Known(ReferenceMilliseconds(RING_MS)),
                stop_duration: ReferenceItemField::Known(true),
                decay_target: decay.map_or(ReferenceItemField::NotApplicable, |key| {
                    ReferenceItemField::Known(target(key))
                }),
            }),
            ..ReferenceItemSemantics::default()
        }
    }

    fn ring() -> TimedItemDefinition {
        resolve_timed_definition(
            &target("oteryn:item.tibia.i3052"),
            &ring_semantics(Some("oteryn:item.tibia.i3999")),
        )
        .expect("resolve")
        .expect("timed")
    }

    fn item(seed: u8) -> HostedItem {
        HostedItem {
            item_instance_id: [seed; 16],
            definition: ring(),
            row: None,
        }
    }

    fn charged(seed: u8, charges: u32) -> HostedItem {
        let semantics = ReferenceItemSemantics {
            charges: ReferenceItemField::Known(ReferenceItemCharges {
                count: ReferenceItemField::Known(charges),
            }),
            ..ReferenceItemSemantics::default()
        };
        HostedItem {
            item_instance_id: [seed; 16],
            definition: resolve_timed_definition(&target("oteryn:item.tibia.i28552"), &semantics)
                .expect("resolve")
                .expect("timed"),
            row: None,
        }
    }

    fn host() -> TimedItemHost {
        TimedItemHost::new(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS).expect("host")
    }

    fn tx() -> [u8; 16] {
        [7; 16]
    }

    #[test]
    fn the_resolver_reads_the_active_time_budget_charges_and_decay_target() {
        let ring = ring();
        assert_eq!(ring.full.remaining_ms(), Some(RING_MS));
        assert_eq!(ring.full.charges(), None);
        assert_eq!(ring.decay_target, Some(target("oteryn:item.tibia.i3999")));
        // No timed field, a durable deadline (RT-1c) and a conflict.
        let plain = target("oteryn:item.tibia.i2854");
        assert_eq!(
            resolve_timed_definition(&plain, &ReferenceItemSemantics::default()),
            Ok(None)
        );
        let mut deadline = ring_semantics(None);
        if let ReferenceItemField::Known(temporal) = &mut deadline.temporal {
            temporal.consumption_mode =
                ReferenceItemField::Known(ReferenceTemporalMode::DurableAbsoluteDeadline);
        }
        assert_eq!(resolve_timed_definition(&plain, &deadline), Ok(None));
        let conflict = ReferenceItemSemantics {
            charges: ReferenceItemField::Conflict,
            ..ReferenceItemSemantics::default()
        };
        assert_eq!(
            resolve_timed_definition(&plain, &conflict),
            Err(TimedHostError::InvalidDefinition)
        );
        // A charges count of 0 is not a timed definition's full value.
        assert_eq!(
            resolve_timed_definition(
                &plain,
                &ReferenceItemSemantics {
                    charges: ReferenceItemField::Known(ReferenceItemCharges {
                        count: ReferenceItemField::Known(0),
                    }),
                    ..ReferenceItemSemantics::default()
                }
            ),
            Err(TimedHostError::InvalidDefinition)
        );
    }

    #[test]
    fn rl_01_host_cadence_at_max_and_max_plus_one() {
        assert!(TimedItemHost::new(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS).is_ok());
        assert_eq!(
            TimedItemHost::new(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS + 1),
            Err(TimedHostError::Lane(TimedItemError::InvalidValues))
        );
        // An item hosted at login checkpoints within 60 s, so a crash returns at most that.
        let mut host = host();
        host.host_live(vec![item(1)], 0).expect("login");
        host.advance(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS - 1);
        assert!(
            host.due_writes(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS - 1, tx)
                .is_empty()
        );
        host.advance(1);
        let writes = host.due_writes(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS, tx);
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].write.expected_revision, 0);
        assert!(matches!(
            writes[0].write.kind,
            LaneWriteKind::Checkpoint { values } if values.remaining_ms()
                == Some(RING_MS - TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS)
        ));
    }

    #[test]
    fn rl_02_ambiguous_outcome_at_max_and_max_plus_one() {
        for (elapsed, held) in [
            (TIMEDITEM0B_RL_02_AMBIGUOUS_BOUND_MS, false),
            (TIMEDITEM0B_RL_02_AMBIGUOUS_BOUND_MS + 1, true),
        ] {
            let mut host = host();
            host.host_live(vec![item(1)], 0).expect("login");
            host.advance(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS);
            let now = TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS;
            assert_eq!(host.due_writes(now, tx).len(), 1);
            host.on_outcome(&[1; 16], LaneWriteOutcome::Ambiguous, now)
                .expect("ambiguous");
            host.on_lookup(&[1; 16], None, now + elapsed)
                .expect("lookup");
            if held {
                // Held: the item is inactive and nothing more is written.
                host.advance(1);
                assert!(host.due_writes(now + elapsed, tx).is_empty());
                assert!(!host.all_empty());
            } else {
                // Still open: the record is found and the lane runs on.
                host.on_lookup(
                    &[1; 16],
                    Some(AmbiguousLookup::FoundSameTransaction),
                    now + elapsed,
                )
                .expect("found");
                assert!(host.all_empty());
            }
        }
    }

    #[test]
    fn rl_04_live_items_at_max_and_max_plus_one() {
        let mut host = host();
        let slots: Vec<_> = (1..=10).map(item).collect();
        host.host_live(slots, 0).expect("ten slots");
        host.bind_exercise(charged(11, 500), 0).expect("exercise");
        assert_eq!(host.live_count(), TIMEDITEM0B_RL_04_LIVE_ITEMS_PER_ACTOR);
        // A 12th is refused before any lane opens, at login and at the exercise binding.
        let mut full = host.clone();
        assert_eq!(
            full.host_live(vec![item(12)], 0),
            Err(TimedHostError::LiveItemLimit)
        );
        let mut fresh = TimedItemHost::new(1_000).expect("host");
        assert_eq!(
            fresh.host_live((1..=12).map(item).collect(), 0),
            Err(TimedHostError::LiveItemLimit)
        );
        assert_eq!(fresh.live_count(), 0);
        // A second exercise binding is refused; spent rows are never live.
        let mut one = TimedItemHost::new(1_000).expect("host");
        one.bind_exercise(charged(1, 5), 0).expect("exercise");
        assert_eq!(
            one.bind_exercise(charged(2, 5), 0),
            Err(TimedHostError::ExerciseBinding)
        );
        let spent = HostedItem {
            row: Some((3, None)),
            ..item(3)
        };
        one.host_live(vec![spent], 0).expect("skipped");
        assert_eq!(one.live_count(), 1);
    }

    #[test]
    fn rl_05_a_write_in_flight_is_never_sent_twice() {
        let mut host = host();
        host.host_live(vec![item(1)], 0).expect("login");
        host.advance(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS);
        let now = TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS;
        let checkpoint = host.due_writes(now, tx);
        assert_eq!(checkpoint.len(), 1);
        // The clock runs out while the checkpoint is in flight: the expiry waits for it.
        host.advance(RING_MS);
        assert!(host.due_writes(now + RING_MS, tx).is_empty());
        host.on_outcome(&[1; 16], LaneWriteOutcome::Committed, now + RING_MS)
            .expect("committed");
        let expiry = host.due_writes(now + RING_MS, tx);
        assert_eq!(expiry.len(), 1);
        assert_eq!(expiry[0].write.expected_revision, 1);
        assert_eq!(
            expiry[0].write.kind,
            LaneWriteKind::Expire {
                reason: ExpireReason::TimeExhausted
            }
        );
        // A known-not-committed outcome hands the same write out again, once.
        host.on_outcome(&[1; 16], LaneWriteOutcome::NotCommitted, now + RING_MS)
            .expect("retry");
        let retry = host.due_writes(now + RING_MS, tx);
        assert_eq!(retry.len(), 1);
        assert_eq!(retry[0].write, expiry[0].write);
        assert!(host.due_writes(now + RING_MS, tx).is_empty());
        host.on_outcome(&[1; 16], LaneWriteOutcome::Committed, now + RING_MS)
            .expect("expired");
        // The expired lane is released.
        assert!(host.due_writes(now + RING_MS, tx).is_empty());
        assert_eq!(host.live_count(), 0);
    }

    #[test]
    fn the_exercise_binding_spends_charges_and_expires() {
        let mut host = host();
        assert!(!host.spend_exercise_charge());
        host.bind_exercise(charged(1, 2), 0).expect("exercise");
        assert!(host.spend_exercise_charge());
        assert!(host.spend_exercise_charge());
        assert!(!host.spend_exercise_charge());
        let expiry = host.due_writes(0, tx);
        assert_eq!(
            expiry[0].write.kind,
            LaneWriteKind::Expire {
                reason: ExpireReason::ChargesExhausted
            }
        );
        assert_eq!(expiry[0].definition.decay_target, None);
        host.on_outcome(&[1; 16], LaneWriteOutcome::Committed, 0)
            .expect("expired");
        assert!(host.due_writes(0, tx).is_empty());
        // The binding is released with its lane.
        host.bind_exercise(charged(2, 2), 0).expect("rebind");
        assert!(host.spend_exercise_charge());
        host.unbind_exercise().expect("unbind");
        assert_eq!(host.due_writes(1, tx).len(), 1);
    }

    #[test]
    fn logout_waits_for_every_lane_and_stores_the_live_values() {
        let mut host = host();
        host.host_live(
            vec![
                item(1),
                HostedItem {
                    row: Some((4, Some(TimedValues::new(None, Some(5_000)).expect("v")))),
                    ..item(2)
                },
            ],
            0,
        )
        .expect("login");
        host.advance(1_000);
        host.stop_all();
        assert!(!host.all_empty());
        let stops = host.due_writes(1_000, tx);
        assert_eq!(stops.len(), 2);
        assert_eq!(
            stops[1].write.kind,
            LaneWriteKind::Checkpoint {
                values: TimedValues::new(None, Some(4_000)).expect("v")
            }
        );
        assert_eq!(stops[1].write.expected_revision, 4);
        host.on_outcome(&[1; 16], LaneWriteOutcome::Committed, 1_000)
            .expect("one");
        assert!(!host.all_empty());
        host.on_outcome(&[2; 16], LaneWriteOutcome::Committed, 1_000)
            .expect("two");
        // The next pass completes both stops and releases the lanes.
        assert!(host.due_writes(1_000, tx).is_empty());
        assert!(host.all_empty());
        assert_eq!(host.live_count(), 0);
    }
}
