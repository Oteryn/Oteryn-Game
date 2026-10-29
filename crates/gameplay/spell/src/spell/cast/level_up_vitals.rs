//! LEVELUP-LIVE-1: install a committed level increase into the live actor.
use super::{CharacterCastFacts, PlayerSpellState};

impl PlayerSpellState {
    /// Canary `Player::addExperience` adds each level's maximum gain to the current pool too.
    /// The live magic level is kept: training owns it and may be ahead of the durable read.
    /// A dead actor, another class, a lower level or a shrinking pool is refused.
    pub fn after_level_up(&self, facts: CharacterCastFacts) -> Option<Self> {
        if facts.vocation != self.facts.vocation
            || facts.level <= self.facts.level
            || self.health == 0
        {
            return None;
        }
        let health_gain = facts.max_health.checked_sub(self.base_max_health)?;
        let mana_gain = facts.max_mana.checked_sub(self.base_max_mana)?;
        let facts = CharacterCastFacts {
            magic_level: self.facts.magic_level,
            ..facts
        };
        // Use the same constructor to validate maxima and the level's base speed.
        let validated = Self::new(facts, 0, 0)?;
        // The current Wheel contribution stays projected over the new base until the Wheel
        // owner reconciles it, so a level-up never clamps pools that the bonus covers.
        let wheel_health = self.facts.max_health.saturating_sub(self.base_max_health);
        let wheel_mana = self.facts.max_mana.saturating_sub(self.base_max_mana);
        let mut next = self.clone();
        next.base_max_health = facts.max_health;
        next.base_max_mana = facts.max_mana;
        next.facts = CharacterCastFacts {
            max_health: facts
                .max_health
                .saturating_add(wheel_health)
                .min(super::MAX_VITAL_POOL),
            max_mana: facts
                .max_mana
                .saturating_add(wheel_mana)
                .min(super::MAX_VITAL_POOL),
            ..facts
        };
        next.base_speed = validated.base_speed;
        next.health = next
            .health
            .saturating_add(health_gain)
            .min(next.facts.max_health);
        next.mana = next.mana.saturating_add(mana_gain).min(next.facts.max_mana);
        next.soul = next.soul.min(facts.max_soul);
        next.revision = next.revision.checked_add(1)?;
        Some(next)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use crate::spell::Vocation;

    fn knight(level: u32, max_health: u32, max_mana: u32) -> CharacterCastFacts {
        CharacterCastFacts {
            vocation: Vocation::Knight,
            level,
            magic_level: 3,
            max_health,
            max_mana,
            max_soul: 100,
        }
    }

    fn wounded(level: u32) -> PlayerSpellState {
        let mut state = PlayerSpellState::new(knight(level, 185, 90), 0, 0).expect("state");
        state.health = 100;
        state.mana = 40;
        state
    }

    #[test]
    fn level_gain_raises_maxima_current_pools_and_base_speed() {
        let before = wounded(8);
        let next = before
            .after_level_up(CharacterCastFacts {
                magic_level: 0,
                ..knight(9, 200, 95)
            })
            .expect("level up");
        assert_eq!(next.facts, knight(9, 200, 95));
        assert_eq!((next.health, next.mana), (115, 45));
        assert_eq!(next.base_speed, before.base_speed + 1);
        assert_eq!(next.revision, before.revision + 1);
    }

    #[test]
    fn full_pools_stay_full_after_several_levels() {
        let before = PlayerSpellState::new(knight(8, 185, 90), 0, 0).expect("state");
        let next = before
            .after_level_up(knight(11, 230, 105))
            .expect("level up");
        assert_eq!((next.health, next.mana), (230, 105));
        assert_eq!(next.base_speed, before.base_speed + 3);
    }

    #[test]
    fn wheel_maximum_replacement_clamps_losses_without_free_gains() {
        let mut state = wounded(8);
        state
            .apply_wheel_maximum_bonus(100, 50)
            .expect("wheel gain");
        assert_eq!((state.health, state.mana), (100, 40));
        assert_eq!(
            (state.vitals().max_health, state.vitals().max_mana),
            (285, 140)
        );
        state.health = 250;
        state.restore_saved_vitals(250, 120);
        state
            .apply_wheel_maximum_bonus(10, 5)
            .expect("wheel replacement");
        assert_eq!((state.health, state.mana), (195, 95));
        assert_eq!(
            (state.vitals().max_health, state.vitals().max_mana),
            (195, 95)
        );
    }

    #[test]
    fn level_up_rebases_while_retaining_current_wheel_projection() {
        let mut before = wounded(8);
        before
            .apply_wheel_maximum_bonus(100, 50)
            .expect("wheel gain");
        let mut next = before
            .after_level_up(knight(9, 200, 95))
            .expect("level up against base maxima");
        assert_eq!(
            (next.vitals().max_health, next.vitals().max_mana),
            (300, 145),
            "the Wheel projection survives the level-up before reconciliation"
        );
        next.apply_wheel_maximum_bonus(100, 50)
            .expect("reproject wheel");
        assert_eq!((next.health, next.mana), (115, 45));
        assert_eq!(
            (next.vitals().max_health, next.vitals().max_mana),
            (300, 145)
        );
    }

    #[test]
    fn full_pools_under_a_wheel_bonus_are_not_clamped_by_a_level_up() {
        let mut before = PlayerSpellState::new(knight(8, 185, 90), 0, 0).expect("state");
        before
            .apply_wheel_maximum_bonus(100, 50)
            .expect("wheel gain");
        before.restore_saved_vitals(285, 140);
        let next = before.after_level_up(knight(9, 200, 95)).expect("level up");
        assert_eq!((next.health, next.mana), (300, 145));
        assert_eq!(
            (next.vitals().max_health, next.vitals().max_mana),
            (300, 145)
        );
    }

    #[test]
    fn stale_dead_foreign_or_shrinking_installs_are_refused() {
        let before = wounded(8);
        assert!(before.after_level_up(knight(8, 185, 90)).is_none());
        assert!(before.after_level_up(knight(7, 170, 85)).is_none());
        assert!(before.after_level_up(knight(9, 180, 95)).is_none());
        assert!(before.after_level_up(knight(9, 200, 85)).is_none());
        assert!(
            before
                .after_level_up(CharacterCastFacts {
                    vocation: Vocation::Druid,
                    ..knight(9, 200, 95)
                })
                .is_none()
        );
        let mut dead = before;
        dead.health = 0;
        assert!(dead.after_level_up(knight(9, 200, 95)).is_none());
    }
}
