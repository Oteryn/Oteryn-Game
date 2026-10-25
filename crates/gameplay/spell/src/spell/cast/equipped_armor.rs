//! ARMOR-MITIGATION-1 / SHIELD-DEFENCE-1: the worn armor total and the hands' defence a
//! creature hit on the player reads.

use super::PlayerSpellState;
use oteryn_gameplay_combat_rules::resolution::player_defence::HandDefence;

impl PlayerSpellState {
    /// The armor of the worn equipment (`Player::getArmor`), installed at admission and after
    /// every committed equipment move; 0 before any install.
    #[must_use]
    pub const fn equipped_armor(&self) -> u32 {
        self.equipped_armor
    }

    /// Replace the worn armor with the total of a fresh committed equipment read.
    pub fn install_equipped_armor(&mut self, armor: u32) {
        self.equipped_armor = armor;
    }

    /// What the hands defend with (`Player::getShieldAndWeapon`), installed with the armor;
    /// `None` before any install or while a held item's weapon facts are not known.
    #[must_use]
    pub const fn equipped_defence(&self) -> Option<HandDefence> {
        self.equipped_defence
    }

    /// Replace the hands' defence with that of a fresh committed equipment read.
    pub fn install_equipped_defence(&mut self, defence: Option<HandDefence>) {
        self.equipped_defence = defence;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::spell::Vocation;
    use crate::spell::cast::CharacterCastFacts;

    #[test]
    fn a_fresh_actor_wears_no_armor_until_an_install_replaces_it() {
        let mut state = PlayerSpellState::new(
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
        .unwrap();
        let revision = state.revision();
        assert_eq!(state.equipped_armor(), 0);
        state.install_equipped_armor(17);
        assert_eq!(state.equipped_armor(), 17);
        state.install_equipped_armor(4);
        assert_eq!(state.equipped_armor(), 4, "an install replaces, never adds");
        assert_eq!(state.revision(), revision, "armor is no vitals write");
    }
}
