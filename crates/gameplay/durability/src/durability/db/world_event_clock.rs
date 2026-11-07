//! A calendar fixture never changes PostgreSQL time or a production root.
use super::*;

impl DurabilityRoot {
    /// Isolated fixtures can exercise calendar-bound paths on any build date.
    /// Dedicated production roots reject this even in a test-support build.
    #[cfg(any(test, feature = "test-support"))]
    pub fn set_world_event_clock_for_test(
        &self,
        now_ms: Option<i64>,
    ) -> Result<(), DurabilityError> {
        if matches!(self.runtime, RootRuntime::Dedicated(_)) || now_ms.is_some_and(|at| at < 0) {
            return Err(DurabilityError::InvalidStoredState);
        }
        self.world_event_test_clock
            .as_ref()
            .ok_or(DurabilityError::InvalidStoredState)?
            .store(now_ms.unwrap_or(-1), Ordering::Release);
        Ok(())
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn world_event_clock_override_ms(&self) -> Option<i64> {
        let now = self
            .world_event_test_clock
            .as_ref()?
            .load(Ordering::Acquire);
        (now >= 0).then_some(now)
    }
    #[cfg(not(any(test, feature = "test-support")))]
    pub(crate) fn world_event_clock_override_ms(&self) -> Option<i64> {
        None
    }
}
