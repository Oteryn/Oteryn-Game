//! Native shell input phase; mechanics keep their input mapping in feature adapters.
use super::*;
use oteryn_client::input::{arrow_step, click_tile, logout_key};

impl Application {
    pub(super) fn handle_input(
        &mut self,
        event_loop: &ActiveEventLoop,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) {
        let controls = self
            .settings
            .control_events(events, self.play.as_ref().map(|play| &play.link));
        let events = controls.as_slice();
        // Route the full batch before a panel can consume it, retaining pointer/reset events.
        let routed = self
            .settings
            .input(events, self.play.as_ref().map(|play| &play.link));
        let events = routed.as_slice();
        let clicks = self.actions.route(events);
        if let Some(play) = &mut self.play {
            play.view.observe_creature_pointer(events);
            if play.link.use_with_cancel_input(events) {
                return;
            }
        }

        if let Some(play) = &mut self.play {
            self.settings.restore(&mut play.view, &play.link);
            if play.view.audio_input(events) {
                self.settings.capture(&mut play.view);
                return;
            }
            if play.view.minimap_input(events)
                || play.view.container_input(&play.link, events)
                || play.link.inventory_input(events)
                || play.link.battle_input(events)
                || (oteryn_client_ui::party::open_key(events) && {
                    play.link.open_party();
                    true
                })
                || play.link.npc_dialogue_input(events, &play.view)
                || play.link.achievement_notices_input(events)
                || play.link.achievements_input(events)
                || play.link.cast_control_input(events)
                || play.link.cast_chat_input(events)
                || play.link.chat_input(events)
                || play.link.chat_public_input(events)
                || play.link.boss_encounter_input(events)
                || play.link.highscores_input(events)
                || play.link.notices_input(events)
                || play.link.animus_input(events)
                || play.link.arena_input(events)
                || play.link.prey_input(events)
                || play.link.task_board_input(events)
                || play.link.wheel_input(events)
                || play.link.discovery_input(events)
                || play.link.exiva_input(events)
                || play.link.analyser_input(events)
                || play.link.spell_catalog_input(events)
                || play.link.spell_bar_input(events)
                || play.link.loyalty_skills_input(events)
                || play.link.mechanics_status_input(events)
                || play.link.condition_status_input(events)
                || play.link.rest_status_input(events)
                || play.link.battle_instinct_status_input(events)
                || play.link.skill_progress_input(events)
                || play.link.party_effects_input(events)
                || play.link.guild_input(events)
                || play.link.stash_input(events)
                || play.link.quest_input(events)
                || play.link.boss_difficulty_input(events)
                || play.link.browse_field_input(events)
                || play.link.chat_party_room_input(events)
                || play.link.cyclopedia_character_input(events)
                || play.link.cyclopedia_tracker_input(events)
                || play.link.world_event_input(events)
                || play.link.daily_boost_input(events)
                || play.link.daily_reward_input(events)
                || play.link.market_input(events)
                || play.link.depot_input(events)
                || play.link.depot_search_input(events)
                || play.link.mail_input(events)
                || play.link.house_podium_input(events)
                || play.link.house_rotation_input(events)
                || play.link.house_furniture_input(events)
                || play.link.appearance_input(events)
                || play.link.charm_progression_input(events)
                || play.link.pvp_expert_input(events)
                || play.link.training_input(events)
                || play.link.guild_bank_input(events)
                || play.link.timed_item_input(events)
                || play.link.cyclopedia_map_input(events)
                || play.link.party_skull_input(events)
                || play.link.character_info_input(events)
                || play.link.raid_schedule_input(events)
                || play.link.container_sort_input(events)
                || play.link.guild_war_input(events)
                || play.link.guildhall_input(events)
                || play.link.house_input(events)
                || play.link.house_move_out_input(events)
                || play.link.house_transfer_input(events)
                || play.link.player_report_input(events)
                || play.link.hireling_input(events)
                || play.link.hotkey_equip_input(events)
                || play.link.potion_durations_input(events)
                || play.link.item_forge_input(events)
                || play.link.store_input(events)
                || play.link.vip_input(events)
                || play.link.vip_presence_input(events)
                || play.link.weapon_proficiency_input(events)
            {
                return;
            }
            play.view.cursor_spell_input(events);
            if play.link.player_trade_input(events) {
                return;
            }
            if play.view.target_move_input(&play.link, events) {
                return;
            }
            if let Some(direction) = arrow_step(events) {
                play.view.arrow(direction);
            }
            for spell in self.hotkeys.route(events) {
                play.view.cast(spell);
            }
            if logout_key(events) {
                play.view.logout();
            }
            let suppress_primary = match play.view.world_pointer_input(&play.link, events) {
                Ok(consumed) => consumed,
                Err(_) => {
                    self.fail(event_loop, ShellError::RendererRender);
                    return;
                }
            };
            let mut render_failed = false;
            for click in clicks {
                if !click.secondary && suppress_primary {
                    continue;
                }
                if !click.secondary && play.view.minimap_click(click.x, click.y) {
                    continue;
                }
                if let Some(tile) = click_tile(play.view.scene().view(), click.x, click.y) {
                    if click.secondary {
                        play.view.properties_context_menu(&play.link, tile);
                        self.actions.context_closed();
                    } else {
                        render_failed |= play.view.primary_click(tile, &play.link).is_err();
                    }
                }
            }
            if render_failed {
                self.fail(event_loop, ShellError::RendererRender);
            }
            return;
        }
        if oteryn_client::reentry::login_key(events) {
            self.request_entry();
        }
        if let Some(direction) = arrow_step(events) {
            self.offline_own = direction.from(self.offline_own);
            self.offline_facing = direction;
            if self.rebuild_offline().is_err() {
                self.fail(event_loop, ShellError::RendererRender);
                return;
            }
        }
        for click in clicks {
            let picked = self.scene.as_mut().and_then(|scene| {
                let tile = click_tile(scene.view(), click.x, click.y)?;
                Some((tile, scene.select_tile(tile)))
            });
            // Without a session a click only selects a visible entity or object.
            if let Some((_, Err(_error))) = picked {
                self.fail(event_loop, ShellError::RendererRender);
            }
        }
    }
}
