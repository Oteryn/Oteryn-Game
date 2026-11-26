use oteryn_client_runtime::spell::{ActionBar, ActorCooldownKind, ActorCooldownUpdate};
use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU32,
    time::Instant,
};

#[derive(Debug, Clone, Default)]
pub struct ActionBarPanel {
    pub observed_spells: BTreeSet<u32>,
    pub open: bool,
    pub bar: ActionBar,
    pub feedback: String,
    pub cooldown_groups: BTreeSet<u32>,
    pub(super) cooldown_timers: BTreeMap<(bool, u32), CooldownTimer>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct CooldownTimer {
    received_at: Instant,
    expires_at_ms: u64,
    remaining_ms: u64,
    duration_ms: u64,
}

impl CooldownTimer {
    fn remaining_at(self, now: Instant) -> u64 {
        let elapsed = now.saturating_duration_since(self.received_at).as_millis();
        self.remaining_ms
            .saturating_sub(u64::try_from(elapsed).unwrap_or(u64::MAX))
    }
}

impl ActionBarPanel {
    pub fn apply_cooldowns(&mut self, update: &ActorCooldownUpdate) {
        let previous = if update.snapshot {
            std::mem::take(&mut self.cooldown_timers)
        } else {
            BTreeMap::new()
        };
        for entry in &update.entries {
            let group = entry.kind == ActorCooldownKind::Group;
            if group {
                self.cooldown_groups.insert(entry.id);
            }
            let key = (group, entry.id);
            let remaining_ms = entry.expires_at_ms.saturating_sub(update.server_now_ms);
            if remaining_ms == 0 {
                self.cooldown_timers.remove(&key);
                continue;
            }
            let duration_ms = previous
                .get(&key)
                .filter(|old| old.expires_at_ms == entry.expires_at_ms)
                .map_or(remaining_ms, |old| old.duration_ms.max(remaining_ms));
            self.cooldown_timers.insert(
                key,
                CooldownTimer {
                    received_at: update.received_at,
                    expires_at_ms: entry.expires_at_ms,
                    remaining_ms,
                    duration_ms,
                },
            );
        }
        self.bar.cooldowns = Some(update.value.clone());
    }

    pub(super) fn cooldown_at(&self, group: bool, id: u32, now: Instant) -> (u64, u64) {
        self.cooldown_timers
            .get(&(group, id))
            .map_or((0, 0), |timer| (timer.remaining_at(now), timer.duration_ms))
    }

    #[must_use]
    pub fn remaining_ms_at(&self, spell: NonZeroU32, groups: &[u32], now: Instant) -> u64 {
        groups
            .iter()
            .map(|id| self.cooldown_at(true, *id, now).0)
            .fold(self.cooldown_at(false, spell.get(), now).0, u64::max)
    }
}
