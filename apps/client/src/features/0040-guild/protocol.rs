use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn guild_initialize(&self, selected: bool) {
        lock(&self.0).guild.selected = selected;
    }
    pub(in crate::play) fn guild_result(&self, reply: &oteryn_client_runtime::guild::GuildReply) {
        oteryn_client_ui::guild::apply_result(&mut lock(&self.0).guild, reply);
    }
}
