use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::daily_boost::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn daily_boost_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.daily_boost, intent)
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            mailbox.daily_boost.pending = false;
            mailbox.daily_boost.feedback = Some("The session has ended");
        }
    }
    pub fn daily_boost_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::daily_boost::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).daily_boost.open = true;
        }
        true
    }
}
