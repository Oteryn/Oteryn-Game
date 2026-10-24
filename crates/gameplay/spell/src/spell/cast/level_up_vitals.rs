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
        let health_gain = facts.max_health.checked_sub(self.facts.max_health)?;
        let mana_gain = facts.max_mana.checked_sub(self.facts.max_mana)?;
        let facts = CharacterCastFacts {
            magic_level: self.facts.magic_level,
            ..facts
        };
        // Use the same constructor to validate maxima and the level's base speed.
        let validated = Self::new(facts, 0, 0)?;
        let mut next = self.clone();
        next.facts = facts;
        next.base_speed = validated.base_speed;
        next.health = next
            .health
            .saturating_add(health_gain)
            .min(facts.max_health);
        next.mana = next.mana.saturating_add(mana_gain).min(facts.max_mana);
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
