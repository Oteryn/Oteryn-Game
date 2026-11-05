//! Native shell input phase; mechanics keep their input mapping in feature adapters.
use super::*;
use oteryn_client::input::{arrow_step, click_tile, logout_key};

impl Application {
    pub(super) fn handle_input(
        &mut self,
        event_loop: &ActiveEventLoop,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) {
        if let Some(play) = &mut self.play {
            if play.view.audio_input(events) {
                return;
            }
            if play.link.inventory_input(events)
                || play.link.achievement_notices_input(events)
                || play.link.achievements_input(events)
                || play.link.chat_input(events)
                || play.link.chat_public_input(events)
                || play.link.boss_encounter_input(events)
                || play.link.highscores_input(events)
                || play.link.notices_input(events)
                || play.link.animus_input(events)
                || play.link.prey_input(events)
                || play.link.task_board_input(events)
                || play.link.wheel_input(events)
                || play.link.exiva_input(events)
                || play.link.loyalty_skills_input(events)
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
                || play.link.depot_search_input(events)
                || play.link.hireling_input(events)
                || play.link.hotkey_equip_input(events)
                || play.link.item_forge_input(events)
                || play.link.store_input(events)
                || play.link.vip_input(events)
                || play.link.vip_presence_input(events)
                || play.link.weapon_proficiency_input(events)
            {
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
            let mut render_failed = false;
            for click in self.actions.route(events) {
                if let Some(tile) = click_tile(play.view.scene().view(), click.x, click.y) {
                    if click.secondary {
                        if let Some(menu) = play.view.world_context(tile, &play.link) {
                            oteryn_client::play::open_world_context_menu(
                                &play.view, &play.link, &menu,
                            );
                            self.actions.context_closed();
                        }
                    } else {
                        render_failed |= play.view.click(tile).is_err();
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
        for click in self.actions.route(events) {
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
