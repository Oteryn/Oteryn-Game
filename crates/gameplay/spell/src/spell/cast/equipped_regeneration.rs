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
        let Some(revision) = self.revision.checked_add(1) else {
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
        if self.health == 0 {
            return;
        }
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

    /// Settle before a position or resting-tier transition, using the previous eligibility.
    pub fn settle_equipped_wear(&mut self, now_ms: u64, in_protection_zone: bool) {
        for index in 0..self.equipped_wear.len() {
            let wear = self.equipped_wear[index];
            self.settle_worn_regeneration(
                wear.item,
                wear.started_at_ms,
                now_ms.min(wear.deadline_ms),
                in_protection_zone,
            );
        }
    }

    /// A surviving ring must never spend time while the actor was dead on regeneration.
    pub(super) fn skip_dead_wear_through(&mut self, now_ms: u64) {
        for clock in &mut self.equipped_regeneration {
            if clock.worn.timed {
                clock.wear_at_ms = Some(clock.wear_at_ms.unwrap_or(0).max(now_ms));
            }
        }
    }

    pub fn retire_worn_effect(&mut self, item: [u8; 16]) {
        self.equipped_regeneration
            .retain(|clock| clock.worn.item != item);
        self.equipped_invisible.retain(|source| *source != item);
        self.equipped_wear.retain(|wear| wear.item != item);
    }

    #[must_use]
    pub fn equipped_regeneration(&self) -> &[EquipmentRegenerationClock] {
        &self.equipped_regeneration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spell::{Vocation, cast::CharacterCastFacts};
    use oteryn_gameplay_progression::regeneration::EquipmentRegeneration;
    use oteryn_simulation_determinism::SemanticTimeMicros;

    #[test]
    fn surviving_life_ring_skips_dead_wear_even_without_an_owner_tick() -> Result<(), String> {
        let mut state = PlayerSpellState::new(
            CharacterCastFacts {
                vocation: Vocation::Monk,
                level: 8,
                magic_level: 0,
                max_health: 100,
                max_mana: 100,
                max_soul: 100,
            },
            0,
            0,
        )
        .ok_or("player spell state")?;
        let item = [1; 16];
        state.install_equipped_regeneration(vec![WornRegeneration {
            slot: 8,
            item,
            timed: true,
            regeneration: EquipmentRegeneration {
                health_gain: 2,
                health_ticks_ms: 6_000,
                mana_gain: 8,
                mana_ticks_ms: 6_000,
            },
        }]);
        state.restore_saved_vitals(0, 0);
        state.settle_worn_regeneration(item, 0, 6_000, false);
        assert_eq!(state.equipped_regeneration()[0].wear_at_ms, Some(6_000));
        let mut state = state
            .respawned(SemanticTimeMicros::from_micros(12_000_000))
            .ok_or("respawn")?;
        state.restore_saved_vitals(25, 0);
        state.settle_worn_regeneration(item, 0, 13_000, false);
        assert_eq!((state.health, state.mana), (25, 0));
        state.settle_worn_regeneration(item, 0, 18_000, false);
        assert_eq!((state.health, state.mana), (27, 8));
        Ok(())
    }
}
