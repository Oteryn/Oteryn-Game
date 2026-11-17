//! Local gameplay choices consume the same capability-gated callers as the game controls.
use crate::play::{PlayLink, PlayView};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GameplayPreferences {
    pub inspect_context_menu: bool,
    pub cancel_chase_on_walk: bool,
    pub secondary_click_loots_nearby: bool,
}

impl Default for GameplayPreferences {
    fn default() -> Self {
        Self {
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

    /// A configured secondary click is consumed even when the server refuses looting.
    pub fn gameplay_secondary_click(&self, link: &PlayLink) -> bool {
        if !self.preferences().gameplay.secondary_click_loots_nearby {
            return false;
        }
        link.quick_loot_action(oteryn_client_ui::quick_loot::Action::LootAll);
        true
    }
}
