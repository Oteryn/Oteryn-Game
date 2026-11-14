use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::hireling::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;

impl PlayLink {
    pub fn hireling_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::hireling::open_key(events) {
            return false;
        }
        if lock(&self.mailbox).hireling_management.selected {
            self.hireling_management_open();
            return true;
        }
        self.hireling_request(Intent::Refresh);
        #[cfg(windows)]
        self.open_hireling_panel();
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).hireling.open = true;
        }
        true
    }

    pub fn hireling_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.hireling, intent)
            && self.commands.send(PlayCommand::Hireling(command)).is_err()
        {
            mailbox.hireling.pending = None;
            mailbox.hireling.feedback = Some(tr!("The session has ended"));
        }
    }

    pub fn hireling_target(&self, x: u16, y: u16, floor: i8) {
        lock(&self.mailbox).hireling.target = Some((x, y, floor));
    }
}
