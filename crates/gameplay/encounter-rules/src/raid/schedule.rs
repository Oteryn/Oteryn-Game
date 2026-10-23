use super::{CHECK_INTERVAL_MS, CalendarDay, RaidDefinition, RaidError, RaidPolicy};
use oteryn_gameplay_content::content::ProductionKey;
use oteryn_protocol_oteryn::WorldId;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FiringState {
    Running,
    Done,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Firing {
    pub ordinal: u64,
    pub firing_id: [u8; 16],
    pub reset_epoch: u64,
    pub fired_at_ms: i64,
    pub deadline_ms: i64,
    pub bundle_digest: [u8; 32],
    pub state: FiringState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaidSchedule {
    pub revision: u64,
    pub reset_epoch: u64,
    pub check_ordinal: u64,
    pub next_firing_ordinal: u64,
    pub next_check_at_ms: i64,
    pub last_fired_at_ms: Option<i64>,
    pub last_fixed_slot_ms: Option<i64>,
    pub retired: bool,
    pub latest_firing: Option<Firing>,
}

#[derive(Debug, Clone, Copy)]
pub enum RaidAction {
    Check {
        now_ms: i64,
        /// Caller-resolved admitted-content / World-change prerequisites.
        /// A true bit is proposed input, never authority to realize anything.
        eligible: bool,
        calendar: Option<CalendarDay>,
        firing_id: [u8; 16],
        bundle_digest: [u8; 32],
    },
    /// Metadata-only close at the deadline. Live runs/LOST reconciliation must
    /// be composed by their future owner before any runtime activation.
    Close {
        now_ms: i64,
    },
    Reset {
        reset_epoch: u64,
        now_ms: i64,
    },
    Retire,
}

/// Constructed only by the pure planner. Applying this metadata does not commit
/// the transaction or issue a realization/announcement/ownership token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaidTransition {
    world: WorldId,
    key: ProductionKey,
    before: RaidSchedule,
    after: RaidSchedule,
}
impl RaidTransition {
    pub fn world(&self) -> WorldId {
        self.world
    }
    pub fn key(&self) -> &ProductionKey {
        &self.key
    }
    pub fn before(&self) -> &RaidSchedule {
        &self.before
    }
    pub fn after(&self) -> &RaidSchedule {
        &self.after
    }
}

impl RaidSchedule {
    pub fn initial(reset_epoch: u64, now_ms: i64) -> Result<Self, RaidError> {
        let state = Self {
            revision: 1,
            reset_epoch,
            check_ordinal: 0,
            next_firing_ordinal: 1,
            next_check_at_ms: now_ms
                .checked_add(CHECK_INTERVAL_MS)
                .ok_or(RaidError::Exhausted)?,
            last_fired_at_ms: None,
            last_fixed_slot_ms: None,
            retired: false,
            latest_firing: None,
        };
        if now_ms < 0 {
            return Err(RaidError::InvalidTime);
        }
        state.validate()?;
        Ok(state)
    }

    /// Used on database decode as well as at every transition boundary.
    pub fn validate(&self) -> Result<(), RaidError> {
        if self.revision == 0
            || self.reset_epoch == 0
            || self.next_firing_ordinal == 0
            || self.next_check_at_ms < CHECK_INTERVAL_MS
            || self
                .last_fired_at_ms
                .is_some_and(|at| at < 0 || at >= self.next_check_at_ms)
            || self
                .last_fixed_slot_ms
                .is_some_and(|at| at < 0 || at >= self.next_check_at_ms)
        {
            return Err(RaidError::InvalidState);
        }
        match &self.latest_firing {
            None if self.next_firing_ordinal != 1 || self.last_fired_at_ms.is_some() => {
                return Err(RaidError::InvalidState);
            }
            Some(firing)
                if firing.ordinal == 0
                    || firing.ordinal.checked_add(1) != Some(self.next_firing_ordinal)
                    || firing.reset_epoch == 0
                    || firing.reset_epoch > self.reset_epoch
                    || firing.fired_at_ms < 0
                    || firing.deadline_ms < firing.fired_at_ms
                    || self.last_fired_at_ms != Some(firing.fired_at_ms)
                    || (firing.state == FiringState::Running
                        && (firing.reset_epoch != self.reset_epoch || self.retired))
                    || WorldId::decode(&firing.firing_id).is_err() =>
            {
                return Err(RaidError::InvalidState);
            }
            _ => {}
        }
        Ok(())
    }

    pub fn plan(
        &self,
        world: WorldId,
        key: &ProductionKey,
        definition: &RaidDefinition,
        action: RaidAction,
    ) -> Result<RaidTransition, RaidError> {
        self.validate()?;
        definition.validate()?;
        let mut after = self.clone();
        match action {
            RaidAction::Check {
                now_ms,
                eligible,
                calendar,
                firing_id,
                bundle_digest,
            } => {
                // Wrong policy/calendar combinations are errors even on an early check.
                let slot = match (definition.policy, calendar) {
                    (RaidPolicy::Random { .. }, None) => None,
                    (
                        RaidPolicy::Fixed {
                            selector,
                            offset_after_reset_ms,
                        },
                        Some(day),
                    ) => {
                        if day.reset_at_ms > now_ms {
                            return Err(RaidError::InvalidCalendar);
                        }
                        selector.slot(day, offset_after_reset_ms)?
                    }
                    _ => return Err(RaidError::InvalidCalendar),
                };
                if now_ms < 0 {
                    return Err(RaidError::InvalidTime);
                }
                if !self.retired && now_ms >= self.next_check_at_ms {
                    after.next_check_at_ms = now_ms
                        .checked_add(CHECK_INTERVAL_MS)
                        .ok_or(RaidError::Exhausted)?;
                    after.check_ordinal = self
                        .check_ordinal
                        .checked_add(1)
                        .ok_or(RaidError::Exhausted)?;
                    // One native deterministic draw for each due random check,
                    // including blocked checks. All variable-length parts are framed.
                    let selected = match definition.policy {
                        RaidPolicy::Random {
                            mean_interval_s,
                            min_gap_s,
                        } => {
                            let won =
                                random_selected(world, key, self.check_ordinal, mean_interval_s)?;
                            let gap_ok = self
                                .last_fired_at_ms
                                .is_none_or(|last| now_ms - last >= i64::from(min_gap_s) * 1_000);
                            won && gap_ok
                        }
                        RaidPolicy::Fixed { .. } => slot.is_some_and(|at| {
                            at <= now_ms
                                && now_ms - at < CHECK_INTERVAL_MS
                                && self.last_fixed_slot_ms.is_none_or(|last| at > last)
                        }),
                    };
                    let running = self
                        .latest_firing
                        .as_ref()
                        .is_some_and(|f| f.state == FiringState::Running);
                    let persistent_blocked = definition.despawn_after_ms.is_none()
                        && self
                            .latest_firing
                            .as_ref()
                            .is_some_and(|f| f.reset_epoch == self.reset_epoch);
                    // A fixed slot is consumed even if blocked; it cannot be retried
                    // after eligibility changes during the same interval.
                    if matches!(definition.policy, RaidPolicy::Fixed { .. }) {
                        after.last_fixed_slot_ms = slot
                            .filter(|at| *at <= now_ms)
                            .map(|at| self.last_fixed_slot_ms.map_or(at, |last| last.max(at)))
                            .or(self.last_fixed_slot_ms);
                    }
                    if selected && eligible && !running && !persistent_blocked {
                        if WorldId::decode(&firing_id).is_err() {
                            return Err(RaidError::InvalidState);
                        }
                        after.next_firing_ordinal = self
                            .next_firing_ordinal
                            .checked_add(1)
                            .ok_or(RaidError::Exhausted)?;
                        after.last_fired_at_ms = Some(now_ms);
                        after.latest_firing = Some(Firing {
                            ordinal: self.next_firing_ordinal,
                            firing_id,
                            reset_epoch: self.reset_epoch,
                            fired_at_ms: now_ms,
                            deadline_ms: definition.deadline(now_ms)?,
                            bundle_digest,
                            state: FiringState::Running,
                        });
                    }
                }
            }
            RaidAction::Close { now_ms } => {
                if now_ms < 0 {
                    return Err(RaidError::InvalidTime);
                }
                if let Some(firing) = &mut after.latest_firing
                    && firing.state == FiringState::Running
                    && now_ms >= firing.deadline_ms
                {
                    firing.state = FiringState::Done;
                }
            }
            RaidAction::Reset {
                reset_epoch,
                now_ms,
            } => {
                if reset_epoch <= self.reset_epoch
                    || now_ms < 0
                    || now_ms < self.next_check_at_ms - CHECK_INTERVAL_MS
                    || self.last_fired_at_ms.is_some_and(|at| at > now_ms)
                {
                    return Err(RaidError::InvalidState);
                }
                after.reset_epoch = reset_epoch;
                after.next_check_at_ms = now_ms
                    .checked_add(CHECK_INTERVAL_MS)
                    .ok_or(RaidError::Exhausted)?;
                cancel_running(&mut after);
            }
            RaidAction::Retire => {
                after.retired = true;
                cancel_running(&mut after);
            }
        }
        if after != *self {
            after.revision = self.revision.checked_add(1).ok_or(RaidError::Exhausted)?;
        }
        after.validate()?;
        Ok(RaidTransition {
            world,
            key: key.clone(),
            before: self.clone(),
            after,
        })
    }
}

fn cancel_running(schedule: &mut RaidSchedule) {
    if let Some(firing) = &mut schedule.latest_firing
        && firing.state == FiringState::Running
    {
        firing.state = FiringState::Cancelled;
    }
}

pub(super) fn random_selected(
    world: WorldId,
    key: &ProductionKey,
    ordinal: u64,
    mean_interval_s: u32,
) -> Result<bool, RaidError> {
    let mut hash = Sha256::new();
    hash.update(b"oteryn.raid.schedule.v1\0");
    hash.update(world.as_bytes());
    hash.update(
        u64::try_from(key.as_str().len())
            .map_err(|_| RaidError::Exhausted)?
            .to_be_bytes(),
    );
    hash.update(key.as_str().as_bytes());
    let root = GameplayDecisionRoot::from_bytes(hash.finalize().into());
    let mut occurrence = [0; 16];
    occurrence[8..].copy_from_slice(&ordinal.to_be_bytes());
    let draw = deterministic_decision_u64(
        &root,
        DecisionOccurrenceId::from_bytes(occurrence),
        "raid-check",
        0,
    )
    .map_err(|_| RaidError::InvalidState)?;
    // BOSS_RAID0 §4.2: chance = min(1, 60 / mean_interval_s). Wide
    // multiplication avoids overflow and modulo bias with exactly one draw.
    Ok(wins_interval_roll(draw, mean_interval_s))
}

pub(super) fn wins_interval_roll(draw: u64, mean_interval_s: u32) -> bool {
    u128::from(draw) * u128::from(mean_interval_s) < (1_u128 << 64) * 60
}
