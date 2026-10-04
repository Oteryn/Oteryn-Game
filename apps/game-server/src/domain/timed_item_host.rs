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
//! `temporal` active-time budget or its `charges` count, and its expiry target is
//! `transform {trigger: decay}`, else `temporal.decay_target` (§8). A durable absolute deadline is RT-1c's and is not hosted.

use std::collections::{BTreeMap, BTreeSet};

use super::timed_item::{
    ActorTimedLanes, AmbiguousLookup, LaneStep, LaneWrite, LaneWriteOutcome,
    TIMEDITEM0B_RL_04_LIVE_ITEMS_PER_ACTOR, TimedItemError, TimedItemLane, TimedValues,
    ambiguous_bound_exceeded,
};
use crate::content::{
    ReferenceItemField, ReferenceItemSemantics, ReferenceItemTarget, ReferenceTemporalMode,
    ReferenceTransformKind,
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
    let (remaining_ms, temporal_target) = match &semantics.temporal {
        Conflict => return Err(TimedHostError::InvalidDefinition),
        Known(temporal) => {
            let target = match &temporal.decay_target {
                Conflict => return Err(TimedHostError::InvalidDefinition),
                Known(target) => Some(target),
                _ => None,
            };
            match (&temporal.consumption_mode, &temporal.duration) {
                (Conflict, _) | (_, Conflict) => return Err(TimedHostError::InvalidDefinition),
                (Known(ReferenceTemporalMode::AuthoritativeActiveTimeBudget), Known(duration)) => {
                    (Some(duration.0), target)
                }
                _ => (None, target),
            }
        }
        _ => (None, None),
    };
    // §8: `transform {trigger: decay}` takes precedence; both set to different targets conflict.
    let transform_target = match &semantics.use_transform {
        Conflict => return Err(TimedHostError::InvalidDefinition),
        Known(transform) => match transform
            .targets
            .iter()
            .find(|entry| entry.kind == ReferenceTransformKind::Decay)
            .map(|entry| &entry.target)
        {
            Some(Conflict) => return Err(TimedHostError::InvalidDefinition),
            Some(Known(target)) => Some(target),
            _ => None,
        },
        _ => None,
    };
    let decay_target = match (transform_target, temporal_target) {
        (Some(transform), Some(temporal)) if transform != temporal => {
            return Err(TimedHostError::InvalidDefinition);
        }
        (Some(target), _) | (None, Some(target)) => Some(target.clone()),
        (None, None) => None,
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
    /// (`TIMEDITEM0B-RL-05`). Lanes that stopped or expired are released here, except that an
    /// expiry into a timed target keeps the item live from the target's full values (§8):
    /// `timed_target` resolves a decay target's timed definition, `None` when it is not timed.
    pub fn due_writes(
        &mut self,
        now_ms: u64,
        mut transaction_id: impl FnMut() -> [u8; 16],
        mut timed_target: impl FnMut(&ReferenceItemTarget) -> Option<TimedItemDefinition>,
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
                LaneStep::Expired => {
                    let target = hosted.definition.decay_target.clone();
                    match target.and_then(|target| {
                        timed_target(&target).map(|resolved| TimedItemDefinition {
                            definition: target,
                            ..resolved
                        })
                    }) {
                        Some(target) if lane.continue_as_target(target.full, now_ms) => {
                            hosted.definition = target;
                        }
                        _ => released.push(*id),
                    }
                }
                LaneStep::Stopped | LaneStep::LostAuthority => {
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
        if !fences_current {
            // The fences are the actor's, not the item's: no lane of it writes again.
            self.lanes.lose_authority();
            self.hosted
                .values_mut()
                .for_each(|hosted| hosted.dispatched = false);
        }
        Ok(())
    }

    /// Logout, channel transfer or death settlement (§6.1): every lane stops.
    pub fn stop_all(&mut self) {
        self.lanes.stop_all();
    }

    /// Every lane is empty: logout, channel transfer and death settlement may proceed. A
    /// committed expiry counts until [`Self::due_writes`] continued its target or released it.
    #[must_use]
    pub fn all_empty(&self) -> bool {
        self.lanes.all_empty() && !self.lanes.any_expired()
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

    fn untimed(_: &ReferenceItemTarget) -> Option<TimedItemDefinition> {
        None
    }

    const SPARE_MS: u64 = 600_000;

    /// Every decay target resolves as a timed spare ring of `SPARE_MS`.
    fn spare(target: &ReferenceItemTarget) -> Option<TimedItemDefinition> {
        Some(TimedItemDefinition {
            definition: target.clone(),
            full: TimedValues::new(None, Some(SPARE_MS)).expect("v"),
            decay_target: None,
        })
    }

    fn values(remaining_ms: u64) -> TimedValues {
        TimedValues::new(None, Some(remaining_ms)).expect("v")
    }

    /// A host whose ring (row at revision 3, 1 s left) has its expiry in flight at 1 s, with a
    /// second ring live beside it.
    fn expiring() -> TimedItemHost {
        let mut host = host();
        host.host_live(
            vec![
                HostedItem {
                    row: Some((3, Some(values(1_000)))),
                    ..item(1)
                },
                item(2),
            ],
            0,
        )
        .expect("login");
        host.advance(1_000);
        let expiry = host.due_writes(1_000, tx, spare);
        assert_eq!(expiry.len(), 1);
        assert!(matches!(expiry[0].write.kind, LaneWriteKind::Expire { .. }));
        host
    }

    fn commit(host: &mut TimedItemHost, seed: u8) {
        host.on_outcome(&[seed; 16], LaneWriteOutcome::Committed, 0)
            .expect("committed");
    }

    fn write_for(writes: &[HostWrite], seed: u8) -> LaneWrite {
        writes
            .iter()
            .find(|write| write.item_instance_id == [seed; 16])
            .expect("a write for the item")
            .write
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

    fn decay_transform(key: &str) -> ReferenceItemField<crate::content::ReferenceItemUseTransform> {
        ReferenceItemField::Known(crate::content::ReferenceItemUseTransform {
            targets: (1..=10)
                .map(|wire| {
                    let kind = ReferenceTransformKind::from_wire(wire).expect("kind");
                    crate::content::ReferenceTransformTarget {
                        kind,
                        target: if kind == ReferenceTransformKind::Decay {
                            ReferenceItemField::Known(target(key))
                        } else {
                            ReferenceItemField::NotApplicable
                        },
                    }
                })
                .collect(),
        })
    }

    #[test]
    fn the_decay_transform_is_the_expiry_target_of_time_and_charges() {
        let plain = target("oteryn:item.tibia.i28552");
        // A charge-only item with `transform {trigger: decay}` transforms at expiry.
        let charge_only = ReferenceItemSemantics {
            charges: ReferenceItemField::Known(ReferenceItemCharges {
                count: ReferenceItemField::Known(3),
            }),
            use_transform: decay_transform("oteryn:item.tibia.i28553"),
            ..ReferenceItemSemantics::default()
        };
        let resolved = resolve_timed_definition(&plain, &charge_only)
            .expect("resolve")
            .expect("timed");
        assert_eq!(
            resolved.decay_target,
            Some(target("oteryn:item.tibia.i28553"))
        );
        // The same target from both sources resolves; different targets conflict.
        let same = ReferenceItemSemantics {
            use_transform: decay_transform("oteryn:item.tibia.i3999"),
            ..ring_semantics(Some("oteryn:item.tibia.i3999"))
        };
        assert_eq!(
            resolve_timed_definition(&plain, &same)
                .expect("resolve")
                .expect("timed")
                .decay_target,
            Some(target("oteryn:item.tibia.i3999"))
        );
        let different = ReferenceItemSemantics {
            use_transform: decay_transform("oteryn:item.tibia.i3998"),
            ..ring_semantics(Some("oteryn:item.tibia.i3999"))
        };
        assert_eq!(
            resolve_timed_definition(&plain, &different),
            Err(TimedHostError::InvalidDefinition)
        );
        let conflict = ReferenceItemSemantics {
            use_transform: ReferenceItemField::Conflict,
            ..charge_only
        };
        assert_eq!(
            resolve_timed_definition(&plain, &conflict),
            Err(TimedHostError::InvalidDefinition)
        );
    }

    #[test]
    fn stale_fences_on_one_lane_stop_every_lane_of_the_actor() {
        let mut host = host();
        host.host_live(vec![item(1), item(2)], 0).expect("login");
        host.bind_exercise(charged(3, 2), 0).expect("exercise");
        host.advance(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS);
        assert!(host.spend_exercise_charge());
        let now = TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS;
        assert_eq!(host.due_writes(now, tx, untimed).len(), 3);
        let stored = TimedValues::new(None, Some(RING_MS)).expect("v");
        host.on_unexpected_revision(&[1; 16], false, 3, stored, true)
            .expect("stale");
        // No lane writes again: every lane is released and the exercise binding with it.
        host.advance(RING_MS);
        assert!(host.due_writes(now + RING_MS, tx, untimed).is_empty());
        assert_eq!(host.live_count(), 0);
        assert!(host.all_empty());
        assert!(!host.spend_exercise_charge());
    }

    #[test]
    fn stale_fences_stop_a_committed_expiry_from_continuing_as_its_timed_target() {
        let mut host = expiring();
        host.advance(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS);
        let now = 1_000 + TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS;
        assert_eq!(host.due_writes(now, tx, spare).len(), 1);
        commit(&mut host, 1);
        // The other lane finds the fences stale before the expired lane is processed.
        host.on_unexpected_revision(&[2; 16], false, 3, values(RING_MS), true)
            .expect("stale");
        assert!(host.due_writes(now, tx, spare).is_empty());
        assert_eq!(host.live_count(), 0);
        assert!(host.all_empty());
    }

    #[test]
    fn a_stop_before_the_expiry_commits_stops_the_continuing_target() {
        let mut host = expiring();
        host.stop_all();
        let stops = host.due_writes(1_000, tx, spare);
        assert_eq!(write_for(&stops, 2).expected_revision, 0);
        commit(&mut host, 2);
        commit(&mut host, 1);
        // The drain waits for the committed expiry to be continued or released.
        assert!(!host.all_empty());
        // The target continues stopped at its full values: nothing to write, and it is released.
        assert!(host.due_writes(1_000, tx, spare).is_empty());
        assert!(!host.all_empty());
        assert!(host.due_writes(1_000, tx, spare).is_empty());
        assert!(host.all_empty());
        assert_eq!(host.live_count(), 0);
    }

    #[test]
    fn a_stop_after_the_expiry_commits_stops_the_target_at_its_debited_time() {
        let mut host = expiring();
        commit(&mut host, 1);
        // §8: the target is live from the commit; the clock stops with the stop.
        host.advance(500);
        host.stop_all();
        host.advance(700);
        assert!(!host.all_empty());
        // The ring beside it stops too; the target continues stopping at revision 4.
        let first = host.due_writes(2_200, tx, spare);
        assert_eq!(first.len(), 1);
        commit(&mut host, 2);
        let stop = host.due_writes(2_200, tx, spare);
        assert_eq!(
            write_for(&stop, 1),
            LaneWrite {
                kind: LaneWriteKind::Checkpoint {
                    values: values(SPARE_MS - 500)
                },
                expected_revision: 4,
                transaction_id: tx(),
            }
        );
        assert_eq!(
            stop[0].definition.definition,
            target("oteryn:item.tibia.i3999")
        );
        commit(&mut host, 1);
        assert!(host.due_writes(2_200, tx, spare).is_empty());
        assert!(host.all_empty());
        assert_eq!(host.live_count(), 0);
        // A reload hosts the target from its row.
        host.host_live(
            vec![HostedItem {
                item_instance_id: [1; 16],
                definition: spare(&target("oteryn:item.tibia.i3999")).expect("timed"),
                row: Some((5, Some(values(SPARE_MS - 500)))),
            }],
            3_000,
        )
        .expect("reload");
        assert_eq!(host.live_count(), 1);
    }

    #[test]
    fn a_delayed_continuation_debits_the_time_since_the_commit() {
        let mut host = expiring();
        commit(&mut host, 1);
        host.advance(300);
        assert!(host.due_writes(1_300, tx, spare).is_empty());
        host.stop_all();
        let stops = host.due_writes(1_300, tx, spare);
        assert_eq!(
            write_for(&stops, 1).kind,
            LaneWriteKind::Checkpoint {
                values: values(SPARE_MS - 300)
            }
        );

        // Time past the target's full duration expires it at once, at revision 4.
        let mut host = expiring();
        commit(&mut host, 1);
        host.advance(SPARE_MS);
        assert!(
            host.due_writes(1_000 + SPARE_MS, tx, spare)
                .iter()
                .all(|write| write.item_instance_id != [1; 16])
        );
        let expiry = host.due_writes(1_000 + SPARE_MS, tx, spare);
        assert_eq!(
            write_for(&expiry, 1),
            LaneWrite {
                kind: LaneWriteKind::Expire {
                    reason: ExpireReason::TimeExhausted
                },
                expected_revision: 4,
                transaction_id: tx(),
            }
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
            host.due_writes(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS - 1, tx, untimed)
                .is_empty()
        );
        host.advance(1);
        let writes = host.due_writes(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS, tx, untimed);
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
            assert_eq!(host.due_writes(now, tx, untimed).len(), 1);
            host.on_outcome(&[1; 16], LaneWriteOutcome::Ambiguous, now)
                .expect("ambiguous");
            host.on_lookup(&[1; 16], None, now + elapsed)
                .expect("lookup");
            if held {
                // Held: the item is inactive and nothing more is written.
                host.advance(1);
                assert!(host.due_writes(now + elapsed, tx, untimed).is_empty());
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
        let checkpoint = host.due_writes(now, tx, untimed);
        assert_eq!(checkpoint.len(), 1);
        // The clock runs out while the checkpoint is in flight: the expiry waits for it.
        host.advance(RING_MS);
        assert!(host.due_writes(now + RING_MS, tx, untimed).is_empty());
        host.on_outcome(&[1; 16], LaneWriteOutcome::Committed, now + RING_MS)
            .expect("committed");
        let expiry = host.due_writes(now + RING_MS, tx, untimed);
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
        let retry = host.due_writes(now + RING_MS, tx, untimed);
        assert_eq!(retry.len(), 1);
        assert_eq!(retry[0].write, expiry[0].write);
        assert!(host.due_writes(now + RING_MS, tx, untimed).is_empty());
        host.on_outcome(&[1; 16], LaneWriteOutcome::Committed, now + RING_MS)
            .expect("expired");
        // The expired lane is released.
        assert!(host.due_writes(now + RING_MS, tx, untimed).is_empty());
        assert_eq!(host.live_count(), 0);
    }

    #[test]
    fn an_expiry_into_a_timed_target_keeps_the_item_live_at_the_next_revision() {
        const SPARE_MS: u64 = 600_000;
        let spare = |target: &ReferenceItemTarget| {
            (target == &self::target("oteryn:item.tibia.i3999")).then(|| TimedItemDefinition {
                definition: target.clone(),
                full: TimedValues::new(None, Some(SPARE_MS)).expect("v"),
                decay_target: None,
            })
        };
        let mut host = host();
        host.host_live(
            vec![HostedItem {
                row: Some((3, Some(TimedValues::new(None, Some(1_000)).expect("v")))),
                ..item(1)
            }],
            0,
        )
        .expect("login");
        host.advance(1_000);
        let expiry = host.due_writes(1_000, tx, spare);
        assert_eq!(expiry[0].write.expected_revision, 3);
        host.on_outcome(&[1; 16], LaneWriteOutcome::Committed, 1_000)
            .expect("expired");
        // §8: the same item stays live as the target, at revision 4 from the target's full values.
        assert!(host.due_writes(1_000, tx, spare).is_empty());
        assert_eq!(host.live_count(), 1);
        host.advance(SPARE_MS - 1);
        host.stop_all();
        let stop = host.due_writes(SPARE_MS, tx, spare);
        assert_eq!(
            stop[0].definition.definition,
            target("oteryn:item.tibia.i3999")
        );
        assert_eq!(stop[0].definition.decay_target, None);
        assert_eq!(
            stop[0].write.kind,
            LaneWriteKind::Checkpoint {
                values: TimedValues::new(None, Some(1)).expect("v")
            }
        );
        assert_eq!(stop[0].write.expected_revision, 4);

        // A target that is not timed leaves a spent row: the lane is released.
        let mut host = self::host();
        host.host_live(
            vec![HostedItem {
                row: Some((3, Some(TimedValues::new(None, Some(1_000)).expect("v")))),
                ..item(1)
            }],
            0,
        )
        .expect("login");
        host.advance(1_000);
        assert_eq!(host.due_writes(1_000, tx, untimed).len(), 1);
        host.on_outcome(&[1; 16], LaneWriteOutcome::Committed, 1_000)
            .expect("expired");
        assert!(host.due_writes(1_000, tx, untimed).is_empty());
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
        let expiry = host.due_writes(0, tx, untimed);
        assert_eq!(
            expiry[0].write.kind,
            LaneWriteKind::Expire {
                reason: ExpireReason::ChargesExhausted
            }
        );
        assert_eq!(expiry[0].definition.decay_target, None);
        host.on_outcome(&[1; 16], LaneWriteOutcome::Committed, 0)
            .expect("expired");
        assert!(host.due_writes(0, tx, untimed).is_empty());
        // The binding is released with its lane.
        host.bind_exercise(charged(2, 2), 0).expect("rebind");
        assert!(host.spend_exercise_charge());
        host.unbind_exercise().expect("unbind");
        assert_eq!(host.due_writes(1, tx, untimed).len(), 1);
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
        let stops = host.due_writes(1_000, tx, untimed);
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
        assert!(host.due_writes(1_000, tx, untimed).is_empty());
        assert!(host.all_empty());
        assert_eq!(host.live_count(), 0);
    }
}
