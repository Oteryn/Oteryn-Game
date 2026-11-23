use super::PlayerSpellState;

/// One qualified timed slot on the Channel owner's monotonic clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquippedWear {
    pub item: [u8; 16],
    pub started_at_ms: u64,
    pub deadline_ms: u64,
}

impl PlayerSpellState {
    pub fn install_equipped_wear(&mut self, wear: Vec<EquippedWear>) {
        self.equipped_wear = wear;
    }

    pub fn equipped_remaining_ms(&self, item: &[u8; 16], now_us: u64) -> Option<i64> {
        let deadline = self
            .equipped_wear
            .iter()
            .find(|wear| wear.item == *item)?
            .deadline_ms;
        let remaining = deadline
            .saturating_mul(1_000)
            .checked_sub(now_us)?
            .div_ceil(1_000);
        i64::try_from(remaining)
            .ok()
            .filter(|remaining| *remaining > 0)
    }

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
    pub fn equipped_invisible(&self, now_us: u64) -> bool {
        self.equipped_invisible
            .iter()
            .any(|item| self.equipped_remaining_ms(item, now_us).is_some())
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
        assert!(!state.equipped_invisible(0));

        state.install_equipped_invisibility(vec![[1; 16]]);
        assert_eq!(state.owned_invisible_at(0), Ok(false));
        state.install_equipped_wear(vec![EquippedWear {
            item: [1; 16],
            started_at_ms: 0,
            deadline_ms: 1,
        }]);
        assert_eq!(state.owned_invisible_at(0), Ok(true));
        assert_eq!(state.owned_invisible_at(1_000), Ok(false));
        assert!(state.equipped_invisible(0));

        state.install_equipped_invisibility(vec![]);
        assert_eq!(state.owned_invisible_at(0), Ok(false));
        assert!(!state.equipped_invisible(0));
    }
}
