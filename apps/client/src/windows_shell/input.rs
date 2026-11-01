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
            if play.link.animus_input(events)
                || play.link.wheel_input(events)
                || play.link.exiva_input(events)
                || play.link.loyalty_skills_input(events)
                || play.link.quest_input(events)
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
                    render_failed |= play.view.click(tile).is_err();
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
