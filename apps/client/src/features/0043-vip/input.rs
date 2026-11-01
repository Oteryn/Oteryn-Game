use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::vip::{Intent, request};
use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};

impl PlayLink {
    pub fn vip_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.vip, intent)
            && self.commands.send(PlayCommand::Vip(command)).is_err()
        {
            mailbox.vip.pending = false;
            mailbox.vip.feedback = Some("The session has ended".into());
        }
    }
    pub fn vip_input(&self, events: &[NormalizedInputEvent]) -> bool {
        let open = events.iter().any(|event| matches!(event, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false } if code.get() == 25 && modifiers.contains(Modifier::Control) && !modifiers.contains(Modifier::Alt) && !modifiers.contains(Modifier::Shift) && !modifiers.contains(Modifier::Super)));
        if !open {
            return false;
        }
        self.vip_request(Intent::Query(0));
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).vip.open = true;
        }
        true
    }
}
