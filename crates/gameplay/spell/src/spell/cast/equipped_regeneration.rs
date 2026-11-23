use super::PlayerSpellState;
use oteryn_gameplay_progression::regeneration::{EquipmentRegenerationClock, WornRegeneration};

impl PlayerSpellState {
    /// Replace the worn regeneration conditions with a fresh committed equipment read.
    /// Only the same occupant with the same params keeps its counters. Crystal creates
    /// a new condition when a different item enters the slot.
    pub fn install_equipped_regeneration(&mut self, worn: Vec<WornRegeneration>) {
        self.equipped_regeneration = worn
            .into_iter()
            .map(|worn| {
                match self
                    .equipped_regeneration
                    .iter()
                    .find(|clock| clock.worn == worn)
                    .copied()
                {
                    Some(clock) => clock,
                    None => EquipmentRegenerationClock {
                        worn,
                        wear_at_ms: None,
                        health_ms: 0,
                        mana_ms: 0,
                    },
                }
            })
            .collect();
    }

    /// Credit a timed item's settled interval and publish a vitals revision if it changed.
    pub fn settle_worn_regeneration(
        &mut self,
        item: [u8; 16],
        started_at_ms: u64,
        settled_until_ms: u64,
        in_protection_zone: bool,
    ) {
        let Some(revision) = self.revision.checked_add(1).filter(|_| self.health != 0) else {
            return;
        };
        let Some(clock) = self
            .equipped_regeneration
            .iter_mut()
            .find(|clock| clock.worn.item == item && clock.worn.timed)
        else {
            return;
        };
        let at = clock.wear_at_ms.unwrap_or(started_at_ms).max(started_at_ms);
        let elapsed_ms = settled_until_ms.saturating_sub(at);
        clock.wear_at_ms = Some(at.max(settled_until_ms));
        let streak = if self.premium_current {
            self.resting_streak
        } else {
            self.resting_streak.min(3)
        };
        let (health, mana) = clock.settle_wear(elapsed_ms, in_protection_zone, streak);
        let health = if health == 0 {
            self.health
        } else {
            self.health
                .saturating_add(health)
                .min(self.facts.max_health)
        };
        let mana = if mana == 0 {
            self.mana
        } else {
            self.mana.saturating_add(mana).min(self.facts.max_mana)
        };
        if (health, mana) != (self.health, self.mana) {
            self.health = health;
            self.mana = mana;
            self.revision = revision;
        }
    }

    pub fn retire_worn_effect(&mut self, item: [u8; 16]) {
        self.equipped_regeneration
            .retain(|clock| clock.worn.item != item);
        self.equipped_invisible.retain(|source| *source != item);
    }

    #[must_use]
    pub fn equipped_regeneration(&self) -> &[EquipmentRegenerationClock] {
        &self.equipped_regeneration
    }
}
