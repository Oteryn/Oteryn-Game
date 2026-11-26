use oteryn_client_runtime::spell::{ActionBar, ActorCooldownKind, ActorCooldowns};
use std::time::Instant;
#[derive(Debug, Clone, Default)]
pub struct ActionBarPanel {
    pub observed_spells: std::collections::BTreeSet<u32>,
    pub open: bool,
    pub bar: ActionBar,
    pub feedback: String,
    pub cooldown_durations: std::collections::BTreeMap<(bool, u32), (u64, u64)>,
    pub cooldown_groups: std::collections::BTreeSet<u32>,
    pub cooldown_received_at: Option<Instant>,
}

impl ActionBarPanel {
    /// Domain 20 deltas replace individual keys. Zero expiries remove them.
    pub fn apply_cooldown_delta(&mut self, delta: &ActorCooldowns) {
        let received = Instant::now();
        let half_rtt = |state: &ActorCooldowns| u64::from(state.rtt_ms.unwrap_or(0)) / 2;
        let mut server_now_ms = delta.server_now_ms.saturating_add(half_rtt(delta));
        let mut entries = std::collections::BTreeMap::new();
        if let Some(current) = &self.bar.cooldowns {
            // The first measured clock replaces the provisional one. Later
            // samples keep the estimate with the least observed delivery delay.
            if (current.rtt_ms.is_some() || delta.rtt_ms.is_none())
                && let Some(previous) = self.cooldown_received_at
            {
                let elapsed = u64::try_from(received.duration_since(previous).as_millis())
                    .unwrap_or(u64::MAX);
                server_now_ms = server_now_ms.max(
                    current
                        .server_now_ms
                        .saturating_add(half_rtt(current))
                        .saturating_add(elapsed),
                );
            }
            entries.extend(
                current
                    .entries
                    .iter()
                    .filter(|entry| entry.expires_at_ms > server_now_ms)
                    .map(|entry| ((entry.kind, entry.id), *entry)),
            );
        }
        for &entry in &delta.entries {
            let key = (entry.kind, entry.id);
            if entry.expires_at_ms > server_now_ms {
                entries.insert(key, entry);
            } else {
                entries.remove(&key);
            }
        }
        self.replace_cooldowns(Some(ActorCooldowns {
            server_now_ms: server_now_ms.saturating_sub(half_rtt(delta)),
            rtt_ms: delta.rtt_ms,
            entries: entries.into_values().collect(),
        }));
        self.cooldown_received_at = Some(received);
    }

    pub fn replace_cooldowns(&mut self, cooldowns: Option<ActorCooldowns>) {
        if let Some(state) = &cooldowns {
            for entry in &state.entries {
                let group = entry.kind == ActorCooldownKind::Group;
                if group {
                    self.cooldown_groups.insert(entry.id);
                }
                let observed = self
                    .cooldown_durations
                    .entry((group, entry.id))
                    .or_default();
                if observed.0 != entry.expires_at_ms {
                    *observed = (
                        entry.expires_at_ms,
                        entry.expires_at_ms.saturating_sub(state.server_now_ms),
                    );
                }
            }
            self.cooldown_durations.retain(|(group, id), _| {
                state.entries.iter().any(|entry| {
                    entry.id == *id && (entry.kind == ActorCooldownKind::Group) == *group
                })
            });
        } else {
            self.cooldown_durations.clear();
        }
        self.bar.cooldowns = cooldowns;
        self.cooldown_received_at = Some(Instant::now());
    }
}
