use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn npc_shop_panel(&self) -> oteryn_client_ui::npc_shop::NpcShopPanel {
        lock(&self.mailbox).npc_shop.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

impl PlayLink {
    /// Cheap retained visibility check for shared frontends before cloning the quote model.
    pub fn npc_shop_is_open(&self) -> bool {
        lock(&self.mailbox).npc_shop.open
    }
    /// Closes presentation only; outstanding server commands retain their owning state.
    pub fn npc_shop_close(&self) {
        lock(&self.mailbox).npc_shop.open = false;
    }
    pub fn npc_shop_look_request(&self, quote: oteryn_client_ui::npc_shop::Quote) {
        super::input::offer_request(&self.mailbox, &self.commands, quote, false);
    }
}
