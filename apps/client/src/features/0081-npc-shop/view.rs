use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    #[cfg(bevy_shell)]
    pub(crate) fn npc_chat_mode(&self) -> oteryn_client_ui::chat::ComposerMode {
        lock(&self.mailbox).chat.console.mode
    }
    pub fn npc_shop_panel(&self) -> oteryn_client_ui::npc_shop::NpcShopPanel {
        let mailbox = lock(&self.mailbox);
        let mut shop = mailbox.npc_shop.clone();
        shop.live.pending |= shop.live.pending_pack_in_backpacks.is_some();
        shop.live.inspection_selected = mailbox.discovery.inspection.selected;
        shop.live.free_capacity = mailbox.inventory.capacity.map(|c| c.free);
        shop
    }
}

impl PlayLink {
    /// Cheap retained visibility check for shared frontends before cloning the quote model.
    pub fn npc_shop_is_open(&self) -> bool {
        let mailbox = lock(&self.mailbox);
        !mailbox.npc_dialog_dismissed && mailbox.npc_shop.open
    }
    /// Closes presentation only; outstanding server commands retain their owning state.
    pub fn npc_shop_close(&self) {
        lock(&self.mailbox).npc_shop.open = false;
    }
    pub fn npc_shop_look_request(&self, quote: oteryn_client_ui::npc_shop::Quote) {
        super::input::offer_request(&self.mailbox, &self.commands, quote, false);
    }
}
