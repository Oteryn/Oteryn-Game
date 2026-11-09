use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::chat_public_channels::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

impl PlayLink {
    pub fn chat_public_text(&self, text: String) {
        lock(&self.mailbox).chat_public.text = text;
    }

    pub fn chat_public_request(&self, intent: Intent) {
        enqueue(&mut lock(&self.mailbox), &self.commands, intent);
    }

    pub fn chat_public_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::chat_public_channels::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).chat_public.open = true;
        }
        true
    }
}

pub(in crate::play) fn enqueue(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    if let Some(command) = request(&mut mailbox.chat_public, intent)
        && commands.send(PlayCommand::ChatPublic(command)).is_err()
    {
        mailbox.chat_public.pending = false;
        mailbox.chat_public.feedback = Some(tr!("The session has ended").into());
    }
}
