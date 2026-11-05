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
