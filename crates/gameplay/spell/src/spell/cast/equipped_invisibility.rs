use super::PlayerSpellState;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

/// One qualified timed slot on the Channel owner's monotonic clock.
#[derive(Debug, Clone)]
pub struct EquippedWear {
    pub item: [u8; 16],
    pub started_at_ms: u64,
    pub deadline_ms: u64,
    /// Shared with the durable lane, including while a prepared actor state is parked.
    pending_until_ms: Arc<AtomicU64>,
}

// The live write gate is shared by snapshots and does not change their identity.
impl PartialEq for EquippedWear {
    fn eq(&self, other: &Self) -> bool {
        self.item == other.item
            && self.started_at_ms == other.started_at_ms
            && self.deadline_ms == other.deadline_ms
    }
}

impl Eq for EquippedWear {}

impl EquippedWear {
    pub fn new(
        item: [u8; 16],
        started_at_ms: u64,
        deadline_ms: u64,
        pending_until_ms: Arc<AtomicU64>,
    ) -> Self {
        Self {
            item,
            started_at_ms,
            deadline_ms,
            pending_until_ms,
        }
    }

    pub(super) fn suspended(&self, now_ms: u64) -> bool {
        let pending_until = self.pending_until_ms.load(Ordering::Acquire);
        pending_until != 0 && now_ms >= pending_until
    }
}

impl PlayerSpellState {
    pub fn install_equipped_wear(&mut self, wear: Vec<EquippedWear>) {
        self.equipped_wear = wear;
    }

    pub fn equipped_remaining_ms(&self, item: &[u8; 16], now_us: u64) -> Option<i64> {
        let wear = self.equipped_wear.iter().find(|wear| wear.item == *item)?;
        if wear.suspended(now_us / 1_000) {
            return None;
        }
        let remaining = wear
            .deadline_ms
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
        state.install_equipped_wear(vec![EquippedWear::new([1; 16], 0, 1, Arc::default())]);
        assert_eq!(state.owned_invisible_at(0), Ok(true));
        assert_eq!(state.owned_invisible_at(1_000), Ok(false));
        assert!(state.equipped_invisible(0));

        state.install_equipped_invisibility(vec![]);
        assert_eq!(state.owned_invisible_at(0), Ok(false));
        assert!(!state.equipped_invisible(0));
    }
}
