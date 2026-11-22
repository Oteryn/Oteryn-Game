use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, SharedMailbox, lock},
};
use tokio::sync::mpsc::UnboundedSender;
impl PlayLink {
    pub fn open_guild_chat(&self) -> bool {
        open(&self.mailbox, &self.commands)
    }
}

pub(in crate::play) fn open(
    mailbox: &SharedMailbox,
    _commands: &UnboundedSender<PlayCommand>,
) -> bool {
    open_locked(&mut lock(mailbox))
}

/// Opens the guild room in the chat window; the guild window's Guild chat button uses it.
pub(in crate::play) fn open_locked(mailbox: &mut Mailbox) -> bool {
    if !oteryn_client_ui::chat_guild_room::available(&mailbox.chat) {
        return false;
    }
    mailbox.chat.guild_open = true;
    mailbox.chat.open = true;
    mailbox.chat.console.open_channel(oteryn_client_ui::chat::ChannelTab::Guild);
    true
}
