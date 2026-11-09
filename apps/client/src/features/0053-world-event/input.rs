use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn world_event_request(&self) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = oteryn_client_ui::world_event::request(&mut mailbox.world_event)
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            mailbox.world_event.pending = false;
            mailbox.world_event.feedback = Some(tr!("The session has ended"));
        }
    }
    pub fn world_event_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::world_event::open_key(events) {
            return false;
        }
        self.world_event_request();
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).world_event.open = true;
        }
        true
    }
}
