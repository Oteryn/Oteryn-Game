use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn npc_shop_panel(&self) -> oteryn_client_ui::npc_shop::NpcShopPanel {
        lock(&self.mailbox).npc_shop.clone()
    }
    #[cfg(windows)]
    pub fn open_npc_shop(&self, look: oteryn_client_runtime::npc_shop::npc_shop::NpcShopIntent) {
        native::open_window(self, look);
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
