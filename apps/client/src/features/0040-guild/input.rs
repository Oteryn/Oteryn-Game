use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::guild::{Intent, request};
impl PlayLink {
    pub fn guild_input(&self, events: &[oteryn_input_actions::NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::guild::open_key(events) {
            return false;
        }
        self.open_guild();
        true
    }

    pub fn guild_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.guild, intent)
            && self.commands.send(PlayCommand::Guild(command)).is_err()
        {
            mailbox.guild.pending = false;
            mailbox.guild.feedback = Some(tr!("The session has ended").into());
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
