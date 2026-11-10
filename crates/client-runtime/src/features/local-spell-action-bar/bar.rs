//! Local spell row; server cast indices are never display row numbers.
use std::num::NonZeroU32;
pub const ACTION_BAR_SLOTS: usize = 150;
#[derive(Debug, Clone)]
pub struct ActionBar {
    slots: [Option<NonZeroU32>; ACTION_BAR_SLOTS],
    pub cooldowns: Option<oteryn_session::ActorCooldowns>,
}
impl Default for ActionBar {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|slot| {
                if slot < 4 {
                    NonZeroU32::new(slot as u32 + 1)
                } else {
                    None
                }
            }),
            cooldowns: None,
        }
    }
}
impl ActionBar {
    #[must_use]
    pub fn spell(&self, slot: NonZeroU32) -> Option<NonZeroU32> {
        self.slots
            .get(usize::try_from(slot.get() - 1).ok()?)
            .copied()
            .flatten()
    }
    pub fn bind(&mut self, slot: usize, spell: Option<NonZeroU32>) -> bool {
        if spell.is_some_and(|id| id.get() > (1 << 28) - 1) {
            return false;
        }
        let Some(value) = self.slots.get_mut(slot) else {
            return false;
        };
        *value = spell;
        true
    }
    pub fn swap(&mut self, from: usize, to: usize) -> bool {
        if from >= self.slots.len() || to >= self.slots.len() {
            return false;
        }
        self.slots.swap(from, to);
        true
    }
    #[must_use]
    pub fn fields(&self) -> Vec<String> {
        self.slots
            .iter()
            .map(|id| id.map_or_else(String::new, |id| id.to_string()))
            .collect()
    }
    #[must_use]
    pub fn remaining_ms(&self, spell: NonZeroU32, groups: &[u32]) -> u64 {
        use oteryn_session::ActorCooldownKind;
        self.cooldowns.as_ref().map_or(0, |state| {
            state
                .entries
                .iter()
                .filter(|entry| match entry.kind {
                    ActorCooldownKind::Spell => entry.id == spell.get(),
                    ActorCooldownKind::Group => groups.contains(&entry.id),
                })
                .map(|entry| entry.expires_at_ms.saturating_sub(state.server_now_ms))
                .max()
                .unwrap_or(0)
        })
    }
}

#[cfg(test)]
#[path = "tests/bar.rs"]
mod tests;
