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

#[allow(
    unused_imports,
    reason = "path-loaded readmission fixtures use only the combat-lock adapter"
)]
pub use oteryn_gameplay_combat_rules::pvp::lock::{
    AggressionOutcome, AggressionRelations, KILL_BLOCK_MS, MAX_RELATIONS,
};

/// Runtime readmission adapter for the pure monotone PvP combat lock.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CombatLock(oteryn_gameplay_combat_rules::pvp::lock::CombatLock);

impl std::ops::Deref for CombatLock {
    type Target = oteryn_gameplay_combat_rules::pvp::lock::CombatLock;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for CombatLock {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
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
