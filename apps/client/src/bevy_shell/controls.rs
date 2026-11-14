//! Existing product input owners, shared with headless caller tests.
use super::game_ui::condition_details::{self, Target};
use super::owner::HostOwner;
use crate::{input::MouseActions, spell::SpellHotkeys};
use oteryn_input_actions::{InputError, NormalizedInputEvent};
pub(super) struct Controls {
    mouse: MouseActions,
    spells: SpellHotkeys,
}
impl Controls {
    pub fn new() -> Result<Self, InputError> {
        Ok(Self {
            mouse: MouseActions::new()?,
            spells: SpellHotkeys::new()?,
        })
    }
}
impl HostOwner {
    pub(super) fn route_input(&mut self, events: &[NormalizedInputEvent], blocked: bool) {
        let chat = self
            .play
            .as_ref()
            .is_some_and(|p| p.link.preference_chat_open());
        // Every reset reaches every stateful router, including releases suppressed by a modal.
        let reset = [NormalizedInputEvent::DeviceLost];
        let events = if blocked || self.modal_active() {
            &reset[..]
        } else {
            events
        };
        let controls = self.settings.control_events(events, chat);
        let (routed, open) = self.settings.input_in_context(&controls, chat);
        let clicks = self.controls.mouse.route(&routed);
        let spells = self.controls.spells.route(&routed);
        let custom = self.settings.take_custom_actions();
        if open {
            self.open_graphics();
            return;
        }
        if blocked || self.modal_active() {
            if let Some(play) = &mut self.play {
                let _ = play.view.world_pointer_input(&play.link, &reset);
                play.view.clear_bar_pointer(&play.link);
            }
            return;
        }
        let Some(play) = &mut self.play else {
            return;
        };
        if play.view.tutorial_input(&routed).unwrap_or(true)
            || play.view.notification_input(&routed)
            || play.link.use_with_cancel_input(&routed)
        {
            return;
        }
        // Ctrl+E belongs to Exiva outside chat; chat's channel action shares the key.
        if !chat && play.link.exiva_input(&routed) {
            return;
        }
        if play.link.chat_input(&routed) || play.link.chat_public_input(&routed) {
            return;
        }
        play.view.observe_creature_pointer(&routed);
        play.view.observe_bar_pointer(&play.link);
        for action in custom {
            play.link.custom_hotkey_action(&play.view, &action);
        }
        if play.link.custom_hotkeys_input(&routed)
            || play.view.target_move_input(&play.link, &routed)
        {
            return;
        }
        if play.view.audio_input(&routed) {
            if let Err(error) = self.settings.capture(&mut play.view) {
                self.status = error.to_string();
            }
            return;
        }
        if !chat && self.route_window_shortcuts(&routed) {
            return;
        }
        let Some(play) = &mut self.play else {
            return;
        };
        play.view.cursor_spell_input(&routed);
        if let Some(direction) = crate::input::arrow_step(&routed) {
            play.view.arrow(direction);
        }
        for spell in spells {
            play.view.cast(spell);
        }
        if crate::input::logout_key(&routed) {
            if self.settings.preferences.miscellaneous.confirm_logout {
                self.request_logout_confirmation();
            } else {
                play.view.logout();
            }
            return;
        }
        let suppress_primary = play
            .view
            .world_pointer_input(&play.link, &routed)
            .unwrap_or(true);
        play.view.observe_bar_pointer(&play.link);
        let mut context = None;
        for click in clicks {
            if suppress_primary && !click.secondary {
                continue;
            }
            if let Some(tile) = crate::input::click_tile(play.view.scene().view(), click.x, click.y)
            {
                if click.secondary {
                    if !play.view.gameplay_secondary_click(&play.link) {
                        context = Some(tile);
                        self.controls.mouse.context_closed();
                        break;
                    }
                } else if let Err(error) = play.view.primary_click(tile, &play.link) {
                    self.status = format!("Scene input failed: {error}");
                }
            }
        }
        if let Some(tile) = context {
            self.request_world_context(tile);
        }
    }

    fn route_window_shortcuts(&mut self, routed: &[NormalizedInputEvent]) -> bool {
        if routed.iter().any(|event| {
            matches!(event, NormalizedInputEvent::Key { modifiers, .. }
            if modifiers.contains(oteryn_input_actions::Modifier::Super))
        }) {
            return false;
        }
        if super::game_ui::cast_chat::shortcut(self, routed) {
            return true;
        }
        let Some(play) = &mut self.play else {
            return false;
        };
        if super::game_ui::highscores_character_info::input(&play.link, routed)
            || play.link.house_podium_input(routed)
            || play.link.quest_input(routed)
            || play.link.appearance_input(routed)
            || play.link.pvp_expert_input(routed)
            || play.link.daily_boost_input(routed)
            || play.link.daily_reward_input(routed)
            || play.link.spell_catalog_input(routed)
            || play.link.loyalty_skills_input(routed)
            || play.link.mail_open_input(routed)
            || play.link.raid_schedule_input(routed)
            || play.link.notices_input(routed)
            || play.link.npc_dialogue_input(routed, &play.view)
            || play.link.charm_progression_input(routed)
            || play.link.bevy_depot_input(routed)
            || play.link.bevy_depot_search_input(routed)
            || play.link.discovery_input(routed)
            || play.link.guild_war_input(routed)
            || play.link.imbue_input(routed)
            || play.link.bevy_stash_input(routed)
            || (routed.iter().any(|event| {
                matches!(
                    event,
                    NormalizedInputEvent::Key { code, .. } if code.get() == 25
                )
            }) && play.link.vip_input(routed))
            || play.link.wheel_input(routed)
        {
            return true;
        }
        if oteryn_client_ui::boss_encounter::open_key(routed)
            || oteryn_client_ui::boss_encounter::cooldowns_key(routed)
        {
            let cooldowns = oteryn_client_ui::boss_encounter::cooldowns_key(routed);
            super::game_ui::bosstiary::toggle(self, cooldowns);
            return true;
        }
        if oteryn_client_ui::hireling::open_key(routed) {
            super::game_ui::hireling_services::toggle(self);
            return true;
        }
        if play.link.guildhall_input(routed) {
            return true;
        }
        if oteryn_client_ui::cyclopedia_tracker::open_key(routed) {
            super::game_ui::cyclopedia_trackers::open(self);
            return true;
        }
        if oteryn_client_ui::cyclopedia_map::open_key(routed) {
            super::game_ui::cyclopedia_map::open(self);
            return true;
        }
        if oteryn_client_ui::boss_difficulty::open_key(routed) {
            super::game_ui::boss_difficulty::open(self);
            return true;
        }
        if oteryn_client_ui::boss_rewards::open_key(routed) {
            super::game_ui::open_boss_reward_chest(self);
            return true;
        }
        if oteryn_client_ui::animus_mastery::open_key(routed) {
            super::game_ui::animus_mastery::open(self);
            return true;
        }
        if oteryn_client_ui::browse_field::open_key(routed)
            && !routed.iter().any(|event| {
                matches!(event,
                NormalizedInputEvent::Key { modifiers, .. }
                    if modifiers.contains(oteryn_input_actions::Modifier::Super))
            })
        {
            play.link.browse_field_set_open_bound(true);
            return true;
        }
        if play.link.item_forge_input(routed) {
            return true;
        }
        if oteryn_client_ui::container_sort::open_key(routed) {
            play.link.container_sort_open();
            return true;
        }
        if oteryn_client_ui::arena::open_key(routed) {
            if !play.view.gameplay_input_blocked(&play.link) {
                let _ = play.link.arena_toggle_bound(&play.link.arena_snapshot());
            }
            return true;
        }
        if oteryn_client_ui::cyclopedia_character::open_key(routed) {
            super::game_ui::cyclopedia_character::toggle(self);
            return true;
        }
        if oteryn_client_ui::house_furniture_wrap::open_key(routed) {
            super::game_ui::furniture_wrapping::toggle(self);
            return true;
        }
        if self.phase == super::owner::Phase::Playing {
            if oteryn_client_ui::condition_status::open_key(routed) {
                condition_details::open_input(&play.link, Target::Conditions);
                return true;
            }
            if oteryn_client_ui::rest_status::open_key(routed) {
                condition_details::open_input(&play.link, Target::Rest);
                return true;
            }
        }
        if oteryn_client_ui::market::open_key(routed) {
            play.link.market_set_open(true);
            return true;
        }
        if oteryn_client_ui::party::open_key(routed) {
            play.link.open_party();
            return true;
        }
        if oteryn_client_ui::task_board::open_key(routed) {
            play.link.open_task_board();
            return true;
        }
        if oteryn_client_ui::task_board::soulseals_key(routed) {
            play.link.open_soulseals();
            return true;
        }
        if play.link.prey_input(routed) {
            return true;
        }
        if oteryn_client_ui::weapon_proficiency::open_key(routed) {
            super::game_ui::weapon_proficiency::open(self);
            return true;
        }
        if oteryn_client_ui::staff_tools::open_key(routed) {
            let _ = play.link.staff_tools_open_bound();
            return true;
        }
        if oteryn_client_ui::player_report::open_key(routed) {
            play.link.player_report_open();
            return true;
        }

        // Existing routes retain chord ownership; new routes are inserted by owner name.
        if oteryn_client_ui::achievement_notices::open_key(routed) {
            super::game_ui::open_achievement_notices(self);
            return true;
        }
        if oteryn_client_ui::cast_control::open_key(routed) {
            super::game_ui::cast_control::open(self);
            return true;
        }
        if oteryn_client_ui::market::coin_open_key(routed) {
            play.link.open_coin_market();
            return true;
        }
        if play.link.guild_input(routed)
            || play.link.guild_bank_input(routed)
            || play.link.mechanics_status_input(routed)
            || play.link.party_effects_input(routed)
            || play.link.potion_durations_input(routed)
            || play.link.store_input(routed)
        {
            return true;
        }
        // Furniture rotation's Ctrl+Alt+Shift+F is already owned by the forge route.
        if oteryn_client_ui::training::open_key(routed) {
            super::game_ui::training::open(self);
            return true;
        }
        if oteryn_client_ui::world_event::open_key(routed) {
            super::game_ui::world_event_window::open(self);
            return true;
        }
        false
    }

    /// A shared popup may consume the OS release; clear its retained pointer state.
    pub(super) fn pointer_context_closed(&mut self) {
        self.controls.mouse.context_closed();
    }
}

#[cfg(test)]
pub(in crate::bevy_shell) mod tests;
