//! Existing product input owners, shared with headless caller tests.
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
        let events = if blocked || self.draft().is_some() {
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
        if blocked || self.draft().is_some() {
            return;
        }
        let Some(play) = &mut self.play else {
            return;
        };
        if play.view.tutorial_input(&routed).unwrap_or(true)
            || play.view.notification_input(&routed)
            || play.link.use_with_cancel_input(&routed)
            || play.link.chat_input(&routed)
            || play.link.chat_public_input(&routed)
        {
            return;
        }
        for action in custom {
            play.link.custom_hotkey_action(&play.view, &action);
        }
        if play.link.custom_hotkeys_input(&routed)
            || play.view.target_move_input(&play.link, &routed)
        {
            return;
        }
        if let Some(direction) = crate::input::arrow_step(&routed) {
            play.view.arrow(direction);
        }
        for spell in spells {
            play.view.cast(spell);
        }
        if crate::input::logout_key(&routed)
            && (!self.settings.preferences.miscellaneous.confirm_logout || confirm_logout())
        {
            play.view.logout();
        }
        let suppress_primary = play
            .view
            .world_pointer_input(&play.link, &routed)
            .unwrap_or(true);
        for click in clicks {
            if suppress_primary && !click.secondary {
                continue;
            }
            if let Some(tile) = crate::input::click_tile(play.view.scene().view(), click.x, click.y)
            {
                if click.secondary {
                    if !play.view.gameplay_secondary_click(&play.link) {
                        #[cfg(windows)]
                        play.view.properties_context_menu(&play.link, tile);
                        self.controls.mouse.context_closed();
                    }
                } else if let Err(error) = play.view.primary_click(tile, &play.link) {
                    self.status = format!("Scene input failed: {error}");
                }
            }
        }
    }
}

fn confirm_logout() -> bool {
    #[cfg(windows)]
    {
        crate::device_login::confirm_action("Log out of the game?")
    }
    #[cfg(not(windows))]
    {
        false
    }
}
