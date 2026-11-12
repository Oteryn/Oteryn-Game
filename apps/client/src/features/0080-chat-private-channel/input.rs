use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::chat_private_channel::{Intent, request};
use tokio::sync::mpsc::UnboundedSender;
impl PlayLink {
    pub fn chat_private_text(&self, text: String) {
        lock(&self.mailbox).chat_private.text = text;
    }
    pub fn chat_private_name(&self, name: String) {
        lock(&self.mailbox).chat_private.name = name;
    }
    pub fn chat_private_select(&self, channel: u16) {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.chat_private.list.as_ref().is_some_and(|list| {
            list.own_channel == Some(channel)
                || list
                    .invited
                    .iter()
                    .any(|invite| invite.channel_id == channel)
        }) {
            mailbox.chat_private.channel = Some(channel);
        }
    }
    pub fn chat_private_request(&self, intent: Intent) {
        enqueue(&mut lock(&self.mailbox), &self.commands, intent);
    }
}
pub(in crate::play) fn enqueue(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    if let Some(command) = request(&mut mailbox.chat_private, intent)
        && commands
            .send(PlayCommand::ChatPublic(
                oteryn_session::ChatIntent::PrivateChannel(command),
            ))
            .is_err()
    {
        mailbox.chat_private.pending = false;
        mailbox.chat_private.pending_intent = None;
        mailbox.chat_private.feedback = Some(tr!("The session has ended").into());
    }
}
