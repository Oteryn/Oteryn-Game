use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{SharedMailbox, lock},
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
    {
        let mut mailbox = lock(mailbox);
        if !oteryn_client_ui::chat_guild_room::available(&mailbox.chat) {
            return false;
        }
        mailbox.chat.guild_open = true;
    }
    #[cfg(windows)]
    crate::play::feature_0007_chat::view::open_shared(mailbox, _commands);
    #[cfg(not(windows))]
    {
        lock(mailbox).chat.open = true;
    }
    true
}
