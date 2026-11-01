use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::guild::{Intent, request};
impl PlayLink {
    pub fn guild_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.guild, intent)
            && self.commands.send(PlayCommand::Guild(command)).is_err()
        {
            mailbox.guild.pending = false;
            mailbox.guild.feedback = Some("The session has ended".into());
        }
    }
    pub fn open_guild(&self) {
        self.guild_request(Intent::RefreshOwn);
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).guild.open = true;
        }
    }
}
