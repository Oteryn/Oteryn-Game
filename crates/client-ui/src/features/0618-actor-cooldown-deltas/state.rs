use oteryn_client_runtime::spell::{ActionBar, ActorCooldownKind, ActorCooldownUpdate};
use std::{collections::BTreeSet, num::NonZeroU32, time::Instant};

#[derive(Debug, Clone, Default)]
pub struct ActionBarPanel {
    pub observed_spells: BTreeSet<u32>,
    pub open: bool,
    pub bar: ActionBar,
    pub feedback: String,
    pub cooldown_groups: BTreeSet<u32>,
}

impl ActionBarPanel {
    pub fn apply_cooldowns(&mut self, update: &ActorCooldownUpdate) {
        apply_cooldowns(self, update);
    }

    #[must_use]
    pub fn remaining_ms_at(&self, spell: NonZeroU32, groups: &[u32], now: Instant) -> u64 {
        self.bar.remaining_ms_at(spell, groups, now)
    }
}

pub(crate) fn apply_cooldowns(panel: &mut ActionBarPanel, update: &ActorCooldownUpdate) {
    panel.cooldown_groups.extend(
        update
            .entries
            .iter()
            .filter(|entry| entry.kind == ActorCooldownKind::Group)
            .map(|entry| entry.id),
    );
    panel.bar.apply_cooldowns(update);
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
