//! Local gameplay choices consume the same capability-gated callers as the game controls.
use crate::play::{PlayLink, PlayView};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GameplayPreferences {
    pub allow_inspection: bool,
    pub loot_click: u8,
    pub sort_recursive: bool,
    pub warn_recursive_sort: bool,
    pub inspect_context_menu: bool,
    pub cancel_chase_on_walk: bool,
    pub secondary_click_loots_nearby: bool,
}

impl Default for GameplayPreferences {
    fn default() -> Self {
        Self {
            allow_inspection: false,
            loot_click: 0,
            sort_recursive: false,
            warn_recursive_sort: true,
            inspect_context_menu: true,
            cancel_chase_on_walk: false,
            secondary_click_loots_nearby: false,
        }
    }
}

impl PlayView {
    pub(in crate::play) fn cancel_chase_for_walk(&self, link: &PlayLink) {
        if !self.preferences().gameplay.cancel_chase_on_walk {
            return;
        }
        if let Some(state) = link.battle_panel().combat
            && state.modes.chase == oteryn_session::ChaseMode::Chase
        {
            link.request_combat(crate::combat_input::CombatCommand::Modes(
                oteryn_session::FightModes {
                    chase: oteryn_session::ChaseMode::Stand,
                    ..state.modes
                },
            ));
        }
    }

    #[cfg(any(test, bevy_shell))]
    pub(crate) fn loot_click(
        &self,
        link: &PlayLink,
        tile: oteryn_renderer::TileCoord,
        secondary: bool,
        shortcut: Option<oteryn_input_actions::Modifier>,
    ) -> bool {
        if !matches!(
            (self.preferences().gameplay.loot_click, secondary, shortcut),
            (1, true, None)
                | (2, true, Some(oteryn_input_actions::Modifier::Shift))
                | (3, false, None)
        ) {
            return false;
        }
        if self.gameplay_input_blocked(link) || self.cursor_spell.is_some() {
            return false;
        }
        {
            let mailbox = crate::play::mailbox::lock(&link.mailbox);
            if mailbox.world_context.targeting_action().is_some()
                || mailbox.use_with_pending
                || mailbox.hireling_picking
                || !mailbox.discovery.loot.selected
            {
                return false;
            }
        }
        let Some(entity) = self.entities.iter().rev().find(|row| {
            row.position.x == tile.x && row.position.y == tile.y && row.position.floor == self.floor
        }) else {
            return false;
        };
        if entity.kind != oteryn_session::EntityKind::Corpse {
            return false;
        }
        link.quick_loot_action(oteryn_client_ui::quick_loot::Action::Corpse(
            entity.entity.identity,
        ));
        true
    }
    /// A configured secondary click is consumed even when the server refuses looting.
    pub fn gameplay_secondary_click(&self, link: &PlayLink) -> bool {
        if !self.preferences().gameplay.secondary_click_loots_nearby {
            return false;
        }
        link.quick_loot_action(oteryn_client_ui::quick_loot::Action::LootAll);
        true
    }
}
