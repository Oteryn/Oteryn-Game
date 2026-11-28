//! A separate projection of server cooldowns, independent of action-bar assignments.
use super::super::ActionBarPanel;
use oteryn_client_runtime::spell_catalog::CatalogState;
use std::time::Instant;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CooldownIcon {
    pub id: u32,
    pub group: bool,
    pub label: String,
    /// Catalogue details; never used as a cast or timer authority.
    pub tooltip: String,
    pub remaining_ms: u64,
    /// White overlay length, from full (1000) to expired (0).
    pub overlay_per_mille: u16,
}

#[must_use]
pub fn cooldown_icons(panel: &ActionBarPanel, catalog: &CatalogState) -> Vec<CooldownIcon> {
    cooldown_icons_at(panel, catalog, Instant::now())
}

#[must_use]
pub fn cooldown_icons_at(
    panel: &ActionBarPanel,
    catalog: &CatalogState,
    now: Instant,
) -> Vec<CooldownIcon> {
    let mut groups = panel.cooldown_groups.clone();
    groups.extend(
        catalog
            .entries()
            .iter()
            .flat_map(|entry| entry.cooldown_groups.iter().copied()),
    );
    let icon = |id, group| {
        let (remaining_ms, duration) = panel.bar.cooldown_at(group, id, now);
        CooldownIcon {
            id,
            group,
            remaining_ms,
            tooltip: String::new(),
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
    icons.extend(
        panel
            .bar
            .spell_cooldown_keys()
            .map(|id| icon(id, false))
            .filter(|entry| entry.remaining_ms > 0),
    );
    icons
}
