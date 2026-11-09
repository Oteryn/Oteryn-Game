use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::cyclopedia_tracker::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn cyclopedia_tracker_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        enqueue(&mut mailbox, &self.commands, intent);
    }
    pub fn cyclopedia_tracker_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::cyclopedia_tracker::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).cyclopedia_tracker.open = true;
        }
        true
    }
}

pub(super) fn enqueue(
    mailbox: &mut crate::play::mailbox::Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    if let Some(command) = request(&mut mailbox.cyclopedia_tracker, intent)
        && commands.send(PlayCommand::Feature(command)).is_err()
    {
        mailbox.cyclopedia_tracker.pending = false;
        mailbox.cyclopedia_tracker.feedback = Some(tr!("The session has ended"));
    }
}
