//! The actor side of the durable BED-REGEN runtime projection.
use super::cast::PlayerSpellState;
use oteryn_gameplay_domain::domain::condition::ConflictKey;

impl PlayerSpellState {
    /// Capture Soul, remaining food milliseconds and the two online fractional periods.
    /// All clocks here are semantic microseconds, as in the condition owner.
    #[must_use]
    pub fn bed_rest_snapshot(&self, now_us: u64) -> [u32; 4] {
        let food = self
            .conditions
            .get(ConflictKey::FoodRegeneration)
            .and_then(|food| food.remaining_duration_ms(now_us))
            .and_then(|ms| u32::try_from(ms).ok())
            .unwrap_or(0);
        [self.soul, food, self.regen_health_ms, self.regen_mana_ms]
    }

    /// Restore only on creation of a fresh actor; a reconnect retains its live state.
    pub fn restore_bed_rest(&mut self, rest: [u32; 4], now_us: u64) -> Option<()> {
        if rest[1] > 1_200_000 || rest[2] > 12_000 || rest[3] > 6_000 {
            return None;
        }
        if rest[1] > 0 {
            super::actor_conditions::restore_bed_food(self, rest[1], now_us / 1000)?;
        }
        self.soul = rest[0].min(self.facts.max_soul);
        self.regen_health_ms = rest[2];
        self.regen_mana_ms = rest[3];
        Some(())
    }
}
#[cfg(test)]
#[path = "bed_regen_tests.rs"]
mod tests;
