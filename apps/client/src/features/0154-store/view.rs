use crate::play::{PlayLink, mailbox::lock};
#[path = "coin_shop.rs"]
mod coin_shop;
impl PlayLink {
    pub fn store_gift_open(&self) {
        lock(&self.mailbox).store.gift_open = true;
    }

    #[must_use]
    pub fn store_panel(&self) -> oteryn_client_ui::store::StorePanel {
        lock(&self.mailbox).store.clone()
    }
}
#[path = "native.rs"]
pub(in crate::play) mod native;

#[path = "../local-premium/game_play.rs"]
pub(in crate::play) mod premium_game;

