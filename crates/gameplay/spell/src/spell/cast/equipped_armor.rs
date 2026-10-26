//! ARMOR-MITIGATION-1 / SHIELD-DEFENCE-1 / EQUIP-PROTECTION-1: the worn armor total, the
//! hands' defence and the worn physical protection a creature hit on the player reads.

use super::PlayerSpellState;
use oteryn_gameplay_combat_rules::resolution::equipped_offence::EquippedOffence;
use oteryn_gameplay_combat_rules::resolution::player_defence::HandDefence;
use oteryn_gameplay_content::content::ReferenceRationalPercent;

impl PlayerSpellState {
    /// Install the equipment owner's current active-form mana shield contribution.
    pub fn install_equipped_mana_shield(&mut self, active: bool) {
        self.equipped_mana_shield = active;
    }

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

    /// The summed physical protection of the worn equipment (EQUIP-0 §3.3), installed with the
    /// armor; `None` before any install or while nothing worn protects against physical damage.
    #[must_use]
    pub const fn equipped_physical_protection(&self) -> Option<ReferenceRationalPercent> {
        self.equipped_physical_protection
    }

    /// Replace the physical protection with that of a fresh committed equipment read.
    pub fn install_equipped_physical_protection(
        &mut self,
        protection: Option<ReferenceRationalPercent>,
    ) {
        self.equipped_physical_protection = protection;
    }

    /// PARITY-EQUIP-MODIFIERS-1: the critical hit and leech of the worn equipment, installed with
    /// the armor; zero before any install.
    #[must_use]
    pub fn equipped_offence(&self) -> EquippedOffence {
        self.equipped_offence.clone()
    }

    /// Replace the offence with that of a fresh committed equipment read.
    pub fn install_equipped_offence(&mut self, offence: EquippedOffence) {
        self.equipped_offence = offence;
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
        assert_eq!(state.equipped_physical_protection(), None);
        let protection = ReferenceRationalPercent {
            numerator: 7,
            denominator: 1,
        };
        state.install_equipped_physical_protection(Some(protection));
        assert_eq!(state.equipped_physical_protection(), Some(protection));
        state.install_equipped_physical_protection(None);
        assert_eq!(state.equipped_physical_protection(), None);
        assert_eq!(state.equipped_offence(), EquippedOffence::default());
        let offence = EquippedOffence {
            critical_chance: 500,
            critical_damage: 2500,
            life_leech: 1800,
            mana_leech: 100,
            ..EquippedOffence::default()
        };
        state.install_equipped_offence(offence.clone());
        assert_eq!(state.equipped_offence(), offence);
        assert_eq!(state.revision(), revision, "armor is no vitals write");
    }

    #[test]
    fn a_leech_restores_up_to_the_maxima_in_one_vitals_write() {
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
        assert_eq!(
            state.after_leech(10, 10),
            None,
            "full vitals restore nothing"
        );
        state.health = 100;
        state.mana = 85;
        let next = state.after_leech(20, 10).unwrap();
        assert_eq!((next.vitals().health, next.vitals().mana), (120, 90));
        assert_eq!(next.revision(), state.revision() + 1);
        assert_eq!(state.after_leech(0, 0), None);
        state.health = 0;
        assert_eq!(state.after_leech(20, 10), None, "the dead leech nothing");
    }
}
