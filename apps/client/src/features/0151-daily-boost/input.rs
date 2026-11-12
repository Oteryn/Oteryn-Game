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
            mailbox.daily_boost.feedback = Some(tr!("The session has ended"));
        }
    }
    pub fn daily_boost_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::daily_boost::open_key(events) {
            return false;
        }
        {
            let mut mailbox = lock(&self.mailbox);
            if mailbox.daily_boost.open {
                return true;
            }
            mailbox.daily_boost.open = true;
        }
        self.daily_boost_request(Intent {
            command: oteryn_client_runtime::cui3::daily_boost::COMMAND_TYPE_DAILY_BOOST_QUERY,
            payload: Vec::new(),
        });
        #[cfg(all(windows, not(test)))]
        super::view::open_window(self);
        true
    }
}
