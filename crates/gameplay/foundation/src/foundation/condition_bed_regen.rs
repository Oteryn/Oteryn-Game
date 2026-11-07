//! Restore the durable food duration without treating restoration as another meal.
use super::{ConditionStore, ConflictKey, FOOD_REGENERATION_CAP_MS, MICROS_PER_MS};

impl<S: Clone> ConditionStore<S> {
    /// The caller has installed the native food definition. Restore its bounded
    /// duration exactly, including the cap that another meal must refuse.
    pub fn restore_bed_food_duration(&mut self, duration_ms: u32, now_us: u64) -> Option<()> {
        if duration_ms == 0 || duration_ms > FOOD_REGENERATION_CAP_MS {
            return None;
        }
        let end = now_us.checked_add(u64::from(duration_ms).checked_mul(MICROS_PER_MS)?)?;
        let tick = now_us.checked_add(1000 * MICROS_PER_MS)?;
        let food = self.instances.iter_mut().find(|food| {
            food.definition.condition_type().conflict_key() == ConflictKey::FoodRegeneration
        })?;
        food.ends_at = Some(end);
        food.next_tick_at = Some(tick);
        Some(())
    }
}
