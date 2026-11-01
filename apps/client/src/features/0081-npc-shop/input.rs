use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::npc_shop::Intent;
impl PlayLink {
    pub fn npc_shop_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = oteryn_client_ui::npc_shop::request(&mut mailbox.npc_shop, intent)
            && self.commands.send(PlayCommand::NpcShop(command)).is_err()
        {
            mailbox.npc_shop.pending = false;
            mailbox.npc_shop.feedback = Some("Session ended");
        }
    }
}
