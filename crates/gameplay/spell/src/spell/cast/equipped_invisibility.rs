use super::PlayerSpellState;

impl PlayerSpellState {
    /// Replace the worn invisibility with that of a fresh committed equipment read.
    pub fn install_equipped_invisibility(&mut self, invisible: Vec<[u8; 16]>) {
        self.equipped_invisible = invisible;
    }

    /// Durable sources used to project the currently worn condition durations.
    pub fn equipped_invisibility_sources(&self) -> &[[u8; 16]] {
        &self.equipped_invisible
    }

    /// Whether a worn item currently grants invisibility (not spell invisibility).
    #[must_use]
    pub fn equipped_invisible(&self) -> bool {
        !self.equipped_invisible.is_empty()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::spell::Vocation;
    use crate::spell::cast::CharacterCastFacts;

    #[test]
    fn equipped_invisibility_makes_owned_invisible_until_reinstalled_false() {
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
        assert_eq!(state.owned_invisible_at(0), Ok(false));
        assert!(!state.equipped_invisible());

        state.install_equipped_invisibility(vec![[1; 16]]);
        assert_eq!(state.owned_invisible_at(0), Ok(true));
        assert!(state.equipped_invisible());

        state.install_equipped_invisibility(vec![]);
        assert_eq!(state.owned_invisible_at(0), Ok(false));
        assert!(!state.equipped_invisible());
    }
}
