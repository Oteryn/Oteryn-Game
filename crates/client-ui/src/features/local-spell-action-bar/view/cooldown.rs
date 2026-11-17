//! A separate projection of server cooldowns, independent of action-bar assignments.
use super::super::ActionBarPanel;
use oteryn_client_runtime::{spell::ActorCooldownKind, spell_catalog::CatalogState};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CooldownIcon {
    pub id: u32,
    pub group: bool,
    pub label: String,
    pub remaining_ms: u64,
    /// White overlay length, from full (1000) to expired (0).
    pub overlay_per_mille: u16,
}

#[must_use]
pub fn cooldown_icons(panel: &ActionBarPanel, catalog: &CatalogState) -> Vec<CooldownIcon> {
    let elapsed = panel.cooldown_received_at.map_or(0, |received| {
        u64::try_from(received.elapsed().as_millis()).unwrap_or(u64::MAX)
    });
    let mut groups = panel.cooldown_groups.clone();
    groups.extend(
        catalog
            .entries()
            .iter()
            .flat_map(|entry| entry.cooldown_groups.iter().copied()),
    );
    let entries = panel.bar.cooldowns.as_ref();
    let icon = |id, group| {
        let duration = entries.map_or(0, |state| {
            state
                .entries
                .iter()
                .filter(|entry| entry.id == id && (entry.kind == ActorCooldownKind::Group) == group)
                .map(|entry| entry.expires_at_ms.saturating_sub(state.server_now_ms))
                .max()
                .unwrap_or(0)
        });
        let remaining_ms = duration.saturating_sub(elapsed);
        let duration = panel
            .cooldown_durations
            .get(&(group, id))
            .map_or(duration, |(_, duration)| *duration);
        CooldownIcon {
            id,
            group,
            remaining_ms,
            label: if group {
                // S9 ordinals from tools/content-schema/spell-authoring/cooldown-groups.json.
                [
                    "Attack",
                    "Healing",
                    "Support",
                    "Special",
                    "Bursts of Nature",
                    "Crippling",
                    "Focus",
                    "Great Beams",
                    "Stance",
                    "Ultimate Strikes",
                    "Virtue",
                ]
                .get(id.saturating_sub(1) as usize)
                .map_or_else(|| format!("Group {id}"), |name| (*name).to_owned())
            } else {
                catalog
                    .entries()
                    .iter()
                    .find(|entry| entry.cast_index == id)
                    .map_or_else(|| format!("Spell {id}"), |entry| entry.name.clone())
            },
            overlay_per_mille: if duration == 0 {
                0
            } else {
                ((u128::from(remaining_ms) * 1000) / u128::from(duration)) as u16
            },
        }
    };
    let mut icons: Vec<_> = groups.into_iter().map(|id| icon(id, true)).collect();
    if let Some(state) = entries {
        icons.extend(
            state
                .entries
                .iter()
                .filter(|entry| entry.kind == ActorCooldownKind::Spell)
                .map(|entry| icon(entry.id, false))
                .filter(|entry| entry.remaining_ms > 0),
        );
    }
    icons
}
