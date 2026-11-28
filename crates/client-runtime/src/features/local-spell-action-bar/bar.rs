//! Local spell row; server cast indices are never display row numbers.
use super::{ActorCooldownKind, ActorCooldownUpdate, ActorCooldowns};
use std::{collections::BTreeMap, num::NonZeroU32, time::Instant};
pub const ACTION_BAR_SLOTS: usize = 450;
#[derive(Debug, Clone)]
pub struct ActionBar {
    slots: [Option<NonZeroU32>; ACTION_BAR_SLOTS],
    // Latest wire sample, retained for observation metadata.
    cooldowns: Option<ActorCooldowns>,
    cooldown_timers: BTreeMap<(bool, u32), CooldownTimer>,
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
            cooldown_timers: BTreeMap::new(),
        }
    }
}
#[derive(Debug, Clone, Copy)]
struct CooldownTimer {
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
    pub fn apply_cooldowns(&mut self, update: &ActorCooldownUpdate) {
        let previous = if update.snapshot {
            std::mem::take(&mut self.cooldown_timers)
        } else {
            BTreeMap::new()
        };
        for entry in &update.entries {
            let group = entry.kind == ActorCooldownKind::Group;
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
        self.cooldowns = Some(update.value.clone());
    }

    #[must_use]
    pub fn last_cooldown_observation(&self) -> Option<&ActorCooldowns> {
        self.cooldowns.as_ref()
    }

    pub fn spell_cooldown_keys(&self) -> impl Iterator<Item = u32> + '_ {
        self.cooldown_timers
            .keys()
            .filter_map(|&(group, id)| (!group).then_some(id))
    }

    pub fn cooldown_at(&self, group: bool, id: u32, now: Instant) -> (u64, u64) {
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

#[cfg(test)]
#[path = "tests/bar.rs"]
mod tests;
