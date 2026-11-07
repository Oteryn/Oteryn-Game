//! DATA for the three pinned Heart of Destruction Outburst globals.
//! Authority and source actor qualification stay with the native callback owner.
#[derive(Debug)]
pub(super) struct OutburstState {
    stage: i64,
    health: i64,
    charging_killed: i64,
    sample: Option<oteryn_gameplay_entities::NativeNullHealthSample>,
}
impl Default for OutburstState {
    fn default() -> Self {
        Self {
            stage: -1,
            health: -1,
            charging_killed: -1,
            sample: None,
        }
    }
}
impl OutburstState {
    pub(super) fn source_health(&self) -> u64 {
        u64::try_from(self.health).unwrap_or(0)
    }
    pub(super) fn source_stage(&self) -> u8 {
        u8::try_from(self.stage).unwrap_or(0)
    }
    pub(super) fn reset_entry(&mut self) {
        self.stage = 0;
        self.health = 290000;
        self.sample = None;
    }
    pub(super) fn observe_health(&mut self, health: u64) -> Option<()> {
        let health = i64::try_from(health).ok()?;
        self.health = health;
        self.sample = None;
        Some(())
    }
    pub(super) fn source_health_sample(
        &self,
    ) -> Option<&oteryn_gameplay_entities::NativeNullHealthSample> {
        self.sample.as_ref()
    }
    pub(super) fn observe_health_sample(
        &mut self,
        sample: oteryn_gameplay_entities::NativeNullHealthSample,
    ) -> Option<()> {
        self.observe_health(u64::try_from(sample.health()).ok()?)?;
        self.sample = Some(sample);
        Some(())
    }
    pub(super) fn reset_entry_sample(
        &mut self,
        sample: oteryn_gameplay_entities::NativeNullHealthSample,
    ) -> Option<()> {
        if sample.health() != 290000 {
            return None;
        }
        self.reset_entry();
        self.sample = Some(sample);
        Some(())
    }
    pub(super) fn advance_phase(&mut self, phase: u8) -> Option<()> {
        if phase > 4 || phase != self.source_stage().checked_add(1)? {
            return None;
        }
        self.stage = i64::from(phase);
        self.charging_killed = -1;
        Some(())
    }
    pub(super) fn mark_exploded(&mut self) {
        self.charging_killed = 1;
    }
    #[allow(
        dead_code,
        reason = "pinned ChargingOutDeath restore predicate awaits the copied source-death production leaf"
    )]
    pub(super) fn charging_death_should_restore(&self) -> bool {
        self.charging_killed < 1
    }
}
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn outburst_source_stored_default_and_effective_think_stage_are_distinct() {
        let state = OutburstState::default();
        assert_eq!(
            state.stage, -1,
            "Game.getStorageValue keeps the actual unset global value"
        );
        assert_eq!(
            state.source_stage(),
            0,
            "OutburstCharge.lua18 explicitly normalizes only its local think stage"
        );
        assert_eq!(
            state.stage, -1,
            "reading local normalization must not perform the lever's global mutation"
        );
    }
    #[test]
    fn outburst_source_health_is_shared_with_phases_and_never_copied_from_charging_form() {
        let mut state = OutburstState::default();
        assert_eq!(state.source_health(), 0);
        state.reset_entry();
        assert_eq!(state.source_health(), 290000);
        assert_eq!(state.source_stage(), 0);
        state.observe_health(220001).expect("actual Outburst HP");
        state.advance_phase(1).expect("source first phase");
        assert_eq!(state.source_health(), 220001);
        assert!(state.charging_death_should_restore());
        state.mark_exploded();
        assert!(!state.charging_death_should_restore());
        assert_eq!(state.source_health(), 220001);
        state.reset_entry();
        assert_eq!(state.source_health(), 290000);
        assert!(
            !state.charging_death_should_restore(),
            "lever does not reset killed storage"
        );
        state
            .advance_phase(1)
            .expect("source first phase resets killed");
        assert!(state.charging_death_should_restore());
    }
    #[test]
    fn outburst_phase_advance_is_ordered_and_preserves_health() {
        let mut state = OutburstState::default();
        state.observe_health(159999).expect("source health");
        assert!(state.advance_phase(2).is_none());
        assert_eq!(state.source_stage(), 0);
        state.advance_phase(1).expect("first source threshold");
        state.advance_phase(2).expect("second source threshold");
        assert!(state.advance_phase(2).is_none());
        assert_eq!(state.source_health(), 159999);
        assert!(state.observe_health(u64::MAX).is_none());
        assert_eq!(state.source_health(), 159999);
    }
}
