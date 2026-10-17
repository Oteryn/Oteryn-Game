use crate::foundation::{ChannelId, ScopeOwnershipGeneration, WorldId};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[path = "calendar.rs"]
pub(crate) mod calendar;
#[path = "store.rs"]
mod store;

const CHECK_MS: i64 = 60_000; // BOSSRAID0-RL-01; registration remains CP.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RaidError {
    InvalidInput,
    InvalidStoredState,
    Overflow,
    TimeBeforeState,
    StaleGeneration,
    NotTargeted,
    NotRunning,
}
impl std::fmt::Display for RaidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for RaidError {}
type Result<T> = std::result::Result<T, RaidError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RaidKey {
    pub(crate) world: WorldId,
    pub(crate) raid: String,
}
impl RaidKey {
    fn valid(&self) -> bool {
        ProductionKey::new(&self.raid).is_ok()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum Policy {
    Random {
        mean_interval_s: u64,
        min_gap_s: u64,
    },
    Fixed,
}

/// Proposed source facts, not admitted content. Fixed calendar interpretation is caller-owned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Timing {
    pub(crate) policy: Policy,
    pub(crate) last_wave_delay_ms: i64,
    pub(crate) despawn_after_ms: Option<i64>,
}
impl Timing {
    fn validate(&self) -> Result<()> {
        if self.last_wave_delay_ms < 0
            || self.despawn_after_ms.is_some_and(|v| v < 0)
            || matches!(
                self.policy,
                Policy::Random {
                    mean_interval_s: 0,
                    ..
                }
            )
        {
            return Err(RaidError::InvalidInput);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Schedule {
    revision: u64,
    reset_epoch: u64,
    bundle_digest: [u8; 32],
    retired: bool,
    timing: Timing,
    last_fired_at_ms: Option<i64>,
    next_check_at_ms: i64,
    check_ordinal: u64,
    next_firing_ordinal: u64,
    last_fixed_slot_ms: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum FiringState {
    Running,
    Done,
    Cancelled,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Run {
    Claimed(ScopeOwnershipGeneration),
    Done(ScopeOwnershipGeneration),
    Lost(Option<ScopeOwnershipGeneration>),
}
impl Run {
    fn terminal(self) -> bool {
        !matches!(self, Self::Claimed(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Firing {
    pub(crate) ordinal: u64,
    pub(crate) id: [u8; 16],
    pub(crate) fired_at_ms: i64,
    pub(crate) reset_epoch: u64,
    pub(crate) bundle_digest: [u8; 32],
    pub(crate) close_at_ms: i64,
    pub(crate) state: FiringState,
    /// None means targeted but no run row yet. This is never terminal before the deadline.
    pub(crate) targets: BTreeMap<ChannelId, Option<Run>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Snapshot {
    key: RaidKey,
    schedule: Schedule,
    pub(crate) latest: Option<Firing>,
}

/// Private fields prevent callers fabricating an arbitrary write through the CAS helper.
/// This seals only a data transition; it is expressly NOT a current-authority capability.
#[derive(Debug, Clone)]
pub(crate) struct Change {
    before: Option<Snapshot>,
    after: Snapshot,
}
impl Change {
    pub(crate) fn proposed(&self) -> &Snapshot {
        &self.after
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CheckFacts {
    pub(crate) now_ms: i64,
    /// Server-generated UUIDv7 supplied by the eventual owner; this core never generates IDs.
    pub(crate) proposed_firing_id: [u8; 16],
    pub(crate) proposed_targets: Vec<ChannelId>,
    /// Proposed absolute slot, not proof of calendar/reset/content truth.
    pub(crate) fixed_slot_ms: Option<i64>,
}

impl Snapshot {
    pub(crate) fn initialize(
        key: RaidKey,
        epoch: u64,
        bundle: [u8; 32],
        timing: Timing,
        now: i64,
    ) -> Result<Change> {
        timing.validate()?;
        if !key.valid() || bundle == [0; 32] || now < 0 {
            return Err(RaidError::InvalidInput);
        }
        let after = Self {
            key,
            schedule: Schedule {
                revision: 0,
                reset_epoch: epoch,
                bundle_digest: bundle,
                retired: false,
                timing,
                last_fired_at_ms: None,
                next_check_at_ms: add(now, CHECK_MS)?,
                check_ordinal: 0,
                next_firing_ordinal: 0,
                last_fixed_slot_ms: None,
            },
            latest: None,
        };
        after.validate()?;
        Ok(Change {
            before: None,
            after,
        })
    }
    pub(crate) fn revision(&self) -> u64 {
        self.schedule.revision
    }
    pub(crate) fn next_check_at_ms(&self) -> i64 {
        self.schedule.next_check_at_ms
    }
    pub(crate) fn check_ordinal(&self) -> u64 {
        self.schedule.check_ordinal
    }
    fn changed(&self, mut after: Self) -> Result<Change> {
        after.schedule.revision = self
            .schedule
            .revision
            .checked_add(1)
            .ok_or(RaidError::Overflow)?;
        after.validate()?;
        Ok(Change {
            before: Some(self.clone()),
            after,
        })
    }
    /// One check only. Missed intervals are not caught up; a retry of the same pre-state uses
    /// the same draw. A persisted exact retry is handled by the private store CAS.
    pub(crate) fn check(&self, facts: &CheckFacts) -> Result<Option<Change>> {
        self.check_resolved_slot(
            facts,
            facts.fixed_slot_ms.map(calendar::ResolvedFixedSlot::At),
        )
    }

    /// Resolve a fixed calendar rule into this same check/ordinal transition. The caller
    /// supplies the reset's calendar interpretation; neither this conversion nor its
    /// proposal establishes current reset/content/channel authority. A nonmatching day
    /// still consumes the ordinary due check, with no firing and no synthetic slot.
    pub(crate) fn check_fixed_calendar(
        &self,
        facts: &CheckFacts,
        rule: &calendar::FixedCalendarRule,
        reset: &calendar::ResetCalendarFacts,
    ) -> Result<Option<Change>> {
        if !matches!(self.schedule.timing.policy, Policy::Fixed) || facts.fixed_slot_ms.is_some() {
            return Err(RaidError::InvalidInput);
        }
        self.check_resolved_slot(facts, Some(calendar::resolve(rule, reset)?))
    }

    fn check_resolved_slot(
        &self,
        facts: &CheckFacts,
        fixed_slot: Option<calendar::ResolvedFixedSlot>,
    ) -> Result<Option<Change>> {
        self.validate()?;
        if facts.now_ms < 0 {
            return Err(RaidError::InvalidInput);
        }
        if self.schedule.retired || facts.now_ms < self.schedule.next_check_at_ms {
            return Ok(None);
        }
        let mut after = self.clone();
        after.schedule.check_ordinal = self
            .schedule
            .check_ordinal
            .checked_add(1)
            .ok_or(RaidError::Overflow)?;
        after.schedule.next_check_at_ms = add(facts.now_ms, CHECK_MS)?;
        let running = self
            .latest
            .as_ref()
            .is_some_and(|f| f.state == FiringState::Running);
        let persistent_in_epoch = self.schedule.timing.despawn_after_ms.is_none()
            && self
                .latest
                .as_ref()
                .is_some_and(|f| f.reset_epoch == self.schedule.reset_epoch);
        let allowed = !(running || persistent_in_epoch);
        let fires = match self.schedule.timing.policy {
            Policy::Random {
                mean_interval_s,
                min_gap_s,
            } => {
                if fixed_slot.is_some() {
                    return Err(RaidError::InvalidInput);
                }
                let gap_ok = match self.schedule.last_fired_at_ms {
                    Some(last) => {
                        let gap = facts
                            .now_ms
                            .checked_sub(last)
                            .ok_or(RaidError::TimeBeforeState)?;
                        if gap < 0 {
                            return Err(RaidError::TimeBeforeState);
                        }
                        u128::from(u64::try_from(gap).map_err(|_| RaidError::TimeBeforeState)?)
                            >= u128::from(min_gap_s) * 1000
                    }
                    None => true,
                };
                // Native RNG only; the tuple is explicitly framed. Multiplication comparison
                // implements interval/mean without modulo bias or a second random generator.
                let draw = draw(&self.key, self.schedule.check_ordinal)?;
                allowed
                    && gap_ok
                    && u128::from(draw) * u128::from(mean_interval_s) < (1_u128 << 64) * 60
            }
            Policy::Fixed => {
                match fixed_slot.ok_or(RaidError::InvalidInput)? {
                    calendar::ResolvedFixedSlot::Absent => false,
                    calendar::ResolvedFixedSlot::At(slot) => {
                        if slot < 0 {
                            return Err(RaidError::InvalidInput);
                        }
                        // A fixed slot outside the current check interval is missed, not caught up.
                        allowed
                            && facts.now_ms >= slot
                            && slot >= facts.now_ms.saturating_sub(CHECK_MS)
                            && self
                                .schedule
                                .last_fixed_slot_ms
                                .is_none_or(|last| slot > last)
                    }
                }
            }
        };
        if fires {
            if !uuid_v7(facts.proposed_firing_id)
                || self
                    .latest
                    .as_ref()
                    .is_some_and(|f| f.id == facts.proposed_firing_id)
            {
                return Err(RaidError::InvalidInput);
            }
            let mut targets = BTreeMap::new();
            for channel in &facts.proposed_targets {
                if targets.insert(*channel, None).is_some() {
                    return Err(RaidError::InvalidInput);
                }
            }
            let close_at = add(
                add(facts.now_ms, self.schedule.timing.last_wave_delay_ms)?,
                self.schedule.timing.despawn_after_ms.unwrap_or(0),
            )?;
            after.latest = Some(Firing {
                ordinal: self.schedule.next_firing_ordinal,
                id: facts.proposed_firing_id,
                fired_at_ms: facts.now_ms,
                reset_epoch: self.schedule.reset_epoch,
                bundle_digest: self.schedule.bundle_digest,
                close_at_ms: close_at,
                state: FiringState::Running,
                targets,
            });
            after.schedule.next_firing_ordinal = self
                .schedule
                .next_firing_ordinal
                .checked_add(1)
                .ok_or(RaidError::Overflow)?;
            after.schedule.last_fired_at_ms = Some(facts.now_ms);
            if matches!(self.schedule.timing.policy, Policy::Fixed) {
                let Some(calendar::ResolvedFixedSlot::At(slot)) = fixed_slot else {
                    return Err(RaidError::InvalidInput);
                };
                after.schedule.last_fixed_slot_ms = Some(slot);
            }
        }
        self.changed(after).map(Some)
    }
    /// Explicit restart scheduling proposal; does not reset last-fired/min-gap/ordinals.
    pub(crate) fn resume_from(&self, now: i64) -> Result<Change> {
        self.validate()?;
        if now < 0
            || self
                .schedule
                .last_fired_at_ms
                .is_some_and(|last| now < last)
        {
            return Err(RaidError::TimeBeforeState);
        }
        let mut after = self.clone();
        after.schedule.next_check_at_ms = add(now, CHECK_MS)?;
        self.changed(after)
    }
    pub(crate) fn retire(&self) -> Result<Option<Change>> {
        self.validate()?;
        if self.schedule.retired {
            return Ok(None);
        }
        let mut after = self.clone();
        after.schedule.retired = true;
        self.changed(after).map(Some)
    }
    /// Proposed activation facts only. Actual current WorldReset/active bundle proof is CP.
    pub(crate) fn reset(
        &self,
        epoch: u64,
        bundle: [u8; 32],
        timing: Timing,
        now: i64,
    ) -> Result<Change> {
        self.validate()?;
        timing.validate()?;
        if epoch <= self.schedule.reset_epoch
            || bundle == [0; 32]
            || now < 0
            || self
                .schedule
                .last_fired_at_ms
                .is_some_and(|last| now < last)
        {
            return Err(RaidError::InvalidInput);
        }
        let mut after = self.clone();
        if let Some(firing) = after.latest.as_mut()
            && firing.state == FiringState::Running
        {
            firing.state = FiringState::Cancelled;
        }
        after.schedule.reset_epoch = epoch;
        after.schedule.bundle_digest = bundle;
        after.schedule.timing = timing;
        after.schedule.retired = false;
        after.schedule.next_check_at_ms = add(now, CHECK_MS)?;
        self.changed(after)
    }
    /// Run-row claim/recovery DATA only. No spawn/announcement action or live-owner proof.
    pub(crate) fn claim(
        &self,
        id: [u8; 16],
        channel: ChannelId,
        generation: ScopeOwnershipGeneration,
    ) -> Result<Option<Change>> {
        self.validate()?;
        let firing = self
            .latest
            .as_ref()
            .filter(|f| f.id == id && f.state == FiringState::Running)
            .ok_or(RaidError::NotRunning)?;
        let row = firing.targets.get(&channel).ok_or(RaidError::NotTargeted)?;
        let next = match row {
            None => Run::Claimed(generation),
            Some(Run::Claimed(old)) if generation.get() > old.get() => Run::Lost(Some(*old)),
            Some(Run::Claimed(old) | Run::Done(old) | Run::Lost(Some(old)))
                if generation.get() < old.get() =>
            {
                return Err(RaidError::StaleGeneration);
            }
            _ => return Ok(None),
        };
        let mut after = self.clone();
        after
            .latest
            .as_mut()
            .ok_or(RaidError::InvalidStoredState)?
            .targets
            .insert(channel, Some(next));
        self.changed(after).map(Some)
    }
    /// Completion metadata proposal. The actual owner must prove run completion separately;
    /// this pure function never claims any physical wave/announcement has happened.
    pub(crate) fn record_done(
        &self,
        id: [u8; 16],
        channel: ChannelId,
        generation: ScopeOwnershipGeneration,
    ) -> Result<Option<Change>> {
        self.validate()?;
        let firing = self
            .latest
            .as_ref()
            .filter(|f| f.id == id && f.state == FiringState::Running)
            .ok_or(RaidError::NotRunning)?;
        match firing.targets.get(&channel).ok_or(RaidError::NotTargeted)? {
            Some(Run::Claimed(old)) if *old == generation => {}
            Some(Run::Done(old)) if *old == generation => return Ok(None),
            _ => return Err(RaidError::StaleGeneration),
        }
        let mut after = self.clone();
        after
            .latest
            .as_mut()
            .ok_or(RaidError::InvalidStoredState)?
            .targets
            .insert(channel, Some(Run::Done(generation)));
        self.changed(after).map(Some)
    }
    pub(crate) fn close(&self, now: i64) -> Result<Option<Change>> {
        self.validate()?;
        let Some(firing) = self
            .latest
            .as_ref()
            .filter(|f| f.state == FiringState::Running)
        else {
            return Ok(None);
        };
        if now < firing.fired_at_ms {
            return Err(RaidError::TimeBeforeState);
        }
        let all_terminal = !firing.targets.is_empty()
            && firing
                .targets
                .values()
                .all(|r| r.is_some_and(Run::terminal));
        if !all_terminal && now < firing.close_at_ms {
            return Ok(None);
        }
        let mut after = self.clone();
        let next = after.latest.as_mut().ok_or(RaidError::InvalidStoredState)?;
        if now >= firing.close_at_ms {
            for row in next.targets.values_mut() {
                if row.is_none() {
                    *row = Some(Run::Lost(None));
                }
            }
        }
        next.state = FiringState::Done;
        self.changed(after).map(Some)
    }
    fn validate(&self) -> Result<()> {
        self.schedule
            .timing
            .validate()
            .map_err(|_| RaidError::InvalidStoredState)?;
        if !self.key.valid()
            || self.schedule.bundle_digest == [0; 32]
            || self.schedule.next_check_at_ms < 0
            || self.schedule.last_fixed_slot_ms.is_some_and(|v| v < 0)
            || self.schedule.last_fired_at_ms.is_some_and(|v| v < 0)
        {
            return Err(RaidError::InvalidStoredState);
        }
        match &self.latest {
            None if self.schedule.next_firing_ordinal == 0
                && self.schedule.last_fired_at_ms.is_none() => {}
            Some(f)
                if f.ordinal.checked_add(1) == Some(self.schedule.next_firing_ordinal)
                    && uuid_v7(f.id)
                    && Some(f.fired_at_ms) == self.schedule.last_fired_at_ms
                    && f.fired_at_ms >= 0
                    && f.close_at_ms >= f.fired_at_ms
                    && f.bundle_digest != [0; 32]
                    && f.reset_epoch <= self.schedule.reset_epoch
                    && (f.state != FiringState::Running
                        || (f.reset_epoch == self.schedule.reset_epoch
                            && f.bundle_digest == self.schedule.bundle_digest)) => {}
            _ => return Err(RaidError::InvalidStoredState),
        }
        Ok(())
    }
}
fn add(a: i64, b: i64) -> Result<i64> {
    a.checked_add(b).ok_or(RaidError::Overflow)
}
fn uuid_v7(id: [u8; 16]) -> bool {
    id[6] >> 4 == 7 && id[8] >> 6 == 2
}
fn draw(key: &RaidKey, ordinal: u64) -> Result<u64> {
    let mut hash = Sha256::new();
    hash.update(b"oteryn.raid-check-tuple.v1\0");
    hash.update(key.world.as_bytes());
    hash.update(
        u64::try_from(key.raid.len())
            .map_err(|_| RaidError::InvalidInput)?
            .to_be_bytes(),
    );
    hash.update(key.raid.as_bytes());
    deterministic_decision_u64(
        &GameplayDecisionRoot::from_bytes(hash.finalize().into()),
        DecisionOccurrenceId::from_bytes(*key.world.as_bytes()),
        "raid-check",
        ordinal,
    )
    .map_err(|_| RaidError::InvalidInput)
}
