use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::chat::{Mode, request};
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;
impl PlayLink {
    pub fn chat_text(&self, text: String) {
        lock(&self.mailbox).chat.text = text;
    }
    pub fn chat_request(&self, mode: Mode, recipient: &str) {
        enqueue(&mut lock(&self.mailbox), &self.commands, mode, recipient);
    }
    pub fn chat_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::chat::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).chat.open = true;
        }
        true
    }
}
pub(in crate::play) fn enqueue(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    mode: Mode,
    recipient: &str,
) {
    if let Some(intent) = request(&mut mailbox.chat, mode, recipient)
        && commands.send(PlayCommand::ChatPublic(intent)).is_err()
    {
        mailbox.chat.pending = false;
        mailbox.chat.feedback = Some("The session has ended".into());
    }
}
