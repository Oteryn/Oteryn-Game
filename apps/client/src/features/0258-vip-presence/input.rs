use crate::play::{PlayLink, mailbox::lock};
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn vip_presence_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::vip_presence::open_key(events) {
            return false;
        }
        if self.uses_shared_frontend() {
            lock(&self.mailbox).vip_presence.open = true;
        } else {
            #[cfg(windows)]
            super::view::open_window(self);
        }
        true
    }
}

#[cfg(all(test, not(windows)))]
mod tests {
    use super::*;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers};

    #[test]
    fn vip_shortcut_leaves_native_open_state_to_the_opener()
    -> Result<(), Box<dyn std::error::Error>> {
        let (link, _commands, _outbox) = crate::play::play_channel();
        let events = [NormalizedInputEvent::Key {
            code: KeyCode::new(25)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE
                .with(Modifier::Control)
                .with(Modifier::Shift),
            repeat: false,
        }];
        assert!(link.vip_presence_input(&events));
        assert!(!link.vip_presence_panel().open);
        link.use_shared_frontend();
        assert!(link.vip_presence_input(&events));
        assert!(link.vip_presence_panel().open);
        Ok(())
    }
}
