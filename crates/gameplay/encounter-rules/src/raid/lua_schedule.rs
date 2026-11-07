//! Crystal 201189f modern Lua raid check semantics. The durable World owner
//! supplies the calendar, next server save and current player idle evidence;
//! these proposed facts do not authorize a firing or its live effects.
use super::{CHECK_INTERVAL_MS, RaidError};
use oteryn_gameplay_content::content::ProductionKey;
use oteryn_protocol_oteryn::WorldId;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// data/libs/systems/raids.lua:14-15,91. The source excludes the final hour
/// before server save and uses 23 hours / one minute in its probability formula.
pub const LUA_RAID_CHECKS_PER_DAY: u32 = 1_380;
pub const LUA_RAID_ACTIVE_IDLE_MS: u64 = 300_000;
const ROLL_DENOMINATOR: u32 = 100_000;

#[derive(Debug, Clone, PartialEq)]
pub struct LuaRaidPolicy {
    /// Source values are percentages: 0.10 means a 0.1% chance per check.
    pub initial_chance: Option<f64>,
    pub target_chance_per_day: f64,
    pub max_chance_per_check: f64,
    pub min_gap_ms: u64,
    /// Monday = 1, Sunday = 7; empty means that no day restriction is set.
    pub allowed_weekdays: Vec<u8>,
    pub min_active_players: u32,
    pub max_checks_per_day: Option<u64>,
}

impl LuaRaidPolicy {
    pub fn validate(&self) -> Result<(), RaidError> {
        let percentage = |value: f64| value.is_finite() && (0.0..=100.0).contains(&value);
        if !percentage(self.target_chance_per_day)
            || !percentage(self.max_chance_per_check)
            || self.initial_chance.is_some_and(|value| !percentage(value))
            || self
                .allowed_weekdays
                .iter()
                .any(|day| !(1..=7).contains(day))
        {
            return Err(RaidError::InvalidDefinition);
        }
        Ok(())
    }

    fn wins(&self, failed_attempts: u64, roll: u32) -> bool {
        let checks = f64::from(LUA_RAID_CHECKS_PER_DAY);
        let initial = self
            .initial_chance
            .unwrap_or(self.target_chance_per_day / checks);
        let increase = ((self.target_chance_per_day - initial) / checks).max(0.0);
        // Preserve the source's floating-point operation order and inclusive
        // integer roll comparison; rounding to whole milli-percent first would
        // change checks close to the threshold.
        let chance = (initial + increase * failed_attempts as f64).min(self.max_chance_per_check);
        f64::from(roll) <= chance * 1_000.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LuaRaidState {
    pub next_check_at_ms: i64,
    pub check_ordinal: u64,
    /// The source stores os.time() in seconds. This value is always second-aligned.
    pub last_occurrence_ms: Option<i64>,
    /// This source KV key is named checks-today, but raids.lua contains no daily
    /// reset. Preserve it across calendar changes instead of inventing one.
    pub checks_today: u64,
    pub failed_attempts: u64,
    pub trigger_when_possible: bool,
    /// Caller-owned monotone calendar-day identity, with no timezone assumption.
    pub check_day: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LuaRaidCheck {
    pub now_ms: i64,
    pub day: i64,
    pub weekday: u8,
    pub server_save_at_ms: i64,
    pub active_players: u32,
    pub running: bool,
    /// A deterministic native draw with the source's inclusive range 1..=100000.
    pub roll: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LuaRaidTransition {
    pub state: LuaRaidState,
    pub start: bool,
}

impl LuaRaidState {
    pub fn initial(now_ms: i64, check_day: i64) -> Result<Self, RaidError> {
        if now_ms < 0 {
            return Err(RaidError::InvalidTime);
        }
        let state = Self {
            next_check_at_ms: now_ms
                .checked_add(CHECK_INTERVAL_MS)
                .ok_or(RaidError::Exhausted)?,
            check_ordinal: 0,
            last_occurrence_ms: None,
            checks_today: 0,
            failed_attempts: 0,
            trigger_when_possible: false,
            check_day,
        };
        state.validate()?;
        Ok(state)
    }

    pub fn validate(&self) -> Result<(), RaidError> {
        if self.next_check_at_ms < CHECK_INTERVAL_MS
            || self.checks_today > self.check_ordinal
            || self.failed_attempts > self.checks_today
            || self.last_occurrence_ms.is_some_and(|at| {
                at < 0 || at % 1_000 != 0 || at > self.next_check_at_ms - CHECK_INTERVAL_MS
            })
        {
            return Err(RaidError::InvalidState);
        }
        Ok(())
    }

    pub fn plan(
        &self,
        policy: &LuaRaidPolicy,
        check: LuaRaidCheck,
    ) -> Result<LuaRaidTransition, RaidError> {
        self.validate()?;
        policy.validate()?;
        if check.now_ms < 0 || check.server_save_at_ms < check.now_ms {
            return Err(RaidError::InvalidTime);
        }
        if check.day < self.check_day || !(1..=7).contains(&check.weekday) {
            return Err(RaidError::InvalidCalendar);
        }
        if !(1..=ROLL_DENOMINATOR).contains(&check.roll) {
            return Err(RaidError::InvalidState);
        }
        let mut transition = LuaRaidTransition {
            state: self.clone(),
            start: false,
        };
        if check.now_ms < self.next_check_at_ms {
            return Ok(transition);
        }
        let after = &mut transition.state;
        after.next_check_at_ms = check
            .now_ms
            .checked_add(CHECK_INTERVAL_MS)
            .ok_or(RaidError::Exhausted)?;
        after.check_ordinal = self
            .check_ordinal
            .checked_add(1)
            .ok_or(RaidError::Exhausted)?;
        after.check_day = check.day;

        // data/scripts/globalevents/raids.lua:4-7: strict >, using integer
        // os.time() seconds. The exact one-hour boundary is still permitted.
        let now_s = check.now_ms / 1_000;
        let server_save_s = check.server_save_at_ms / 1_000;
        if now_s > server_save_s - 3_600 || check.running {
            return Ok(transition);
        }
        if !self.trigger_when_possible {
            // raids.lua:69-86: gap and check budget precede the random draw.
            let last = self.last_occurrence_ms.unwrap_or(0);
            let elapsed =
                u64::try_from(now_s * 1_000 - last).map_err(|_| RaidError::InvalidTime)?;
            if elapsed < policy.min_gap_ms
                || policy
                    .max_checks_per_day
                    .is_some_and(|maximum| self.checks_today >= maximum)
            {
                return Ok(transition);
            }
            after.checks_today = self
                .checks_today
                .checked_add(1)
                .ok_or(RaidError::Exhausted)?;
            if !policy.wins(self.failed_attempts, check.roll) {
                after.failed_attempts = self
                    .failed_attempts
                    .checked_add(1)
                    .ok_or(RaidError::Exhausted)?;
                return Ok(transition);
            }
        }
        // raids.lua:108-121: a winning draw is retained until day/player gates
        // pass. Pending starts bypass gap, check budget and all later rolls.
        if (!policy.allowed_weekdays.is_empty()
            && !policy.allowed_weekdays.contains(&check.weekday))
            || check.active_players < policy.min_active_players
        {
            after.trigger_when_possible = true;
            return Ok(transition);
        }
        after.trigger_when_possible = false;
        after.failed_attempts = 0;
        after.last_occurrence_ms = Some(now_s * 1_000);
        transition.start = true;
        after.validate()?;
        Ok(transition)
    }
}

/// raids.lua:146-153: idle time equal to five minutes is already inactive.
pub fn lua_raid_player_is_active(idle_ms: u64) -> bool {
    idle_ms < LUA_RAID_ACTIVE_IDLE_MS
}

/// One reproducible, unbiased draw per durable check identity. Rejections stay
/// within that identity and never consume another raid's or check's ordinal.
pub fn lua_raid_roll(world: WorldId, key: &ProductionKey, ordinal: u64) -> Result<u32, RaidError> {
    let mut hash = Sha256::new();
    hash.update(b"oteryn.raid.lua.schedule.v1\0");
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
    let bound = u64::from(ROLL_DENOMINATOR);
    let threshold = bound.wrapping_neg() % bound;
    let mut rejection = 0_u64;
    loop {
        let draw = deterministic_decision_u64(
            &root,
            DecisionOccurrenceId::from_bytes(occurrence),
            "raid-lua-check",
            rejection,
        )
        .map_err(|_| RaidError::InvalidState)?;
        if draw >= threshold {
            return u32::try_from(draw % bound + 1).map_err(|_| RaidError::Exhausted);
        }
        rejection = rejection.checked_add(1).ok_or(RaidError::Exhausted)?;
    }
}

#[cfg(test)]
#[path = "lua_schedule_tests.rs"]
mod tests;
