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
pub struct TimedItemDefinition<T> {
    pub definition: T,
    pub full: TimedValues,
    /// `None` burns the item at expiry (§8).
    pub decay_target: Option<T>,
}

/// Content supplies the timed definition without coupling the host to its schema.
pub trait TimedDefinitionSource<T> {
    fn resolve_timed_definition(
        &self,
        definition: &T,
    ) -> Result<Option<TimedItemDefinition<T>>, TimedHostError>;
}

/// The item's timed definition, or `None` when the hosting runtime does not run it.
pub fn resolve_timed_definition<T>(
    definition: &T,
    semantics: &impl TimedDefinitionSource<T>,
) -> Result<Option<TimedItemDefinition<T>>, TimedHostError> {
    semantics.resolve_timed_definition(definition)
}

/// One item to make live: its definition and its row (`None` when it has none).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedItem<T> {
    pub item_instance_id: [u8; 16],
    pub definition: TimedItemDefinition<T>,
    /// The row's revision and values; values `None` is a spent row, which is never live.
    pub row: Option<(u64, Option<TimedValues>)>,
}

/// One write for the caller to run, keyed by the lane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostWrite<T> {
    pub item_instance_id: [u8; 16],
    pub definition: TimedItemDefinition<T>,
    pub write: LaneWrite,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Hosted<T> {
    definition: TimedItemDefinition<T>,
    /// The lane's write was handed to the caller and its outcome is not known yet.
    dispatched: bool,
    /// When the write in flight turned ambiguous.
    ambiguous_at_ms: Option<u64>,
}

/// The live timed items of one actor in its hosting runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedItemHost<T> {
    lanes: ActorTimedLanes,
    hosted: BTreeMap<[u8; 16], Hosted<T>>,
    exercise: Option<[u8; 16]>,
    checkpoint_interval_ms: u64,
}

impl<T: Clone> TimedItemHost<T> {
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
    pub fn host_live(
        &mut self,
        items: Vec<HostedItem<T>>,
        now_ms: u64,
    ) -> Result<(), TimedHostError> {
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
    pub fn bind_exercise(
        &mut self,
        item: HostedItem<T>,
        now_ms: u64,
    ) -> Result<(), TimedHostError> {
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
        mut timed_target: impl FnMut(&T) -> Option<TimedItemDefinition<T>>,
    ) -> Vec<HostWrite<T>> {
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

    fn open(&mut self, item: HostedItem<T>, now_ms: u64) -> Result<(), TimedHostError> {
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

    fn lane(
        &mut self,
        id: &[u8; 16],
    ) -> Result<(&mut TimedItemLane, &mut Hosted<T>), TimedHostError> {
        let hosted = self.hosted.get_mut(id).ok_or(TimedHostError::NotHosted)?;
        let lane = self.lanes.get_mut(id).ok_or(TimedHostError::NotHosted)?;
        Ok((lane, hosted))
    }
}
