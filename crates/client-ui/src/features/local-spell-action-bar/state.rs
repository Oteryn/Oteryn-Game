use oteryn_client_runtime::spell::ActionBar;
#[derive(Debug, Clone, Default)]
pub struct ActionBarPanel {
    pub observed_spells: std::collections::BTreeSet<u32>,
    pub open: bool,
    pub bar: ActionBar,
    pub feedback: String,
    pub cooldown_durations: std::collections::BTreeMap<(bool, u32), (u64, u64)>,
    pub cooldown_groups: std::collections::BTreeSet<u32>,
    pub cooldown_received_at: Option<std::time::Instant>,
}

impl ActionBarPanel {
    pub fn replace_cooldowns(
        &mut self,
        cooldowns: Option<oteryn_client_runtime::spell::ActorCooldowns>,
    ) {
        if let Some(state) = &cooldowns {
            for entry in &state.entries {
                let group = entry.kind == oteryn_client_runtime::spell::ActorCooldownKind::Group;
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
                    entry.id == *id
                        && (entry.kind == oteryn_client_runtime::spell::ActorCooldownKind::Group)
                            == *group
                })
            });
        } else {
            self.cooldown_durations.clear();
        }
        self.bar.cooldowns = cooldowns;
        self.cooldown_received_at = Some(std::time::Instant::now());
    }
}
