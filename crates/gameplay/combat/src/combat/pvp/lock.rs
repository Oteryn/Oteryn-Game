//! PARTY-PVP-0 §8.1 combat lock (logout block, PZ block, kill block) and §8.2 aggression
//! relations. Readmission composes the existing fenced PVP-1 read with this monotone lock;
//! actor installation, fresh-harm write-ahead and full PvP activation remain separate owners.

use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::pvp_state::{PvpReadmissionRemaining, PvpRemainingMicros};
use crate::foundation::owner_timer::OwnerClock;
use oteryn_simulation_determinism::SemanticTimeMicros;

/// A refused readmission changes no runtime deadline. Stored state is data, not a controller grant.
#[derive(Debug)]
pub enum PvpReadmissionError {
    Read(CharacterProgressionError),
    DeadlineOverflow,
}

fn readmission_deadline(
    now: u64,
    remaining: PvpRemainingMicros,
) -> Result<Option<SemanticTimeMicros>, PvpReadmissionError> {
    if remaining.get() == 0 {
        return Ok(None);
    }
    now.checked_add(remaining.get())
        .map(SemanticTimeMicros::from_micros)
        .map(Some)
        .ok_or(PvpReadmissionError::DeadlineOverflow)
}

/// `PARTYPVP0-RL-12`: PZ block and PvP logout block, refreshed by each aggression or hit.
const BLOCK_MS: u64 = 60_000;
/// `PARTYPVP0-RL-13`: kill block.
pub const KILL_BLOCK_MS: u64 = 15 * 60_000;
/// `PARTYPVP0-RL-24`: aggression relations per actor.
pub const MAX_RELATIONS: usize = 64;

fn after(now: SemanticTimeMicros, ms: u64) -> SemanticTimeMicros {
    SemanticTimeMicros::from_micros(now.get().saturating_add(ms.saturating_mul(1_000)))
}

fn raise(slot: &mut Option<SemanticTimeMicros>, to: SemanticTimeMicros) {
    if slot.is_none_or(|current| current < to) {
        *slot = Some(to);
    }
}

fn running(slot: Option<SemanticTimeMicros>, now: SemanticTimeMicros) -> bool {
    slot.is_some_and(|until| until > now)
}

/// The PvP part of one actor's combat lock. A deadline is only ever raised, never lowered.
/// ATTACK-0's PvE in-fight deadline stays in `AttackState` and is passed to [`Self::blocks_logout`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CombatLock {
    logout_until: Option<SemanticTimeMicros>,
    pz_until: Option<SemanticTimeMicros>,
    kill_until: Option<SemanticTimeMicros>,
}

impl CombatLock {
    /// D1180: apply a completed, fenced database duration read synchronously. The caller
    /// finishes the database await first; this method contains no I/O or await and samples
    /// the owner's clock exactly once. It never converts a database epoch to owner time.
    /// All three checked deadlines are computed before the single monotone mutation.
    /// A missing row restores nothing; a read/overflow error preserves every existing slot.
    pub fn restore_pvp_readmission(
        &mut self,
        read: Result<Option<PvpReadmissionRemaining>, CharacterProgressionError>,
        clock: &impl OwnerClock,
    ) -> Result<bool, PvpReadmissionError> {
        let Some(remaining) = read.map_err(PvpReadmissionError::Read)? else {
            return Ok(false);
        };
        let now = clock.now().get();
        let logout = readmission_deadline(now, remaining.logout)?;
        let pz = readmission_deadline(now, remaining.pz)?;
        let kill = readmission_deadline(now, remaining.kill)?;
        self.restore(logout, pz, kill);
        Ok(true)
    }

    /// A PvP hit dealt to or taken from a character: the 60 s logout block (both sides).
    pub fn on_pvp_hit(&mut self, now: SemanticTimeMicros) {
        raise(&mut self.logout_until, after(now, BLOCK_MS));
    }

    /// Aggression that is not retaliation (or a field rune cast): the 60 s PZ block, which also
    /// blocks logout.
    pub fn on_aggression(&mut self, now: SemanticTimeMicros) {
        raise(&mut self.pz_until, after(now, BLOCK_MS));
    }

    /// A kill as a direct or assisting contributor: `until` is on this owner's axis.
    /// Convert the durable database kill deadline through D1180 remaining duration first;
    /// a database epoch must never be passed as an owner semantic timestamp.
    pub fn on_kill(&mut self, until: SemanticTimeMicros) {
        raise(&mut self.kill_until, until);
    }

    /// Restore stored deadlines at admission; never shortens a running one.
    pub fn restore(
        &mut self,
        logout: Option<SemanticTimeMicros>,
        pz: Option<SemanticTimeMicros>,
        kill: Option<SemanticTimeMicros>,
    ) {
        for (slot, stored) in [
            (&mut self.logout_until, logout),
            (&mut self.pz_until, pz),
            (&mut self.kill_until, kill),
        ] {
            if let Some(stored) = stored {
                raise(slot, stored);
            }
        }
    }

    pub fn pz_block_running(&self, now: SemanticTimeMicros) -> bool {
        running(self.pz_until, now)
    }

    pub fn kill_block_running(&self, now: SemanticTimeMicros) -> bool {
        running(self.kill_until, now)
    }

    /// Logout: any PvP block, the PZ block, the kill block or the PvE in-fight deadline.
    pub fn blocks_logout(
        &self,
        now: SemanticTimeMicros,
        pve_in_fight_until: Option<SemanticTimeMicros>,
    ) -> bool {
        running(self.logout_until, now)
            || self.pz_block_running(now)
            || self.kill_block_running(now)
            || running(pve_in_fight_until, now)
    }

    /// Stepping onto a protection-zone tile: the PZ block or the kill block bars it.
    pub fn blocks_pz_entry(&self, now: SemanticTimeMicros) -> bool {
        self.pz_block_running(now) || self.kill_block_running(now)
    }

    /// A landing (step, teleport, fall) on `destination_is_protection_zone` is refused while a
    /// PZ-entry block runs. Hook for `map::floor` landing resolution.
    pub fn landing_allowed(
        &self,
        now: SemanticTimeMicros,
        destination_is_protection_zone: bool,
    ) -> bool {
        !(destination_is_protection_zone && self.blocks_pz_entry(now))
    }

    /// A voluntary channel switch and leaving a party: blocked by the whole combat lock.
    pub fn blocks_voluntary_switch(
        &self,
        now: SemanticTimeMicros,
        pve_in_fight_until: Option<SemanticTimeMicros>,
    ) -> bool {
        self.blocks_logout(now, pve_in_fight_until)
    }
}

/// What recording an aggression did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggressionOutcome {
    /// New aggression: the actor was first; the PZ block applies.
    Started,
    /// The target was already an aggressor toward the actor (retaliation) or the relation is
    /// refreshed: no new PZ block.
    Retaliation,
    Refreshed,
    /// The set is full of unexpired relations.
    Refused,
}

#[derive(Debug, Clone, Copy)]
struct Relation<T> {
    target: T,
    until: SemanticTimeMicros,
    first: bool,
}

/// The characters one actor has aggression toward (§8.2), at most [`MAX_RELATIONS`].
#[derive(Debug, Clone)]
pub struct AggressionRelations<T> {
    relations: Vec<Relation<T>>,
}

impl<T> Default for AggressionRelations<T> {
    fn default() -> Self {
        Self {
            relations: Vec::new(),
        }
    }
}

impl<T: Copy + Eq> AggressionRelations<T> {
    pub fn has_aggression_toward(&self, target: T, now: SemanticTimeMicros) -> bool {
        self.relations
            .iter()
            .any(|r| r.target == target && r.until > now)
    }

    /// Whether the actor was first against `target` while that relation runs (yellow skull).
    pub fn was_first_against(&self, target: T, now: SemanticTimeMicros) -> bool {
        self.relations
            .iter()
            .any(|r| r.target == target && r.until > now && r.first)
    }

    /// Record aggression toward `target`; `target_is_aggressor` is whether the target already
    /// has aggression toward this actor. The oldest expired relation goes first.
    pub fn record(
        &mut self,
        target: T,
        target_is_aggressor: bool,
        now: SemanticTimeMicros,
    ) -> AggressionOutcome {
        let until = after(now, BLOCK_MS);
        if let Some(existing) = self.relations.iter_mut().find(|r| r.target == target) {
            let was_running = existing.until > now;
            existing.until = existing.until.max(until);
            // A running relation keeps its first-aggressor bit (yellow skull) while it runs;
            // only a lapsed retaliation (no counter-relation any more) becomes new aggression
            // (§8.1 PZ block). An expired relation restarts from the current counter-relation.
            return if !was_running {
                existing.first = !target_is_aggressor;
                if existing.first {
                    AggressionOutcome::Started
                } else {
                    AggressionOutcome::Retaliation
                }
            } else if existing.first {
                if target_is_aggressor {
                    AggressionOutcome::Retaliation
                } else {
                    AggressionOutcome::Refreshed
                }
            } else if target_is_aggressor {
                AggressionOutcome::Retaliation
            } else {
                existing.first = true;
                AggressionOutcome::Started
            };
        }
        if self.relations.len() >= MAX_RELATIONS {
            let oldest_expired = self
                .relations
                .iter()
                .enumerate()
                .filter(|(_, r)| r.until <= now)
                .min_by_key(|(_, r)| r.until)
                .map(|(i, _)| i);
            match oldest_expired {
                Some(i) => {
                    self.relations.swap_remove(i);
                }
                None => return AggressionOutcome::Refused,
            }
        }
        let first = !target_is_aggressor;
        self.relations.push(Relation {
            target,
            until,
            first,
        });
        if first {
            AggressionOutcome::Started
        } else {
            AggressionOutcome::Retaliation
        }
    }
}

#[cfg(test)]
mod readmission_clock_tests {
    use super::*;
    use crate::foundation::owner_timer::SemanticTimeMicros as OwnerTime;
    struct Clock {
        now: u64,
        samples: std::cell::Cell<u32>,
    }
    impl OwnerClock for Clock {
        fn now(&self) -> OwnerTime {
            self.samples.set(self.samples.get() + 1);
            OwnerTime::from_micros(self.now)
        }
    }
    fn remaining(values: [u64; 3]) -> PvpReadmissionRemaining {
        let [logout, pz, kill] = values.map(PvpRemainingMicros::for_test);
        PvpReadmissionRemaining { logout, pz, kill }
    }
    #[test]
    fn completed_duration_restore_samples_owner_once_and_expires_at_exact_boundary() {
        let clock = Clock {
            now: 7_000,
            samples: Default::default(),
        };
        let mut lock = CombatLock::default();
        assert!(matches!(
            lock.restore_pvp_readmission(Ok(Some(remaining([2_000, 3_000, 4_000]))), &clock),
            Ok(true)
        ));
        assert_eq!(clock.samples.get(), 1);
        assert!(lock.blocks_logout(SemanticTimeMicros::from_micros(10_999), None));
        assert!(!lock.blocks_logout(SemanticTimeMicros::from_micros(11_000), None));
        assert!(!lock.pz_block_running(SemanticTimeMicros::from_micros(10_000)));
        let before = lock;
        assert!(
            lock.restore_pvp_readmission(Ok(Some(remaining([1_000; 3]))), &clock)
                .is_ok()
        );
        assert_eq!(lock, before);
    }
    #[test]
    fn zero_and_absent_data_create_no_lock_and_never_shorten_existing_slots() {
        let clock = Clock {
            now: u64::MAX,
            samples: Default::default(),
        };
        let mut lock = CombatLock::default();
        assert!(matches!(
            lock.restore_pvp_readmission(Ok(None), &clock),
            Ok(false)
        ));
        assert_eq!(clock.samples.get(), 0);
        assert!(matches!(
            lock.restore_pvp_readmission(Ok(Some(remaining([0; 3]))), &clock),
            Ok(true)
        ));
        assert_eq!(lock, CombatLock::default());
        lock.on_kill(SemanticTimeMicros::from_micros(u64::MAX));
        let before = lock;
        assert!(
            lock.restore_pvp_readmission(Ok(Some(remaining([0; 3]))), &clock)
                .is_ok()
        );
        assert_eq!(lock, before);
    }
    #[test]
    fn later_slot_overflow_and_read_failure_preserve_all_previous_slots() {
        let clock = Clock {
            now: u64::MAX - 5,
            samples: Default::default(),
        };
        let mut lock = CombatLock::default();
        lock.on_pvp_hit(SemanticTimeMicros::from_micros(1));
        lock.on_aggression(SemanticTimeMicros::from_micros(1));
        lock.on_kill(SemanticTimeMicros::from_micros(9));
        let before = lock;
        assert!(matches!(
            lock.restore_pvp_readmission(Ok(Some(remaining([1, 2, 6]))), &clock),
            Err(PvpReadmissionError::DeadlineOverflow)
        ));
        assert_eq!(lock, before);
        assert!(matches!(
            lock.restore_pvp_readmission(Err(CharacterProgressionError::InvalidInput), &clock),
            Err(PvpReadmissionError::Read(_))
        ));
        assert_eq!(lock, before);
        assert_eq!(clock.samples.get(), 1);
    }
}
