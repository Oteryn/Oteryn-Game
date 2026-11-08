//! Deterministic quest time belongs exclusively to a fixture root and its clones.
use super::{DurabilityError, DurabilityRoot, RootRuntime};
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

pub(super) fn new() -> Arc<AtomicI64> {
    Arc::new(AtomicI64::new(i64::MIN))
}

impl DurabilityRoot {
    fn fixture_quest_clock(&self) -> Option<&Arc<AtomicI64>> {
        match &self.runtime {
            RootRuntime::Dedicated(_) => None,
            RootRuntime::OwnedTestFixture { quest_clock, .. }
            | RootRuntime::AmbientTestFixture { quest_clock } => Some(quest_clock),
        }
    }

    pub(crate) fn quest_test_clock_handle(&self) -> Option<Arc<AtomicI64>> {
        self.fixture_quest_clock().cloned()
    }

    /// Install a fixed Unix time once on a fixture root. Production roots reject it.
    /// This affects the quest writer's predicates and SET_NOW, scoped to this root's clones.
    pub fn set_test_quest_clock(&self, unix_seconds: i64) -> Result<(), DurabilityError> {
        let clock = self
            .fixture_quest_clock()
            .ok_or(DurabilityError::InvalidStoredState)?;
        if unix_seconds < 0 {
            return Err(DurabilityError::InvalidStoredState);
        }
        clock
            .compare_exchange(i64::MIN, unix_seconds, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ())
            .map_err(|_| DurabilityError::InvalidStoredState)
    }

    /// Advance fixture time between completed commands; quest state changes only through its writer.
    pub fn advance_test_quest_clock(&self, seconds: i64) -> Result<i64, DurabilityError> {
        let clock = self
            .fixture_quest_clock()
            .ok_or(DurabilityError::InvalidStoredState)?;
        if seconds < 0 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let mut current = clock.load(Ordering::Acquire);
        loop {
            if current < 0 {
                return Err(DurabilityError::InvalidStoredState);
            }
            let next = current
                .checked_add(seconds)
                .ok_or(DurabilityError::InvalidStoredState)?;
            match clock.compare_exchange_weak(current, next, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => return Ok(next),
                Err(observed) => current = observed,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quest_test_clock_is_scoped_monotone_and_unavailable_to_production_roots()
    -> Result<(), Box<dyn std::error::Error>> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let _entered = runtime.enter();
        let root = DurabilityRoot::connect_test_runtime("postgresql://fixture@127.0.0.1/unused")?;
        let other = DurabilityRoot::connect_test_runtime("postgresql://fixture@127.0.0.1/unused")?;
        assert!(root.advance_test_quest_clock(1).is_err());
        assert!(root.set_test_quest_clock(-1).is_err());
        root.set_test_quest_clock(100)?;
        assert!(root.set_test_quest_clock(99).is_err());
        assert!(root.advance_test_quest_clock(-1).is_err());
        assert_eq!(root.clone().advance_test_quest_clock(20)?, 120);
        assert!(other.advance_test_quest_clock(1).is_err());
        other.set_test_quest_clock(i64::MAX)?;
        assert!(other.advance_test_quest_clock(1).is_err());
        assert_eq!(other.advance_test_quest_clock(0)?, i64::MAX);
        let mut production = root.clone();
        production.runtime = RootRuntime::Dedicated(tokio::runtime::Handle::current());
        assert!(production.quest_test_clock_handle().is_none());
        assert!(production.set_test_quest_clock(200).is_err());
        assert!(production.advance_test_quest_clock(1).is_err());
        assert_eq!(root.advance_test_quest_clock(0)?, 120);
        Ok(())
    }
}
