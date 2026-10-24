//! MELEE-SKILL-1: the live attack and shielding skills an auto-attack swing reads.

use super::PlayerSpellState;

/// The shielding skill's index in the seven owned skills.
const SHIELDING: usize = 5;

/// The live skills of one swing: the swung weapon's skill and the shielding skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwingSkills {
    pub attack: u32,
    pub shielding: u32,
}

impl PlayerSpellState {
    /// The live swing skills at `now_us`: the durable skills loaded at admission with the live
    /// condition adjustments. `Some(None)` when no durable build was loaded, which fresh
    /// admission refuses, so the caller keeps the ATTACK-1 starting skill. `None` (no swing) when
    /// `attack_index` is not a skill or a live adjustment does not resolve.
    #[must_use]
    pub fn swing_skills(&self, attack_index: usize, now_us: u64) -> Option<Option<SwingSkills>> {
        let Some(owned) = self.owned_skills else {
            return Some(None);
        };
        let adjustments = self.owned_skill_adjustments(now_us).ok()?;
        let attack =
            adjustments.adjust_skill(u32::from(*owned.get(attack_index)?), attack_index)?;
        let shielding = adjustments.adjust_skill(u32::from(owned[SHIELDING]), SHIELDING)?;
        Some(Some(SwingSkills { attack, shielding }))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::spell::Vocation;
    use crate::spell::cast::CharacterCastFacts;

    fn state() -> PlayerSpellState {
        PlayerSpellState::new(
            CharacterCastFacts {
                vocation: Vocation::Knight,
                level: 8,
                magic_level: 0,
                max_health: 185,
                max_mana: 90,
                max_soul: 100,
            },
            0,
            0,
        )
        .unwrap()
    }

    #[test]
    fn an_actor_without_a_loaded_build_keeps_the_starting_skill() {
        assert_eq!(state().swing_skills(2, 0), Some(None));
    }

    #[test]
    fn the_swing_reads_the_owned_weapon_and_shielding_skills() {
        let mut state = state();
        state.owned_skills = Some([11, 12, 42, 14, 15, 37, 10]);
        assert_eq!(
            state.swing_skills(2, 0),
            Some(Some(SwingSkills {
                attack: 42,
                shielding: 37
            }))
        );
        assert_eq!(state.swing_skills(0, 0).unwrap().unwrap().attack, 11);
        assert_eq!(state.swing_skills(7, 0), None, "no eighth skill");
    }
}
